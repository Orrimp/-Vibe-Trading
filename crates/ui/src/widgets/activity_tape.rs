//! Activity tape region widget — cockpit-activity-status-bar v0.1.0 (T-D-N6).
//!
//! Renders the activity tape that sits to the LEFT of the server-time field
//! inside the bottom status bar.
//!
//! Layout per R2.2: `Row[dot · label · elapsed | dot · label · elapsed | … | +N more]`
//! Max 3 visible activities + overflow chip (R3.1).
//!
//! **Zero string literals** — all copy from [`crate::strings`].
//! **Zero new Lumen tokens** — colours from `color::ACCENT`, `color::DOWN_500`,
//! `color::FG_3` (R-NR.3).
//!
//! Constraints enforced by this module:
//! - R2.3: 200 ms render-floor — activities younger than 200 ms are hidden.
//! - R3.1: max 3 visible + "+N more" overflow chip.
//! - Q5=(a): failed activities render in `color::DOWN_500` (red).
//! - R7.2: zero inline string literals.

use std::time::{Duration, Instant};

use iced::widget::{Container, Row, Space, Text};
use iced::{Alignment, Length};

use agent::{ActivityKind, ActivityOutcome};

use crate::lab::activity::ActivityTape;
use crate::strings::{
    ACTIVITY_KIND_AUDIT_LABEL, ACTIVITY_KIND_LAB_RUN_LABEL, ACTIVITY_KIND_TRAINING_LABEL,
    ACTIVITY_KIND_YAHOO_LABEL, ACTIVITY_TAPE_MORE_PREFIX, ACTIVITY_TAPE_MORE_SUFFIX,
};
use crate::theme::{ThemeMode, color, space, text};

// ── Constants (R7.1) ─────────────────────────────────────────────────────────

/// R3.1 — maximum number of simultaneous visible activity slots.
const ACTIVITY_TAPE_MAX_VISIBLE: usize = 3;

/// R2.3 — minimum age before an activity is rendered (prevents flicker on
/// sub-frame activities such as fast cache hits).
const ACTIVITY_TICK_RENDER_FLOOR_MS: u64 = 200;

/// R2.5 — fixed-width budget per activity slot in logical pixels.
const ACTIVITY_SLOT_WIDTH_PX: f32 = 96.0;

/// Diameter of the activity indicator dot in logical pixels.
const DOT_SIZE: f32 = 6.0;

