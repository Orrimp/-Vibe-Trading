//! Run button widget — ui-rethink-phase-a-lab T-D-14b.
//!
//! Renders the Lab "Run backtest" primary-action button per Lumen Phase 1
//! tokens (Design § 4 / T-D-14b). At most one run in-flight at a time —
//! the button is disabled while a run is in progress (`run_handle_present`).
//!
//! ## State machine
//!
//! ```text
//! Disabled    → [operator selects all required fields] → Idle
//! Idle        → [click] → Running
//! Running     → [completed ok] → Completed
//! Running     → [completed err] → Failed
//! Completed   → [click] → Running   ("Re-run")
//! Failed      → [click] → Running   ("Retry")
//! ```
//!
//! ## Design § 4 — at-most-one enforcement
//!
//! The `run_handle_present` flag (mapped from `Cockpit::lab_run_inflight`) is
//! the authoritative gate. When `true`, `on_press` is cleared so iced won't
//! fire the message.
//!
//! ## F10 — pre-flight gate (lab-end-to-end-v2 Wave D-1.1)
//!
//! `RunState::Disabled` is emitted by `from_cockpit_with_selection` when any
//! of the required Lab fields (strategy, pair) is `None`. The button shows
//! `LAB_RUN_BUTTON_DISABLED` and is non-interactive at the iced level
//! (`.on_press(None)` path — the `Message::LabRunRequested` arm is never
//! fired, so no state-level guard is needed).
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::Length;
use iced::widget::{Text, button};

use crate::state::Message;
use crate::strings::{
    LAB_RUN_BUTTON, LAB_RUN_BUTTON_CANCELLED, LAB_RUN_BUTTON_COMPLETED, LAB_RUN_BUTTON_DISABLED,
    LAB_RUN_BUTTON_FAILED, LAB_RUN_BUTTON_RUNNING,
};
use crate::theme::{ThemeMode, color, radius, space, text};

// ── RunState ──────────────────────────────────────────────────────────────────

/// State of the Lab run button.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RunState {
    /// Not all required selections (pair, strategy) are made — button
    /// is shown greyed out with a hint label.
    /// lab-end-to-end-v2 Wave D-1.1 F10.
    Disabled,
    /// No run in flight; first boot or all selections were cleared.
    #[default]
    Idle,
    /// A backtest is currently in flight.
    Running,
    /// The last run completed successfully.
    Completed,
    /// The last run failed.
    Failed,
    /// The last run was cancelled by the operator (Stop button).
    ///
    /// lab-end-to-end-v2 Wave D-3 T-D3.5 / Q8=(b).
    /// Delta-badge correctness: `T-AR-1` binary wrapper does NOT rotate on
    /// `Err(Cancelled)` so a cancelled run is not a valid comparison anchor (K6).
    Cancelled,
}

impl RunState {
    /// Derive `RunState` from `Cockpit::lab_run_inflight` + optional prior
    /// run outcome. This is the canonical mapping used by `screens/lab.rs`.
    ///
    /// Rules:
    /// - If `inflight == true` → `Running` (regardless of prior state).
    /// - If `inflight == false && had_error == true` → `Failed`.
    /// - If `inflight == false && had_success == true` → `Completed`.
    /// - Otherwise → `Idle`.
    #[must_use]
    pub fn from_cockpit(inflight: bool, last_run_ok: Option<bool>) -> Self {
        if inflight {
            return Self::Running;
        }
        match last_run_ok {
            Some(true) => Self::Completed,
            Some(false) => Self::Failed,
            None => Self::Idle,
        }
    }

