//! Control screen — Phase 5 (T1906).
//!
//! Hosts the `HumanControl` panel as the screen body. Per Phase 5 Q1
//! ratification, `HumanControl` is reachable as the 7th sidebar entry
//! `Screen::Control`. The kill widget migrates here from the `Debug`
//! screen as the bottom action of the `HumanControl` panel (R2.1 / R2.2).
//!
//! **Zero string literals** — the panel's copy lives in
//! `crate::strings::HUMAN_CONTROL_*` / `crate::strings::EXECUTION_MODE_*`
//! / `crate::strings::KILL_*` (preserved per Q12).

use iced::widget::{Column, Container};
use iced::Length;

use crate::state::Cockpit;
use crate::theme::{layout, space, ThemeMode};
use crate::widgets::human_control;

/// Render the Control screen body.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, _mode: ThemeMode) -> crate::Element<'_> {
    let body = Column::new()
        .padding(space::L as u16)
        .spacing(layout::PANEL_OUTER_GAP)
        .push(human_control::view(model))
        .width(Length::Fill)
        .height(Length::Fill);
    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
