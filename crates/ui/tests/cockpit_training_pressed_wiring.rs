//! Integration tests for cockpit-training-pressed-wiring v0.1.0 T-D-N4 / T-D-N5,
//! swept by story 2-63 (lab-recipe-test-harness v0.3) Wave E2.
//!
//! 1. `training_pressed_dispatches_spawn` — `TrainingPressed` flips `training_inflight`
//!    to `Some` and populates `training_activity_handle` + `training_log_rx`.
//! 2. `training_completed_clears_inflight_and_drops_activity` — `TrainingExited`,
//!    dispatched through `ui::state::update`, clears `training_inflight`.
//! 3. `double_press_is_inert` — second `TrainingPressed` while in-flight is a no-op.
//! 4. `k5_toast_non_clobber_run_completed_then_training_completed` — `TrainingExited`,
//!    dispatched through `update`, leaves an existing toast queue untouched (no
//!    toast is emitted on success).
//! 5. `spawn_failure_surfaces_toast` — invalid binary path surfaces an error toast
//!    without panicking; handles remain `None`.
//! 6. `stub_child_is_rejected_by_libtest` — a platform measurement: libtest
//!    rejects the stub child's argv shape (see "The stub child" below).
//!
//! ## What is production code here, and what is a test-local copy
//!
//! Production: `ui::lab::trainer::spawn_training_run` (the real subprocess
//! spawn, synchronous: it calls `rt_handle.block_on` internally) and the
//! `ui::state::update` arms (`TrainingExited`, `ShowToast`,
//! `ShowToastWithSeverity`).
//!
//! A TEST-LOCAL COPY, so an edit to the original is invisible here (story 2-63
//! K3 #6 residual): `TrainingSimState` re-implements the binary's
//! `AppState::update` interception (`crates/ui/src/bin/cockpit_live.rs`), which
//! an integration test cannot link.
//!
//! - `simulate_training_pressed` copies the block under
//!   `if matches!(msg, Message::TrainingPressed) {`: the in-flight
//!   short-circuit, the cancel pair, the 256-slot log channel, the salt bump,
//!   the spawn, and the toast on a spawn error. It takes the config as a
//!   parameter where the bin calls `let cfg = default_training_config();`.
//! - `simulate_training_exited` copies the bin-only clears, in the bin's order:
//!   `training_log_rx` and `training_cancel` (the block under
//!   `if matches!(msg, Message::TrainingExited(_)) {`), then the production
//!   `ui::state::update(TrainingExited)`, which owns the `training_inflight`
//!   clear, then the activity End (`self.training_activity_handle = None;`).
//!
//! Before Wave E2, tests 2 and 4 were tautological: the sim (test 2) and the
//! test body (test 4) WROTE `training_inflight = None` themselves and never
//! dispatched `TrainingExited`, so deleting the clear in `update` (T-T4 E2-P7)
//! or clearing the toast queue in that arm (E2-P8) passed them both. The file
//! name is kept (it names the bin wiring the sim copies), so the test binary
//! and its CI annotation keep their names.
//!
//! ## The stub child is this test binary
//!
//! [`stub_training_config`] points `binary_path` at `std::env::current_exe()`.
//! The child stays inert while its argv carries ANY option libtest does not
//! know, because libtest rejects an unknown option before it runs any test.
//! Today the argv carries two, both unconditional in `spawn_training_run`
//! (`crates/ui/src/lab/trainer.rs`): `"--config".into(),` and
//! `"--output-dir".into(),`. Measured 2026-11-14 on this file's test binary
//! and on `training_log_state_gating`'s: exit 101, no test run, and on stderr
//! `error: Unrecognized option: 'config'`. Since story 2-63 Wave E2's
//! follow-up, [`stub_child_is_rejected_by_libtest`] re-measures that on every
//! run, on both CI legs (a red there writes a CI annotation). No test here
//! needs the child alive: the handles are set by a successful spawn, whatever
//! the child does next.
//!
//! The residual, unguarded: an argv with NO option at all would run the WHOLE
//! test binary in the child, whose spawning tests spawn again. Nothing guards
//! that production change: the self-test measures the libtest half, not the
//! argv half, and it has no probe row, because an argv libtest would accept
//! could recurse. `training_log_state_gating.rs`'s module doc gives the
//! detail.
//!
//! ## Windows (UNMEASURED)
//!
//! Tests 1-3 were `#[cfg(unix)]` (they spawned `sleep`), and the `Option`
//! default's `cfg!(unix)` "Windows fallback" (`timeout`) never ran: no caller
//! passed `None`. With the stub child they compile on the Windows step "Test ui
//! (non-visual, Windows)" in `.github/workflows/ci.yml`, which runs
//! `scripts/ci_run_annotated.sh "ui-windows" cargo test -p ui --features fixtures --no-fail-fast`,
//! as do tests 4-6. That the six RUN and pass there is UNMEASURED: a green
//! step writes no annotation, and run logs need repo admin. A red one does
//! write an annotation, which is what test 6's Windows run is for.
//!
//! The tests only compile under `live` (a default feature) because
//! `spawn_training_run` and the activity bus require a tokio runtime.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure: snapshot the file (`cp -p`), apply
//! the edit, run `cargo test -p ui --test cockpit_training_pressed_wiring` (the
//! named test goes RED), restore from the snapshot THEN `touch` it, and re-run:
//! the GREEN counts only if that run shows `Compiling ui` and the file's
//! SHA-256 equals the pre-probe one.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E2-P7 | `ui::state::update`, arm `Message::TrainingExited(_status)` (`crates/ui/src/state.rs`) | `// Subprocess has exited; clear the inflight handle.`, and the clear under it | replace the clear with a comment | `training_completed_clears_inflight_and_drops_activity`, at "TrainingExited through ui::state::update clears training_inflight". GREEN before Wave E2: the sim wrote `None` itself |
//! | E2-P8 | `ui::state::update`, the same arm | `Message::TrainingExited(_status) => {` | add `model.toast_queue.clear();` as the arm's first statement | `k5_toast_non_clobber_run_completed_then_training_completed`, at "TrainingExited success must not touch the toast queue (K5)" (left 0, right 1). GREEN before Wave E2: the test never dispatched `TrainingExited` |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;

