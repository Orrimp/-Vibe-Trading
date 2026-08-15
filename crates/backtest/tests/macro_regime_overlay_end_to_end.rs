//! Day-1 mandatory gate — ADR-0073 D6 (NON-NEGOTIABLE).
//!
//! Asserts that `run_macro_gated_buyhold_path` (the `v0.macro_riskon` arm's
//! equity-path function) behaves as a real overlay — not a no-op — by
//! exercising all four mandatory scenarios from ADR-0073 § D6:
//!
//! - **S1 (divergence):** regime flips risk-OFF mid-window → gated equity
//!   diverges from un-gated buy-and-hold by ≥ 1 bp. FAILS against a no-op
//!   implementation (regime computed but never applied).
//! - **S2 (no-op control):** regime pinned risk-ON whole window → gated equity
//!   ≈ buy-and-hold (equal, because the first bar buy is the same step in both).
//! - **S3 (causality falsifier):** two regime series that AGREE up to hour 47
//!   and differ only afterwards must produce an IDENTICAL equity prefix over
//!   bars 0..47 (`assert_eq!`) — future information cannot change the past.
//!   Rewritten by review 3-16: the previous S3 asserted only that two DIFFERENT
//!   regimes give different equity, which is input-sensitivity (already covered
//!   by S1) and which a forward-peeking implementation passes unchanged.
//! - **S4 (warm-up flat):** coin bars arrive BEFORE the first regime timestamp
//!   → `as_of_value → None` → arm holds FLAT at initial capital. Guards against
//!   treating warm-up `None` as risk-ON (spurious early exposure).
//!
//! # Why these tests are mandatory
//!
//! Per CLAUDE.md non-negotiables ("Every strategy overlay or sizing-modifier
//! ships with a baseline-equity-divergence end-to-end test from day 1") and
//! the `v3-volatility-forecaster-noop-fix` 2026-05-22 precedent: a no-op
//! overlay (where the regime boolean is computed but never actually suppresses
//! the position) is exactly the failure class that unit tests and anchored
//! reports do NOT catch. This test is the gate.
//!
//! # Fixture design (synthetic, no network, no corpus dependency)
//!
//! All coin bars are constructed in-memory. Regime series are built from
//! hand-coded `(TimestampMs, bool)` pairs. No `YahooBarSource`, no Binance
//! corpus, no network.
//!
//! Pattern reference: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
//! and `crates/backtest/tests/dvol_regime_divergence_end_to_end.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]

use backtest::bakeoff::buyhold::{run_buyhold_path, run_macro_gated_buyhold_path};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::pit::{PitSeries, TimestampMs};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a minimal hourly `Bar` at epoch + `hour_offset` hours with the given close price.
fn make_bar(hour_offset: i64, close: Decimal) -> Bar {
    let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(hour_offset));
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

/// Unix milliseconds for epoch + `hours` hours.
fn epoch_ms(hours: i64) -> i64 {
    hours * 3_600_000
}

// ── S1: Divergence (mandatory) ────────────────────────────────────────────────

