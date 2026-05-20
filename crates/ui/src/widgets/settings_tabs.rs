//! Settings tab-strip widget — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! Three-tab chrome strip at the top of `screens::settings::view`. Each tab
//! button carries `Message::SwitchSettingsTab(<tab>)` and is wrapped in
//! `frame::active_chip` for the T1609 bottom-edge accent rule on the active
//! tab (Design § A10).
//!
//! Tab order: Risk · Control · Debug per Q2a.
//!
//! **No new Lumen tokens.** Reuses `frame::active_chip` + existing button style.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::widget::{Button, Row, Text, button};
use iced::{Border, Length};

use crate::state::{Message, SettingsTab};
use crate::strings::{SETTINGS_TAB_CONTROL, SETTINGS_TAB_DEBUG, SETTINGS_TAB_RISK};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::frame;

/// Render the three-tab strip for the Settings rollup screen (Design § A10).
///
/// Each tab button: label from `strings::SETTINGS_TAB_*`, wrapped in
/// `frame::active_chip` (T1609 bottom-edge accent when `active`). Tab order:
/// Risk · Control · Debug per Q2a.
#[must_use]
pub fn view(active: SettingsTab, mode: ThemeMode) -> crate::Element<'static> {
    let tabs: &[(SettingsTab, &str)] = &[
        (SettingsTab::Risk, SETTINGS_TAB_RISK),
        (SettingsTab::Control, SETTINGS_TAB_CONTROL),
        (SettingsTab::Debug, SETTINGS_TAB_DEBUG),
    ];

    let mut row = Row::new().spacing(space::M);
    for &(tab, label) in tabs {
        let is_active = tab == active;
        let btn = Button::new(Text::new(label).size(text::BODY).color(if is_active {
            color::FG_1.current(mode)
        } else {
            color::FG_2.current(mode)
        }))
        .on_press(Message::SwitchSettingsTab(tab))
        .padding([space::XS as u16, space::M as u16])
        .width(Length::Shrink)
        .style(move |_theme: &iced::Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: if is_active {
                    color::FG_1.current(mode)
                } else {
                    color::FG_2.current(mode)
                },
                border: Border {
                    radius: radius::R2.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        // T1609 active_chip: bottom-edge ACCENT rule when this tab is active.
        row = row.push(frame::active_chip(btn.into(), is_active, mode));
    }

    row.into()
}
