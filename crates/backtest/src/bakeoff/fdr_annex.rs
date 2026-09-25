//! Cross-run multiple-testing annex — story 4-13, gap-analysis B7.
//!
//! ## The gap this closes
//!
//! The per-run scorecard (ADR-0075) deflates the crown's Sharpe by how many arms were
//! tried **in that run**. It cannot see the other axis: an operator who re-runs the
//! bake-off on a new coin, a new window, or simply a later date is running a *sequence*
//! of tests, and the chance that at least one of them says "beats holding" grows with
//! the length of the sequence even when nothing ever beat holding.
//!
//! `research/backtesting/papers.md` [73] (Ramdas et al. 2017) names the tool:
//! online-FDR / alpha-investing over the run sequence, with decaying memory so a 2021
//! discovery does not spend 2026's error budget. The 2026-07-11 gap analysis lists this
//! as **the single build-candidate** in the corpus — everything else there is
//! stated-limit or leave-alone.
//!
//! ## What SHIPPED here, and what did not (AC2 asks this to be stated plainly)
//!
//! **Shipped — the static family-wise version.** Two numbers over the recorded sequence:
//!
//! - the **expected** number of false "beats holding" verdicts, `N × α`, which is what
//!   B7 names as the minimum report-annex line;
//! - the **Šidák** per-run bar, `DSR ≥ (1−α)^(1/N)`, i.e. how high a single run's
//!   deflated Sharpe would have to be for the whole sequence to hold family-wise error
//!   at α — and how many recorded crowns actually clear it.
//!
//! **Not shipped — LORD alpha-investing with decaying memory.** It is the better fit for
//! crypto non-stationarity, and it needs wealth accounting across the sequence (a long
//! nothing-beats-hold streak *tightens* the budget, which is the correct self-skeptical
//! response). That is a real piece of work and the AC explicitly permits the cheap
//! version for v0.1 provided the choice is stated. This is the statement.
//!
//! The difference matters in one direction worth knowing: Šidák assumes independence and
//! treats every run as equally current, so it is **conservative about old runs** and
//! **optimistic about correlated ones** (re-running the same coin on an overlapping
//! window is not a fresh test). Neither error is hidden by the numbers below; both are
//! named in the rendered line's caption.
//!
//! ## REPORT-ONLY
//!
//! Nothing here is read by `rank_candidates`, `classify_verdict`, `verdict_bands` or
//! `compute_robustness_flag`. The FROZEN gate is byte-untouched and
//! `crates/backtest/tests/fdr_annex_identity.rs` proves crowning and ranking are
//! identical with the annex present, absent, empty and corrupt.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The per-run significance level the crown is already judged at.
///
/// Not a new knob: `scorecard::DSR_THRESHOLD` is 0.95, so the crown's implied per-run
/// level is 0.05 and the cross-run annex must use the same one or it would be describing
/// a different test than the one that ran.
pub const ALPHA: f64 = 0.05;

/// Below this many recorded runs the annex reports insufficiency rather than a number.
///
/// A statement about a *sequence* needs a sequence. At N = 1 the Šidák bar collapses to
/// the per-run bar and the expected-false-positive count is a restatement of α, so the
/// annex would be dressing up something the scorecard already says.
pub const MIN_RUNS: usize = 2;

/// One completed bake-off, as recorded in the cross-run ledger.
///
/// Deliberately carries no money and no equity curve: this row exists to count tests,
/// not to re-describe a run. (So AD-9 is not implicated — there is no `Decimal` here to
/// get wrong.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerRow {
    /// Run date label, e.g. `"2026-09-25"`.
    pub run_label: String,
    /// The coin the bake-off ranked.
    pub symbol: String,
    /// Human-readable lookback label.
    pub window: String,
    /// Arms ranked, including the benchmark.
    pub n_candidates: usize,
    /// The run's effective (correlation-adjusted) trial count.
    pub n_eff: f64,
    /// The crowned strategy id.
    pub crown: String,
    /// The crown's deflated Sharpe — `P(true Sharpe > 0)` after per-run deflation.
    pub crown_dsr: f64,
    /// Did this run conclude that an active strategy beats holding?
    pub beats_hold: bool,
}

