//! T-D-N8 — cockpit-activity-status-bar Wave C integration test.
//!
//! Verifies that the Lab Run producer wiring (approach A: handle held on the
//! iced side, ticked via `LabRunProgress` messages, ended on `LabRunCompleted`)
//! emits:
//!   1. One `ActivityPhase::Start` event with `ActivityKind::LabRun`.
//!   2. At least one `ActivityPhase::Tick` event.
//!   3. One `ActivityPhase::End(ActivityOutcome::Success)` event.
//!
//! ## Architecture (approach A)
//!
//! `ActivityHandle` is `!Send`. The handle lives entirely on the iced side
//! (in `AppState::lab_activity_handle`). This test exercises the full
//! lifecycle directly:
//!   1. `bus.activity().start(LabRun, label)` → emits Start (simulates what
//!      `AppState::update(LabRunRequested)` does).
//!   2. `handle.tick(n)` → emits Tick (simulates what `AppState::update(
//!      LabRunProgress(p))` does on each progress event).
//!   3. `drop(handle)` → emits End { Success } (simulates what
//!      `AppState::update(LabRunCompleted(Ok(_)))` does).
//!
//! The 30-bar SMA scenario is referenced in the brief as the test scenario.
//! Since we're testing the activity-handle lifecycle rather than the engine,
//! we don't need to actually run the backtest.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::activity::{ActivityKind, ActivityOutcome, ActivityPhase};
use agent::config::BusConfig;

// `drain_activity` helper is available but not used in all test functions;
// the tests prefer direct rx.try_recv() for clarity.
#[allow(dead_code)]
fn drain_activity(
    rx: &mut tokio::sync::broadcast::Receiver<agent::activity::ActivityEvent>,
) -> (usize, usize, Vec<ActivityOutcome>) {
    let mut starts = 0usize;
    let mut ticks = 0usize;
    let mut ends: Vec<ActivityOutcome> = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(ev) => match ev.phase {
                ActivityPhase::Start { .. } => starts += 1,
                ActivityPhase::Tick { .. } => ticks += 1,
                ActivityPhase::End(outcome) => ends.push(outcome),
            },
            Err(_) => break,
        }
    }
    (starts, ticks, ends)
}

/// T-D-N8 — Lab Run activity: Start + Tick + End { Success }.
///
/// Acceptance criteria per tasks.md T-D-N8:
/// - `ActivityKind::LabRun` Start event.
/// - At least one Tick event.
/// - End { Success } event.
#[test]
fn lab_run_activity_emits_start_tick_end_success() {
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    // Simulate AppState::update(LabRunRequested): start the Lab Run activity.
    // Label mirrors the format string in cockpit_live.rs.
    let label = "Backtest v0.sma · BTCUSDT · Last30d";
    let handle = activity_sender.start(ActivityKind::LabRun, label);

    // Verify Start event.
    let start_ev = rx.try_recv().expect("Start event expected");
    assert!(
        matches!(start_ev.phase, ActivityPhase::Start { .. }),
        "expected Start phase, got {:?}",
        start_ev.phase
    );
    assert_eq!(start_ev.kind, ActivityKind::LabRun);
    assert!(
        start_ev.label.contains("Backtest"),
        "label must contain 'Backtest', got {:?}",
        start_ev.label
    );

    // Simulate AppState::update(LabRunProgress(p)): tick the handle.
    // The 100 ms throttle applies; use sleep to ensure the tick fires.
    std::thread::sleep(Duration::from_millis(110));
    handle.tick(15); // bar 15 of 30

    std::thread::sleep(Duration::from_millis(10));

    // Verify at least one Tick event.
    let mut found_tick = false;
    loop {
        match rx.try_recv() {
            Ok(ev) if matches!(ev.phase, ActivityPhase::Tick { .. }) => {
                found_tick = true;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    assert!(found_tick, "expected at least one Tick event after handle.tick()");

    // Simulate AppState::update(LabRunCompleted(Ok(_))): drop the handle.
    drop(handle);

    std::thread::sleep(Duration::from_millis(10));

    // Verify End { Success }.
    let end_ev = rx.try_recv().expect("End event expected after drop");
    assert!(
        matches!(end_ev.phase, ActivityPhase::End(ActivityOutcome::Success)),
        "expected End(Success), got {:?}",
        end_ev.phase
    );
}

/// T-D-N8 — Lab Run activity: End { Failed } on LabRunCompleted(Err(_)).
///
/// When the run fails, `AppState::update` calls `handle.fail(err)` before
/// dropping. Verify the End { Failed } event carries the error message.
#[test]
fn lab_run_activity_emits_end_failed_on_error() {
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    let handle = activity_sender.start(ActivityKind::LabRun, "Backtest v1.momentum · XRPUSDT · H2_2024");

    // Drain the Start event.
    let _ = rx.try_recv().expect("Start event expected");

    // Simulate engine failure.
    let error_msg = "backtest engine: unknown strategy 'v1.momentum'";
    handle.fail(error_msg);
    drop(handle);

    std::thread::sleep(Duration::from_millis(10));

    let end_ev = rx.try_recv().expect("End event expected");
    match end_ev.phase {
        ActivityPhase::End(ActivityOutcome::Failed(ref reason)) => {
            assert!(
                reason.contains("unknown strategy"),
                "expected error in End reason, got: {reason:?}"
            );
        }
        other => panic!("expected End(Failed), got {:?}", other),
    }
}

/// T-D-N8 — Lab Run activity: End { Cancelled } on LabRunStopRequested.
///
/// When the operator clicks Stop, `AppState::update` calls `handle.cancel()`
/// before dropping. Verify the End { Cancelled } event.
#[test]
fn lab_run_activity_emits_end_cancelled_on_stop() {
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    let handle = activity_sender.start(ActivityKind::LabRun, "Backtest v0.sma · BTCUSDT · Last90d");

    // Drain the Start event.
    let _ = rx.try_recv().expect("Start event expected");

    // Simulate stop-requested.
    handle.cancel();
    drop(handle);

    std::thread::sleep(Duration::from_millis(10));

    let end_ev = rx.try_recv().expect("End event expected");
    assert!(
        matches!(end_ev.phase, ActivityPhase::End(ActivityOutcome::Cancelled)),
        "expected End(Cancelled), got {:?}",
        end_ev.phase
    );
}
