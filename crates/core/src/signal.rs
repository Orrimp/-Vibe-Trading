//! Signal, Decision, and supporting types.
//!
//! ## v1.5a additions (T701)
//!
//! Three new `SignalKind` variants cover mean-reversion pair signals:
//! - `OpenPairLong` — open a long position on the `a` leg of a pair.
//! - `ClosePair` — close the `a`-leg position (reversion exit or hard-stop).
//! - `PairShortObservation` — observation-only; the would-have-shorted `b` leg
//!   (formulation C residual — no `Order` constructed, no money moves).
//!
//! `Signal` is extended with an optional `pair_data` field that carries
//! pair-specific context when `kind ∈ {OpenPairLong, ClosePair,
//! PairShortObservation}`. For all other `kind` values `pair_data` is `None`.
//!
//! Existing consumers that match on `sig.kind` must use a `_ => {}` default
//! arm or explicitly handle the three new variants — the compiler enforces
//! exhaustiveness as always.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::money::Price;
use crate::order::ProposedOrder;
use crate::pair::PairKey;
use crate::symbol::{StrategyId, Symbol};
use crate::time::Timestamp;

/// Direction emitted by a strategy.
///
/// ## v1.5a additions
///
/// Three new variants carry pair mean-reversion intent. Existing handlers
/// that exhaustively match `Buy | Sell | Hold` must add `_ => {}` (or handle
/// the new variants explicitly) after v1.5a lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    Buy,
    Sell,
    Hold,
    /// v1.5a — open a long position on the `a` leg of a pair.
    OpenPairLong,
    /// v1.5a — close the `a`-leg position (reversion exit or hard-stop).
    ClosePair,
    /// v1.5a — observation-only; no `Order` is constructed. The would-have-
    /// shorted `b` leg is recorded in the audit ledger for future v2 use.
    PairShortObservation,
}

/// Why a pair position was closed (v1.5a, R4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason_kind", rename_all = "snake_case")]
pub enum StopReason {
    /// Normal exit: `|z| <= z_exit` — spread reverted to mean.
    Reversion {
        /// The z-score value at the exit bar.
        z_at_exit: Decimal,
    },
    /// Hard stop: `z >= z_stop` while long — spread blew through.
    HardStop {
        /// The z-score value at the stop bar.
        z_at_stop: Decimal,
    },
}

/// Pair-specific signal context (v1.5a, T701).
///
/// Carried by `Signal.pair_data` when `kind ∈ {OpenPairLong, ClosePair,
/// PairShortObservation}`.  `None` for all v0/v0.5/v1 signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairSignalData {
    /// The `(a, b)` pair this signal refers to.
    pub pair_key: PairKey,
    /// The z-score that triggered this signal.
    pub z_at_signal: Decimal,
    /// For `OpenPairLong`: fraction of equity to allocate (the strategy's
    /// `exposure_cap_per_pair`).  `None` for other kinds.
    pub weight: Option<Decimal>,
    /// For `ClosePair`: reason for the close.  `None` for other kinds.
    pub stop_reason: Option<StopReason>,
}

/// Evidence that produced a signal (for audit trail).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalEvidence {
    /// SMA fast value at signal time.
    pub fast_ma: Option<Decimal>,
    /// SMA slow value at signal time.
    pub slow_ma: Option<Decimal>,
    /// Arbitrary extra key-value pairs for future indicators.
    pub extra: Vec<(SmolStr, Decimal)>,
}

impl SignalEvidence {
    /// Create a minimal evidence record from fast/slow MA values.
    #[must_use]
    pub fn sma(fast_ma: Decimal, slow_ma: Decimal) -> Self {
        Self {
            fast_ma: Some(fast_ma),
            slow_ma: Some(slow_ma),
            extra: vec![],
        }
    }

    /// Create evidence for a momentum signal (v1).
    ///
    /// `action` is a human-readable tag: `"open"`, `"close"`, `"resize"`.
    /// `score` is the strategy's cached selection score — for the
    /// cross-sectional family this is the vol-adjusted momentum score AS
    /// CACHED, which under `Direction::Reversion` (D-MR.1, review 1-16) is the
    /// SIGN-FLIPPED (negated) momentum score. Neither this evidence nor the
    /// enclosing [`Signal`] carries a direction marker, so the stored value is
    /// not interpretable as "the momentum score" on its own: consumers must
    /// consult the emitting strategy's config (or its config hash) to know
    /// whether the sign was flipped.
    #[must_use]
    pub fn momentum(action: &str, score: Decimal) -> Self {
        Self {
            fast_ma: None,
            slow_ma: None,
            extra: vec![
                (SmolStr::new("action"), Decimal::ZERO), // tag — score not Decimal but works
                (SmolStr::new(action), score),
            ],
        }
    }

