//! E2E test — Stop during the Yahoo preload exits within ≤ 500 ms and aborts
//! the preload task, with the PRODUCTION arm polled off-runtime.
//!
//! Bug #64 D.1.1 attempt-3 / ADR-0050 § D2 + D3; rewritten by story 2-63
//! (v0.3) Wave F.
//!
//! ## What this test proves
//!
//! D-R2.3: with the `CancellationToken` and the cancel arm of the preload
//! `select!`, Stop (dropping the `RunCancelHandle`) during a 5 s mock preload
//! makes the arm return `Err(cancelled_message("operator cancelled during
//! preload"))` within 500 ms. T-BUG64-RS3: the cancel arm must `abort()` the preload task —
//! dropping its `JoinHandle` would only DETACH it, and the HTTP request would
//! keep running.
//!
//! The subject is `ui::lab::runner::run_yahoo_preload_arm`, the function
//! `spawn_lab_run` calls for every Yahoo source, driven by
//! `tests/yahoo_arm_support/mod.rs` with `futures::executor::block_on` on a
//! bare thread with NO tokio context (how iced polls the Lab Task). The mock
//! source records when it is DROPPED: the spawned preload task owns it, so
//! `dropped && !completed` right after the return is the observable of
//! `abort()`.
//!
//! Until Wave F this file copied the arm's `select!` loop inline under
//! `#[tokio::test]` and asserted on the copy; deleting the cancel arm or the
//! `abort()` from `crates/ui/src/lab/runner.rs` left it green.
//!
//! ## A Stop is a Stop, end to end (bug-log `#215`)
//!
//! Until `#215` the cancelled preload ended its `YahooPreload` activity
//! `Failed("operator cancelled")` and returned an untagged error, so the Lab
//! drew a red run error and "Retry" for the operator's own action (story 2-63
//! Wave F pinned that as current behaviour). Now:
//!
//! - the activity ends `Cancelled` (tests 1 and 3, `assert_cancel_activity`);
//! - the arm's error is `preload_notice::CANCELLED_TAG`-tagged, and `classify`
//!   gives `Cancelled("operator cancelled during preload")` (tests 1-3,
//!   `assert_stop_result`);
//! - fed through the production `ui::state::update` arms, that result sets
//!   `last_run_cancelled` and neither the error nor the notice, the activity
//!   events leave no row on the status bar's tape, and the session log records
//!   the body without the tag (test 4); the negative control, a genuine
//!   preload failure, still lands as a red error and a held tape row (test 5);
//! - an operator Stop the ENGINE sees (here a pre-cancelled Synthetic run,
//!   driven through the real `spawn_lab_run` Task) lands the same way (test
//!   6), and an engine error that is not a Stop stays an error although the
//!   token was cancelled (test 7): the mapping keys on `RunError::Cancelled`,
//!   not on the token.
//!
//! Test 6 is deterministic: the v0.sma dispatch (`backtest::engine`, the
//! `"v0.sma"` arm, ~line 1338) runs `sma_composed_run::run`, which polls the
//! token at bar 0 (`sma_composed_run.rs`, `poll_now` → `is_cancelled()`, ~line
//! 519), pinned by the engine's own `run_scenario_cancellation_returns_cancelled`.
//! Test 7 cannot reach that poll: dispatch returns `UnknownStrategy` first
//! (the `other =>` arm).
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in `crates/ui/src/lab/runner.rs`,
//! located by symbol + quoted text (a one-line OLD has `grep -c` = 1; F-P1's
//! OLD is a 101-line block, unique as a block, verbatim in `PROBES.md`). Run
//! `cargo test -p ui --test lab_runner_cancel_e2e`, restore from a `cp -p`
//! snapshot, `cmp` it, `touch` it, re-run GREEN.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | F-P4 (AC4, designated here) | `run_yahoo_preload_arm` | `() = cancel.cancelled() => {` | `() = std::future::pending::<()>() => {` | `stop_during_preload_exits_within_500ms` at "arm must return within 500 ms of Stop"; the other two tests also red |
//! | F-P5 (AC4, designated here) | `run_yahoo_preload_arm` | `fetch_join.abort();` | delete it | `stop_during_preload_aborts_preload_task` at "preload task must be aborted (source dropped) within 200 ms of the arm returning"; `cancel_before_preload_start_is_instant` also red |
//! | F-P8 (re-designated T215-P2 by bug-log `#215`) | `run_yahoo_preload_arm` | `h.cancel();` | `let _ = &h;` | tests 1 and 3 at "cancel must end the YahooPreload activity Cancelled" (`End(Success)`) |
//! | F-P1 (AC4) | `run_yahoo_preload_arm` | ONE block: from the sentinel `try_send` directly above `let preload_start = std::time::Instant::now();` through `drop(ticker);` | the same block with the sentinel moved after `drop(ticker);` | `cancel_before_preload_start_is_instant` at "the items before the marker must be exactly [sentinel]"; `stop_during_preload_exits_within_500ms` also red at "precondition: the sentinel must arrive first" (reasoned) |
//!
//! Expected under F-P4 (reasoned): rustc warns that `cancel` is unused, and a
//! detached "runtime shutting down" line may print when the test's runtime
//! drops under the still-running arm.
//!
//! ## Bug-log `#215` probes (MEASURED 2026-11-24, one edit each)
//!
//! Each OLD occurs exactly once in its file (P9, P11 and P13 are unique as the
//! quoted block). Each run: `cargo test -p ui --lib -- lab::runner::preload_notice
//! widgets::run_button`, then `cargo test -p ui --test lab_runner_cancel_e2e
//! --test spawn_lab_run_yahoo_harness --test spawn_lab_run_binance_harness
//! --test lab_stop_button_gating --test cockpit_live_lab_run_smoke --test
//! lab_stop_render --test lab_yahoo_empty_range_classification --test
//! lab_runner_ticker_e2e`; the file restored with `cp -p`, `cmp`'d and
//! `touch`ed. Red here is by test number; the full red set is bug-log `#215`'s.
//!
//! | probe | file: OLD → NEW | red here |
//! |---|---|---|
//! | T215-P1 | `runner.rs`: `h.cancel();` → `h.fail("operator cancelled");` (the bug) | 1, 3 (`End(Failed("operator cancelled"))`), 4 (a row left on the tape) |
//! | T215-P2 | `runner.rs`: `h.cancel();` → `let _ = &h;` | 1, 3 (`End(Success)`); 4 stays green by design (a `Success` row is dropped like a `Cancelled` one) |
//! | T215-P3 | `runner.rs`: `return Err(preload_notice::cancelled_message("operator cancelled during preload"));` → `return Err(SmolStr::new("operator cancelled during preload"));` | 1, 2, 3 ("a Stop must return a CANCELLED_TAG-tagged error"), 4 |
//! | T215-P4 | `runner.rs` `classify`: `if let Some(stripped) = raw.strip_prefix(CANCELLED_TAG) {` → `if let Some(stripped) = None::<&str> {` | 1, 2, 3, 4, 6 |
//! | T215-P5 | `runner.rs` `spawn_lab_run`: `Err(backtest::engine::RunError::Cancelled) => {` → `… if false => {` | 6 alone (7 stays green) |
//! | T215-P6 | `state.rs`: `model.lab_state.last_run_cancelled = true;` → `= false;` | 4, 6 |
//! | T215-P10 | `runner.rs` `spawn_lab_run`: `Err(e) => Err(SmolStr::new(format!("{e}"))),` → `Err(e) => Err(preload_notice::cancelled_message(&e.to_string())),` | 7 alone |
//! | T215-P11 | `runner.rs` `run_yahoo_preload_arm`, the classified-error arm's trailing `Err(e)` (after `handle.fail(preload_notice::classify(e.as_str()).msg().as_str());`) → `Err(preload_notice::cancelled_message(&e))` | 5 alone |
//! | T215-P13 | `state.rs` `RunFinished.detail`: `crate::lab::runner::preload_notice::classify(e)` / `.msg()` / `.to_string()` → `e.to_string()` | 4 alone ("the session log must record the Stop's body, without the U+0001 tag") |
//!
//! P7, P8, P9, P12 and P14 have no red here by design (render, `run_button` and
//! gating-test probes). T215-P14 is `run_button.rs` `view`'s
//! `RunState::Cancelled => LAB_RUN_BUTTON_CANCELLED,` →
//! `RunState::Cancelled => LAB_RUN_BUTTON_COMPLETED,`: red only at
//! `lab_stop_render.rs` R1's label clause (it was green under R1's first form).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "yahoo_arm_support/mod.rs"]
mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent::activity::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase};
use backtest::engine::ScenarioDataSource;
use smol_str::SmolStr;
use support::{
    Behavior, Next, STALE_BARS, Spec, becomes_true, is_sentinel, sleep_until, start_arm,
};
use ui::lab::runner::preload_notice::{self, RunMessageKind};
use ui::session_log::{Event, SessionSink};
use ui::state::{Cockpit, Message, update};

