//! No-op-source divergence guard + loader gate for the Binance Lab path
//! (simple-strategies-realdata T-C1 / T-A3 — AC3 / AC4).
//!
//! ## Why this file exists (the v3-vol-overlay-noop analog)
//!
//! CLAUDE.md's baseline-equity-divergence gate is scoped to strategy overlays /
//! sizing modifiers — N/A here (this feature adds NO overlay, NO sizing change,
//! NO new decision variable; only the *bars* differ). BUT the precise failure
//! mode that gate exists to catch has a direct analog: a `BinanceCache` toggle
//! that is WIRED but silently feeds synthetic bars. The operator would believe
//! they are testing real BTC while seeing a random walk — the exact
//! "computed-but-not-applied" class the v3 precedent burned us on.
//!
//! **The guard (AC4):** run `v0.sma × BTCUSDT × 2023` on Binance bars and on
//! synthetic bars with the SAME `(strategy, symbol, range, seed)`, and assert
//! the two equity curves DIVERGE by ≥ epsilon — proving the real parquet bytes
//! drove the result, not a silent synthetic fallback. The loader's
//! no-silent-fallback rule (T-A3 — `Err` on miss, never synthesize) is the
//! design-side half; THIS test is the behavioural half.
//!
//! ## Gating
//!
//! `#[cfg(all(feature = "live", feature = "binance"))]` — needs the engine
//! (`live`) + the Binance loader (`binance`). The pinned corpus
//! (`data/binance/`, revision `3a8b96c4…`) must be present on disk AT THE
//! WORKSPACE ROOT. Skip policy (review patch 1): the tests pin the process
//! cwd to the workspace root (cargo runs ui test binaries with
//! cwd=`crates/ui/`) and probe `data/binance/REVISION.toml` — SKIP only when
//! the probe is genuinely absent (CI without the gitignored corpus); when the
//! probe exists, any loader error FAILS loudly. The old cwd-relative
//! any-Err→skip made every real-data body here vacuous on every machine.

#![cfg(all(feature = "live", feature = "binance"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use backtest::engine::{DateRange, ScenarioConfig, ScenarioDataSource, run_scenario};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::defaults::LAB_DEFAULT_SEED;
use ui::lab::runner::{DefaultLabBinanceBarSource, LabBarSource, LabRunConfig};

/// 2023 H1 as a `Custom` ms range: 2023-01-01 .. 2023-07-01 UTC. The pinned
/// corpus covers 2023-01..2024-12 at 1h, so this window resolves to on-disk
/// months for BTCUSDT.
const RANGE_2023_H1: DateRange = DateRange::Custom {
    start_ms: 1_672_531_200_000, // 2023-01-01T00:00:00Z
    end_ms: 1_688_169_600_000,   // 2023-07-01T00:00:00Z
};

/// Build the Lab run config the loader consumes (BTCUSDT, Binance source).
fn binance_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        // range_label is informational for the loader; the actual window comes
        // from the DateRange passed to preload (RANGE_2023_H1).
        range_label: SmolStr::new("Custom:2023-01-01:2023-07-01"),
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: ui::lab::state::LabDataSource::BinanceCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

/// Build the engine `ScenarioConfig` for `v0.sma × BTCUSDT`, parameterized by
/// data source + optional pre-loaded bars. Same seed + range for both arms so
/// the ONLY difference is the bar source (the divergence isolation).
fn scenario_cfg(
    data_source: ScenarioDataSource,
    bars_override: Option<Vec<trading_core::Bar>>,
) -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: RANGE_2023_H1,
        params: None,
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source,
        bars_override,
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
        dvol_override: None,
        macro_regime_series: None,
    }
}

/// Resolve the workspace root (`crates/ui` → `crates` → root) and pin the
/// process cwd there. The loader's corpus root `data/binance` is cwd-relative,
/// and cargo runs ui test binaries with cwd = `crates/ui/` — which made every
/// guard body in this file skip on EVERY machine (review patch 1, empirically
/// proven 2026-07-26 with the corpus present). Tests in one binary run
/// multi-threaded; per-test `set_current_dir` to the SAME directory is the
/// established benign pattern (`crates/backtest/tests/binance_cache_dispatch.rs`).
fn pin_cwd_to_workspace_root() -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("locate workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("set_current_dir({root:?}): {e}"));
    root
}

/// Corpus-presence probe at the WORKSPACE root. Skipping is legitimate ONLY
/// when this is genuinely absent; with the probe present, any loader error is
/// a hard FAIL (the old any-Err→skip made the tests vacuous).
fn corpus_probe(root: &std::path::Path) -> bool {
    root.join("data/binance/REVISION.toml").is_file()
}