    /// Empty evidence (e.g. for Hold signals or pair signals).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            fast_ma: None,
            slow_ma: None,
            extra: vec![],
        }
    }

    /// Create evidence for a pair mean-reversion signal (v1.5a).
    ///
    /// `action` is a human-readable tag: `"open_pair_long"`, `"close_pair"`,
    /// `"pair_short_observation"`.
    /// `z` is the z-score that triggered the signal.
    #[must_use]
    pub fn pair_mr(action: &str, z: Decimal) -> Self {
        Self {
            fast_ma: None,
            slow_ma: None,
            extra: vec![
                (SmolStr::new("action"), Decimal::ZERO),
                (SmolStr::new(action), z),
            ],
        }
    }
}

/// A trading signal emitted by a strategy.
///
/// ## v1.5a pair fields
///
/// For `kind ∈ {OpenPairLong, ClosePair, PairShortObservation}` the
/// `pair_data` field carries pair-specific context. For all other kinds
/// `pair_data` is `None`.
///
/// The `symbol` field carries the **traded leg** (`a`) for `OpenPairLong`
/// and `ClosePair`. For `PairShortObservation` it carries the reference leg
/// (`b`) — purely informational; no position is opened on `b` in v1.5a.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub strategy_id: StrategyId,
    /// The primary symbol: traded `a` leg for pair signals; the signal
    /// symbol for v0/v0.5/v1 signals.
    pub symbol: Symbol,
    pub ts: Timestamp,
    pub kind: SignalKind,
    /// The indicator values that produced this signal.
    pub evidence: SignalEvidence,
    /// Pair-specific context; `None` for v0/v0.5/v1 signals.
    pub pair_data: Option<PairSignalData>,
}

impl Signal {
    /// Construct an `OpenPairLong` signal (v1.5a helper).
    #[must_use]
    pub fn open_pair_long(
        strategy_id: StrategyId,
        pair_key: PairKey,
        entry_z: Decimal,
        weight: Decimal,
        ts: Timestamp,
    ) -> Self {
        let symbol = pair_key.a.clone();
        Self {
            strategy_id,
            symbol,
            ts,
            kind: SignalKind::OpenPairLong,
            evidence: SignalEvidence::pair_mr("open_pair_long", entry_z),
            pair_data: Some(PairSignalData {
                pair_key,
                z_at_signal: entry_z,
                weight: Some(weight),
                stop_reason: None,
            }),
        }
    }

    /// Construct a `ClosePair` signal (v1.5a helper).
    #[must_use]
    pub fn close_pair(
        strategy_id: StrategyId,
        pair_key: PairKey,
        reason: StopReason,
        ts: Timestamp,
    ) -> Self {
        let z = match &reason {
            StopReason::Reversion { z_at_exit } => *z_at_exit,
            StopReason::HardStop { z_at_stop } => *z_at_stop,
        };
        let symbol = pair_key.a.clone();
        Self {
            strategy_id,
            symbol,
            ts,
            kind: SignalKind::ClosePair,
            evidence: SignalEvidence::pair_mr("close_pair", z),
            pair_data: Some(PairSignalData {
                pair_key,
                z_at_signal: z,
                weight: None,
                stop_reason: Some(reason),
            }),
        }
    }

    /// Construct a `PairShortObservation` signal (v1.5a helper).
    ///
    /// Observation-only — no `Order` constructed, no money moves.
    #[must_use]
    pub fn pair_short_observation(
        strategy_id: StrategyId,
        pair_key: PairKey,
        z_at_signal: Decimal,
        ts: Timestamp,
    ) -> Self {
        let symbol = pair_key.b.clone(); // reference leg — informational only
        Self {
            strategy_id,
            symbol,
            ts,
            kind: SignalKind::PairShortObservation,
            evidence: SignalEvidence::pair_mr("pair_short_observation", z_at_signal),
            pair_data: Some(PairSignalData {
                pair_key,
                z_at_signal,
                weight: None,
                stop_reason: None,
            }),
        }
    }
}