/// The production cancel BODY `run_yahoo_preload_arm` returns on a Stop,
/// stated here, not read off the subject (bug-log `#191`).
const STOP_BODY: &str = "operator cancelled during preload";

/// The engine's Stop body: `RunError::Cancelled`'s Display, stated here.
const ENGINE_STOP_BODY: &str = "backtest run cancelled by operator";

/// A genuine (untagged) preload failure, for the negative controls.
const FAILURE: &str = "network error: connection refused";

/// Assert `result` is the arm's Stop: an `Err` tagged with `CANCELLED_TAG`
/// that `classify` turns into `Cancelled(STOP_BODY)` (bug-log `#215`).
fn assert_stop_result(result: &Result<(), SmolStr>) {
    let e = result
        .as_ref()
        .expect_err("a Stop must end the arm in Err, never Ok");
    assert!(
        e.starts_with(preload_notice::CANCELLED_TAG),
        "a Stop must return a CANCELLED_TAG-tagged error; got {e:?}"
    );
    match preload_notice::classify(e) {
        RunMessageKind::Cancelled(body) => assert_eq!(
            body.as_str(),
            STOP_BODY,
            "a Stop must return the production cancel text, classified Cancelled"
        ),
        RunMessageKind::Error(m) => {
            panic!("a Stop must classify as Cancelled, not Error({m:?})")
        }
        RunMessageKind::Notice(m) => {
            panic!("a Stop must classify as Cancelled, not Notice({m:?})")
        }
    }
}

