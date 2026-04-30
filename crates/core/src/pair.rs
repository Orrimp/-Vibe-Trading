//! Pair types for v1.5a mean-reversion pairs strategy (T701).
//!
//! An ordered tuple `(a, b)` represents a pair where `a` is the **target**
//! leg (traded long-only in v1.5a formulation C) and `b` is the **hedge
//! reference** (price feeds the spread computation; no position opened in
//! v1.5a).
//!
//! ## Invariants
//!
//! - `a != b` enforced by [`PairKey::new`].
//! - `beta > 0` enforced by [`Pair::new`].
//! - `(a, b)` and `(b, a)` are **distinct** because the `a` leg is the
//!   traded leg.
//!
//! ## Determinism
//!
//! All pair-keyed maps in the strategy use `BTreeMap<PairKey, _>` so
//! iteration order is lexicographic (`PairKey` derives `Ord`), matching
//! the R9.3 / R9.4 requirements for byte-identical backtest reports.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::symbol::Symbol;

/// Errors from pair construction or configuration validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PairError {
    /// Both legs refer to the same symbol — pairs must be distinct.
    #[error("degenerate pair: a == b")]
    DegeneratePair,

    /// Beta is non-positive. A non-positive hedge ratio is not a hedge.
    #[error("invalid beta {beta}: must be > 0")]
    InvalidBeta { beta: Decimal },

    /// Beta is outside the sanity range `[0.1, 10]` (R2.3).
    #[error("beta {beta} out of range [0.1, 10]")]
    BetaOutOfRange { beta: Decimal },

    /// USDC-quoted pair — blocked until v1.5b multi-venue ingest (Q5).
    #[error("unsupported quote: USDC pairs require v1.5b multi-venue ingest")]
    UnsupportedQuote,
}

/// Ordered pair key — `(a, b)` is distinct from `(b, a)` because the
/// `a` leg is the traded long-only leg in v1.5a.
///
/// Implements `Ord` so `BTreeMap<PairKey, _>` iteration is lexicographic
/// (R9.3 determinism gate).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PairKey {
    pub a: Symbol,
    pub b: Symbol,
}

impl PairKey {
    /// Create a `PairKey`.
    ///
    /// # Errors
    ///
    /// Returns [`PairError::DegeneratePair`] if `a == b`.
    pub fn new(a: Symbol, b: Symbol) -> Result<Self, PairError> {
        if a == b {
            return Err(PairError::DegeneratePair);
        }
        Ok(Self { a, b })
    }
}

impl std::fmt::Display for PairKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.a, self.b)
    }
}

/// Configured pair: `(a, b, β)`.
///
/// - `a` is the **target** leg (traded long-only in v1.5a).
/// - `b` is the **hedge reference** (price feeds the spread; no position
///   opened in v1.5a).
/// - `beta` is the fixed hedge ratio (R2.1 — default `1.0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    pub key: PairKey,
    /// Fixed hedge ratio β > 0 (R2.1). Default `1.0`.
    pub beta: Decimal,
}

impl Pair {
    /// Construct and validate a `Pair`.
    ///
    /// Sanity range for beta: `0.1 ≤ beta ≤ 10` (R2.3).
    ///
    /// # Errors
    ///
    /// - [`PairError::DegeneratePair`] — if `a == b`.
    /// - [`PairError::InvalidBeta`] — if `beta <= 0`.
    /// - [`PairError::BetaOutOfRange`] — if `beta < 0.1` or `beta > 10`.
    pub fn new(a: Symbol, b: Symbol, beta: Decimal) -> Result<Self, PairError> {
        let key = PairKey::new(a, b)?;
        if beta <= Decimal::ZERO {
            return Err(PairError::InvalidBeta { beta });
        }
        let min_beta = Decimal::new(1, 1); // 0.1
        let max_beta = Decimal::TEN; // 10
        if beta < min_beta || beta > max_beta {
            return Err(PairError::BetaOutOfRange { beta });
        }
        Ok(Self { key, beta })
    }
}

/// Membership row used by `audit::query::pnl_by_pair` to project
/// per-asset P&L into per-pair rows.
///
/// Captured at strategy-load time (R1.2 — pairs are frozen for the run).
/// The `traded_a_asset` field names the **base asset** of the `a` leg
/// (e.g. `"BTC"` for `BTCUSDT`) — this is the account leaf used by
/// `income:realized_pnl` journal entries that `pnl_by_symbol` aggregates.
///
/// ## Multiplicity note (Q9 / architect risk #3)
///
/// When the same `a` symbol appears in more than one pair
/// (e.g. `BTCUSDT` in both `(BTCUSDT, ETHUSDT)` and
/// `(BTCUSDT, SOLUSDT)`), `pnl_by_pair[(a, b)] == pnl_by_symbol[a]`
/// no longer holds for individual pairs — the per-asset P&L is split
/// across both pairs. The `k > 1` multiplicity case is documented here
/// and asserted in the audit integration test (T708).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairMembership {
    /// The `(a, b)` pair this membership row describes.
    pub key: PairKey,
    /// The base asset symbol used in `pnl_by_symbol` attribution.
    /// For `BTCUSDT` this is `Symbol::new("BTCUSDT")` (the traded `a` symbol).
    pub traded_a_symbol: Symbol,
}