/// Why the annex has nothing to say yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Insufficient {
    /// No ledger on disk — the first run, or a fresh clone.
    NoHistory,
    /// Fewer than [`MIN_RUNS`] readable rows.
    TooFewRuns,
    /// The ledger exists and some rows did not parse. The count of GOOD rows is
    /// reported alongside, because "3 of 40 rows are unreadable" and "3 runs happened"
    /// are very different states and the operator should not have to guess which.
    PartiallyUnreadable,
}

/// The report-only cross-run annex.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FdrAnnex {
    /// Readable rows found.
    pub runs: usize,
    /// Rows that could not be parsed — `0` in the healthy case.
    ///
    /// Reported rather than swallowed: a ledger that silently drops half its history
    /// would understate the sequence length, which biases the annex *optimistic*, which
    /// is the one direction an honesty surface must never fail in.
    pub unreadable_rows: usize,
    /// Of the readable rows, how many concluded an active strategy beats holding.
    pub beats_hold: usize,
    /// The per-run level, echoed so the rendered line can state it.
    pub alpha: f64,
    /// `runs × α` — the number of "beats holding" verdicts chance alone would produce
    /// over this sequence if no strategy ever truly beat holding.
    pub expected_false_beats_hold: f64,
    /// `(1−α)^(1/runs)` — the deflated Sharpe a single run must reach for the SEQUENCE
    /// to hold family-wise error at α. Always ≥ the per-run 0.95 bar, and it rises as
    /// the sequence grows.
    pub sidak_dsr_bar: f64,
    /// How many recorded crowns clear [`Self::sidak_dsr_bar`].
    pub crowns_clearing_sidak: usize,
    /// `Some(..)` when the annex cannot make its statement; the numeric fields are then
    /// whatever the partial history supports and must not be rendered as a conclusion.
    pub insufficient: Option<Insufficient>,
}

impl FdrAnnex {
    /// Is the sequence's "beats holding" count within what chance alone predicts?
    ///
    /// The honest headline. `None` when the annex is insufficient — deliberately not
    /// `false`, because "we cannot say" and "no, it is not" are different answers and
    /// conflating them is how an honesty surface starts lying.
    #[must_use]
    pub fn beats_hold_within_chance(&self) -> Option<bool> {
        if self.insufficient.is_some() {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        Some((self.beats_hold as f64) <= self.expected_false_beats_hold)
    }
}

/// Compute the annex from a run sequence.
///
/// Pure and total: no I/O, no panic, no clock. `unreadable` is passed in by the reader
/// because only it can know how many rows it failed to parse.
#[must_use]
pub fn compute_annex(rows: &[LedgerRow], unreadable: usize) -> FdrAnnex {
    let runs = rows.len();
    let beats_hold = rows.iter().filter(|r| r.beats_hold).count();

    #[allow(clippy::cast_precision_loss)]
    let n = runs as f64;
    let expected_false_beats_hold = n * ALPHA;
    // (1−α)^(1/N): the per-run confidence each of N independent tests needs for the
    // family to hold at α. N = 0 has no bar; report the per-run bar so the field is
    // never NaN, and the `insufficient` flag is what stops it being read.
    let sidak_dsr_bar = if runs == 0 {
        1.0 - ALPHA
    } else {
        (1.0 - ALPHA).powf(1.0 / n)
    };
    let crowns_clearing_sidak = rows.iter().filter(|r| r.crown_dsr >= sidak_dsr_bar).count();

    let insufficient = if runs == 0 {
        Some(Insufficient::NoHistory)
    } else if unreadable > 0 {
        Some(Insufficient::PartiallyUnreadable)
    } else if runs < MIN_RUNS {
        Some(Insufficient::TooFewRuns)
    } else {
        None
    };

    FdrAnnex {
        runs,
        unreadable_rows: unreadable,
        beats_hold,
        alpha: ALPHA,
        expected_false_beats_hold,
        sidak_dsr_bar,
        crowns_clearing_sidak,
        insufficient,
    }
}

/// The ledger's path: `advisor-runs/fdr-ledger.jsonl` under `root`.
///
/// Git-ignored, and therefore outside every `evidence/**` anchor glob **by
/// construction** — the ADR-0055 `/lab-runs/` argument. The 119 anchored bodies cannot
/// be perturbed by anything written here, which is why this location was chosen over a
/// path under `evidence/`.
#[must_use]
pub fn ledger_path(root: &Path) -> PathBuf {
    root.join("advisor-runs").join("fdr-ledger.jsonl")
}

/// Read the ledger, tolerating damage.
///
/// Returns `(rows, unreadable_count)`. A missing file is `(vec![], 0)` — the first run is
/// not an error. A malformed line is counted, not fatal: a ledger that refused to load
/// because one row was truncated would take the whole annex down for the least important
/// reason available.
#[must_use]
pub fn read_ledger(path: &Path) -> (Vec<LedgerRow>, usize) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (Vec::new(), 0);
    };
    let mut rows = Vec::new();
    let mut unreadable = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<LedgerRow>(line) {
            Ok(r) => rows.push(r),
            Err(_) => unreadable += 1,
        }
    }
    (rows, unreadable)
}

