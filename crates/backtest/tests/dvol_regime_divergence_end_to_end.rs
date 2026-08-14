//! T7a — Day-1 mandatory gate (ADR-0072 D7, NON-NEGOTIABLE).
//!
//! Asserts that the `v0.dvol_regime` signal moves equity: the arm run with a
//! REAL alternating DVOL series must diverge by ≥ 1 bp from the SAME arm run
//! with NO DVOL series, and must complete at least one round trip (a Buy and a
//! Sell).
//!
//! # Why this test is mandatory
//!
//! Per CLAUDE.md non-negotiables (v3-volatility-forecaster-noop-fix 2026-05-22
//! precedent) every strategy overlay must ship with a baseline-equity-divergence
//! e2e test from day 1. A unit test on the math plus anchored reports are NOT
//! sufficient to catch a no-op overlay. This test is the gate.
//!
//! # What was wrong with the first version of this gate (review 3-15 CRITICAL)
//!
//! It compared the DVOL arm against `run_buyhold_path` — a **100%-invested**
//! benchmark — while the arm runs at `FixedFractionSizer(0.10)`, i.e. **10%
//! invested**. On a window where the price moves at all, those two equity curves
//! diverge by construction: the divergence measured the SIZER, not the signal.
//! Measured under its own documented FAIL-before trigger (Sell branch removed)
//! it still passed by ~1,800×, and its `dvol_equity > buyhold_equity` "sanity"
//! assertion passed too.
//!
//! **The fix is the `channel` probe:** the baseline is now the SAME strategy,
//! through the SAME runner, with the SAME sizer, fees and bars — differing ONLY
//! in the DVOL series it is handed. Anything the harness contributes cancels;
//! what is left is attributable to the signal.
//!
//! # FAIL-before triggers this gate answers to (all verified by mutation)
//!
//! | mutation | what goes RED |
//! |---|---|
//! | comment out the `Sell` branch in `DvolRegimeStrategy::on_bar` | `sells >= 1`, and the divergence collapses to 0 (the arm becomes the control) |
//! | revert the warm-up fix (emit on weight TRANSITION, `weight: 1` init) | the CONTROL sits in 100% cash: `control.buys >= 1` fails, and the signalled arm's flat-phase entry disappears so the divergence collapses |
//! | hand the arm an empty/all-`None` series (the `dvol_override: None` stub) | the arm IS the control → divergence 0 |
//!
//! # Test design
//!
//! - 90 hourly bars; the DVOL series warms the W=30 ring over bars 0–29, is CALM
//!   for bars 30–44 and STRESS from bar 45.
//! - Price is flat for bars 0–44 and falls 0.5% per bar from bar 45, so the
//!   signalled arm (in cash from bar 45) and the control (long throughout) end
//!   in visibly different places.

