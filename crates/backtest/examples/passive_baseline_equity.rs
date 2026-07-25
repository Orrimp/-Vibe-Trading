//! Read-only probe: realized equal-weight buy-once-hold equity curve + full metrics.
//!
//! ## Purpose
//!
//! Produces the realized (actual-history, single-path) equity curve and full metrics
//! for the equal-weight buy-and-hold baseline on the 10-symbol USDT universe.
//! Matches the `run_buyhold_path` construction in `bin/param_robustness_sweep.rs`
//! (lines 1675-1760) exactly: equal-weight allocation at bar-0 close, zero
//! rebalancing, no fees.
//!
//! ## Scope boundary
//!
//! READ-ONLY: reads data via `RealDataBarSource`, calls existing `stats::` fns.
//! Does NOT modify any strategy, `ScoreSource`, anchor, or production binary.
//! Matches the pattern of `crates/data/examples/basis_diag.rs`.
//!
//! ## Output
//!
//! - Writes daily-sampled equity-curve CSVs to the artifacts dir.
//! - Prints a metrics table (Sharpe, Sortino, Calmar, MaxDD, TotalReturn).
//! - Exits non-zero on data-load or I/O error.
//!
//! ## Run
//!
//! ```sh
//! cargo run -p backtest --features realdata --example passive_baseline_equity \
//!     2>&1 | tee /tmp/passive-baseline-run.log
//! ```
//!
//! Optional flags (defaults match the sweep harness):
//!   --data-root   <path>   (default: data/binance)
//!   --out-dir     <path>   (default: docs/runbooks/artifacts/passive-baseline-2026-06-08)
//!   --year        <2023|2024|all>  (default: all — runs both years)

// This probe uses float arithmetic throughout the statistical/CSV layer
// (price series → f64 for output and stats). Money stays Decimal until metric
// computation. Matching the pattern of the existing stat fns (stats/mod.rs).
#![allow(clippy::float_arithmetic)]
// Indexing and casting in small bounded loops — matching the existing codebase pattern.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use time::OffsetDateTime;

use backtest::realdata::{RealDataBarSource, TimeSpan};
use backtest::scenarios::momentum::top10_symbols_with_prices;
use backtest::stats::{
    compute_calmar, compute_max_drawdown_f64, compute_sharpe_hourly, compute_sortino_hourly,
    compute_total_return,
};

// ── Pinned data-revision SHA (matches the sweep harness default) ───────────────
const EXPECTED_REVISION_SHA: &str =
    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7";

// ── Expected bar counts (from sweep harness) ───────────────────────────────────
const BARS_2023: usize = 8760;
const BARS_2024: usize = 8784;

// ── Initial capital (matches sweep harness) ────────────────────────────────────
const INITIAL_CAPITAL: Decimal = dec!(100_000);

// ── Sampling cadence: one sample per 24h (hourly → daily for CSV) ─────────────
const DAILY_STRIDE: usize = 24;

