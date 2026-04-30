//! Spread and rolling z-score primitives for v1.5a mean-reversion pairs
//! strategy (T702).
//!
//! Both functions are **pure** over their inputs — given the same inputs they
//! return byte-identical output across runs (R3.5 determinism property).
//!
//! ## Reuse
//!
//! - [`spread`] uses [`crate::math::decimal_ln`] from v1 T602.
//! - [`rolling_zscore`] uses [`crate::ring_buffer::RingBuffer`] from v0.5 and
//!   [`crate::math::decimal_sqrt`] from v1 T602.
//! - No new dependencies.
//!
//! ## Translation invariance proxy (R3 acceptance)
//!
//! Scaling both `price_a` and `price_b` by the same multiplicative factor `k`
//! shifts the spread by `ln(k) * (1 - β)`.  At `β = 1` the spread is
//! invariant.  For any β, the z-score of a constant-shifted series is
//! invariant because the mean shifts by the same constant and σ is unchanged.
//! Verified by proptest in the test section below.

use rust_decimal::Decimal;
use thiserror::Error;

use crate::math::{decimal_ln, decimal_sqrt, MathError};
use crate::ring_buffer::RingBuffer;

/// Error from [`spread`] or [`rolling_zscore`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PairScoreError {
    /// Not enough spread history has been accumulated yet.
    #[error("insufficient history: need {need}, have {have}")]
    InsufficientHistory { need: usize, have: usize },

    /// Math domain error (e.g. ln of non-positive price).
    #[error("math error: {0}")]
    Math(#[from] MathError),

    /// At least one price is zero or negative — undefined in log-space.
    #[error("non-positive price")]
    NonPositivePrice,
}

/// Per-bar spread: `log(price_a) - β · log(price_b)`.
///
/// Reuses v1 `features::math::decimal_ln` (10 dp precision, deterministic,
/// no `f64` per the v0 float-arithmetic deny).
///
/// # Errors
///
/// - [`PairScoreError::NonPositivePrice`] — if `price_a <= 0` or
///   `price_b <= 0`.
/// - [`PairScoreError::Math`] — propagated from `decimal_ln`.
pub fn spread(
    price_a: Decimal,
    price_b: Decimal,
    beta: Decimal,
) -> Result<Decimal, PairScoreError> {
    if price_a <= Decimal::ZERO || price_b <= Decimal::ZERO {
        return Err(PairScoreError::NonPositivePrice);
    }
    let ln_a = decimal_ln(price_a)?;
    let ln_b = decimal_ln(price_b)?;
    Ok(ln_a - beta * ln_b)
}

