//! Block-bootstrap overfit / robustness guard for the 4 simple strategies
//! on the 2024 AVAX and DOT down-market cells (+ AVAX 2023 up-market control).
//!
//! # What this tests
//!
//! The 2026-06-14 real-data survey found that in the only 2 bear-market cells
//! (AVAX·2024: B&H −8.2% → SMA +5.0%, MACD +6.1%; DOT·2024: B&H −19.6% → SMA +6.4%)
//! trend-following protected capital while passive lost. The survey itself notes
//! that "two data points is suggestive, not statistically conclusive."
//!
//! This harness answers: **is that protection a repeatable property of the
//! strategy, or an artefact of the one exact price ordering?**
//!
//! It generates N=500 stationary-bootstrap resamples of each real year's bars
//! (Politis–White auto block length), runs each of the 4 survey strategies on
//! every resample via `run_scenario` + `bars_override`, reduces the 500
//! per-path equity curves to Sharpe p5/p25/p50/p75/p95, prob-of-loss, and
//! max-DD tail, and scores against the pre-registered § 0 decision rule:
//!
//! - **ROBUST**: `sharpe.p5 ≥ 0.5` AND `prob_loss ≤ 0.15` AND `max_dd_tail_p95 ≤ 0.50`
//! - **MARGINAL**: anything else that is not FRAGILE
//! - **FRAGILE**: `sharpe.p5 < 0` OR `prob_loss > 0.35` OR `max_dd_tail_p95 > 0.70`
//!
//!   Composite = worst band; **p5 Sharpe < 0 ⇒ FRAGILE**.
//!
//! # Negative control (AC-OG.4 / D-OG.6 miscalibration tripwire)
//!
//! RSI and BBands are the no-edge mean-reverters. Per the survey they have no
//! edge anywhere. They MUST score FRAGILE or MARGINAL on the down-market cells.
//! If RSI/BBands come back ROBUST, the harness is miscalibrated — escalate.
//!
//! # Determinism (AC-OG.3 / R-OG.7)
//!
//! Seeds per ADR-0051 D1:
//! - `ensemble_seed`: a DISTINCT `u64` per (strategy × cell) pair (see `ENSEMBLE_SEEDS`).
//! - `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`.
//! - `ScenarioConfig.seed`: the CONSTANT `SEED` (`0xC0FFEE…`) for every path
//!   (orthogonality — per-path variation lives only in the bootstrap `path_seed_j`,
//!   not the engine fill-tie-break seed). DO NOT vary this per path.
//!
//! Two consecutive `--nocapture` runs are byte-identical.
//!
//! # UN-ANCHORED (R-OG.8 / D-OG.4)
//!
//! This is a `#[ignore]` one-shot finding harness. It writes no report file and
//! adds NO `anchors.toml` row. The `--nocapture` stdout is the deliverable.
//! Determinism (fixed seeds) makes runs byte-reproducible without an anchor.
//!
//! # CLAUDE.md baseline-divergence e2e gate: N/A — D-OG.6
//!
//! The CLAUDE.md non-negotiable "every strategy overlay ships with a
//! baseline-equity-divergence e2e test" covers **no-op overlays** (a `scale`
//! computed but never applied). This harness introduces **no overlay and no
//! sizing modifier**. It is read-only analysis tooling: it runs the four
//! already-shipped survey strategy ids unchanged through the already-shipped
//! `run_scenario` engine over bootstrap-resampled bars, and reduces the output.
//! There is no new decision variable that could silently fail to wire.
//! The applicable correctness guards are AC-OG.3 (two-run byte-identical
//! determinism) + AC-OG.4 (RSI/BBands MUST land FRAGILE/MARGINAL), which form
//! the miscalibration tripwire equivalent to the divergence gate for analysis
//! tooling. Gate stated N/A on substance, not skipped (D-OG.6).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};

