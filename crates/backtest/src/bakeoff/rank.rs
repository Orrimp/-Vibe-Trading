//! F2 ranking comparator — pure, total, deterministic.
//!
//! Implements the normative F2 ranking contract from `feature.md § F2`:
//!
//! **Eligibility.** A candidate is *ineligible to be crowned* iff
//! `robustness == Some(Fragile)`. All other flags are eligible.
//!
//! **Order (best-first), strict total order:**
//! 1. eligible before ineligible;
//! 2. then Sharpe descending (`f64::total_cmp`);
//! 3. then `total_return_pct` descending (`Decimal::cmp`, exact);
//! 4. then `max_drawdown` ascending (`Decimal::cmp`, smaller is better);
//! 5. then `strategy` id lexicographic ascending (determinism backstop).
//!
//! **Crown** = `order[0]`. Fragile iff all candidates are Fragile.
//!
//! **Determinism:** pure function, no I/O, no f64 arithmetic — comparisons only.

use std::cmp::Ordering;

use super::{CandidateResult, ReasonCode, RecommendationOutcome, RobustnessFlag};

/// Output of `rank_candidates`.
#[derive(Debug, Clone)]
pub struct Ranking {
    /// Indices into the input slice, best-first.
    pub order: Vec<usize>,
    /// The crowned index (= `order[0]`; `None` only when input is empty).
    pub crowned: Option<usize>,
    /// Why the crowned candidate won (deterministic reason codes).
    pub reasons: Vec<ReasonCode>,
    /// Which honesty branch fired.
    pub outcome: RecommendationOutcome,
}

/// Rank `candidates` by the F2 comparator.
///
/// Inputs must not be modified — this is a pure function of the slice.
/// Identical input ⇒ identical `Ranking` on any run.
#[must_use]
pub fn rank_candidates(candidates: &[CandidateResult]) -> Ranking {
    if candidates.is_empty() {
        return Ranking {
            order: vec![],
            crowned: None,
            reasons: vec![],
            outcome: RecommendationOutcome::AllFragile,
        };
    }

    // Build index vector and sort by the total comparator.
    let mut indices: Vec<usize> = (0..candidates.len()).collect();
    indices.sort_by(|&a, &b| compare(candidates, a, b));

    let crowned_idx = indices[0];
    let crowned = &candidates[crowned_idx];

    // Determine outcome.
    let crown_is_fragile = crowned.robustness == Some(RobustnessFlag::Fragile);
    let all_fragile = candidates
        .iter()
        .all(|c| c.robustness == Some(RobustnessFlag::Fragile));

    let outcome = if all_fragile {
        RecommendationOutcome::AllFragile
    } else if crowned.is_benchmark {
        RecommendationOutcome::BenchmarkWins
    } else {
        RecommendationOutcome::ActiveWins
    };

    // Build reasons.
    let reasons = build_reasons(candidates, crowned_idx, &indices, outcome, crown_is_fragile);

    Ranking {
        order: indices,
        crowned: Some(crowned_idx),
        reasons,
        outcome,
    }
}

// ── Comparator ────────────────────────────────────────────────────────────────

/// Total comparator implementing the F2 contract (best-first).
///
/// Returns `Less` when `a` is **better** than `b` (sort ascending → best first).
fn compare(candidates: &[CandidateResult], a: usize, b: usize) -> Ordering {
    let ca = &candidates[a];
    let cb = &candidates[b];

    // 1. Eligibility partition: eligible (non-Fragile) before Fragile.
    let ea = is_eligible(ca);
    let eb = is_eligible(cb);
    match (ea, eb) {
        (true, false) => return Ordering::Less, // a eligible, b fragile → a wins
        (false, true) => return Ordering::Greater, // b eligible, a fragile → b wins
        _ => {}
    }

    // 2. Sharpe descending (f64::total_cmp — total order, NaN-safe).
    match cb.kpis.sharpe.total_cmp(&ca.kpis.sharpe) {
        Ordering::Equal => {}
        ord => return ord, // higher sharpe is better
    }

    // 3. total_return_pct descending (Decimal::cmp, exact).
    match cb.kpis.total_return_pct.cmp(&ca.kpis.total_return_pct) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // 4. max_drawdown ascending (lower is better → a wins when a < b).
    match ca.kpis.max_drawdown.cmp(&cb.kpis.max_drawdown) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // 5. Strategy id lexicographic ascending (determinism backstop).
    ca.strategy.0.as_str().cmp(cb.strategy.0.as_str())
}