/// Assert `events` is exactly `[Start{YahooPreload}, End(Cancelled)]` (one id).
fn assert_cancel_activity(events: &[ActivityEvent]) {
    assert_eq!(
        events.len(),
        2,
        "cancel must end the YahooPreload activity Cancelled: \
         exactly Start + End expected; got {events:?}"
    );
    assert_eq!(events[0].id, events[1].id, "one activity id: {events:?}");
    assert!(
        events[0].kind == ActivityKind::YahooPreload
            && matches!(events[0].phase, ActivityPhase::Start { .. }),
        "the first event must be the YahooPreload Start; got {events:?}"
    );
    assert!(
        matches!(
            &events[1].phase,
            ActivityPhase::End(ActivityOutcome::Cancelled)
        ),
        "cancel must end the YahooPreload activity Cancelled; got {:?}",
        events[1].phase
    );
}

/// A session-log sink that keeps every event, so a test can read what the
/// production `state::update` arms logged.
#[derive(Debug, Default)]
struct Capture(Mutex<Vec<Event>>);

impl SessionSink for Capture {
    fn record(&self, event: &Event) {
        self.0.lock().unwrap().push(event.clone());
    }
}

impl Capture {
    /// The `detail` of the one `RunFinished` event logged.
    fn run_finished_detail(&self) -> Option<String> {
        let events = self.0.lock().unwrap();
        let finished: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::RunFinished { detail, .. } => Some(detail.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            finished.len(),
            1,
            "HARNESS: exactly one RunFinished must be logged; got {events:?}"
        );
        finished.into_iter().next().flatten()
    }
}

/// A Lab cockpit after one run, through the PRODUCTION `state::update` arms:
/// `LabRunRequested`, each activity event as `ActivityEventReceived`, then
/// `LabRunCompleted(Err(error))`, the message `spawn_lab_run`'s `.await?`
/// posts for an arm error. Returns the cockpit and its session-log capture.
fn lab_after(error: SmolStr, events: &[ActivityEvent]) -> (Cockpit, Arc<Capture>) {
    let capture = Arc::new(Capture::default());
    let mut cockpit = Cockpit::new();
    cockpit.session_log = Some(Arc::clone(&capture) as Arc<dyn SessionSink>);
    update(&mut cockpit, Message::LabRunRequested);
    for ev in events {
        update(&mut cockpit, Message::ActivityEventReceived(ev.clone()));
    }
    update(&mut cockpit, Message::LabRunCompleted(Err(error)));
    (cockpit, capture)
}

// ── Test 1 — Stop during the preload exits ≤ 500 ms ──────────────────────────

