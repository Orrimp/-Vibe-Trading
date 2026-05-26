//! T-D-N9 — cockpit-activity-status-bar Wave C integration test.
//!
//! Verifies that the Training subprocess producer wiring emits:
//!   1. One `ActivityPhase::Start` event with `ActivityKind::Training`.
//!   2. One `ActivityPhase::End(ActivityOutcome::Success)` event.
//!
//! ## Architecture (approach A)
//!
//! `ActivityHandle` is `!Send`. For T-D-N9, `spawn_training_run` accepts an
//! `ActivitySender` and returns `(TrainingHandle, Option<ActivityHandle>)`.
//! The caller (iced side) holds the `ActivityHandle` alongside `TrainingHandle`
//! and ends the activity when the subprocess exits.
//!
//! This test:
//!   1. Creates EventBus + subscribes.
//!   2. Calls `spawn_training_run(sleep 1)` with an activity_sender.
//!   3. Gets back `(TrainingHandle, ActivityHandle)`.
//!   4. Waits for the subprocess to exit (~1 second).
//!   5. Drops both handles → Activity End { Success } fires.
//!   6. Asserts Start + End(Success) events arrived.
//!
//! Ticks are optional in the test (1 Hz poll may not fire within 1 s).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::activity::{ActivityOutcome, ActivityPhase};
use agent::config::BusConfig;
use ui::lab::trainer::TrainingLogLine;
use ui::lab::trainer::{TrainingConfig, cancellation_pair, spawn_training_run};

/// Drain activity events from a broadcast receiver.
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

/// T-D-N9 — Training subprocess: Start + End { Success }.
///
/// Spawns `sleep 1` (available on all Unix test hosts) via `spawn_training_run`
/// with a real `ActivitySender`. Waits for the subprocess to exit, then drops
/// the returned `ActivityHandle` → emits End { Success }.
///
/// Acceptance criteria per tasks.md T-D-N9:
/// - `ActivityKind::Training` Start event.
/// - `End { Success }` event (ticks optional — 1 Hz poll may not fire in 1s).
#[test]
#[cfg(unix)]
fn training_run_activity_emits_start_and_end_success() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create EventBus + subscribe to activity channel.
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    // Config for `sleep 1` — minimal, dry-run not required.
    let cfg = TrainingConfig {
        binary_path: PathBuf::from("sleep"),
        config_path: PathBuf::from("/dev/null"),
        output_dir: PathBuf::from("/tmp"),
        dry_run: false,
        epochs: None,
        scenario: None,
        audit_db: None,
    };

    let (_cancel_handle, cancel_rx) = cancellation_pair();
    let (line_tx, _line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

    // Spawn `sleep 1` with an ActivitySender. Returns (TrainingHandle, ActivityHandle).
    let result = spawn_training_run(
        Some(rt.handle()),
        &cfg,
        cancel_rx,
        line_tx,
        Some(activity_sender),
    );

    let (training_handle, activity_handle) = match result {
        Ok(pair) => pair,
        Err(e) => {
            // `sleep` must be available on Unix test hosts.
            panic!("spawn_training_run failed: {e}");
        }
    };

    // Verify Start event was emitted immediately on spawn.
    std::thread::sleep(Duration::from_millis(20));
    let (starts, _, ends_before) = drain_activity(&mut rx);
    assert_eq!(starts, 1, "expected exactly 1 Start event after spawn");
    assert_eq!(
        ends_before.len(),
        0,
        "expected 0 End events before subprocess exits"
    );

    // Wait for `sleep 1` to finish (~1 second).
    std::thread::sleep(Duration::from_millis(1200));

    // Drop the training handle first (SIGKILL — but sleep has already exited).
    drop(training_handle);

    // Drop the activity handle → emits End { Success } (approach A, iced side).
    drop(activity_handle);

    std::thread::sleep(Duration::from_millis(20));

    // Assert End { Success } arrived.
    let (extra_starts, _ticks, ends) = drain_activity(&mut rx);
    assert_eq!(extra_starts, 0, "no extra Start events expected");
    assert_eq!(ends.len(), 1, "expected exactly 1 End event after drop");
    assert!(
        matches!(ends[0], ActivityOutcome::Success),
        "expected End(Success), got {:?}",
        ends[0]
    );
}

/// T-D-N9 — Training subprocess: End { Cancelled } on cancel.
///
/// When the operator cancels training, `AppState::update(TrainingCancelPressed)`
/// calls `handle.cancel()` then drops the training_activity_handle.
#[test]
#[cfg(unix)]
fn training_run_activity_emits_end_cancelled_on_stop() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    let cfg = TrainingConfig {
        binary_path: PathBuf::from("sleep"),
        config_path: PathBuf::from("/dev/null"),
        output_dir: PathBuf::from("/tmp"),
        dry_run: false,
        epochs: None,
        scenario: None,
        audit_db: None,
    };

    let (_cancel_handle, cancel_rx) = cancellation_pair();
    let (line_tx, _line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

    let (training_handle, activity_handle) = spawn_training_run(
        Some(rt.handle()),
        &cfg,
        cancel_rx,
        line_tx,
        Some(activity_sender),
    )
    .expect("spawn must succeed with sleep binary");

    // Drain the Start event.
    std::thread::sleep(Duration::from_millis(20));
    let _ = rx.try_recv().expect("Start event expected");

    // Simulate cancel: call cancel() on the activity handle, then drop both.
    if let Some(handle) = activity_handle {
        handle.cancel();
        drop(handle);
    }
    drop(training_handle);

    std::thread::sleep(Duration::from_millis(20));

    let end_ev = rx.try_recv().expect("End event expected");
    assert!(
        matches!(end_ev.phase, ActivityPhase::End(ActivityOutcome::Cancelled)),
        "expected End(Cancelled), got {:?}",
        end_ev.phase
    );
}
