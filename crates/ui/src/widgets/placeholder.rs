//! Placeholder widget — Phase A (ui-rethink-phase-a-lab T-D-3).
//!
//! Renders an empty-state card with a single centred sentence pointing
//! the operator at the future phase that will fill this route.
//!
//! **Why a dedicated widget instead of an inline `frame::empty_state`:**
//! the placeholder body shape (Tier-2 surface, full-screen centre,
//! one sentence, no action affordance) is reused for five routes
//! (Compare / Memory / Models / Trail / Settings) — extracting it keeps
//! `shell.rs` call sites to a one-liner and makes the placeholder visually
//! consistent without duplicating the four style closures.
//!
//! **Zero hex colours** — tokens via `crate::theme`.
//! **Zero string literals** — all copy comes from `crate::strings`;
//! callers pass a `&'static str` constant from that module.

use iced::widget::{container, Column, Container, Text};
use iced::Length;

use crate::theme::{color, radius, space, text, ThemeMode};

/// Render a full-body placeholder card for a not-yet-implemented route.
///
/// `title` — a `&'static str` constant from `crate::strings` (e.g.
/// `strings::COMPARE_PLACEHOLDER`). It is rendered as centred `BODY`-
/// sized text on a `PANEL_RAISED` tier-2 surface.
///
/// `mode` — the active `ThemeMode`; forwarded to all token lookups.
///
/// The card fills the entire body allocation (`Length::Fill` on both
/// axes) so it behaves identically regardless of which screen slot it
/// occupies.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn view(title: &'static str, mode: ThemeMode) -> crate::Element<'static> {
    let label = Text::new(title)
        .size(text::BODY)
        .color(color::FG_3.current(mode));

    let card = Container::new(Column::new().push(label))
        .padding(space::XL as u16)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    Container::new(card)
        .padding(space::L as u16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    /// T-D-3 — placeholder view compiles and doesn't panic on construction.
    /// We can't easily assert visual output without a renderer, but we can
    /// at least verify the element is produced without panic.
    #[test]
    fn placeholder_view_constructs() {
        use crate::theme::ThemeMode;
        // If this panics the test fails.
        let _el = super::view(crate::strings::COMPARE_PLACEHOLDER, ThemeMode::Dark);
        let _el2 = super::view(crate::strings::SETTINGS_PLACEHOLDER, ThemeMode::Light);
    }
}
