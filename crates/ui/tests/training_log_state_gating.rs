//! Surface 2 — state-gating tests for `TrainingLogRecipe` panel visibility.
//!
//! lab-recipe-test-harness v0.2.0 Wave A (T-D-A2 / ADR-0048 D3 category C);
//! swept and ported off `cfg(unix)` by story 2-63 (v0.3) Wave E2.
//!
//! ## What this file tests
//!
//! The `model.lab_state.training_inflight` predicate that gates the training
//! panel's "Cancel" button at `crates/ui/src/screens/lab.rs`. Verifies that
//! `training_inflight` transitions correctly across the training lifecycle:
//!
//! ```text
//! Cockpit::default()         → training_inflight == None  (Cancel button hidden)
//! TrainingPressed (binary)   → training_inflight == Some  (Cancel button appears)
//! TrainingExited(status)     → training_inflight == None  (Cancel button hidden)
//! TrainingCancelPressed      → training_inflight == None  (immediately hidden)
//! ```
//!
//! Pattern: the production `spawn_training_run` (live feature) populates
//! `training_inflight` exactly as the binary does; the lifecycle messages then
//! go through the production `ui::state::update`, and the tests assert on
//! `training_inflight.is_some()`. Every clear therefore starts from a `Some`
//! the production spawn produced, never from a value the test wrote. Mirrors
//! `lab_stop_button_gating.rs`.
//!
//! ## The stub child is this test binary
//!
//! [`stub_training_config`] points `binary_path` at `std::env::current_exe()`,
//! this very test binary, so the spawn needs no `sleep` (Unix-only) or any other
//! PATH tool and behaves the same on macOS and Windows.
//!
//! Why the child cannot recurse into these tests: it stays inert while its
//! argv carries ANY option libtest does not know, because libtest rejects an
//! unknown option before it runs any test. Today the argv carries two, both
//! unconditional in `spawn_training_run` (`crates/ui/src/lab/trainer.rs`): it
//! opens with `"--config".into(),` and also passes `"--output-dir".into(),`.
//! Measured 2026-11-14 on this file's test binary with
//! `--config <p> --output-dir <p>`: exit 101, no test run, and on stderr
//! `error: Unrecognized option: 'config'`. Since story 2-63 Wave E2's follow-up,
//! [`stub_child_is_rejected_by_libtest`] re-measures that on every run, on both
//! CI legs (a red there writes a CI annotation). The stub keeps `dry_run` false
//! and `epochs`/`scenario`/`audit_db` at `None`, so the child's argv is exactly
//! the measured one. The child exits at once; no test here needs it alive,
//! because `training_inflight` holds the handle whatever the child does.
//!
//! The residual, unguarded: dropping ONE of the two options cannot recurse
//! (the other is still rejected), but an argv with NO option at all would run
//! the WHOLE test binary in the child, whose arming tests spawn again, a
//! process fan-out. Nothing guards that production change. The self-test
//! measures the libtest half (an unknown option is rejected, on this platform
//! and toolchain), not the argv half (that `spawn_training_run` still passes
//! one). It cannot be probed safely: an argv libtest would accept could
//! recurse, so the self-test is a measurement, with no probe row.
//!
//! ## Windows (UNMEASURED)
//!
//! Until story 2-63 Wave E2 this file was `#![cfg(unix)]`, so the Windows CI
//! leg compiled 0 tests from it, although the subject (the `state.rs` arms) is
//! platform-neutral. The Windows step "Test ui (non-visual, Windows)" in
//! `.github/workflows/ci.yml`, which runs
//! `scripts/ci_run_annotated.sh "ui-windows" cargo test -p ui --features fixtures --no-fail-fast`,
//! now compiles all four
//! tests (the three lifecycle tests and the libtest self-test). That they RUN
//! and pass there is UNMEASURED: a green step writes no annotation (the
//! success path of `scripts/ci_run_annotated.sh`), and run logs need repo
//! admin, so from outside 0 tests and 4 look the same. A red one does write an
//! annotation, which is what the self-test's Windows run is for.
//!
//! ## `live` gate
//!
//! `spawn_training_run` needs a tokio runtime, which exists only under `live`
//! (a default feature): `cargo test -p ui --test training_log_state_gating`.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure: snapshot the file (`cp -p`), apply
//! the edit, run `cargo test -p ui --test training_log_state_gating` (the named
//! tests go RED), restore from the snapshot THEN `touch` it, and re-run: the
//! GREEN counts only if that run shows `Compiling ui` and the file's SHA-256
//! equals the pre-probe one. The clear itself,
//! `model.lab_state.training_inflight = None;`, occurs in BOTH arms of
//! `crates/ui/src/state.rs`, so each row quotes the comment line above it,
//! which is unique.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E2-P7 (was P3) | `ui::state::update`, arm `Message::TrainingExited(_status)` (`crates/ui/src/state.rs`) | `// Subprocess has exited; clear the inflight handle.`, and the clear under it | replace the clear with a comment | `training_log_panel_visibility_gated_on_inflight`, at "training_inflight must be None after TrainingExited (Cancel button must disappear)", and `training_log_panel_clears_on_completion`, at "training_inflight must be None after TrainingExited(success)" |
//! | E2-P9 (was P4) | `ui::state::update`, arm `Message::TrainingCancelPressed` | `// Drop the handle — Drop impl calls start_kill() → SIGKILL.`, and the clear under it | replace the clear with a comment | `training_log_panel_state_after_cancellation`, at "training_inflight must be None immediately after TrainingCancelPressed" |
//!
//! Both probes were red here before Wave E2 too, on macOS: this file always
//! drove the production arms. The port's value is that the same rows now
//! compile on Windows, whose half stays UNMEASURED (above).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;