/// S1 — `run_macro_gated_buyhold_path` diverges from `run_buyhold_path` by ≥ 1 bp
/// when the regime is risk-OFF across a mid-window stretch.
///
/// **Fixture design:**
/// - 48 hourly coin bars on a monotone up-trend (1% per bar rise).
/// - Regime: risk-ON for bars 0–23 (first 24 hours), risk-OFF for bars 24–47.
///   → gated arm sells to cash at bar 24, missing the second half of the up-move.
///   → un-gated buy-and-hold holds the whole window.
/// - On an up-trending coin, missing the second half means the gated arm
///   underperforms always-hold → gated_final < buyhold_final by > 1 bp.
///
/// FAILS against a no-op implementation (regime ignored → same equity as
/// buy-and-hold → divergence = 0 < 1 bp → assertion trips).
#[test]
fn s1_regime_flip_divergence_ge_1bp() {
    let n_bars: i64 = 48;
    let initial_capital = dec!(100_000);

    // Up-trending price: starts at 50_000, rises 1% per bar.
    let bars: Vec<Bar> = (0..n_bars)
        .map(|h| {
            let mut price = dec!(50_000);
            for _ in 0..h {
                price *= dec!(1.01);
            }
            make_bar(h, price)
        })
        .collect();

    // Regime: risk-ON for bars 0–23 (hours 0–23 in epoch ms),
    //         risk-OFF for bars 24–47.
    // We emit one regime record per day (daily cadence):
    //   - Day 0 (epoch+0ms): risk-ON  → covers bars 0–23
    //   - Day 1 (epoch+24h): risk-OFF → covers bars 24–47
    //
    // `as_of_value(bar.open_ts)` returns the most-recent record with
    // close_ts ≤ bar.open_ts — so bar at hour 24 gets the day-1 record.
    let regime = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(0)), true),   // risk-ON from hour 0
        (TimestampMs(epoch_ms(24)), false), // risk-OFF from hour 24
    ])
    .expect("sorted regime is valid");

    // ── Run gated path ───────────────────────────────────────────────────────
    let (_gated_curve, gated_final) = run_macro_gated_buyhold_path(&bars, &regime, initial_capital);

    // ── Run un-gated buy-and-hold ────────────────────────────────────────────
    let (_bh_curve, bh_final) = run_buyhold_path(&bars, initial_capital, 1);

    // ── Divergence assertion ─────────────────────────────────────────────────
    let one_bp = initial_capital / dec!(10_000); // 1 bp of initial capital
    let divergence = (gated_final - bh_final).abs();

    assert!(
        divergence >= one_bp,
        "S1 FAIL (no-op signature detected): \
        gated_final={gated_final}, bh_final={bh_final}, \
        divergence={divergence}, threshold(1bp)={one_bp}. \
        If divergence ≈ 0, the regime gate is a no-op (not applied)."
    );

    // Direction check: gated arm went to cash during the up-trend's second half
    // → it must have LESS final equity than always-hold (missed the up-move).
    assert!(
        gated_final < bh_final,
        "S1 direction check: on an up-trending coin, the gated arm (flat during \
        the up-move) must underperform always-hold. \
        gated_final={gated_final}, bh_final={bh_final}"
    );
}

// ── S2: No-op control (mandatory) ─────────────────────────────────────────────

/// S2 — when the regime is pinned risk-ON for the entire window, the gated
/// arm must produce IDENTICAL equity to `run_buyhold_path`.
///
/// Both arms buy at bar-0 close and hold. There are no regime transitions, so
/// no sells → the equity curves are byte-identical step-for-step.
///
/// Catches the failure mode where the overlay incorrectly gates even when the
/// regime never fires.
#[test]
fn s2_always_risk_on_equals_buyhold() {
    let _n_bars: i64 = 24;
    let initial_capital = dec!(100_000);

    // Mixed-movement price (to exercise non-trivial path).
    let prices = [
        dec!(50_000),
        dec!(51_000),
        dec!(50_500),
        dec!(52_000),
        dec!(51_800),
        dec!(53_000),
        dec!(52_500),
        dec!(54_000),
        dec!(53_500),
        dec!(55_000),
        dec!(54_500),
        dec!(56_000),
        dec!(55_500),
        dec!(57_000),
        dec!(56_500),
        dec!(58_000),
        dec!(57_500),
        dec!(59_000),
        dec!(58_500),
        dec!(60_000),
        dec!(59_500),
        dec!(61_000),
        dec!(60_500),
        dec!(62_000),
    ];
    let bars: Vec<Bar> = prices
        .iter()
        .enumerate()
        .map(|(i, &p)| make_bar(i as i64, p))
        .collect();

    // Regime: risk-ON from epoch 0 → covers every bar.
    let regime = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(0)), true), // risk-ON whole window
    ])
    .expect("sorted regime is valid");

    // ── Run gated path ───────────────────────────────────────────────────────
    let (_gated_curve, gated_final) = run_macro_gated_buyhold_path(&bars, &regime, initial_capital);

    // ── Run un-gated buy-and-hold ────────────────────────────────────────────
    let (_bh_curve, bh_final) = run_buyhold_path(&bars, initial_capital, 1);

    // ── No-op assertion ──────────────────────────────────────────────────────
    // When always risk-ON, the gated path opens at bar-0 and never sells.
    // It is byte-identical to buy-and-hold.
    assert_eq!(
        gated_final, bh_final,
        "S2 FAIL: always-risk-ON gated arm must equal buy-and-hold. \
        gated_final={gated_final}, bh_final={bh_final}"
    );
}

// ── S3: Causality falsifier (a REAL leak-check) ──────────────────────────────