/// T-BUG64-D7 / D-R2.3 / ADR-0050 § D2.
///
/// **Pass condition**: with a 5 s mock preload still running at t0+100 ms,
/// Stop makes the arm return the production cancel text within 500 ms, with
/// `scenario_cfg` untouched and the activity ended.
#[test]
fn stop_during_preload_exits_within_500ms() {
    let mut run = start_arm(Spec {
        with_activity: true,
        ..Spec::new(5_000, Behavior::Bars(120))
    });
    match run.next_item(Duration::from_millis(200)) {
        Next::Item(p, _) if is_sentinel(&p) => {}
        other => panic!("precondition: the sentinel must arrive first; got {other:?}"),
    }
    sleep_until(run.t0, Duration::from_millis(100));
    assert!(
        !run.has_returned(),
        "precondition: the arm must still be in its 5 s preload at t0+100 ms"
    );
    assert!(
        !run.probe.completed(),
        "precondition: the 5 s mock preload must not have completed"
    );

    let at = run.cancel_now();
    let done = run
        .wait_done(Duration::from_millis(500))
        .expect("arm must return within 500 ms of Stop");
    assert!(
        done.returned_at.duration_since(at) < Duration::from_millis(500),
        "arm must return within 500 ms of Stop"
    );
    assert_stop_result(&done.result);
    assert_eq!(
        done.scenario_cfg.data_source,
        ScenarioDataSource::Synthetic,
        "a cancelled preload must leave data_source untouched"
    );
    assert_eq!(
        done.scenario_cfg.bars_override.as_ref().map(Vec::len),
        Some(STALE_BARS),
        "a cancelled preload must leave bars_override untouched (the stale seed)"
    );
    assert_cancel_activity(&run.activity());
}

// ── Test 2 — Stop aborts the preload task, not just detaches it ──────────────

/// T-BUG64-RS3: after Stop, the spawned preload task is ABORTED — its source
/// is dropped within 200 ms of the arm's return, without completing. Without
/// `abort()`, dropping the `JoinHandle` detaches the task and the source lives
/// until its 5 s sleep ends.
#[test]
fn stop_during_preload_aborts_preload_task() {
    let mut run = start_arm(Spec::new(5_000, Behavior::Bars(120)));
    run.spin_until(
        Duration::from_millis(500),
        || run.probe.calls() == 1,
        "preload task polled",
    );
    assert!(
        !run.probe.dropped(),
        "precondition: the source must be alive before Stop"
    );

    let _at = run.cancel_now();
    let done = run
        .wait_done(Duration::from_millis(500))
        .expect("arm must return within 500 ms of Stop");
    assert_stop_result(&done.result);
    let dropped_in_time = becomes_true(Duration::from_millis(200), || run.probe.dropped());
    assert!(
        dropped_in_time,
        "preload task must be aborted (source dropped) within 200 ms of the arm returning"
    );
    assert!(
        !run.probe.completed(),
        "an aborted preload must not complete"
    );
}

// ── Test 3 — a cancel before the preload starts is instant ───────────────────

/// A run whose cancel handle is dropped BEFORE the arm starts returns the
/// cancel text within 200 ms of t0: the cancel arm wins the first `select!`
/// iteration, long before the first 250 ms tick, so the only progress item is
/// the sentinel.
#[test]
fn cancel_before_preload_start_is_instant() {
    let mut run = start_arm(Spec {
        pre_cancelled: true,
        with_activity: true,
        ..Spec::new(5_000, Behavior::Bars(120))
    });
    let done = run
        .wait_done(Duration::from_millis(200))
        .expect("a pre-cancelled arm must return within 200 ms");
    assert!(
        done.returned_at.duration_since(run.t0) < Duration::from_millis(200),
        "a pre-cancelled arm must return within 200 ms of t0"
    );
    assert_stop_result(&done.result);

    let items = run.items_until_marker(Duration::from_millis(500));
    assert!(
        items.len() == 1 && is_sentinel(&items[0].0),
        "the items before the marker must be exactly [sentinel]; got {items:?}"
    );
    assert!(
        run.probe.calls() <= 1,
        "the source is preloaded at most once; got {}",
        run.probe.calls()
    );
    assert!(
        becomes_true(Duration::from_millis(200), || run.probe.dropped()),
        "the preload task must be aborted (source dropped) within 200 ms"
    );
    assert_cancel_activity(&run.activity());
}

