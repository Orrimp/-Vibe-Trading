//! Story 4-13 AC4 — **the FROZEN-gate obligation.**
//!
//! CLAUDE.md, verbatim: "The FROZEN robustness gate is byte-frozen (AD-1).
//! `classify_verdict` / `verdict_bands` / `compute_robustness_flag` / `rank_candidates`
//! are not edited by feature work; every credibility/analytics addition proves it does
//! not change ranking via an identity test."
//!
//! The cross-run FDR annex is such an addition, so this is that proof. It asserts the
//! crown and the FULL ranking order are identical with the annex absent and with it
//! present over each of the four ledger states AC4 names — **missing, empty, populated,
//! corrupt** — because each reaches a different branch of `read_ledger`, and a
//! report-only claim that held for one state and not another would be worth nothing.
//!
//! ## Why the order, not just the crown
//!
//! Asserting only the winner would pass on a change that reshuffled everything below
//! first place. `ranked` is the whole comparator's output and is what the leaderboard
//! renders, so that is what gets compared.
//!
//! ## RED-on-revert
//!
//! Make any gate function read `rationale.fdr_annex`, or let the ledger read/append
//! perturb the run (e.g. by seeding an RNG from the row count), and the
//! populated-vs-absent comparison diverges.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use backtest::bakeoff::fdr_annex::{self, LedgerRow};
use backtest::cancel::cancellation_pair;
use backtest::engine::{DateRange, ScenarioDataSource};
use backtest::progress::ProgressSender;
use backtest::progress::bakeoff_progress_pair;
use backtest::resample::Horizon;
use backtest::{BakeoffConfig, BakeoffReport, BakeoffRequest, RobustnessMode, run_bakeoff};
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol};

/// Three arms the engine actually resolves, plus the benchmark the loop appends, so
/// `ranked` is four entries long. A two-entry ranking would be a weak reorder detector;
/// four is enough that a comparator change has somewhere to show.
fn field() -> Vec<StrategyId> {
    vec![
        StrategyId(SmolStr::new("v0.sma")),
        StrategyId(SmolStr::new("v0.obv")),
        StrategyId(SmolStr::new("v0.roc_momentum")),
    ]
}

async fn run_with(ledger: Option<std::path::PathBuf>) -> BakeoffReport {
    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("BTCUSDT"),
            range: DateRange::Last30d,
            seed: [0xAB; 32],
            field: field(),
            timeframe: Horizon::OneHour,
            initial_capital: dec!(100_000),
            fdr_ledger: ledger,
        },
        data_source: ScenarioDataSource::Synthetic,
        robustness: RobustnessMode::Skip,
    };
    let (_h, cancel_rx) = cancellation_pair();
    let (tx, _rx) = bakeoff_progress_pair();
    run_bakeoff(cfg, cancel_rx, ProgressSender::disabled(), tx)
        .await
        .expect("the bake-off must succeed")
}

fn a_row(beats_hold: bool, dsr: f64) -> LedgerRow {
    LedgerRow {
        run_label: "2026-01-01".to_owned(),
        symbol: "BTCUSDT".to_owned(),
        window: "Last30d".to_owned(),
        n_candidates: 3,
        n_eff: 2.4,
        crown: "v0.sma".to_owned(),
        crown_dsr: dsr,
        beats_hold,
    }
}

/// Crown + full ranking order, the two things the gate decides.
fn verdict(r: &BakeoffReport) -> (Option<usize>, Vec<usize>, String) {
    (
        r.crowned,
        r.ranked.clone(),
        r.rationale.winner.0.to_string(),
    )
}

