//! R6 — Memory highlights (placeholder body) + T811 strategy-decay
//! heuristic for R9.
//!
//! ## Forward-compatibility note (T811 / Q9 carry-forward)
//!
//! The placeholder body string returned by [`render`] is **locked into
//! the v1+ operator-success-report anchor SHAs** captured at T816's
//! first successful run.  When the future reflection-memory feature
//! ships, that feature's brief MUST include a deliverable to re-lock
//! the two new operator-success-report anchors (the same precedent
//! v1.5a applied to the top10-momentum anchors at task **T717** of
//! `spec/tasks/v15a-mean-reversion-pairs.md` — "anchor re-lock"
//! pattern).  Without re-locking, the determinism gate will FAIL on
//! the first run after the placeholder body changes.  See
//! `spec/reports/memory-anchor-relock-TBD.md` for a stub note that
//! the eventual reflection-memory architect can grep for.
//!
//! The decay heuristic below is the R9 "Strategy decay" risk:
//! `last7d_sharpe < 0 && inception_sharpe > 0` for any strategy in the
//! active set.  It accepts a [`SharpeFn`] so this module can be unit
//! tested before the real R4 risk-metrics implementation lands.

use rust_decimal::Decimal;

use crate::render::risk_metrics::SharpeFn;

/// R6 placeholder body — locked into v1+ anchor SHAs.  See the module
/// rustdoc above for the relock contract.
///
/// The string is intentionally short, deterministic, and free of any
/// run-varying field (no timestamps, no run-id, no hostname).  R10.3
/// determinism + R10.4 negative-invariant tests rely on this.
pub const PLACEHOLDER: &str = "_reflection memory not yet implemented._\n";

/// Render the R6 memory-highlights body for the report.
///
/// v1+ ships the placeholder verbatim; reflection memory replaces this
/// in a future feature.  Until then the byte-stable [`PLACEHOLDER`]
/// constant is the single source of truth.
#[must_use]
pub fn render() -> String {
    PLACEHOLDER.to_string()
}

/// Render the R6 memory-highlights body, including a one-line "decay
/// candidates" footer when the strategy-decay heuristic fired in this
/// period.
///
/// `decayed` is the list of strategy ids that the heuristic flagged
/// (already lex-sorted by [`decayed_strategies`]).  An empty slice is
/// equivalent to [`render`].
///
/// The heading + section title `## Memory highlights` is prepended so
/// the orchestrator does not need to wrap the placeholder.  Output is
/// pure over `decayed` — same input → byte-identical bytes.
#[must_use]
pub fn render_with_decay(decayed: &[String]) -> String {
    let mut out = String::with_capacity(256);
    out.push_str("## Memory highlights\n\n");
    out.push_str(PLACEHOLDER);
    if !decayed.is_empty() {
        out.push('\n');
        out.push_str("decay candidates: ");
        out.push_str(&decayed.join(", "));
        out.push('\n');
    }
    out
}

/// One strategy's per-window equity slice + identifier.  Used by
/// [`decay_fired`] to thread per-strategy series into the heuristic
/// without depending on the audit query types.
#[derive(Debug, Clone)]
pub struct StrategyEquitySlice {
    /// Strategy identifier (display only — the heuristic does not key
    /// on it; sorting is the caller's responsibility).
    pub strategy_id: String,
    /// Restricted equity curve at 1m / 5m cadence (Decimal-only, no
    /// `f64`).
    pub equity: Vec<Decimal>,
    /// Same curve restricted to the trailing 7 days.
    pub last_7d_equity: Vec<Decimal>,
}

/// Compute "decay fired?" for the strategy decay R9 risk.
///
/// Returns `true` iff any strategy in `slices` has `last_7d_sharpe < 0`
/// **and** `inception_sharpe > 0`.  Both Sharpe values come from
/// `sharpe_fn`, the injected R4 risk-metrics callable; a synthetic
/// stats function is acceptable for unit tests.
///
/// The function is **pure** over its inputs — no I/O, no clock, no
/// hidden state.  Two calls with the same `slices` + `sharpe_fn`
/// return the same boolean.
#[must_use]
pub fn decay_fired(slices: &[StrategyEquitySlice], sharpe_fn: SharpeFn) -> bool {
    for s in slices {
        let inception = sharpe_fn(&s.equity);
        let last_7d = sharpe_fn(&s.last_7d_equity);
        if last_7d.last_7d < 0.0 && inception.inception > 0.0 {
            return true;
        }
    }
    false
}

