//! Outcome classifier (Q3c — analyst strawman pinned).
//!
//! Pure function over signed P&L + opening capital.  Threshold pinned
//! at ±0.5% of opening capital (`OUTCOME_THRESHOLD_PCT = dec!(0.005)`).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use trading_core::{Money, Usdt};

/// Outcome class — Win / Loss / Scratch.
///
/// `Display` emits `Win|Loss|Scratch` (PascalCase) — matches the body
/// line shape in R4.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OutcomeClass {
    Win,
    Loss,
    Scratch,
}

impl std::fmt::Display for OutcomeClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Win => "Win",
            Self::Loss => "Loss",
            Self::Scratch => "Scratch",
        };
        f.write_str(s)
    }
}

/// ±0.5% of opening capital — analyst strawman pinned (R1.4 / Q3c).
///
/// Grep-changeable in one place; any change re-anchors the two
/// `report-sample-*` SHAs.
pub const OUTCOME_THRESHOLD_PCT: Decimal = dec!(0.005);

/// Classify a closed trade by signed P&L over opening capital.
///
/// - `Win`     iff `signed_pnl / opening_capital > +0.005`,
/// - `Loss`    iff `signed_pnl / opening_capital < -0.005`,
/// - `Scratch` otherwise (or when `opening_capital == 0` — defensive).
///
/// Pure: no I/O, no clock.
#[must_use]
pub fn classify_outcome(signed_pnl: Money<Usdt>, opening_capital: Money<Usdt>) -> OutcomeClass {
    let cap = opening_capital.amount();
    if cap == Decimal::ZERO {
        return OutcomeClass::Scratch;
    }
    let ratio = signed_pnl.amount() / cap;
    if ratio > OUTCOME_THRESHOLD_PCT {
        OutcomeClass::Win
    } else if ratio < -OUTCOME_THRESHOLD_PCT {
        OutcomeClass::Loss
    } else {
        OutcomeClass::Scratch
    }
}