use backtest::cancel::cancellation_pair;
use backtest::engine::{DateRange, RunReport, ScenarioConfig, ScenarioDataSource};
use backtest::progress::ProgressSender;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};
use data::{BlockBootstrapPathGen, BlockLengthPolicy, MonteCarloPathGen};
use rust_decimal::Decimal;
use tokio_stream::StreamExt as _;
use trading_core::{Bar, StrategyId, Symbol, Timeframe, Venue};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Constant engine seed per ADR-0051 D1 orthogonality.
/// Per-path variation lives ONLY in `path_seed_j` (the bootstrap draw seed),
/// NOT in this engine fill-tie-break seed. DO NOT vary this per path.
const SEED: [u8; 32] = [
    0xC0, 0xFF, 0xEE, 0x01, 0x02, 0x03, 0x04, 0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

/// Number of bootstrap paths per ensemble (C1/Q-RH-1 ratified default).
/// The § 0 decision bands were calibrated at N=500 — DO NOT reduce.
const N_PATHS: usize = 500;

/// Strategy ids (survey verbatim).
const STRATS: &[(&str, &str)] = &[
    ("v0.sma", "SMA 20/50"),
    ("v0.5.macd", "MACD"),
    ("v0.5.rsi", "RSI"),
    ("v0.5.bbands", "BBands"),
];

/// UTC year-boundary timestamps in milliseconds.
const Y2023_START: u64 = 1_672_531_200_000;
const Y2023_END: u64 = 1_704_067_200_000;
const Y2024_START: u64 = 1_704_067_200_000;
const Y2024_END: u64 = 1_735_689_600_000;

/// Distinct ensemble seeds per (strategy × cell) per ADR-0051 D1.
///
/// Order: one row per strategy (SMA, MACD, RSI, BBands), one column per cell
/// (AVAX·2024, DOT·2024, AVAX·2023).
/// Values derived from a base `0x00C0_FFEE_0000_0000u64` incremented by cell-index
/// then strategy-index * 0x100, ensuring full orthogonality.
const ENSEMBLE_SEEDS: [[u64; 3]; 4] = [
    // SMA:   AVAX·2024,                  DOT·2024,                   AVAX·2023
    [
        0x00C0_FFEE_0000_0000,
        0x00C0_FFEE_0000_0001,
        0x00C0_FFEE_0000_0002,
    ],
    // MACD
    [
        0x00C0_FFEE_0000_0100,
        0x00C0_FFEE_0000_0101,
        0x00C0_FFEE_0000_0102,
    ],
    // RSI
    [
        0x00C0_FFEE_0000_0200,
        0x00C0_FFEE_0000_0201,
        0x00C0_FFEE_0000_0202,
    ],
    // BBands
    [
        0x00C0_FFEE_0000_0300,
        0x00C0_FFEE_0000_0301,
        0x00C0_FFEE_0000_0302,
    ],
];

// ── Helpers ───────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Load hourly bars for `sym` in `[start_ms, end_ms)` from the Binance corpus.
/// Returns an empty `Vec` when the parquet files are absent — caller SKIPs.
/// Copied verbatim from `realdata_simple_strategy_survey.rs:55`.
async fn load_year_bars(root: &Path, sym: &Symbol, start_ms: u64, end_ms: u64) -> Vec<Bar> {
    use data::source::MarketDataSource as _;
    let feed = data::ReplayFeed::new(root.join("data/binance"), true);
    let Ok(mut stream) = feed.subscribe_bars(sym.clone(), Timeframe::OneHour).await else {
        return Vec::new();
    };
    let mut bars = Vec::new();
    while let Some(Ok(b)) = stream.next().await {
        let ts = b.open_ts.unix_millis() as u64;
        if ts >= start_ms && ts < end_ms {
            bars.push(b);
        } else if ts >= end_ms {
            break;
        }
    }
    bars
}

/// Run the strategy on one bootstrap path's bars.
///
/// Returns `None` only if `run_scenario` returns an error (logged to stderr).
/// KEEPS the full `RunReport` (unlike the survey which discards it) — we need
/// `equity_series` to compute per-path metrics.
async fn run_one_path(sym: &Symbol, strat: &str, path_bars: Vec<Bar>) -> Option<RunReport> {
    let cfg = ScenarioConfig {
        strategy: StrategyId(strat.into()),
        pair: (Venue::Binance, sym.clone()),
        range: DateRange::Last30d, // ignored when bars_override is Some
        params: None,
        seed: SEED, // CONSTANT across all paths (ADR-0051 D1 orthogonality)
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(path_bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    };
    let (_h, cancel_rx) = cancellation_pair();
    match backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled()).await {
        Ok(report) => Some(report),
        Err(e) => {
            eprintln!("run_one_path({sym}/{strat}) error: {e}");
            None
        }
    }
}

/// Extract `PathMetrics` from a completed `RunReport`.
///
/// Maps `report.equity_series → Vec<Decimal>` (via `.amount()` on each
/// `Money<Usdt>`) and calls the shipped `compute_*` helpers. Money stays
/// `Decimal`/`Money<Usdt>` until the `compute_*` boundary (R-OG.6).
fn path_metrics_from_report(report: &RunReport) -> PathMetrics {
    let equity: Vec<Decimal> = report
        .equity_series
        .iter()
        .map(|(_, m)| m.amount())
        .collect();

    PathMetrics {
        sharpe: compute_sharpe_hourly(&equity),
        sortino: compute_sortino_hourly(&equity),
        calmar: compute_calmar(&equity),
        max_drawdown: compute_max_drawdown_f64(&equity),
        total_return: compute_total_return(&equity),
        final_equity: report.kpis.final_equity.amount(),
        initial_equity: report.kpis.initial_equity.amount(),
    }
}

/// Score an ensemble's `DistributionSummary` against the frozen § 0 bands.
///
/// Decision rule (pre-registered, applied AS-IS — do NOT re-derive or soften):
/// - **FRAGILE**: `sharpe.p5 < 0` OR `prob_loss > 0.35` OR `max_dd_tail_p95 > 0.70`
/// - **ROBUST**: `sharpe.p5 ≥ 0.5` AND `prob_loss ≤ 0.15` AND `max_dd_tail_p95 ≤ 0.50`
/// - **MARGINAL**: everything else
///
/// Composite = worst band (§ 0 step 3).
fn score_verdict(s: &DistributionSummary) -> &'static str {
    // Any FRAGILE trigger → FRAGILE (§ 0 composite = worst band).
    if s.sharpe.p5 < 0.0 || s.prob_loss > 0.35 || s.max_dd_tail_p95 > 0.70 {
        return "FRAGILE";
    }
    // All ROBUST conditions must be met simultaneously.
    if s.sharpe.p5 >= 0.5 && s.prob_loss <= 0.15 && s.max_dd_tail_p95 <= 0.50 {
        return "ROBUST";
    }
    "MARGINAL"
}

