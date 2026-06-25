//! Persist + Compare round-trip for a Binance Lab run (simple-strategies-realdata
//! T-C2 — AC5, the close-out AC).
//!
//! ## What this proves (the "free win" from lab-run-save-compare)
//!
//! `lab-run-save-compare` (ADR-0055) wired persist/compare/overlay at the
//! `run_scenario` dispatch boundary, so a Binance-sourced run produces a
//! `RunReport` of IDENTICAL shape and inherits the whole chain with ZERO new
//! persist/compare code. This test asserts the shipped chain works end-to-end
//! for a Binance run:
//!
//!   (i)   `run_scenario(write_report=true, reports_dir=lab-runs/)` writes the
//!         `.md` report (+ companion equity CSV) under `lab-runs/<slug>/reports/`,
//!   (ii)  `EquityCache::get_or_load` parses the equity series element-by-element
//!         equal to the in-memory series (the H3 round-trip — holds because the
//!         body is deterministic given fixed parquet + seed, R6), and
//!   (iii) `compare::scan_spec_tree` builds a `CachedCell` for the run with a
//!         loadable per-bar equity series (KPIs + overlay-ready).
//!
//! ## Gating + skip
//!
//! `#[cfg(all(feature = "live", feature = "binance"))]`. The pinned corpus
//! (`data/binance/`) must be present; if absent the run's loader returns a
//! typed `Err` and the test SKIPS (the gitignored corpus may be missing in CI).

#![cfg(all(feature = "live", feature = "binance"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use backtest::engine::{
    DateRange as EngDateRange, ScenarioConfig, ScenarioDataSource, run_scenario,
};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};
use ui::compare::cache::scan_spec_tree;
use ui::lab::defaults::LAB_DEFAULT_SEED;
use ui::lab::equity_loader::{EquityCache, LabTuple};
use ui::lab::runner::{DefaultLabBinanceBarSource, LabBarSource, LabRunConfig};
use ui::lab::state::{DateRange, LabDataSource, Preset};

/// 2024 H1 — `binance_range_to_ms_pair(H1_2024)` resolves to 2024-01..2024-07
/// UTC, on-disk for BTCUSDT in the pinned corpus.
const ENG_RANGE: EngDateRange = EngDateRange::H1_2024;

/// Load real Binance bars for `BTCUSDT × H1_2024` via the production seam.
/// Returns `None` (skip) if the gitignored corpus is absent.
fn try_load_binance_bars() -> Option<Vec<trading_core::Bar>> {
    let cfg = LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range_label: SmolStr::new("H1_2024"),
        seed: LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::BinanceCache,
        sma_fast_len: None,
        sma_slow_len: None,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    match rt.block_on(async { DefaultLabBinanceBarSource.preload(&cfg, &ENG_RANGE).await }) {
        Ok((bars, _sha)) => Some(bars),
        Err(e) => {
            eprintln!("[skip] Binance corpus absent ({e}); persist/Compare round-trip skipped");
            None
        }
    }
}

/// **AC5 — the full lab-run-save-compare chain round-trips a Binance run.**
#[test]
fn binance_run_persists_and_round_trips_through_compare() {
    let Some(bars) = try_load_binance_bars() else {
        return; // corpus absent — skip
    };

    // Write-root == read-root: a tempdir `lab-runs/`-shaped tree (ADR-0055 § D6).
    let tmp = tempfile::tempdir().expect("tempdir");
    let reports_dir = tmp.path().to_path_buf();

    let scenario = ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: ENG_RANGE,
        params: None,
        seed: LAB_DEFAULT_SEED,
        write_report: true,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: Some(reports_dir.clone()),
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds");
    let (_cancel, recv) = ui::lab::runner::cancellation_pair();
    let report = rt
        .block_on(run_scenario(scenario, recv, ProgressSender::disabled()))
        .expect("Binance v0.sma run (write_report) succeeds");

    // In-memory equity, projected for the cache comparison.
    let in_memory: Vec<(i64, Decimal)> = report
        .equity_series
        .iter()
        .map(|(ts, money)| (ts.unix_millis(), money.amount()))
        .collect();
    assert!(
        !in_memory.is_empty(),
        "Binance run must produce a non-empty in-memory equity series"
    );

    // (i) the .md report (+ companion CSV) was written under lab-runs/.
    let report_path = report
        .report_path
        .as_ref()
        .expect("write_report=true must yield a report_path for the Binance run");
    assert!(
        report_path.exists(),
        "the written .md report must exist on disk"
    );
    assert!(
        report_path.extension().and_then(|e| e.to_str()) == Some("md"),
        "the written report must be a .md file: {report_path:?}"
    );
    // Companion equity CSV lives next to the .md (H3 fidelity fix, ADR-0055).
    let csv_path = report_path.with_extension("csv");
    assert!(
        csv_path.exists(),
        "the companion equity CSV must be written next to the report: {csv_path:?}"
    );

    // Derive read-root = report_path.parent().parent().parent() (== reports_dir).
    let read_root = report_path
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("report_path has a lab-runs/<slug>/reports/ shape");

    // (ii) EquityCache round-trip — element-by-element equality (H3 for Binance).
    let tuple = LabTuple {
        strategy: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        range: DateRange::Preset(Preset::H1_2024),
    };
    let mut cache = EquityCache::new();
    let cached = cache
        .get_or_load(&tuple, read_root)
        .expect("EquityCache must find + parse the just-written Binance report");
    assert_eq!(
        in_memory.len(),
        cached.samples.len(),
        "Binance round-trip: equity series length mismatch (in-memory={}, cached={})",
        in_memory.len(),
        cached.samples.len()
    );
    for (i, ((ts_m, eq_m), (ts_c, eq_c))) in in_memory.iter().zip(cached.samples.iter()).enumerate()
    {
        assert_eq!(ts_m, ts_c, "ts mismatch at {i}: {ts_m} vs {ts_c}");
        assert_eq!(eq_m, eq_c, "equity mismatch at {i}: {eq_m} vs {eq_c}");
    }

    // (iii) Compare scan builds a cell with a loadable per-bar equity series.
    let cells = scan_spec_tree(read_root);
    assert!(
        !cells.is_empty(),
        "compare::scan_spec_tree must surface ≥1 cell for the Binance run"
    );
    let has_loadable_equity = cells
        .values()
        .any(|cell| cell.equity_series_ts.len() == in_memory.len());
    assert!(
        has_loadable_equity,
        "Compare must build a CachedCell whose equity_series_ts round-trips the \
         Binance run's {} per-bar points (KPIs + overlay-ready)",
        in_memory.len()
    );

    eprintln!(
        "AC5: PASS — Binance run persisted (.md + .csv), EquityCache round-trip \
         held over {} points, Compare cell built.",
        in_memory.len()
    );
}
