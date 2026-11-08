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
            // P1-2 fields: zero for test helper (gate tests don't read these).
            cvar_95: 0.0,
            cvar_99: 0.0,
            median_terminal_wealth: 0.0,
            skew: 0.0,
        }
    }

    // ── Every ROBUST edge pinned from BOTH sides (bug-log #161, sweep F2) ─────
    //
    // `classify_verdict` and `verdict_bands` are BYTE-FROZEN by AD-1 and are not
    // touched here. The gap was entirely in the fixtures.
    //
    // MEASURED 2026-11-07: `P50_SHARPE_ROBUST` 1.0 -> 0.0, `PROB_LOSS_ROBUST`
    // 0.15 -> 0.99, `PROB_SHARPE_GT1_ROBUST` 0.60 -> 0.0 and `P95_MAXDD_ROBUST`
    // 0.50 -> 0.99 — ALL FOUR AT ONCE, which reduces ROBUST to "not FRAGILE and
    // p5 >= 0.5" — left 241 lib + 43 bin + 84 integration = 368 shipped tests
    // GREEN. Only the FRAGILE five and `P5_SHARPE_ROBUST` were two-sided.
    //
    // The reason is specific and worth keeping: the one existing ROBUST-boundary
    // fixture, `param_robustness_sweep.rs::classifier_robust_boundary_all_just_
    // above`, sets all five signals EXACTLY at their band. That pins the
    // inclusive side, so it catches NARROWING a band. And all four MARGINAL
    // fixtures in the tree (p5 = 0.0, 0.3, 0.3, 0.3) are Marginal via
    // `p5 < P5_SHARPE_ROBUST` ALONE — so once p5 fails, the other four
    // criteria are never the reason for any expected outcome, and widening them
    // changes no test's result. Every unpinned direction labels MORE candidates
    // ROBUST, which is the direction that matters for a credibility gate.
    //
    // Latent, not live: the anchored per-cell verdicts are 784 FRAGILE / 80
    // MARGINAL / 0 ROBUST, and max p5 on any MARGINAL cell is 0.182983 — so the
    // gap relabels nothing today. That is exactly why no anchor could catch it.

    /// Every signal comfortably inside its ROBUST band AND comfortably clear of
    /// every FRAGILE band. Each boundary case below moves exactly ONE signal and
    /// leaves the other four here, so a flip is attributable to that one edge.
    ///
    /// Order matches `make_summary`: p5 Sharpe, p50 Sharpe, `prob_loss`,
    /// P(Sharpe>1), p95 `MaxDD`.
    const ROBUST_BASELINE: [f64; 5] = [0.8, 1.5, 0.05, 0.80, 0.30];

    fn summary_from(v: [f64; 5]) -> DistributionSummary {
        make_summary(v[0], v[1], v[2], v[3], v[4])
    }

    /// Two-sided pin per ROBUST edge: exactly AT the band must be ROBUST (the
    /// comparisons are inclusive), and just OUTSIDE it must be MARGINAL with the
    /// other four held inside. Narrowing an edge breaks the first assertion;
    /// widening it breaks the second, and before 2026-11-08 nothing did.
    #[test]
    fn classifier_every_robust_edge_is_pinned_from_both_sides() {
        // Non-vacuity: the baseline must itself be ROBUST, or every case below
        // would be comparing against a verdict reached for another reason.
        assert_eq!(
            classify_verdict(&summary_from(ROBUST_BASELINE)),
            ParamRobustnessVerdict::Robust,
            "the baseline must be ROBUST for the per-edge cases to attribute \
             anything; if this fails the baseline drifted into a band"
        );

        for (label, idx, at_band, just_outside) in [
            ("P5_SHARPE_ROBUST (>= 0.5)", 0usize, 0.5, 0.499),
            ("P50_SHARPE_ROBUST (>= 1.0)", 1, 1.0, 0.999),
            ("PROB_LOSS_ROBUST (<= 0.15)", 2, 0.15, 0.151),
            ("PROB_SHARPE_GT1_ROBUST (>= 0.60)", 3, 0.60, 0.599),
            ("P95_MAXDD_ROBUST (<= 0.50)", 4, 0.50, 0.501),
        ] {
            let mut at = ROBUST_BASELINE;
            at[idx] = at_band;
            assert_eq!(
                classify_verdict(&summary_from(at)),
                ParamRobustnessVerdict::Robust,
                "{label}: a signal EXACTLY at the band must still be ROBUST — the \
                 comparison is inclusive. A failure here means the band was \
                 NARROWED (or the operator flipped to a strict comparison)."
            );

            let mut outside = ROBUST_BASELINE;
            outside[idx] = just_outside;
            assert_eq!(
                classify_verdict(&summary_from(outside)),
                ParamRobustnessVerdict::Marginal,
                "{label}: just outside the band, with the other four held \
                 comfortably inside and nothing in a FRAGILE band, must be \
                 MARGINAL. A failure here means the band was WIDENED and this \
                 candidate is now crowned ROBUST on {label} alone."
            );
        }
    }

    /// The measured collapse, made visible. Widening P50 / `PROB_LOSS` /
    /// P(Sharpe>1) / P95 `MaxDD` together reduces ROBUST to "not FRAGILE and
    /// p5 >= `P5_SHARPE_ROBUST`". This fixture is a candidate that would be
    /// crowned under that collapsed rule and must NOT be: p5 clears ROBUST, the
    /// other four each sit just outside, and no signal is in a FRAGILE band.
    #[test]
    fn classifier_robust_does_not_collapse_to_p5_alone() {
        let summary = make_summary(0.8, 0.999, 0.151, 0.599, 0.501);
        // Stated explicitly so the fixture's premise is checked, not assumed:
        // nothing here is FRAGILE, so the verdict turns only on the ROBUST set.
        assert_ne!(
            classify_verdict(&summary),
            ParamRobustnessVerdict::Fragile,
            "fixture premise broken: no signal should be in a FRAGILE band \
             (p5 0.8 >= 0.0, p50 0.999 >= 0.5, loss 0.151 <= 0.35, \
             P(S>1) 0.599 >= 0.35, dd 0.501 <= 0.70)"
        );
        assert_eq!(
            classify_verdict(&summary),
            ParamRobustnessVerdict::Marginal,
            "ROBUST must require ALL FIVE signals, not p5 alone. If this reads \
             ROBUST, four of the five bands have been widened into no-ops and \
             the gate now crowns on one signal."
        );
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
