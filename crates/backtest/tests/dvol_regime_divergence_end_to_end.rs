//! T7a — Day-1 mandatory gate (ADR-0072 D7, NON-NEGOTIABLE).
//!
//! Asserts that `DvolRegimeStrategy` produces equity that DIVERGES from
//! buy-and-hold by ≥ 1 bp over a synthetic BTCUSDT bar series where the
//! implied-vol regime clearly alternates between CALM and STRESS.
//!
//! # Why this test is mandatory
//!
//! Per CLAUDE.md non-negotiables (v3-volatility-forecaster-noop-fix 2026-05-22
//! precedent) every strategy overlay must ship with a baseline-equity-divergence
//! e2e test from day 1. A unit test on the math plus anchored reports are NOT
//! sufficient to catch a no-op overlay. This test is the gate.
//!
//! FAIL-before trigger: commenting out the Sell branch in
//! `DvolRegimeStrategy::on_bar` makes the strategy a buy-and-hold proxy → the
//! divergence assertion fails.
//!
//! # Test design
//!
//! - 90 hourly bars (15 distinct DVOL closes → warm-up finishes after 30)
//! - DVOL series is CALM for bars 0–44 (weight=1), STRESS for bars 45–89 (weight=0).
//! - BuyAndHold holds 100% → equity tracks price throughout.
//! - DVOL arm buys during CALM, sells to cash during STRESS → lower drawdown
//!   (or higher equity) depending on the price path.
//! - On a flat price path (all bars same close) the two strategies produce the
//!   same equity — so we use a monotone-falling price path during STRESS phase
//!   to force a measurable divergence.

use backtest::{
    bakeoff::buyhold::run_buyhold_path,
    cancel::cancellation_pair,
    cli_types::{LatencySlippageSimConfig, SmaComposedRunInput},
    progress::ProgressSender,
    scenarios::sma_composed_run::run_with_strategy,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use strategy::{DVOL_REGIME_WINDOW, DvolRegimeStrategy};
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

/// Build a minimal hourly bar with the given unix-epoch hour offset and close price.
fn make_bar(hour: i64, close: Decimal) -> Bar {
    let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(hour));
    let price =
        Price::new(close).unwrap_or_else(|_| Price::new(dec!(1)).expect("dec!(1) is valid price"));
    let qty = Quantity::new(Decimal::ZERO).expect("zero qty is valid");
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneHour,
        venue: Venue::Binance,
        open_ts: ts,
        close_ts: ts,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: qty,
        trade_count: 0,
        local_recv_ts: ts,
    }
}

/// Build a DVOL as-of series for `n_bars` hourly bars.
///
/// Design:
/// - Days 0–4 (hours 0–119): CALM — distinct daily closes 10, 11, 12, … (30 distinct values
///   land after W=30 days, i.e. hour 720). For a shorter test we need W distinct closes
///   to appear in the first portion.
///
/// Since W=30 is large and we only have 90 bars (3.75 days), we use a trick:
/// supply 30 distinct closes right at the start (one per bar for the first 30 bars),
/// then CALM DVOL (below the trailing median) for bars 30–44, then STRESS (above)
/// for bars 45–89.
///
/// The median of 30 values [1..30] = mean(15, 16) = 15.5.
/// CALM close = 10.0 (below 15.5 → weight=1, hold/long).
/// STRESS close = 20.0 (above 15.5 → weight=0, cash).
///
/// Returns `Vec<Option<Decimal>>` parallel to bars.
fn make_dvol_series(n_bars: usize) -> Vec<Option<Decimal>> {
    // First 30 bars: distinct closes 1, 2, …, 30 to warm up the ring.
    // Next 15 bars (30–44): CALM = 10.0 (< median 15.5).
    // Remaining bars (45..): STRESS = 20.0 (≥ median 15.5).
    let mut series = Vec::with_capacity(n_bars);
    for i in 0..n_bars {
        let val = if i < 30 {
            // Warm-up distinct values
            Decimal::from(i as u64 + 1)
        } else if i < 45 {
            dec!(10) // CALM: below median 15.5
        } else {
            dec!(20) // STRESS: above or equal to median
        };
        series.push(Some(val));
    }
    series
}

