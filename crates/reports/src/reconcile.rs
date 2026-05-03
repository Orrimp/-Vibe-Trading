//! Reconciliation engine (R11 / Q6).
//!
//! Exact-cent equality (`Decimal == Decimal`) across the four R11
//! identities:
//!
//! 1. `headline_return = realized + unrealized`
//! 2. `Σ pnl_by_strategy = Σ realized`
//! 3. `Σ pnl_by_symbol = Σ realized`
//! 4. `equity_period_end - equity_period_start = realized + unrealized + fees_delta`
//!    (rendered split across two lines in the appendix for width)
//!
//! On any mismatch the renderer (T813) prepends a FAIL banner above
//! R9, the appendix table prints `FAIL` in the failing rows, a sibling
//! JSON artifact lands at `<output stem>_reconciliation_failure.json`,
//! and the bin exits 1.  No tolerance — `passed = (delta ==
//! Decimal::ZERO)` per the operator's Q6 default.

use std::fmt::Write;

use rust_decimal::Decimal;

/// One row of the four reconciliation identities.
#[derive(Debug, Clone)]
pub struct ReconciliationRow {
    /// Human-readable identity, e.g. `"headline_return = realized +
    /// unrealized"`.
    pub identity: &'static str,
    /// Report-side value (the value carried by the report's narrative
    /// — e.g. headline return).
    pub report_side: Decimal,
    /// Ledger-side value (the value derived from a fresh audit query
    /// at render time).
    pub ledger_side: Decimal,
    /// `report_side - ledger_side`.  `passed = (delta == 0)`.
    pub delta: Decimal,
    /// Exact-cent equality result (`delta == Decimal::ZERO`).
    pub passed: bool,
}

impl ReconciliationRow {
    /// Build a row from `(report_side, ledger_side)`; `delta` and
    /// `passed` are derived.
    #[must_use]
    pub fn new(identity: &'static str, report_side: Decimal, ledger_side: Decimal) -> Self {
        let delta = report_side - ledger_side;
        Self {
            identity,
            report_side,
            ledger_side,
            delta,
            passed: delta == Decimal::ZERO,
        }
    }
}

/// All four R11 identities for one report.
#[derive(Debug, Clone)]
pub struct ReconciliationReport {
    /// `headline_return = realized + unrealized`.
    pub headline: ReconciliationRow,
    /// `Σ pnl_by_strategy = Σ realized`.
    pub by_strategy: ReconciliationRow,
    /// `Σ pnl_by_symbol = Σ realized`.
    pub by_symbol: ReconciliationRow,
    /// `equity_period_end - equity_period_start
    ///    = realized + unrealized + fees_delta`.
    pub equity: ReconciliationRow,
}

/// Inputs to the reconciliation engine.
///
/// The 4 identities are computed from these `Decimal` inputs alone —
/// no I/O.  The orchestrator in `crate::generate` runs the queries and
/// hands frozen values to [`compute`].
#[derive(Debug, Clone)]
pub struct ReconciliationInputs {
    /// R11 identity #1: report-side headline return.
    pub headline_return: Decimal,
    /// R11 identity #1, ledger-side: realized + unrealized at
    /// `period_end`.
    pub realized: Decimal,
    /// R11 identity #1, ledger-side: unrealized P&L at `period_end`
    /// (mark-to-market over open positions).
    pub unrealized: Decimal,
    /// R11 identity #2: `Σ pnl_by_strategy.realized` over the window.
    pub sum_by_strategy: Decimal,
    /// R11 identity #3: `Σ pnl_by_symbol.realized` over the window.
    pub sum_by_symbol: Decimal,
    /// R11 identity #4: `equity_period_end - equity_period_start`.
    pub equity_delta: Decimal,
    /// R11 identity #4, ledger-side: `realized + unrealized +
    /// fees_delta`.
    pub equity_check_sum: Decimal,
}

/// Run all four reconciliation identities and return a populated
/// [`ReconciliationReport`].
///
/// Pure over its inputs — same `inputs` produce the same report on
/// every invocation.
#[must_use]
pub fn compute(inputs: &ReconciliationInputs) -> ReconciliationReport {
    let headline = ReconciliationRow::new(
        "headline_return = realized + unrealized",
        inputs.headline_return,
        inputs.realized + inputs.unrealized,
    );
    let by_strategy = ReconciliationRow::new(
        "Σ pnl_by_strategy = Σ realized",
        inputs.sum_by_strategy,
        inputs.realized,
    );
    let by_symbol = ReconciliationRow::new(
        "Σ pnl_by_symbol = Σ realized",
        inputs.sum_by_symbol,
        inputs.realized,
    );
    let equity = ReconciliationRow::new(
        "equity_delta = realized + unrealized + fees_delta",
        inputs.equity_delta,
        inputs.equity_check_sum,
    );
    ReconciliationReport {
        headline,
        by_strategy,
        by_symbol,
        equity,
    }
}

