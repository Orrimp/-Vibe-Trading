//! R9 — Open risks (5 threshold checks).
//!
//! Each risk's input is a `Result<RiskOutcome, String>`:
//!  - `Ok(RiskOutcome { fired, threshold, observed })` when the
//!    underlying query succeeded.  `fired = true` triggers a bullet in
//!    the output; `false` is silently dropped.
//!  - `Err(_)` renders `unknown — see logs` per R9.3.
//!
//! All-clear renders the literal `_no open risks._` sentinel.

use std::fmt::Write;

/// One risk's outcome.
#[derive(Debug, Clone)]
pub struct RiskOutcome {
    /// Whether the threshold fired.
    pub fired: bool,
    /// Human-readable threshold (e.g. `"current_drawdown >= 11.25%"`).
    pub threshold: String,
    /// Human-readable observed value (e.g. `"12.10%"`).
    pub observed: String,
}

/// Per-risk source result.  `Err` arms render `unknown — see logs`.
pub type RiskCell = Result<RiskOutcome, String>;

/// Inputs for the R9 open-risks section.
///
/// Order matches the Design's R9 table — the rendered bullets follow
/// the same order so the operator's mental model maps 1:1.
#[derive(Debug, Clone)]
pub struct OpenRisksInputs {
    /// Drawdown approaching limit.
    pub drawdown: RiskCell,
    /// LLM budget approaching cap.
    pub llm_budget: RiskCell,
    /// Strategy decay (any strategy: last-7d Sharpe < 0 + inception > 0).
    pub strategy_decay: RiskCell,
    /// Rebalance rejections accumulating.
    pub rebalance_rejections: RiskCell,
    /// Mean-reversion hard stops accumulating.
    pub mr_stops: RiskCell,
}

/// Render the R9 section.  Pinned above the equity curve in the body
/// per R9.1 — the orchestrator places this output ahead of the R3
/// equity-curve section.
#[must_use]
pub fn render(inputs: &OpenRisksInputs) -> String {
    let mut out = String::with_capacity(384);
    out.push_str("## Open risks\n\n");

    let mut any_fired = false;
    any_fired |= push_risk(&mut out, "Drawdown approaching limit", &inputs.drawdown);
    any_fired |= push_risk(&mut out, "LLM budget approaching cap", &inputs.llm_budget);
    any_fired |= push_risk(&mut out, "Strategy decay", &inputs.strategy_decay);
    any_fired |= push_risk(
        &mut out,
        "Rebalance rejections accumulating",
        &inputs.rebalance_rejections,
    );
    any_fired |= push_risk(
        &mut out,
        "Mean-reversion hard stops accumulating",
        &inputs.mr_stops,
    );

    if !any_fired {
        out.push_str("_no open risks._\n");
    }
    out
}

fn push_risk(out: &mut String, name: &str, cell: &RiskCell) -> bool {
    match cell {
        Ok(outcome) if outcome.fired => {
            let _ = writeln!(
                out,
                "- {name}: threshold {} (observed {})",
                outcome.threshold, outcome.observed,
            );
            true
        }
        Ok(_) => false,
        Err(_) => {
            let _ = writeln!(out, "- {name}: unknown — see logs");
            // Treat `unknown` as a non-clear state — operator still sees it.
            true
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_wraps)]
mod tests {
    use super::*;

    fn clear() -> RiskCell {
        Ok(RiskOutcome {
            fired: false,
            threshold: String::new(),
            observed: String::new(),
        })
    }

    fn fired(threshold: &str, observed: &str) -> RiskCell {
        Ok(RiskOutcome {
            fired: true,
            threshold: threshold.into(),
            observed: observed.into(),
        })
    }

    #[test]
    fn t813_open_risks_all_clear_renders_sentinel() {
        let inp = OpenRisksInputs {
            drawdown: clear(),
            llm_budget: clear(),
            strategy_decay: clear(),
            rebalance_rejections: clear(),
            mr_stops: clear(),
        };
        let body = render(&inp);
        assert!(body.contains("## Open risks"));
        assert!(body.contains("_no open risks._"));
    }

    #[test]
    fn t813_open_risks_fired_risks_render_threshold_and_observed() {
        let inp = OpenRisksInputs {
            drawdown: fired("current_drawdown >= 11.25%", "12.10%"),
            llm_budget: clear(),
            strategy_decay: clear(),
            rebalance_rejections: clear(),
            mr_stops: clear(),
        };
        let body = render(&inp);
        assert!(body.contains(
            "- Drawdown approaching limit: threshold current_drawdown >= 11.25% (observed 12.10%)"
        ));
        assert!(!body.contains("_no open risks._"));
    }

    #[test]
    fn t813_open_risks_err_cell_renders_unknown() {
        let inp = OpenRisksInputs {
            drawdown: Err("query failed".into()),
            llm_budget: clear(),
            strategy_decay: clear(),
            rebalance_rejections: clear(),
            mr_stops: clear(),
        };
        let body = render(&inp);
        assert!(body.contains("- Drawdown approaching limit: unknown — see logs"));
    }

    #[test]
    fn t813_open_risks_byte_stable_across_runs() {
        let inp = OpenRisksInputs {
            drawdown: clear(),
            llm_budget: clear(),
            strategy_decay: clear(),
            rebalance_rejections: clear(),
            mr_stops: clear(),
        };
        let a = render(&inp);
        let b = render(&inp);
        assert_eq!(a, b);
    }
}