/// Append one run to the ledger. Best-effort by contract (AC1).
///
/// A failure warns and returns; it never fails the bake-off. The run's result is the
/// thing the operator asked for, and losing a bookkeeping row is not a reason to throw
/// it away.
pub fn append_row(path: &Path, row: &LedgerRow) {
    use std::io::Write;
    let Ok(line) = serde_json::to_string(row) else {
        tracing::warn!(target: "bakeoff.fdr_annex", "could not serialise the ledger row");
        return;
    };
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(target: "bakeoff.fdr_annex", error = %e, "could not create the ledger directory");
        return;
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{line}") {
                tracing::warn!(target: "bakeoff.fdr_annex", error = %e, "could not append the ledger row");
            }
        }
        Err(e) => {
            tracing::warn!(target: "bakeoff.fdr_annex", error = %e, "could not open the ledger");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn row(beats_hold: bool, dsr: f64) -> LedgerRow {
        LedgerRow {
            run_label: "2026-01-01".to_owned(),
            symbol: "BTCUSDT".to_owned(),
            window: "Last90d".to_owned(),
            n_candidates: 5,
            n_eff: 3.2,
            crown: "v0.sma".to_owned(),
            crown_dsr: dsr,
            beats_hold,
        }
    }

    /// AC4's arithmetic fixture — the expected-false-positive count on a KNOWN sequence,
    /// checked against a hand-computed value rather than against the implementation.
    #[test]
    fn expected_false_positives_is_n_times_alpha() {
        let rows: Vec<LedgerRow> = (0..20).map(|_| row(false, 0.5)).collect();
        let a = compute_annex(&rows, 0);
        assert_eq!(a.runs, 20);
        // 20 runs at α = 0.05 → chance alone produces 1.0 "beats holding" verdicts.
        assert!(
            (a.expected_false_beats_hold - 1.0).abs() < 1e-12,
            "20 × 0.05 = 1.0, got {}",
            a.expected_false_beats_hold
        );
    }

    /// The Šidák bar RISES with the sequence — the property that makes it worth
    /// reporting at all. A bar that did not move with N would be the per-run bar
    /// wearing a different name.
    #[test]
    fn the_sidak_bar_tightens_as_the_sequence_grows() {
        let short: Vec<LedgerRow> = (0..2).map(|_| row(false, 0.5)).collect();
        let long: Vec<LedgerRow> = (0..50).map(|_| row(false, 0.5)).collect();
        let (a, b) = (compute_annex(&short, 0), compute_annex(&long, 0));
        assert!(
            b.sidak_dsr_bar > a.sidak_dsr_bar,
            "50 runs must demand a higher per-run confidence than 2: {} vs {}",
            b.sidak_dsr_bar,
            a.sidak_dsr_bar
        );
        // And it is always at least the per-run bar it generalises.
        assert!(a.sidak_dsr_bar >= 1.0 - ALPHA - 1e-12);
        // Hand-checked: (1-0.05)^(1/2) = 0.974679…
        assert!((a.sidak_dsr_bar - 0.974_679_434_480_896_1).abs() < 1e-9);
    }

    /// The honest headline, and the reason it is an `Option`.
    #[test]
    fn within_chance_distinguishes_cannot_say_from_no() {
        // 20 runs, 1 beats-hold, expected 1.0 → within chance.
        let mut rows: Vec<LedgerRow> = (0..19).map(|_| row(false, 0.5)).collect();
        rows.push(row(true, 0.99));
        assert_eq!(
            compute_annex(&rows, 0).beats_hold_within_chance(),
            Some(true)
        );

        // 20 runs, 5 beats-hold, expected 1.0 → NOT within chance.
        let mut rows: Vec<LedgerRow> = (0..15).map(|_| row(false, 0.5)).collect();
        rows.extend((0..5).map(|_| row(true, 0.99)));
        assert_eq!(
            compute_annex(&rows, 0).beats_hold_within_chance(),
            Some(false)
        );

        // Insufficient → `None`, NOT `false`. "We cannot say" and "no" are different
        // answers and an honesty surface must not conflate them.
        assert_eq!(compute_annex(&[], 0).beats_hold_within_chance(), None);
        assert_eq!(
            compute_annex(&[row(true, 0.99)], 0).beats_hold_within_chance(),
            None
        );
    }

    /// AC5 — every degradation state reports itself rather than erroring or guessing.
    #[test]
    fn degradation_states_are_named_not_swallowed() {
        assert_eq!(
            compute_annex(&[], 0).insufficient,
            Some(Insufficient::NoHistory)
        );
        assert_eq!(
            compute_annex(&[row(false, 0.5)], 0).insufficient,
            Some(Insufficient::TooFewRuns)
        );
        // Damage wins over sufficiency: 40 good rows with 3 unreadable is still a
        // ledger whose length we do not know, and under-counting the sequence biases
        // the annex OPTIMISTIC — the one direction it must never fail in.
        let rows: Vec<LedgerRow> = (0..40).map(|_| row(false, 0.5)).collect();
        let a = compute_annex(&rows, 3);
        assert_eq!(a.insufficient, Some(Insufficient::PartiallyUnreadable));
        assert_eq!(a.unreadable_rows, 3);
        assert_eq!(a.runs, 40, "the good rows are still counted and reported");

        assert_eq!(compute_annex(&rows, 0).insufficient, None);
    }

    /// The reader tolerates damage and counts it; the writer appends without rewriting.
    #[test]
    fn the_ledger_round_trips_and_survives_garbage() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("advisor-runs").join("fdr-ledger.jsonl");

        // A missing file is not an error — the first run is normal.
        assert_eq!(read_ledger(&path), (Vec::new(), 0));

        append_row(&path, &row(true, 0.97));
        append_row(&path, &row(false, 0.31));
        let (rows, bad) = read_ledger(&path);
        assert_eq!(rows.len(), 2);
        assert_eq!(bad, 0);
        assert!(rows[0].beats_hold && !rows[1].beats_hold, "order preserved");

        // Garbage between good rows is counted, and the good rows still load.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            writeln!(f, "not json").unwrap();
            writeln!(f, "{{\"half\": \"a row\"}}").unwrap();
        }
        append_row(&path, &row(true, 0.5));
        let (rows, bad) = read_ledger(&path);
        assert_eq!(
            rows.len(),
            3,
            "the good rows survive the garbage between them"
        );
        assert_eq!(bad, 2, "and the garbage is counted, not silently dropped");
    }
}
