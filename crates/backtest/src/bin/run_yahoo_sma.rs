//! Yahoo SMA backtest — lab-yahoo-realdata v0.1.2.
//!
//! Runs the compiled-in SMA crossover (fast=20, slow=50) on real Yahoo-ticker
//! 1-day bars from the local Yahoo parquet cache (`data/yahoo/`).
//!
//! Supported tickers (10 crypto-mirror pairs):
//!   BTC-USD, ETH-USD, BNB-USD, SOL-USD, XRP-USD,
//!   ADA-USD, DOGE-USD, AVAX-USD, DOT-USD, LINK-USD
//!
//! Period:   2024-01-01 → 2024-12-31 (full year, daily cadence)
//! Seed:     0xC0FFEE (matching the canonical Binance anchor seed)
//!
//! Usage (from workspace root):
//! ```bash
//! cargo run -p backtest --features yahoo \
//!   --bin run_yahoo_sma -- \
//!   --cache-root data/yahoo \
//!   --reports-dir spec/lab-yahoo-realdata/reports
//! ```
//!
//! ETH-USD anchor (v0.1.2):
//! ```bash
//! cargo run -p backtest --features yahoo \
//!   --bin run_yahoo_sma -- \
//!   --ticker ETH-USD \
//!   --reports-dir spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/reports
//! ```
//!
//! # Anchor discipline
//!
//! This binary emits scenario IDs of the form `{ticker}-yahoo-2024-1d-sma-cross`
//! (e.g. `btc-yahoo-2024-1d-sma-cross`, `eth-yahoo-2024-1d-sma-cross`).
//! The default invocation (no `--ticker` flag) emits `btc-yahoo-2024-1d-sma-cross`
//! byte-identical to v0.1.1 anchor 69 (H3 anchor-preservation gate).
//! Existing anchors are byte-immutable (ADR-0038 § D6).
//!
//! # Determinism contract
//!
//! - Bars come from the pinned parquet cache; REVISION.toml SHA is verified.
//! - Seed is fixed at 0xC0FFEE (matching Binance CLI convention).
//! - No `SystemTime` / `Instant` inside the strategy loop.
//! - Wall-clock elapsed lives in the YAML front-matter only (`wall_clock_s:`).

#![deny(clippy::unwrap_used)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;

// Yahoo data source (feature-gated).
use data::yahoo::{Interval, YahooBarSource};

// Backtest building blocks.
use backtest::cancel::cancellation_pair;
use backtest::cli_types::{SmaComposedRunInput, SmaScenarioInput, StrategyMeta};
use backtest::progress::ProgressSender;
use backtest::scenarios::sma_composed_run;
use trading_core::Symbol;

// ── CLI ───────────────────────────────────────────────────────────────────────

/// 10 crypto-mirror Yahoo tickers (RHS of `data::yahoo::binance_to_yahoo_ticker` table).
/// Must stay in sync with that table; the cross-crate pinned-table test in
/// `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs` enforces drift parity.
pub const ALLOWED_YAHOO_TICKERS: &[&str] = &[
    "BTC-USD", "ETH-USD", "BNB-USD", "SOL-USD", "XRP-USD", "ADA-USD", "DOGE-USD", "AVAX-USD",
    "DOT-USD", "LINK-USD",
];

#[derive(Parser, Debug)]
#[command(
    name = "run_yahoo_sma",
    about = "Yahoo 2024 1d SMA-cross backtest on the Yahoo parquet cache (10 crypto-mirror tickers)"
)]
struct Args {
    /// Yahoo ticker to run the backtest on.
    /// Default: BTC-USD (preserves v0.1.1 BTC anchor byte-identically; H3 gate).
    /// Allowed: BTC-USD, ETH-USD, BNB-USD, SOL-USD, XRP-USD, ADA-USD, DOGE-USD, AVAX-USD, DOT-USD, LINK-USD
    #[arg(long, default_value = "BTC-USD")]
    ticker: String,

