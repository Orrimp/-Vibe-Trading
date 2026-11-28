//! bug-log `#230` review — one order rule for a Lab `Custom` range, for every data source.
//!
//! **What was wrong.** ADR-0105 first said "Start > end still commits, and the engine refuses it
//! with `InvalidRange`: one rule in one place". That held for Synthetic only. `spawn_lab_run`
//! maps the config (`lab_config_to_scenario`), then runs the Yahoo or Binance preload arm, and
//! only then the engine, whose `InvalidRange` check (`backtest::engine`, `start_ms > end_ms`) is
//! the only order check. A reversed pair on Binance hit the preload's out-of-span test
//! (`end.min(SPAN_END) <= start.max(SPAN_START)`) and was answered "No pinned Binance data …
//! pick a Custom range inside 2023-2024" for a pair INSIDE 2023-2024, and on Yahoo it went into
//! the preload arm (the review REASONED this from the code; MEASURED here under probe P28, below).
//!
//! **The rule now.** `lab_config_to_scenario` refuses a Custom pair whose start is not before its
//! end with `strings::fmt_lab_custom_range_not_increasing`, naming both bounds as typed, and
//! every source's run passes the mapper before any preload. A reversed pair still COMMITS in the
//! editor (each bound parses on its own, ADR-0105 D3); Run refuses it here by name.
//!
//! **What these tests drive.** The REAL `spawn_lab_run` `iced::Task`, drained through
//! `iced_test::runtime::task::into_stream` on a thread with no tokio context (the
//! `lab_runner_cancel_e2e` tests 6-7 pattern, `tests/yahoo_arm_support/mod.rs`), with the
//! harness's mock Yahoo source as the override, which counts its preload calls. Each test is
//! two-sided on one source: the SAME pre-cancelled spec with a reversed pair and with its valid
//! twin (one day long). The reversed pair must complete with the named refusal (and, on Yahoo,
//! with the mock never called); the valid pair must complete with anything else (it passed the
//! mapper: Cancelled, a preload notice, or a run). Pre-cancelling keeps the valid side short and
//! never reads a corpus or the network on purpose; whichever of the arm's cancel and preload
//! branches wins, the outcome is not the order refusal.
//!
//! **Measured** (macOS dev box, 2026-11-28, `--nocapture` prints both sides): on every source
//! the reversed pair completes with `Custom range must start before it ends: '2024-03-01' is not
//! before '2024-02-01'` and 0 mock preload calls. The valid twin reaches each arm: Synthetic the
//! engine (`backtest run cancelled by operator`, the pre-cancel), Yahoo the preload arm
//! (`operator cancelled during preload`; the mock ran once in one run and not at all in another:
//! with the run pre-cancelled, the arm's cancel branch can win before the source is polled, so
//! the valid side's call count is not asserted), Binance the arm's revision check
//! (`REVISION.toml not found at data/binance/REVISION.toml`: the corpus path is relative and a
//! test runs from the crate root). None of the three is the order refusal.
//!
//! Under P28 (the check removed), MEASURED: Synthetic answered the reversed pair with the
//! engine's `invalid date range: Custom range start_ms (1709251200000) > end_ms
//! (1706745600000)`; Yahoo sent it into the preload arm (`operator cancelled during preload`);
//! Binance answered `No pinned Binance data for BTCUSDT in 2024-03-01..2024-02-01 — the pinned
//! corpus covers 2023-01..2024-12. Pick 2024 H1, 2024 H2, or a Custom range inside 2023-2024.`
//! for a pair inside 2023-2024, the review's finding.
//!
//! **Probes** (bug-log `#230`): P28 (the mapper's order check removed) reddens all three; P28b
//! (`>=` → `>`) reddens none of them (their reversed pair is strictly reversed) and reddens the
//! lib's R9 at its equal pair.
//!
//! `live` gate: `spawn_lab_run`'s runtime path and the harness are compiled under `live` (a
//! default ui feature on both CI legs). `scripts/ci_test_target_counts.py` declares this target
//! at 3 on both legs, so a build without `live` (0) is a counted silent zero.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "yahoo_arm_support/mod.rs"]
mod support;

use std::time::Duration;

use smol_str::SmolStr;
use support::{Behavior, Spec};
use ui::lab::runner::LabRunResult;
use ui::lab::state::{DateRange, LabDataSource};
use ui::state::Message;

/// A reversed pair: inside the pinned Binance corpus span (2023-2024), so the pre-fix Binance
/// answer ("pick a Custom range inside 2023-2024") was misleading.
const REVERSED: (&str, &str) = ("2024-03-01", "2024-02-01");
/// Its valid twin: the same start, one day later end.
const VALID: (&str, &str) = ("2024-03-01", "2024-03-02");

