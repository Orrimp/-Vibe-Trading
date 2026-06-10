//! Late-subscriber regression guard — paced replay feed.
//!
//! ## Problem
//!
//! `cockpit_live` starts the agent on a side thread that begins replaying bars
//! **immediately**, while iced boots on the main thread and the UI's bus
//! subscriptions connect only **after** iced initialises.  With
//! `replay_fast = true`, all ~17 520 bars are emitted in milliseconds — before
//! the UI subscribes — so the UI receives nothing.
//!
//! ## Fix
//!
//! `ReplayFeed` now supports a `pace_ms: Option<u64>` field that sleeps `N` ms
//! between every bar.  `config/agent.toml` sets `replay_pace_ms = 30` so the
//! cockpit's research-mode replay streams over a watchable timeline; the iced
//! subscription layer (late subscriber) catches every fill/pnl/position event.
//! The headless `trading` bin can override back to fast via `--fast-replay`.
//!
//! ## This test (the regression guard)
//!
//! 1. Build a `MockFeed` with 60 synthetic bars at 10 ms/bar — enough to
//!    warm up `SmaCrossover(fast=3, slow=5)` and generate several fills.
//! 2. Spawn `spawn_research_trading_loop` (the same function `runtime::run`
//!    uses in research mode) against the bus.
//! 3. **Sleep 50 ms** to simulate iced booting (late subscriber).
//! 4. Subscribe to `bus.fills()`, `bus.positions()`, `bus.pnl()`.
//! 5. Assert at least one fill, one position update, and one PnL snapshot
//!    arrive within a 5 s timeout — proving the paced feed delivers events
//!    after the subscriber connects.
//!
//! A fast feed (pace = 0 / instant) would fail step 5 because all bars emit
//! before step 4, so the broadcast history is gone.
//!
//! ## What this does NOT test
//!
//! This test does not run the full `runtime::run` — it calls
//! `spawn_research_trading_loop` directly with an injected feed.  A separate
//! smoke test (`cockpit_live_lab_run_smoke.rs`) covers the full binary path.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::uninlined_format_args
)]

