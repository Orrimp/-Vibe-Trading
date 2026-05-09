//! Regime classifier (Q3b — analyst strawman pinned).
//!
//! Pure function over a BTC daily-close series.  No I/O, no clock,
//! no `f64`.  Boundary at exactly ±2% maps to `Chop` (strict
//! inequality, R1.3).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use trading_core::Timestamp;

/// Regime tag — one of three discrete buckets.
///
/// `Display` emits `bull|bear|chop` (lowercase, no quotes) — body byte
/// stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RegimeTag {
    Bull,
    Bear,
    Chop,
}

impl std::fmt::Display for RegimeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Bull => "bull",
            Self::Bear => "bear",
            Self::Chop => "chop",
        };
        f.write_str(s)
    }
}

/// Classifier error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegimeError {
    /// `btc_closes` is empty or contains no sample at-or-before the
    /// requested timestamp.
    #[error("no BTC close found at or before requested timestamp")]
    NoCloseAtTimestamp,
    /// No close found 7 days prior to the requested timestamp.
    #[error("no BTC close found at or before t-7d")]
    NoCloseAtMinus7d,
    /// Reference close was zero — refuse to compute the ratio.
    #[error("BTC close at t-7d was zero")]
    ZeroReferenceClose,
}

/// Threshold ±2% — analyst strawman pinned (R1.3).  Pinned as a
/// `pub const` so a future architect grep-changes in one place.
pub const REGIME_THRESHOLD_RATIO: Decimal = dec!(0.02);

/// Classify the BTC regime at `at` using the trailing 7d return.
///
/// Inputs:
/// - `btc_closes`: an ascending-by-`Timestamp` slice of `(ts, close)`
///   pairs (the agent's BTC daily closes — `Decimal` only, no `f64`).
/// - `at`: the timestamp for which to classify.
///
/// Logic:
/// - `Bull`  iff `(close[at] - close[at - 7d]) / close[at - 7d] > +0.02`,
/// - `Bear`  iff same ratio `< -0.02`,
/// - `Chop`  otherwise (boundary at exactly ±0.02 is `Chop`).
///
/// # Errors
///
/// - [`RegimeError::NoCloseAtTimestamp`] — no sample at or before `at`.
/// - [`RegimeError::NoCloseAtMinus7d`] — no sample at or before `at - 7d`.
/// - [`RegimeError::ZeroReferenceClose`] — t-7d close was zero.
pub fn classify_regime(
    btc_closes: &[(Timestamp, Decimal)],
    at: Timestamp,
) -> Result<RegimeTag, RegimeError> {
    let close_at = latest_at_or_before(btc_closes, at).ok_or(RegimeError::NoCloseAtTimestamp)?;

    let seven_d_inner = at.inner() - time::Duration::days(7);
    let at_minus_7d = Timestamp::new(seven_d_inner);
    let close_minus_7d =
        latest_at_or_before(btc_closes, at_minus_7d).ok_or(RegimeError::NoCloseAtMinus7d)?;

    if close_minus_7d == Decimal::ZERO {
        return Err(RegimeError::ZeroReferenceClose);
    }

    let ratio = (close_at - close_minus_7d) / close_minus_7d;
    if ratio > REGIME_THRESHOLD_RATIO {
        Ok(RegimeTag::Bull)
    } else if ratio < -REGIME_THRESHOLD_RATIO {
        Ok(RegimeTag::Bear)
    } else {
        Ok(RegimeTag::Chop)
    }
}

/// Return the most-recent close at or before `at`, or `None` if no
/// such sample exists.  Linear scan — `btc_closes` is small (≤ 7
/// daily samples in production callers).
fn latest_at_or_before(closes: &[(Timestamp, Decimal)], at: Timestamp) -> Option<Decimal> {
    closes
        .iter()
        .filter(|(ts, _)| *ts <= at)
        .max_by_key(|(ts, _)| *ts)
        .map(|(_, c)| *c)
}