/// T7a — `DvolRegimeStrategy` diverges from buy-and-hold by ≥ 1 bp.
///
/// Price design: flat for bars 0–44 (so no divergence during warm-up/CALM),
/// falling 0.5% per bar for bars 45–89 (STRESS phase — DVOL arm goes to cash,
/// buyhold stays exposed → buyhold loses value, DVOL arm preserves capital).
#[allow(clippy::expect_used, clippy::unwrap_used)]
#[tokio::test]
async fn dvol_regime_diverges_from_buyhold_by_at_least_1bp() {
    const N_BARS: usize = 90;
    let initial_capital = dec!(100_000);

    // Build price series: flat then falling.
    let bars: Vec<Bar> = (0..N_BARS as i64)
        .map(|h| {
            let close = if h < 45 {
                dec!(50_000) // flat
            } else {
                // 0.5% drop per bar during STRESS phase
                let drop_bars = h - 45;
                // 50_000 * (0.995)^drop_bars — approximate in Decimal
                let factor = dec!(0.995);
                let mut price = dec!(50_000);
                for _ in 0..drop_bars {
                    price *= factor;
                }
                price
            };
            make_bar(h, close)
        })
        .collect();

    // Build DVOL as-of series.
    let dvol_series = make_dvol_series(N_BARS);

    // ── Run DVOL regime strategy ──────────────────────────────────────────────
    let symbol = Symbol::new("BTCUSDT");
    let dvol_strategy = Box::new(DvolRegimeStrategy::new(
        symbol.clone(),
        dvol_series,
        DVOL_REGIME_WINDOW,
    ));

    let mut seed = [0u8; 32];
    seed[0] = 0xDA; // non-zero, arbitrary (ADR-0072 test seed)

    let seed_u64 = u64::from_le_bytes([
        seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], seed[6], seed[7],
    ]);

    let input = SmaComposedRunInput {
        strategy_id: "v0.dvol_regime".to_string(),
        symbol: symbol.clone(),
        start_year: 2023,
        bar_count: N_BARS,
        initial_capital,
        slippage_bps: 0,  // no slippage — clean divergence signal
        taker_fee_bps: 0, // no fees — clean divergence signal
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: LatencySlippageSimConfig::default(),
        short_enabled: false,
        composed_toml_override: None,
    };

    let (_h_dvol, cancel_dvol) = cancellation_pair();
    let dvol_result = run_with_strategy(
        &input,
        Some(bars.clone()),
        seed_u64,
        dvol_strategy,
        cancel_dvol,
        ProgressSender::disabled(),
    )
    .await
    .expect("run_with_strategy(v0.dvol_regime) must succeed");

    let dvol_equity: Decimal = dvol_result.final_equity;

    // ── Run buy-and-hold ─────────────────────────────────────────────────────
    let (_curve, buyhold_equity) = run_buyhold_path(&bars, initial_capital, 1);

    // ── Divergence assertion ─────────────────────────────────────────────────
    // 1 bp of initial capital.
    let one_bp = initial_capital / dec!(10_000);

    let divergence = (dvol_equity - buyhold_equity).abs();
    assert!(
        divergence >= one_bp,
        "DVOL regime divergence e2e FAILED (no-op signature): \
        dvol_equity={dvol_equity}, buyhold_equity={buyhold_equity}, \
        divergence={divergence}, threshold(1bp)={one_bp}. \
        If dvol_equity == buyhold_equity, the Sell branch is a no-op."
    );

    // Sanity: in the falling-price STRESS phase, DVOL arm should preserve
    // capital better than buyhold (dvol_equity > buyhold_equity).
    assert!(
        dvol_equity > buyhold_equity,
        "DVOL regime should outperform buyhold on falling-price STRESS phase: \
        dvol={dvol_equity}, buyhold={buyhold_equity}"
    );
}
