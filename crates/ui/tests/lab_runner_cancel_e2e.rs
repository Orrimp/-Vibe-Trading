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
//! makes the arm return `Err("operator cancelled during preload")` within
//! 500 ms. T-BUG64-RS3: the cancel arm must `abort()` the preload task —
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
//! The cancelled preload ends its `YahooPreload` activity
//! `Failed("operator cancelled")`, not `Cancelled`: pinned here AS CURRENT
//! BEHAVIOUR (a bug-log candidate recorded by story 2-63 Wave F, not a design
//! claim).
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
//! | F-P8 | `run_yahoo_preload_arm` | `h.fail("operator cancelled");` | `let _ = &h;` | `stop_during_preload_exits_within_500ms` at "cancel must end the YahooPreload activity Failed(\"operator cancelled\")" |
//! | F-P1 (AC4) | `run_yahoo_preload_arm` | ONE block: from the sentinel `try_send` directly above `let preload_start = std::time::Instant::now();` through `drop(ticker);` | the same block with the sentinel moved after `drop(ticker);` | `cancel_before_preload_start_is_instant` at "the items before the marker must be exactly [sentinel]"; `stop_during_preload_exits_within_500ms` also red at "precondition: the sentinel must arrive first" (reasoned) |
//!
//! Expected under F-P4 (reasoned): rustc warns that `cancel` is unused, and a
//! detached "runtime shutting down" line may print when the test's runtime
//! drops under the still-running arm.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "yahoo_arm_support/mod.rs"]
mod support;

use std::time::Duration;

use agent::activity::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase};
use backtest::engine::ScenarioDataSource;
use smol_str::SmolStr;
use support::{
    Behavior, Next, STALE_BARS, Spec, becomes_true, is_sentinel, sleep_until, start_arm,
};

/// The production cancel text `run_yahoo_preload_arm` returns.
const CANCELLED: &str = "operator cancelled during preload";

/// Assert `events` is exactly `[Start{YahooPreload}, End(Failed("operator
/// cancelled"))]` (one id).
fn assert_cancel_activity(events: &[ActivityEvent]) {
    assert_eq!(
        events.len(),
        2,
        "cancel must end the YahooPreload activity Failed(\"operator cancelled\"): \
         exactly Start + End expected; got {events:?}"
    );
    assert_eq!(events[0].id, events[1].id, "one activity id: {events:?}");
    assert!(
        events[0].kind == ActivityKind::YahooPreload
            && matches!(events[0].phase, ActivityPhase::Start { .. }),
        "the first event must be the YahooPreload Start; got {events:?}"
    );
    assert!(
        matches!(&events[1].phase, ActivityPhase::End(ActivityOutcome::Failed(r)) if r == "operator cancelled"),
        "cancel must end the YahooPreload activity Failed(\"operator cancelled\"); got {:?}",
        events[1].phase
    );
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
    assert_eq!(
        done.result,
        Err(SmolStr::new(CANCELLED)),
        "Stop during the preload must return the production cancel text"
    );
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
    assert_eq!(
        done.result,
        Err(SmolStr::new(CANCELLED)),
        "Stop must return the production cancel text"
    );
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
    assert_eq!(
        done.result,
        Err(SmolStr::new(CANCELLED)),
        "a pre-cancelled run must return the production cancel text"
    );

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