/// Run one full ensemble: N=500 bootstrap paths for `(sym, year, strat)`.
///
/// - Loads the real year bars ONCE from disk.
/// - Constructs ONE `BlockBootstrapPathGen` (single-symbol universe,
///   `BlockLengthPolicy::Auto`).
/// - Loops `j in 0..N_PATHS`, deriving `path_seed_j` per ADR-0051 D1.
/// - Collects `PathMetrics` in index order (j=0..N ascending, as required by
///   `DistributionSummary::from_path_metrics`).
/// - Calls `DistributionSummary::from_path_metrics` on the collected slice.
///
/// Returns `None` when the corpus is absent or too short (< 100 bars).
async fn run_ensemble(
    root: &Path,
    sym: &Symbol,
    year: (u64, u64),
    strat: &str,
    ensemble_seed: u64,
) -> Option<DistributionSummary> {
    let (start_ms, end_ms) = year;
    let real_bars = load_year_bars(root, sym, start_ms, end_ms).await;
    if real_bars.len() < 100 {
        eprintln!(
            "SKIP ensemble {}/{strat}: only {} bars",
            sym,
            real_bars.len()
        );
        return None;
    }

    let start_price = real_bars[0].close.get();
    let n_bars = real_bars.len();

    // Build the bootstrap generator (single-symbol universe, Auto block length).
    // Single-symbol mode is a directly-tested path (bootstrap.rs fp_c1_2/3/4,
    // auto_block_length_is_some — all use 1-entry universes).
    let bootstrap_gen =
        BlockBootstrapPathGen::new(vec![(sym.clone(), real_bars)], BlockLengthPolicy::Auto)
            .expect("BlockBootstrapPathGen::new must not fail for a valid single-symbol series");

    // ── Ensemble loop: j = 0..N_PATHS ────────────────────────────────────────
    // path_seed_j per ADR-0051 D1: ensemble_seed.wrapping_add(j * 0x9E37_79B9).
    // Collect in index order (ADR-0051 D2 mandate — do NOT sort before calling
    // DistributionSummary::from_path_metrics).
    let mut metrics: Vec<PathMetrics> = Vec::with_capacity(N_PATHS);

    for j in 0..N_PATHS {
        let path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9));
        let path = bootstrap_gen
            .generate(&[(sym.clone(), start_price)], n_bars, path_seed_j)
            .expect("generate must not fail for a valid universe");

        // Log the auto-selected block length from the first path (informational, D-OG.3).
        if j == 0
            && let Some(l) = path.selected_block_length
        {
            if l <= 1 {
                eprintln!(
                    "WARN {sym}/{strat}: Auto block length degenerated to L={l} (≈ i.i.d.). \
                     Consider BlockLengthPolicy::Fixed per D-OG.3 fallback."
                );
            } else {
                eprintln!("  block_length(Auto)={l} for {sym}/{strat}");
            }
        }

        let path_bars = path.bars_by_symbol[0].clone();
        if let Some(report) = run_one_path(sym, strat, path_bars).await {
            metrics.push(path_metrics_from_report(&report));
        }
    }

    if metrics.is_empty() {
        eprintln!("SKIP ensemble {}/{strat}: all paths errored", sym);
        return None;
    }

    match DistributionSummary::from_path_metrics(&metrics) {
        Ok(summary) => Some(summary),
        Err(e) => {
            eprintln!("DistributionSummary error for {sym}/{strat}: {e}");
            None
        }
    }
}

