//! Surface 2 — Stop-button gating state-machine tests.
//!
//! lab-recipe-test-harness v0.1.0 (T-D3 / ADR-0048 D3 category C).
//!
//! ## What this file tests
//!
//! The `model.lab_run_inflight` predicate that gates the Stop button at
//! `crates/ui/src/screens/lab.rs:419`. Verifies that the flag transitions
//! correctly across the full Lab-run message lifecycle:
//!
//! ```text
//! Cockpit::default()       → lab_run_inflight == false
//! LabRunRequested          → lab_run_inflight == true   (Stop button appears)
//! LabRunProgress × 5       → stays true  (linger/partial must not flip it)
//! LabRunCompleted(Ok)      → lab_run_inflight == false  (Stop button disappears)
//! LabRunCompleted(Err)     → lab_run_inflight == false  (error path also clears)
//! LabRunStopRequested      → pure state unchanged (binary side clears cancel)
//! ```
//!
//! Pattern: K5 shape (`cockpit_training_pressed_wiring.rs`) — construct
//! `Cockpit::new()`, dispatch `state::update(...)`, assert on the field.
//! No tokio runtime required; all assertions are on pure state transitions.
//!
//! ## Regression category C
//!
//! Bug #64 attempt 1 (D.2.1 post-completion linger) broke the Stop button by
//! causing `LabRunCompleted` to not clear `run_progress` immediately and the
//! binary-side linger timer to interact with `lab_run_inflight` in unexpected
//! ways. While `lab_run_inflight` itself was still cleared by `LabRunCompleted`
//! in that attempt, the tests here ensure any future regression to this
//! contract is caught immediately.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use smol_str::SmolStr;
use ui::lab::runner::RunSummary;
use ui::state::{Cockpit, Message, update};

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Build a minimal `RunSummary` for `LabRunCompleted(Ok(...))` dispatch.
fn dummy_run_summary() -> RunSummary {
    use std::sync::Arc;
    RunSummary {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        report_path: None,
        equity_series: Vec::new(),
        fills: Vec::new(),
        kpis: backtest::BacktestKpis::default(),
        bars: Arc::new(Vec::new()),
        position_curve: Vec::new(),
    }
}

/// Build a `Progress` event for `LabRunProgress` dispatch.
fn progress_event(current_bar: usize, total_bars: usize) -> backtest::progress::Progress {
    backtest::progress::Progress {
        current_bar,
        total_bars,
        elapsed_ms: current_bar as u64 * 10,
    }
}

// ── Test 1 — Full lifecycle: LabRunRequested → progress × 5 → LabRunCompleted ─

/// T-D3 / ADR-0048 D3 category C — full lifecycle with Ok completion.
///
/// Asserts:
/// 1. Default → `lab_run_inflight == false`.
/// 2. After `LabRunRequested` → `true`.
/// 3. After 5 × `LabRunProgress(p)` → stays `true`.
/// 4. After `LabRunCompleted(Ok(_))` → `false`.
/// 5. `run_progress` is `None` after completion (belt-and-suspenders clear).
///
/// **Regression catch**: if `LabRunCompleted` stops clearing `lab_run_inflight`
/// (e.g. someone accidentally removes the `model.lab_run_inflight = false` line
/// from `state.rs`), step 4 fails and this test catches it immediately.
#[test]
fn full_lifecycle_ok_completion_clears_inflight() {
    let mut cockpit = Cockpit::new();

    // Step 1: default state.
    assert!(
        !cockpit.lab_run_inflight,
        "default Cockpit must have lab_run_inflight == false"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "default Cockpit must have run_progress == None"
    );

    // Step 2: LabRunRequested → inflight = true.
    update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "lab_run_inflight must be true after LabRunRequested (Stop button must appear)"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "LabRunRequested must clear stale run_progress from prior run"
    );

    // Step 3: 5 × LabRunProgress — inflight stays true.
    for bar in [0usize, 128, 256, 384, 512] {
        update(
            &mut cockpit,
            Message::LabRunProgress(progress_event(bar, 720)),
        );
        assert!(
            cockpit.lab_run_inflight,
            "lab_run_inflight must stay true after LabRunProgress({bar}/720)"
        );
        assert!(
            cockpit.lab_state.run_progress.is_some(),
            "run_progress must be Some after LabRunProgress({bar}/720)"
        );
    }

    // Step 4: LabRunCompleted(Ok) → inflight = false.
    update(
        &mut cockpit,
        Message::LabRunCompleted(Ok(dummy_run_summary())),
    );
    assert!(
        !cockpit.lab_run_inflight,
        "lab_run_inflight must be false after LabRunCompleted(Ok) \
         (Stop button must disappear). \
         Regression C detected: LabRunCompleted no longer clears inflight flag."
    );

    // Step 5: run_progress must be None after completion.
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "run_progress must be None after LabRunCompleted(Ok)"
    );
    // last_run_error must be None on success.
    assert!(
        cockpit.lab_state.last_run_error.is_none(),
        "last_run_error must be None after LabRunCompleted(Ok)"
    );
}

// ── Test 2 — Error path also clears inflight + sets last_run_error ─────────