// ── Test 4 — a stopped preload lands in the Lab as Cancelled ─────────────────

/// Bug-log `#215`: the arm's own Stop output, fed through the production
/// `state::update` arms (no hand-built string), sets `last_run_cancelled` and
/// neither the red error nor the muted notice; its activity events leave no
/// row on the status bar's tape (a `Cancelled` row is dropped at once); and the
/// session log records the Stop's body without the tag.
#[test]
fn a_stopped_preload_lands_in_the_lab_as_cancelled() {
    let mut run = start_arm(Spec {
        pre_cancelled: true,
        with_activity: true,
        ..Spec::new(5_000, Behavior::Bars(120))
    });
    let done = run
        .wait_done(Duration::from_millis(500))
        .expect("a pre-cancelled arm must return within 500 ms");
    let error = done
        .result
        .expect_err("a Stop must end the arm in Err, never Ok");
    let events = run.activity();
    let (cockpit, log) = lab_after(error, &events);

    assert!(
        cockpit.lab_state.last_run_cancelled,
        "a Stop must land as last_run_cancelled (the Lab draws RunState::Cancelled)"
    );
    assert!(
        cockpit.lab_state.last_run_error.is_none(),
        "a Stop must not set the red run error; got {:?}",
        cockpit.lab_state.last_run_error
    );
    assert!(
        cockpit.lab_state.last_run_notice.is_none(),
        "a Stop must not set the muted notice; got {:?}",
        cockpit.lab_state.last_run_notice
    );
    assert!(
        !cockpit.lab_run_inflight,
        "the Stop's completion must clear lab_run_inflight"
    );
    assert!(
        cockpit.activity_tape.visible().is_empty(),
        "a Stop must leave no activity row on the status bar's tape; got {:?}",
        cockpit.activity_tape.visible()
    );
    assert_eq!(
        log.run_finished_detail().as_deref(),
        Some(STOP_BODY),
        "the session log must record the Stop's body, without the U+0001 tag"
    );
}

// ── Test 5 — the negative control: a failed preload stays an error ───────────

/// The same pipeline with a genuine preload failure: the activity ends
/// `Failed(<the error>)`, the Lab gets the red error and no Stop, and the tape
/// holds the failed row red.
#[test]
fn a_failed_preload_still_lands_in_the_lab_as_an_error() {
    let mut run = start_arm(Spec {
        with_activity: true,
        ..Spec::new(10, Behavior::Fail(FAILURE))
    });
    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("the arm must return within 3 s");
    let error = done
        .result
        .expect_err("a failed preload must end the arm in Err");
    let events = run.activity();
    assert!(
        matches!(events.last().map(|e| &e.phase), Some(ActivityPhase::End(ActivityOutcome::Failed(r))) if r == FAILURE),
        "a failed preload must end the activity Failed(<the error>); got {events:?}"
    );
    let (cockpit, log) = lab_after(error, &events);

    assert_eq!(
        cockpit.lab_state.last_run_error.as_deref(),
        Some(FAILURE),
        "a failed preload must land as the red run error"
    );
    assert!(
        !cockpit.lab_state.last_run_cancelled,
        "a failed preload is not a Stop: last_run_cancelled must stay false"
    );
    assert!(
        cockpit.lab_state.last_run_notice.is_none(),
        "a failed preload is not a notice"
    );
    let tape = cockpit.activity_tape.visible();
    assert!(
        tape.len() == 1 && tape[0].red_hold_until.is_some(),
        "a failed preload must leave one red-held row on the tape; got {tape:?}"
    );
    assert_eq!(
        log.run_finished_detail().as_deref(),
        Some(FAILURE),
        "the session log must record the error verbatim"
    );
}

// ── Tests 6-7 — a Stop the engine sees, through the real spawn_lab_run ───────

/// What the Task thread reports once the Task's stream yielded.
struct TaskDone {
    result: ui::lab::runner::LabRunResult,
}

