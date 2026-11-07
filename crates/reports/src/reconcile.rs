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
    ///
    /// Meaningless when `measured` is `false` — read that first.
    pub passed: bool,
    /// Whether this identity's inputs were actually available.
    ///
    /// `false` when a required input was declared absent rather than measured
    /// — today that is a period-end mark miss, which zeroes `unrealized` and so
    /// makes identities #1 and #4 uncomputable. Such a row renders `unmeasured`
    /// and is excluded from [`ReconciliationReport::all_passed`].
    ///
    /// Added 2026-11-07 (bug-log `#156`). Before, a mark miss left all four rows
    /// claiming PASS with a footnote underneath: the report zeroed an input and
    /// then reconciled against the zero. This project's rule is that a skip is
    /// unmeasured, never a pass — and symmetrically it is not a FAIL either,
    /// because the T1006 V6 contract requires a mark miss to degrade gracefully
    /// rather than block the report.
    pub measured: bool,
}

impl ReconciliationRow {
    /// Build a row from `(report_side, ledger_side)`; `delta` and
    /// `passed` are derived.
    #[must_use]
    pub fn new(identity: &'static str, report_side: Decimal, ledger_side: Decimal) -> Self {
        Self::new_with(true, identity, report_side, ledger_side)
    }

    /// Build a row that declares whether its inputs were measured at all.
    ///
    /// `measured: false` is for an identity whose input was zeroed because it was
    /// unavailable, not computed — see [`ReconciliationRow::measured`].
    #[must_use]
    pub fn new_with(
        measured: bool,
        identity: &'static str,
        report_side: Decimal,
        ledger_side: Decimal,
    ) -> Self {
        let delta = report_side - ledger_side;
        Self {
            identity,
            report_side,
            ledger_side,
            delta,
            passed: delta == Decimal::ZERO,
            measured,
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
    ///    = realized + unrealized - fees`.
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
    /// Whether `unrealized` above was MEASURED or zeroed because a period-end
    /// mark was unavailable (`mark_misses > 0` in `crate::generate`).
    ///
    /// Identities #1 and #4 both consume `unrealized`, so both are unmeasured
    /// when this is `false`. #2 and #3 read only realized figures and stay
    /// measured. Declaring the condition in the inputs — rather than taking a
    /// second argument to `compute` — keeps the caller's claim about its own
    /// data quality next to the data (bug-log `#156`).
    pub unrealized_measured: bool,
    /// R11 identity #4, ledger-side: realized P&L **over the window**
    /// (`realized_pnl_since(period_start)`), which is a DIFFERENT quantity
    /// from `realized` above (`realized_pnl_since(inception)`).
    ///
    /// The old wiring conflated them: identity #1 was fed the inception
    /// figure and identity #4 the period figure, from one `realized` field,
    /// and nothing said so. Identity #4 is a period identity — its report
    /// side is `equity_period_end - equity_period_start` — so it takes the
    /// period figure, and the two now have separate fields (bug-log `#156`).
    pub realized_period: Decimal,
    /// R11 identity #4, ledger-side: fee spend over the window
    /// (`expense:fees:taker` + `expense:fees:maker`), a POSITIVE number
    /// that identity #4 subtracts.
    ///
    /// Added 2026-11-07, bug-log `#156`: the identity's label has always
    /// read `+ fees_delta` while no fees term existed in the computation
    /// at all — `grep fees_delta` found three doc comments and the label
    /// string. The sign was wrong too: fees reduce equity.
    pub fees: Decimal,
    /// R11 identity #2: `Σ pnl_by_strategy.realized` over the window.
    pub sum_by_strategy: Decimal,
    /// R11 identity #3: `Σ pnl_by_symbol.realized` over the window.
    pub sum_by_symbol: Decimal,
    /// R11 identity #4, REPORT-side: `equity_period_end -
    /// equity_period_start`, taken from the equity curve the report
    /// actually publishes.
    ///
    /// There is deliberately **no** `equity_check_sum` field beside this
    /// one. Until 2026-11-07 there was, and `crate::generate` assigned
    /// the identical expression `realized_period.amount() + unrealized`
    /// to both — so `compute` subtracted a value from itself and the row
    /// published `Δ = 0 | PASS` unconditionally, for every input, in four
    /// committed operator-facing bodies (bug-log `#156`).
    ///
    /// Identities #1, #2 and #3 never had that defect because `compute`
    /// derives their ledger side itself from the primitives above. #4 was
    /// the only one that let the caller supply both sides. Removing the
    /// field makes the shape **impossible by construction** rather than
    /// merely fixed: a caller can now supply report-side values and
    /// ledger primitives, and nothing else.
    pub equity_delta: Decimal,
}

/// Run all four reconciliation identities and return a populated
/// [`ReconciliationReport`].
///
/// Pure over its inputs — same `inputs` produce the same report on
/// every invocation.
#[must_use]
pub fn compute(inputs: &ReconciliationInputs) -> ReconciliationReport {
    let headline = ReconciliationRow::new_with(
        inputs.unrealized_measured,
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
    let equity = ReconciliationRow::new_with(
        inputs.unrealized_measured,
        "equity_delta = realized + unrealized - fees",
        inputs.equity_delta,
        inputs.realized_period + inputs.unrealized - inputs.fees,
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
    /// An UNMEASURED row neither passes nor fails: it is excluded here, because
    /// a report whose period-end mark was unavailable must still be produced
    /// (T1006 V6) and must not claim a reconciliation it could not perform
    /// (bug-log `#156`). Read `measured` on each row to tell the two apart.
    pub fn all_passed(&self) -> bool {
        self.rows().iter().all(|r| !r.measured || r.passed)
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
            let pass_cell = if !row.measured {
                "unmeasured"
            } else if row.passed {
                "PASS"
            } else {
                "FAIL"
            };
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
            let _ = writeln!(out, "      \"measured\": {},", row.measured);
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
            unrealized_measured: true,
            realized_period: dec!(100.00),
            // 10.00, not 0: a zero fees term would be pinned by nothing.
            // 100 + 50 - 10 = 140 = equity_delta.
            fees: dec!(10.00),
            sum_by_strategy: dec!(100.00),
            sum_by_symbol: dec!(100.00),
            equity_delta: dec!(140.00),
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
