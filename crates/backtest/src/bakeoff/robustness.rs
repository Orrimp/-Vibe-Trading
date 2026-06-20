//! Robustness verdict classifier (extracted from `bin/param_robustness_sweep.rs`).
//!
//! This is a **behaviour-preserving relocation** of `classify_verdict`,
//! `ParamRobustnessVerdict`, and the band constants from the sweep bin into the
//! `backtest` library so the bake-off orchestrator and any future caller can use
//! them without depending on the bin.
//!
//! The sweep bin is updated to re-import from here.
//!
//! # Contract
//!
//! The FRAGILE band (p5 Sharpe < 0) and the full 5-signal weakest-link composite
//! are **unchanged** — this is a pure structural move, not a semantic change.
//! The frozen decision-rule bands live in `verdict_bands` below and are identical
//! to the originals in the sweep bin.

#![allow(clippy::float_arithmetic)] // statistical threshold comparisons

use crate::stats::DistributionSummary;

// ── Public verdict type ────────────────────────────────────────────────────────

/// Robustness flag for one bake-off candidate.
///
/// Emitted by `classify` (or `RobustnessMode::Skip` → always `Skipped`).
/// The ranking comparator treats `Fragile` as the **only** ineligible-to-crown
/// flag; all other variants are crown-eligible (F2 contract § Eligibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobustnessFlag {
    /// Every primary signal in the ROBUST band.
    Robust,
    /// No primary signal in the FRAGILE band, but not all in ROBUST.
    Marginal,
    /// At least one primary signal in the FRAGILE band — ineligible to be
    /// crowned unless ALL candidates are Fragile.
    Fragile,
    /// Gate intentionally not run (e.g. robustness disabled for a fast bake-off).
    /// Crown-eligible (treated same as `Robust`/`Marginal` by the comparator).
    Skipped,
}

// ── Per-θ verdict (kept from sweep bin for compatibility) ────────────────────

/// Per-θ verdict from the 5-signal weakest-link composite.
///
/// Relocated from `bin/param_robustness_sweep.rs` into the library so both the
/// bake-off and the sweep bin share a single implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamRobustnessVerdict {
    /// Every primary signal in ROBUST band.
    Robust,
    /// No primary signal in FRAGILE band, but not all in ROBUST.
    Marginal,
    /// At least one primary signal in FRAGILE band.
    Fragile,
}

impl ParamRobustnessVerdict {
    /// The verdict as a static string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Robust => "ROBUST",
            Self::Marginal => "MARGINAL",
            Self::Fragile => "FRAGILE",
        }
    }
}

impl From<ParamRobustnessVerdict> for RobustnessFlag {
    fn from(v: ParamRobustnessVerdict) -> Self {
        match v {
            ParamRobustnessVerdict::Robust => Self::Robust,
            ParamRobustnessVerdict::Marginal => Self::Marginal,
            ParamRobustnessVerdict::Fragile => Self::Fragile,
        }
    }
}

// ── Frozen decision-rule bands (robustness-decision-rule-2026-05-30 § 0) ──────

/// Frozen decision-rule bands — identical to those in `bin/param_robustness_sweep.rs`.
/// FRAGILE if ANY primary signal breaches its threshold.
/// ROBUST only if ALL primary signals clear their threshold.
pub mod verdict_bands {
    // FRAGILE thresholds
    /// `p5` Sharpe < 0 → FRAGILE (tail loses money).
    pub const P5_SHARPE_FRAGILE: f64 = 0.0;
    /// `p50` Sharpe < 0.5 → FRAGILE (central tendency weak).
    pub const P50_SHARPE_FRAGILE: f64 = 0.5;
    /// `prob_loss` > 0.35 → FRAGILE (coin-flip-ish loss rate).
    pub const PROB_LOSS_FRAGILE: f64 = 0.35;
    /// P(Sharpe > 1.0) < 0.35 → FRAGILE (minority clears gate).
    pub const PROB_SHARPE_GT1_FRAGILE: f64 = 0.35;
    /// `p95` `MaxDD` > 0.70 → FRAGILE (tail drawdown worse than ~73%).
    pub const P95_MAXDD_FRAGILE: f64 = 0.70;

    // ROBUST thresholds
    /// `p5` Sharpe ≥ 0.5 → ROBUST band.
    pub const P5_SHARPE_ROBUST: f64 = 0.5;
    /// `p50` Sharpe ≥ 1.0 → ROBUST band.
    pub const P50_SHARPE_ROBUST: f64 = 1.0;
    /// `prob_loss` ≤ 0.15 → ROBUST band.
    pub const PROB_LOSS_ROBUST: f64 = 0.15;
    /// P(Sharpe > 1.0) ≥ 0.60 → ROBUST band.
    pub const PROB_SHARPE_GT1_ROBUST: f64 = 0.60;
    /// `p95` `MaxDD` ≤ 0.50 → ROBUST band.
    pub const P95_MAXDD_ROBUST: f64 = 0.50;
}

// ── Classifier (behaviour-preserving copy of the sweep-bin fn) ─────────────────

