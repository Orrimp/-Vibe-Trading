//! Sidebar nav widget — Phase 2 (T1602), extended Phase A (T-D-3).
//!
//! Renders a fixed-width left rail with one row per `Screen` entry. Each
//! row is a `button` carrying `Message::SwitchScreen(*screen)`, wrapped
//! in `frame::active_row` so the T1507 left-rule applies to the
//! currently-active screen.
//!
//! Phase A adds labels for the seven new `Screen` variants (Lab, Live,
//! Compare, Memory, Models, Trail, Settings) and updates the active-row
//! check to use `Screen::Lab` as the new default.
//!
//! Stateless — `current_screen` lives on `Cockpit`; the widget reads it
//! as a parameter to know which row carries the active rule.
//!
//! **Zero string literals** — labels resolve via `crate::strings`.
//! **Zero hex colours** — tokens come from `crate::theme`.

use iced::widget::{Button, Column, Container, Space, Text, button, container};
use iced::{Border, Length};

use crate::state::{Message, Screen};
use crate::strings::{
    LAB_TITLE, LIVE_TITLE, SIDEBAR_NAV_AUDIT, SIDEBAR_NAV_CHARTS, SIDEBAR_NAV_COMPARE,
    SIDEBAR_NAV_CONTROL, SIDEBAR_NAV_DEBUG, SIDEBAR_NAV_HOME, SIDEBAR_NAV_MEMORY,
    SIDEBAR_NAV_MODELS, SIDEBAR_NAV_RISK, SIDEBAR_NAV_SETTINGS, SIDEBAR_NAV_STRATEGIES,
    TRAIL_TITLE,
};
use crate::theme::{ThemeMode, color, layout, radius, space, text};
use crate::widgets::frame;

/// Stable label for a `Screen`. Pure function over the strings table so
/// snapshot tests can pin the rendered text without going through iced.
#[must_use]
#[allow(deprecated)]
pub const fn label_for(screen: Screen) -> &'static str {
    match screen {
        // Phase A active routes
        Screen::Lab => LAB_TITLE,
        Screen::Live => LIVE_TITLE,
        Screen::Compare => SIDEBAR_NAV_COMPARE,
        Screen::Memory => SIDEBAR_NAV_MEMORY,
        Screen::Models => SIDEBAR_NAV_MODELS,
        Screen::Trail => TRAIL_TITLE,
        Screen::Settings => SIDEBAR_NAV_SETTINGS,
        Screen::Strategies => SIDEBAR_NAV_STRATEGIES,

        // Deprecated aliases
        Screen::Home => SIDEBAR_NAV_HOME,
        Screen::Debug => SIDEBAR_NAV_DEBUG,
        Screen::Charts => SIDEBAR_NAV_CHARTS,
        Screen::Risk => SIDEBAR_NAV_RISK,
        Screen::Audit => SIDEBAR_NAV_AUDIT,
        Screen::Control => SIDEBAR_NAV_CONTROL,
    }
}

/// Render the sidebar nav.
///
/// `entries` is the operator's scan-ordered nav list. Phase A passes
/// `SIDEBAR_ENTRIES_PHASE_A` — the new workflow-group list. Phase 2/3
/// constants still work for test scenarios.
// `cast_possible_truncation`: space::* constants are u32 with bounded values;
// cast to u16 padding is safe.
#[allow(
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    deprecated
)]
#[must_use]
pub fn view(current_screen: Screen, entries: &[Screen], mode: ThemeMode) -> crate::Element<'_> {
    let mut column = Column::new()
        .padding([space::M as u16, 0_u16])
        .spacing(space::S);

    for screen in entries {
        let active = current_screen == *screen;
        let row_text = Text::new(label_for(*screen))
            .size(text::BODY)
            .color(if active {
                color::FG_1.current(mode)
            } else {
                color::FG_2.current(mode)
            });

        // Button carries the SwitchScreen message; styling renders no fill
        // for inactive rows and a subtle PANEL_SUNKEN tint on hover only.
        let button = Button::new(row_text)
            .on_press(Message::SwitchScreen(*screen))
            .padding([space::XS as u16, space::M as u16])
            .width(Length::Fill)
            .style(move |_theme: &iced::Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                    _ => None,
                };
                button::Style {
                    background: bg,
                    text_color: if active {
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

        column = column.push(frame::active_row(button.into(), active, mode));
    }

    // 1 px right-edge BORDER_1 hairline, rendered via the same Container
    // trick `frame::panel` uses for the header separator.
    let right_edge = Container::new(Space::new().width(Length::Fixed(1.0)).height(Length::Fill))
        .width(Length::Fixed(1.0))
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::BORDER_1.current(mode).into()),
            ..Default::default()
        });

    let body = iced::widget::Row::new()
        .push(column.width(Length::Fill))
        .push(right_edge);

    Container::new(body)
        .width(Length::Fixed(layout::SIDEBAR_WIDTH_PX))
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            text_color: Some(color::FG_2.current(mode)),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
#[allow(
    clippy::format_push_string,
    clippy::useless_format,
    clippy::uninlined_format_args,
    deprecated
)]
mod tests {
    use super::*;
    use crate::theme::layout::{SIDEBAR_ENTRIES_PHASE_3, SIDEBAR_ENTRIES_PHASE_A};
    use insta::assert_snapshot;

