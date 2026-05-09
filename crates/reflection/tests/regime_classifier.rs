//! T1802 / R1.3 — regime classifier acceptance.
//!
//! Boundary at exactly ±2% maps to `Chop` (strict inequality).
//! Bull / Bear / Chop cases + boundary case + determinism gate
//! (same fixture in twice → byte-identical output).

use reflection::regime::{classify_regime, RegimeError, RegimeTag};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::Timestamp;

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn day(n: i64) -> i64 {
    1_700_000_000 + n * 86_400
}

fn build_btc_closes(closes: &[Decimal]) -> Vec<(Timestamp, Decimal)> {
    closes
        .iter()
        .enumerate()
        .map(|(i, c)| (ts(day(i as i64)), *c))
        .collect()
}

#[test]
fn t1802_bull_when_seven_d_return_above_two_percent() {
    // close[0] = 100, close[7] = 103 (3% return) → Bull
    let mut closes = vec![dec!(100); 8];
    closes[7] = dec!(103);
    let series = build_btc_closes(&closes);
    let r = classify_regime(&series, ts(day(7))).expect("regime ok");
    assert_eq!(r, RegimeTag::Bull);
}

#[test]
fn t1802_bear_when_seven_d_return_below_minus_two_percent() {
    // close[0] = 100, close[7] = 97 (-3% return) → Bear
    let mut closes = vec![dec!(100); 8];
    closes[7] = dec!(97);
    let series = build_btc_closes(&closes);
    let r = classify_regime(&series, ts(day(7))).expect("regime ok");
    assert_eq!(r, RegimeTag::Bear);
}

#[test]
fn t1802_chop_when_seven_d_return_within_two_percent() {
    // close[0] = 100, close[7] = 101 (+1%) → Chop
    let mut closes = vec![dec!(100); 8];
    closes[7] = dec!(101);
    let series = build_btc_closes(&closes);
    let r = classify_regime(&series, ts(day(7))).expect("regime ok");
    assert_eq!(r, RegimeTag::Chop);
}

#[test]
fn t1802_boundary_at_exactly_plus_two_percent_is_chop() {
    // close[0] = 100, close[7] = 102 → ratio = 0.02 (NOT > 0.02) → Chop
    let mut closes = vec![dec!(100); 8];
    closes[7] = dec!(102);
    let series = build_btc_closes(&closes);
    let r = classify_regime(&series, ts(day(7))).expect("regime ok");
    assert_eq!(r, RegimeTag::Chop);
}

#[test]
fn t1802_boundary_at_exactly_minus_two_percent_is_chop() {
    // close[0] = 100, close[7] = 98 → ratio = -0.02 (NOT < -0.02) → Chop
    let mut closes = vec![dec!(100); 8];
    closes[7] = dec!(98);
    let series = build_btc_closes(&closes);
    let r = classify_regime(&series, ts(day(7))).expect("regime ok");
    assert_eq!(r, RegimeTag::Chop);
}

#[test]
fn t1802_classify_regime_byte_stable() {
    // Determinism gate (R1.3 / R1.4): same fixture in twice → same output.
    let mut closes = vec![dec!(100); 8];
    closes[7] = dec!(103);
    let series = build_btc_closes(&closes);
    let a = classify_regime(&series, ts(day(7))).expect("a");
    let b = classify_regime(&series, ts(day(7))).expect("b");
    assert_eq!(a, b);
    assert_eq!(format!("{a}"), format!("{b}"));
    assert_eq!(format!("{a}"), "bull");
}

#[test]
fn t1802_returns_err_when_no_close_at_minus_7d() {
    // Only one sample → no t-7d.
    let series = vec![(ts(day(7)), dec!(100))];
    let r = classify_regime(&series, ts(day(7)));
    assert_eq!(r, Err(RegimeError::NoCloseAtMinus7d));
}

#[test]
fn t1802_returns_err_when_zero_reference_close() {
    let series = vec![(ts(day(0)), Decimal::ZERO), (ts(day(7)), dec!(100))];
    let r = classify_regime(&series, ts(day(7)));
    assert_eq!(r, Err(RegimeError::ZeroReferenceClose));
}
