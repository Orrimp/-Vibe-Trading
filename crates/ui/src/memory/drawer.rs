//! Phase F — Memory entry side-drawer widget (Q5=(b)).
//!
//! Mirrors the Phase D `widgets/trail_drawer.rs` body verbatim in pattern.
//! Width is `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`.
//! Dismissal via `Message::MemoryCloseDrawer`.

use iced::Length;
use iced::widget::{Button, Column, Container, Row, Scrollable, Text, button};
use iced::{Border, Element};

use crate::memory::state::LessonCardCard;
use crate::state::Message;
use crate::theme::{ThemeMode, color, layout::RIGHT_RAIL_OPEN_WIDTH_PX, radius, space, text};

const CLOSE_LABEL: &str = "Close";
const DRAWER_TITLE: &str = "Memory Entry";

/// Render the Memory entry side-drawer.
///
/// Width is `RIGHT_RAIL_OPEN_WIDTH_PX` (320.0). Dismissal via
/// `Message::MemoryCloseDrawer`. Cross-link to Trail via
/// `Message::OpenTrailFor(tx_id)` when a `close_transaction_id` is
/// present (R6.1).
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn view<'a>(card: &'a LessonCardCard, mode: ThemeMode) -> Element<'a, Message> {
    let close_btn = Button::new(
        Text::new(CLOSE_LABEL)
            .size(text::SMALL)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::MemoryCloseDrawer)
    .padding([space::XS as u16, space::S as u16])
    .style(move |_theme: &iced::Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: color::FG_1.current(mode),
            border: Border {
                radius: radius::R2.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let header = Row::new()
        .spacing(space::M)
        .push(
            Text::new(DRAWER_TITLE)
                .size(text::SMALL)
                .color(color::ACCENT.current(mode)),
        )
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(close_btn);

    // Build the card detail rows.
    let symbol_row = kv("Symbol", card.symbol_or_pair.as_str(), mode);
    let date_row = kv("Closed", card.closed_at.as_str(), mode);
    let strategy_row = kv("Strategy", card.strategy_id.as_str(), mode);
    let pnl_row = kv("P&L", card.signed_pnl_display.as_str(), mode);
    let outcome_row = kv("Outcome", card.outcome_class.as_str(), mode);

    let mut detail_col = Column::new()
        .spacing(space::XS)
        .push(symbol_row)
        .push(date_row)
        .push(strategy_row)
        .push(pnl_row)
        .push(outcome_row);

    if let Some(note) = &card.note {
        detail_col = detail_col
            .push(iced::widget::Space::new().height(space::S))
            .push(
                Text::new("Lesson")
                    .size(text::MICRO)
                    .color(color::FG_3.current(mode)),
            )
            .push(
                Text::new(note.as_str())
                    .size(text::SMALL)
                    .color(color::FG_1.current(mode)),
            );
    }

    // Memory→Trail cross-link (R6.1 / Q6=(c)).
    if let Some(tx_id) = &card.close_transaction_id {
        let tx_id_clone = tx_id.clone();
        let trail_btn = Button::new(
            Text::new("View in Trail →")
                .size(text::MICRO)
                .color(color::ACCENT.current(mode)),
        )
        .on_press(Message::OpenTrailFor(tx_id_clone))
        .padding([space::XS as u16, space::S as u16])
        .style(
            move |_theme: &iced::Theme, _status: button::Status| button::Style {
                background: None,
                text_color: color::ACCENT.current(mode),
                ..Default::default()
            },
        );
        detail_col = detail_col.push(trail_btn);
    }

    let body: Element<'a, Message> = Scrollable::new(detail_col).height(Length::Fill).into();

    let drawer_col = Column::new()
        .spacing(space::S)
        .push(header)
        .push(body)
        .padding(space::M as u16)
        .height(Length::Fill);

    Container::new(drawer_col)
        .width(Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: iced::border::Radius::default(),
            },
            ..Default::default()
        })
        .into()
}

fn kv<'a>(key: &'static str, value: &'a str, mode: ThemeMode) -> Element<'a, Message> {
    Row::new()
        .spacing(space::XS)
        .push(
            Text::new(key)
                .size(text::MICRO)
                .color(color::FG_3.current(mode)),
        )
        .push(
            Text::new(value)
                .size(text::MICRO)
                .color(color::FG_1.current(mode)),
        )
        .into()
}