/// S3 — **information that does not exist yet cannot change the past.**
///
/// # Why this test was rewritten (review 3-16 HIGH)
///
/// S3 used to assert only `final_a != final_b` for two *different* regime
/// series: input-sensitivity, which S1 already establishes, and which **a
/// forward-peeking implementation passes unchanged** — peeking still yields two
/// different numbers. It was named "look-ahead leak-check" and could not detect
/// a look-ahead leak.
///
/// # The falsifier
///
/// Two regime series that **AGREE on every record up to hour 47** and differ
/// only afterwards:
///
/// | | ≤ h47 | h48 | h60 |
/// |---|---|---|---|
/// | A | ON from h0 | **OFF** | — |
/// | B | ON from h0 | — | **OFF** |
///
/// A causal (`as_of_value(bar.open_ts)`) arm cannot see either flip while
/// pricing bars 0..47, so the two equity curves must be **identical over that
/// prefix** — the assertion below. An arm that peeks forward by ≥ 24 h reads A's
/// h48 flip at bar 24 and B's (nothing yet) at bar 24, sells in A and holds in
/// B, and the prefixes diverge → RED.
///
/// Mutation-proven: changing the production join in
/// `run_macro_gated_buyhold_path` to `as_of_value(TimestampMs(ts_ms + 86_400_000))`
/// turns this test RED with a prefix mismatch at bar 24; reverting turns it
/// green.
///
/// The old input-sensitivity content is retained below the causality assertion
/// (the suffix, where the two series legitimately differ, MUST diverge) so the
/// test still fails against a regime-ignoring no-op.
#[test]
fn s3_causality_future_regime_cannot_change_the_past() {
    let n_bars: i64 = 72;
    let initial_capital = dec!(100_000);

    // Monotone up-trend: 1% per bar (so any change in exposure moves equity).
    let bars: Vec<Bar> = (0..n_bars)
        .map(|h| {
            let mut price = dec!(50_000);
            for _ in 0..h {
                price *= dec!(1.01);
            }
            make_bar(h, price)
        })
        .collect();

    // Identical through hour 47; they diverge only in the FUTURE of that prefix.
    let regime_a = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(0)), true),
        (TimestampMs(epoch_ms(48)), false),
    ])
    .expect("sorted regime_a is valid");
    let regime_b = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(0)), true),
        (TimestampMs(epoch_ms(60)), false),
    ])
    .expect("sorted regime_b is valid");

    let (curve_a, final_a) = run_macro_gated_buyhold_path(&bars, &regime_a, initial_capital);
    let (curve_b, final_b) = run_macro_gated_buyhold_path(&bars, &regime_b, initial_capital);

    // ── Causality assertion ──────────────────────────────────────────────────
    // curve[0] is the pre-trade initial capital; curve[i+1] is bar i. The two
    // regimes agree on bars 0..=47, so those 49 entries must match exactly.
    const AGREE_THROUGH_BAR: usize = 47;
    assert!(
        curve_a.len() > AGREE_THROUGH_BAR + 1 && curve_b.len() > AGREE_THROUGH_BAR + 1,
        "fixture error: curves must cover bar {AGREE_THROUGH_BAR} \
         (len_a={}, len_b={})",
        curve_a.len(),
        curve_b.len()
    );
    for i in 0..=AGREE_THROUGH_BAR + 1 {
        assert_eq!(
            curve_a[i],
            curve_b[i],
            "S3 FAIL (LOOK-AHEAD LEAK): the two regimes are byte-identical for every \
             record at or before hour 47, so equity at curve index {i} (bar {}) cannot \
             depend on which of them is supplied — unless the arm is reading a regime \
             value from the FUTURE. a={}, b={}",
            i.saturating_sub(1),
            curve_a[i],
            curve_b[i]
        );
    }

    // ── Retained input-sensitivity (the old S3 content) ─────────────────────
    // After hour 47 the series genuinely differ, so the equity must too — this
    // is what fails against a regime-ignoring no-op.
    assert_ne!(
        final_a, final_b,
        "S3 FAIL (no-op signature): the regimes differ from hour 48 on (OFF at h48 vs \
         OFF at h60), so the final equity must differ. Identical ({final_a}) means the \
         regime is not applied at all."
    );
    assert!(
        final_b > final_a,
        "S3 direction: B stays risk-ON 12 bars longer on an up-trend → must end higher. \
         final_a={final_a}, final_b={final_b}"
    );
}

// ── S4: Warm-up flat (mandatory) ─────────────────────────────────────────────