/// Load real Binance bars via the PRODUCTION `DefaultLabBinanceBarSource`
/// (the trait seam), routed exactly as `spawn_lab_run` routes it. Returns
/// `None` (test skips) ONLY when the workspace-root probe
/// `data/binance/REVISION.toml` is genuinely absent; when the probe exists
/// but the loader errors, this PANICS — that is a real regression, not a
/// missing corpus (review patch 1).
fn try_load_binance_bars() -> Option<(Vec<trading_core::Bar>, SmolStr)> {
    let root = pin_cwd_to_workspace_root();
    if !corpus_probe(&root) {
        eprintln!(
            "[skip] data/binance/REVISION.toml not present at the workspace root \
             ({}) — the gitignored pinned corpus is absent on this machine; \
             divergence test skipped. (The no-silent-fallback contract is still \
             proven by loader_missing_corpus_returns_typed_err_not_synthetic.)",
            root.display()
        );
        return None;
    }
    let cfg = binance_cfg();
    let src = DefaultLabBinanceBarSource;
    // `LabBarSource::preload` is an async fn returning a boxed future; drive it
    // on a throwaway current-thread runtime (the loader is a pure parquet read,
    // no reactor-sensitive spawn_blocking — see preload_binance_bars docs).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    let result = rt.block_on(async { src.preload(&cfg, &RANGE_2023_H1).await });
    match result {
        Ok((bars, sha)) => Some((bars, sha)),
        Err(e) => panic!(
            "corpus PRESENT (data/binance/REVISION.toml exists under {}) but the \
             Binance loader failed: {e} — hard FAIL, not a skip (review patch 1: \
             any-Err→skip left this guard vacuous on every machine).",
            root.display()
        ),
    }
}

/// Final equity (USDT) of a `RunReport` — the headline number a divergence is
/// measured on. Empty series → 0 (the test asserts non-empty separately).
fn final_equity(report: &backtest::RunReport) -> Decimal {
    report
        .equity_series
        .last()
        .map_or(Decimal::ZERO, |(_, money)| money.amount())
}

/// **AC3 — the loader gate (revision-asserted, non-empty hourly bars).**
///
/// The production Binance loader, when the corpus is present, returns
/// non-empty HOURLY bars for `BTCUSDT × 2023 H1` AND carries the pinned
/// aggregate revision SHA (which it asserted on load). All `BTCUSDT` bars are
/// in range and stamped on the Binance venue.
#[test]
fn loader_returns_nonempty_hourly_bars_with_revision_sha() {
    let Some((bars, sha)) = try_load_binance_bars() else {
        return; // corpus absent — skip (see helper log)
    };

    assert!(
        !bars.is_empty(),
        "Binance loader must return non-empty bars for BTCUSDT × 2023 H1"
    );
    assert!(
        !sha.is_empty(),
        "loader must carry the pinned aggregate revision SHA (forensics)"
    );
    // Review patch 3 through the seam: the loader pin-asserts, so the carried
    // SHA IS the pin (mirrors the CLI literal in crates/backtest/src/main.rs).
    assert_eq!(
        sha.as_str(),
        "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7",
        "loader-carried SHA must equal the pinned corpus revision (AC3)"
    );
    // Every bar is in the requested window and on the Binance venue.
    for b in &bars {
        let ts_ms = b.open_ts.unix_millis();
        assert!(
            (1_672_531_200_000..1_688_169_600_000).contains(&ts_ms),
            "bar at {ts_ms} ms is outside the requested 2023-H1 window"
        );
        assert_eq!(b.symbol, Symbol::new("BTCUSDT"), "wrong symbol in bars");
    }
    // Hourly cadence sanity: consecutive open timestamps differ by ~1h
    // (3_600_000 ms). Check the median-ish gap on the first two bars to avoid
    // depending on the exact count.
    if bars.len() >= 2 {
        let gap_ms = bars[1].open_ts.unix_millis() - bars[0].open_ts.unix_millis();
        assert_eq!(
            gap_ms, 3_600_000,
            "Binance Lab bars must be HOURLY (Q-tf): first gap was {gap_ms} ms"
        );
    }
}