/// Status bar is always dark mode (matches `status_bar.rs`).
const MODE: ThemeMode = ThemeMode::Dark;

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the activity tape region.
///
/// Called from `widgets::status_bar::view` and inserted to the LEFT of the
/// server-time field per Q2=(a) placement.
///
/// The function is pure — it reads `&ActivityTape` + `Instant::now()` and
/// emits an `Element<Message>`. No async work, no allocations except the
/// iced element tree.
#[must_use]
pub fn view(tape: &ActivityTape) -> crate::Element<'_> {
    let now = Instant::now();
    let render_floor = Duration::from_millis(ACTIVITY_TICK_RENDER_FLOOR_MS);

    // Filter: only show activities older than the render floor.
    let renderable: Vec<_> = tape
        .visible()
        .iter()
        .filter(|s| {
            // Always show red-held failed rows (they are already past the floor).
            if s.red_hold_until.is_some() {
                return true;
            }
            // For in-flight rows, apply the 200 ms floor.
            now.duration_since(s.started_at) >= render_floor
        })
        .collect();

    if renderable.is_empty() {
        // R2.7 — empty tape: a blank space, no "no activity" label.
        return Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into();
    }

    let visible_count = renderable.len().min(ACTIVITY_TAPE_MAX_VISIBLE);
    let overflow = renderable.len().saturating_sub(ACTIVITY_TAPE_MAX_VISIBLE);

    let fg3 = color::FG_3.current(MODE);

    let mut row = Row::new().spacing(space::S).align_y(Alignment::Center);

    for state in renderable.iter().take(visible_count) {
        // Determine dot colour and text colour based on activity state.
        let is_failed =
            matches!(&state.outcome, Some(ActivityOutcome::Failed(_))) || state.is_red_held(now);

        let dot_color = if is_failed {
            color::DOWN_500.current(MODE)
        } else {
            color::ACCENT.current(MODE)
        };
        let text_color = if is_failed {
            color::DOWN_500.current(MODE)
        } else {
            fg3
        };

        // ── Dot ──
        let dot = Container::new(
            Space::new()
                .width(Length::Fixed(DOT_SIZE))
                .height(Length::Fixed(DOT_SIZE)),
        )
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(dot_color.into()),
            border: iced::Border {
                radius: crate::theme::radius::PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        // ── Kind label ──
        let kind_label = activity_kind_label(state.kind);
        let elapsed = format_elapsed(now.duration_since(state.started_at));

        // Combine label + elapsed into one compact string.
        let label_str = format!("{kind_label} {elapsed}");

        let label = Text::new(label_str).size(text::MICRO).color(text_color);

        let slot = Row::new()
            .spacing(space::XS)
            .align_y(Alignment::Center)
            .push(dot)
            .push(label);

        let slot_container = Container::new(slot).width(Length::Fixed(ACTIVITY_SLOT_WIDTH_PX));

        row = row.push(slot_container);
    }

    // ── Overflow chip ──
    if overflow > 0 {
        let chip_str = format!("{ACTIVITY_TAPE_MORE_PREFIX}{overflow}{ACTIVITY_TAPE_MORE_SUFFIX}");
        let chip = Text::new(chip_str).size(text::MICRO).color(fg3);
        row = row.push(chip);
    }

    row.into()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Map `ActivityKind` to its operator-facing label prefix (R7.2 — no inline literals).
///
/// `pub(crate)` so the T-D-N4 test in this module can call it directly without
/// going through the full `view` rendering path.
pub(crate) fn activity_kind_label(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::YahooPreload => ACTIVITY_KIND_YAHOO_LABEL,
        ActivityKind::LabRun => ACTIVITY_KIND_LAB_RUN_LABEL,
        // Forward-listed variant (v0.1.1 — LLM forecaster) shares the Training label.
        ActivityKind::Training | ActivityKind::LlmCall => ACTIVITY_KIND_TRAINING_LABEL,
        // cockpit-activity-audit-ledger-producer v0.1.0 — now wired (R2.1/Q2=(a)).
        ActivityKind::AuditLedgerWrite => ACTIVITY_KIND_AUDIT_LABEL,
    }
}