use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::config::{BacktestConfig, BusConfig, RiskConfig, SizingConfig};
use agent::runtime::spawn_research_trading_loop;
use data::MockFeed;
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use trading_core::{Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn ts_at_us(us: i64) -> Timestamp {
    let dt = OffsetDateTime::from_unix_timestamp_nanos(i128::from(us) * 1_000).expect("valid ts");
    Timestamp::new(dt)
}

/// Build a `MockFeed` with `n` synthetic ticks spread across `n` distinct
/// 1-second buckets so the bar aggregator produces `n` 1-second bars.
/// Each bucket carries one tick.  Prices oscillate +/- around a base so
/// the SMA crossover strategy generates buy/sell signals (flat prices
/// produce no crossover and never trigger fills).
fn build_mock_feed(symbol: &Symbol, n: usize, bar_interval: Duration) -> MockFeed {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    // Base price 40_000; oscillate ±200 across bars to drive crossovers.
    // The pattern alternates rising / falling runs so the SMA(3) crosses
    // SMA(5) multiple times over 60 bars.
    let ticks: Vec<Tick> = (0..n)
        .map(|i| {
            let us = (i as i64) * 1_000_000;
            // Create a price wave: rising for 10 bars, falling for 10 bars, repeat.
            let phase = (i % 20) as i64;
            let delta = if phase < 10 {
                phase * 40
            } else {
                (20 - phase) * 40
            };
            let price_d = dec!(40_000) + Decimal::from(delta);
            let price = Price::new(price_d).expect("valid price");
            Tick {
                symbol: symbol.clone(),
                venue_ts: ts_at_us(us),
                local_recv_ts: ts_at_us(us),
                price,
                qty: Quantity::new(dec!(0.01)).expect("valid qty"),
                side: if i % 2 == 0 { Side::Buy } else { Side::Sell },
                trade_id: i as u64,
                venue: Venue::Binance,
            }
        })
        .collect();
    MockFeed::new(ticks, bar_interval, Venue::Binance)
}

/// Minimal `BacktestConfig` for the trading loop.
fn test_backtest_cfg() -> BacktestConfig {
    BacktestConfig {
        slippage_bps: 0,
        taker_fee_bps: 0,
        maker_fee_bps: 0,
        initial_capital_usdt: 10_000.0,
    }
}

/// Minimal `RiskConfig` for the trading loop.
fn test_risk_cfg() -> RiskConfig {
    RiskConfig {
        per_symbol_exposure_cap: 0.4,
        daily_loss_stop_pct: -5.0,
        max_drawdown_stop_pct: -15.0,
        sizing: SizingConfig {
            fixed_fraction: 0.1,
        },
    }
}

// ── Core late-subscriber test ─────────────────────────────────────────────────

/// Regression guard — a bus subscriber that connects AFTER the paced
/// trading loop has started still receives fills, positions, and PnL
/// snapshots within the timeout window.
///
/// Failure mode this guards against: `replay_fast = true` causes all bars
/// to emit in a single tokio turn BEFORE the UI subscribes → broadcast
/// ring buffer overflows → late subscriber receives nothing.  With
/// `pace_ms = Some(10)` the trading loop drips events over 600 ms (60
/// bars × 10 ms), and the late subscriber (connecting at t+50 ms) still
/// sees the remaining ~55 bars' worth of events.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paced_replay_late_subscriber_receives_fills_positions_pnl() {
    let symbol = Symbol::new("BTCUSDT");
    let tf = Timeframe::OneSecond;

    // 60 bars at 10 ms/bar = 600 ms total replay.
    // SmaCrossover(fast=3, slow=5) warms up at bar 5 and begins signalling
    // from bar 5 onward → plenty of fills in the remaining 55 bars.
    let bar_pace = Duration::from_millis(10);
    let feed = Arc::new(build_mock_feed(&symbol, 60, bar_pace));

    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Build a strategy registry with SmaCrossover(fast=3, slow=5) so warmup
    // is very short (5 bars) and fills start early.
    let registry = {
        let reg = strategy::StrategyRegistry::new();
        reg.register(Box::new(strategy::SmaCrossover::new(3, 5)));
        Arc::new(reg)
    };

    let mut set: JoinSet<()> = JoinSet::new();
    let cancel = CancellationToken::new();

    // Spawn the trading loop — it starts emitting bars immediately.
    spawn_research_trading_loop(
        feed.clone() as Arc<dyn data::MarketDataSource>,
        Arc::clone(&bus),
        Arc::clone(&registry),
        &test_backtest_cfg(),
        &test_risk_cfg(),
        symbol.clone(),
        tf,
        &mut set,
        &cancel,
    );

    // ── Simulate iced boot delay (late subscriber) ────────────────────────────
    // The UI connects to the bus AFTER a ~50 ms delay.  With `bar_pace = 10ms`
    // only ~5 bars have been emitted; the remaining ~55 bars (including all
    // fills after warmup) are still in the future.  The subscriber must
    // receive events for those remaining bars.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // NOW subscribe — simulating the iced subscription layer connecting late.
    let mut fills_rx = bus.fills();
    let mut positions_rx = bus.positions();
    let mut pnl_rx = bus.pnl();

    // ── Assert events arrive ──────────────────────────────────────────────────
    // With 55+ bars remaining at 10 ms/bar, the first fill should arrive
    // within well under 1 s (bar 5 is the first signal after warmup of 5 bars;
    // at t+50ms we're already at bar 5, so first fill lands at ~bar 8 when the
    // strategy accumulates enough signal, well within the 5 s window).
    let recv_timeout = Duration::from_secs(5);

    let fill_result = timeout(recv_timeout, fills_rx.recv()).await;
    assert!(
        fill_result.is_ok(),
        "late subscriber: timed out waiting for a fill — \
         the paced trading loop did not emit any fills after the UI-boot delay. \
         This means replay_pace_ms is not working or all bars were consumed \
         before the subscriber connected."
    );
    let fill = fill_result
        .unwrap()
        .expect("fills channel closed before first fill arrived");
    assert_eq!(fill.symbol, symbol, "fill must be for BTCUSDT");

    let pos_result = timeout(recv_timeout, positions_rx.recv()).await;
    assert!(
        pos_result.is_ok(),
        "late subscriber: timed out waiting for a position update — \
         positions must be published alongside fills (PaperEnginePublisher::on_fill)"
    );
    let pos = pos_result
        .unwrap()
        .expect("positions channel closed before first position arrived");
    assert_eq!(pos.symbol, symbol, "position must be for BTCUSDT");

    let pnl_result = timeout(recv_timeout, pnl_rx.recv()).await;
    assert!(
        pnl_result.is_ok(),
        "late subscriber: timed out waiting for a PnL snapshot — \
         pnl is published every bar; at least one must arrive after boot delay"
    );
    let _pnl = pnl_result
        .unwrap()
        .expect("pnl channel closed before first snapshot arrived");

    // Clean up.
    cancel.cancel();
    let drain_result = timeout(Duration::from_secs(2), async {
        while set.join_next().await.is_some() {}
    })
    .await;
    assert!(
        drain_result.is_ok(),
        "trading loop tasks did not drain within 2 s"
    );
}