    /// Plain-text summary mirroring what the rendered sidebar shows. The
    /// summary captures: width, entries in scan order, and which row
    /// carries the `ACCENT` left rule (the active-row marker).
    fn sidebar_summary(current: Screen, entries: &[Screen]) -> String {
        let mut out = String::new();
        out.push_str("widget: sidebar_nav\n");
        out.push_str(&format!("width_px: {}\n", layout::SIDEBAR_WIDTH_PX));
        out.push_str(&format!("active: {:?}\n", current));
        out.push_str("entries:\n");
        for s in entries {
            let marker = if *s == current { "ACCENT" } else { "—" };
            let fg = if *s == current { "fg_1" } else { "fg_2" };
            out.push_str(&format!(
                "  rule={} label={} screen={:?} fg={}\n",
                marker,
                label_for(*s),
                s,
                fg,
            ));
        }
        out
    }

    #[test]
    #[allow(non_snake_case)]
    fn sidebar_nav__six_entries() {
        let summary = sidebar_summary(Screen::Home, SIDEBAR_ENTRIES_PHASE_3);
        assert_snapshot!("sidebar_nav__six_entries", summary);
    }

    #[test]
    #[allow(non_snake_case)]
    fn sidebar_nav__active_debug() {
        let summary = sidebar_summary(Screen::Debug, SIDEBAR_ENTRIES_PHASE_3);
        assert_snapshot!("sidebar_nav__active_debug", summary);
    }

    #[test]
    #[allow(non_snake_case)]
    fn sidebar_nav__active_charts() {
        let summary = sidebar_summary(Screen::Charts, SIDEBAR_ENTRIES_PHASE_3);
        assert_snapshot!("sidebar_nav__active_charts", summary);
    }

    #[test]
    #[allow(non_snake_case)]
    fn sidebar_nav__active_strategies() {
        let summary = sidebar_summary(Screen::Strategies, SIDEBAR_ENTRIES_PHASE_3);
        assert_snapshot!("sidebar_nav__active_strategies", summary);
    }

    #[test]
    #[allow(non_snake_case)]
    fn sidebar_nav__active_risk() {
        let summary = sidebar_summary(Screen::Risk, SIDEBAR_ENTRIES_PHASE_3);
        assert_snapshot!("sidebar_nav__active_risk", summary);
    }

    #[test]
    #[allow(non_snake_case)]
    fn sidebar_nav__active_audit() {
        let summary = sidebar_summary(Screen::Audit, SIDEBAR_ENTRIES_PHASE_3);
        assert_snapshot!("sidebar_nav__active_audit", summary);
    }

    /// T-D-3 — Phase A sidebar has the expected 8 entries in the correct
    /// workflow-group order.
    #[test]
    #[allow(non_snake_case)]
    fn sidebar__phase_a_workflow_group() {
        let summary = sidebar_summary(Screen::Lab, SIDEBAR_ENTRIES_PHASE_A);
        assert_snapshot!("sidebar__phase_a_workflow_group", summary);
    }

    /// T-D-2 — default boot screen (Screen::default()) is Lab.
    #[test]
    fn default_screen_is_lab() {
        assert_eq!(Screen::default(), Screen::Lab);
    }

    /// T-D-3 — every Phase A entry has a non-empty label.
    #[test]
    fn all_phase_a_labels_non_empty() {
        for screen in SIDEBAR_ENTRIES_PHASE_A {
            let label = label_for(*screen);
            assert!(!label.is_empty(), "empty label for screen {:?}", screen);
        }
    }
}
