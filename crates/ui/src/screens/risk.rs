//! Risk / Limits screen — Phase 3 (T1708).
//!
//! Layout (Phase 3 Design § Risk / Limits screen contract):
//!
//! 1. Per-venue exposure section — one `frame::threshold_bar` per
//!    `(Venue, Symbol)` entry in `risk_state.per_symbol_exposure`.
//! 2. Daily loss section — single bar, `daily_loss_used_pct /
//!    daily_loss_cap_pct`.
//! 3. Kill-threshold proximity gauge — single bar,
//!    `heartbeat_age_ms / heartbeat_timeout_ms` (Q9 — horizontal bar).
//!
//! Read-only (Q10 — Phase 5 `HumanControl` ratifies operator-write
//! exceptions).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

// `cast_possible_truncation`: `space::*` constants are `u32` with bounded
// values 0..64; cast to `u16` for iced padding is safe by construction.
// `elidable_lifetime_names`: explicit `'a` lifetimes document the
// borrow chain through helper functions.
#![allow(
    clippy::cast_possible_truncation,
    clippy::elidable_lifetime_names,
    clippy::needless_pass_by_value
)]

use iced::widget::{Column, Container, Row, Text};
use iced::Length;
use rust_decimal::Decimal;

use crate::state::{Cockpit, Message, PanelState, RiskState};
use crate::strings::{
    RISK_DAILY_LOSS_SECTION_TITLE, RISK_EXPOSURE_SECTION_TITLE, RISK_FEED_UNAVAILABLE_PREFIX,
    RISK_KILL_THRESHOLD_SECTION_TITLE, RISK_LOADING, RISK_PANEL_TITLE,
};
use crate::theme::{color, layout, space, text, ThemeMode};
use crate::widgets::frame::{self, muted_body, panel, threshold_bar};

/// Render the Risk / Limits screen body.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let body: iced::Element<'_, Message> = match &model.risk_state {
        PanelState::Loading | PanelState::Empty => muted_body(RISK_LOADING),
        PanelState::Error(e) => frame::error_body(RISK_FEED_UNAVAILABLE_PREFIX, e.as_str()),
        PanelState::Ready(state) => ready_body(state, mode),
    };

    let panel_body = Container::new(body)
        .width(Length::Fill)
        .padding(layout::PANEL_PADDING as u16);

    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(panel(RISK_PANEL_TITLE, panel_body.into(), mode))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn ready_body<'a>(state: &'a RiskState, mode: ThemeMode) -> iced::Element<'a, Message> {
    Column::new()
        .spacing(space::M)
        .push(exposure_section(state, mode))
        .push(daily_loss_section(state, mode))
        .push(kill_threshold_section(state, mode))
        .into()
}

fn exposure_section<'a>(state: &'a RiskState, mode: ThemeMode) -> iced::Element<'a, Message> {
    let header = section_header(RISK_EXPOSURE_SECTION_TITLE, mode);
    let mut col = Column::new().spacing(space::XS);
    // Sort keys for deterministic snapshot baselines.
    let mut keys: Vec<&(trading_core::Venue, trading_core::Symbol)> =
        state.per_symbol_exposure.keys().collect();
    keys.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1 .0.cmp(&b.1 .0)));

    for key in keys {
        let used = state
            .per_symbol_exposure
            .get(key)
            .copied()
            .unwrap_or_default();
        let cap = state.per_symbol_caps.get(key).copied().unwrap_or_default();
        let row_label = format!("{} \u{00b7} {}", key.0, key.1);
        let label = Text::new(row_label)
            .size(text::SMALL)
            .color(color::FG_2.current(mode));
        let bar = threshold_bar::<Message>(used, cap, mode);
        let pct = pct_label(used, cap);
        let value_label = Text::new(format!("{used} / {cap} ({pct})"))
            .size(text::SMALL)
            .color(color::FG_1.current(mode));
        let row = Row::new()
            .spacing(space::M)
            .push(Container::new(label).width(Length::Fixed(160.0)))
            .push(Container::new(bar).width(Length::Fill))
            .push(Container::new(value_label).width(Length::Fixed(160.0)));
        col = col.push(row);
    }
    Column::new()
        .spacing(space::S)
        .push(header)
        .push(col)
        .into()
}

fn daily_loss_section<'a>(state: &'a RiskState, mode: ThemeMode) -> iced::Element<'a, Message> {
    let header = section_header(RISK_DAILY_LOSS_SECTION_TITLE, mode);
    let used = state.daily_loss_used_pct;
    let cap = state.daily_loss_cap_pct;
    let bar = threshold_bar::<Message>(used, cap, mode);
    let pct = pct_label(used, cap);
    let value_label = Text::new(format!("{used} % / {cap} % ({pct})"))
        .size(text::SMALL)
        .color(color::FG_1.current(mode));
    let row = Row::new()
        .spacing(space::M)
        .push(Container::new(bar).width(Length::Fill))
        .push(Container::new(value_label).width(Length::Fixed(180.0)));
    Column::new()
        .spacing(space::S)
        .push(header)
        .push(row)
        .into()
}

fn kill_threshold_section<'a>(state: &'a RiskState, mode: ThemeMode) -> iced::Element<'a, Message> {
    let header = section_header(RISK_KILL_THRESHOLD_SECTION_TITLE, mode);
    let used = Decimal::from(state.heartbeat_age_ms);
    let cap = Decimal::from(state.heartbeat_timeout_ms);
    let bar = threshold_bar::<Message>(used, cap, mode);
    let pct = pct_label(used, cap);
    let value_label = Text::new(format!(
        "{} ms / {} ms ({})",
        state.heartbeat_age_ms, state.heartbeat_timeout_ms, pct
    ))
    .size(text::SMALL)
    .color(color::FG_1.current(mode));
    let row = Row::new()
        .spacing(space::M)
        .push(Container::new(bar).width(Length::Fill))
        .push(Container::new(value_label).width(Length::Fixed(220.0)));
    Column::new()
        .spacing(space::S)
        .push(header)
        .push(row)
        .into()
}

fn section_header<'a>(title: &'static str, mode: ThemeMode) -> iced::Element<'a, Message> {
    Text::new(title)
        .size(text::H3)
        .color(color::FG_1.current(mode))
        .into()
}

fn pct_label(used: Decimal, cap: Decimal) -> String {
    if cap == Decimal::ZERO {
        return "—".to_string();
    }
    let pct = (used / cap) * Decimal::from(100);
    let rounded = pct.round_dp(0);
    format!("{rounded} %")
}
