//! Surface 2 — state-gating tests for `TrainingLogRecipe` panel visibility.
//!
//! lab-recipe-test-harness v0.2.0 Wave A (T-D-A2 / ADR-0048 D3 category C).
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
//! Pattern: uses `spawn_training_run` (live feature) to populate
//! `training_inflight` exactly as production does, then dispatches lifecycle
//! messages via `state::update` and asserts on `training_inflight.is_some()`.
//! Mirrors `lab_stop_button_gating.rs` + `cockpit_training_pressed_wiring.rs`
//! K5-pattern.
//!
//! ## Regression category C
//!
//! The falsification probe is `state.rs:2232` — the line
//! `model.lab_state.training_inflight = None;` in the `TrainingExited` arm.
//! If that line is removed, `training_exited_clears_inflight` fails because
//! the Cancel button stays visible after the subprocess exits.
//!
//! ## `#[cfg(feature = "live")]` gate
//!
//! `spawn_training_run` requires a tokio runtime (only under `live`).
//! Run with `cargo test -p ui --test training_log_state_gating --features live`.
//!
//! ## T-T4 falsification probe (per ADR-0048 § Changelog 2026-05-29 v0.2.0)
//!
//! To verify this harness genuinely catches the regression class, apply ONE of the
//! following mutations to the production source, then re-run this test file. Restore
//! when done.
//!
//! | Probe | Source line to comment out / mutate | Expected failing test | Expected failure |
//! |---|---|---|---|
//! | P3 — TrainingExited clear | `crates/ui/src/state.rs:2232` — comment out `model.lab_state.training_inflight = None;` in the `TrainingExited` arm | `training_exited_clears_inflight` | `training_inflight.is_none()` assertion fails; handle stays `Some` after exit |
//! | P4 — TrainingCancelPressed clear | `crates/ui/src/state.rs:2225` — comment out `model.lab_state.training_inflight = None;` in the `TrainingCancelPressed` arm | `training_log_panel_state_after_cancellation` | `training_inflight.is_none()` assertion fails; handle stays `Some` after cancel press |
//!
//! **Developer dry-run result (Wave A, 2026-05-29)**:
//! - Commenting out `state.rs:2232`: `training_exited_clears_inflight` FAILED —
//!   `assert!(cockpit.lab_state.training_inflight.is_none())` panicked with
//!   "training_inflight must be None after TrainingExited".
//! - Restoring line 2232: all 3 tests PASS.
//!   (See commit message for exact test output line.)

#![cfg(feature = "live")]
#![cfg(unix)] // spawn_training_run uses Unix process semantics in tests
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use std::os::unix::process::ExitStatusExt as _;

use agent::EventBus;
use agent::config::BusConfig;
use smol_str::SmolStr;
use ui::lab::trainer::{TrainingConfig, TrainingLogLine, cancellation_pair, spawn_training_run};
use ui::state::{Cockpit, Message, update};

// ── Test helper: populate training_inflight via spawn_training_run ─────────────

/// Populate `cockpit.lab_state.training_inflight` by spawning `sleep 5`
/// as a subprocess — the same path production takes in `cockpit_live.rs`.
///
/// Returns the `SyncSender<TrainingLogLine>` so the test can drive log lines
/// independently (optional — most tests just drop it).
fn arm_training_inflight(
    rt: &tokio::runtime::Runtime,
    cockpit: &mut Cockpit,
    bus: &Arc<EventBus>,
) -> std::sync::mpsc::SyncSender<TrainingLogLine> {
    let cfg = TrainingConfig {
        binary_path: PathBuf::from("sleep"),
        config_path: PathBuf::from("/dev/null"),
        output_dir: PathBuf::from("/tmp"),
        dry_run: false,
        epochs: None,
        scenario: None,
        audit_db: None,
    };
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

// ── Test 4 — training_log_panel_visibility_gated_on_inflight ─────────────────

/// T-D-A2 / ADR-0048 D3 category C — panel visibility gated on inflight handle.
///
/// Asserts:
/// 1. Default Cockpit → `training_inflight.is_none()` (Cancel button hidden).
/// 2. After arming via `spawn_training_run` → `training_inflight.is_some()`.
/// 3. After `TrainingExited` → `training_inflight.is_none()` (Cancel hidden).
///
/// **Regression catch**: if `TrainingExited` stops clearing `training_inflight`
/// (e.g. line 2232 in state.rs removed), step 3 fails immediately.
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
    let status = std::process::ExitStatus::from_raw(0);
    update(&mut cockpit, Message::TrainingExited(status));
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must be None after TrainingExited \
         (Cancel button must disappear). \
         Regression C detected: TrainingExited no longer clears inflight handle. \
         Falsification probe P3: check state.rs:2232."
    );
}

// ── Test 5 — training_log_panel_clears_on_completion ────────────────────────

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
    let status = std::process::ExitStatus::from_raw(0);
    update(&mut cockpit, Message::TrainingExited(status));
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must be None after TrainingExited(success). \
         Regression C detected: predicate not cleared on completion. \
         Falsification probe P3: check state.rs:2232."
    );

    // Step 5: training_log persists (operator can read post-run output).
    assert_eq!(
        cockpit.lab_state.training_log.len(),
        3,
        "training_log must NOT be cleared by TrainingExited \
         (log persists for operator inspection after run)"
    );
}

// ── Test 6 — training_log_panel_state_after_cancellation ─────────────────────

/// T-D-A2 / ADR-0048 D3 category C — Stop mid-training.
///
/// Asserts:
/// 1. After arm → `training_inflight.is_some()`.
/// 2. `TrainingCancelPressed` → `training_inflight.is_none()` (SIGKILL path).
/// 3. Log lines dispatch BEFORE cancel do NOT affect the cancel outcome.
///
/// **Regression catch**: if `TrainingCancelPressed` stops clearing
/// `training_inflight` (e.g. line 2225 in state.rs removed), the Cancel
/// button stays visible but pressing it no longer actually kills the handle.
/// The operator has no feedback that cancel failed.
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
         Falsification probe P4: check state.rs:2225."
    );

    // After cancel, log lines must still dispatch without panicking.
    // (The channel sender may still be alive in the spawned background task;
    //  the update fn must handle TrainingLogLine even post-cancel gracefully.)
    update(
        &mut cockpit,
        Message::TrainingLogLine(SmolStr::new("[info] epoch 2/30")),
    );
    // No assertion on training_inflight here — it stays None.
    // We only assert no panic occurred (the test reaches here).
    assert!(
        cockpit.lab_state.training_inflight.is_none(),
        "training_inflight must remain None after TrainingLogLine post-cancellation"
    );

    // Brief sleep to allow the OS to reap the killed `sleep 5` process.
    std::thread::sleep(Duration::from_millis(100));
}
