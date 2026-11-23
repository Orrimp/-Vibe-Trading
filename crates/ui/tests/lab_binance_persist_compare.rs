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
//! ## Gating
//!
//! `#[cfg(all(feature = "live", feature = "binance"))]`, both default
//! features.
//!
//! The pinned corpus (`data/binance/`) is COMMITTED IN FULL (`.gitignore`
//! `!/data/binance/**`, "TRACKED IN FULL since 2026-09-27"), so an absent
//! BTCUSDT 2024 H1 month is a broken checkout and FAILS (bug-log `#66`); no
//! skip path remains (story 2-63 Wave E3, AC3). The loader's corpus root is
//! cwd-relative and cargo runs ui test binaries with cwd = `crates/ui/`, so the
//! real test pins the cwd to the workspace root; the absent-corpus twin never
//! changes the cwd (it fails at M1, [`assert_binance_corpus`], before any load).
//!
//! Shape (story 2-63 Wave E2 / E3): the round trip is the root-parameterized
//! `persist_body`; the real test calls it at the workspace root, and
//! `persist_absent_corpus_is_a_failure_not_a_skip` calls the SAME body at an
//! empty temp dir under `#[should_panic(expected = "absent means a broken
//! checkout")]`.
//!
//! `persist_body` reads back through `ui::compare::cache::scan_spec_tree`.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in THIS file (`grep -c` of the OLD
//! text is 1), run with `cargo test -p ui --test lab_binance_persist_compare`,
//! then restored from a `cp -p` snapshot (`cmp`). The skip prefix NEW inserts
//! is `if !corpus_data_present(root) { return; }`, the shape Wave E3 removed.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E3-BP-P1 | `assert_binance_corpus` | `assert!(` followed by `corpus_data_present(root),` | the skip prefix above the assert | `persist_absent_corpus_is_a_failure_not_a_skip`: "panic did not contain expected string" (the load then fails at the twin's cwd `crates/ui`) or "test did not panic as expected" (the real test already pinned the cwd to the workspace root and the load succeeds); scheduling decides which, both are red |
//! | E3-BP-P2 | `persist_body` | `let bars = load_binance_bars(root);` | the skip prefix above it | `persist_absent_corpus_is_a_failure_not_a_skip`: "test did not panic as expected" |
//!
//! GREEN in both probes: `binance_run_persists_and_round_trips_through_compare`.

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

/// Probe for the DATA this test loads — never for `REVISION.toml` (a probe on
/// the manifest alone was true while every month beneath it was absent, which
/// is what once turned the Linux and Windows UI legs red invisibly).
///
/// It backs the M1 assert ([`assert_binance_corpus`]), not a skip: the corpus
/// is committed (module doc § Gating).
///
/// The months named here are the ones `ENG_RANGE` (H1 **2024**) actually loads. They are NOT the same
/// across these tests — this one reads 2024, its siblings read another year —
/// so this probe is deliberately per-file rather than a shared helper that would
/// be right for one caller and wrong for the next.
fn corpus_data_present(root: &std::path::Path) -> bool {
    (1..=6).all(|m| {
        root.join(format!("data/binance/BTCUSDT/2024/{m:02}.parquet"))
            .is_file()
    })
}

/// M1, the committed-corpus precondition (bug-log `#66`): PANIC when the six
/// 2024 H1 BTCUSDT parquets are missing under `root`.
fn assert_binance_corpus(root: &std::path::Path) {
    assert!(
        corpus_data_present(root),
        "the committed Binance corpus (data/binance/BTCUSDT/2024/01..06.parquet) must ship under {} — absent means a broken checkout, not a skip (bug-log #66)",
        root.display()
    );
}

/// 2024 H1 — `binance_range_to_ms_pair(H1_2024)` resolves to 2024-01..2024-07
/// UTC, on-disk for BTCUSDT in the pinned corpus.
const ENG_RANGE: EngDateRange = EngDateRange::H1_2024;

/// Resolve the workspace root (`crates/ui` → `crates` → root) and pin the
/// process cwd there (review patch 1 — the loader's corpus root is
/// cwd-relative and cargo runs ui test binaries with cwd=`crates/ui/`).
/// Per-test `set_current_dir` to the SAME dir is the established benign
/// pattern (`crates/backtest/tests/binance_cache_dispatch.rs`). The
/// absent-corpus twin never calls it.
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
///
/// M1 first: PANICS when the committed corpus is missing under `root` (a
/// broken checkout). The load reads the cwd the caller pinned; with the corpus
/// present, a loader error also PANICS (review patch 1, bug-log `#66`).
fn load_binance_bars(root: &std::path::Path) -> Vec<trading_core::Bar> {
    assert_binance_corpus(root);
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
        Ok((bars, _sha)) => bars,
        Err(e) => panic!(
            "the committed Binance corpus is present under {} but the loader failed: {e} — a regression, not a missing corpus (bug-log #66)",
            root.display()
        ),
    }
}

/// **AC5 — the full lab-run-save-compare chain round-trips a Binance run.**
#[test]
fn binance_run_persists_and_round_trips_through_compare() {
    persist_body(&pin_cwd_to_workspace_root());
}

/// The twin: the SAME round-trip body at an empty root must FAIL at M1, never
/// return early (bug-log `#66`; T-T4 E3-BP-P2).
#[test]
#[should_panic(expected = "absent means a broken checkout")]
fn persist_absent_corpus_is_a_failure_not_a_skip() {
    let empty = tempfile::tempdir().expect("a temp dir to stand in for a broken checkout");
    persist_body(empty.path());
}

/// The persist + Compare round trip, parameterized by the corpus root (M1
/// runs at `root`; the load reads the cwd the real test pinned).
fn persist_body(root: &std::path::Path) {
    let bars = load_binance_bars(root);

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