/// Compute the composite per-θ verdict (5-signal weakest-link).
///
/// Relocated verbatim from `bin/param_robustness_sweep.rs::classify_verdict`.
/// This is a pure function — unit-testable at band boundaries.
///
/// The sweep bin now delegates to this function (behaviour-preserving).
#[must_use]
pub fn classify_verdict(summary: &DistributionSummary) -> ParamRobustnessVerdict {
    use verdict_bands::{
        P5_SHARPE_FRAGILE, P5_SHARPE_ROBUST, P50_SHARPE_FRAGILE, P50_SHARPE_ROBUST,
        P95_MAXDD_FRAGILE, P95_MAXDD_ROBUST, PROB_LOSS_FRAGILE, PROB_LOSS_ROBUST,
        PROB_SHARPE_GT1_FRAGILE, PROB_SHARPE_GT1_ROBUST,
    };

    let sharpe_p5 = summary.sharpe.p5;
    let sharpe_p50 = summary.sharpe.p50;
    let p_loss = summary.prob_loss;
    let p_sharpe_gt1 = summary.prob_sharpe_gt_1;
    let maxdd_p95 = summary.max_dd_tail_p95;

    // FRAGILE: any single primary signal in FRAGILE band → composite FRAGILE.
    let is_fragile = sharpe_p5 < P5_SHARPE_FRAGILE
        || sharpe_p50 < P50_SHARPE_FRAGILE
        || p_loss > PROB_LOSS_FRAGILE
        || p_sharpe_gt1 < PROB_SHARPE_GT1_FRAGILE
        || maxdd_p95 > P95_MAXDD_FRAGILE;

    if is_fragile {
        return ParamRobustnessVerdict::Fragile;
    }

    // ROBUST: ALL primary signals in ROBUST band.
    let is_robust = sharpe_p5 >= P5_SHARPE_ROBUST
        && sharpe_p50 >= P50_SHARPE_ROBUST
        && p_loss <= PROB_LOSS_ROBUST
        && p_sharpe_gt1 >= PROB_SHARPE_GT1_ROBUST
        && maxdd_p95 <= P95_MAXDD_ROBUST;

    if is_robust {
        ParamRobustnessVerdict::Robust
    } else {
        ParamRobustnessVerdict::Marginal
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names)]
    use super::*;
    use crate::stats::{DistributionSummary, MetricDistribution};

    fn zero_dist() -> MetricDistribution {
        MetricDistribution {
            mean: 0.0,
            std: 0.0,
            p5: 0.0,
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
            p95: 0.0,
            min: 0.0,
            max: 0.0,
        }
    }

    /// Build a `DistributionSummary` fixture from individual values.
    fn make_summary(
        p5_sharpe: f64,
        p50_sharpe: f64,
        prob_loss: f64,
        prob_sharpe_gt1: f64,
        p95_maxdd: f64,
    ) -> DistributionSummary {
        DistributionSummary {
            sharpe: MetricDistribution {
                mean: p50_sharpe,
                std: 0.0,
                p5: p5_sharpe,
                p25: p5_sharpe,
                p50: p50_sharpe,
                p75: p50_sharpe,
                p95: p50_sharpe + 0.5,
                min: p5_sharpe,
                max: p50_sharpe + 0.5,
            },
            sortino: zero_dist(),
            calmar: zero_dist(),
            max_drawdown: zero_dist(),
            total_return: zero_dist(),
            prob_loss,
            prob_sharpe_gt_0: 0.0,
            prob_sharpe_gt_1: prob_sharpe_gt1,
            max_dd_tail_p50: 0.0,
            max_dd_tail_p95: p95_maxdd,
        }
    }

    /// T1.3a — FRAGILE: p5 Sharpe < 0 triggers FRAGILE.
    #[test]
    fn fragile_p5_sharpe_below_zero() {
        let summary = make_summary(-0.1, 1.0, 0.1, 0.7, 0.3);
        assert_eq!(classify_verdict(&summary), ParamRobustnessVerdict::Fragile);
    }

    /// T1.3b — MARGINAL: p5 Sharpe ≥ 0 but below ROBUST threshold.
    #[test]
    fn marginal_p5_in_between() {
        // p5=0.3 ≥ P5_FRAGILE(0.0) but < P5_ROBUST(0.5) → Marginal
        let summary = make_summary(0.3, 1.0, 0.1, 0.7, 0.3);
        assert_eq!(classify_verdict(&summary), ParamRobustnessVerdict::Marginal);
    }

    /// T1.3c — ROBUST: all signals in ROBUST band.
    #[test]
    fn robust_all_signals_clear() {
        // p5=0.6 ≥ 0.5, p50=1.2 ≥ 1.0, prob_loss=0.1 ≤ 0.15,
        // prob_sharpe_gt1=0.65 ≥ 0.60, p95_maxdd=0.4 ≤ 0.50
        let summary = make_summary(0.6, 1.2, 0.1, 0.65, 0.4);
        assert_eq!(classify_verdict(&summary), ParamRobustnessVerdict::Robust);
    }

    /// Boundary check — p5 Sharpe exactly 0.0 is FRAGILE.
    #[test]
    fn boundary_p5_zero_is_fragile() {
        let summary = make_summary(0.0, 1.0, 0.1, 0.7, 0.3);
        // 0.0 < 0.0 is false → not fragile on p5 alone;
        // but prob_loss=0.1 ≤ 0.35 fine, prob_sharpe_gt1=0.7 fine, p95_maxdd=0.3 fine.
        // p5_sharpe=0.0 ≥ P5_FRAGILE(0.0): NOT fragile on that signal.
        // p50=1.0 ≥ 0.5 fine; all ROBUST signals: p5=0.0 < 0.5 → Marginal.
        assert_eq!(classify_verdict(&summary), ParamRobustnessVerdict::Marginal);
    }

    /// Verify `RobustnessFlag::from(ParamRobustnessVerdict)` maps correctly.
    #[test]
    fn from_verdict_to_flag() {
        assert_eq!(
            RobustnessFlag::from(ParamRobustnessVerdict::Robust),
            RobustnessFlag::Robust
        );
        assert_eq!(
            RobustnessFlag::from(ParamRobustnessVerdict::Marginal),
            RobustnessFlag::Marginal
        );
        assert_eq!(
            RobustnessFlag::from(ParamRobustnessVerdict::Fragile),
            RobustnessFlag::Fragile
        );
    }
}
