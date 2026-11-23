//! T-D-N9 — cockpit-activity-status-bar Wave C integration test; ported off
//! `cfg(unix)` by story 2-63 (v0.3) Wave E3.
//!
//! Verifies that the Training subprocess producer wiring emits:
//!   1. One `ActivityPhase::Start` event with `ActivityKind::Training` and the
//!      label `Train <binary-name> · running`.
//!   2. One `ActivityPhase::End(..)` event when the returned handle drops.
//!
//! ## Architecture (approach A) — what is production and what is not
//!
//! `spawn_training_run` accepts an `ActivitySender` and returns
//! `(TrainingHandle, Option<ActivityHandle>)`. `ActivityHandle` is `Send` but
//! not `Sync` (it holds `Cell`s of `Send` types); the caller (iced side) holds
//! it alongside `TrainingHandle` and ends the activity when the subprocess
//! exits or is cancelled.
//!
//! So, honestly:
//! - the `Start` event is the PRODUCTION emit (`spawn_training_run`'s
//!   `s.start(ActivityKind::Training, training_label)`), and this file pins
//!   its kind and label;
//! - the `End(Success)` / `End(Cancelled)` events are emitted by THIS test's
//!   own `drop` / `cancel()` through agent's `ActivityHandle` API. The bin's
//!   producer that does this on `TrainingExited` / `TrainingCancelPressed`
//!   lives in `AppState` (`crates/ui/src/bin/cockpit_live.rs`), which a test
//!   cannot build: that half stays unpinned (the story 2-63 K3 #6 residual).
//!
//! ## The stub child is this test binary (story 2-63 Wave E2 pattern)
//!
//! [`stub_training_config`] points `binary_path` at `std::env::current_exe()`,
//! this very test binary, so the spawn needs no `sleep` (Unix-only) and behaves
//! the same on macOS and Windows. The child stays inert because its argv
//! carries options libtest does not know (`spawn_training_run` passes
//! `"--config".into(),` and `"--output-dir".into(),` unconditionally): libtest
//! rejects an unknown option before it runs any test, and exits 101.
//! [`stub_child_is_rejected_by_libtest`] (copied verbatim from
//! `training_log_state_gating.rs`) re-measures that per binary, on every run.
//! The child exits at once; no test here observes it, so there is no wait for
//! it (until Wave E3 a 1200 ms sleep "waited for `sleep 1` to finish", which
//! had in fact already exited on its unknown `--config` argument).
//!
//! The residual (unguarded, as in `training_log_state_gating.rs`): an argv
//! with NO option at all would run this whole binary in the child, a process
//! fan-out. Nothing guards that production change.
//!
//! ## Windows (UNMEASURED)
//!
//! Until Wave E3 both lifecycle tests were `#[cfg(unix)]`, so the Windows CI
//! leg compiled 0 tests from this file. It now compiles 3; that they run and
//! pass there is UNMEASURED until the story's Close `--list` step (a green CI
//! step writes no annotation, and run logs are 403 from outside).
//!
//! ## `live` gate
//!
//! `spawn_training_run` needs a tokio runtime, which exists only under `live`
//! (a default feature): `cargo test -p ui --test activity_tape_training_run`.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in PRODUCTION code (`grep -c` of the
//! OLD text is 1), located by symbol + quoted line. Run
//! `cargo test -p ui --test activity_tape_training_run`, restore from a
//! `cp -p` snapshot, `cmp` it, `touch` it, re-run GREEN.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E3-TR-P1 | `ui::lab::trainer::spawn_training_run` (`crates/ui/src/lab/trainer.rs`) | `.map(\|s\| s.start(ActivityKind::Training, training_label));` | `ActivityKind::Training` → `ActivityKind::LabRun` | `training_run_activity_emits_start_and_end_success` at "Start kind must be ActivityKind::Training"; `training_run_activity_emits_end_cancelled_on_stop` at its own kind assert |
//!
//! Vacuity of the pre-E3 file (measured by the E3 base run): it stays GREEN
//! under E3-TR-P1, because it counted `Start` events without reading their
//! kind. No probe for `stub_child_is_rejected_by_libtest`: it is a platform
//! measurement, and an argv mutation libtest would accept could recurse.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use agent::EventBus;
use agent::activity::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase};
use agent::config::BusConfig;
use ui::lab::trainer::TrainingLogLine;
use ui::lab::trainer::{TrainingConfig, cancellation_pair, spawn_training_run};

/// The stub `TrainingConfig`: the child is this test binary (see the module doc
/// for why it cannot recurse). The optional flags stay off, so the child's argv
/// is exactly the measured `--config <p> --output-dir <p>`. Neither path is
/// read: the child exits on the unknown `--config` before it could open one.
fn stub_training_config() -> TrainingConfig {
    TrainingConfig {
        binary_path: std::env::current_exe().expect("the test binary's own path"),
        config_path: std::env::temp_dir().join("training-stub-never-read.toml"),
        output_dir: std::env::temp_dir(),
        dry_run: false,
        epochs: None,
        scenario: None,
        audit_db: None,
    }
}

/// The label `spawn_training_run` must give the `Start` event, derived from
/// the fixture (the stub's binary name) through the documented literal shape
/// `Train <binary-name> · running`.
fn expected_training_label() -> String {
    let cfg = stub_training_config();
    let name = cfg
        .binary_path
        .file_name()
        .and_then(|n| n.to_str())
        .expect("utf-8 exe name");
    format!("Train {name} · running")
}

