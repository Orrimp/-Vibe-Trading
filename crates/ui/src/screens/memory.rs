#![allow(clippy::cast_possible_truncation)]
//! Memory screen — Phase F (ui-rethink-phase-f-memory-models-assistant R1.1-R1.4).
//!
//! Body composition (top to bottom):
//!
//! 1. **Toolbar row** — `Cards` / `Cluster` mode toggle (Cluster disabled at
//!    v0.1.0 per R1.2 bullet 3) + optional strategy/symbol filter chips.
//! 2. **Main area** — cards list (left) + optional side-drawer (right) when
//!    `MemoryScreenState::drawer_open` is `Some(card_id)` (Q5=(b)).
//! 3. **Empty-state** — when `cache` is empty post-hydrate, shows
//!    `MEMORY_EMPTY_STATE` placeholder text (R1.4).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{Button, Column, Container, Row, Scrollable, Text, button};
use iced::{Border, Element, Length};

use crate::memory::drawer;
use crate::memory::state::{LessonCardCard, MemoryViewMode};
use crate::state::{Cockpit, Message};
use crate::strings::{
    MEMORY_CARD_TRAIL_LINK_LABEL, MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP, MEMORY_EMPTY_STATE,
    MEMORY_TOOLBAR_CARDS_LABEL, MEMORY_TOOLBAR_CLUSTER_LABEL,
};
use crate::theme::{ThemeMode, color, radius, space, text};

/// Render the Memory screen body.
///
/// Called by `shell::screen_body` when `current_screen == Screen::Memory`
/// (replacing the Phase A `placeholder::view` route per R1.3).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_, Message> {
    // ── Toolbar ─────────────────────────────────────────────────────────────
    let toolbar = build_toolbar(&model.memory_screen_state.mode, mode);

    // ── Main area ────────────────────────────────────────────────────────────
    let cache = &model.memory_screen_state.cache;
    let drawer_open = model.memory_screen_state.drawer_open.as_deref();

    let body_row: Element<'_, Message> = if cache.is_empty() {
        // Empty-state placeholder (R1.4) — shown until the first hydrate fires.
        Container::new(
            Text::new(MEMORY_EMPTY_STATE)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        // Cards list (left column).
        let cards_list = build_cards_list(cache, drawer_open, mode);

        // Optional side-drawer (right column, Q5=(b)).
        if let Some(open_id) = drawer_open {
            if let Some(card) = cache.iter().find(|c| c.card_id.as_str() == open_id) {
                Row::new()
                    .push(cards_list)
                    .push(drawer::view(card, mode))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                cards_list
            }
        } else {
            cards_list
        }
    };

    Column::new()
        .spacing(space::S)
        .push(toolbar)
        .push(body_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(space::M as u16)
        .into()
}