/// Whether a candidate is eligible to be crowned (not Fragile).
fn is_eligible(c: &CandidateResult) -> bool {
    c.robustness != Some(RobustnessFlag::Fragile)
}

// ── Reason builder ────────────────────────────────────────────────────────────

/// Build the ordered reason codes for the crowned pick.
///
/// Checks in order: primary outcome code, then tie-break codes if the crown
/// was decided by a tie-break (detected by comparing values).
fn build_reasons(
    candidates: &[CandidateResult],
    crowned_idx: usize,
    order: &[usize],
    outcome: RecommendationOutcome,
    _crown_is_fragile: bool,
) -> Vec<ReasonCode> {
    let mut reasons = Vec::new();
    let crowned = &candidates[crowned_idx];

    // Find the benchmark arm (if any).
    let benchmark = candidates.iter().find(|c| c.is_benchmark);

    match outcome {
        RecommendationOutcome::AllFragile => {
            reasons.push(ReasonCode::AllCandidatesFragile);
        }
        RecommendationOutcome::BenchmarkWins => {
            reasons.push(ReasonCode::BenchmarkUndefeated);
        }
        RecommendationOutcome::ActiveWins => {
            reasons.push(ReasonCode::HighestRobustSharpe);
            // Add BeatBenchmarkSharpe if winner Sharpe > benchmark Sharpe.
            if benchmark.is_some_and(|bh| crowned.kpis.sharpe > bh.kpis.sharpe) {
                reasons.push(ReasonCode::BeatBenchmarkSharpe);
            }
        }
    }

    // Detect tie-break at position 1 (if there is a runner-up).
    if order.len() >= 2 {
        let runner_idx = order[1];
        let runner = &candidates[runner_idx];

        // Check if Sharpe is equal between crowned and runner-up.
        if crowned.kpis.sharpe.total_cmp(&runner.kpis.sharpe) == Ordering::Equal {
            // Crown was decided by return or drawdown tie-break.
            if crowned.kpis.total_return_pct != runner.kpis.total_return_pct {
                reasons.push(ReasonCode::TieBrokenByReturn);
            } else if crowned.kpis.max_drawdown != runner.kpis.max_drawdown {
                reasons.push(ReasonCode::TieBrokenByDrawdown);
            }
            // else: lexicographic tie-break (no user-visible reason code added)
        }
    }

    reasons
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::bakeoff::{CandidateKpis, CandidateResult, RobustnessFlag};
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use trading_core::StrategyId;

    fn make_candidate(
        id: &str,
        sharpe: f64,
        total_return: rust_decimal::Decimal,
        max_drawdown: rust_decimal::Decimal,
        is_benchmark: bool,
        robustness: Option<RobustnessFlag>,
    ) -> CandidateResult {
        CandidateResult {
            strategy: StrategyId(SmolStr::new(id)),
            is_benchmark,
            kpis: CandidateKpis {
                sharpe,
                sortino: 0.0,
                calmar: 0.0,
                total_return_pct: total_return,
                max_drawdown,
                trade_count: 0,
            },
            equity_curve: vec![],
            robustness,
        }
    }

    // ── T6.2 — Sharpe-primary ordering ───────────────────────────────────────

    /// Three candidates with distinct Sharpes → order is Sharpe-desc, crown = highest.
    #[test]
    fn t62_sharpe_primary_ordering() {
        let candidates = vec![
            make_candidate("v0.sma", 1.5, dec!(0.10), dec!(0.05), false, None),
            make_candidate("v0.5.macd", 0.8, dec!(0.08), dec!(0.04), false, None),
            make_candidate("v0.buyhold", 0.5, dec!(0.05), dec!(0.03), true, None),
        ];
        let r = rank_candidates(&candidates);
        assert_eq!(r.order[0], 0, "v0.sma (highest sharpe) should be first");
        assert_eq!(r.order[1], 1, "v0.5.macd should be second");
        assert_eq!(r.order[2], 2, "v0.buyhold should be last");
        assert_eq!(r.crowned, Some(0));
        assert_eq!(r.outcome, RecommendationOutcome::ActiveWins);
    }

    // ── T6.3 — Robustness gate: Fragile high-Sharpe vs Robust lower-Sharpe ──

    /// High-Sharpe FRAGILE candidate loses to lower-Sharpe ROBUST one.
    #[test]
    fn t63_robustness_gate() {
        let candidates = vec![
            make_candidate(
                "v0.sma",
                2.0,
                dec!(0.20),
                dec!(0.05),
                false,
                Some(RobustnessFlag::Fragile),
            ),
            make_candidate(
                "v0.5.macd",
                1.2,
                dec!(0.12),
                dec!(0.04),
                false,
                Some(RobustnessFlag::Robust),
            ),
            make_candidate("v0.buyhold", 0.5, dec!(0.05), dec!(0.03), true, None),
        ];
        let r = rank_candidates(&candidates);
        // Robust v0.5.macd should be crowned, not Fragile v0.sma.
        assert_eq!(r.crowned, Some(1), "ROBUST lower-Sharpe should be crowned");
        assert_eq!(r.outcome, RecommendationOutcome::ActiveWins);
        // Fragile v0.sma still in the leaderboard but at position 2 or 3.
        assert!(
            r.order.contains(&0),
            "fragile candidate still appears in ranking"
        );
        // The Fragile candidate must be ranked AFTER the eligible ones.
        let fragile_pos = r.order.iter().position(|&i| i == 0).unwrap();
        let robust_pos = r.order.iter().position(|&i| i == 1).unwrap();
        assert!(robust_pos < fragile_pos, "Robust must rank before Fragile");
        assert_ne!(r.outcome, RecommendationOutcome::AllFragile);
    }

    // ── T6.4 — Buy-and-hold wins ─────────────────────────────────────────────

    /// Benchmark has highest eligible Sharpe → `BenchmarkWins` + `BenchmarkUndefeated`.
    #[test]
    fn t64_benchmark_wins() {
        let candidates = vec![
            make_candidate("v0.sma", 0.8, dec!(0.08), dec!(0.05), false, None),
            make_candidate("v0.buyhold", 1.5, dec!(0.15), dec!(0.02), true, None),
        ];
        let r = rank_candidates(&candidates);
        // buyhold (idx 1) has higher Sharpe → crowned.
        assert_eq!(r.crowned, Some(1));
        assert!(candidates[r.crowned.unwrap()].is_benchmark);
        assert_eq!(r.outcome, RecommendationOutcome::BenchmarkWins);
        assert!(
            r.reasons.contains(&ReasonCode::BenchmarkUndefeated),
            "BenchmarkUndefeated should be in reasons"
        );
    }

    // ── T6.5 — All fragile ──────────────────────────────────────────────────

    /// Every candidate Fragile → `AllFragile`, crown = highest Sharpe overall.
    #[test]
    fn t65_all_fragile() {
        let candidates = vec![
            make_candidate(
                "v0.sma",
                2.0,
                dec!(0.20),
                dec!(0.05),
                false,
                Some(RobustnessFlag::Fragile),
            ),
            make_candidate(
                "v0.buyhold",
                1.0,
                dec!(0.10),
                dec!(0.03),
                true,
                Some(RobustnessFlag::Fragile),
            ),
        ];
        let r = rank_candidates(&candidates);
        assert_eq!(r.outcome, RecommendationOutcome::AllFragile);
        // Crown = highest Sharpe (v0.sma, idx 0).
        assert_eq!(r.crowned, Some(0));
        assert!(
            r.reasons.contains(&ReasonCode::AllCandidatesFragile),
            "AllCandidatesFragile should be in reasons"
        );
    }

    // ── T6.6 — Tie-breaks ────────────────────────────────────────────────────

    /// Equal Sharpe → higher total return wins (`TieBrokenByReturn`).
    #[test]
    fn t66_tiebreak_return() {
        let candidates = vec![
            make_candidate("v0.sma", 1.0, dec!(0.10), dec!(0.05), false, None),
            make_candidate("v0.5.macd", 1.0, dec!(0.20), dec!(0.05), false, None),
        ];
        let r = rank_candidates(&candidates);
        // v0.5.macd has higher total return → crowned.
        assert_eq!(r.crowned, Some(1), "higher return should win the tie");
        assert!(
            r.reasons.contains(&ReasonCode::TieBrokenByReturn),
            "TieBrokenByReturn should be in reasons"
        );
    }

    /// Equal Sharpe + return → lower drawdown wins (`TieBrokenByDrawdown`).
    #[test]
    fn t66_tiebreak_drawdown() {
        let candidates = vec![
            make_candidate("v0.sma", 1.0, dec!(0.10), dec!(0.10), false, None),
            make_candidate("v0.5.macd", 1.0, dec!(0.10), dec!(0.05), false, None),
        ];
        let r = rank_candidates(&candidates);
        // v0.5.macd has lower max_drawdown → crowned.
        assert_eq!(r.crowned, Some(1), "lower drawdown should win the tie");
        assert!(
            r.reasons.contains(&ReasonCode::TieBrokenByDrawdown),
            "TieBrokenByDrawdown should be in reasons"
        );
    }

    /// Fully-equal KPIs → lexicographic id (determinism backstop).
    #[test]
    fn t66_tiebreak_lexicographic() {
        let candidates = vec![
            make_candidate("v0.sma", 1.0, dec!(0.10), dec!(0.05), false, None),
            make_candidate("v0.5.macd", 1.0, dec!(0.10), dec!(0.05), false, None),
        ];
        let r1 = rank_candidates(&candidates);
        let r2 = rank_candidates(&candidates);
        // Deterministic: same result twice.
        assert_eq!(r1.order, r2.order, "ranking must be deterministic");
        // Lexicographic: "v0.5.macd" < "v0.sma" (ASCII: '5' < 's')
        // so v0.5.macd should be crowned.
        assert_eq!(r1.crowned, Some(1), "v0.5.macd < v0.sma lexicographically");
    }

    /// Determinism: calling `rank_candidates` twice on the same input produces
    /// an identical `Ranking`.
    #[test]
    fn determinism_same_input_same_output() {
        let candidates = vec![
            make_candidate("v0.sma", 1.5, dec!(0.10), dec!(0.05), false, None),
            make_candidate("v0.5.macd", 0.8, dec!(0.08), dec!(0.04), false, None),
            make_candidate("v0.buyhold", 1.8, dec!(0.18), dec!(0.02), true, None),
        ];
        let r1 = rank_candidates(&candidates);
        let r2 = rank_candidates(&candidates);
        assert_eq!(r1.order, r2.order);
        assert_eq!(r1.crowned, r2.crowned);
        assert_eq!(r1.reasons, r2.reasons);
        assert_eq!(r1.outcome, r2.outcome);
    }

    /// Empty input → no panic, no crowned.
    #[test]
    fn empty_input_no_panic() {
        let r = rank_candidates(&[]);
        assert!(r.order.is_empty());
        assert_eq!(r.crowned, None);
    }

    /// Single candidate → always crowned (even if Fragile — no other candidates).
    #[test]
    fn single_fragile_candidate_is_crowned() {
        let candidates = vec![make_candidate(
            "v0.sma",
            1.0,
            dec!(0.10),
            dec!(0.05),
            false,
            Some(RobustnessFlag::Fragile),
        )];
        let r = rank_candidates(&candidates);
        assert_eq!(r.crowned, Some(0));
        assert_eq!(r.outcome, RecommendationOutcome::AllFragile);
    }

    /// `ActiveWins`: winner Sharpe > benchmark Sharpe → `BeatBenchmarkSharpe` in reasons.
    #[test]
    fn active_wins_beat_benchmark_reason() {
        let candidates = vec![
            make_candidate("v0.sma", 2.0, dec!(0.20), dec!(0.05), false, None),
            make_candidate("v0.buyhold", 1.0, dec!(0.10), dec!(0.02), true, None),
        ];
        let r = rank_candidates(&candidates);
        assert_eq!(r.outcome, RecommendationOutcome::ActiveWins);
        assert!(
            r.reasons.contains(&ReasonCode::HighestRobustSharpe),
            "HighestRobustSharpe expected"
        );
        assert!(
            r.reasons.contains(&ReasonCode::BeatBenchmarkSharpe),
            "BeatBenchmarkSharpe expected when winner beats BH"
        );
    }
}
