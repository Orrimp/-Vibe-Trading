//! cockpit-reports-viewer v0.1.0 — markdown body render (D2 / D3).
//!
//! **Lifted verbatim from `bin/viewer.rs:263` (`mod body_render`).** The
//! offline `viewer` bin and the in-cockpit `Screen::Reports` both call this
//! one [`view`] fn, so the body render can never drift between them
//! (AC5). A minimal heading pre-pass — not a full markdown engine — which
//! is the established v0.1.0 contract (D3).

// Per-module clippy allow-pattern (mirrors `screens/baseline.rs:32`): the
// `space::* as u16` padding cast is bounded + safe; this introduces no new
// warning beyond the crate's pre-existing pedantic baseline.
#![allow(clippy::cast_possible_truncation)]

use iced::Length;
use iced::widget::{Column, Text, container, scrollable};

use crate::theme::{ThemeMode, color, space, text};
use crate::viewer::ViewerMessage;

/// Render the report body verbatim with a tiny heading-level pre-pass:
/// `# / ## / ###` lines map to `text::H2` / `text::H3` rows; everything
/// else stays as `text::BODY`. Wrapped in a `scrollable` so a long body
/// scrolls inside the detail pane.
///
/// Returns `Element<'a, ViewerMessage>` (the widget-layer message type);
/// the Reports screen bridges it to `crate::state::Message` via the
/// never-fired no-op arm, exactly as the Baseline screen does for the
/// equity/curve widgets.
#[must_use]
pub fn view(markdown: &str, mode: ThemeMode) -> iced::Element<'_, ViewerMessage> {
    let mut col = Column::new().spacing(space::XS);
    for line in markdown.lines() {
        let stripped = line.trim_start();
        let element = if let Some(rest) = stripped.strip_prefix("### ") {
            Text::new(rest.to_string())
                .size(text::H3)
                .color(color::FG_1.current(mode))
        } else if let Some(rest) = stripped.strip_prefix("## ") {
            Text::new(rest.to_string())
                .size(text::H2)
                .color(color::FG_1.current(mode))
        } else if let Some(rest) = stripped.strip_prefix("# ") {
            Text::new(rest.to_string())
                .size(text::H2)
                .color(color::FG_1.current(mode))
        } else {
            Text::new(line.to_string())
                .size(text::BODY)
                .color(color::FG_2.current(mode))
        };
        col = col.push(element);
    }
    scrollable(container(col).padding(space::S as u16).width(Length::Fill))
        .height(Length::Fill)
        .into()
}