use agent::EventBus;
use agent::activity::{ActivityKind, ActivityPhase};
use agent::config::BusConfig;
use smol_str::SmolStr;
use tokio::sync::broadcast::error::TryRecvError;
use ui::lab::trainer::{TrainingConfig, TrainingLogLine};
use ui::state::{Cockpit, Message, ToastSeverity, update};

// ── Test helpers ──────────────────────────────────────────────────────────────

/// A successful `ExitStatus`. `from_raw` takes an `i32` on Unix and a `u32` on
/// Windows (`ExitStatusExt` per platform, imported above); the literal fits both.
///
/// The same 10-line helper pair lives in `training_log_state_gating.rs`. Each
/// integration test is its own crate, and two small helpers do not justify a
/// shared `tests/` module.
fn exit_ok() -> ExitStatus {
    ExitStatus::from_raw(0)
}

/// The stub `TrainingConfig`: the child is this test binary (module doc). The
/// optional flags stay off, so the child's argv is exactly the measured
/// `--config <p> --output-dir <p>`. Neither path is read: the child exits on
/// the unknown `--config` first.
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

// ── Test helper: the AppState interception, copied (K3 #6) ─────────────────────
//
// Instead of constructing a full `AppState` (which requires a real audit ledger,
// kill switch, etc.), we copy the bin-only state mutations `AppState::update`
// performs for `TrainingPressed` / `TrainingExited`, and dispatch the message
// itself through the production `ui::state::update` (module doc).

/// Minimal state holder that mirrors the AppState fields exercised by T-D-N1.
struct TrainingSimState {
    cockpit: Cockpit,
    rt_handle: tokio::runtime::Handle,
    bus: Arc<EventBus>,
    // Mirrors AppState::training_activity_handle
    training_activity_handle: Option<agent::ActivityHandle>,
    // Mirrors AppState::training_log_rx
    training_log_rx:
        Option<Arc<std::sync::Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>>>,
    training_log_recipe_salt: u64,
}

