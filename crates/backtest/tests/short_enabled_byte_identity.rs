//! T-D8 — Long-only byte-identity re-proof for ADR-0068 (load-bearing safety gate).
//!
//! Mirrors the MN `run_path_k_short_zero_byte_identical_to_head` pattern
//! (see `crates/backtest/src/scenarios/montecarlo.rs:964`).
//!
//! ## Contract
//!
//! With `short_enabled=false` the single-coin SMA-crossover engine output must be
//! **bit-identical** across two runs on the same inputs (determinism gate).
//! With `short_enabled=true` the same inputs must produce a DIFFERENT equity curve
//! on a downtrend (the short path is live, not a no-op).
//!
//! ## Why this test exists (CLAUDE.md non-negotiable)
//!
//! The v3-volatility-forecaster-noop-fix precedent (2026-05-22) showed that unit
//! tests on the math layer + anchored backtest reports are NOT sufficient to catch a
//! no-op overlay where `scale` is computed but never applied.  This test provides
//! the live-path proof for the short gate.
//!
//! ## RED-on-revert triggers
//!
//! - If the `short_enabled` flag is removed or ignored, the `short/long` equity
//!   comparison in the downtrend test will fail (they become equal).
//! - If a non-deterministic RNG or SystemTime call leaks into the bar loop, the
//!   determinism assertion will fail (two `short_enabled=false` runs diverge).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use backtest::cancel::cancellation_pair;
use backtest::cli_types::{LatencySlippageSimConfig, SmaComposedRunInput};
use backtest::progress::ProgressSender;
use backtest::scenarios::sma_composed_run::run;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Test seed (fixed for determinism — must never be 0) ───────────────────────

const SEED: u64 = 0xBEEF_CA5E_DEAD_B175; // "beef-case dead bytes"

// ── Shared bar series ─────────────────────────────────────────────────────────

/// A representative 200-bar downtrend series.
/// Identical to `downtrend_bars` in the T-D7 test, duplicated here so T-D8
/// compiles independently (no cross-test import).
fn downtrend_bars_200() -> Vec<Bar> {
    let sym = Symbol::new("BTCUSDT");
    let epoch = OffsetDateTime::UNIX_EPOCH;
    let mut bars = Vec::with_capacity(200);
    let mut price = 50_000.0_f64;
    for i in 0..200_usize {
        let close_d = Decimal::try_from(price.max(1.0)).unwrap_or(dec!(1));
        let ts = Timestamp::new(epoch + time::Duration::hours(i as i64));
        let close_ts = Timestamp::new(epoch + time::Duration::hours(i as i64 + 1));
        bars.push(Bar {
            symbol: sym.clone(),
            tf: Timeframe::OneHour,
            open: Price::new(close_d).expect("open"),
            high: Price::new(close_d).expect("high"),
            low: Price::new(close_d).expect("low"),
            close: Price::new(close_d).expect("close"),
            volume: Quantity::new(dec!(10)).expect("vol"),
            open_ts: ts,
            close_ts,
            trade_count: 0,
            local_recv_ts: ts,
            venue: Venue::Binance,
        });
        price *= 0.995; // 0.5% drop per bar
    }
    bars
}

/// A representative 200-bar flat series.
fn flat_bars_200() -> Vec<Bar> {
    let sym = Symbol::new("BTCUSDT");
    let epoch = OffsetDateTime::UNIX_EPOCH;
    let mut bars = Vec::with_capacity(200);
    let close_d = dec!(30_000); // constant price
    for i in 0..200_usize {
        let ts = Timestamp::new(epoch + time::Duration::hours(i as i64));
        let close_ts = Timestamp::new(epoch + time::Duration::hours(i as i64 + 1));
        bars.push(Bar {
            symbol: sym.clone(),
            tf: Timeframe::OneHour,
            open: Price::new(close_d).expect("open"),
            high: Price::new(close_d).expect("high"),
            low: Price::new(close_d).expect("low"),
            close: Price::new(close_d).expect("close"),
            volume: Quantity::new(dec!(10)).expect("vol"),
            open_ts: ts,
            close_ts,
            trade_count: 0,
            local_recv_ts: ts,
            venue: Venue::Binance,
        });
    }
    bars
}

// ── Shared input builder ───────────────────────────────────────────────────────

fn long_only_input() -> SmaComposedRunInput {
    SmaComposedRunInput {
        strategy_id: "sma_crossover".to_string(),
        symbol: Symbol::new("BTCUSDT"),
        start_year: 2023,
        bar_count: 200,
        initial_capital: dec!(10_000),
        slippage_bps: 2,
        taker_fee_bps: 4,
        sma_fast_len: Some(5),
        sma_slow_len: Some(20),
        latency_slippage_sim: LatencySlippageSimConfig::default(),
        short_enabled: false,
    }
}

fn short_enabled_input() -> SmaComposedRunInput {
    SmaComposedRunInput {
        short_enabled: true,
        ..long_only_input()
    }
}

// ── T-D8a: determinism gate — two identical long-only runs are bit-identical ──

