//! `CostBudget` — monthly ceiling and auto-degrade logic.
use rust_decimal::Decimal;

use crate::event::LlmTier;

/// Monthly cost budget with auto-degrade rules.
///
/// v0: emitters are wired but no LLM calls happen; `spent` stays at zero.
/// v0.5: `mode_override()` is consulted before every LLM call.
pub struct CostBudget {
    /// Monthly ceiling in USD.
    pub ceiling_usd: Decimal,
    /// Accumulated spend this month in USD.
    spent_usd: Decimal,
}

impl CostBudget {
    #[must_use]
    pub fn new(ceiling_usd: Decimal) -> Self {
        Self {
            ceiling_usd,
            spent_usd: Decimal::ZERO,
        }
    }

    /// Add spend.
    pub fn add_spend(&mut self, usd: Decimal) {
        self.spent_usd += usd;
    }

    /// Remaining budget.
    #[must_use]
    pub fn remaining(&self) -> Decimal {
        self.ceiling_usd - self.spent_usd
    }

    /// If spend >= 80% of ceiling, degrade to `QuickThink`.
    /// If spend >= 100%, returns `None` to block the call.
    #[must_use]
    pub fn mode_override(&self) -> Option<LlmTier> {
        if self.ceiling_usd == Decimal::ZERO {
            return None;
        }
        let pct = self.spent_usd / self.ceiling_usd;
        let threshold_80 = Decimal::new(80, 2);
        if pct >= Decimal::ONE {
            None // Block
        } else if pct >= threshold_80 {
            Some(LlmTier::QuickThink)
        } else {
            Some(LlmTier::DeepThink)
        }
    }
}
