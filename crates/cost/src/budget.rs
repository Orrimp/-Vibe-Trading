//! `CostBudget` — monthly ceiling and auto-degrade logic.
//!
//! v2 (T1907) — atomic-cents refactor per Design § Q6:
//! `spent_usd: Decimal` is replaced with `spent_cents: AtomicU64` so the
//! pre-call budget gate ([`CostBudget::try_reserve`]) and the post-call
//! reconciliation ([`CostBudget::add_spend`]) are contention-free on the
//! LLM hot path. `mode_override()` stays a pure read.
//!
//! Concurrent-overshoot bound: with M concurrent in-flight calls each
//! reserving estimate `E`, the worst-case post-call settled cents may
//! exceed the ceiling by `M × max_per_call_usd`. The feature.md V12
//! verification documents this 0.2% bound on a $200 ceiling.

use std::sync::atomic::{AtomicU64, Ordering};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::event::LlmTier;

/// Conversion factor: USD to cents.
const CENTS_PER_USD: u64 = 100;

/// Budget-gate refusal reason — surfaced by [`CostBudget::try_reserve`].
///
/// The `llm` crate converts this into `LlmError::BudgetExceeded` via a
/// `From` impl; non-LLM callers can pattern-match directly. Carries the
/// already-spent and ceiling amounts (Decimal-typed) for forensic logging.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BudgetError {
    /// Pre-call estimate would push spent over ceiling — call rejected.
    #[error("budget exceeded: spent {spent_usd} of {ceiling_usd}")]
    BudgetExceeded {
        /// Already-spent amount when the reservation was attempted, in USD.
        spent_usd: Decimal,
        /// Monthly ceiling, in USD.
        ceiling_usd: Decimal,
    },
}

/// Monthly cost budget with auto-degrade rules.
///
/// v0: emitters are wired but no LLM calls happen; `spent` stays at zero.
/// v0.5: `mode_override()` is consulted before every LLM call.
/// v2 (T1907): `spent_cents` is an `AtomicU64`; `add_spend` and
/// `try_reserve` take `&self` so the budget can be wrapped in
/// `Arc<CostBudget>` and shared across concurrent calls without `Mutex`.
#[derive(Debug)]
pub struct CostBudget {
    /// Monthly ceiling in USD.
    pub ceiling_usd: Decimal,
    /// Accumulated spend this month, in cents (× 100 of USD).
    ///
    /// Stored as cents so the pre-call gate's `(spent + estimate) >
    /// ceiling` check is a single atomic load + integer compare, free of
    /// `Decimal` arithmetic on the hot path. Sub-cent rounding error per
    /// add is bounded at ≤ $0.01 — and the post-call reconcile is the
    /// source of truth anyway (R4.3).
    spent_cents: AtomicU64,
}

/// Convert a USD `Decimal` amount to cents, saturating-rounding-down.
///
/// Sub-cent fractions are truncated (not rounded) so a stream of small
/// adds never over-bills. Worst-case per-call error: ≤ $0.01.
fn usd_to_cents(usd: Decimal) -> u64 {
    if usd <= Decimal::ZERO {
        return 0;
    }
    let cents_dec = usd * Decimal::from(CENTS_PER_USD);
    // `floor` truncates toward zero for non-negative values, satisfying
    // the round-down contract.
    let floored = cents_dec.floor();
    // `to_u64` returns None on overflow / non-finite; clamp at u64::MAX.
    u64_from_decimal_saturating(floored)
}

/// Convert a cents `u64` back to USD `Decimal`.
fn cents_to_usd(cents: u64) -> Decimal {
    Decimal::from(cents) / Decimal::from(CENTS_PER_USD)
}

/// Saturating `Decimal → u64` conversion.
fn u64_from_decimal_saturating(d: Decimal) -> u64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_u64().unwrap_or(u64::MAX)
}

impl CostBudget {
    /// Construct a fresh budget with `ceiling_usd` and zero spent.
    #[must_use]
    pub fn new(ceiling_usd: Decimal) -> Self {
        Self {
            ceiling_usd,
            spent_cents: AtomicU64::new(0),
        }
    }

    /// Add spend (post-call reconcile per Design § Q6).
    ///
    /// Atomic — safe to call from concurrent tasks against an
    /// `Arc<CostBudget>`. Sub-cent fractions are truncated (round-down);
    /// see [`usd_to_cents`].
    pub fn add_spend(&self, usd: Decimal) {
        let cents = usd_to_cents(usd);
        if cents > 0 {
            self.spent_cents.fetch_add(cents, Ordering::SeqCst);
        }
    }

    /// Remaining budget (Decimal-typed for renderer use).
    #[must_use]
    pub fn remaining(&self) -> Decimal {
        let spent = cents_to_usd(self.spent_cents.load(Ordering::SeqCst));
        self.ceiling_usd - spent
    }

    /// Already-spent amount this month.
    #[must_use]
    pub fn spent(&self) -> Decimal {
        cents_to_usd(self.spent_cents.load(Ordering::SeqCst))
    }

    /// If spend ≥ 80% of ceiling, degrade to `QuickThink`.
    /// If spend ≥ 100%, returns `None` to block the call.
    #[must_use]
    pub fn mode_override(&self) -> Option<LlmTier> {
        if self.ceiling_usd == Decimal::ZERO {
            return None;
        }
        let spent = cents_to_usd(self.spent_cents.load(Ordering::SeqCst));
        let pct = spent / self.ceiling_usd;
        let threshold_80 = dec!(0.80);
        if pct >= Decimal::ONE {
            None // Block
        } else if pct >= threshold_80 {
            Some(LlmTier::QuickThink)
        } else {
            Some(LlmTier::DeepThink)
        }
    }

