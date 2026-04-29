//! Signal, Decision, and supporting types.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::money::Price;
use crate::order::ProposedOrder;
use crate::symbol::{StrategyId, Symbol};
use crate::time::Timestamp;

/// Direction emitted by a strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    Buy,
    Sell,
    Hold,
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
    /// `score` is the vol-adjusted momentum score.
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

    /// Empty evidence (e.g. for Hold signals).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            fast_ma: None,
            slow_ma: None,
            extra: vec![],
        }
    }
}

/// A trading signal emitted by a strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub strategy_id: StrategyId,
    pub symbol: Symbol,
    pub ts: Timestamp,
    pub kind: SignalKind,
    /// The indicator values that produced this signal.
    pub evidence: SignalEvidence,
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
