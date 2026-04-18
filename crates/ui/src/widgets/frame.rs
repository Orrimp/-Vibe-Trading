//! Shared panel frame — the bordered card every panel sits inside.
//!
//! One helper so panel visuals stay identical and layout code doesn't
//! duplicate. Title and body are composed by the caller; this module only
//! controls the frame padding, the gap between title and body, and the
//! title/body typography.

use iced::widget::{container, Column, Container, Text};
use iced::Element;

use crate::theme::{color, layout, radius, text};

/// Wraps a panel body in the standard frame with a title.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn panel<'a, Message: 'a>(title: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    let header = Text::new(title).size(text::TITLE).color(color::FG);

    let stack = Column::new()
        .push(header)
        .push(body)
        .spacing(layout::PANEL_GAP);

    // `Padding: From<u16>` in iced 0.14; our scale maxes out at 32, so the
    // cast is always lossless. Pedantic clippy wants the explicit allow.
    #[allow(clippy::cast_possible_truncation)]
    let padding = layout::PANEL_PADDING as u16;

    Container::new(stack)
        .padding(padding)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(color::BG_ELEV.into()),
            border: iced::Border {
                color: color::BORDER,
                width: 1.0,
                radius: radius::MEDIUM.into(),
            },
            text_color: Some(color::FG),
            ..Default::default()
        })
        .into()
}

/// Small helper for body text in the muted foreground.
#[must_use]
pub fn muted_body<'a, Message: 'a>(t: &'a str) -> Element<'a, Message> {
    Text::new(t).size(text::BODY).color(color::FG_MUTED).into()
}

/// Helper for a red-tinted error row inside a panel body.
#[must_use]
pub fn error_body<'a, Message: 'a>(prefix: &'a str, detail: &'a str) -> Element<'a, Message> {
    Text::new(format!("{prefix}{detail}"))
        .size(text::BODY)
        .color(color::NEG)
        .into()
}

/// Helper for a caption-sized column header row.
#[must_use]
pub fn col_header<'a, Message: 'a>(t: &'a str) -> Element<'a, Message> {
    Text::new(t)
        .size(text::CAPTION)
        .color(color::FG_MUTED)
        .into()
}