    /// Pre-call budget check (Design § Q6).
    ///
    /// Atomic load of `spent_cents`; if `(spent + estimate) > ceiling`,
    /// returns [`BudgetError::BudgetExceeded`]. **Check-only:** the
    /// reservation does NOT increment `spent_cents` — only
    /// [`Self::add_spend`] (post-call reconcile) does. The concurrent-
    /// overshoot bound is `M × max_per_call_usd`; see V12.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError::BudgetExceeded`] when the pre-call
    /// estimate would push the spent counter over the ceiling.
    pub fn try_reserve(&self, estimate_usd: Decimal) -> Result<(), BudgetError> {
        let estimate_cents = usd_to_cents(estimate_usd);
        let ceiling_cents = usd_to_cents(self.ceiling_usd);
        let spent_cents = self.spent_cents.load(Ordering::SeqCst);
        // saturating add so a pathologically large estimate doesn't wrap.
        let projected = spent_cents.saturating_add(estimate_cents);
        if projected > ceiling_cents {
            Err(BudgetError::BudgetExceeded {
                spent_usd: cents_to_usd(spent_cents),
                ceiling_usd: self.ceiling_usd,
            })
        } else {
            Ok(())
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// T1907 (a): seed budget at $179.99 / $200, `try_reserve(0.01)` Ok.
    #[test]
    fn t1907_try_reserve_within_ceiling_returns_ok() {
        let b = CostBudget::new(dec!(200.00));
        b.add_spend(dec!(179.99));
        assert!(b.try_reserve(dec!(0.01)).is_ok());
        // and the reservation did NOT mutate spent_cents.
        assert_eq!(b.spent(), dec!(179.99));
    }

    /// T1907 (b): seed at $200.01, `try_reserve(any)` → BudgetExceeded.
    #[test]
    fn t1907_try_reserve_over_ceiling_returns_budget_exceeded() {
        let b = CostBudget::new(dec!(200.00));
        b.add_spend(dec!(200.01));
        let err = b
            .try_reserve(dec!(0.00))
            .expect_err("over-ceiling reservation should fail");
        let BudgetError::BudgetExceeded {
            spent_usd,
            ceiling_usd,
        } = err;
        assert_eq!(spent_usd, dec!(200.01));
        assert_eq!(ceiling_usd, dec!(200.00));
    }

    /// T1907 (c): 100 parallel `add_spend(0.10)` calls → final cents == 1000.
    #[test]
    fn t1907_parallel_add_spend_no_torn_writes() {
        let b = Arc::new(CostBudget::new(dec!(1000.00)));
        let mut handles = Vec::new();
        for _ in 0..100 {
            let b2 = Arc::clone(&b);
            handles.push(std::thread::spawn(move || {
                b2.add_spend(dec!(0.10));
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // 100 × $0.10 = $10.00 = 1000 cents.
        assert_eq!(b.spent(), dec!(10.00));
    }

    /// T1907 (d): `remaining()` reads consistent — sequential add_spend
    /// + remaining returns ceiling - spent.
    #[test]
    fn t1907_remaining_reads_consistent() {
        let b = CostBudget::new(dec!(200.00));
        assert_eq!(b.remaining(), dec!(200.00));
        b.add_spend(dec!(50.00));
        assert_eq!(b.remaining(), dec!(150.00));
        b.add_spend(dec!(25.00));
        assert_eq!(b.remaining(), dec!(125.00));
    }

    /// `mode_override()` semantics preserved across the refactor.
    #[test]
    fn t1907_mode_override_thresholds_preserved() {
        let b = CostBudget::new(dec!(100.00));
        assert_eq!(b.mode_override(), Some(LlmTier::DeepThink));
        b.add_spend(dec!(50.00));
        assert_eq!(b.mode_override(), Some(LlmTier::DeepThink));
        b.add_spend(dec!(30.00)); // 80 / 100 = 0.80 → QuickThink
        assert_eq!(b.mode_override(), Some(LlmTier::QuickThink));
        b.add_spend(dec!(20.00)); // 100 / 100 = 1.00 → Block
        assert_eq!(b.mode_override(), None);
    }

    /// Zero-ceiling budget blocks every call (preserves v0.5 behavior).
    #[test]
    fn t1907_zero_ceiling_blocks() {
        let b = CostBudget::new(Decimal::ZERO);
        assert_eq!(b.mode_override(), None);
    }

    /// Sub-cent fractions are truncated (round-down). Adding $0.009 ten
    /// times keeps the spent counter at zero cents — bounded under-bill.
    #[test]
    fn t1907_sub_cent_truncation_bounded_underbill() {
        let b = CostBudget::new(dec!(1.00));
        for _ in 0..10 {
            b.add_spend(dec!(0.009));
        }
        // Each call rounds 0.9 cents → 0 cents; 10 adds → 0 cents.
        // Post-call reconcile in BudgetedProvider sums the *true* USD
        // value in the cost event; this is the budget gate's
        // approximation only.
        assert_eq!(b.spent(), dec!(0.00));
    }
}