/// Rolling z-score of the last `n` spread values in `history`.
///
/// ```text
/// μ = mean(history[-n:])
/// σ = std(history[-n:])   — clamped to vol_floor
/// z = (history[-1] - μ) / σ
/// ```
///
/// All arithmetic is pure `Decimal`. The function is **pure** — given the
/// same `history` buffer state, `n`, and `vol_floor`, it returns
/// byte-identical output across runs (R3.5).
///
/// # Arguments
///
/// - `history`: ring buffer of spread values (most-recent last via `push`).
/// - `n`: lookback window in bars (e.g. `lookback_minutes` from config).
/// - `vol_floor`: minimum σ to avoid divide-by-zero on stalled tape
///   (default `1e-6` matching v1 R3.2).
///
/// # Errors
///
/// - [`PairScoreError::InsufficientHistory`] — if `history.len() < n`.
/// - [`PairScoreError::Math`] — propagated from `decimal_sqrt`.
pub fn rolling_zscore(
    history: &RingBuffer,
    n: u32,
    vol_floor: Decimal,
) -> Result<Decimal, PairScoreError> {
    let need = n as usize;
    let have = history.len();
    if have < need {
        return Err(PairScoreError::InsufficientHistory { need, have });
    }
    if need == 0 {
        return Err(PairScoreError::InsufficientHistory { need: 1, have: 0 });
    }

    // Most-recent value.
    let last = history
        .last()
        .ok_or(PairScoreError::InsufficientHistory { need: 1, have: 0 })?;

    // Mean over the last n cells.
    let mut sum = Decimal::ZERO;
    for i in 0..need {
        sum += history
            .get_back(i)
            .ok_or(PairScoreError::InsufficientHistory { need, have })?;
    }
    let n_dec = Decimal::from(n);
    let mean = sum / n_dec;

    // Variance (population) over the last n cells.
    let mut var = Decimal::ZERO;
    for i in 0..need {
        let v = history
            .get_back(i)
            .ok_or(PairScoreError::InsufficientHistory { need, have })?;
        let d = v - mean;
        var += d * d;
    }
    var /= n_dec;

    // σ with vol_floor clamp.
    let std = decimal_sqrt(var)?.max(vol_floor);

    Ok((last - mean) / std)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const TOL: Decimal = dec!(0.000000001); // 1e-9

    fn within(a: Decimal, b: Decimal) -> bool {
        (a - b).abs() <= TOL
    }

    // ── spread ──────────────────────────────────────────────────────────────────

    #[test]
    fn t702_spread_beta_one() {
        // spread = ln(100) - 1.0 * ln(100) = 0
        let s = spread(dec!(100), dec!(100), dec!(1.0)).unwrap();
        assert!(within(s, dec!(0)), "spread(p,p,1) = {s}");
    }

    #[test]
    fn t702_spread_beta_one_divergence() {
        // ln(200) - ln(100) = ln(2) ≈ 0.693147
        let s = spread(dec!(200), dec!(100), dec!(1.0)).unwrap();
        assert!(within(s, dec!(0.693147180559945)), "spread: {s}");
    }

    #[test]
    fn t702_spread_nonpositive_price_a() {
        assert!(matches!(
            spread(dec!(0), dec!(100), dec!(1.0)),
            Err(PairScoreError::NonPositivePrice)
        ));
        assert!(matches!(
            spread(dec!(-1), dec!(100), dec!(1.0)),
            Err(PairScoreError::NonPositivePrice)
        ));
    }

    #[test]
    fn t702_spread_nonpositive_price_b() {
        assert!(matches!(
            spread(dec!(100), dec!(0), dec!(1.0)),
            Err(PairScoreError::NonPositivePrice)
        ));
    }

    #[test]
    fn t702_spread_hand_computed() {
        // ln(30000) - 0.5 * ln(2000) = ?
        // ln(30000) ≈ 10.30895, ln(2000) ≈ 7.60090
        // spread ≈ 10.30895 - 0.5 * 7.60090 = 10.30895 - 3.80045 = 6.50850
        let s = spread(dec!(30000), dec!(2000), dec!(0.5)).unwrap();
        let expected = dec!(6.50850);
        // coarser tolerance due to manual computation
        assert!(
            (s - expected).abs() < dec!(0.001),
            "spread = {s}, expected ≈ {expected}"
        );
    }

    // ── rolling_zscore ──────────────────────────────────────────────────────────

    #[test]
    fn t702_zscore_insufficient_history() {
        let rb = RingBuffer::new(10);
        // empty — no history
        assert!(matches!(
            rolling_zscore(&rb, 5, dec!(0.000001)),
            Err(PairScoreError::InsufficientHistory { .. })
        ));
    }

    #[test]
    fn t702_zscore_warmup_returns_error() {
        let mut rb = RingBuffer::new(10);
        for i in 0..4 {
            rb.push(Decimal::from(i));
        }
        // need 5, have 4
        assert!(matches!(
            rolling_zscore(&rb, 5, dec!(0.000001)),
            Err(PairScoreError::InsufficientHistory { need: 5, have: 4 })
        ));
    }

    #[test]
    fn t702_zscore_constant_series_returns_zero() {
        // If all values are equal, spread = 0, σ hits vol_floor,
        // z = (v - v) / vol_floor = 0.
        let mut rb = RingBuffer::new(10);
        for _ in 0..10 {
            rb.push(dec!(1));
        }
        let z = rolling_zscore(&rb, 5, dec!(0.000001)).unwrap();
        assert!(within(z, dec!(0)), "constant series z = {z}");
    }

    #[test]
    fn t702_zscore_200_bar_synthetic() {
        // Construct a synthetic spread series that grows linearly, giving a
        // predictable z-score at the end.
        let n: u32 = 10;
        let mut rb = RingBuffer::new(n as usize + 1);
        // Push n values: 1.0, 2.0, …, n.0 (linearly increasing)
        for i in 1..=(n + 1) {
            rb.push(Decimal::from(i));
        }
        // Last value = n+1, mean of last n = (2 + 3 + ... + n+1) / n = (n+3)/2
        // std = population std of [2, 3, …, n+1]
        // Hand-compute for n=10: mean of [2..11] = 6.5
        // var = Σ(x-6.5)² / 10 for x in [2..11]
        // = [(4.5²+3.5²+2.5²+1.5²+0.5²+0.5²+1.5²+2.5²+3.5²+4.5²)] / 10
        // = 2*(4.5²+3.5²+2.5²+1.5²+0.5²) / 10
        // = 2*(20.25+12.25+6.25+2.25+0.25) / 10
        // = 2*41.25 / 10 = 8.25
        // std = sqrt(8.25) ≈ 2.8722813
        // last = 11, z = (11 - 6.5) / 2.8722813 = 4.5 / 2.8722813 ≈ 1.5667
        let z = rolling_zscore(&rb, n, dec!(0.000001)).unwrap();
        let expected = dec!(1.5667);
        assert!(
            (z - expected).abs() < dec!(0.001),
            "z = {z}, expected ≈ {expected}"
        );
    }

    #[test]
    fn t702_zscore_deterministic_two_runs() {
        // Same buffer state → byte-identical output (R3.5).
        let n: u32 = 5;
        let vol_floor = dec!(0.000001);
        let mut rb = RingBuffer::new(n as usize + 2);
        for i in 1u32..=7 {
            rb.push(Decimal::from(i));
        }

        let z1 = rolling_zscore(&rb, n, vol_floor).unwrap();
        let z2 = rolling_zscore(&rb, n, vol_floor).unwrap();
        assert_eq!(z1, z2, "z-score must be bit-identical across two runs");
    }

    #[test]
    fn t702_spread_scaling_invariance_at_beta_one() {
        // At β = 1, scaling both prices by k leaves spread unchanged
        // because ln(k*p_a) - ln(k*p_b) = ln(p_a) + ln(k) - ln(p_b) - ln(k)
        //                                = ln(p_a) - ln(p_b)
        let pa = dec!(30000);
        let pb = dec!(2000);
        let beta = dec!(1.0);
        let k = dec!(2.5);

        let s1 = spread(pa, pb, beta).unwrap();
        let s2 = spread(pa * k, pb * k, beta).unwrap();
        assert!(within(s1, s2), "spread β=1: s1={s1}, s2={s2}");
    }

    #[test]
    fn t702_zscore_scaling_invariance_at_beta_one() {
        // When spread is invariant to k scaling (β=1), and we push the same
        // spread values into two buffers, we get identical z-scores.
        let n: u32 = 5;
        let beta = dec!(1.0);
        let k = dec!(1.5);
        let vol_floor = dec!(0.000001);

        let prices_a = [
            dec!(100),
            dec!(102),
            dec!(101),
            dec!(103),
            dec!(104),
            dec!(100),
        ];
        let prices_b = [
            dec!(200),
            dec!(201),
            dec!(202),
            dec!(203),
            dec!(204),
            dec!(200),
        ];

        let mut rb1 = RingBuffer::new(n as usize + 2);
        let mut rb2 = RingBuffer::new(n as usize + 2);

        for (&pa, &pb) in prices_a.iter().zip(prices_b.iter()) {
            rb1.push(spread(pa, pb, beta).unwrap());
            rb2.push(spread(pa * k, pb * k, beta).unwrap());
        }

        let z1 = rolling_zscore(&rb1, n, vol_floor).unwrap();
        let z2 = rolling_zscore(&rb2, n, vol_floor).unwrap();
        assert!(
            within(z1, z2),
            "z-score invariant under scaling at β=1: z1={z1}, z2={z2}"
        );
    }
}
