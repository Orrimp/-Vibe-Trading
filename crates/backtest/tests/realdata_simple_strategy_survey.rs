//! Re-runnable real-data survey — the four simple strategies (sma / macd /
//! rsi / bbands) on the REAL Binance hourly corpus (BTC + ETH, 2023 + 2024),
//! vs buy-and-hold, net of cost. Built on the `simple-strategies-realdata`
//! `BinanceCache` engine path.
//!
//! This is an UN-ANCHORED research artifact (`#[ignore]` — never runs in the
//! default suite, writes no report, touches no `anchors.toml`). Run it
//! explicitly to (re)produce the findings:
//!
//! ```text
//! cargo test -p backtest --test realdata_simple_strategy_survey \
//!     -- --ignored --nocapture
//! ```
//!
//! SKIPS cleanly when the gitignored `data/binance/` corpus is absent.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};

use backtest::cancel::cancellation_pair;
use backtest::engine::{BacktestKpis, DateRange, ScenarioConfig, ScenarioDataSource};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use tokio_stream::StreamExt as _;
use trading_core::{Bar, StrategyId, Symbol, Timeframe, Venue};

const SEED: [u8; 32] = [
    0xC0, 0xFF, 0xEE, 0x01, 0x02, 0x03, 0x04, 0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
];

const STRATS: &[(&str, &str)] = &[
    ("v0.sma", "SMA 20/50"),
    ("v0.5.macd", "MACD"),
    ("v0.5.rsi", "RSI"),
    ("v0.5.bbands", "BBands"),
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

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

async fn run_strategy(sym: &Symbol, strat: &str, bars: Vec<Bar>) -> Option<BacktestKpis> {
    let cfg = ScenarioConfig {
        strategy: StrategyId(strat.into()),
        pair: (Venue::Binance, sym.clone()),
        range: DateRange::Last30d, // ignored — bars_override supplies the data
        params: None,
        seed: SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
    };
    let (_h, cancel_rx) = cancellation_pair();
    backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .ok()
        .map(|r| r.kpis)
}

#[tokio::test]
#[ignore]
async fn realdata_simple_strategy_survey() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap();
    if !root.join("data/binance/BTCUSDT/2023/01.parquet").is_file() {
        eprintln!("SKIP survey: data/binance corpus absent");
        return;
    }

    // [year_label, start_ms, end_ms) — UTC year boundaries.
    let years: &[(&str, u64, u64)] = &[
        ("2023", 1_672_531_200_000, 1_704_067_200_000),
        ("2024", 1_704_067_200_000, 1_735_689_600_000),
    ];
    // Full Binance corpus universe (10 symbols). A symbol/year with no on-disk
    // parquet is reported as a "(only N bars)" row and skipped — never silently
    // synthetic.
    let symbols = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT",
        "DOTUSDT", "LINKUSDT",
    ];

    println!("\n## Real-data simple-strategy survey — Binance hourly, net of 4 bps taker cost\n");
    println!("Each cell: strategy total return % (trade count). Compare against Buy & Hold.\n");
    let header_strats = STRATS
        .iter()
        .map(|(_, l)| *l)
        .collect::<Vec<_>>()
        .join(" | ");
    println!("| Symbol · Year | Buy & Hold | {header_strats} |");
    println!("|---|---|{}", "---|".repeat(STRATS.len()));

    for sym_s in symbols {
        let sym = Symbol::new(sym_s);
        for (yr, s, e) in years {
            let bars = load_year_bars(&root, &sym, *s, *e).await;
            if bars.len() < 100 {
                println!(
                    "| {sym_s} · {yr} | (only {} bars loaded) | {} |",
                    bars.len(),
                    " | ".repeat(STRATS.len() - 1)
                );
                continue;
            }
            let bh = buy_and_hold_pct(&bars);
            let mut cells = Vec::new();
            for (id, _label) in STRATS {
                let cell = match run_strategy(&sym, id, bars.clone()).await {
                    Some(k) => {
                        // Compute return from absolute equity (unambiguous units):
                        // started at initial_equity, ended at final_equity.
                        let init = k.initial_equity.amount();
                        let fin = k.final_equity.amount();
                        let ret_pct = if init.is_zero() {
                            Decimal::ZERO
                        } else {
                            (fin - init) / init * Decimal::ONE_HUNDRED
                        };
                        format!("{ret_pct:+.1}% ({}t)", k.trade_count)
                    }
                    None => "ERR".to_string(),
                };
                cells.push(cell);
            }
            println!(
                "| {sym_s} · {yr} ({} bars) | **{bh:+.1}%** | {} |",
                bars.len(),
                cells.join(" | ")
            );
        }
    }
    println!();
}