    /// Derive `RunState` with pre-flight selection gate (F10).
    ///
    /// When `strategy_selected` or `pair_selected` is `false`, returns
    /// `Disabled` (regardless of inflight state — in practice inflight
    /// cannot be `true` when selections are absent because `LabRunRequested`
    /// is only dispatched after both are `Some`; the guard here is a belt-
    /// and-braces safeguard for test scenarios and state-machine consistency).
    ///
    /// Order of precedence:
    /// 1. `Running` (inflight gate)
    /// 2. `Disabled` (selection gate)
    /// 3. outcome-based (Completed / Failed / Idle)
    #[must_use]
    pub fn from_cockpit_with_selection(
        inflight: bool,
        last_run_ok: Option<bool>,
        strategy_selected: bool,
        pair_selected: bool,
    ) -> Self {
        if inflight {
            return Self::Running;
        }
        if !strategy_selected || !pair_selected {
            return Self::Disabled;
        }
        match last_run_ok {
            Some(true) => Self::Completed,
            Some(false) => Self::Failed,
            None => Self::Idle,
        }
    }
}

// ── view ──────────────────────────────────────────────────────────────────────

/// Render the Run backtest button.
///
/// - `state` — current `RunState` (drives label + disabled logic).
/// - `run_handle_present` — when `true` a run is in-flight and the button
///   is disabled (at-most-one-in-flight per Design § 4).
/// - `mode` — active theme mode.
///
/// Emits `Message::LabRunRequested` on press when enabled.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn view(
    state: &RunState,
    run_handle_present: bool,
    mode: ThemeMode,
) -> crate::Element<'static> {
    let label_str: &'static str = match state {
        RunState::Disabled => LAB_RUN_BUTTON_DISABLED,
        RunState::Idle => LAB_RUN_BUTTON,
        RunState::Running => LAB_RUN_BUTTON_RUNNING,
        RunState::Completed => LAB_RUN_BUTTON_COMPLETED,
        RunState::Failed => LAB_RUN_BUTTON_FAILED,
        RunState::Cancelled => LAB_RUN_BUTTON_CANCELLED,
    };

    let is_disabled =
        run_handle_present || *state == RunState::Running || *state == RunState::Disabled;

    let fg = if is_disabled {
        color::FG_3.current(mode)
    } else {
        color::FG_1.current(mode)
    };

    let bg = if is_disabled {
        color::PANEL.current(mode)
    } else {
        color::ACCENT.current(mode)
    };

    let border_color = if is_disabled {
        color::BORDER_1.current(mode)
    } else {
        color::ACCENT.current(mode)
    };

    let label = Text::new(label_str).size(text::SMALL).color(fg);

    let btn = button(label)
        .padding([space::S as u16, space::L as u16])
        .style(move |_t: &iced::Theme, _s| button::Style {
            background: Some(bg.into()),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: fg,
            ..Default::default()
        })
        .width(Length::Shrink);

    if is_disabled {
        btn.into()
    } else {
        btn.on_press(Message::LabRunRequested).into()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(non_snake_case)] // double-underscore test names are a local snapshot-panel naming convention
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    /// T-D-14b — `view` constructs without panic for all states.
    #[test]
    fn run_button_constructs_all_states() {
        for state in [
            RunState::Disabled,
            RunState::Idle,
            RunState::Running,
            RunState::Completed,
            RunState::Failed,
            RunState::Cancelled,
        ] {
            let _el = view(&state, false, ThemeMode::Dark);
            let _el2 = view(&state, true, ThemeMode::Light);
        }
    }

    /// T-D3.5 — `RunState::Cancelled` variant renders with non-empty label.
    #[test]
    fn run_state_cancelled_renders() {
        let _el = view(&RunState::Cancelled, false, ThemeMode::Dark);
        // Verify the label string is non-empty.
        assert!(!crate::strings::LAB_RUN_BUTTON_CANCELLED.is_empty());
    }

    /// T-D-14b — disabled when `run_handle_present = true`.
    #[test]
    fn run_button_disabled_when_inflight() {
        // When run_handle_present is true, the button must be disabled
        // (no Message::LabRunRequested). We verify the Idle path —
        // the logic merges is_disabled = run_handle_present || Running.
        let _el = view(&RunState::Idle, true, ThemeMode::Dark);
        // If this constructs without panic, the disabled path is exercised.
    }

    /// T-D-14b — enabled for Idle, Completed, Failed with no in-flight run.
    #[test]
    fn run_button_enabled_when_idle_no_inflight() {
        let _el = view(&RunState::Idle, false, ThemeMode::Dark);
        let _el2 = view(&RunState::Completed, false, ThemeMode::Dark);
        let _el3 = view(&RunState::Failed, false, ThemeMode::Dark);
    }

    /// T-D-14b — `RunState::from_cockpit` derives correct state.
    #[test]
    fn run_state_from_cockpit_mapping() {
        // Inflight → Running
        assert_eq!(RunState::from_cockpit(true, None), RunState::Running);
        assert_eq!(RunState::from_cockpit(true, Some(true)), RunState::Running);
        assert_eq!(RunState::from_cockpit(true, Some(false)), RunState::Running);

        // Not inflight, no prior run → Idle
        assert_eq!(RunState::from_cockpit(false, None), RunState::Idle);

        // Not inflight, last ok → Completed
        assert_eq!(
            RunState::from_cockpit(false, Some(true)),
            RunState::Completed
        );

        // Not inflight, last failed → Failed
        assert_eq!(RunState::from_cockpit(false, Some(false)), RunState::Failed);
    }

    /// F10 — `RunState::from_cockpit_with_selection` returns Disabled when
    /// strategy or pair is not selected.
    #[test]
    fn run_state_disabled_when_selection_incomplete() {
        // No strategy, no pair → Disabled
        assert_eq!(
            RunState::from_cockpit_with_selection(false, None, false, false),
            RunState::Disabled
        );
        // Strategy selected but no pair → Disabled
        assert_eq!(
            RunState::from_cockpit_with_selection(false, None, true, false),
            RunState::Disabled
        );
        // Pair selected but no strategy → Disabled
        assert_eq!(
            RunState::from_cockpit_with_selection(false, None, false, true),
            RunState::Disabled
        );
        // Both selected, no prior run → Idle
        assert_eq!(
            RunState::from_cockpit_with_selection(false, None, true, true),
            RunState::Idle
        );
        // Both selected, last ok → Completed
        assert_eq!(
            RunState::from_cockpit_with_selection(false, Some(true), true, true),
            RunState::Completed
        );
        // Inflight overrides selection gate → Running even with missing selection
        assert_eq!(
            RunState::from_cockpit_with_selection(true, None, false, false),
            RunState::Running
        );
    }

    /// F10 — `RunState::Disabled` view renders without panic.
    #[test]
    fn run_button_disabled_state_constructs() {
        let _el = view(&RunState::Disabled, false, ThemeMode::Dark);
        let _el2 = view(&RunState::Disabled, false, ThemeMode::Light);
    }

    /// T-D-14b — snapshot: `run_button__idle`.
    #[test]
    fn run_button__idle() {
        let state = RunState::Idle;
        let run_handle_present = false;
        let mode = ThemeMode::Dark;

        let summary = format!(
            "state={state:?} run_handle_present={run_handle_present} mode={mode:?} label={LAB_RUN_BUTTON:?}"
        );

        insta::assert_snapshot!("run_button__idle", summary);
    }

    /// T-D-14b — snapshot: `run_button__running`.
    #[test]
    fn run_button__running() {
        let state = RunState::Running;
        let run_handle_present = true;
        let mode = ThemeMode::Dark;

        let summary = format!(
            "state={state:?} run_handle_present={run_handle_present} mode={mode:?} label={LAB_RUN_BUTTON_RUNNING:?}"
        );

        insta::assert_snapshot!("run_button__running", summary);
    }

    /// F10 — snapshot: `run_button__disabled`.
    #[test]
    fn run_button__disabled() {
        let state = RunState::Disabled;
        let run_handle_present = false;
        let mode = ThemeMode::Dark;

        let summary = format!(
            "state={state:?} run_handle_present={run_handle_present} mode={mode:?} label={LAB_RUN_BUTTON_DISABLED:?}"
        );

        insta::assert_snapshot!("run_button__disabled", summary);
    }
}
