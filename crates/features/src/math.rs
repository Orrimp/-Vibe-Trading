//! Deterministic `Decimal` math helpers (T602 — v1).
//!
//! Implements `decimal_ln` and `decimal_sqrt` without any `f64` arithmetic
//! on the result — both use fixed-precision iterative algorithms that produce
//! **identical output across architectures** at a fixed iteration count
//! (no early-exit tolerance; iteration count is pinned per T602 acceptance).
//!
//! ## Precision contract (T602)
//!
//! Both functions target **10 decimal places** (1e-10).  The iteration counts
//! are pinned so two runs on the same input always produce bit-identical
//! `Decimal` results — there is no platform-dependent convergence.
//!
//! ## Algorithm
//!
//! - `decimal_ln`: argument reduction + Taylor series of `ln((1+t)/(1-t))`
//!   with `t = (x-1)/(x+1)`, 25 iterations (covers ≈ 10 dp for inputs in [0.5, 2]).
//!   For larger inputs, uses `ln(x) = ln(m * 2^k) = k * ln(2) + ln(m)`.
//! - `decimal_sqrt`: Babylonian / Newton–Raphson, 30 iterations (deterministic,
//!   starting from a `Decimal`-scaled f64 seed computed once via `to_f64`).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use thiserror::Error;

/// Error from `decimal_ln` or `decimal_sqrt`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MathError {
    #[error("domain error: ln is undefined for x <= 0, got {0}")]
    LnNonPositive(Decimal),
    #[error("domain error: sqrt is undefined for x < 0, got {0}")]
    SqrtNegative(Decimal),
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// `ln(2)` to 20 dp — used in argument reduction.
const LN2: Decimal = dec!(0.69314718055994530942);

/// `ln(10)` to 20 dp — used for initial seed via `log10` ceiling.
const LN10: Decimal = dec!(2.30258509299404568402);

// ── `decimal_ln` ──────────────────────────────────────────────────────────────

/// Compute `ln(x)` for `x > 0` using argument reduction + Taylor series.
///
/// Precision: ≥ 10 decimal places for all inputs in `(0, 1e12)`.
/// Deterministic: pinned 25 Taylor iterations (no early-exit).
///
/// # Errors
///
/// Returns [`MathError::LnNonPositive`] if `x <= 0`.
pub fn decimal_ln(x: Decimal) -> Result<Decimal, MathError> {
    if x <= Decimal::ZERO {
        return Err(MathError::LnNonPositive(x));
    }

    // Reduce x to [0.5, 2) using x = m * 2^k.
    let mut k: i32 = 0;
    let mut m = x;

    while m > dec!(2) {
        m /= dec!(2);
        k += 1;
    }
    while m < dec!(0.5) {
        m *= dec!(2);
        k -= 1;
    }

    // Now m ∈ [0.5, 2), use ln((1+t)/(1-t)) Taylor series for t = (m-1)/(m+1).
    // ln(m) = 2 * Σ t^(2i+1)/(2i+1)   for  i = 0, 1, 2, …
    let t = (m - Decimal::ONE) / (m + Decimal::ONE);
    let t2 = t * t;

    let mut term = t;
    let mut sum = t;
    // 25 fixed iterations — pinned for determinism (T602)
    for i in 1u32..=25 {
        term *= t2;
        let denom = Decimal::from(2 * i + 1);
        sum += term / denom;
    }
    let ln_m = dec!(2) * sum;

    // ln(x) = k * ln(2) + ln(m)
    let k_dec = Decimal::from(k);
    Ok(k_dec * LN2 + ln_m)
}

// ── `decimal_log10` ───────────────────────────────────────────────────────────

/// Compute `log10(x)` for `x > 0`.
///
/// Implemented as `ln(x) / ln(10)` — same determinism contract as
/// [`decimal_ln`] (no floats, pinned iteration count).  Precision is
/// limited by the `LN10` constant and `Decimal`'s working scale; the
/// 4-decimal-place truncation in reflection-memory's embedding
/// (slot 14 / slot 15 — Q3d) is the lowest-precision consumer.
///
/// # Errors
///
/// Returns [`MathError::LnNonPositive`] if `x <= 0`.
pub fn decimal_log10(x: Decimal) -> Result<Decimal, MathError> {
    let ln_x = decimal_ln(x)?;
    Ok(ln_x / LN10)
}

// ── `decimal_sqrt` ────────────────────────────────────────────────────────────