impl TrainingSimState {
    fn new(rt: &tokio::runtime::Runtime) -> Self {
        let bus_cfg = BusConfig::default();
        let bus = Arc::new(EventBus::new(&bus_cfg));
        Self {
            cockpit: Cockpit::new(),
            rt_handle: rt.handle().clone(),
            bus,
            training_activity_handle: None,
            training_log_rx: None,
            training_log_recipe_salt: 0,
        }
    }

    /// The copy of the TrainingPressed interception in `AppState::update`.
    ///
    /// Returns `Ok(())` if the spawn succeeded (or the press was a no-op because
    /// a run is in flight), `Err(SmolStr)` if the interception set a toast
    /// (i.e., the spawn path hit an error).
    fn simulate_training_pressed(&mut self, cfg: &TrainingConfig) -> Result<(), SmolStr> {
        use ui::lab::trainer::{cancellation_pair, spawn_training_run};

        // Short-circuit if already in-flight.
        if self.cockpit.lab_state.training_inflight.is_some() {
            return Ok(()); // no-op (button disabled)
        }

        let (cancel_handle, cancel_rx) = cancellation_pair();
        let (line_tx, line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);
        let line_rx_arc = Arc::new(std::sync::Mutex::new(Some(line_rx)));
        self.training_log_rx = Some(Arc::clone(&line_rx_arc));
        self.training_log_recipe_salt = self.training_log_recipe_salt.wrapping_add(1);

        match spawn_training_run(
            Some(&self.rt_handle),
            cfg,
            cancel_rx,
            line_tx,
            Some(self.bus.activity()),
        ) {
            Ok((training_handle, activity_handle)) => {
                self.cockpit.lab_state.training_inflight = Some(training_handle);
                self.training_activity_handle = activity_handle;
                self.cockpit.lab_state.training_cancel = Some(cancel_handle);
                Ok(())
            }
            Err(e) => {
                update(
                    &mut self.cockpit,
                    Message::ShowToastWithSeverity(
                        SmolStr::new(format!("Training failed to launch: {e}")),
                        ToastSeverity::Danger,
                    ),
                );
                self.training_log_rx = None;
                Err(e)
            }
        }
    }

    /// `TrainingExited` as the binary handles it: the bin-only clears (copied),
    /// in the bin's order around the PRODUCTION `ui::state::update`, which owns
    /// the `training_inflight` clear (Drop = kill if the child still runs).
    fn simulate_training_exited(&mut self) {
        // Copy of the bin's `TrainingExited` block: the log channel and the
        // cancel handle.
        self.training_log_rx = None;
        self.cockpit.lab_state.training_cancel = None;
        // Production.
        update(&mut self.cockpit, Message::TrainingExited(exit_ok()));
        // Copy of the bin's T-D-N9 lifecycle clear (Drop = End { Success }).
        self.training_activity_handle = None;
    }

    /// Subscribe to activity events from the bus.
    fn activity_rx(&self) -> tokio::sync::broadcast::Receiver<agent::activity::ActivityEvent> {
        self.bus.activity().subscribe()
    }
}

// ── Test 1: pressing the train button spawns subprocess + flips training_inflight ──

/// T-D-N4 case 1 / R1 acceptance: dispatch `TrainingPressed` and assert
/// `training_inflight.is_some()` + `training_activity_handle.is_some()`
/// + `training_log_rx.is_some()`.
#[test]
fn training_pressed_dispatches_spawn() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut sim = TrainingSimState::new(&rt);

    // The stub child (this test binary) spawns for real, then exits on its
    // own; the handles below are set by the successful spawn either way.
    let result = sim.simulate_training_pressed(&stub_training_config());
    assert!(
        result.is_ok(),
        "TrainingPressed must not return error: {result:?}"
    );

    assert!(
        sim.cockpit.lab_state.training_inflight.is_some(),
        "training_inflight must be Some after TrainingPressed"
    );
    assert!(
        sim.training_activity_handle.is_some(),
        "training_activity_handle must be Some after TrainingPressed"
    );
    assert!(
        sim.training_log_rx.is_some(),
        "training_log_rx must be Some after TrainingPressed"
    );
    assert!(
        sim.cockpit.lab_state.training_cancel.is_some(),
        "training_cancel must be Some after TrainingPressed"
    );
    assert!(
        sim.cockpit.toast_queue.is_empty(),
        "toast_queue must remain empty on successful spawn"
    );
}