// ── Default artifact output directory ─────────────────────────────────────────
const DEFAULT_OUT_DIR: &str = "docs/runbooks/artifacts/passive-baseline-2026-06-08";

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    let years: Vec<i32> = match args.year.as_deref() {
        Some("2023") => vec![2023],
        Some("2024") => vec![2024],
        _ => vec![2023, 2024],
    };

    // Workspace-relative path resolution — match backtest::paths pattern.
    let data_root = backtest::paths::resolve_workspace_path(&args.data_root);
    let out_dir = backtest::paths::resolve_workspace_path(&args.out_dir);

    fs::create_dir_all(&out_dir).unwrap_or_else(|e| {
        eprintln!("ERROR: cannot create output dir {}: {e}", out_dir.display());
        std::process::exit(1);
    });

    let symbols_prices = top10_symbols_with_prices();
    let n_symbols = symbols_prices.len();
    let symbols: Vec<trading_core::Symbol> =
        symbols_prices.iter().map(|(s, _)| s.clone()).collect();

    let mut all_results: Vec<YearResult> = Vec::new();

    for year in years {
        eprintln!("--- passive_baseline_equity: loading {year} ---");

        let bar_count = if year == 2024 { BARS_2024 } else { BARS_2023 };
        let expected_total = bar_count * n_symbols;

        let src = RealDataBarSource::new(data_root.clone(), symbols.clone());
        let span = TimeSpan::full_year(year);
        let scenario_name = format!("passive-baseline-equity-{year}");

        let loaded = src
            .load(span, expected_total, &scenario_name)
            .unwrap_or_else(|e| {
                eprintln!("ERROR loading {year} data: {e}");
                std::process::exit(1);
            });

        eprintln!("  revision_sha   = {}", &loaded.revision_sha[..8]);
        if loaded.revision_sha != EXPECTED_REVISION_SHA {
            eprintln!(
                "WARNING: data revision mismatch!\n  expected: {}\n  got:      {}",
                EXPECTED_REVISION_SHA, loaded.revision_sha
            );
        }

        eprintln!(
            "  bars loaded    = {} / {} expected",
            loaded.loaded_count, expected_total
        );

        // ── Build BH equity curve (matches run_buyhold_path exactly) ──────────
        let (equity_curve, timestamps_ms) = build_buyhold_curve(&loaded.bars, n_symbols);

        // ── Compute metrics ────────────────────────────────────────────────────
        let sharpe = compute_sharpe_hourly(&equity_curve);
        let sortino = compute_sortino_hourly(&equity_curve);
        let calmar = compute_calmar(&equity_curve);
        let max_dd = compute_max_drawdown_f64(&equity_curve);
        let total_return = compute_total_return(&equity_curve);

        let initial_equity = equity_curve.first().copied().unwrap_or(INITIAL_CAPITAL);
        let final_equity = equity_curve.last().copied().unwrap_or(INITIAL_CAPITAL);
        let min_equity = equity_curve
            .iter()
            .copied()
            .reduce(Decimal::min)
            .unwrap_or(INITIAL_CAPITAL);
        let max_equity = equity_curve
            .iter()
            .copied()
            .reduce(Decimal::max)
            .unwrap_or(INITIAL_CAPITAL);

        eprintln!("  equity curve   = {} points", equity_curve.len());
        eprintln!(
            "  initial equity = {:.2}",
            initial_equity.to_f64().unwrap_or(0.0)
        );
        eprintln!(
            "  final equity   = {:.2}",
            final_equity.to_f64().unwrap_or(0.0)
        );
        eprintln!("  Sharpe         = {sharpe:.6}   Sortino = {sortino:.6}");
        eprintln!(
            "  Calmar         = {calmar:.6}   MaxDD   = {:.4}%   TR = {:.4}%",
            max_dd * 100.0,
            total_return * 100.0
        );

        // ── Write daily-sampled CSV ────────────────────────────────────────────
        let csv_path = out_dir.join(format!("bh-equity-curve-{year}.csv"));
        write_equity_csv(&csv_path, &equity_curve, &timestamps_ms, DAILY_STRIDE).unwrap_or_else(
            |e| {
                eprintln!("ERROR writing CSV: {e}");
                std::process::exit(1);
            },
        );
        eprintln!("  CSV written    = {}", csv_path.display());

        all_results.push(YearResult {
            year,
            sharpe,
            sortino,
            calmar,
            max_dd,
            total_return,
            initial_equity,
            final_equity,
            min_equity,
            max_equity,
            n_bars: equity_curve.len().saturating_sub(1),
            csv_path,
        });
    }

    // ── Print final metrics table ──────────────────────────────────────────────
    println!();
    println!("=== Realized Equal-Weight Buy-and-Hold Metrics ===");
    println!(
        "Universe : {} symbols (ADAUSDT/AVAXUSDT/BNBUSDT/BTCUSDT/DOGEUSDT/DOTUSDT/ETHUSDT/LINKUSDT/SOLUSDT/XRPUSDT)",
        n_symbols
    );
    println!(
        "Capital  : $100,000 USDT ({} per symbol)",
        INITIAL_CAPITAL.to_f64().unwrap_or(100_000.0) / n_symbols as f64
    );
    println!(
        "Data     : data/binance/ 1h OHLCV, revision {}",
        &EXPECTED_REVISION_SHA[..8]
    );
    println!("Construction: buy at bar-0 close, equal-weight, zero rebalancing, zero fees");
    println!();
    println!(
        "{:<6}  {:>10}  {:>10}  {:>10}  {:>10}  {:>12}  {:>14}  {:>14}  {:>14}  {:>14}  {:>6}",
        "Year",
        "Sharpe",
        "Sortino",
        "Calmar",
        "MaxDD%",
        "TotalReturn%",
        "InitEquity",
        "FinalEquity",
        "MinEquity",
        "MaxEquity",
        "Bars"
    );
    println!("{}", "-".repeat(130));
    for r in &all_results {
        println!(
            "{:<6}  {:>10.4}  {:>10.4}  {:>10.4}  {:>10.4}  {:>12.4}  {:>14.2}  {:>14.2}  {:>14.2}  {:>14.2}  {:>6}",
            r.year,
            r.sharpe,
            r.sortino,
            r.calmar,
            r.max_dd * 100.0,
            r.total_return * 100.0,
            r.initial_equity.to_f64().unwrap_or(0.0),
            r.final_equity.to_f64().unwrap_or(0.0),
            r.min_equity.to_f64().unwrap_or(0.0),
            r.max_equity.to_f64().unwrap_or(0.0),
            r.n_bars,
        );
    }
    println!();
    println!("=== Bootstrap Reconciliation ===");
    println!(
        "The realized Sharpe is the SINGLE actual-history path. The bootstrap p50 (+1.735/+1.105)"
    );
    println!("is the MEDIAN over 200 block-resampled paths seeded 0xC0FFEE.");
    println!("Agreement in sign + order-of-magnitude is the sanity gate.");
    for r in &all_results {
        let bh_p50 = if r.year == 2023 {
            1.735_275_f64
        } else {
            1.104_731_f64
        };
        let gap = (r.sharpe - bh_p50).abs();
        let pct_gap = if bh_p50.abs() > 1e-9 {
            gap / bh_p50.abs() * 100.0
        } else {
            0.0
        };
        println!(
            "  {}: realized Sharpe = {:.4}, bootstrap p50 = {:.4}, |gap| = {:.4} ({:.1}%)",
            r.year, r.sharpe, bh_p50, gap, pct_gap
        );
    }
    println!();
    println!("=== Output Files ===");
    for r in &all_results {
        println!("  {}", r.csv_path.display());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BH equity-curve construction (mirrors run_buyhold_path in param_robustness_sweep)
// ─────────────────────────────────────────────────────────────────────────────

/// Build the realized BH equity curve from the merged bar slice.
///
/// Matches `run_buyhold_path` (lines 1683-1760 of `param_robustness_sweep.rs`)
/// exactly: equal-weight buy of all N symbols at bar-0 close, mark-to-market per
/// bar, no rebalancing, no fee.
///
/// Returns `(equity_curve, timestamps_ms)` where:
/// - `equity_curve[0] = initial_capital` (before bar 0).
/// - `equity_curve[i+1]` = mark-to-market equity after bar `i`.
/// - `timestamps_ms[i]` = open_ts of bar `i` in Unix ms (length = equity_curve.len() - 1).
fn build_buyhold_curve(bars: &[trading_core::Bar], n_symbols: usize) -> (Vec<Decimal>, Vec<i64>) {
    if bars.is_empty() || n_symbols == 0 {
        return (vec![INITIAL_CAPITAL], vec![]);
    }

    // Equal-weight allocation per symbol (matches run_buyhold_path).
    let weight = INITIAL_CAPITAL / Decimal::try_from(n_symbols as f64).unwrap_or(dec!(10));

    // Group closes by symbol (BTreeMap for deterministic order).
    let mut by_symbol: BTreeMap<String, Vec<Decimal>> = BTreeMap::new();
    for bar in bars {
        by_symbol
            .entry(bar.symbol.to_string())
            .or_default()
            .push(bar.close.get());
    }

    // Buy at bar-0 close; compute qty per symbol.
    let mut qtys: BTreeMap<String, Decimal> = BTreeMap::new();
    for (sym, prices) in &by_symbol {
        let buy_price = *prices.first().unwrap_or(&dec!(1));
        if buy_price > Decimal::ZERO {
            qtys.insert(sym.clone(), weight / buy_price);
        }
    }

    // Build a timestamp-keyed map: ts_ns → { sym → close }.
    let mut bar_map: BTreeMap<i128, BTreeMap<String, Decimal>> = BTreeMap::new();
    let mut ts_to_ms: BTreeMap<i128, i64> = BTreeMap::new();
    for bar in bars {
        let ts_ns = bar.open_ts.inner().unix_timestamp_nanos();
        let ts_ms = bar.open_ts.inner().unix_timestamp() * 1_000;
        bar_map
            .entry(ts_ns)
            .or_default()
            .insert(bar.symbol.to_string(), bar.close.get());
        ts_to_ms.entry(ts_ns).or_insert(ts_ms);
    }

    let n_bars = bar_map.len();
    let mut equity_curve: Vec<Decimal> = Vec::with_capacity(n_bars + 1);
    let mut timestamps_ms: Vec<i64> = Vec::with_capacity(n_bars);
    equity_curve.push(INITIAL_CAPITAL);

    // Carry last known price (matches run_buyhold_path's missing-bar handling).
    let mut last_prices: BTreeMap<String, Decimal> = BTreeMap::new();

    for (ts_ns, prices_at_ts) in &bar_map {
        for (sym, price) in prices_at_ts {
            last_prices.insert(sym.clone(), *price);
        }
        let equity: Decimal = qtys
            .iter()
            .map(|(sym, qty)| {
                let p = last_prices.get(sym).copied().unwrap_or(dec!(0));
                qty * p
            })
            .sum();
        equity_curve.push(equity);
        timestamps_ms.push(*ts_to_ms.get(ts_ns).unwrap_or(&0));
    }

    (equity_curve, timestamps_ms)
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV writer
// ─────────────────────────────────────────────────────────────────────────────

/// Write a daily-sampled equity CSV.
///
/// Header: `bar_index,timestamp_utc,equity_usd`
/// Stride: every `stride` hourly bars (stride=24 → one row per day).
/// Always includes the first and last equity points.
fn write_equity_csv(
    path: &PathBuf,
    equity_curve: &[Decimal],
    timestamps_ms: &[i64],
    stride: usize,
) -> Result<(), std::io::Error> {
    let mut f = fs::File::create(path)?;
    writeln!(f, "bar_index,timestamp_utc,equity_usd")?;

    let n = equity_curve.len();
    if n == 0 {
        return Ok(());
    }

    // equity_curve[0] is the initial capital before bar 0 — output it with
    // a synthetic timestamp equal to bar-0's open time (or 0 if no bars).
    let t0_ms = timestamps_ms.first().copied().unwrap_or(0);
    let t0_utc = ms_to_utc_label(t0_ms);
    writeln!(
        f,
        "0,{},{:.2}",
        t0_utc,
        equity_curve[0].to_f64().unwrap_or(0.0)
    )?;

    // equity_curve[i+1] corresponds to timestamps_ms[i] (bar i's open time).
    // Sample every `stride` points starting from bar 0.
    let mut last_written_bar = 0usize;
    let max_bar = n.saturating_sub(2); // equity[n-1] = after bar n-2
    let mut bar = 0usize;
    while bar <= max_bar {
        let eq_idx = bar + 1; // equity_curve[bar+1] = after bar `bar`
        if bar > 0 {
            // bar 0 was written above as the initial condition
            let ts_ms = if bar < timestamps_ms.len() {
                timestamps_ms[bar]
            } else {
                0
            };
            let utc = ms_to_utc_label(ts_ms);
            writeln!(
                f,
                "{},{},{:.2}",
                bar + 1,
                utc,
                equity_curve[eq_idx].to_f64().unwrap_or(0.0)
            )?;
        }
        last_written_bar = bar;
        if bar == max_bar {
            break;
        }
        bar = (bar + stride).min(max_bar);
    }

    // Always write the final bar if not already written.
    if last_written_bar < max_bar {
        let ts_ms = if max_bar < timestamps_ms.len() {
            timestamps_ms[max_bar]
        } else {
            0
        };
        let utc = ms_to_utc_label(ts_ms);
        writeln!(
            f,
            "{},{},{:.2}",
            max_bar + 1,
            utc,
            equity_curve[max_bar + 1].to_f64().unwrap_or(0.0)
        )?;
    }

    Ok(())
}

/// Format a Unix-milliseconds timestamp as a UTC label (`YYYY-MM-DDThh:00Z`).
fn ms_to_utc_label(ms: i64) -> String {
    let secs = ms / 1_000;
    match OffsetDateTime::from_unix_timestamp(secs) {
        Ok(dt) => format!(
            "{:04}-{:02}-{:02}T{:02}:00Z",
            dt.year(),
            dt.month() as u8,
            dt.day(),
            dt.hour()
        ),
        Err(_) => format!("{ms}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI argument parsing (manual — no clap dep needed for a throwaway probe)
// ─────────────────────────────────────────────────────────────────────────────

struct Args {
    data_root: PathBuf,
    out_dir: PathBuf,
    /// `Some("2023")`, `Some("2024")`, or `None` (both).
    year: Option<String>,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut data_root = PathBuf::from("data/binance");
    let mut out_dir = PathBuf::from(DEFAULT_OUT_DIR);
    let mut year: Option<String> = None;

    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--data-root" if i + 1 < argv.len() => {
                data_root = PathBuf::from(&argv[i + 1]);
                i += 2;
            }
            "--out-dir" if i + 1 < argv.len() => {
                out_dir = PathBuf::from(&argv[i + 1]);
                i += 2;
            }
            "--year" if i + 1 < argv.len() => {
                year = Some(argv[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("WARNING: unknown argument ignored: {other}");
                i += 1;
            }
        }
    }

    Args {
        data_root,
        out_dir,
        year,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-year result record
// ─────────────────────────────────────────────────────────────────────────────

struct YearResult {
    year: i32,
    sharpe: f64,
    sortino: f64,
    calmar: f64,
    max_dd: f64,
    total_return: f64,
    initial_equity: Decimal,
    final_equity: Decimal,
    min_equity: Decimal,
    max_equity: Decimal,
    n_bars: usize,
    csv_path: PathBuf,
}