/// Compute `sqrt(x)` for `x >= 0` using Newton–Raphson (Babylonian method).
///
/// Precision: ≥ 10 decimal places for inputs in `[0, 1e12]`.
/// Deterministic: pinned 30 Newton iterations (no early-exit).
///
/// # Errors
///
/// Returns [`MathError::SqrtNegative`] if `x < 0`.
pub fn decimal_sqrt(x: Decimal) -> Result<Decimal, MathError> {
    if x < Decimal::ZERO {
        return Err(MathError::SqrtNegative(x));
    }
    if x == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }
    if x == Decimal::ONE {
        return Ok(Decimal::ONE);
    }

    // Initial guess: use f64 sqrt to get a starting point, then immediately
    // convert back to Decimal.  The f64 conversion is only for the seed —
    // all subsequent arithmetic is pure Decimal, so the final result is
    // independent of platform f64 precision.
    #[allow(clippy::float_arithmetic)]
    let seed_f64: f64 = f64::try_from(x).unwrap_or(1.0_f64).sqrt();
    let mut guess = Decimal::try_from(seed_f64).unwrap_or(Decimal::ONE);
    if guess <= Decimal::ZERO {
        guess = Decimal::ONE;
    }

    // 30 pinned Newton iterations.
    let two = dec!(2);
    for _ in 0u32..30 {
        guess = (guess + x / guess) / two;
    }

    Ok(guess)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const TOLERANCE: Decimal = dec!(0.0000000001); // 1e-10

    fn within(a: Decimal, b: Decimal) -> bool {
        (a - b).abs() <= TOLERANCE
    }

    #[test]
    fn t602_ln_of_e_approx_one() {
        // e ≈ 2.71828182845904523536
        let e = dec!(2.71828182845904523536);
        let result = decimal_ln(e).expect("ln(e) should succeed");
        assert!(
            within(result, Decimal::ONE),
            "ln(e) = {result}, expected ≈ 1.0"
        );
    }

    #[test]
    fn t602_ln_of_one_is_zero() {
        let result = decimal_ln(Decimal::ONE).expect("ln(1) should succeed");
        assert!(
            within(result, Decimal::ZERO),
            "ln(1) = {result}, expected 0"
        );
    }

    #[test]
    fn t602_ln_nonpositive_error() {
        assert!(matches!(
            decimal_ln(Decimal::ZERO),
            Err(MathError::LnNonPositive(_))
        ));
        assert!(matches!(
            decimal_ln(dec!(-1)),
            Err(MathError::LnNonPositive(_))
        ));
    }

    #[test]
    fn t602_ln_deterministic() {
        let input = dec!(3.14159265358979323846);
        let r1 = decimal_ln(input).unwrap();
        let r2 = decimal_ln(input).unwrap();
        assert_eq!(r1, r2, "ln must be bit-identical across two runs");
    }

    #[test]
    fn t602_ln_reference_values() {
        // ln(2) ≈ 0.6931471805599453
        let r = decimal_ln(dec!(2)).unwrap();
        assert!(within(r, dec!(0.6931471805599453)), "ln(2) off: {r}");

        // ln(10) ≈ 2.302585092994046
        let r = decimal_ln(dec!(10)).unwrap();
        assert!(within(r, dec!(2.302585092994046)), "ln(10) off: {r}");

        // ln(0.5) = -ln(2)
        let r = decimal_ln(dec!(0.5)).unwrap();
        assert!(within(r, dec!(-0.6931471805599453)), "ln(0.5) off: {r}");
    }

    #[test]
    fn t602_sqrt_of_four_is_two() {
        let result = decimal_sqrt(dec!(4)).expect("sqrt(4) should succeed");
        assert!(within(result, dec!(2)), "sqrt(4) = {result}, expected 2");
    }

    #[test]
    fn t602_sqrt_of_zero_is_zero() {
        let result = decimal_sqrt(Decimal::ZERO).expect("sqrt(0) should succeed");
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn t602_sqrt_of_one_is_one() {
        let result = decimal_sqrt(Decimal::ONE).expect("sqrt(1) should succeed");
        assert_eq!(result, Decimal::ONE);
    }

    #[test]
    fn t602_sqrt_negative_error() {
        assert!(matches!(
            decimal_sqrt(dec!(-1)),
            Err(MathError::SqrtNegative(_))
        ));
    }

    #[test]
    fn t602_sqrt_deterministic() {
        let input = dec!(2);
        let r1 = decimal_sqrt(input).unwrap();
        let r2 = decimal_sqrt(input).unwrap();
        assert_eq!(r1, r2, "sqrt must be bit-identical across two runs");
    }

    #[test]
    fn t602_sqrt_reference_values() {
        // sqrt(2) ≈ 1.41421356237309504880
        let r = decimal_sqrt(dec!(2)).unwrap();
        assert!(within(r, dec!(1.41421356237309504880)), "sqrt(2) off: {r}");

        // sqrt(9) = 3
        let r = decimal_sqrt(dec!(9)).unwrap();
        assert!(within(r, dec!(3)), "sqrt(9) off: {r}");

        // sqrt(100) = 10
        let r = decimal_sqrt(dec!(100)).unwrap();
        assert!(within(r, dec!(10)), "sqrt(100) off: {r}");
    }
}