use agent::EventBus;
use agent::config::BusConfig;
use smol_str::SmolStr;
use ui::lab::trainer::{TrainingConfig, TrainingLogLine, cancellation_pair, spawn_training_run};
use ui::state::{Cockpit, Message, update};

// ── Test helpers ──────────────────────────────────────────────────────────────

/// A successful `ExitStatus`. `from_raw` takes an `i32` on Unix and a `u32` on
/// Windows (`ExitStatusExt` per platform, imported above); the literal fits both.
fn exit_ok() -> ExitStatus {
    ExitStatus::from_raw(0)
}

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

/// Populate `cockpit.lab_state.training_inflight` through the production
/// `spawn_training_run` (the call `cockpit_live.rs` makes on `TrainingPressed`),
/// with the stub child.
///
/// Returns the `SyncSender<TrainingLogLine>` so the test can drive log lines
/// independently (optional — most tests just drop it).
fn arm_training_inflight(
    rt: &tokio::runtime::Runtime,
    cockpit: &mut Cockpit,
    bus: &Arc<EventBus>,
) -> std::sync::mpsc::SyncSender<TrainingLogLine> {
    let cfg = stub_training_config();
    let (_, cancel_rx) = cancellation_pair();
    let (line_tx, line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(16);
    let _line_rx_arc = Arc::new(std::sync::Mutex::new(Some(line_rx)));

    let (training_handle, _activity_handle) = spawn_training_run(
        Some(rt.handle()),
        &cfg,
        cancel_rx,
        line_tx.clone(),
        Some(bus.activity()),
    )
    .expect("spawn_training_run must succeed in test environment");

    cockpit.lab_state.training_inflight = Some(training_handle);
    line_tx
}

// ── Test 1 — training_log_panel_visibility_gated_on_inflight ─────────────────

/// T-D-A2 / ADR-0048 D3 category C — panel visibility gated on inflight handle.
///
/// Asserts:
/// 1. Default Cockpit → `training_inflight.is_none()` (Cancel button hidden).
/// 2. After arming via `spawn_training_run` → `training_inflight.is_some()`.
/// 3. After `TrainingExited` → `training_inflight.is_none()` (Cancel hidden).
///
/// **Regression catch**: if the `TrainingExited` arm of `ui::state::update`
/// stops clearing `training_inflight` (T-T4 E2-P7), step 3 fails immediately.
#[test]
fn training_log_panel_visibility_gated_on_inflight() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut cockpit = Cockpit::new();

    // Step 1: default state.
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "default Cockpit must have training_inflight == None \
         (training panel Cancel button must be hidden)"
    );

    // Step 2: arm training (subprocess spawned).
    let _line_tx = arm_training_inflight(&rt, &mut cockpit, &bus);
    assert!(
        cockpit.lab_state.training_inflight.is_some(),
        "training_inflight must be Some after spawn \
         (Cancel button must be visible)"
    );

    // Step 3: TrainingExited clears the handle.
    update(&mut cockpit, Message::TrainingExited(exit_ok()));
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must be None after TrainingExited \
         (Cancel button must disappear). \
         Regression C detected: TrainingExited no longer clears inflight handle. \
         Falsification probe E2-P7: the TrainingExited arm of ui::state::update."
    );
}

// ── Test 2 — training_log_panel_clears_on_completion ────────────────────────