fn custom((start, end): (&str, &str)) -> DateRange {
    DateRange::Custom {
        start_raw: SmolStr::new(start),
        end_raw: SmolStr::new(end),
    }
}

/// The named refusal for `pair`, from the one string builder the mapper uses.
fn refusal((start, end): (&str, &str)) -> SmolStr {
    SmolStr::new(ui::strings::fmt_lab_custom_range_not_increasing(start, end))
}

/// What the Task's stream yielded first, and how many times the mock preload ran.
struct Outcome {
    result: LabRunResult,
    mock_calls: usize,
}

/// Run the REAL `spawn_lab_run` Task for a pre-cancelled `v0.sma × BTCUSDT` run on `source`
/// over `pair`, and return its first output.
fn run(source: LabDataSource, pair: (&'static str, &'static str)) -> Outcome {
    use futures::StreamExt as _;
    use iced_test::runtime::Action;

    let mut handle = support::start_with(
        Spec {
            pre_cancelled: true,
            ..Spec::new(50, Behavior::Bars(120))
        },
        move |inputs, returned| {
            let support::ArmInputs {
                handle,
                source: mock,
                mut cfg,
                scenario_cfg: _,
                progress_tx,
                cancel_rx,
                sender,
            } = inputs;
            cfg.data_source = source;
            cfg.range = custom(pair);
            let task = ui::lab::runner::spawn_lab_run(
                Some(&handle),
                cfg,
                cancel_rx,
                progress_tx,
                sender,
                Some(mock),
            );
            let mut stream = iced_test::runtime::task::into_stream(task)
                .expect("HARNESS: spawn_lab_run must return a Task with a stream");
            let first = futures::executor::block_on(stream.next());
            returned.store(true, std::sync::atomic::Ordering::SeqCst);
            match first {
                Some(Action::Output(Message::LabRunCompleted(r))) => r,
                Some(Action::Output(other)) => {
                    panic!("HARNESS: the Task must output LabRunCompleted; got {other:?}")
                }
                Some(_) => panic!("HARNESS: the Task must output a message"),
                None => panic!("HARNESS: the Task's stream ended without an output"),
            }
        },
    );
    let result = handle
        .wait_done(Duration::from_secs(30))
        .expect("HARNESS: the Task must complete within 30 s");
    Outcome {
        result,
        mock_calls: handle.probe.calls(),
    }
}

/// Run both sides on `source` and assert them together.
fn assert_one_order_rule(source: LabDataSource) {
    let reversed = run(source, REVERSED);
    let valid = run(source, VALID);
    let refused_by_name = reversed.result.as_ref().err() == Some(&refusal(REVERSED));
    let valid_passed = valid.result.as_ref().err() != Some(&refusal(VALID))
        && !valid
            .result
            .as_ref()
            .err()
            .is_some_and(|e| e.contains("must start before it ends"));
    let no_preload = reversed.mock_calls == 0;
    let show = |r: &LabRunResult| {
        r.as_ref()
            .err()
            .map_or_else(|| "Ok".to_owned(), ToString::to_string)
    };
    eprintln!(
        "{source:?}: reversed -> {} (mock calls {}); valid -> {} (mock calls {})",
        show(&reversed.result),
        reversed.mock_calls,
        show(&valid.result),
        valid.mock_calls
    );
    assert!(
        refused_by_name && valid_passed && no_preload,
        "{source:?}: [{}] the reversed pair {REVERSED:?} completes with the named refusal (got \
         {:?}); [{}] its valid twin {VALID:?} passes the mapper (got {:?}); [{}] no preload ran \
         for the reversed pair (mock calls {})",
        if refused_by_name { "ok" } else { "RED" },
        reversed.result.as_ref().err(),
        if valid_passed { "ok" } else { "RED" },
        valid.result.as_ref().err(),
        if no_preload { "ok" } else { "RED" },
        reversed.mock_calls
    );
}

/// O1 — Synthetic: refused by the mapper (before, the engine's `InvalidRange`).
#[test]
fn a_reversed_custom_range_is_refused_by_name_on_synthetic() {
    assert_one_order_rule(LabDataSource::Synthetic);
}

/// O2 — Yahoo: refused by the mapper, the preload never called (before, a no-data notice).
#[test]
fn a_reversed_custom_range_is_refused_by_name_before_the_yahoo_preload() {
    assert_one_order_rule(LabDataSource::YahooCache);
}

/// O3 — Binance: refused by the mapper (before, the out-of-span notice for an in-span pair).
#[test]
fn a_reversed_custom_range_is_refused_by_name_before_the_binance_preload() {
    assert_one_order_rule(LabDataSource::BinanceCache);
}
