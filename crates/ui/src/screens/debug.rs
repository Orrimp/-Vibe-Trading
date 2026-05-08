//! Debug screen — Phase 2 (T1605); Phase 5 (Q1) removed the kill widget
//! (kill migrates to the `HumanControl` panel's bottom action — see
//! `screens::control` / `widgets::human_control`).
//!
//! Operations chrome screen: latency detail, per-venue market-health
//! rows, server-time detail, version string, and a placeholder
//! logs/metrics surface.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{Column, Container, Row, Text};
use iced::Length;

use crate::state::{Cockpit, MarketHealthState};
use crate::strings::{
    DEBUG_LOGS_PLACEHOLDER, STATUS_BAR_NO_SERVER_TIME, STATUS_BAR_SERVER_LABEL,
    STATUS_BAR_UTC_SUFFIX, STATUS_BAR_VERSION,
};
use crate::theme::{color, layout, space, text, ThemeMode};
use crate::widgets::{frame, latency};

/// Render the Debug screen body.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    // Phase 5 (Q1) — kill widget removed from this screen; it now lives
    // as the bottom action of the HumanControl panel under
    // `Screen::Control`.
    let body = Column::new()
        .padding(space::L as u16)
        .spacing(layout::PANEL_OUTER_GAP)
        .push(latency::view(model))
        .push(market_health_section(model, mode))
        .push(server_time_row(model, mode))
        .push(version_row(mode))
        .push(logs_row())
        .width(Length::Fill)
        .height(Length::Fill);
    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn market_health_section(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let mut col = Column::new().spacing(space::S);
    let mut entries: Vec<(_, MarketHealthState)> =
        model.market_health.iter().map(|(v, s)| (v, *s)).collect();
    entries.sort_by_key(|(v, _)| v.to_string());
    for (venue, state) in entries {
        let last_age = age_label_for_venue(model);
        let state_label = match state {
            MarketHealthState::Fresh => "fresh",
            MarketHealthState::Stale => "stale",
        };
        col = col.push(
            Row::new()
                .spacing(space::S)
                .push(
                    Text::new(venue.to_string())
                        .size(text::BODY)
                        .color(color::FG_1.current(mode)),
                )
                .push(
                    Text::new(state_label)
                        .size(text::SMALL)
                        .color(color::FG_3.current(mode)),
                )
                .push(
                    Text::new(last_age)
                        .size(text::SMALL)
                        .color(color::FG_3.current(mode)),
                ),
        );
    }
    col.into()
}

fn age_label_for_venue(model: &Cockpit) -> String {
    match (model.last_tick_ts, model.server_time_now) {
        (Some(tick), Some(now)) => {
            let delta = now.unix_millis() - tick.unix_millis();
            // Guard against negative skew.
            let secs = delta.max(0) / 1_000;
            format!("last_tick {secs}s")
        }
        _ => "last_tick —".to_string(),
    }
}

fn server_time_row(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let server_text = match model.server_time_now {
        Some(ts) => {
            let dt = ts.inner();
            format!(
                "{STATUS_BAR_SERVER_LABEL} {:02}:{:02}:{:02} {STATUS_BAR_UTC_SUFFIX}",
                dt.hour(),
                dt.minute(),
                dt.second(),
            )
        }
        None => {
            format!("{STATUS_BAR_SERVER_LABEL} {STATUS_BAR_NO_SERVER_TIME} {STATUS_BAR_UTC_SUFFIX}")
        }
    };
    Text::new(server_text)
        .size(text::BODY)
        .color(color::FG_2.current(mode))
        .into()
}

fn version_row(mode: ThemeMode) -> crate::Element<'static> {
    Text::new(STATUS_BAR_VERSION)
        .size(text::SMALL)
        .color(color::FG_3.current(mode))
        .into()
}

fn logs_row() -> crate::Element<'static> {
    frame::muted_body(DEBUG_LOGS_PLACEHOLDER)
}
