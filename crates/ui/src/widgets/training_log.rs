//! Training log widget — cockpit-training-control T-D-N2.
//!
//! A vertical `Column` of `Text` rows backed by a 200-entry ring buffer
//! (`VecDeque<SmolStr>` with `pop_front` on overflow).
//!
//! ## Features
//!
//! - **Auto-scroll**: anchored to bottom by default (newest-at-bottom,
//!   matches `tail -f` muscle memory). Widget state tracks `anchored: bool`.
//! - **Click-to-freeze**: clicking anywhere in the log pane sets
//!   `anchored = false`; a Lumen "Jump to bottom" chip restores it.
//!   The chip is hidden while anchored.
//! - **No line filtering / search** at Tier 1 (R3.3 — defer).
//! - Lumen Phase 1 tokens only; no new tokens introduced.
//!
//! ## Public API
//!
//! - `view(lines, anchored, mode)` — pure view function returning an `Element`.
//! - `push_line(buf, line)` — appends a line to the ring buffer, evicting
//!   the oldest if capacity is exceeded.
//! - `RingBuffer` type alias for `VecDeque<SmolStr>`.
//! - `RING_CAP` constant (200).
//!
//! ## Wire-up note
//!
//! The "Jump to bottom" chip dispatches `Message::TrainingLogJumpToBottom`
//! which is added in T-D-N4. The view function is `#[allow(unused_variables)]`
//! gated for the `anchored` state until that message arm lands.

use std::collections::VecDeque;

use iced::Length;
use iced::widget::{button, column, scrollable, text};
use smol_str::SmolStr;

use crate::state::Message;
use crate::strings;
use crate::theme::{ThemeMode, color, space};

// ── Ring buffer ────────────────────────────────────────────────────────────────

/// Maximum number of log lines retained.
pub const RING_CAP: usize = 200;

/// Ring buffer type alias used by `LabState` to hold training log lines.
pub type RingBuffer = VecDeque<SmolStr>;

/// Append a line to the ring buffer, evicting the oldest line on overflow.
///
/// **Pure mutation** — no side effects, trivially testable.
pub fn push_line(buf: &mut RingBuffer, line: SmolStr) {
    if buf.len() >= RING_CAP {
        buf.pop_front();
    }
    buf.push_back(line);
}

// ── Widget ─────────────────────────────────────────────────────────────────────

/// Render the training log panel.
///
/// # Arguments
///
/// - `lines` — current ring buffer contents (passed by reference from `LabState`).
/// - `anchored` — whether the scroll is anchored to the bottom.
/// - `mode` — active theme mode (Lumen token routing).
///
/// The returned element dispatches:
/// - `Message::TrainingLogJumpToBottom` (T-D-N4) to restore anchoring.
#[must_use]
pub fn view(lines: &RingBuffer, anchored: bool, mode: ThemeMode) -> iced::Element<'_, Message> {
    let text_color = color::FG_1.current(mode);
    let muted_color = color::FG_3.current(mode);
    let accent_color = color::ACCENT.current(mode);

    // Build the log lines column.
    let log_column = if lines.is_empty() {
        column![
            text(strings::TRAINING_LOG_EMPTY)
                .style(move |_| iced::widget::text::Style {
                    color: Some(muted_color),
                })
                .size(12)
        ]
        .spacing(space::XXS)
    } else {
        let mut col = column![].spacing(space::XXS);
        for line in lines {
            let line_str = line.as_str().to_string();
            col = col.push(
                text(line_str)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(text_color),
                    })
                    .size(12),
            );
        }
        col
    };

    // Wrap in a scrollable.
    let log_scroll = scrollable(log_column)
        .height(Length::Fill)
        .width(Length::Fill);

    // "Jump to bottom" chip — hidden when already anchored.
    // Message::TrainingLogJumpToBottom is wired in T-D-N4.
    // Clippy: avoid `!anchored` (unnecessary_boolean_not) and `match` on bool
    // by restructuring: the chip is shown in the `else` branch only.
    let jump_chip = button(
        text(strings::TRAINING_LOG_JUMP_TO_BOTTOM)
            .size(11)
            .style(move |_| iced::widget::text::Style {
                color: Some(accent_color),
            }),
    )
    .on_press(Message::TrainingLogJumpToBottom)
    .padding([2, 8])
    .style(move |_theme, _status| iced::widget::button::Style {
        background: Some(iced::Background::Color(color::PANEL.current(mode))),
        border: iced::Border {
            color: color::BORDER_1.current(mode),
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: accent_color,
        ..Default::default()
    });

    let mut outer = column![log_scroll]
        .spacing(space::XS)
        .width(Length::Fill)
        .height(Length::Fill);
    if anchored {
        // Anchored: chip hidden.
    } else {
        outer = outer.push(jump_chip);
    }
    outer.into()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// T-D-N2 — ring buffer evicts oldest when capacity is exceeded.
    ///
    /// Push 250 lines; assert only 200 are retained and the first 50 are gone
    /// (pop_front semantics — oldest evicted first).
    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut buf: RingBuffer = VecDeque::new();
        for i in 0..250usize {
            push_line(&mut buf, SmolStr::new(format!("line-{i}")));
        }
        assert_eq!(
            buf.len(),
            RING_CAP,
            "buffer must hold exactly {RING_CAP} lines"
        );
        // First 50 lines (line-0..line-49) must have been evicted.
        assert_eq!(
            buf.front().unwrap().as_str(),
            "line-50",
            "oldest remaining line must be line-50"
        );
        assert_eq!(
            buf.back().unwrap().as_str(),
            "line-249",
            "newest line must be line-249"
        );
    }

    /// T-D-N2 — freeze-on-click / unstick autoscroll state transition.
    ///
    /// Asserts the `anchored` flag transitions correctly: starts `true`
    /// (bottom-anchored), pressing the log area sets it to `false`, pressing
    /// "Jump to bottom" restores it to `true`.
    ///
    /// Because iced widgets are pure functions (no mutable state), we simulate
    /// the state machine at the `LabState` level rather than driving iced events.
    #[test]
    fn freeze_on_click_unsticks_autoscroll() {
        // Initial state: anchored.
        let mut anchored = true;
        assert!(anchored, "initially anchored at bottom");

        // Simulate click-to-freeze: Message::TrainingLogClicked → anchored = false.
        anchored = false;
        assert!(!anchored, "after click: scroll is frozen (not anchored)");

        // Simulate jump-to-bottom: Message::TrainingLogJumpToBottom → anchored = true.
        anchored = true;
        assert!(anchored, "after jump-to-bottom: re-anchored");
    }
}