/// T-D-A2 / ADR-0048 D3 category C — predicate-gated clear on subprocess exit.
///
/// Asserts the full lifecycle sequence:
/// 1. Default → `training_inflight.is_none()`.
/// 2. After arm → `training_inflight.is_some()`.
/// 3. `TrainingLogLine` messages arrive while inflight (log populates without
///    clearing the inflight flag).
/// 4. `TrainingExited(success)` → `training_inflight.is_none()`.
/// 5. `training_log` ring buffer is NOT cleared by `TrainingExited` — log
///    lines persist for operator inspection after the run.
///
/// **Regression catch**: verifies that `TrainingLogLine` messages do NOT
/// accidentally clear `training_inflight` (belt-and-suspenders; should
/// be obvious from state.rs but worth asserting explicitly).
#[test]
fn training_log_panel_clears_on_completion() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut cockpit = Cockpit::new();

    // Step 1: default.
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "default must have training_inflight == None"
    );
    assert!(
        cockpit.lab_state.training_log.is_empty(),
        "default must have empty training_log ring buffer"
    );

    // Step 2: arm.
    let _line_tx = arm_training_inflight(&rt, &mut cockpit, &bus);
    assert!(
        cockpit.lab_state.training_inflight.is_some(),
        "training_inflight must be Some after spawn"
    );

    // Step 3: dispatch 3 log lines — must NOT clear inflight.
    for i in 1..=3 {
        update(
            &mut cockpit,
            Message::TrainingLogLine(SmolStr::new(format!("[info] epoch {i}/30"))),
        );
        assert!(
            cockpit.lab_state.training_inflight.is_some(),
            "training_inflight must stay Some after TrainingLogLine (epoch {i})"
        );
    }
    assert_eq!(
        cockpit.lab_state.training_log.len(),
        3,
        "training_log must have 3 lines after 3 TrainingLogLine messages"
    );

    // Step 4: TrainingExited(success) → inflight clears.
    update(&mut cockpit, Message::TrainingExited(exit_ok()));
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must be None after TrainingExited(success). \
         Regression C detected: predicate not cleared on completion. \
         Falsification probe E2-P7: the TrainingExited arm of ui::state::update."
    );

    // Step 5: training_log persists (operator can read post-run output).
    assert_eq!(
        cockpit.lab_state.training_log.len(),
        3,
        "training_log must NOT be cleared by TrainingExited \
         (log persists for operator inspection after run)"
    );
}

// ── Test 3 — training_log_panel_state_after_cancellation ─────────────────────

/// T-D-A2 / ADR-0048 D3 category C — Stop mid-training.
///
/// Asserts:
/// 1. After arm → `training_inflight.is_some()`.
/// 2. `TrainingCancelPressed` → `training_inflight.is_none()` (SIGKILL path).
/// 3. Log lines dispatch BEFORE cancel do NOT affect the cancel outcome.
///
/// **Regression catch**: if the `TrainingCancelPressed` arm of
/// `ui::state::update` stops clearing `training_inflight` (T-T4 E2-P9), the
/// Cancel button stays visible but pressing it no longer actually kills the
/// handle. The operator has no feedback that cancel failed.
///
/// This is the Stop-mid-training analogue of `lab_stop_button_gating.rs`
/// Test 3 (`stop_requested_mid_run_leaves_inflight_true`), but for training:
/// unlike `LabRunStopRequested` (which leaves inflight true until
/// `LabRunCompleted` arrives), `TrainingCancelPressed` clears `training_inflight`
/// IMMEDIATELY because Drop-on-clear IS the cancel mechanism (SIGKILL semantics
/// per ADR-0034).
#[test]
fn training_log_panel_state_after_cancellation() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut cockpit = Cockpit::new();

    // Arm: subprocess starts.
    let _line_tx = arm_training_inflight(&rt, &mut cockpit, &bus);
    assert!(
        cockpit.lab_state.training_inflight.is_some(),
        "training_inflight must be Some after spawn"
    );

    // Dispatch some log lines before cancel — must not affect cancel outcome.
    update(
        &mut cockpit,
        Message::TrainingLogLine(SmolStr::new("[info] epoch 1/30")),
    );
    assert!(
        cockpit.lab_state.training_inflight.is_some(),
        "training_inflight must stay Some after TrainingLogLine (pre-cancel)"
    );

    // Cancel pressed — SIGKILL fires via Drop of TrainingHandle.
    update(&mut cockpit, Message::TrainingCancelPressed);

    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must be None immediately after TrainingCancelPressed \
         (Drop-on-clear IS the kill mechanism — ADR-0034 Q2). \
         Regression C detected: TrainingCancelPressed no longer clears inflight handle. \
         Falsification probe E2-P9: the TrainingCancelPressed arm of ui::state::update."
    );

    // After cancel, log lines must still dispatch without panicking.
    // (The channel sender may still be alive in the spawned background task;
    //  the update fn must handle TrainingLogLine even post-cancel gracefully.)
    update(
        &mut cockpit,
        Message::TrainingLogLine(SmolStr::new("[info] epoch 2/30")),
    );
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must remain None after TrainingLogLine post-cancellation"
    );

    // A short pause before the runtime drops, so the OS can reap the stub child
    // (it exits on its own; the handle's kill on drop finds it gone or ending).
    std::thread::sleep(Duration::from_millis(100));
}

// ── Test 4 — stub_child_is_rejected_by_libtest (a platform measurement) ──────

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
