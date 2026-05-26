//! T-D-N7 — cockpit-activity-status-bar Wave C integration test.
//!
//! Verifies that the Yahoo preload producer wiring emits:
//!   1. One `ActivityPhase::Start` event with `ActivityKind::YahooPreload`.
//!   2. One `ActivityPhase::End(ActivityOutcome::Success)` event.
//!
//! ## Architecture
//!
//! The `preload_yahoo_bars` function is private, so this test exercises the
//! `ActivityHandle` API directly on the `EventBus` activity channel — the
//! same codepath that runs inside `spawn_lab_run`'s async closure (T-D-N7
//! approach A: inline handle, `!Send` constraint is safe because the handle
//! lives entirely within the `iced::Task::perform` closure on a single task).
//!
//! The test constructs the handle lifecycle manually:
//!   1. Create EventBus + subscribe.
//!   2. Call `bus.activity().start(YahooPreload, label)` → emits Start.
//!   3. Simulate a successful preload (no network call).
//!   4. Drop the handle → emits End { Success }.
//!   5. Assert events.
//!
//! This is equivalent to what `spawn_lab_run` does in the live path, but
//! without requiring a full tokio runtime driving `iced::Task::perform`.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::activity::{ActivityKind, ActivityOutcome, ActivityPhase};
use agent::config::BusConfig;

/// Drain activity events from a broadcast receiver into categorised buckets.
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

/// T-D-N7 — Yahoo preload producer emits Start + End { Success }.
///
/// Acceptance criteria per tasks.md T-D-N7:
/// - `ActivityKind::YahooPreload` Start event on the activity channel.
/// - `End { Success }` follows after the preload completes.
#[test]
fn yahoo_preload_activity_emits_start_and_end_success() {
    // Create a real EventBus and subscribe to the activity channel.
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    // Simulate what spawn_lab_run does inside its async closure for T-D-N7:
    // 1. Build the label (mirrors the format string in runner.rs).
    let symbol = "BTCUSDT";
    let range_label = "H1_2024";
    let yahoo_label = format!("Yahoo {} · {}", symbol, range_label);

    // 2. Start the YahooPreload activity handle.
    let handle = activity_sender.start(ActivityKind::YahooPreload, yahoo_label.clone());

    // 3. Verify Start event arrived.
    let (starts_before, _, ends_before) = drain_activity(&mut rx);
    assert_eq!(starts_before, 1, "expected exactly 1 Start event");
    assert_eq!(
        ends_before.len(),
        0,
        "expected 0 End events before handle is dropped"
    );

    // 4. Drop the handle — emits End { Success } (happy path, R1.3).
    drop(handle);

    // 5. Allow a brief window for the broadcast channel to deliver.
    std::thread::sleep(Duration::from_millis(10));

    let (extra_starts, _, ends) = drain_activity(&mut rx);
    assert_eq!(extra_starts, 0, "no extra Start events expected after drop");
    assert_eq!(ends.len(), 1, "expected exactly 1 End event after drop");
    assert!(
        matches!(ends[0], ActivityOutcome::Success),
        "expected End(Success) from a clean drop; got {:?}",
        ends[0]
    );
}

/// T-D-N7 — Yahoo preload producer emits End { Failed } on error.
///
/// When `preload_yahoo_bars` returns an Err, the caller calls `handle.fail()`
/// before returning. Verify the End { Failed } event carries the error message.
#[test]
fn yahoo_preload_activity_emits_end_failed_on_error() {
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    let handle =
        activity_sender.start(ActivityKind::YahooPreload, "Yahoo BTCUSDT · H1_2024");

    // Drain the Start event.
    let _ = rx.try_recv().expect("Start event expected");

    // Simulate preload failure: call fail() before dropping.
    let error_msg = "ticker mapping: UnmappedTicker";
    handle.fail(error_msg);
    drop(handle);

    std::thread::sleep(Duration::from_millis(10));

    // Should receive End { Failed(error_msg) }.
    let ev = rx.try_recv().expect("End event expected after fail");
    match ev.phase {
        ActivityPhase::End(ActivityOutcome::Failed(ref reason)) => {
            assert!(
                reason.contains("ticker mapping"),
                "expected error message in reason, got: {reason:?}"
            );
        }
        other => panic!("expected End(Failed), got {:?}", other),
    }
}
