//! Live trading dashboard screen — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! Layout per dev-note §J6 lines 528-542 (R2.2):
//!
//! 1. **Top:** system-health strip — latency row + market-health summary
//!    + server-time badge. Draws from `screens::debug` helpers exposed as
//!    `pub(crate)` so both surfaces stay in sync without code duplication.
//! 2. **Mid-top:** full-width equity curve (`widgets::equity_curve`) bound to
//!    `model.live_equity_curve` — the session-scoped live equity series
//!    accumulated from the per-bar `pnl` feed (cockpit-live-dashboard-wiring
//!    v0.1.0 / D1=(a)). `Loading` until the first bar; grows per bar.
//! 3. **Mid-bottom:** KPI strip (`widgets::kpi_strip`) bound to
//!    `model.live_kpi` (live session Total-return + Max-DD; Sharpe/CAGR/Win
//!    `—`; Trades 0 — D2) + LLM-spend text tile (Design § A9 / Q4b — a
//!    separate Phase-F placeholder, untouched here).
//! 4. **Bottom:** 2-column row — `widgets::positions` LEFT,
//!    `widgets::agent_feed` RIGHT.
//!
//! `widgets::equity_curve` / `widgets::kpi_strip` return
//! `Element<'_, ViewerMessage>`. The `.map(|_| Message::ChartMarkerHoverEnded)`
//! adapter bridges the type gap — a harmless never-fired no-op arm (the live
//! curve/strip emit no interactions), exactly as `screens/baseline.rs` does.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget** (AC7).

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::Length;
use iced::widget::{Column, Container, Row, Text};

use crate::state::{Cockpit, Message};
use crate::strings::{
    LIVE_HEADLINE, LIVE_LLM_SPEND_LABEL, LIVE_LLM_SPEND_PLACEHOLDER, LIVE_SESSION_RETURN_CAPTION,
    LIVE_SYSTEM_HEALTH_LABEL,
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

    // ── Equity curve (full-width, live) ──────────────────────────────────────
    // cockpit-live-dashboard-wiring — bound to the model-cached, session-scoped
    // live equity series (derived on-append in the `PnlRefreshed` arm, D5).
    // `Loading` until the first bar; grows per bar; `Error`/`Empty` degrade
    // honestly. `.map(|_| Message::ChartMarkerHoverEnded)` bridges
    // ViewerMessage → Message — a never-fired no-op (the live curve emits no
    // interactions), mirroring `screens/baseline.rs`.
    let equity_state = &model.live_equity_curve;
    let equity = equity_curve::view(equity_state, mode).map(|_| Message::ChartMarkerHoverEnded);

    // ── KPI strip + LLM spend tile ──────────────────────────────────────────
    // cockpit-live-dashboard-wiring — bound to the model-cached live KPI strip
    // (Loading until ≥2 points per the `is_all_absent` trap, D2). Total-return
    // (session) + Max-DD are live; Sharpe/CAGR/Win-rate render `—`; Trades 0.
    let kpi_state = &model.live_kpi;
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

    // cockpit-live-dashboard-wiring (R5 / AC5) — honest scope caption for the
    // strip's Total-return card: the live figure is session-to-date, NOT an
    // annualized / characterized result. Static scope label (describes what
    // the card means, never a fabricated number).
    let session_caption = Text::new(LIVE_SESSION_RETURN_CAPTION)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

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
        .push(session_caption)
        .push(bottom_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