impl PairMembership {
    /// Create a `PairMembership` from a `Pair`.
    #[must_use]
    pub fn from_pair(pair: &Pair) -> Self {
        Self {
            key: pair.key.clone(),
            traded_a_symbol: pair.key.a.clone(),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sym(s: &str) -> Symbol {
        Symbol::new(s)
    }

    // ── PairKey ─────────────────────────────────────────────────────────────────

    #[test]
    fn t701_pair_key_valid() {
        let key = PairKey::new(sym("BTCUSDT"), sym("ETHUSDT")).unwrap();
        assert_eq!(key.a, sym("BTCUSDT"));
        assert_eq!(key.b, sym("ETHUSDT"));
    }

    #[test]
    fn t701_pair_key_degenerate() {
        let err = PairKey::new(sym("BTCUSDT"), sym("BTCUSDT")).unwrap_err();
        assert_eq!(err, PairError::DegeneratePair);
    }

    #[test]
    fn t701_pair_key_ordered_distinct() {
        // (a, b) and (b, a) are distinct keys
        let k1 = PairKey::new(sym("BTCUSDT"), sym("ETHUSDT")).unwrap();
        let k2 = PairKey::new(sym("ETHUSDT"), sym("BTCUSDT")).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn t701_pair_key_btreemap_order() {
        // BTreeMap iteration follows lexicographic order: BNBUSDT < BTCUSDT < ETHUSDT
        let mut map = std::collections::BTreeMap::new();
        map.insert(PairKey::new(sym("ETHUSDT"), sym("SOLUSDT")).unwrap(), 3u32);
        map.insert(PairKey::new(sym("BTCUSDT"), sym("ETHUSDT")).unwrap(), 1u32);
        map.insert(PairKey::new(sym("BNBUSDT"), sym("BTCUSDT")).unwrap(), 2u32);
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(keys[0].a, sym("BNBUSDT"));
        assert_eq!(keys[1].a, sym("BTCUSDT"));
        assert_eq!(keys[2].a, sym("ETHUSDT"));
    }

    #[test]
    fn t701_pair_key_serde_roundtrip() {
        let key = PairKey::new(sym("BTCUSDT"), sym("ETHUSDT")).unwrap();
        let json = serde_json::to_string(&key).unwrap();
        let back: PairKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }

    // ── Pair ────────────────────────────────────────────────────────────────────

    #[test]
    fn t701_pair_valid_beta_one() {
        let p = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(1.0)).unwrap();
        assert_eq!(p.beta, dec!(1.0));
    }

    #[test]
    fn t701_pair_valid_beta_half() {
        let p = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(0.5)).unwrap();
        assert_eq!(p.beta, dec!(0.5));
    }

    #[test]
    fn t701_pair_valid_beta_two() {
        let p = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(2.0)).unwrap();
        assert_eq!(p.beta, dec!(2.0));
    }

    #[test]
    fn t701_pair_invalid_beta_zero() {
        let err = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(0)).unwrap_err();
        assert!(matches!(err, PairError::InvalidBeta { .. }));
    }

    #[test]
    fn t701_pair_invalid_beta_negative() {
        let err = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(-1)).unwrap_err();
        assert!(matches!(err, PairError::InvalidBeta { .. }));
    }

    #[test]
    fn t701_pair_invalid_beta_out_of_range_high() {
        let err = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(11)).unwrap_err();
        assert!(matches!(err, PairError::BetaOutOfRange { .. }));
    }

    #[test]
    fn t701_pair_invalid_beta_out_of_range_low() {
        let err = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(0.05)).unwrap_err();
        assert!(matches!(err, PairError::BetaOutOfRange { .. }));
    }

    #[test]
    fn t701_pair_serde_roundtrip() {
        let p = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(1.0)).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let back: Pair = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    // ── PairMembership ──────────────────────────────────────────────────────────

    #[test]
    fn t701_pair_membership_from_pair() {
        let p = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(1.0)).unwrap();
        let m = PairMembership::from_pair(&p);
        assert_eq!(m.key.a, sym("BTCUSDT"));
        assert_eq!(m.key.b, sym("ETHUSDT"));
        assert_eq!(m.traded_a_symbol, sym("BTCUSDT"));
    }

    #[test]
    fn t701_pair_membership_serde_roundtrip() {
        let p = Pair::new(sym("BTCUSDT"), sym("ETHUSDT"), dec!(1.0)).unwrap();
        let m = PairMembership::from_pair(&p);
        let json = serde_json::to_string(&m).unwrap();
        let back: PairMembership = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
