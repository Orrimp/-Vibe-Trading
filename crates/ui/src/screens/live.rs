//! Live trading dashboard screen — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! Layout per dev-note §J6 lines 528-542 (R2.2):
//!
//! 1. **Top:** system-health strip — latency row + market-health summary
//!    + server-time badge. Draws from `screens::debug` helpers exposed as
//!    `pub(crate)` so both surfaces stay in sync without code duplication.
//! 2. **Mid-top:** full-width equity curve (`widgets::equity_curve`) in
//!    `PanelState::Loading` placeholder (Design § A7 — no live feed yet).
//! 3. **Mid-bottom:** KPI strip (`widgets::kpi_strip`) in `PanelState::Loading`
//!    placeholder (Design § A8) + LLM-spend text tile (Design § A9 / Q4b).
//! 4. **Bottom:** 2-column row — `widgets::positions` LEFT,
//!    `widgets::agent_feed` RIGHT.
//!
//! `widgets::equity_curve` / `widgets::kpi_strip` return
//! `Element<'_, ViewerMessage>`. The `.map(|_| Message::ServerTimeTick(...))`
//! adapter bridges the type gap per Design § A7 + the gallery precedent at
//! `crates/ui/src/gallery/routes.rs:533`.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::Length;
use iced::widget::{Column, Container, Row, Text};
use trading_core::{BacktestMetrics, EquitySeries};

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    LIVE_HEADLINE, LIVE_LLM_SPEND_LABEL, LIVE_LLM_SPEND_PLACEHOLDER, LIVE_SYSTEM_HEALTH_LABEL,
};
use crate::theme::{ThemeMode, color, layout, space, text};
use crate::widgets::{agent_feed, equity_curve, kpi_strip, latency, positions};

/// Render the Live screen body (R2.1 / R2.2).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    // Headline row.
    let headline = Text::new(LIVE_HEADLINE)
        .size(text::H2)
        .color(color::FG_1.current(mode));

    // ── System health strip (top) ────────────────────────────────────────────
    let health_label = Text::new(LIVE_SYSTEM_HEALTH_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let health_strip = Row::new()
        .spacing(space::M)
        .push(health_label)
        .push(latency::view(model))
        .push(crate::screens::debug::server_time_compact(model, mode))
        .push(crate::screens::debug::market_health_compact(model, mode));

    // ── Equity curve (full-width placeholder) ────────────────────────────────
    // Design § A7: feed PanelState::Loading so the widget renders its
    // VIEWER_NO_EQUITY_DATA placeholder. No new Cockpit field needed.
    let equity_state: &PanelState<EquitySeries> = &PanelState::Loading;
    // `.map(|_| Message::ChartMarkerHoverEnded)` bridges ViewerMessage → Message.
    // The callback is never called because Loading renders no interactive elements.
    // Design § A7 + gallery/routes.rs:533 adapter pattern.
    let equity = equity_curve::view(equity_state, mode).map(|_| Message::ChartMarkerHoverEnded);

    // ── KPI strip + LLM spend tile ──────────────────────────────────────────
    // Design § A8: same Loading-placeholder treatment for the KPI strip.
    let kpi_state: &PanelState<BacktestMetrics> = &PanelState::Loading;
    let kpi = kpi_strip::view(kpi_state, mode).map(|_| Message::ChartMarkerHoverEnded);

    // Design § A9 / Q4b: text-only LLM spend tile (real wiring in Phase F).
    let llm_label = Text::new(LIVE_LLM_SPEND_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let llm_value = Text::new(LIVE_LLM_SPEND_PLACEHOLDER)
        .size(text::BODY)
        .color(color::FG_2.current(mode));
    let llm_tile = Column::new()
        .spacing(space::XS)
        .push(llm_label)
        .push(llm_value);

    let kpi_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(kpi)
        .push(Container::new(llm_tile).width(Length::Fixed(120.0)));

    // ── 2-column positions / activity row ───────────────────────────────────
    let bottom_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(positions::view(model))
        .push(agent_feed::view(model));

    // ── Full-screen column ───────────────────────────────────────────────────
    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(headline)
        .push(health_strip)
        .push(equity)
        .push(kpi_row)
        .push(bottom_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