use backtest::{
    bakeoff::buyhold::run_buyhold_path,
    cancel::cancellation_pair,
    cli_types::{LatencySlippageSimConfig, SmaComposedRunInput},
    progress::ProgressSender,
    scenarios::sma_composed_run::{SmaComposedRunResult, run_with_strategy},
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use strategy::{DVOL_REGIME_WINDOW, DvolRegimeStrategy};
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

const N_BARS: usize = 90;
const INITIAL_CAPITAL: Decimal = dec!(100_000);

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

/// Price series: flat for bars 0–44, falling 0.5% per bar for bars 45–89.
fn make_bars() -> Vec<Bar> {
    (0..N_BARS as i64)
        .map(|h| {
            let close = if h < 45 {
                dec!(50_000)
            } else {
                let mut price = dec!(50_000);
                for _ in 0..(h - 45) {
                    price *= dec!(0.995);
                }
                price
            };
            make_bar(h, close)
        })
        .collect()
}

/// Build the SIGNALLED DVOL as-of series.
///
/// - bars 0–29: 30 distinct closes 1..30 → the W=30 ring fills; median = 15.5.
/// - bars 30–44: CALM = 10.0 (below the median) → hold the coin.
/// - bars 45–89: STRESS = 20.0 (above the median) → step to cash.
fn make_dvol_series(n_bars: usize) -> Vec<Option<Decimal>> {
    (0..n_bars)
        .map(|i| {
            Some(if i < 30 {
                Decimal::from(i as u64 + 1)
            } else if i < 45 {
                dec!(10)
            } else {
                dec!(20)
            })
        })
        .collect()
}

/// Run `DvolRegimeStrategy` over `bars` with the given as-of DVOL series.
///
/// Every parameter except the series is identical between the two calls — same
/// runner, same sizer, same (zero) fees and slippage, same seed, same bars. That
/// is what makes the difference attributable to the signal (`channel` probe).
async fn run_arm(dvol_series: Vec<Option<Decimal>>, bars: &[Bar]) -> SmaComposedRunResult {
    let symbol = Symbol::new("BTCUSDT");
    let strategy = Box::new(DvolRegimeStrategy::new(
        symbol.clone(),
        dvol_series,
        DVOL_REGIME_WINDOW,
    ));

    // Non-zero, arbitrary (ADR-0072 test seed).
    let seed_u64 = u64::from_le_bytes([0xDA, 0, 0, 0, 0, 0, 0, 0]);

    let input = SmaComposedRunInput {
        strategy_id: "v0.dvol_regime".to_string(),
        symbol,
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: INITIAL_CAPITAL,
        slippage_bps: 0,  // no slippage — clean divergence signal
        taker_fee_bps: 0, // no fees — clean divergence signal
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: LatencySlippageSimConfig::default(),
        short_enabled: false,
        composed_toml_override: None,
    };

    let (_handle, cancel_rx) = cancellation_pair();
    run_with_strategy(
        &input,
        Some(bars.to_vec()),
        seed_u64,
        strategy,
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await
    .expect("run_with_strategy(v0.dvol_regime) must succeed")
}

/// T7a — the DVOL signal moves equity relative to the same arm without it.
#[allow(clippy::expect_used, clippy::unwrap_used)]
#[tokio::test]
async fn dvol_regime_diverges_from_the_same_arm_without_the_signal() {
    let bars = make_bars();

    let signalled = run_arm(make_dvol_series(N_BARS), &bars).await;
    // CONTROL: the exact degenerate series the bake-off used to hand this arm
    // when the corpus was missing (`cfg.dvol_override.unwrap_or_default()`).
    let control = run_arm(vec![None; N_BARS], &bars).await;

    println!(
        "dvol_regime_divergence: signalled  equity={} buys={} sells={} trades={}",
        signalled.final_equity, signalled.buys, signalled.sells, signalled.trades
    );
    println!(
        "dvol_regime_divergence: control    equity={} buys={} sells={} trades={}",
        control.final_equity, control.buys, control.sells, control.trades
    );

    // ── 1. The control must actually HOLD THE COIN ───────────────────────────
    //
    // Review 3-15 CRITICAL / bug-log #78: with the warm-up defect this arm sat in
    // 100% CASH — zero buys, equity pinned at initial capital — while five code
    // comments called it a "buy-and-hold proxy". If the control is in cash, every
    // divergence measured below is a comparison against nothing.
    assert!(
        control.buys >= 1,
        "the no-DVOL control must ENTER the coin (ADR-0072 D3: warm-up = HOLD = \
         benchmark behaviour). buys={}, equity={} — a control that never buys is \
         100% cash, which is the exact defect the 3-15 review found.",
        control.buys,
        control.final_equity
    );
    assert_eq!(
        control.sells, 0,
        "the no-DVOL control has no signal, so it must never exit: sells={}",
        control.sells
    );
    assert!(
        control.final_equity < INITIAL_CAPITAL,
        "the control is long through a falling window, so its equity must FALL \
         below initial capital ({INITIAL_CAPITAL}). equity={} — equality with \
         initial capital is the signature of an arm sitting in cash.",
        control.final_equity
    );

    // ── 2. The signalled arm must complete a round trip ──────────────────────
    //
    // This is the assertion that goes RED under the gate's own documented
    // FAIL-before trigger (comment out the `Sell` branch in
    // `DvolRegimeStrategy::on_bar`). The previous version of this gate had no
    // such assertion — it inferred "the Sell branch works" from an equity gap
    // that the 10%-vs-100% sizing produced on its own.
    assert!(
        signalled.buys >= 1 && signalled.sells >= 1,
        "the signalled arm must ENTER and EXIT at least once over a CALM→STRESS \
         window: buys={}, sells={}. sells=0 means the Sell branch is a no-op.",
        signalled.buys,
        signalled.sells
    );

    // ── 3. Divergence, measured against the channel-matched control ──────────
    let one_bp = INITIAL_CAPITAL / dec!(10_000);
    let divergence = (signalled.final_equity - control.final_equity).abs();
    println!("dvol_regime_divergence: |signalled - control| = {divergence} (1bp = {one_bp})");
    assert!(
        divergence >= one_bp,
        "DVOL regime divergence e2e FAILED (no-op signature): signalled={}, \
         control={}, divergence={divergence}, threshold(1bp)={one_bp}. The two runs \
         differ ONLY in the DVOL series, so equality means the series changes no \
         decision.",
        signalled.final_equity,
        control.final_equity
    );

    // ── 4. Direction ─────────────────────────────────────────────────────────
    // The signalled arm steps to cash for the falling STRESS phase; the control
    // rides it down. Same sizer, so this comparison IS meaningful.
    assert!(
        signalled.final_equity > control.final_equity,
        "stepping to cash for the falling STRESS phase must preserve capital \
         relative to staying long: signalled={}, control={}",
        signalled.final_equity,
        control.final_equity
    );

    // ── Diagnostic only — NOT a discriminator ────────────────────────────────
    //
    // Buy-and-hold is 100% invested while this arm runs at
    // `FixedFractionSizer(0.10)`. Their equity curves diverge whenever the price
    // moves, with the signal fully dead. Printed for context; never asserted on.
    let (_curve, buyhold_equity) = run_buyhold_path(&bars, INITIAL_CAPITAL, 1);
    println!(
        "dvol_regime_divergence: [diagnostic, NOT a gate] buyhold(100% invested)={buyhold_equity} \
         vs signalled(10% invested)={} — this gap is structural sizing, not signal",
        signalled.final_equity
    );
}
