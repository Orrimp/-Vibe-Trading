//! Bear-market survey (2021-22) — do any of the 4 simple strategies show a
//! PATH-ROBUST edge on a real, deep bear market, or does this firm the
//! 2026-06-08 "ship passive" terminal verdict?
//!
//! # Method (two-stage)
//!
//! **Stage 1 (cheap, ~80 backtests):** Point survey — 10 symbols × {2021, 2022}
//! × 4 strategies (SMA 20/50, MACD, RSI, BBands) = 80 cells. Each cell's
//! strategy total return % vs buy-and-hold, net of 4 bps taker cost. Corpus:
//! `data/binance-2122/` (2021-22 hourly, pin `4f390622`).
//!
//! **Stage 2 (expensive, ~140 s release):** Block-bootstrap path-robustness guard
//! — N=500 stationary-bootstrap paths per apparent-winner candidate, reduced to
//! Sharpe p5/p25/p50/p75/p95 + prob-of-loss + max-DD tail, scored against the
//! frozen § 0 decision rule.
//!
//! # Candidate predicate (PRE-REGISTERED / FROZEN before any Stage-1 numbers)
//!
//! A cell `(symbol · year · strategy)` is an **apparent winner** iff BOTH hold:
//! 1. `buy_and_hold_pct < 0`  (down-market gate — the thesis is *protection*)
//! 2. `strat_ret_pct − buy_and_hold_pct ≥ 10.0 pp`  (margin gate)
//!
//! Cap: top-16 by margin DESC; deterministic tie-break `(margin DESC, symbol ASC,
//! year ASC [2021<2022], strat_idx ASC)`.
//!
//! Plus one fixed out-of-predicate **up-market contrast cell** (SMA on the
//! highest-2021-B&H symbol) for the AC-BS.6 discrimination check.
//!
//! # Frozen § 0 decision rule (applied AS-IS — do NOT re-derive or soften)
//!
//! - **FRAGILE**: `sharpe.p5 < 0` OR `prob_loss > 0.35` OR `max_dd_tail_p95 > 0.70`
//! - **ROBUST**: `sharpe.p5 ≥ 0.5` AND `prob_loss ≤ 0.15` AND `max_dd_tail_p95 ≤ 0.50`
//! - **MARGINAL**: everything else. Composite = worst band.
//!
//! # Seeds (ADR-0051 D1 — orthogonality)
//!
//! - `ScenarioConfig.seed`: CONSTANT `SEED` (`0xC0FFEE…`) for every path.
//! - `ensemble_seed`: DISTINCT per `(strat_idx × candidate_rank)` via
//!   `0x00C0_FFEE_0000_0000 + strat_idx*0x100 + candidate_rank`.
//! - `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`.
//!
//! # Determinism (AC-BS.5)
//!
//! Two consecutive `--release --ignored --nocapture` runs are byte-identical.
//! Byte-identity contracted on the Apple-Silicon canonical box (ADR-0051 D5 /
//! ADR-0043 precedent); cross-platform parity is not contracted.
//!
//! # UN-ANCHORED (D-BS.4 / R-BS.9)
//!
//! `#[ignore]`, no `evidence/*/reports/` file, no `anchors.toml` row. The
//! `--nocapture` stdout + a `findings` dev-note are the deliverable.
//!
//! # Baseline-divergence e2e gate: N/A (D-BS.4 / D-OG.6 analogue)
//!
//! This harness introduces no overlay and no sizing modifier. It runs the four
//! already-shipped strategy ids unchanged through the already-shipped
//! `run_scenario` over (Stage 1) real bars and (Stage 2) bootstrap-resampled
//! bars, and reduces the output. The applicable correctness tripwires are
//! AC-BS.5 (two-run byte-identical determinism) and AC-BS.6 (a no-edge
//! mean-reverter must NOT come back ROBUST).
//!
//! ```text
//! cargo test -p backtest --test realdata_simple_strategy_bear_survey \
//!     --release -- --ignored --nocapture
//! ```
//!
//! SKIPS cleanly when `data/binance-2122/BTCUSDT/2022/01.parquet` is absent.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};

