//! Phase F — Assistant slot view (Lumen Phase 6 wake, Q4=(a) stub-only).
//!
//! When `state.is_open == false` → returns a 0-width `Container<Space>`
//! (byte-identical to today's `right_track` at shell.rs:47-49).
//! When `state.is_open == true` → renders the Phase 6 stub placeholder
//! body (`ASSISTANT_OFFLINE_TITLE` + `ASSISTANT_OFFLINE_BODY` per
//! R3.2(a) + K7 mitigation).

use iced::Length;
use iced::widget::{Column, Container, Space, Text};

use crate::assistant::state::AssistantState;
use crate::state::Message;
use crate::strings::{ASSISTANT_OFFLINE_BODY, ASSISTANT_OFFLINE_TITLE};
use crate::theme::{ThemeMode, color, layout::RIGHT_RAIL_OPEN_WIDTH_PX, radius, space, text};

/// Render the right-rail Assistant slot.
///
/// When `state.is_open == false` this returns a 0-width `Container<Space>`
/// — identical to Phase 2's `Space::new()` in shell.rs:47-49. The
/// `shell::view` caller picks the right-rail width based on `is_open`
/// (K6 Option A), so this function's returned element is always sized
/// by the shell's outer `Length::Fixed(...)` container.
///
/// When `state.is_open == true` this renders the Phase 6 stub placeholder
/// with a heading + body copy (R3.2(a) / K7 mitigation copy).
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn view(state: &AssistantState, mode: ThemeMode) -> crate::Element<'_> {
    if !state.is_open {
        // Closed state — return a zero-fill Space so the shell right_track
        // Container collapses to the RIGHT_RAIL_WIDTH_PX = 0.0 width.
        // Byte-identical to Phase 2's shell.rs:47-49 `Space::new()` body.
        return Container::new(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    // Open state — render stub placeholder (Q4=(a)).
    let title = Text::new(ASSISTANT_OFFLINE_TITLE)
        .size(text::BODY)
        .color(color::FG_1.current(mode));

    let body = Text::new(ASSISTANT_OFFLINE_BODY)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    let content = Column::new()
        .spacing(space::S)
        .push(title)
        .push(body)
        .padding(space::M as u16);

    Container::new(content)
        .width(Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Type alias to suppress the unused import warning on `Message` when
/// the view fn doesn't emit any messages. The Assistant slot at
/// Q4=(a) renders static copy with no interactive affordances
/// (the toggle button is in the status bar, not here).
#[allow(dead_code)]
fn _message_used(_: &Message) {}
