//! R11 — Reconciliation appendix (table + banner-on-FAIL + sibling
//! JSON).
//!
//! The body-side rendering for the appendix table delegates to
//! [`crate::reconcile::ReconciliationReport::to_appendix_table`] (T808).
//! The banner literal lives in this module so the orchestrator can
//! prepend it above R9 on FAIL per R11.4.

use std::fmt::Write;

use crate::reconcile::ReconciliationReport;

/// Banner line prepended above R9 on reconciliation failure (R11.4).
pub const FAIL_BANNER: &str = "*** RECONCILIATION FAILURE — see Reconciliation section ***";

/// Q6 / T1003 footnote — appended to the R11 appendix on the
/// no-mark-available branch.  Constant text (no run-varying
/// interpolation) so the footnote's presence/absence is fully
/// determined by the fixture's mark coverage, not parquet-root
/// wall-clock state.
pub const MARK_UNAVAILABLE_FOOTNOTE: &str =
    "*one or more open-position marks were unavailable at period_end; see logs*";

/// Render the R11 reconciliation appendix section.
///
/// Pure over its inputs — same `(report, mark_unavailable)` produce
/// byte-identical output.  When `mark_unavailable == true`, a
/// constant footnote (Q6) is appended below the appendix table; when
/// `false` (the default no-open-positions / all-marks-resolved
/// branch), the body emits the SAME bytes as the pre-T1003
/// no-footnote path.
#[must_use]
pub fn render(report: &ReconciliationReport, mark_unavailable: bool) -> String {
    let mut out = String::with_capacity(640);
    out.push_str("## Reconciliation\n\n");
    let _ = write!(out, "{}", report.to_appendix_table());
    if mark_unavailable {
        out.push('\n');
        out.push_str(MARK_UNAVAILABLE_FOOTNOTE);
        out.push('\n');
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::reconcile::{ReconciliationInputs, compute};
    use rust_decimal_macros::dec;

    fn balanced() -> ReconciliationInputs {
        ReconciliationInputs {
            headline_return: dec!(150.00),
            realized: dec!(100.00),
            unrealized: dec!(50.00),
            sum_by_strategy: dec!(100.00),
            sum_by_symbol: dec!(100.00),
            equity_delta: dec!(140.00),
            equity_check_sum: dec!(140.00),
        }
    }

    #[test]
    fn t813_reconciliation_section_contains_table_and_pass_cells() {
        let r = compute(&balanced());
        let body = render(&r, false);
        assert!(body.contains("## Reconciliation"));
        assert!(body.contains("| PASS |"));
    }

    #[test]
    fn t813_reconciliation_byte_stable_across_runs() {
        let r = compute(&balanced());
        let a = render(&r, false);
        let b = render(&r, false);
        assert_eq!(a, b);
    }

    #[test]
    fn t1003_reconciliation_no_footnote_on_false_flag() {
        // Critical anchor-preservation invariant: when
        // `mark_unavailable == false`, the body bytes match the
        // pre-T1003 no-footnote rendering exactly.  Existing fixtures
        // (`build_ledger_7d` / `build_ledger_90d`) have zero open
        // positions → `mark_unavailable = false` → 11/11 anchors
        // stay byte-identical.
        let r = compute(&balanced());
        let body = render(&r, false);
        assert!(!body.contains(MARK_UNAVAILABLE_FOOTNOTE));
    }

    #[test]
    fn t1003_reconciliation_footnote_appended_on_true_flag() {
        let r = compute(&balanced());
        let body = render(&r, true);
        assert!(body.contains(MARK_UNAVAILABLE_FOOTNOTE));
    }
}
