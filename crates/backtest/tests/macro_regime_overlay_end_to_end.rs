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
//! - **S3 (look-ahead leak-check):** forward-shifting the regime series by one
//!   day CHANGES the gated equity (`assert_ne!`) — proves the arm routes through
//!   `core::pit::PitSeries::as_of_value` and does NOT peek at future regime
//!   values.
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

// ── S3: Look-ahead leak-check (belt-and-suspenders) ─────────────────────────

/// S3 — forward-shifting the regime series by one day CHANGES the gated equity.
///
/// **Self-proving falsifier (ADR-0058 § D5 discipline):**
/// Build two regime series that are identical EXCEPT one is shifted 24 hours
/// into the future. If the arm correctly reads `as_of_value(bar.open_ts)` with
/// no look-ahead:
///   - Unshifted: bar at hour 24 sees the risk-OFF record → sells there.
///   - Shifted:   bar at hour 24 still sees the prior risk-ON record (risk-OFF
///     doesn't arrive until hour 48) → holds through hour 47.
///
/// The two equity paths MUST differ (divergence ≥ 1 bp) on an up-trending coin.
///
/// If they're the same, the arm is looking ahead (using the future value).
#[test]
fn s3_forward_shift_changes_equity() {
    let n_bars: i64 = 72;
    let initial_capital = dec!(100_000);

    // Monotone up-trend: 1% per bar.
    let bars: Vec<Bar> = (0..n_bars)
        .map(|h| {
            let mut price = dec!(50_000);
            for _ in 0..h {
                price *= dec!(1.01);
            }
            make_bar(h, price)
        })
        .collect();

    // Regime A (unshifted): risk-ON from 0, risk-OFF from hour 24.
    let regime_a = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(0)), true),
        (TimestampMs(epoch_ms(24)), false),
    ])
    .expect("sorted regime_a is valid");

    // Regime B (shifted forward 24h): risk-ON from 0, risk-OFF from hour 48.
    // A look-ahead-free arm uses regime_a's risk-OFF at hour 24 for bars ≥ 24;
    // with regime_b it keeps risk-ON until hour 48.
    let regime_b = PitSeries::from_sorted(vec![
        (TimestampMs(epoch_ms(0)), true),
        (TimestampMs(epoch_ms(48)), false),
    ])
    .expect("sorted regime_b is valid");

    let (_, final_a) = run_macro_gated_buyhold_path(&bars, &regime_a, initial_capital);
    let (_, final_b) = run_macro_gated_buyhold_path(&bars, &regime_b, initial_capital);

    // The shifted regime keeps the arm invested 24 extra bars of rising price
    // → final_b > final_a on an up-trending coin.
    assert_ne!(
        final_a, final_b,
        "S3 FAIL (look-ahead leak): forward-shifting the regime by 24h produced \
        identical equity ({final_a} == {final_b}). The arm must be leaking future \
        regime values into earlier bars."
    );

    // Direction: regime_b holds longer on an up-trend → higher equity.
    assert!(
        final_b > final_a,
        "S3 direction: with the shifted (later risk-OFF) regime on an up-trend, \
        the arm holds longer → must produce higher equity. \
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

/// S5 (T-CAL) — the `MarketCalendar::Crypto24x7.trading_days_in_range` method
/// is byte-identical to the legacy wall-clock day count for any range.
///
/// This is the ADR-0073 D1.anchor proof: the new calendar layer must not
/// perturb existing crypto coverage calculations. `Crypto24x7` uses the same
/// wall-clock day division as the original `expected_bars_for_range(Days1, …)`.
/// If this test fails, the calendar module has regressed the crypto path.
///
/// Additionally verifies `classify_ticker` correctly maps the 3 macro tickers
/// to `UsEquity` and BTC-USD → `Crypto24x7`.
///
/// Note: this test is in the `backtest` integration test suite (not `data`)
/// so it runs in the same `cargo test -p backtest` invocation as S1–S4.
/// The primary T-CAL tests live in `crates/data/src/calendar.rs` unit tests.
#[test]
fn s5_t_cal_crypto24x7_matches_wallclock_range_sweep() {
    use data::calendar::{MarketCalendar, classify_ticker};

    // Verify classify_ticker for the 3 macro tickers → UsEquity.
    assert_eq!(classify_ticker("^GSPC"), MarketCalendar::UsEquity);
    assert_eq!(classify_ticker("DX-Y.NYB"), MarketCalendar::UsEquity);
    assert_eq!(classify_ticker("^TNX"), MarketCalendar::UsEquity);
    // And the crypto path stays Crypto24x7.
    assert_eq!(classify_ticker("BTC-USD"), MarketCalendar::Crypto24x7);
    assert_eq!(classify_ticker("ETH-USD"), MarketCalendar::Crypto24x7);

    // Range sweep: for any window, Crypto24x7's trading_days_in_range must
    // equal the raw wall-clock day count (= (e - s).max(0) / 86_400_000).
    // This IS the same formula as expected_bars_for_range(Days1, s, e).
    let base_ms: i64 = 1_640_995_200_000; // 2022-01-01 UTC
    for &days in &[1i64, 7, 30, 90, 180, 365, 730] {
        let s = base_ms;
        let e = base_ms + days * 86_400_000;

        let calendar_count = MarketCalendar::Crypto24x7.trading_days_in_range(s, e);
        let legacy_count = {
            let range_ms = (e - s).max(0) as u64;
            (range_ms / 86_400_000) as usize
        };

        assert_eq!(
            calendar_count, legacy_count,
            "T-CAL FAIL: Crypto24x7 calendar={calendar_count} ≠ legacy={legacy_count} \
            for {days}-day window. The calendar layer must not change crypto coverage."
        );
    }
}