/// Return the per-strategy slices that triggered the decay risk.
///
/// Used by the R9 renderer (T813) when the operator wants to see which
/// specific strategies decayed.  Same purity guarantees as
/// [`decay_fired`].
#[must_use]
pub fn decayed_strategies(slices: &[StrategyEquitySlice], sharpe_fn: SharpeFn) -> Vec<String> {
    let mut out = Vec::new();
    for s in slices {
        let inception = sharpe_fn(&s.equity);
        let last_7d = sharpe_fn(&s.last_7d_equity);
        if last_7d.last_7d < 0.0 && inception.inception > 0.0 {
            out.push(s.strategy_id.clone());
        }
    }
    // Sort by strategy id ASC for byte-stable output (HashMap-iter
    // determinism rule from the developer-agent checklist).
    out.sort();
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::render::risk_metrics::SharpeStats;
    use rust_decimal_macros::dec;

    /// Synthetic Sharpe provider used so we can exercise the heuristic
    /// without the real R4 module.  The first cell of the slice carries
    /// the "inception" Sharpe; the rest are unused.  The last cell of
    /// the slice carries the `last_7d` Sharpe (we read both so a single
    /// call returns a structurally-correct `SharpeStats` and the
    /// helper sees both fields).
    fn synthetic_sharpe(values: &[Decimal]) -> SharpeStats {
        let parse_first = values
            .first()
            .copied()
            .map_or(0.0, |d| d.to_string().parse::<f64>().unwrap_or(0.0));
        let parse_last = values
            .last()
            .copied()
            .map_or(0.0, |d| d.to_string().parse::<f64>().unwrap_or(0.0));
        SharpeStats {
            inception: parse_first,
            last_7d: parse_last,
        }
    }

    #[test]
    fn t811_render_returns_placeholder_byte_stable() {
        let a = render();
        let b = render();
        assert_eq!(a, b);
        assert_eq!(a, PLACEHOLDER);
    }

    #[test]
    fn t813_render_with_decay_no_decay_emits_placeholder() {
        let body = render_with_decay(&[]);
        assert!(body.starts_with("## Memory highlights\n\n"));
        assert!(body.contains(PLACEHOLDER));
        assert!(!body.contains("decay candidates:"));
    }

    #[test]
    fn t813_render_with_decay_emits_one_line_per_decay() {
        let decayed = vec!["alpha".to_string(), "zeta".to_string()];
        let body = render_with_decay(&decayed);
        assert!(body.contains("decay candidates: alpha, zeta"));
    }

    #[test]
    fn t811_placeholder_contains_no_run_varying_fields() {
        // No timestamps, no run-id, no hostname leak into the body.
        for forbidden in [
            "generated:",
            "run_id:",
            "wall_clock_s:",
            "ledger_snapshot_sha:",
            "data_source:",
            "agent_pid:",
            "host:",
            "git_commit:",
        ] {
            assert!(
                !PLACEHOLDER.contains(forbidden),
                "placeholder leaks volatile field {forbidden}"
            );
        }
    }

    #[test]
    fn t811_decay_fires_when_inception_pos_and_last7d_neg() {
        // Strategy A: inception positive (1.0), last_7d negative (-1.0).
        let slice = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0), dec!(2.0), dec!(3.0)],
            last_7d_equity: vec![dec!(2.0), dec!(1.0), dec!(-1.0)],
        };
        assert!(decay_fired(&[slice], synthetic_sharpe));
    }

    #[test]
    fn t811_decay_does_not_fire_when_both_positive() {
        let slice = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0), dec!(2.0), dec!(3.0)],
            last_7d_equity: vec![dec!(2.0), dec!(3.0), dec!(4.0)],
        };
        assert!(!decay_fired(&[slice], synthetic_sharpe));
    }

    #[test]
    fn t811_decay_does_not_fire_when_inception_negative() {
        let slice = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(-1.0), dec!(2.0)],
            last_7d_equity: vec![dec!(2.0), dec!(-1.0)],
        };
        assert!(!decay_fired(&[slice], synthetic_sharpe));
    }

    #[test]
    fn t811_decay_two_strategy_fixture() {
        // Two strategies — `alpha` decays, `beta` healthy.
        let alpha = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0)],
            last_7d_equity: vec![dec!(-2.0)],
        };
        let beta = StrategyEquitySlice {
            strategy_id: "beta".into(),
            equity: vec![dec!(1.0)],
            last_7d_equity: vec![dec!(1.0)],
        };
        assert!(decay_fired(
            &[alpha.clone(), beta.clone()],
            synthetic_sharpe
        ));
        let names = decayed_strategies(&[alpha, beta], synthetic_sharpe);
        assert_eq!(names, vec!["alpha".to_string()]);
    }

    #[test]
    fn t811_decayed_strategies_returns_sorted_ids() {
        // Reverse-input order; output must be ASC sorted.
        let zeta = StrategyEquitySlice {
            strategy_id: "zeta".into(),
            equity: vec![dec!(1.0)],
            last_7d_equity: vec![dec!(-1.0)],
        };
        let alpha = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0)],
            last_7d_equity: vec![dec!(-1.0)],
        };
        let names = decayed_strategies(&[zeta, alpha], synthetic_sharpe);
        assert_eq!(names, vec!["alpha".to_string(), "zeta".to_string()]);
    }

    #[test]
    fn t811_decay_pure_two_calls_equal() {
        let slice = StrategyEquitySlice {
            strategy_id: "alpha".into(),
            equity: vec![dec!(1.0), dec!(2.0)],
            last_7d_equity: vec![dec!(2.0), dec!(-1.0)],
        };
        let one = std::slice::from_ref(&slice);
        let a = decay_fired(one, synthetic_sharpe);
        let b = decay_fired(one, synthetic_sharpe);
        assert_eq!(a, b);
    }
}
