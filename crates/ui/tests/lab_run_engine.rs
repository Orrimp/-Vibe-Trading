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
//!
//! ## bug-log #212 — every Lab chip's run is read back
//!
//! [`inner::every_lab_chip_run_is_read_back_from_its_write_root`] runs each
//! chip `cockpit_live` seeds (`ui::lab::defaults::LAB_SEED_STRATEGIES`)
//! through the real engine (Synthetic, `Last30d`, BTCUSDT, `write_report`,
//! `reports_dir` = a tempdir) and reads it back through `EquityCache`. Per
//! chip: the written report's `id:` line, scanned from its frontmatter here,
//! equals `backtest::engine::report_strategy_id(chip)`; and for each of the
//! Lab's 10 pairs (`ui::lab::universe::XRP_FIRST_UNIVERSE`) the cold read
//! resolves the written file iff the report states that coin (its Summary
//! `| Symbol |` row or its `## Universe`, scanned here), else `NoReport`. So
//! the pairs arm, whose universe is 4 coins, is pinned two-sided: BTCUSDT
//! resolves, XRPUSDT does not (ADR-0055 D9). `v2.5.tcn.weights` is asserted
//! to return its "requires --features candle" error and write nothing: no ui
//! build enables `backtest/candle` (a declared outcome, not a skip; its
//! read-back is `lab::equity_loader`'s M8, from the committed reports' id).
//! H3 above stays green only because the cross-sectional arms' Lab scenario
//! names carry their range since #212 (`v1.momentum-last90d-synthetic`); the
//! #212 probe table is in `lab_buy_sell_overlay_render.rs`.

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

    /// The `id:` value(s) in a report's frontmatter (between its first two
    /// `---` lines), scanned without the Lab's parser: the engine writes the
    /// `strategy:` block's `id:` unindented, a hand-written report indents it.
    fn frontmatter_ids(text: &str) -> Vec<String> {
        text.lines()
            .skip_while(|l| l.trim() != "---")
            .skip(1)
            .take_while(|l| l.trim() != "---")
            .filter_map(|l| l.trim_start().strip_prefix("id: "))
            .map(|v| v.trim().to_string())
            .collect()
    }

    /// The coins a report states, scanned without the Lab's parser: the value
    /// of a `| Symbol | … |` table row, and every `- <coin>` line of its
    /// `## Universe` section.
    fn stated_coins(text: &str) -> Vec<String> {
        let mut coins: Vec<String> = text
            .lines()
            .filter_map(|l| {
                let cells: Vec<&str> = l.split('|').map(str::trim).collect();
                (cells.len() >= 3 && cells[1] == "Symbol").then(|| cells[2].to_string())
            })
            .collect();
        coins.extend(
            text.lines()
                .skip_while(|l| !l.starts_with("## Universe"))
                .skip(1)
                .take_while(|l| !l.starts_with("## "))
                .filter_map(|l| l.strip_prefix("- "))
                .map(|c| c.trim().to_string()),
        );
        coins
    }

    /// bug-log #212 — every Lab chip's persisted run is read back from the
    /// root the engine wrote it to, for exactly the coins its report states
    /// (module doc § bug-log #212).
    #[tokio::test]
    async fn every_lab_chip_run_is_read_back_from_its_write_root() {
        use ui::lab::equity_loader::EquityLoadError;
        for (chip, _) in ui::lab::defaults::LAB_SEED_STRATEGIES {
            let tmp = tempfile::tempdir().expect("tempdir");
            let mut cfg = test_config(tmp.path());
            cfg.strategy = StrategyId(chip.into());
            cfg.pair = (Venue::Binance, Symbol::new("BTCUSDT"));
            cfg.range = EngDateRange::Last30d;
            let (_handle, cancel_rx) = backtest::cancel::cancellation_pair();
            let progress_tx = backtest::progress::ProgressSender::disabled();
            let result = backtest::engine::run_scenario(cfg, cancel_rx, progress_tx).await;

            if chip == "v2.5.tcn.weights" {
                let err = result
                    .err()
                    .unwrap_or_else(|| panic!("{chip}: without backtest/candle the run must fail"));
                assert!(
                    err.to_string().contains("requires --features candle"),
                    "{chip}: the declared outcome is the candle error, got: {err}"
                );
                assert_eq!(
                    std::fs::read_dir(tmp.path()).expect("read tmp").count(),
                    0,
                    "{chip}: a failed run writes nothing"
                );
                continue;
            }

            let report = result.unwrap_or_else(|e| panic!("{chip}: run_scenario failed: {e}"));
            let path = report
                .report_path
                .clone()
                .unwrap_or_else(|| panic!("{chip}: write_report=true must return a report_path"));
            let read_root = path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .expect("a <root>/<slug>/reports/<file> report path");
            assert_eq!(read_root, tmp.path(), "{chip}: write-root == read-root");
            let text = std::fs::read_to_string(&path).expect("read the written report");
            let ids = frontmatter_ids(&text);
            assert_eq!(
                ids.iter().map(String::as_str).collect::<Vec<_>>(),
                backtest::engine::report_strategy_id(chip)
                    .into_iter()
                    .collect::<Vec<_>>(),
                "{chip}: the written `id:` must be the one report_strategy_id names"
            );
            let coins = stated_coins(&text);
            assert!(
                coins.iter().any(|c| c == "BTCUSDT"),
                "{chip}: the BTCUSDT run states BTCUSDT; got {coins:?}"
            );
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("utf-8 report name")
                .to_string();

            let mut cache = EquityCache::new();
            for (_, coin) in ui::lab::universe::XRP_FIRST_UNIVERSE {
                let tuple = LabTuple {
                    strategy: SmolStr::new(chip),
                    symbol: SmolStr::new(coin),
                    range: DateRange::Preset(Preset::Last30d),
                    source: LabDataSource::Synthetic,
                };
                let got = cache.get_or_load(&tuple, read_root);
                if coins.iter().any(|c| c == coin) {
                    let series =
                        got.unwrap_or_else(|e| panic!("{chip} × {coin}: must read back: {e}"));
                    assert_eq!(
                        series.source_report.as_str(),
                        name,
                        "{chip} × {coin}: the cold read must be the written report"
                    );
                } else {
                    assert!(
                        matches!(got, Err(EquityLoadError::NoReport { .. })),
                        "{chip} × {coin}: the report does not state the coin, so NoReport; \
                         got {got:?}"
                    );
                }
            }
        }
    }
}