// ── Main harness ──────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn realdata_simple_strategy_overfit_guard() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap();

    // Skip cleanly when the corpus is absent (AC-OG.2).
    if !root.join("data/binance/AVAXUSDT/2024/01.parquet").is_file() {
        eprintln!(
            "SKIP overfit-guard: data/binance corpus absent (data/binance/AVAXUSDT/2024/01.parquet not found)"
        );
        return;
    }

    let avax = Symbol::new("AVAXUSDT");
    let dot = Symbol::new("DOTUSDT");

    // Cell matrix: (label, sym, year (start_ms, end_ms), cell_index)
    // cell_index selects the column in ENSEMBLE_SEEDS.
    // 0 = AVAX·2024, 1 = DOT·2024, 2 = AVAX·2023
    let cells: &[(&str, &Symbol, (u64, u64), usize)] = &[
        ("AVAX·2024 (down)", &avax, (Y2024_START, Y2024_END), 0),
        ("DOT·2024  (down)", &dot, (Y2024_START, Y2024_END), 1),
        (
            "AVAX·2023 (up-market control)",
            &avax,
            (Y2023_START, Y2023_END),
            2,
        ),
    ];

    println!();
    println!("## Simple-strategy overfit / robustness guard — block-bootstrap N={N_PATHS}");
    println!("## Frozen § 0 rule: FRAGILE if sharpe.p5<0 OR prob_loss>0.35 OR dd_p95>0.70");
    println!("##                  ROBUST  if sharpe.p5≥0.5 AND prob_loss≤0.15 AND dd_p95≤0.50");
    println!("##                  MARGINAL otherwise. Composite = worst band.");
    println!();
    println!(
        "| Cell | Strategy | N | sharpe p5/p25/p50/p75/p95 | prob_loss | P(sharpe>0) | dd_p50 | dd_p95 | VERDICT |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");

    for (cell_label, sym, year, cell_idx) in cells {
        for (strat_idx, (strat_id, strat_label)) in STRATS.iter().enumerate() {
            // Skip the up-market contrast for strategies other than SMA
            // (R-OG.5: "≥1 up-market contrast" — we run AVAX·2023 for SMA only
            // as the miscalibration check; the other 3 strats are the 8 down-market ensembles).
            if *cell_idx == 2 && strat_idx != 0 {
                continue;
            }

            let ensemble_seed = ENSEMBLE_SEEDS[strat_idx][*cell_idx];
            eprint!("  running {cell_label} × {strat_label} (seed=0x{ensemble_seed:016X}) … ");

            let result = run_ensemble(&root, sym, *year, strat_id, ensemble_seed).await;

            match result {
                None => {
                    println!(
                        "| {cell_label} | {strat_label} | — | SKIP (no data or all errors) | — | — | — | — | SKIP |"
                    );
                    eprintln!("SKIP");
                }
                Some(s) => {
                    let verdict = score_verdict(&s);
                    println!(
                        "| {cell_label} | {strat_label} | {N_PATHS} | \
                        {:.3}/{:.3}/{:.3}/{:.3}/{:.3} | \
                        {:.3} | {:.3} | {:.3} | {:.3} | **{verdict}** |",
                        s.sharpe.p5,
                        s.sharpe.p25,
                        s.sharpe.p50,
                        s.sharpe.p75,
                        s.sharpe.p95,
                        s.prob_loss,
                        s.prob_sharpe_gt_0,
                        s.max_dd_tail_p50,
                        s.max_dd_tail_p95,
                    );
                    eprintln!("{verdict}");
                }
            }
        }
    }

    println!();
    println!("## Legend");
    println!(
        "## - sharpe p5/.../p95: percentile distribution of annualised hourly Sharpe over {N_PATHS} paths"
    );
    println!("## - prob_loss: P(final_equity < initial_equity)");
    println!("## - P(sharpe>0): fraction of paths with positive Sharpe");
    println!("## - dd_p50/dd_p95: max-drawdown tail percentiles (p50 and p95 across paths)");
    println!("## - Negative control: RSI + BBands MUST score FRAGILE/MARGINAL (AC-OG.4).");
    println!("##   If RSI/BBands come back ROBUST → harness miscalibrated, escalate.");
}