/// Two independent `short_enabled=false` runs on the same input must produce
/// bit-identical equity curves and fill counts.
///
/// RED-on-revert: any non-deterministic element in the bar loop (RNG, SystemTime)
/// will make this fail.
#[tokio::test]
async fn t_d8_long_only_deterministic_across_two_runs() {
    let bars = downtrend_bars_200();
    let input = long_only_input();

    let (_h, cancel_rx) = cancellation_pair();
    let r1 = run(
        &input,
        Some(bars.clone()),
        SEED,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("first long-only run");

    let (_h, cancel_rx) = cancellation_pair();
    let r2 = run(
        &input,
        Some(bars.clone()),
        SEED,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("second long-only run");

    assert_eq!(
        r1.equity_curve, r2.equity_curve,
        "T-D8a FAIL: two long-only runs on the same inputs must produce identical equity curves. \
         First run={} bars, Second run={} bars.",
        r1.equity_curve.len(),
        r2.equity_curve.len(),
    );
    assert_eq!(
        r1.fills.len(),
        r2.fills.len(),
        "T-D8a FAIL: fill counts must be identical across two runs"
    );
    assert_eq!(
        r1.final_equity,
        r2.final_equity,
        "T-D8a FAIL: final equity must be identical across two runs"
    );
}

// ── T-D8b: short gate is live — short_enabled=true diverges from short_enabled=false ─

/// On a downtrend series, `short_enabled=true` must produce a DIFFERENT equity curve
/// from `short_enabled=false`.  This proves the gate is live (not a no-op).
///
/// RED-on-revert: reverting the `short_enabled` gate in `sma_composed_run.rs` makes
/// the two equity curves identical (the assertion fails).
#[tokio::test]
async fn t_d8_short_enabled_diverges_from_long_only_on_downtrend() {
    let bars = downtrend_bars_200();
    let initial_capital = dec!(10_000);
    let one_bp = initial_capital / dec!(10_000); // 1bp = 1.0 USDT on 10k capital

    let (_h, cancel_rx) = cancellation_pair();
    let long_result = run(
        &long_only_input(),
        Some(bars.clone()),
        SEED,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("long-only run");

    let (_h, cancel_rx) = cancellation_pair();
    let short_result = run(
        &short_enabled_input(),
        Some(bars.clone()),
        SEED,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("short-enabled run");

    let diff = (short_result.final_equity - long_result.final_equity).abs();
    assert!(
        diff >= one_bp,
        "T-D8b FAIL: short_enabled=true must diverge from short_enabled=false by ≥1bp \
         on a downtrend. diff={diff}, long_equity={}, short_equity={}. \
         Check: is the Sell-when-flat → short branch active?",
        long_result.final_equity,
        short_result.final_equity,
    );
}

// ── T-D8c: flat-price re-proof — long-only stays flat when signal is absent ───

/// On a flat price series (constant price), the long-only arm never trades
/// (no golden/death cross emerges since SMA(fast) == SMA(slow) for all bars
/// after warmup).  Equity stays at initial capital minus zero fees.
///
/// This is a sanity check that the bar loop is not introducing phantom trades.
#[tokio::test]
async fn t_d8_long_only_flat_price_no_trades() {
    let bars = flat_bars_200();
    let initial_capital = dec!(10_000);
    let input = long_only_input();

    let (_h, cancel_rx) = cancellation_pair();
    let result = run(
        &input,
        Some(bars),
        SEED,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("flat-price run");

    // On a flat price series with SMA-5/SMA-20, there is no crossover → no signals.
    assert_eq!(
        result.trades, 0,
        "T-D8c FAIL: long-only arm on a flat price series must have 0 trades (no crossover); \
         got trades={}",
        result.trades,
    );
    // Equity must remain at initial capital (no fills → no fees).
    assert_eq!(
        result.final_equity,
        initial_capital,
        "T-D8c FAIL: long-only equity on flat price must equal initial_capital={initial_capital}; \
         got final_equity={}",
        result.final_equity,
    );
}

// ── T-D8d: short_enabled=false does NOT enter short (gate leak check) ─────────

/// With `short_enabled=false`, the arm must NEVER open a short position
/// (position qty must never go below 0) even on a severe downtrend.
///
/// RED-on-revert: if the gate is removed, a Sell-when-flat signal opens a short
/// and the position_curve will contain negative values.
#[tokio::test]
async fn t_d8_short_enabled_false_never_enters_short_on_downtrend() {
    let bars = downtrend_bars_200();
    let input = long_only_input();

    let (_h, cancel_rx) = cancellation_pair();
    let result = run(
        &input,
        Some(bars),
        SEED,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("long-only downtrend run");

    // The position_curve tracks cumulative signed qty. With short_enabled=false
    // every entry must be >= 0 (never short).
    for (ts_ms, qty) in &result.position_curve {
        assert!(
            *qty >= Decimal::ZERO,
            "T-D8d FAIL (GATE LEAK): long-only arm must never have qty < 0. \
             Found qty={qty} at ts_ms={ts_ms}. Check: is the Sell-when-flat gate active?"
        );
    }
}