impl ReconciliationReport {
    /// Returns `true` iff every identity has `delta == 0`.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.headline.passed
            && self.by_strategy.passed
            && self.by_symbol.passed
            && self.equity.passed
    }

    /// Iterator over the four rows in the locked render order.
    fn rows(&self) -> [&ReconciliationRow; 4] {
        [
            &self.headline,
            &self.by_strategy,
            &self.by_symbol,
            &self.equity,
        ]
    }

    /// R11.3 markdown appendix table.  Columns:
    ///
    /// `| identity | report_side | ledger_side | Δ | Pass? |`
    ///
    /// Decimal values render as plain TEXT (no scientific notation,
    /// no locale separators).  `Pass?` cells are `PASS` (literal,
    /// uppercase) on success and `FAIL` on mismatch.
    #[must_use]
    pub fn to_appendix_table(&self) -> String {
        let mut out = String::with_capacity(640);
        out.push_str("| Identity | Report | Ledger | Δ | Pass? |\n");
        out.push_str("|----------|--------|--------|---|-------|\n");
        for row in self.rows() {
            let pass_cell = if row.passed { "PASS" } else { "FAIL" };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} |",
                row.identity, row.report_side, row.ledger_side, row.delta, pass_cell
            );
        }
        out
    }

    /// Build the JSON body for the sibling
    /// `_reconciliation_failure.json` artifact.
    ///
    /// Caller passes the same identifiers it wrote into the
    /// front-matter so the JSON sidecar can be cross-referenced
    /// without re-parsing the markdown.
    #[must_use]
    pub fn to_failure_json(
        &self,
        run_id: &str,
        ledger_sha: &str,
        period: &str,
        period_start: &str,
        period_end: &str,
    ) -> String {
        let mut out = String::with_capacity(1024);
        out.push_str("{\n");
        let _ = writeln!(out, "  \"schema_version\": 1,");
        let _ = writeln!(out, "  \"run_id\": \"{run_id}\",");
        let _ = writeln!(out, "  \"ledger_snapshot_sha\": \"{ledger_sha}\",");
        let _ = writeln!(out, "  \"period\": \"{period}\",");
        let _ = writeln!(out, "  \"period_start\": \"{period_start}\",");
        let _ = writeln!(out, "  \"period_end\": \"{period_end}\",");
        out.push_str("  \"rows\": [\n");
        let rows = self.rows();
        for (i, row) in rows.iter().enumerate() {
            out.push_str("    {\n");
            let _ = writeln!(out, "      \"identity\": \"{}\",", row.identity);
            let _ = writeln!(out, "      \"report_side\": \"{}\",", row.report_side);
            let _ = writeln!(out, "      \"ledger_side\": \"{}\",", row.ledger_side);
            let _ = writeln!(out, "      \"delta\": \"{}\",", row.delta);
            let _ = writeln!(out, "      \"passed\": {}", row.passed);
            out.push_str("    }");
            if i + 1 < rows.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n");
        out.push_str("}\n");
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn balanced_inputs() -> ReconciliationInputs {
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
    fn t808_balanced_inputs_all_pass() {
        let r = compute(&balanced_inputs());
        assert!(r.all_passed());
        assert!(r.headline.passed);
        assert!(r.by_strategy.passed);
        assert!(r.by_symbol.passed);
        assert!(r.equity.passed);
    }

    #[test]
    fn t808_one_cent_delta_in_headline_only_fails_that_row() {
        let mut inp = balanced_inputs();
        inp.headline_return = dec!(150.01);
        let r = compute(&inp);
        assert!(!r.all_passed());
        assert!(!r.headline.passed);
        assert!(r.by_strategy.passed);
        assert!(r.by_symbol.passed);
        assert!(r.equity.passed);
        assert_eq!(r.headline.delta, dec!(0.01));
    }

    #[test]
    fn t808_strategy_mismatch_only_fails_strategy_row() {
        let mut inp = balanced_inputs();
        inp.sum_by_strategy = dec!(99.99);
        let r = compute(&inp);
        assert!(!r.all_passed());
        assert!(r.headline.passed);
        assert!(!r.by_strategy.passed);
        assert!(r.by_symbol.passed);
        assert!(r.equity.passed);
    }

    #[test]
    fn t808_appendix_table_pass_cells_uppercase() {
        let r = compute(&balanced_inputs());
        let t = r.to_appendix_table();
        assert!(t.contains("| PASS |"));
        assert!(!t.contains("| FAIL |"));
    }

    #[test]
    fn t808_appendix_table_fail_cells_uppercase() {
        let mut inp = balanced_inputs();
        inp.headline_return = dec!(150.05);
        let r = compute(&inp);
        let t = r.to_appendix_table();
        assert!(t.contains("| FAIL |"));
        assert!(t.contains("| PASS |"));
    }

    #[test]
    fn t808_failure_json_round_trip_serde_value() {
        let mut inp = balanced_inputs();
        inp.headline_return = dec!(150.05);
        let r = compute(&inp);
        let s = r.to_failure_json(
            "deadbeefcafebabe",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "7d",
            "2026-04-24T00:00:00.000000Z",
            "2026-05-01T00:00:00.000000Z",
        );
        // Round-trip via serde_json::Value to assert it's valid JSON.
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["run_id"], "deadbeefcafebabe");
        assert_eq!(v["period"], "7d");
        assert!(v["rows"].is_array());
        assert_eq!(v["rows"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn t808_compute_pure_two_calls_equal() {
        let a = compute(&balanced_inputs());
        let b = compute(&balanced_inputs());
        assert_eq!(a.all_passed(), b.all_passed());
        assert_eq!(a.headline.delta, b.headline.delta);
        assert_eq!(a.by_strategy.delta, b.by_strategy.delta);
    }
}