    /// Path to the Yahoo parquet cache root (must contain REVISION.toml).
    /// Default: data/yahoo (relative to workspace root).
    #[arg(long, default_value = "data/yahoo")]
    cache_root: PathBuf,

    /// Directory to write the backtest report into.
    /// Default: spec/lab-yahoo-realdata/reports
    #[arg(long, default_value = "spec/lab-yahoo-realdata/reports")]
    reports_dir: PathBuf,

    /// Full-year start in epoch-millis (default: 2024-01-01T00:00:00Z).
    #[arg(long)]
    start_ms: Option<i64>,

    /// Full-year end in epoch-millis (default: 2024-12-31T23:59:59Z).
    #[arg(long)]
    end_ms: Option<i64>,
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Canonical seed, matches Binance anchor convention.
const SEED: u64 = 0xC0FFEE;

/// 2024-01-01 00:00:00 UTC in epoch-millis.
const DEFAULT_START_MS: i64 = 1_704_067_200_000;

/// 2024-12-31 23:59:59 UTC in epoch-millis.
const DEFAULT_END_MS: i64 = 1_735_689_599_000;

const INITIAL_CAPITAL: Decimal = dec!(100_000);
const SLIPPAGE_BPS: u32 = 2;
const TAKER_FEE_BPS: u32 = 4;

// ── Scenario name helper ──────────────────────────────────────────────────────

/// Derive the canonical scenario name from a Yahoo ticker.
///
/// Rule (D-V0.1.2-3): `{lc-ticker-no-USD}-yahoo-2024-1d-sma-cross`
///   - Strip trailing `-USD` suffix.
///   - Lowercase.
///   - Append `-yahoo-2024-1d-sma-cross`.
///
/// Examples:
///   `BTC-USD` → `btc-yahoo-2024-1d-sma-cross`
///   `ETH-USD` → `eth-yahoo-2024-1d-sma-cross`
pub fn scenario_name(ticker: &str) -> String {
    let base = ticker.strip_suffix("-USD").unwrap_or(ticker);
    format!("{}-yahoo-2024-1d-sma-cross", base.to_lowercase())
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
    llm::tracing_init::install_global(&[], false)?;

    let args = Args::parse();

    // Validate ticker against the 10-row allowed list (R4.3).
    if !ALLOWED_YAHOO_TICKERS.contains(&args.ticker.as_str()) {
        let allowed = ALLOWED_YAHOO_TICKERS.join(", ");
        eprintln!(
            "error: invalid value '{}' for '--ticker <TICKER>'\n  \
             [possible values: {}]\n\n\
             For more information, try '--help'.",
            args.ticker, allowed
        );
        std::process::exit(2);
    }

    let ticker = &args.ticker;
    let cache_root = resolve_workspace_path(&args.cache_root);
    let reports_dir = resolve_workspace_path(&args.reports_dir);

    let start_ms = args.start_ms.unwrap_or(DEFAULT_START_MS);
    let end_ms = args.end_ms.unwrap_or(DEFAULT_END_MS);

    let scenario = scenario_name(ticker);

    println!("Scenario     : {scenario}");
    println!("Ticker       : {ticker}");
    println!("Cache root   : {}", cache_root.display());
    println!("Period       : 2024-01-01 → 2024-12-31 (1d cadence)");
    println!("Seed         : 0x{SEED:X}");

    // ── 1. Load Yahoo bars ────────────────────────────────────────────────────
    let source = YahooBarSource::new(cache_root.clone());
    let loaded = source
        .load_cached(ticker, Interval::Days1, start_ms, end_ms)
        .with_context(|| {
            format!(
                "Failed to load {ticker} 1d bars from {}\n\
                 Ensure you ran: cargo run -p data --features yahoo-online \\\n\
                 --bin fetch_yahoo_klines -- \\\n\
                 --tickers {ticker} --interval 1d --start 2024-01-01 --end 2024-12-31",
                cache_root.display()
            )
        })?;

    let bar_count = loaded.bars.len();
    let revision_sha = loaded.revision_sha.clone();
    println!("Bars loaded  : {bar_count}");
    println!("Revision SHA : {revision_sha}");

    // ── 2. Run SMA crossover ──────────────────────────────────────────────────
    let symbol = Symbol::new(ticker);
    let input = SmaComposedRunInput {
        strategy_id: "sma_crossover".to_string(),
        symbol: symbol.clone(),
        start_year: 2024,
        bar_count,
        initial_capital: INITIAL_CAPITAL,
        slippage_bps: SLIPPAGE_BPS,
        taker_fee_bps: TAKER_FEE_BPS,
        // Use default SMA params (20/50) — matches Binance anchor convention.
        sma_fast_len: None,
        sma_slow_len: None,
        // v5-latency-slippage-sim: noop for Yahoo SMA runs (not part of v5 re-emission).
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        short_enabled: false,
    };

    let run_start = std::time::Instant::now();
    let (_cancel_handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();

    let result = sma_composed_run::run(&input, Some(loaded.bars), SEED, cancel_rx, progress_tx)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let elapsed_secs = run_start.elapsed().as_secs_f64();
    let final_equity = result.final_equity;

    println!("Bars replayed: {}", result.bar_count);
    println!("Trades       : {}", result.trades);
    println!("Final equity : ${final_equity:.2} USDT");
    println!("Elapsed      : {elapsed_secs:.1}s");

    // ── 3. Write report ───────────────────────────────────────────────────────
    std::fs::create_dir_all(&reports_dir)
        .with_context(|| format!("create reports dir: {}", reports_dir.display()))?;

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let report_path = reports_dir.join(format!("backtest-{stamp}-{scenario}.md"));

    let sma_input = SmaScenarioInput {
        scenario_name: scenario.clone(),
        body_name: scenario.clone(),
        // No elapsed override — this is a new scenario, not replicating an anchor.
        body_elapsed_override: None,
        symbol,
        start_year: 2024,
        initial_capital: INITIAL_CAPITAL,
        slippage_bps: SLIPPAGE_BPS,
        taker_fee_bps: TAKER_FEE_BPS,
        baseline_report: None,
    };

    let strategy_meta: StrategyMeta = result.strategy_meta.clone();

    let yahoo_ctx = backtest::report::yahoo::YahooReportContext {
        ticker,
        interval: "1d",
        year: 2024,
        revision_sha: &revision_sha,
    };
    backtest::report::yahoo::emit_sma_report(
        &yahoo_ctx,
        &sma_input,
        &result.state,
        INITIAL_CAPITAL,
        final_equity,
        SEED,
        elapsed_secs,
        &report_path,
        &strategy_meta,
    )?;

    println!("Report       : {}", report_path.display());
    println!();
    println!("Next step: hash the report body with:");
    println!("  python3 scripts/hash_report.py {}", report_path.display());

    Ok(())
}

// ── Workspace-path resolver (mirrors backtest::paths) ─────────────────────────

/// Resolve a workspace-relative path by walking up to find `Cargo.lock`.
/// This ensures the binary works whether invoked from workspace root or not.
fn resolve_workspace_path(rel: &std::path::Path) -> PathBuf {
    // Fast path: exists relative to CWD.
    if rel.exists() {
        return rel.to_path_buf();
    }

    // Walk up looking for Cargo.lock.
    if let Ok(cwd) = std::env::current_dir() {
        let mut probe = cwd.as_path();
        for _ in 0..8 {
            if probe.join("Cargo.lock").is_file() {
                let resolved = probe.join(rel);
                return resolved;
            }
            match probe.parent() {
                Some(parent) => probe = parent,
                None => break,
            }
        }
    }

    rel.to_path_buf()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_name_btc() {
        assert_eq!(scenario_name("BTC-USD"), "btc-yahoo-2024-1d-sma-cross");
    }

    #[test]
    fn scenario_name_eth() {
        assert_eq!(scenario_name("ETH-USD"), "eth-yahoo-2024-1d-sma-cross");
    }
}
