//! Cross-sectional momentum score (T603 — v1).
//!
//! `score_vol_adjusted_return` computes the vol-adjusted log return per R3.1/R3.2:
//!
//! ```text
//! score(s, t) = ln(close[t] / close[t-n]) / realized_vol(close[s], n)
//! realized_vol = std(ln(close[t-i] / close[t-i-1]) for i in 0..n)
//! ```
//!
//! All arithmetic is pure `Decimal` — no `f64` per the v0 float-arithmetic deny.
//! Uses `features::ring_buffer::RingBuffer` for O(1) window access.

use rust_decimal::Decimal;
use thiserror::Error;

use crate::math::{decimal_ln, decimal_sqrt, MathError};
use crate::ring_buffer::RingBuffer;

/// Error from `score_vol_adjusted_return`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScoreError {
    #[error("insufficient history: need at least {needed} bars, have {have}")]
    InsufficientHistory { needed: usize, have: usize },
    #[error("empty ring buffer")]
    Empty,
    #[error("math error: {0}")]
    Math(#[from] MathError),
    #[error("zero or near-zero price in history")]
    ZeroPrice,
}

/// Compute the vol-adjusted log return score for a single symbol (R3.1, R3.2).
///
/// The ring buffer must contain at least `n + 1` values (the current close plus
/// `n` prior closes) so that `n` log-returns can be computed.  Returns
/// `Err(ScoreError::InsufficientHistory)` if the buffer is not yet warm enough.
///
/// # Arguments
///
/// - `history`: ring buffer of close prices (most-recent last via `push`).
/// - `n`: lookback window in bars (default 60 for 1-hour at 1m granularity).
/// - `vol_floor`: minimum realized vol to avoid divide-by-zero on stalled tape
///   (default `1e-6`).
///
/// # Errors
///
/// Returns [`ScoreError`] on insufficient history, math domain errors, or
/// zero prices in the history window.
pub fn score_vol_adjusted_return(
    history: &RingBuffer,
    n: u32,
    vol_floor: Decimal,
) -> Result<Decimal, ScoreError> {
    let needed = n as usize + 1;
    if history.len() < needed {
        return Err(ScoreError::InsufficientHistory {
            needed,
            have: history.len(),
        });
    }

    // R3.1: log return over n bars.
    let close_now = history.last().ok_or(ScoreError::Empty)?;
    let close_back = history.get_back(n as usize).ok_or(ScoreError::Empty)?;

    if close_back <= Decimal::ZERO || close_now <= Decimal::ZERO {
        return Err(ScoreError::ZeroPrice);
    }

    let log_return = decimal_ln(close_now / close_back)?;

    // R3.2: realized vol = std of log returns over the n-bar window.
    let mut log_rets = Vec::with_capacity(n as usize);
    for i in 0..(n as usize) {
        let now_val = history.get_back(i).ok_or(ScoreError::Empty)?;
        let prev_val = history.get_back(i + 1).ok_or(ScoreError::Empty)?;
        if prev_val <= Decimal::ZERO || now_val <= Decimal::ZERO {
            return Err(ScoreError::ZeroPrice);
        }
        log_rets.push(decimal_ln(now_val / prev_val)?);
    }

    let realized_vol = decimal_std(&log_rets)?.max(vol_floor);

    Ok(log_return / realized_vol)
}

/// Standard deviation of a slice of `Decimal` values (population std dev).
///
/// # Errors
///
/// Returns [`ScoreError::InsufficientHistory`] if the slice is empty.
/// Returns [`ScoreError::Math`] on sqrt domain error.
pub fn decimal_std(values: &[Decimal]) -> Result<Decimal, ScoreError> {
    if values.is_empty() {
        return Err(ScoreError::InsufficientHistory { needed: 1, have: 0 });
    }

    let n = Decimal::from(values.len());
    let mean = values.iter().copied().sum::<Decimal>() / n;
    let variance = values
        .iter()
        .map(|v| (*v - mean) * (*v - mean))
        .sum::<Decimal>()
        / n;

    let std = decimal_sqrt(variance)?;
    Ok(std)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::float_arithmetic)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal_macros::dec;

    fn make_history(closes: &[f64]) -> RingBuffer {
        let mut rb = RingBuffer::new(closes.len().max(1));
        for &c in closes {
            rb.push(Decimal::try_from(c).expect("test f64 to Decimal"));
        }
        rb
    }

    #[test]
    fn t603_insufficient_history_error() {
        let mut rb = RingBuffer::new(10);
        // Push only 5 values, need n+1=61 for n=60
        for i in 1u32..=5 {
            rb.push(Decimal::from(i * 100));
        }
        let result = score_vol_adjusted_return(&rb, 60, dec!(0.000001));
        assert!(
            matches!(result, Err(ScoreError::InsufficientHistory { .. })),
            "expected InsufficientHistory"
        );
    }

    #[test]
    fn t603_score_on_200_bar_warmup() {
        // 200-bar synthetic ascending series: close[i] = 10000 + i * 10
        let n: u32 = 60;
        let count = 200usize;
        let mut rb = RingBuffer::new(count);
        for i in 0..count {
            rb.push(Decimal::from(10_000u32 + (i as u32) * 10));
        }
        let vol_floor = dec!(0.000001);
        let score =
            score_vol_adjusted_return(&rb, n, vol_floor).expect("should succeed with 200 bars");
        // For a strictly increasing series the score should be positive
        assert!(
            score > Decimal::ZERO,
            "positive trend → positive score, got {score}"
        );
    }

    #[test]
    fn t603_determinism() {
        let n: u32 = 10;
        let closes = vec![
            100.0_f64, 101.0, 102.0, 103.0, 104.0, 105.0, 104.0, 103.0, 102.0, 101.0, 100.0,
        ];
        let rb = make_history(&closes);
        let s1 = score_vol_adjusted_return(&rb, n, dec!(0.000001)).unwrap();
        let s2 = score_vol_adjusted_return(&rb, n, dec!(0.000001)).unwrap();
        assert_eq!(s1, s2, "score must be bit-identical across runs");
    }

    #[test]
    fn t603_empty_history_errors() {
        let rb = RingBuffer::new(5);
        let result = score_vol_adjusted_return(&rb, 3, dec!(0.000001));
        assert!(matches!(
            result,
            Err(ScoreError::InsufficientHistory { .. })
        ));
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(500))]
        #[test]
        fn t603_strictly_increasing_gives_positive_score(
            // 62 prices, all strictly increasing from 100 to 100+62
            start in 100u32..500u32,
        ) {
            let n: u32 = 60;
            let mut rb = RingBuffer::new(62);
            for i in 0u32..62 {
                rb.push(Decimal::from(start + i));
            }
            let score = score_vol_adjusted_return(&rb, n, dec!(0.000001));
            prop_assert!(
                matches!(score, Ok(s) if s > Decimal::ZERO),
                "strictly increasing → positive score"
            );
        }
    }
}