// ── Test 2: TrainingExited clears training_inflight + drops activity handle ────

/// T-D-N4 case 2: after `TrainingExited`, dispatched through the production
/// `ui::state::update`, `training_inflight` is `None` (T-T4 E2-P7).
///
/// The `training_activity_handle`, `training_log_rx` and `training_cancel`
/// checks read the sim's own copy of the bin's clears: they pin no production
/// code, and are kept only as the copy's self-check.
#[test]
fn training_completed_clears_inflight_and_drops_activity() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut sim = TrainingSimState::new(&rt);

    sim.simulate_training_pressed(&stub_training_config())
        .unwrap();
    assert!(
        sim.cockpit.lab_state.training_inflight.is_some(),
        "precondition: training_inflight is Some after the production spawn"
    );

    // Let the stub child exit, as it has before the binary posts TrainingExited.
    std::thread::sleep(Duration::from_millis(300));

    sim.simulate_training_exited();

    assert!(
        sim.cockpit.lab_state.training_inflight.is_none(),
        "TrainingExited through ui::state::update clears training_inflight"
    );
    assert!(
        sim.training_activity_handle.is_none(),
        "training_activity_handle must be None after TrainingExited (the sim's copy of the bin clear)"
    );
    assert!(
        sim.training_log_rx.is_none(),
        "training_log_rx must be None after TrainingExited (the sim's copy of the bin clear)"
    );
    assert!(
        sim.cockpit.lab_state.training_cancel.is_none(),
        "training_cancel must be None after TrainingExited (the sim's copy of the bin clear)"
    );
}

// ── Test 3: double press is inert (button disabled Q2) ────────────────────────

/// T-D-N4 case 3 / R4 acceptance: pressing `TrainingPressed` twice in rapid
/// succession spawns only ONE subprocess (only one `Start` event in the bus).
/// The second press is a no-op because `training_inflight.is_some()`.
///
/// The count is read off a drain that ends ONLY on `Err(TryRecvError::Empty)`;
/// a `Lagged` (events lost, a second `Start` among them) or `Closed` channel
/// panics instead of passing as "drained" (story 2-63 Wave E2 follow-up; no
/// probe: with one or two events on the default bus, REASONED unreachable).
/// The second press is short-circuited by the sim's COPY of the bin's in-flight
/// check (K3 #6, module doc), so the only production fact pinned here is that
/// one `spawn_training_run` emits one `Training` `Start`.
#[test]
fn double_press_is_inert() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut sim = TrainingSimState::new(&rt);
    let mut activity_rx = sim.activity_rx();

    let cfg = stub_training_config();

    // First press — should spawn.
    let first = sim.simulate_training_pressed(&cfg);
    assert!(first.is_ok(), "first TrainingPressed must succeed");
    assert!(sim.cockpit.lab_state.training_inflight.is_some());

    // Second press — must be a no-op (training_inflight is already Some).
    let second = sim.simulate_training_pressed(&cfg);
    assert!(second.is_ok(), "second TrainingPressed must not error");

    // Wait briefly for the Start event to propagate.
    std::thread::sleep(Duration::from_millis(50));

    // Count Start events in the bus.
    let mut start_count = 0usize;
    loop {
        match activity_rx.try_recv() {
            Ok(ev) if matches!(ev.phase, ActivityPhase::Start { .. }) => {
                start_count += 1;
                // Check it's a Training kind.
                assert_eq!(
                    ev.kind,
                    ActivityKind::Training,
                    "Start event must be Training kind"
                );
            }
            Ok(_) => {} // Tick or End events — ignore for this assertion.
            Err(TryRecvError::Empty) => break,
            Err(other) => panic!(
                "the activity drain hit {other:?}: an event was lost or the bus closed, \
                 so the Start count would not be the bus's"
            ),
        }
    }
    assert_eq!(
        start_count, 1,
        "exactly 1 Start event expected; double-press must not spawn a second subprocess"
    );

    // training_inflight still Some (single handle).
    assert!(
        sim.cockpit.lab_state.training_inflight.is_some(),
        "training_inflight must still be Some after double-press"
    );
}

