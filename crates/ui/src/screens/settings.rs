//! Settings rollup screen — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! Three-tab chrome at top of body (Design § A10 / R4.2); each tab body
//! renders the existing screen unchanged (R4.4 — H5 guard):
//!
//! - `SettingsTab::Risk` → `screens::risk::view(model, mode)`
//! - `SettingsTab::Control` → `screens::control::view(model, mode)`
//! - `SettingsTab::Debug` → `screens::debug::view(model, mode)`
//!
//! Active tab is `Cockpit::settings_active_tab` (default `Risk` per Q2a).
//! No persistence across boots in Phase C (R4.5).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::Length;
use iced::widget::{Column, Container};

use crate::state::{Cockpit, SettingsTab};
use crate::theme::{ThemeMode, space};
use crate::widgets::settings_tabs;

/// Render the Settings rollup screen body (R4.1).
///
/// Composition: `Column[ settings_tabs::view(active, mode), <tab_body> ]`
/// where `<tab_body>` dispatches on `model.settings_active_tab`.
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let tab_strip = settings_tabs::view(model.settings_active_tab, mode);

    let tab_body: crate::Element<'_> = match model.settings_active_tab {
        SettingsTab::Risk => crate::screens::risk::view(model, mode),
        SettingsTab::Control => crate::screens::control::view(model, mode),
        SettingsTab::Debug => crate::screens::debug::view(model, mode),
    };

    let content = Column::new()
        .spacing(space::S)
        .push(tab_strip)
        .push(tab_body)
        .width(Length::Fill)
        .height(Length::Fill);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(space::L as u16)
        .into()
}