/// Run the REAL `spawn_lab_run` `iced::Task` for a pre-cancelled Synthetic run
/// of `strategy_id`, drained through `iced_test::runtime::task::into_stream` on
/// a thread with no tokio context. The mock Yahoo source is passed as the
/// override and must never be called (the config is Synthetic).
fn start_engine_stop(strategy_id: &'static str) -> support::Run<TaskDone> {
    use futures::StreamExt as _;
    use iced_test::runtime::Action;

    support::start_with(
        Spec {
            pre_cancelled: true,
            ..Spec::new(5_000, Behavior::Bars(120))
        },
        move |inputs, returned| {
            let support::ArmInputs {
                handle,
                source,
                mut cfg,
                scenario_cfg: _,
                progress_tx,
                cancel_rx,
                sender,
            } = inputs;
            cfg.data_source = ui::lab::state::LabDataSource::Synthetic;
            cfg.strategy_id = SmolStr::new(strategy_id);
            let task = ui::lab::runner::spawn_lab_run(
                Some(&handle),
                cfg,
                cancel_rx,
                progress_tx,
                sender,
                Some(source),
            );
            let mut stream = iced_test::runtime::task::into_stream(task)
                .expect("HARNESS: spawn_lab_run must return a Task with a stream");
            let first = futures::executor::block_on(async {
                assert!(
                    tokio::runtime::Handle::try_current().is_err(),
                    "HARNESS: the Task must be polled with no tokio context"
                );
                stream.next().await
            });
            returned.store(true, std::sync::atomic::Ordering::SeqCst);
            let result = match first {
                Some(Action::Output(Message::LabRunCompleted(r))) => r,
                Some(Action::Output(other)) => {
                    panic!("HARNESS: the Task must output LabRunCompleted; got {other:?}")
                }
                Some(other) => panic!("HARNESS: the Task must output a message; got {other:?}"),
                None => panic!("HARNESS: the Task's stream ended without an output"),
            };
            TaskDone { result }
        },
    )
}

/// Bug-log `#215`: a Stop the engine sees (a pre-cancelled Synthetic v0.sma
/// run; the engine polls the token at bar 0) completes the Task with
/// `RunError::Cancelled`'s body tagged as a Stop, and lands in the Lab as
/// Cancelled. No preload runs.
#[test]
fn a_stop_during_the_engine_lands_in_the_lab_as_cancelled() {
    let mut run = start_engine_stop("v0.sma");
    let done = run
        .wait_done(Duration::from_secs(30))
        .expect("the pre-cancelled Task must complete within 30 s");
    let error = match done.result {
        Err(e) => e,
        Ok(summary) => panic!("a pre-cancelled run must not complete Ok; got {summary:?}"),
    };
    assert!(
        matches!(preload_notice::classify(&error), RunMessageKind::Cancelled(body) if body == ENGINE_STOP_BODY),
        "an engine Stop must complete the Task with RunError::Cancelled's body, classified \
         Cancelled; got {error:?}"
    );
    let (cockpit, _log) = lab_after(error, &[]);
    assert!(
        cockpit.lab_state.last_run_cancelled,
        "an engine Stop must land as last_run_cancelled"
    );
    assert!(
        cockpit.lab_state.last_run_error.is_none() && cockpit.lab_state.last_run_notice.is_none(),
        "an engine Stop must set neither the error nor the notice; got {:?} / {:?}",
        cockpit.lab_state.last_run_error,
        cockpit.lab_state.last_run_notice
    );
    assert_eq!(
        run.probe.calls(),
        0,
        "a Synthetic run must not call the Yahoo source"
    );
}

/// The negative control for test 6: the same pre-cancelled token, but an
/// unknown strategy id, so the engine returns `UnknownStrategy` before any bar
/// poll. That is an error, not a Stop: the mapping keys on
/// `RunError::Cancelled`, not on whether the token was cancelled.
#[test]
fn an_engine_error_that_is_not_a_stop_stays_an_error() {
    let mut run = start_engine_stop("v9.not-a-strategy");
    let done = run
        .wait_done(Duration::from_secs(30))
        .expect("the Task must complete within 30 s");
    let error = match done.result {
        Err(e) => e,
        Ok(summary) => panic!("an unknown strategy must not complete Ok; got {summary:?}"),
    };
    assert_eq!(
        error.as_str(),
        "unknown strategy: v9.not-a-strategy",
        "an unknown strategy must complete the Task with the engine's verbatim, untagged error"
    );
    let (cockpit, _log) = lab_after(error, &[]);
    assert!(
        !cockpit.lab_state.last_run_cancelled,
        "an engine error is not a Stop, even with the token cancelled"
    );
    assert_eq!(
        cockpit.lab_state.last_run_error.as_deref(),
        Some("unknown strategy: v9.not-a-strategy"),
        "an engine error must land as the red run error"
    );
}