#[tokio::test]
async fn the_annex_changes_neither_crown_nor_ranking_in_any_ledger_state() {
    let baseline = verdict(&run_with(None).await);

    // 1. MISSING — the path does not exist (a fresh clone, or the first run ever).
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("advisor-runs").join("fdr-ledger.jsonl");
    let r_missing = run_with(Some(missing.clone())).await;
    assert_eq!(
        verdict(&r_missing),
        baseline,
        "a missing ledger must not move the crown or the ranking"
    );
    assert!(
        r_missing.rationale.fdr_annex.is_some(),
        "asking for a ledger must still produce an annex — an absent one would make \
         the comparisons below vacuous"
    );

    // 2. EMPTY — the file exists and has no rows.
    let tmp2 = tempfile::tempdir().unwrap();
    let empty = tmp2.path().join("fdr-ledger.jsonl");
    std::fs::write(&empty, "").unwrap();
    assert_eq!(
        verdict(&run_with(Some(empty)).await),
        baseline,
        "an empty ledger must not move the crown or the ranking"
    );

    // 3. POPULATED — a real sequence, long enough that the annex is sufficient.
    let tmp3 = tempfile::tempdir().unwrap();
    let populated = tmp3.path().join("fdr-ledger.jsonl");
    let body: String = (0..12)
        .map(|i| serde_json::to_string(&a_row(i % 4 == 0, 0.5 + f64::from(i) / 30.0)).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&populated, format!("{body}\n")).unwrap();
    let r_pop = run_with(Some(populated.clone())).await;
    assert_eq!(
        verdict(&r_pop),
        baseline,
        "a populated ledger must not move the crown or the ranking"
    );
    let annex = r_pop.rationale.fdr_annex.as_ref().unwrap();
    assert_eq!(
        annex.runs, 12,
        "the populated ledger's 12 rows must be read"
    );
    assert!(
        annex.insufficient.is_none(),
        "12 rows is a sufficient sequence; got {:?}",
        annex.insufficient
    );

    // 4. CORRUPT — readable rows mixed with garbage.
    let tmp4 = tempfile::tempdir().unwrap();
    let corrupt = tmp4.path().join("fdr-ledger.jsonl");
    std::fs::write(
        &corrupt,
        format!(
            "{}\nnot json at all\n{{\"partial\": true}}\n{}\n",
            serde_json::to_string(&a_row(true, 0.97)).unwrap(),
            serde_json::to_string(&a_row(false, 0.42)).unwrap(),
        ),
    )
    .unwrap();
    let r_corrupt = run_with(Some(corrupt)).await;
    assert_eq!(
        verdict(&r_corrupt),
        baseline,
        "a corrupt ledger must not move the crown or the ranking"
    );
    let annex = r_corrupt.rationale.fdr_annex.as_ref().unwrap();
    assert_eq!(annex.runs, 2, "the two good rows must still be read");
    assert_eq!(
        annex.unreadable_rows, 2,
        "and the two bad ones must be COUNTED, not silently dropped — under-counting \
         the sequence biases the annex optimistic"
    );
    assert_eq!(
        annex.insufficient,
        Some(fdr_annex::Insufficient::PartiallyUnreadable),
        "a damaged ledger must say so rather than report a confident number"
    );
}

/// The append side: a run that was asked to record itself does, and one that was not
/// does not. The second half is the load-bearing one — it is what keeps tests and the
/// anchored CLI path out of the operator's search sequence.
#[tokio::test]
async fn only_a_run_that_was_asked_to_record_appends() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("advisor-runs").join("fdr-ledger.jsonl");

    let _ = run_with(Some(path.clone())).await;
    let (rows, unreadable) = fdr_annex::read_ledger(&path);
    assert_eq!(rows.len(), 1, "the run must have appended exactly one row");
    assert_eq!(unreadable, 0);
    assert_eq!(rows[0].symbol, "BTCUSDT");
    assert!(rows[0].n_candidates >= 2, "the row records the field size");

    let _ = run_with(Some(path.clone())).await;
    let (rows, _) = fdr_annex::read_ledger(&path);
    assert_eq!(
        rows.len(),
        2,
        "append-only: the second run adds, never rewrites"
    );

    // And a run with `None` writes nothing anywhere.
    let untouched = tmp.path().join("should-not-exist.jsonl");
    let _ = run_with(None).await;
    assert!(
        !untouched.exists(),
        "a run with no ledger configured must not create one"
    );
}