/// Drain activity events from a broadcast receiver, keeping them.
fn drain_activity(
    rx: &mut tokio::sync::broadcast::Receiver<ActivityEvent>,
) -> (Vec<ActivityEvent>, usize, Vec<ActivityOutcome>) {
    let mut starts: Vec<ActivityEvent> = Vec::new();
    let mut ticks = 0usize;
    let mut ends: Vec<ActivityOutcome> = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev.phase, ActivityPhase::Start { .. }) {
            starts.push(ev);
            continue;
        }
        match ev.phase {
            ActivityPhase::Start { .. } => unreachable!("Start events are kept above"),
            ActivityPhase::Tick { .. } => ticks += 1,
            ActivityPhase::End(outcome) => ends.push(outcome),
        }
    }
    (starts, ticks, ends)
}

/// T-D-N9 — Training subprocess: Start (production) + End { Success } (the
/// test's own drop of the returned handle; module doc § Architecture).
#[test]
fn training_run_activity_emits_start_and_end_success() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create EventBus + subscribe to activity channel BEFORE the spawn.
    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    let cfg = stub_training_config();
    let (_cancel_handle, cancel_rx) = cancellation_pair();
    let (line_tx, _line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

    // Spawn the stub child with an ActivitySender. Returns (TrainingHandle, ActivityHandle).
    let (training_handle, activity_handle) = spawn_training_run(
        Some(rt.handle()),
        &cfg,
        cancel_rx,
        line_tx,
        Some(activity_sender),
    )
    .unwrap_or_else(|e| panic!("spawn_training_run failed: {e}"));

    // The Start event is emitted synchronously, before the spawn returns.
    std::thread::sleep(Duration::from_millis(20));
    let (starts, _, ends_before) = drain_activity(&mut rx);
    assert_eq!(
        starts.len(),
        1,
        "expected exactly 1 Start event after spawn"
    );
    let start = &starts[0];
    assert_eq!(
        start.kind,
        ActivityKind::Training,
        "Start kind must be ActivityKind::Training"
    );
    assert_eq!(
        start.label,
        expected_training_label(),
        "Start label must name the training binary"
    );
    assert_eq!(
        ends_before.len(),
        0,
        "expected 0 End events before the handle drops"
    );

    // Drop the training handle first (kill on drop — the stub has already exited).
    drop(training_handle);

    // Drop the activity handle → emits End { Success } (approach A, iced side).
    drop(activity_handle);

    std::thread::sleep(Duration::from_millis(20));

    // Assert End { Success } arrived.
    let (extra_starts, _ticks, ends) = drain_activity(&mut rx);
    assert_eq!(extra_starts.len(), 0, "no extra Start events expected");
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
/// calls `handle.cancel()` then drops the training_activity_handle; this test
/// makes those two calls itself (module doc § Architecture).
#[test]
fn training_run_activity_emits_end_cancelled_on_stop() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let bus_cfg = BusConfig::default();
    let bus = Arc::new(EventBus::new(&bus_cfg));
    let activity_sender = bus.activity();
    let mut rx = activity_sender.subscribe();

    let cfg = stub_training_config();
    let (_cancel_handle, cancel_rx) = cancellation_pair();
    let (line_tx, _line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

    let (training_handle, activity_handle) = spawn_training_run(
        Some(rt.handle()),
        &cfg,
        cancel_rx,
        line_tx,
        Some(activity_sender),
    )
    .expect("spawn must succeed with the stub child");

    // The Start event, kind checked.
    std::thread::sleep(Duration::from_millis(20));
    let start = rx.try_recv().expect("Start event expected");
    assert!(
        matches!(start.phase, ActivityPhase::Start { .. }),
        "the first event must be Start, got {:?}",
        start.phase
    );
    assert_eq!(
        start.kind,
        ActivityKind::Training,
        "Start kind must be ActivityKind::Training (cancel path)"
    );

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

// ── stub_child_is_rejected_by_libtest (a platform measurement) ───────────────

/// Measures the libtest half of the stub child's no-recursion claim (module
/// doc): this test binary, run with an argv of the shape `spawn_training_run`
/// builds (`--config <p> --output-dir <p>`, the paths from
/// [`stub_training_config`]), exits with libtest's usage error before running
/// any test. It runs on both CI legs, so the Windows half of the claim is
/// measured wherever this test runs (story 2-63 Wave E2 follow-up).
///
/// It spawns `std::env::current_exe()` DIRECTLY with that FIXED argv, never
/// through `spawn_training_run`: were production ever to drop every option,
/// that route would recurse inside this very self-test, while the fixed argv
/// cannot. The child gets 5 s; on expiry it is killed and the test fails.
/// The deadline's kill reaches the direct child only: in the hazard it guards
/// (libtest accepting the argv), grandchildren already spawned through
/// `spawn_training_run` would be orphaned.
///
/// A measurement, no probe (story 2-63 AC4): an argv mutation libtest would
/// accept could recurse, so the subject cannot be mutated safely.
#[test]
fn stub_child_is_rejected_by_libtest() {
    let cfg = stub_training_config();
    let mut child = std::process::Command::new(&cfg.binary_path)
        .arg("--config")
        .arg(&cfg.config_path)
        .arg("--output-dir")
        .arg(&cfg.output_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn this test binary as the stub child");
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().expect("poll the stub child").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "the stub child still ran after 5 s, so libtest did not reject its argv \
                 and it may be running this binary's tests (the recursion hazard); killed it"
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = child
        .wait_with_output()
        .expect("collect the stub child's output");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "libtest must reject the stub argv (got {:?}; stderr {stderr:?})",
        out.status
    );
    assert_eq!(
        out.status.code(),
        Some(101),
        "libtest's usage error exits 101 (stderr {stderr:?})"
    );
    assert!(
        !stdout.lines().any(|line| line.starts_with("running ")),
        "the stub child must run no test (stdout {stdout:?})"
    );
    assert!(
        stderr.contains("Unrecognized option"),
        "libtest names the unknown option on stderr (got {stderr:?})"
    );
}