/// **AC4 — THE no-op-source divergence guard (the purpose-built gate).**
///
/// Run `v0.sma × BTCUSDT × 2023 H1` twice with the SAME `(strategy, symbol,
/// range, seed)`:
///   - arm A: real Binance bars injected via `bars_override` + `BinanceCache`,
///   - arm B: synthetic GBM bars (`bars_override = None` + `Synthetic`).
///
/// Assert the two equity curves DIVERGE — proving the real parquet bytes
/// reached the strategy, not a silent synthetic fallback. If the Binance path
/// ever silently synthesized, the two arms would be byte-identical and this
/// test would FAIL (exactly the v3-vol-overlay-noop failure signature).
#[test]
fn binance_run_diverges_from_synthetic_baseline() {
    let Some((binance_bars, _sha)) = try_load_binance_bars() else {
        return; // corpus absent — skip (see helper log)
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");

    // Arm A — real Binance bars.
    let (cancel_a, recv_a) = ui::lab::runner::cancellation_pair();
    let report_binance = rt
        .block_on(run_scenario(
            scenario_cfg(ScenarioDataSource::BinanceCache, Some(binance_bars.clone())),
            recv_a,
            ProgressSender::disabled(),
        ))
        .expect("Binance-sourced v0.sma run succeeds");
    drop(cancel_a);

    // Arm B — synthetic baseline, same seed + range.
    let (cancel_b, recv_b) = ui::lab::runner::cancellation_pair();
    let report_synth = rt
        .block_on(run_scenario(
            scenario_cfg(ScenarioDataSource::Synthetic, None),
            recv_b,
            ProgressSender::disabled(),
        ))
        .expect("synthetic v0.sma baseline run succeeds");
    drop(cancel_b);

    // Both runs produced equity.
    assert!(
        !report_binance.equity_series.is_empty(),
        "Binance run must produce a non-empty equity series"
    );
    assert!(
        !report_synth.equity_series.is_empty(),
        "synthetic baseline must produce a non-empty equity series"
    );

    // (The report's "binance" data_source label is compile-enforced in the
    // engine's data_source_str match and verified at the body layer by the
    // persist/Compare round-trip test `lab_binance_persist_compare`.)

    // ── THE divergence assertion (AC4) ────────────────────────────────────────
    // The Binance arm runs on ~hourly 2023-H1 BTC bars; the synthetic arm on a
    // fixed-seed GBM series. Two independent signals must differ:
    //   (1) the equity SERIES are not element-by-element identical, and
    //   (2) the final equity differs by a non-trivial epsilon.
    // A silent synthetic fallback in the Binance arm would make BOTH hold with
    // equality → this test fails (the no-op-source trap is caught).

    let eq_binance = final_equity(&report_binance);
    let eq_synth = final_equity(&report_synth);
    let delta = (eq_binance - eq_synth).abs();

    // Epsilon: ≥ 1 USDT on a ~10_000 USDT book is ~1 bp — above
    // rounding-noise scale, and trivially satisfied by two genuinely
    // different bar sources. (Real BTC 2023-H1 vs a GBM random walk diverge by
    // orders of magnitude more; 1 USDT is a deliberately conservative floor.)
    let epsilon = Decimal::ONE;
    assert!(
        delta >= epsilon,
        "NO-OP-SOURCE GUARD FAILED: Binance final equity ({eq_binance}) and \
         synthetic final equity ({eq_synth}) differ by only {delta} (< {epsilon}). \
         The Binance toggle may be silently feeding synthetic bars — the operator \
         would see a random walk while believing they test real BTC (the \
         v3-vol-overlay-noop failure class)."
    );

    // Series-level proof: the two equity curves are not identical sequences.
    let identical = report_binance.equity_series.len() == report_synth.equity_series.len()
        && report_binance
            .equity_series
            .iter()
            .zip(report_synth.equity_series.iter())
            .all(|((_, a), (_, b))| a.amount() == b.amount());
    assert!(
        !identical,
        "NO-OP-SOURCE GUARD FAILED: the Binance and synthetic equity SERIES are \
         element-by-element identical — the real parquet bytes did not reach the \
         strategy (silent synthetic fallback)."
    );
}

/// **AC4 design-side half — the loader NEVER synthesizes on miss.**
///
/// Point the SAME loader logic at a symbol that does not exist in the corpus
/// (`ZZZUSDT`) and assert it returns a typed `Err` with a re-fetch hint — NOT
/// a synthetic-bars `Ok`. This is the no-silent-fallback contract that makes
/// the divergence guard above trustworthy: a miss is loud, never a random walk.
#[test]
fn loader_missing_corpus_returns_typed_err_not_synthetic() {
    // cwd-pin so the loader resolves the same workspace-root corpus the other
    // tests use (with the corpus present this exercises the missing-SYMBOL
    // path; without it, the missing-manifest revision path — both are typed
    // Errs and both satisfy the assertions below).
    let _root = pin_cwd_to_workspace_root();
    // A symbol guaranteed absent from the 10-symbol pinned corpus.
    let cfg = LabRunConfig {
        symbol: SmolStr::new("ZZZUSDT"),
        ..binance_cfg()
    };
    let src = DefaultLabBinanceBarSource;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    let result = rt.block_on(async { src.preload(&cfg, &RANGE_2023_H1).await });

    match result {
        Ok((bars, _sha)) => {
            panic!(
                "NO-SILENT-FALLBACK VIOLATED: loader returned Ok with {} bars for a \
                 missing symbol (ZZZUSDT) — it must return a typed cache-miss Err, \
                 NEVER synthesize bars.",
                bars.len()
            );
        }
        Err(e) => {
            // The message must be the operator-friendly cache-miss notice (or
            // the revision error if the corpus itself is absent), and must
            // mention the symbol + a re-fetch path — never an internal panic
            // string. Both acceptable errors point the operator at the fetch tool.
            let msg = e.as_str();
            assert!(
                msg.contains("ZZZUSDT") || msg.contains("revision"),
                "cache-miss Err must name the symbol or be a revision error; got: {msg}"
            );
            assert!(
                !msg.is_empty(),
                "cache-miss Err must carry an operator-facing message"
            );
        }
    }
}