/// Format elapsed time into a compact terse string (R2.3 display contract):
/// - `<1s`
/// - `Ns` (1–59 s)
/// - `NmNs` (≥ 60 s)
///
/// silent-quarantine-fix-2026-05-26: delegates to
/// `crate::strings::activity_tape_elapsed_label` so the format-string
/// literals live in the canonical strings module per
/// `consistency::no_inline_user_visible_strings_in_widgets`.
fn format_elapsed(elapsed: Duration) -> String {
    crate::strings::activity_tape_elapsed_label(elapsed)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use agent::{ActivityEvent, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase};

    use crate::lab::activity::ActivityTape;

    use super::{activity_kind_label, view};

    fn make_start_event(id: u64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityId(id),
            kind: ActivityKind::YahooPreload,
            label: format!("label {id}"),
            phase: ActivityPhase::Start { total_units: None },
            ts_ms: 0,
        }
    }

    fn make_failed_event(id: u64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityId(id),
            kind: ActivityKind::LabRun,
            label: format!("label {id}"),
            phase: ActivityPhase::End(ActivityOutcome::Failed("err".to_owned())),
            ts_ms: 0,
        }
    }

    /// Summarise tape visible state for snapshot comparison.
    fn tape_summary(tape: &ActivityTape, label: &str) -> String {
        let visible = tape.visible();
        let in_flight: Vec<String> = visible
            .iter()
            .map(|s| {
                format!(
                    "id={} kind={:?} outcome={} red_held={}",
                    s.id.0,
                    s.kind,
                    if s.outcome.is_some() { "some" } else { "none" },
                    if s.red_hold_until.is_some() {
                        "yes"
                    } else {
                        "no"
                    },
                )
            })
            .collect();
        format!(
            "scenario={label} count={} entries=[{}]",
            visible.len(),
            in_flight.join(", ")
        )
    }

    /// T-D-N6 test 1 — empty tape renders without panic.
    #[test]
    fn widget_renders_empty_state_no_panic() {
        let tape = ActivityTape::new();
        let _element = view(&tape);
        // Snapshot: empty tape.
        let summary = tape_summary(&tape, "empty");
        insta::assert_snapshot!("status_bar__activity_tape_empty", summary);
    }

    /// T-D-N6 test 2 — single in-flight activity renders without panic.
    #[test]
    fn widget_renders_one_inflight_no_panic() {
        let mut tape = ActivityTape::new();
        tape.apply(make_start_event(1));
        let _element = view(&tape);
        // Snapshot: one in-flight row.
        let summary = tape_summary(&tape, "one_inflight");
        insta::assert_snapshot!("status_bar__activity_tape_one_inflight", summary);
    }

    /// T-D-N6 test 3 — five activities → max 3 visible + overflow chip (no panic).
    #[test]
    fn widget_renders_three_plus_overflow_chip() {
        let mut tape = ActivityTape::new();
        for id in 1..=5u64 {
            tape.apply(make_start_event(id));
        }
        let _element = view(&tape);
        // Snapshot: 5 activities total (widget shows 3 + "+2 more").
        let summary = tape_summary(&tape, "three_plus_overflow");
        insta::assert_snapshot!("status_bar__activity_tape_three_plus_overflow", summary);
    }

    /// T-D-N6 test 4 — failed activity (in red 3-second hold) renders without panic.
    #[test]
    fn widget_renders_failed_in_red() {
        let mut tape = ActivityTape::new();
        tape.apply(make_start_event(7));
        tape.apply(make_failed_event(7));
        let _element = view(&tape);
        // Snapshot: one failed row in the red-hold window.
        let summary = tape_summary(&tape, "failed_red");
        insta::assert_snapshot!("status_bar__activity_tape_failed_red", summary);
    }

    /// T-D-N4 new test — `ActivityKind::AuditLedgerWrite` label renders correctly.
    ///
    /// Asserts the `activity_kind_label` function maps `AuditLedgerWrite` to
    /// `ACTIVITY_KIND_AUDIT_LABEL` ("Audit") — per R2.1 / Q2=(a) redacted label.
    #[test]
    fn audit_ledger_label_renders_correctly() {
        use crate::strings::ACTIVITY_KIND_AUDIT_LABEL;

        let label = activity_kind_label(ActivityKind::AuditLedgerWrite);
        assert_eq!(
            label,
            ACTIVITY_KIND_AUDIT_LABEL,
            "AuditLedgerWrite must map to ACTIVITY_KIND_AUDIT_LABEL"
        );

        // Also verify other kinds are not accidentally mapped to "Audit".
        assert_ne!(
            activity_kind_label(ActivityKind::LabRun),
            ACTIVITY_KIND_AUDIT_LABEL
        );
        assert_ne!(
            activity_kind_label(ActivityKind::Training),
            ACTIVITY_KIND_AUDIT_LABEL
        );
        assert_ne!(
            activity_kind_label(ActivityKind::YahooPreload),
            ACTIVITY_KIND_AUDIT_LABEL
        );

        // The AuditLedgerWrite activity tape entry renders without panic.
        let mut tape = ActivityTape::new();
        tape.apply(ActivityEvent {
            id: ActivityId(99),
            kind: ActivityKind::AuditLedgerWrite,
            label: "Audit: 42 writes".to_owned(),
            phase: ActivityPhase::Start { total_units: None },
            ts_ms: 0,
        });
        let _element = view(&tape);
        // Summary check: one AuditLedgerWrite in the tape.
        let summary = tape_summary(&tape, "audit_ledger_write");
        insta::assert_snapshot!("status_bar__activity_tape_audit_ledger_write", summary);
    }
}