/// T-D3 / ADR-0048 D3 category C — Err completion path.
///
/// Asserts:
/// 1. After `LabRunCompleted(Err(_))` → `lab_run_inflight == false`.
/// 2. `last_run_error` is `Some(msg)` (error surface for the Run button).
/// 3. `run_progress` is `None` (progress cleared regardless of outcome).
///
/// **Regression catch**: if the Err path fails to clear `lab_run_inflight`
/// (asymmetric treatment of Ok/Err), the Stop button gets stuck visible.
#[test]
fn err_completion_clears_inflight() {
    let mut cockpit = Cockpit::new();

    update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "lab_run_inflight must be true after LabRunRequested"
    );

    // Simulate partial progress.
    update(
        &mut cockpit,
        Message::LabRunProgress(progress_event(64, 720)),
    );
    assert!(cockpit.lab_state.run_progress.is_some());

    // Err completion.
    let err_msg = SmolStr::new("Yahoo auto-fetch failed: timeout");
    update(&mut cockpit, Message::LabRunCompleted(Err(err_msg.clone())));

    assert!(
        !cockpit.lab_run_inflight,
        "lab_run_inflight must be false after LabRunCompleted(Err) \
         — Stop button must not remain stuck visible on error. \
         Regression C detected."
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "run_progress must be None after LabRunCompleted(Err)"
    );
    assert_eq!(
        cockpit.lab_state.last_run_error.as_deref(),
        Some(err_msg.as_str()),
        "last_run_error must surface the error message after LabRunCompleted(Err)"
    );
}

// ── Test 3 — Stop requested mid-run: pure state unchanged ─────────────────

/// T-D3 / ADR-0048 D3 category C — Stop button press mid-run.
///
/// Per `state.rs:2179`, `LabRunStopRequested` is a pure-state no-op
/// (the binary side drops `lab_state.run_cancel`). This test asserts that
/// the pure-state arm does NOT accidentally flip `lab_run_inflight` to false
/// (which would make the Stop button disappear before the engine actually stops).
///
/// The flag stays `true` until `LabRunCompleted` arrives (the engine's
/// cancellation path still calls back via the Task).
///
/// **Regression catch**: if someone accidentally adds
/// `model.lab_run_inflight = false` to the `LabRunStopRequested` arm,
/// the Stop button would vanish immediately on press but the engine would
/// keep running (no visual feedback). This test catches that.
#[test]
fn stop_requested_mid_run_leaves_inflight_true() {
    let mut cockpit = Cockpit::new();

    update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "inflight must be true after LabRunRequested"
    );

    // Dispatch LabRunStopRequested — pure state: no change.
    update(&mut cockpit, Message::LabRunStopRequested);
    assert!(
        cockpit.lab_run_inflight,
        "lab_run_inflight must remain true after LabRunStopRequested \
         (binary side handles cancel; pure state is unchanged per state.rs:2179). \
         Regression C detected: Stop request incorrectly cleared inflight flag."
    );

    // Engine eventually returns Err(Cancelled) → LabRunCompleted(Err).
    let cancelled_msg = SmolStr::new("cancelled");
    update(&mut cockpit, Message::LabRunCompleted(Err(cancelled_msg)));
    assert!(
        !cockpit.lab_run_inflight,
        "lab_run_inflight must be false after LabRunCompleted(Err(cancelled))"
    );
}

// ── Test 4 — D-ER-4 T5 (R3): no-data notice also clears inflight ─────────────

/// T5 — D-ER-4 R3 terminal-state gate (lab-yahoo-empty-range-ux v0.1.0 / M-DEV.15).
///
/// A no-data `LabRunCompleted(Err(tagged))` outcome MUST:
/// 1. Clear `lab_run_inflight` → false (no spinner hang, R3).
/// 2. Clear `run_progress` → None.
/// 3. Set `last_run_notice` (muted) and NOT `last_run_error` (red ⚠).
///
/// This is the T5 gate from feature.md § D-ER-4: "after the no-data
/// `LabRunCompleted(Err)`, assert `lab_run_inflight == false` and
/// `run_progress.is_none()`".
#[test]
fn no_data_notice_completion_clears_inflight_and_progress() {
    use ui::lab::runner::preload_notice;

    let mut cockpit = Cockpit::new();

    update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "inflight must be true after LabRunRequested"
    );

    // Simulate partial progress (spinner animating during preload).
    update(&mut cockpit, Message::LabRunProgress(progress_event(0, 1)));
    assert!(
        cockpit.lab_state.run_progress.is_some(),
        "progress must be Some during preload"
    );

    // Simulate the no-data outcome: tagged sentinel error.
    let no_data_msg = preload_notice::no_data_message("SOL-USD", "2026-04-29", "2026-05-29");
    update(&mut cockpit, Message::LabRunCompleted(Err(no_data_msg)));

    // R3: terminal state — no spinner hang.
    assert!(
        !cockpit.lab_run_inflight,
        "T5 FAIL: lab_run_inflight must be false after no-data LabRunCompleted(Err) — \
         spinner must not hang (R3)"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "T5 FAIL: run_progress must be None after no-data LabRunCompleted(Err) — \
         progress bar must clear (R3)"
    );

    // D-ER-3: notice routes to last_run_notice, NOT last_run_error.
    assert!(
        cockpit.lab_state.last_run_notice.is_some(),
        "T5 FAIL: last_run_notice must be Some after no-data outcome — \
         muted notice must be set (D-ER-3). \
         Actual: {:?}",
        cockpit.lab_state.last_run_notice
    );
    assert!(
        cockpit.lab_state.last_run_error.is_none(),
        "T5 FAIL: last_run_error must be None for no-data outcome — \
         Run button must NOT show Failed (R3, D-ER-3). \
         Actual: {:?}",
        cockpit.lab_state.last_run_error
    );
}