/// Toolbar: Cards / Cluster mode toggle chips.
///
/// `Cluster` chip is visually disabled at v0.1.0 (R1.2 bullet 3);
/// the `MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP` string is attached
/// as the button label suffix for operator discoverability.
fn build_toolbar(mode_state: &MemoryViewMode, mode: ThemeMode) -> Element<'_, Message> {
    let is_cards = *mode_state == MemoryViewMode::Cards;

    let cards_style = move |_t: &iced::Theme, _s: button::Status| button::Style {
        background: if is_cards {
            Some(color::ACCENT.current(mode).into())
        } else {
            Some(color::PANEL_RAISED.current(mode).into())
        },
        text_color: if is_cards {
            color::FG_1.current(mode)
        } else {
            color::FG_2.current(mode)
        },
        border: Border {
            radius: radius::R2.into(),
            ..Default::default()
        },
        ..Default::default()
    };

    let cards_btn = Button::new(
        Text::new(MEMORY_TOOLBAR_CARDS_LABEL)
            .size(text::SMALL)
            .color(if is_cards {
                color::FG_1.current(mode)
            } else {
                color::FG_2.current(mode)
            }),
    )
    .on_press(Message::MemoryToggleMode(MemoryViewMode::Cards))
    .padding([space::XS as u16, space::S as u16])
    .style(cards_style);

    // Cluster chip — disabled at v0.1.0; no `on_press` so the button is inert.
    let cluster_label =
        format!("{MEMORY_TOOLBAR_CLUSTER_LABEL} {MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP}");
    let cluster_btn = Button::new(
        Text::new(cluster_label)
            .size(text::SMALL)
            .color(color::FG_4.current(mode)),
    )
    .padding([space::XS as u16, space::S as u16])
    .style(move |_t: &iced::Theme, _s: button::Status| button::Style {
        background: Some(color::PANEL_RAISED.current(mode).into()),
        text_color: color::FG_4.current(mode),
        border: Border {
            radius: radius::R2.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    Row::new()
        .spacing(space::S)
        .align_y(iced::alignment::Vertical::Center)
        .push(cards_btn)
        .push(cluster_btn)
        .into()
}

/// Build the scrollable cards list.
fn build_cards_list<'a>(
    cache: &'a [LessonCardCard],
    drawer_open: Option<&str>,
    mode: ThemeMode,
) -> Element<'a, Message> {
    let mut col = Column::new().spacing(space::S);

    for card in cache {
        col = col.push(build_card_row(card, drawer_open, mode));
    }

    let scrollable = Scrollable::new(col)
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Build one memory card row (condensed view).
///
/// Shows: outcome badge | symbol | `closed_at` | pnl | strategy | chevron button.
/// Chevron emits `Message::MemoryOpenDrawer(card_id)` on press (Q5=(b)).
/// The cross-link label `MEMORY_CARD_TRAIL_LINK_LABEL` is used as a hint on
/// the chevron when `close_transaction_id` is present (R6.1).
fn build_card_row<'a>(
    card: &'a LessonCardCard,
    drawer_open: Option<&str>,
    mode: ThemeMode,
) -> Element<'a, Message> {
    let is_open = drawer_open == Some(card.card_id.as_str());

    // Outcome badge colour: Win=green-ish accent, Loss=muted, Scratch=FG_3.
    let outcome_color = match card.outcome_class.as_str() {
        "Win" => color::ACCENT.current(mode),
        _ => color::FG_3.current(mode),
    };

    let outcome = Text::new(card.outcome_class.as_str())
        .size(text::MICRO)
        .color(outcome_color);

    let symbol = Text::new(card.symbol_or_pair.as_str())
        .size(text::SMALL)
        .color(color::FG_1.current(mode));

    let closed = Text::new(card.closed_at.as_str())
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    let pnl = Text::new(card.signed_pnl_display.as_str())
        .size(text::SMALL)
        .color(color::FG_1.current(mode));

    let strategy = Text::new(card.strategy_id.as_str())
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    // Chevron / trail-link label.
    let chevron_label = if card.close_transaction_id.is_some() {
        MEMORY_CARD_TRAIL_LINK_LABEL
    } else {
        "\u{203a}" // ›
    };

    let card_id_clone = card.card_id.clone();
    let chevron_btn = Button::new(
        Text::new(chevron_label)
            .size(text::SMALL)
            .color(color::ACCENT.current(mode)),
    )
    .on_press(Message::MemoryOpenDrawer(card_id_clone))
    .padding([space::XS as u16, space::XS as u16])
    .style(move |_t: &iced::Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: color::ACCENT.current(mode),
            border: Border {
                radius: radius::R2.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let row = Row::new()
        .spacing(space::S)
        .align_y(iced::alignment::Vertical::Center)
        .push(outcome)
        .push(symbol)
        .push(closed)
        .push(pnl)
        .push(strategy)
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(chevron_btn);

    let bg_color = if is_open {
        color::PANEL_SUNKEN.current(mode)
    } else {
        color::PANEL_RAISED.current(mode)
    };

    Container::new(row)
        .width(Length::Fill)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(bg_color.into()),
            border: iced::Border {
                radius: radius::R2.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