/// S4 — coin bars arriving BEFORE the first regime timestamp must produce FLAT
/// equity (arm holds cash = initial_capital throughout).
///
/// The first regime record is timestamped at hour 100 (far in the future).
/// All 24 hourly coin bars are at hours 0–23, well before the regime is defined.
/// `as_of_value(bar.open_ts)` returns `None` for all bars → the arm stays flat
/// → equity is constant at `initial_capital` for the entire window.
///
/// Guards against treating warm-up `None` as risk-ON (spurious early exposure).
#[test]
fn s4_warmup_before_first_regime_timestamp_is_flat() {
    let n_bars: i64 = 24;
    let initial_capital = dec!(100_000);

    // Rising price (so a risk-ON arm would gain; the flat arm must NOT gain).
    let bars: Vec<Bar> = (0..n_bars)
        .map(|h| {
            let mut price = dec!(50_000);
            for _ in 0..h {
                price *= dec!(1.01);
            }
            make_bar(h, price)
        })
        .collect();

    // Regime with first record far in the future (hour 100) — no record ≤ bar ts.
    let regime = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(100)), true), // risk-ON from hour 100 (future)
    ])
    .expect("sorted regime is valid");

    let (curve, final_eq) = run_macro_gated_buyhold_path(&bars, &regime, initial_capital);

    // All bar-equities in the curve must be equal to initial_capital (flat/cash).
    // The curve has n_bars+1 entries (entry[0] = initial_capital; entries[1..] = each bar).
    for (i, &eq) in curve.iter().enumerate() {
        assert_eq!(
            eq, initial_capital,
            "S4 FAIL: curve[{i}] = {eq} ≠ {initial_capital} (arm held coin during warm-up). \
            Warm-up `None` must be treated as risk-OFF (flat)."
        );
    }

    // Final equity = initial_capital (no position opened during warm-up).
    assert_eq!(
        final_eq, initial_capital,
        "S4 FAIL: final equity {final_eq} ≠ initial_capital {initial_capital}. \
        Arm must remain flat during warm-up."
    );
}

// ── S5: T-CAL — Crypto24x7 calendar inertness (calendar anchor safety) ────────

/// S5 (T-CAL) — the calendar layer does not perturb the crypto path.
///
/// Two independent claims, neither of which re-implements the code under test
/// (review 3-16 MEDIUM: this test used to re-declare the wall-clock formula
/// inline as `legacy_count`, so mutating production left it green):
///
/// 1. **Classification** — the 3 macro tickers resolve to `UsEquity` and the
///    crypto mirrors to `Crypto24x7`. This is what makes the whole change inert
///    for the corpus: all 12 corpus tickers end `-USD`.
/// 2. **Day count** — `Crypto24x7.trading_days_in_range` equals a
///    hand-computed literal for each window. A literal is an oracle; a re-derived
///    formula is not.
///
/// The full anchor-safety equivalence (`expected_bars_for_calendar` vs
/// `expected_bars_for_range`, both production) lives in
/// `crates/data/src/calendar.rs`'s unit tests, where `crate::yahoo` is reachable
/// — those functions are behind `data/yahoo` and cannot be called from this
/// suite in the default `cargo test -p backtest` build.
#[test]
fn s5_t_cal_crypto24x7_day_count_and_classification() {
    use data::calendar::{MarketCalendar, classify_ticker};

    // Verify classify_ticker for the 3 macro tickers → UsEquity.
    assert_eq!(classify_ticker("^GSPC"), MarketCalendar::UsEquity);
    assert_eq!(classify_ticker("DX-Y.NYB"), MarketCalendar::UsEquity);
    assert_eq!(classify_ticker("^TNX"), MarketCalendar::UsEquity);
    // And the crypto path stays Crypto24x7.
    assert_eq!(classify_ticker("BTC-USD"), MarketCalendar::Crypto24x7);
    assert_eq!(classify_ticker("ETH-USD"), MarketCalendar::Crypto24x7);

    // Hand-computed expectations — NOT a copy of the production formula.
    let base_ms: i64 = 1_640_995_200_000; // 2022-01-01 UTC
    for &(days, expected) in &[
        (1i64, 1usize),
        (7, 7),
        (30, 30),
        (90, 90),
        (180, 180),
        (365, 365),
        (730, 730),
    ] {
        let s = base_ms;
        let e = base_ms + days * 86_400_000;
        let calendar_count = MarketCalendar::Crypto24x7.trading_days_in_range(s, e);
        assert_eq!(
            calendar_count, expected,
            "T-CAL FAIL: Crypto24x7 counted {calendar_count} days in a {days}-day window. \
             The calendar layer must leave every wall-clock day a trading day for crypto."
        );
    }
}