/// A decision to act on a signal (v0: passes through directly; v0.5 adds LLM debate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub signal: Signal,
    pub proposed: ProposedOrder,
    /// Human-readable rationale, e.g. `"sma_crossover sizing=fixed_fraction"`.
    pub rationale: SmolStr,
    /// Last known mark price at decision time.
    pub last_mark: Price,
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;

    use crate::pair::PairKey;
    use crate::symbol::Symbol;

    fn ts() -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH)
    }
    fn sym(s: &str) -> Symbol {
        Symbol::new(s)
    }
    fn strat() -> StrategyId {
        StrategyId::new("test")
    }
    fn pair_key() -> PairKey {
        PairKey::new(sym("BTCUSDT"), sym("ETHUSDT")).unwrap()
    }

    #[test]
    fn t701_open_pair_long_serde_roundtrip() {
        let sig = Signal::open_pair_long(strat(), pair_key(), dec!(-2.1), dec!(0.25), ts());
        assert_eq!(sig.kind, SignalKind::OpenPairLong);
        assert_eq!(sig.symbol, sym("BTCUSDT")); // traded a leg
        let pd = sig.pair_data.as_ref().unwrap();
        assert_eq!(pd.pair_key.a, sym("BTCUSDT"));
        assert_eq!(pd.pair_key.b, sym("ETHUSDT"));
        assert_eq!(pd.weight, Some(dec!(0.25)));
        assert!(pd.stop_reason.is_none());

        let json = serde_json::to_string(&sig).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, SignalKind::OpenPairLong);
        assert_eq!(back.pair_data.unwrap().pair_key.a, sym("BTCUSDT"));
    }

    #[test]
    fn t701_close_pair_reversion_serde_roundtrip() {
        let sig = Signal::close_pair(
            strat(),
            pair_key(),
            StopReason::Reversion {
                z_at_exit: dec!(0.3),
            },
            ts(),
        );
        assert_eq!(sig.kind, SignalKind::ClosePair);
        let pd = sig.pair_data.as_ref().unwrap();
        assert!(matches!(pd.stop_reason, Some(StopReason::Reversion { .. })));

        let json = serde_json::to_string(&sig).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back.pair_data.unwrap().stop_reason,
            Some(StopReason::Reversion { .. })
        ));
    }

    #[test]
    fn t701_close_pair_hard_stop_serde_roundtrip() {
        let sig = Signal::close_pair(
            strat(),
            pair_key(),
            StopReason::HardStop {
                z_at_stop: dec!(4.2),
            },
            ts(),
        );
        assert_eq!(sig.kind, SignalKind::ClosePair);
        let pd = sig.pair_data.as_ref().unwrap();
        assert!(matches!(pd.stop_reason, Some(StopReason::HardStop { .. })));
        let json = serde_json::to_string(&sig).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back.pair_data.unwrap().stop_reason,
            Some(StopReason::HardStop { .. })
        ));
    }

    #[test]
    fn t701_pair_short_observation_serde_roundtrip() {
        let sig = Signal::pair_short_observation(strat(), pair_key(), dec!(-2.1), ts());
        assert_eq!(sig.kind, SignalKind::PairShortObservation);
        assert_eq!(sig.symbol, sym("ETHUSDT")); // reference b leg

        let json = serde_json::to_string(&sig).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, SignalKind::PairShortObservation);
    }

    #[test]
    fn t701_signal_kind_no_pair_data_for_legacy() {
        // v0/v0.5/v1 signals have no pair_data
        let sig = Signal {
            strategy_id: strat(),
            symbol: sym("BTCUSDT"),
            ts: ts(),
            kind: SignalKind::Buy,
            evidence: SignalEvidence::empty(),
            pair_data: None,
        };
        assert!(sig.pair_data.is_none());
    }

    #[test]
    fn t701_stop_reason_serde_roundtrip() {
        let r = StopReason::HardStop {
            z_at_stop: dec!(4.5),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: StopReason = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, StopReason::HardStop { .. }));

        let r2 = StopReason::Reversion {
            z_at_exit: dec!(0.4),
        };
        let json2 = serde_json::to_string(&r2).unwrap();
        let back2: StopReason = serde_json::from_str(&json2).unwrap();
        assert!(matches!(back2, StopReason::Reversion { .. }));
    }
}
