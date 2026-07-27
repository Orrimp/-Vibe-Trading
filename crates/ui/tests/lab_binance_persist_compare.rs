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
//! (`data/binance/`) must be present AT THE WORKSPACE ROOT. Skip policy
//! (review patch 1): the test pins the process cwd to the workspace root
//! (cargo runs ui test binaries with cwd=`crates/ui/`) and probes
//! `data/binance/REVISION.toml` — SKIP only when the probe is genuinely
//! absent; probe present + loader error = hard FAIL (the old cwd-relative
//! any-Err→skip made the body vacuous on every machine).

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

/// Resolve the workspace root (`crates/ui` → `crates` → root) and pin the
/// process cwd there (review patch 1 — the loader's corpus root is
/// cwd-relative and cargo runs ui test binaries with cwd=`crates/ui/`).
/// Per-test `set_current_dir` to the SAME dir is the established benign
/// pattern (`crates/backtest/tests/binance_cache_dispatch.rs`).
fn pin_cwd_to_workspace_root() -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("locate workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("set_current_dir({root:?}): {e}"));
    root
}

/// Load real Binance bars for `BTCUSDT × H1_2024` via the production seam.
/// Returns `None` (skip) ONLY when the workspace-root probe
/// `data/binance/REVISION.toml` is genuinely absent; probe present + loader
/// error PANICS (review patch 1 — no more any-Err→skip vacuity).
fn try_load_binance_bars() -> Option<Vec<trading_core::Bar>> {
    let root = pin_cwd_to_workspace_root();
    if !root.join("data/binance/REVISION.toml").is_file() {
        eprintln!(
            "[skip] data/binance/REVISION.toml not present at the workspace root \
             ({}) — the gitignored pinned corpus is absent on this machine; \
             persist/Compare round-trip skipped.",
            root.display()
        );
        return None;
    }
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
        Err(e) => panic!(
            "corpus PRESENT (data/binance/REVISION.toml exists under {}) but the \
             Binance loader failed: {e} — hard FAIL, not a skip (review patch 1).",
            root.display()
        ),
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
        dvol_override: None,
        macro_regime_series: None,
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
    // Naming per engine.rs `write_equity_companion_csv`: `<md-stem>-equity.csv`
    // (the original `.with_extension("csv")` expectation was a day-1 latent bug,
    // masked while this test's body skipped vacuously — 2026-07-26 review P1).
    let md_stem = report_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("report filename is utf-8");
    let csv_path = report_path.with_file_name(format!("{md_stem}-equity.csv"));
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
    // Source-keyed tuple (review D1): the engine wrote `data_source: binance`
    // into the report frontmatter, so the Binance-sourced tuple resolves it.
    let tuple = LabTuple {
        strategy: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        range: DateRange::Preset(Preset::H1_2024),
        source: LabDataSource::BinanceCache,
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
