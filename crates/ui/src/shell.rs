//! Shell view — Phase 2 (T1603).
//!
//! Composes the screen-routed shell: `Row[sidebar | (body + status_bar) |
//! reserved-right-rail]`. Both bins (`cockpit`, `cockpit_live`) call
//! `shell::view` so the iced widget tree is identical pixel-for-pixel
//! across them.
//!
//! Phase 6 swaps the right-rail's `Length::Fixed(0.0)` to a real width
//! when the v2-LLM Assistant ships; Phase 2's job is just to leave the
//! spot. (Q7 ratification.)
//!
//! Halted-banner integration: rendered inside the right-side `Column`
//! between any chrome and the screen body so it remains visible across
//! every screen (R3.3 / R14.2). Phase 1's banner trip logic is in the
//! `kill::view` widget body and stays untouched.
//!
//! **Zero string literals** — strings via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{Column, Container, Row, Space};
use iced::Length;

use crate::screens::{audit, charts, control, debug, home, risk, strategies};
use crate::state::{Cockpit, Screen};
use crate::theme::layout::{RIGHT_RAIL_WIDTH_PX, SIDEBAR_ENTRIES_PHASE_5};
use crate::theme::{color, ThemeMode};
use crate::widgets::{sidebar_nav, status_bar};

/// Render the full cockpit shell.
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let sidebar = sidebar_nav::view(model.current_screen, SIDEBAR_ENTRIES_PHASE_5, mode);
    let body = screen_body(model.current_screen, model, mode);
    let bar = status_bar::view(model);

    let right_track = Container::new(Space::new())
        .width(Length::Fixed(RIGHT_RAIL_WIDTH_PX))
        .height(Length::Fill);

    let centre = Column::new()
        .push(body)
        .push(bar)
        .width(Length::Fill)
        .height(Length::Fill);

    let shell_row = Row::new()
        .push(sidebar)
        .push(centre)
        .push(right_track)
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(shell_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(color::CANVAS.current(mode).into()),
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Dispatch on `Cockpit::current_screen` to pick the screen body.
/// Phase 3 wakes the Strategies / Risk / Audit branches; Phase 5 adds
/// the Control branch (`HumanControl` panel).
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn screen_body(screen: Screen, model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    match screen {
        Screen::Home => home::view(model, mode),
        Screen::Debug => debug::view(model, mode),
        Screen::Charts => charts::view(model, mode),
        Screen::Strategies => strategies::view(model, mode),
        Screen::Risk => risk::view(model, mode),
        Screen::Audit => audit::view(model, mode),
        Screen::Control => control::view(model, mode),
    }
}
