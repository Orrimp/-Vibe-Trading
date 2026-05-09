//! T1802 / R1.4 — outcome classifier acceptance.
//!
//! 5 cases per Design → test strategy:
//! - +0.6% → Win
//! - -0.6% → Loss
//! - +0.4% → Scratch
//! - -0.4% → Scratch
//! - opening_capital == 0 → Scratch (defensive)

use reflection::outcome::{classify_outcome, OutcomeClass, OUTCOME_THRESHOLD_PCT};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Money, Usdt};

#[test]
fn t1802_win_at_plus_0_6_percent() {
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(60)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(r, OutcomeClass::Win);
    assert_eq!(format!("{r}"), "Win");
}

#[test]
fn t1802_loss_at_minus_0_6_percent() {
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(-60)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(r, OutcomeClass::Loss);
    assert_eq!(format!("{r}"), "Loss");
}

#[test]
fn t1802_scratch_at_plus_0_4_percent() {
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(40)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(r, OutcomeClass::Scratch);
    assert_eq!(format!("{r}"), "Scratch");
}

#[test]
fn t1802_scratch_at_minus_0_4_percent() {
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(-40)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(r, OutcomeClass::Scratch);
}

#[test]
fn t1802_scratch_when_opening_capital_zero() {
    // Defensive — denominator-zero is no-signal, not an error.
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(100)),
        Money::<Usdt>::from_decimal(dec!(0)),
    );
    assert_eq!(r, OutcomeClass::Scratch);
}

#[test]
fn t1802_threshold_constant_pinned_at_half_percent() {
    assert_eq!(OUTCOME_THRESHOLD_PCT, dec!(0.005));
}

#[test]
fn t1802_classify_outcome_byte_stable() {
    // Determinism gate: same fixture in twice → byte-identical output.
    let a = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(60)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    let b = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(60)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(a, b);
    assert_eq!(format!("{a}"), format!("{b}"));
}

#[test]
fn t1802_boundary_at_exactly_plus_threshold_is_scratch() {
    // ratio = 0.005 (not > 0.005) → Scratch.
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(50)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(r, OutcomeClass::Scratch);
}

#[test]
fn t1802_boundary_at_exactly_minus_threshold_is_scratch() {
    let r = classify_outcome(
        Money::<Usdt>::from_decimal(dec!(-50)),
        Money::<Usdt>::from_decimal(dec!(10000)),
    );
    assert_eq!(r, OutcomeClass::Scratch);
}

#[test]
fn _ignore_decimal_unused() {
    // Prevent rustc warn on Decimal import in case of future re-shape.
    let _: Decimal = dec!(1);
}