// ── Test 4: K5 toast non-clobber ──────────────────────────────────────────────

/// T-D-N4 case 4 / K5: when an existing toast ("Backtest complete") is queued
/// and `TrainingExited(success)` is dispatched through the production
/// `ui::state::update`, the toast queue is untouched: same length, same front
/// (the silent no-op contract from T-AR-2: no toast on a successful exit).
/// T-T4 E2-P8.
#[test]
fn k5_toast_non_clobber_run_completed_then_training_completed() {
    let mut cockpit = Cockpit::new();

    // Pre-set an existing toast (e.g., from a just-completed backtest).
    update(
        &mut cockpit,
        Message::ShowToast(SmolStr::new("Backtest complete")),
    );
    assert_eq!(
        cockpit.toast_queue.len(),
        1,
        "precondition: the backtest toast is queued"
    );

    // The production arm, dispatched. Before Wave E2 the test wrote the arm's
    // effect by hand and never sent the message.
    update(&mut cockpit, Message::TrainingExited(exit_ok()));

    assert_eq!(
        cockpit.toast_queue.len(),
        1,
        "TrainingExited success must not touch the toast queue (K5)"
    );
    assert_eq!(
        cockpit.toast_queue.front().map(|t| t.message.as_str()),
        Some("Backtest complete"),
        "existing toast must not be clobbered by TrainingExited success (K5 silent-no-op contract)"
    );
}

// ── Test 5: spawn failure surfaces toast ─────────────────────────────────────

/// T-D-N5 / R1.1 step 7: when the binary path is invalid, `TrainingPressed`
/// surfaces an error toast and leaves all handles as `None`.
#[test]
fn spawn_failure_surfaces_toast() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut sim = TrainingSimState::new(&rt);

    let bad_cfg = TrainingConfig {
        binary_path: PathBuf::from("/nonexistent/train_tcn_xyzzy_test"),
        ..stub_training_config()
    };

    let result = sim.simulate_training_pressed(&bad_cfg);
    assert!(
        result.is_err(),
        "TrainingPressed with invalid binary must return Err"
    );

    // Toast must be set (enqueued via message dispatch).
    assert!(
        !sim.cockpit.toast_queue.is_empty(),
        "toast_queue must be non-empty after spawn failure"
    );
    let toast = sim.cockpit.toast_queue.front().unwrap().message.as_str();
    assert!(
        toast.contains("Training failed"),
        "toast must contain 'Training failed'; got: {toast}"
    );

    // All handles must remain None.
    assert!(
        sim.cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must be None after spawn failure"
    );
    assert!(
        sim.training_activity_handle.is_none(),
        "training_activity_handle must be None after spawn failure"
    );
    assert!(
        sim.training_log_rx.is_none(),
        "training_log_rx must be None after spawn failure"
    );
}

// ── Test 6: stub_child_is_rejected_by_libtest (a platform measurement) ────────

/// Measures the libtest half of the stub child's no-recursion claim (module
/// doc): this test binary, run with an argv of the shape `spawn_training_run`
/// builds (`--config <p> --output-dir <p>`, the paths from
/// [`stub_training_config`]), exits with libtest's usage error before running
/// any test. It runs on both CI legs, so the Windows half of the claim is
/// measured wherever this test runs (story 2-63 Wave E2 follow-up). The same
/// self-test lives in `training_log_state_gating.rs`: each test binary is its
/// own child, so each measures itself.
///
/// It spawns `std::env::current_exe()` DIRECTLY with that FIXED argv, never
/// through `spawn_training_run` or the sim: were production ever to drop every
/// option, that route would recurse inside this very self-test, while the
/// fixed argv cannot. The child gets 5 s; on expiry it is killed and the test
/// fails. The deadline's kill reaches the direct child only: in the hazard it
/// guards (libtest accepting the argv), grandchildren already spawned through
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
