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
    /// Identity #1 is the only row that consumes `unrealized`, so it is the only
    /// one unmeasured when this is `false`. #2, #3 and #4 read journal figures
    /// only and stay measured — #4 deliberately excludes unrealized, because
    /// the journal never records it (bug-log `#159`). Declaring the condition in
    /// the inputs — rather than taking a second argument to `compute` — keeps
    /// the caller's claim about its own data quality next to the data
    /// (bug-log `#156`).
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
    /// R11 identity #4, ledger-side: every NON-fee expense over the window
    /// (`expense:%` minus `expense:fees:%`), a POSITIVE number that identity #4
    /// subtracts.
    ///
    /// Added 2026-11-07, bug-log `#159`. Identity #4's ledger side walks the
    /// journal by account CLASS, so it sums every `income:%` and `expense:%`
    /// row. Without this term the identity holds only for a ledger whose sole
    /// expense is trading fees; the live advisor also posts `expense:llm:%`,
    /// and the row would FAIL on a real ledger for a reason that is not a real
    /// imbalance.
    pub other_expense: Decimal,
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
    // Identity #4 asserts only what the JOURNAL can witness, which deliberately
    // excludes `unrealized` — see bug-log `#159`. `income:unrealized_pnl` and
    // `assets:position_mark:*` are created by `audit::bootstrap` and **nothing
    // in the repo ever posts to them**: unrealized P&L is computed at render
    // time from the mark source, never journalled. An identity of the form
    // `equity_delta = realized + unrealized - fees` therefore cannot hold for
    // any ledger with an open position and a mark that moved, and the first
    // version of this fix shipped exactly that — passing only because the two
    // anchored fixtures hold no open positions, which is the same accident the
    // tautology it replaced had been living on.
    //
    // Unrealized is still reconciled: identity #1 checks it against the
    // published headline. This row checks the journal against itself from two
    // independent directions — a class walk over `income:%`/`expense:%` on the
    // left, three targeted account queries on the right — so it fails if an
    // income or expense account moves that the report does not attribute. It is
    // `measured` unconditionally, because no term in it depends on a mark.
    let equity = ReconciliationRow::new(
        "equity_delta = realized - fees - other_expense (journal only; excl. unrealized)",
        inputs.equity_delta,
        inputs.realized_period - inputs.fees - inputs.other_expense,
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
            // fees 10 and other_expense 7: both non-zero and DIFFERENT, so the
            // identity pins each term separately and a swap of the two is caught.
            // 100 - 10 - 7 = 83 = equity_delta (bug-log #156, #159).
            fees: dec!(10.00),
            other_expense: dec!(7.00),
            sum_by_strategy: dec!(100.00),
            sum_by_symbol: dec!(100.00),
            equity_delta: dec!(83.00),
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

    // ── Identity #4's FAIL path, one test per term (bug-log #156 / #159) ──────
    //
    // Every other row had a one-cent FAIL-path test; this one had none. That is
    // not incidental to how the tautology survived — the row that could never
    // fail was also the row nothing asserted could fail, so the missing test and
    // the missing computation hid each other. Each of the three ledger-side terms
    // gets its own perturbation, so no term can go missing from the identity
    // again without a named test going red.

    #[test]
    fn t808_one_cent_fee_delta_only_fails_the_equity_row() {
        let mut inp = balanced_inputs();
        inp.fees = dec!(10.01);
        let r = compute(&inp);
        assert!(!r.all_passed());
        assert!(!r.equity.passed, "the fees term must be load-bearing");
        assert_eq!(r.equity.delta, dec!(0.01));
        assert!(r.headline.passed);
        assert!(r.by_strategy.passed);
        assert!(r.by_symbol.passed);
    }

    #[test]
    fn t808_one_cent_other_expense_delta_only_fails_the_equity_row() {
        let mut inp = balanced_inputs();
        inp.other_expense = dec!(7.01);
        let r = compute(&inp);
        assert!(!r.all_passed());
        assert!(
            !r.equity.passed,
            "the other_expense term must be load-bearing — without it a non-fee              expense moves journal equity and the row still passes"
        );
        assert_eq!(r.equity.delta, dec!(0.01));
        assert!(r.headline.passed);
    }

    /// `realized` (inception) and `realized_period` (window) are separate fields.
    /// Before 2026-11-07 one field fed both identity #1 and identity #4, so the
    /// two quantities were conflated with nothing saying so. Perturbing the
    /// PERIOD figure must move identity #4 and leave identity #1 alone.
    #[test]
    fn t808_realized_period_is_wired_only_to_the_equity_row() {
        let mut inp = balanced_inputs();
        inp.realized_period = dec!(100.01);
        let r = compute(&inp);
        assert!(!r.all_passed());
        assert!(!r.equity.passed);
        // delta = report - ledger, and `realized_period` ADDS to the ledger side
        // while `fees`/`other_expense` SUBTRACT from it — so this perturbation
        // signs the opposite way from the two above. Asserting the sign, not just
        // the magnitude, is what distinguishes a term that is added from one that
        // is subtracted; the original defect's label had the fee sign backwards.
        assert_eq!(r.equity.delta, dec!(-0.01));
        assert!(
            r.headline.passed,
            "identity #1 reads `realized` (inception), not `realized_period`"
        );
        assert!(r.by_strategy.passed, "identity #2 reads `realized` too");
        assert!(r.by_symbol.passed);
    }

    /// Identity #4 must NOT consume `unrealized`: nothing in the repo posts to
    /// `income:unrealized_pnl`, so a journal-walked equity delta can never
    /// contain it (bug-log #159). Moving `unrealized` must move identity #1 only.
    #[test]
    fn t808_unrealized_does_not_reach_the_equity_row() {
        let mut inp = balanced_inputs();
        inp.unrealized = dec!(999.00);
        let r = compute(&inp);
        assert!(
            r.equity.passed,
            "identity #4 is journal-only; if an unrealized move breaks it, the \
             row has gone back to asserting a term the ledger never records"
        );
        assert!(
            !r.headline.passed,
            "identity #1 is where unrealized is checked"
        );
    }

    /// A period-end mark miss zeroes `unrealized`, so identity #1 is UNMEASURED
    /// rather than passing over a zero the report itself substituted. Identity #4
    /// reads journal figures only and stays MEASURED (bug-log #156 / #159).
    #[test]
    fn t808_mark_miss_unmeasures_only_the_headline_row() {
        let mut inp = balanced_inputs();
        inp.unrealized_measured = false;
        let r = compute(&inp);
        assert!(!r.headline.measured, "#1 consumes unrealized");
        assert!(r.equity.measured, "#4 is journal-only and stays measured");
        assert!(r.by_strategy.measured);
        assert!(r.by_symbol.measured);
        // An unmeasured row is excluded from all_passed() rather than counted
        // as a pass — a skip is unmeasured, never a pass.
        inp.headline_return = dec!(999.99);
        let r = compute(&inp);
        assert!(
            r.all_passed(),
            "an UNMEASURED row must not fail the report either — it is excluded"
        );
        assert!(!r.headline.measured);
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