use backtest::cancel::cancellation_pair;
use backtest::engine::{BacktestKpis, DateRange, RunReport, ScenarioConfig, ScenarioDataSource};
use backtest::progress::ProgressSender;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};
use data::{BlockBootstrapPathGen, BlockLengthPolicy, MonteCarloPathGen};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio_stream::StreamExt as _;
use trading_core::{Bar, StrategyId, Symbol, Timeframe, Venue};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Constant engine seed per ADR-0051 D1 orthogonality.
/// Per-path variation lives ONLY in `path_seed_j`, NOT in this seed.
const SEED: [u8; 32] = [
    0xC0, 0xFF, 0xEE, 0x01, 0x02, 0x03, 0x04, 0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

/// Number of bootstrap paths per ensemble. DO NOT reduce — § 0 bands calibrated at N=500.
const N_PATHS: usize = 500;

/// Strategy ids (survey verbatim, strat_idx 0=SMA 1=MACD 2=RSI 3=BBands).
const STRATS: &[(&str, &str)] = &[
    ("v0.sma", "SMA 20/50"),
    ("v0.5.macd", "MACD"),
    ("v0.5.rsi", "RSI"),
    ("v0.5.bbands", "BBands"),
];

/// UTC year-boundary timestamps in milliseconds (2021-01-01 00:00:00 UTC = 1609459200).
const Y2021_START: u64 = 1_609_459_200_000;
const Y2021_END: u64 = 1_640_995_200_000; // 2022-01-01 00:00:00 UTC
const Y2022_START: u64 = 1_640_995_200_000;
const Y2022_END: u64 = 1_672_531_200_000; // 2023-01-01 00:00:00 UTC

/// Minimum bars to constitute a valid cell.
const MIN_BARS: usize = 100;

/// Candidate cap (D-BS.2 frozen). Top-16 by margin DESC.
const CANDIDATE_CAP: usize = 16;

/// Up-market contrast cell: rank slot reserved above the cap (never collides with candidates).
const CONTRAST_RANK: u64 = 0xF0;

// ── Stage-1 cell ─────────────────────────────────────────────────────────────

/// One (symbol × year × strategy) point-survey result.
#[derive(Debug, Clone)]
struct Stage1Cell {
    sym: String,
    year_label: &'static str,
    year_start: u64,
    year_end: u64,
    strat_idx: usize,
    strat_id: &'static str,
    strat_label: &'static str,
    /// Buy-and-hold return in percentage points for this symbol-year.
    bh_pct: Decimal,
    /// Strategy total return in percentage points.
    strat_ret_pct: Decimal,
    n_bars: usize,
}

impl Stage1Cell {
    fn margin(&self) -> Decimal {
        self.strat_ret_pct - self.bh_pct
    }
}

// ── Stage-1 helpers (adapted from realdata_simple_strategy_survey.rs) ────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Load hourly bars for `sym` in `[start_ms, end_ms)` from `data/binance-2122/`.
/// Returns empty Vec when parquet is absent — caller handles.
async fn load_year_bars(root: &Path, sym: &Symbol, start_ms: u64, end_ms: u64) -> Vec<Bar> {
    use data::source::MarketDataSource as _;
    // Corpus root: data/binance-2122/ (NOT data/binance/ — the 2023-24 corpus).
    let feed = data::ReplayFeed::new(root.join("data/binance-2122"), true);
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

/// Buy-and-hold return in percentage points over the bar window.
fn buy_and_hold_pct(bars: &[Bar]) -> Decimal {
    if bars.len() < 2 {
        return Decimal::ZERO;
    }
    let first = bars.first().unwrap().close.get();
    let last = bars.last().unwrap().close.get();
    if first.is_zero() {
        return Decimal::ZERO;
    }
    (last - first) / first * Decimal::ONE_HUNDRED
}

/// Run one strategy backtest and return KPIs (None on error or absent data).
async fn run_strategy(sym: &Symbol, strat: &str, bars: Vec<Bar>) -> Option<BacktestKpis> {
    let cfg = ScenarioConfig {
        strategy: StrategyId(strat.into()),
        pair: (Venue::Binance, sym.clone()),
        range: DateRange::Last30d, // ignored — bars_override supplies data
        params: None,
        seed: SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
        dvol_override: None,
        macro_regime_series: None,
    };
    let (_h, cancel_rx) = cancellation_pair();
    backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .ok()
        .map(|r| r.kpis)
}

// ── Candidate selection (D-BS.2 — FROZEN predicate, do NOT change) ───────────

/// Select Stage-2 candidates from the Stage-1 results.
///
/// FROZEN predicate (D-BS.2 — PRE-REGISTERED before any Stage-1 numbers):
///   apparent winner iff `bh_pct < 0`  AND  `strat_ret_pct − bh_pct ≥ 10.0 pp`
///
/// Sort qualifiers by `(margin DESC, symbol ASC, year ASC [2021<2022], strat_idx ASC)`.
/// Truncate to CANDIDATE_CAP (= 16).
fn select_candidates(cells: &[Stage1Cell]) -> Vec<&Stage1Cell> {
    let threshold = dec!(10.0);

    // Filter: down-market gate AND margin gate.
    let mut qualifiers: Vec<&Stage1Cell> = cells
        .iter()
        .filter(|c| c.bh_pct < Decimal::ZERO && c.margin() >= threshold)
        .collect();

    // Deterministic sort: (margin DESC, symbol ASC, year ASC, strat_idx ASC).
    qualifiers.sort_by(|a, b| {
        b.margin()
            .cmp(&a.margin())
            .then(a.sym.cmp(&b.sym))
            .then(a.year_label.cmp(b.year_label)) // "2021" < "2022" lexicographically
            .then(a.strat_idx.cmp(&b.strat_idx))
    });

    qualifiers.truncate(CANDIDATE_CAP);
    qualifiers
}

/// Derive a DISTINCT ensemble seed per (strat_idx × candidate_rank).
/// Per D-BS.3: `0x00C0_FFEE_0000_0000 + strat_idx*0x100 + candidate_rank`.
fn ensemble_seed_for(strat_idx: usize, candidate_rank: u64) -> u64 {
    0x00C0_FFEE_0000_0000u64
        .wrapping_add((strat_idx as u64).wrapping_mul(0x100))
        .wrapping_add(candidate_rank)
}

// ── Stage-2 helpers (adapted from realdata_simple_strategy_overfit_guard.rs) ─

/// Run one bootstrap path and return the full RunReport.
async fn run_one_path(sym: &Symbol, strat: &str, path_bars: Vec<Bar>) -> Option<RunReport> {
    let cfg = ScenarioConfig {
        strategy: StrategyId(strat.into()),
        pair: (Venue::Binance, sym.clone()),
        range: DateRange::Last30d,
        params: None,
        seed: SEED, // CONSTANT (ADR-0051 D1 orthogonality — do NOT vary per path)
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(path_bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
        dvol_override: None,
        macro_regime_series: None,
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

/// Extract PathMetrics from a RunReport.
/// Money stays Decimal/Money<Usdt> until the compute_* boundary (R-BS.7).
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

/// Score a DistributionSummary against the frozen § 0 bands (do NOT soften).
fn score_verdict(s: &DistributionSummary) -> &'static str {
    if s.sharpe.p5 < 0.0 || s.prob_loss > 0.35 || s.max_dd_tail_p95 > 0.70 {
        return "FRAGILE";
    }
    if s.sharpe.p5 >= 0.5 && s.prob_loss <= 0.15 && s.max_dd_tail_p95 <= 0.50 {
        return "ROBUST";
    }
    "MARGINAL"
}

/// Run one full ensemble: N=500 bootstrap paths for `(sym, year, strat)`.
/// Uses pre-loaded `real_bars` to avoid double-loading for contrast cell.
async fn run_ensemble_from_bars(
    sym: &Symbol,
    strat_id: &str,
    real_bars: Vec<Bar>,
    ensemble_seed: u64,
) -> Option<DistributionSummary> {
    if real_bars.len() < MIN_BARS {
        eprintln!(
            "SKIP ensemble {sym}/{strat_id}: only {} bars",
            real_bars.len()
        );
        return None;
    }

    let start_price = real_bars[0].close.get();
    let n_bars = real_bars.len();

    let bootstrap_gen =
        BlockBootstrapPathGen::new(vec![(sym.clone(), real_bars)], BlockLengthPolicy::Auto)
            .expect("BlockBootstrapPathGen::new must not fail for a valid single-symbol series");

    let mut metrics: Vec<PathMetrics> = Vec::with_capacity(N_PATHS);

    for j in 0..N_PATHS {
        let path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9));
        let path = bootstrap_gen
            .generate(&[(sym.clone(), start_price)], n_bars, path_seed_j)
            .expect("generate must not fail for a valid universe");

        // Log auto-selected block length from the first path (Q-BS.5).
        if j == 0
            && let Some(l) = path.selected_block_length
        {
            if l <= 1 {
                eprintln!(
                    "WARN {sym}/{strat_id}: Auto block length degenerated to L={l} (≈ i.i.d.). \
                     Consider BlockLengthPolicy::Fixed per D-OG.3 fallback. Surface as a finding."
                );
            } else {
                eprintln!("  block_length(Auto)={l} for {sym}/{strat_id}");
            }
        }

        let path_bars = path.bars_by_symbol[0].clone();
        if let Some(report) = run_one_path(sym, strat_id, path_bars).await {
            metrics.push(path_metrics_from_report(&report));
        }
    }

    if metrics.is_empty() {
        eprintln!("SKIP ensemble {sym}/{strat_id}: all paths errored");
        return None;
    }

    match DistributionSummary::from_path_metrics(&metrics) {
        Ok(summary) => Some(summary),
        Err(e) => {
            eprintln!("DistributionSummary error for {sym}/{strat_id}: {e}");
            None
        }
    }
}

/// Print the Stage-2 verdict row.
fn print_verdict_row(cell_label: &str, strat_label: &str, result: Option<&DistributionSummary>) {
    match result {
        None => {
            println!(
                "| {cell_label} | {strat_label} | — | SKIP (no data or all errors) | — | — | — | — | SKIP |"
            );
        }
        Some(s) => {
            let verdict = score_verdict(s);
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
        }
    }
}

// ── Main harness ──────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn realdata_simple_strategy_bear_survey() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap();

    // SKIP-guard (AC-BS.4): corpus absent → print SKIP, return cleanly.
    if !root
        .join("data/binance-2122/BTCUSDT/2022/01.parquet")
        .is_file()
    {
        eprintln!(
            "SKIP bear-survey: data/binance-2122 corpus absent \
             (data/binance-2122/BTCUSDT/2022/01.parquet not found)"
        );
        return;
    }

    let symbols: &[&str] = &[
        "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
        "SOLUSDT", "XRPUSDT",
    ];

    let years: &[(&'static str, u64, u64)] = &[
        ("2021", Y2021_START, Y2021_END),
        ("2022", Y2022_START, Y2022_END),
    ];

    // ─────────────────────────────────────────────────────────────────────────
    // STAGE 1 — Point survey
    // ─────────────────────────────────────────────────────────────────────────

    println!();
    println!("## Bear-market survey (2021-22) — simple strategies vs buy-and-hold");
    println!("## Corpus: data/binance-2122/ (Binance hourly, net of 4 bps taker cost)");
    println!();
    println!("### Stage 1 — Point survey (80 cells: 10 symbols × 2 years × 4 strategies)");
    println!();

    let header_strats = STRATS
        .iter()
        .map(|(_, l)| *l)
        .collect::<Vec<_>>()
        .join(" | ");
    println!("| Symbol · Year | B&H % | {header_strats} |");
    println!("|---|---|{}", "---|".repeat(STRATS.len()));

    let mut all_cells: Vec<Stage1Cell> = Vec::new();

    for sym_s in symbols {
        let sym = Symbol::new(*sym_s);
        for (yr, start_ms, end_ms) in years {
            let bars = load_year_bars(&root, &sym, *start_ms, *end_ms).await;
            if bars.len() < MIN_BARS {
                println!(
                    "| {sym_s} · {yr} | (only {} bars) | {} |",
                    bars.len(),
                    " | ".repeat(STRATS.len() - 1)
                );
                continue;
            }

            let bh = buy_and_hold_pct(&bars);
            let n_bars = bars.len();
            let mut cells_row: Vec<String> = Vec::new();

            for (strat_idx, (strat_id, strat_label)) in STRATS.iter().enumerate() {
                match run_strategy(&sym, strat_id, bars.clone()).await {
                    Some(k) => {
                        let init = k.initial_equity.amount();
                        let fin = k.final_equity.amount();
                        let ret_pct = if init.is_zero() {
                            Decimal::ZERO
                        } else {
                            (fin - init) / init * Decimal::ONE_HUNDRED
                        };
                        let trade_count = k.trade_count;
                        cells_row.push(format!("{ret_pct:+.1}% ({}t)", trade_count));

                        all_cells.push(Stage1Cell {
                            sym: sym_s.to_string(),
                            year_label: yr,
                            year_start: *start_ms,
                            year_end: *end_ms,
                            strat_idx,
                            strat_id,
                            strat_label,
                            bh_pct: bh,
                            strat_ret_pct: ret_pct,
                            n_bars,
                        });
                    }
                    None => {
                        cells_row.push("ERR".to_string());
                    }
                }
            }
            println!(
                "| {sym_s} · {yr} ({n_bars} bars) | **{bh:+.1}%** | {} |",
                cells_row.join(" | ")
            );
        }
    }

    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // CANDIDATE SELECTION (D-BS.2 — FROZEN predicate, printed explicitly)
    // ─────────────────────────────────────────────────────────────────────────

    println!("### Candidate selection (PRE-REGISTERED frozen predicate)");
    println!();
    println!(
        "Predicate: `bh_pct < 0`  AND  `strat_ret_pct − bh_pct ≥ 10.0 pp`  (down-market + margin gate)"
    );
    println!(
        "Cap: top-{CANDIDATE_CAP} by margin DESC; tie-break (margin DESC, symbol ASC, year ASC, strat_idx ASC)."
    );
    println!();

    // Collect all qualifiers before cap (for visibility).
    let threshold = dec!(10.0);
    let mut all_qualifiers: Vec<&Stage1Cell> = all_cells
        .iter()
        .filter(|c| c.bh_pct < Decimal::ZERO && c.margin() >= threshold)
        .collect();
    all_qualifiers.sort_by(|a, b| {
        b.margin()
            .cmp(&a.margin())
            .then(a.sym.cmp(&b.sym))
            .then(a.year_label.cmp(b.year_label))
            .then(a.strat_idx.cmp(&b.strat_idx))
    });

    let total_qualifiers = all_qualifiers.len();

    if total_qualifiers == 0 {
        println!("**Stage-1 result: 0 qualifying cells.** Nothing clears the predicate.");
        println!("This is a strong ship-passive result: no strategy even apparently wins.");
        println!();
    } else {
        println!(
            "**Total qualifying cells: {}** (before cap)",
            total_qualifiers
        );
        println!();
        println!("| Rank | Symbol · Year | Strategy | B&H% | Strat% | Margin | Keep? |");
        println!("|---|---|---|---|---|---|---|");
        for (i, c) in all_qualifiers.iter().enumerate() {
            let keep = if i < CANDIDATE_CAP {
                "KEEP"
            } else {
                "DROP (cap)"
            };
            println!(
                "| {} | {} · {} | {} | {:+.1}% | {:+.1}% | {:+.1} pp | {} |",
                i + 1,
                c.sym,
                c.year_label,
                c.strat_label,
                c.bh_pct,
                c.strat_ret_pct,
                c.margin(),
                keep
            );
        }
        println!();
    }

    let candidates = select_candidates(&all_cells);
    println!(
        "**Candidates advanced to Stage 2: {}** (cap={})",
        candidates.len(),
        CANDIDATE_CAP
    );
    println!();

    // ─────────────────────────────────────────────────────────────────────────
    // Up-market contrast cell (AC-BS.6 / D-BS.2):
    // SMA on the symbol with the highest 2021 full-year B&H.
    // Deterministic: pick the cell with the highest bh_pct among 2021 SMA cells.
    // ─────────────────────────────────────────────────────────────────────────

    let contrast_cell: Option<&Stage1Cell> = all_cells
        .iter()
        .filter(|c| c.year_label == "2021" && c.strat_idx == 0) // SMA only
        .max_by_key(|c| c.bh_pct);

    if let Some(cc) = contrast_cell {
        println!(
            "Up-market contrast cell (AC-BS.6): {} · {} SMA (B&H {:+.1}%)",
            cc.sym, cc.year_label, cc.bh_pct
        );
        println!("(Bootstrapped separately, does NOT count against the cap.)");
        println!();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // STAGE 2 — Block-bootstrap path-robustness guard
    // ─────────────────────────────────────────────────────────────────────────

    println!("### Stage 2 — Block-bootstrap robustness guard (N={N_PATHS} per candidate)");
    println!("### Frozen § 0 rule: FRAGILE if sharpe.p5<0 OR prob_loss>0.35 OR dd_p95>0.70");
    println!("###                  ROBUST  if sharpe.p5≥0.5 AND prob_loss≤0.15 AND dd_p95≤0.50");
    println!("###                  MARGINAL otherwise. Composite = worst band.");
    println!();
    println!(
        "| Cell | Strategy | N | sharpe p5/p25/p50/p75/p95 | prob_loss | P(sharpe>0) | dd_p50 | dd_p95 | VERDICT |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");

    // Track whether any candidate is ROBUST (the high-value tail).
    let mut any_robust = false;
    let mut any_mean_reverter_robust = false;

    for (rank, cand) in candidates.iter().enumerate() {
        let sym = Symbol::new(&cand.sym);
        let ensemble_seed = ensemble_seed_for(cand.strat_idx, rank as u64);
        let cell_label = format!("{} · {} ({} bars)", cand.sym, cand.year_label, cand.n_bars);

        eprint!(
            "  running {cell_label} × {} (seed=0x{ensemble_seed:016X}) … ",
            cand.strat_label
        );

        let real_bars = load_year_bars(&root, &sym, cand.year_start, cand.year_end).await;
        let result = run_ensemble_from_bars(&sym, cand.strat_id, real_bars, ensemble_seed).await;

        if let Some(ref s) = result {
            let verdict = score_verdict(s);
            eprintln!("{verdict}");
            if verdict == "ROBUST" {
                any_robust = true;
                // Check if the ROBUST candidate is a mean-reverter (RSI=2 or BBands=3).
                if cand.strat_idx >= 2 {
                    any_mean_reverter_robust = true;
                }
            }
        } else {
            eprintln!("SKIP");
        }

        print_verdict_row(&cell_label, cand.strat_label, result.as_ref());
    }

    // Up-market contrast cell.
    if let Some(cc) = contrast_cell {
        let sym = Symbol::new(&cc.sym);
        let contrast_label = format!(
            "{} · {} (up-market contrast, {} bars)",
            cc.sym, cc.year_label, cc.n_bars
        );
        let contrast_seed = ensemble_seed_for(0 /* SMA */, CONTRAST_RANK);

        eprint!("  running {contrast_label} × SMA (seed=0x{contrast_seed:016X}) … ");

        let real_bars = load_year_bars(&root, &sym, cc.year_start, cc.year_end).await;
        let result = run_ensemble_from_bars(&sym, cc.strat_id, real_bars, contrast_seed).await;

        if let Some(ref s) = result {
            let verdict = score_verdict(s);
            eprintln!("{verdict} (contrast)");
        } else {
            eprintln!("SKIP (contrast)");
        }

        print_verdict_row(
            &contrast_label,
            "SMA 20/50 (up-market contrast)",
            result.as_ref(),
        );
    }

    println!();
    println!("## Headline");
    println!();

    if candidates.is_empty() {
        println!(
            "**Stage-1 returned 0 candidates.** No strategy clears the predicate \
             (B&H<0 AND margin≥10pp) in 2021-22. This is a strong null result — \
             the bear sample FIRMS ship-passive. No Stage-2 path-robustness test needed."
        );
    } else if any_robust {
        println!("**AT LEAST ONE CANDIDATE SCORED ROBUST under the frozen § 0 rule.**");
        println!(
            "This is the HIGH-VALUE tail: a ROBUST survivor on a real market-wide bear \
             is the most credible non-passive signal the program has produced. \
             This REOPENS the active-vs-passive question for a scoped v0.2.0 follow-on."
        );
        if any_mean_reverter_robust {
            println!(
                "WARNING: a no-edge mean-reverter (RSI or BBands) scored ROBUST. \
                 This is a RED FLAG — harness may be miscalibrated. ESCALATE before acting."
            );
        }
    } else {
        println!("**All candidates scored FRAGILE or MARGINAL under the frozen § 0 rule.**");
        println!(
            "The 2021-22 bear sample FIRMS ship-passive. Even in the deepest available bear \
             (2022: BTC ≈−64%, cross-universe drawdown), no simple strategy shows a \
             path-robust edge via block-bootstrap. The 2026-06-08 terminal verdict stands."
        );
    }

    println!();
    println!("## Legend");
    println!(
        "## - sharpe p5/.../p95: percentile distribution of annualised hourly Sharpe over {N_PATHS} paths"
    );
    println!("## - prob_loss: P(final_equity < initial_equity)");
    println!("## - P(sharpe>0): fraction of paths with positive Sharpe");
    println!("## - dd_p50/dd_p95: max-drawdown tail percentiles across paths");
    println!(
        "## - Negative control (AC-BS.6): any RSI/BBands candidate MUST score FRAGILE/MARGINAL."
    );
    println!(
        "##   A mean-reverter scoring ROBUST is a RED FLAG — harness miscalibrated, escalate."
    );
    println!("## - Up-market contrast: SMA on highest-2021-B&H symbol must score differently");
    println!("##   from the down-market candidates (discrimination check).");
    println!("## - Scope cap: hourly bars, default params, 10 large-caps, 2 specific bear years.");
    println!(
        "##   Null firms ship-passive on available evidence — does NOT prove no strategy ever wins."
    );
    println!("##   ROBUST survivor REOPENs the question — flag it loudly.");

    // Negative-control assertion (AC-BS.6): if a mean-reverter scored ROBUST, that is a
    // harness-miscalibration RED flag — we have already printed a warning above.
    // We do NOT fail the test (the ROBUST result IS the finding); we surface it visibly.
}
