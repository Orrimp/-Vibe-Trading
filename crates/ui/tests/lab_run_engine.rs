//! Integration test for H3 hypothesis — in-memory equity series equals
//! cached-disk equity series after a completed backtest run.
//!
//! ## H3 (from feature.md § Hypotheses)
//!
//! "The in-memory equity series returned by `engine::run_scenario` is
//! element-by-element identical to the series that `EquityCache::get_or_load`
//! parses from the written report file."
//!
//! Falsification criteria: any element pair `(ts, equity)` differs between
//! the two sources. A mismatch flags a determinism bug in the report writer
//! or the equity parser.
//!
//! ## Test strategy
//!
//! 1. Call `backtest::engine::run_scenario` with a fixed-seed
//!    `(v1.momentum, XRPUSDT, Last90d, write_report=true)` config.
//! 2. From the returned `RunReport`, extract the `equity_series`.
//! 3. Point `EquityCache::get_or_load` at the report root for the same tuple,
//!    after asserting that root IS the temp dir the engine was told to write
//!    to (write-root == read-root, ADR-0055 § D6).
//! 4. Assert element-by-element equality.
//!
//! ## Gating — no skip
//!
//! `#[cfg(feature = "live")]` on `mod inner` (a default feature; `trace.toml`
//! cites the test as `lab_run_engine.rs::inner::h3_in_memory_equals_cached_disk`).
//!
//! Until story 2-63 Wave E3 the test returned early, printing a "skipped"
//! line, in two cases: when `run_scenario` returned `NotImplemented` (a Phase B
//! stub long since replaced), and when the report came back with
//! `report_path: None` (a Phase B contract the engine no longer has: the
//! momentum arm assigns `report.report_path = maybe_write_report(..)?`). A
//! third, `cfg(not(feature = "live"))` stub always passed. All three are gone
//! (bug-log `#66` shape, AC3): ANY engine error fails, and a missing
//! `report_path` fails. The doc line in `crates/backtest/src/engine.rs` that
//! still says the engine "currently returns `report_path: None`" is stale
//! production text, recorded and not edited here (story 2-63 AC5).
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in PRODUCTION code (the OLD text
//! occurs exactly once; `grep -c`), located by symbol + quoted line. Run
//! `cargo test -p ui --test lab_run_engine`, then restore from a `cp -p`
//! snapshot, `cmp` it, `touch` it, and re-run GREEN.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E3-RE-P1 | `backtest::engine::run_scenario`'s `"v1.momentum" \| "top10_momentum_h1"` arm (`crates/backtest/src/engine.rs`) | `\|path\| crate::report::momentum::write(&input, &result, seed_u64, "synthetic", path),` followed by `)?;` | append `report.report_path = None;` after the `)?;` | `inner::h3_in_memory_equals_cached_disk` at "H3: write_report=true with reports_dir=Some must return a report_path" |
//! | E3-RE-P2 (optional) | the same arm | `let input = crate::cli_types::MomentumScenarioInput {` | insert `return Err(RunError::NotImplemented);` above it | the same test at "H3 test: run_scenario returned an error: …" |
//!
//! Vacuity of the pre-E3 file (measured by the E3 base run, same build as
//! E3-RE-P1): the old file stays GREEN under E3-RE-P1 and prints its
//! "report_path=None … skipped" line under `--nocapture`; under E3-RE-P2 it
//! stays GREEN through its `NotImplemented` skip.

#[cfg(feature = "live")]
mod inner {
    use std::path::Path;

    use backtest::engine::{DateRange as EngDateRange, ScenarioConfig};
    use smol_str::SmolStr;
    use trading_core::{StrategyId, Symbol, Venue};
    use ui::lab::defaults::LAB_DEFAULT_SEED;
    use ui::lab::equity_loader::{EquityCache, LabTuple};
    use ui::lab::state::{DateRange, LabDataSource, Preset};

    /// Build the canonical test `ScenarioConfig`.
    ///
    /// lab-run-save-compare ADR-0055 § D6 (H3 invariant): `reports_dir` is
    /// set to `Some(tmp_dir)` so the engine writes
    /// `<tmp_dir>/<slug>/reports/backtest-<ms-stamp>-<scenario>.md`.
    /// The test derives the read-root as
    /// `report_path.parent().parent().parent()`, asserts it equals `tmp_dir`,
    /// and calls `get_or_load(tuple, read_root)`.  Equal roots ⇒ the
    /// just-written file is the one parsed ⇒ element-by-element
    /// `in_memory == cached_disk`.
    fn test_config(tmp_dir: &std::path::Path) -> ScenarioConfig {
        ScenarioConfig {
            strategy: StrategyId("v1.momentum".into()),
            pair: (Venue::Binance, Symbol::new("XRPUSDT")),
            range: EngDateRange::Last90d,
            params: None,
            seed: LAB_DEFAULT_SEED,
            write_report: true,
            data_source: backtest::engine::ScenarioDataSource::default(),
            bars_override: None,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
            // ADR-0055 § D6: write-root == read-root — engine writes to tmp_dir,
            // test reads from tmp_dir; equality is structural, not incidental.
            reports_dir: Some(tmp_dir.to_path_buf()),
            short_enabled: false,
            initial_capital: None,
            composed_toml_override: None,
            dvol_override: None,
            macro_regime_series: None,
        }
    }

    /// H3: in-memory equity series equals the parsed report equity series.
    ///
    /// Fails on any engine error and on a missing `report_path` (no skip;
    /// module doc § Gating).
    #[tokio::test]
    async fn h3_in_memory_equals_cached_disk() {
        // Use a temp directory for the report root so we don't pollute the tree.
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg = test_config(tmp.path());
        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::Last90d),
            source: LabDataSource::Synthetic,
        };

        // Step 1: run the backtest. Wave D-3+D-4 added cancel + progress args.
        let (_handle, cancel_rx) = backtest::cancel::cancellation_pair();
        let progress_tx = backtest::progress::ProgressSender::disabled();
        let report = backtest::engine::run_scenario(cfg, cancel_rx, progress_tx)
            .await
            .unwrap_or_else(|e| panic!("H3 test: run_scenario returned an error: {e}"));

        // Step 2: extract the in-memory equity series. Today's API uses
        // Money<Usdt> for equity; project to (i64, Decimal) for the
        // EquityCache comparison.
        let in_memory: Vec<(i64, rust_decimal::Decimal)> = report
            .equity_series
            .iter()
            .map(|(ts, money)| (ts.unix_millis(), money.amount()))
            .collect();

        assert!(
            !in_memory.is_empty(),
            "H3: in-memory equity series must not be empty"
        );

        // Step 3: the report was written, under the root the engine was given.
        let report_path = report
            .report_path
            .as_ref()
            .expect("H3: write_report=true with reports_dir=Some must return a report_path");
        let read_root = report_path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .expect("H3: report_path has a <root>/<slug>/reports/<file> shape");
        assert_eq!(
            read_root,
            tmp.path(),
            "H3: write-root == read-root (ADR-0055 § D6)"
        );

        let mut cache = EquityCache::new();
        let cached = cache
            .get_or_load(&tuple, read_root)
            .expect("H3: EquityCache must find the just-written report");

        // Step 4: element-by-element equality.
        assert_eq!(
            in_memory.len(),
            cached.samples.len(),
            "H3: series length mismatch: in-memory={} cached={}",
            in_memory.len(),
            cached.samples.len()
        );

        for (i, ((ts_mem, eq_mem), (ts_cache, eq_cache))) in
            in_memory.iter().zip(cached.samples.iter()).enumerate()
        {
            assert_eq!(
                ts_mem, ts_cache,
                "H3: timestamp mismatch at index {i}: in-memory={ts_mem} cached={ts_cache}"
            );
            assert_eq!(
                eq_mem, eq_cache,
                "H3: equity mismatch at index {i}: in-memory={eq_mem} cached={eq_cache}"
            );
        }

        eprintln!(
            "H3: PASS — {} equity points equal between in-memory and cached-disk",
            in_memory.len()
        );
    }
}
