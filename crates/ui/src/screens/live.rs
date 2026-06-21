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

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    LIVE_FORWARD_BUDGET_LABEL, LIVE_FORWARD_DISCLAIMER, LIVE_FORWARD_FX_NOTE,
    LIVE_FORWARD_PNL_LABEL, LIVE_FORWARD_RUNNING_FMT, LIVE_HEADLINE, LIVE_LLM_SPEND_LABEL,
    LIVE_LLM_SPEND_PLACEHOLDER, LIVE_SESSION_RETURN_CAPTION, LIVE_SINCE_INCEPTION_CAPTION,
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

    // cockpit-live-dashboard-wiring (R5 / AC5) + live-equity-history-durable
    // (R6) — honest scope caption for the strip's Total-return card. When a
    // durable paper/live history has been hydrated (`live_equity_hydrated`), the
    // figure is measured from the first persisted point (account inception) and
    // may span sessions/days → "Since inception"; otherwise (research mode, or a
    // paper boot with no prior history) it is session-to-date → "Session to
    // date". Both are honest scope labels — never an annualized / characterized
    // result, never a fabricated number.
    let caption_text = if model.live_equity_hydrated {
        LIVE_SINCE_INCEPTION_CAPTION
    } else {
        LIVE_SESSION_RETURN_CAPTION
    };
    let session_caption = Text::new(caption_text)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    // ── F5 — Forward paper-trade P/L framing ────────────────────────────────
    // Rendered only when `model.forward_budget` is `Some(budget)`.
    //
    // P/L = equity − budget   (USDT; no FX conversion)
    // P/L% = (equity − budget) / budget × 100
    //
    // Color: green (UP_500) when P/L ≥ 0, red (DOWN_500) when P/L < 0.
    // Label: `LIVE_FORWARD_PNL_LABEL` ("P/L").
    // FX note: `LIVE_FORWARD_FX_NOTE` ("€200 ≈ 200 USDT — FX not modelled.")
    // Disclaimer: `LIVE_FORWARD_DISCLAIMER` (not-advice + simulated-budget).
    //
    // When `forward_budget` is `None` (legacy research / soak path), the
    // block is not rendered — pre-F5 byte-identical behaviour preserved.
    let forward_pnl_block: Option<crate::Element<'_>> = model
        .forward_budget
        .as_ref()
        .map(|budget| build_forward_pnl_block(model, budget, mode));

    // ── 2-column positions / activity row ───────────────────────────────────
    let bottom_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(positions::view(model))
        .push(agent_feed::view(model));

    // ── Full-screen column ───────────────────────────────────────────────────
    let mut col = Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(headline)
        .push(health_strip)
        .push(equity)
        .push(kpi_row)
        .push(session_caption);

    if let Some(block) = forward_pnl_block {
        col = col.push(block);
    }

    col.push(bottom_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Build the F5 forward P/L card for `budget`.
///
/// Computes P/L = `total_equity` − `budget` from the latest `PnlSnapshot` in
/// `model.pnl`. Falls back to "—" while the first snapshot is still pending.
///
/// The block is a `Column` with:
/// 1. Running-caption row (strategy id + "simulated budget" label).
/// 2. P/L row: label "P/L" + value (coloured green / red + sign).
///    Budget row: label "Budget" + formatted budget amount.
/// 3. FX note: "€200 ≈ 200 USDT — FX not modelled." (`LIVE_FORWARD_FX_NOTE`).
/// 4. Disclaimer (`LIVE_FORWARD_DISCLAIMER`).
fn build_forward_pnl_block<'a>(
    model: &'a Cockpit,
    budget: &trading_core::Money<trading_core::Usdt>,
    mode: ThemeMode,
) -> crate::Element<'a> {
    use iced::widget::{Column, Row, Text};

    // ── Compute P/L ──────────────────────────────────────────────────────────
    let (pnl_text, pnl_pct_text, pnl_color) = if let PanelState::Ready(snap) = &model.pnl {
        let equity_amt = snap.total_equity.amount();
        let budget_amt = budget.amount();
        let pnl_raw = equity_amt - budget_amt;
        let sign = if pnl_raw >= dec!(0) { "+" } else { "" };
        let pnl_str = format!("{sign}{pnl_raw:.2} USDT");
        let pnl_pct_str = if budget_amt.is_zero() {
            String::from("(\u{2014}%)")
        } else {
            let pct = pnl_raw / budget_amt * Decimal::from(100);
            format!("({sign}{pct:.2}%)")
        };
        let col = if pnl_raw >= dec!(0) {
            color::UP_500.current(mode)
        } else {
            color::DOWN_500.current(mode)
        };
        (pnl_str, pnl_pct_str, col)
    } else {
        (
            String::from("\u{2014}"),
            String::new(),
            color::FG_2.current(mode),
        )
    };

    // ── Running caption ───────────────────────────────────────────────────────
    // If we know the strategy from the leaderboard state, show it; otherwise
    // just "Simulated budget — see Leaderboard."
    let caption_str = {
        let sid = if let PanelState::Ready(mirror) = &model.leaderboard_screen_state.result {
            mirror.crowned_row().map_or_else(
                || String::from("selected strategy"),
                |r| r.strategy.as_str().to_owned(),
            )
        } else {
            String::from("selected strategy")
        };
        LIVE_FORWARD_RUNNING_FMT.replace("{strategy}", &sid)
    };

    let running_caption = Text::new(caption_str)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    // ── P/L row ───────────────────────────────────────────────────────────────
    let pnl_label = Text::new(LIVE_FORWARD_PNL_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let pnl_value_text = if pnl_pct_text.is_empty() {
        pnl_text.clone()
    } else {
        format!("{pnl_text}  {pnl_pct_text}")
    };
    let pnl_value = Text::new(pnl_value_text).size(text::BODY).color(pnl_color);
    let pnl_col = Column::new()
        .spacing(space::XS)
        .push(pnl_label)
        .push(pnl_value);

    // ── Budget row ────────────────────────────────────────────────────────────
    let budget_label = Text::new(LIVE_FORWARD_BUDGET_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let budget_value = Text::new(format!("{:.0} USDT", budget.amount()))
        .size(text::BODY)
        .color(color::FG_2.current(mode));
    let budget_col = Column::new()
        .spacing(space::XS)
        .push(budget_label)
        .push(budget_value);

    // ── Metric row (P/L + Budget side by side) ────────────────────────────────
    let metric_row = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(pnl_col)
        .push(budget_col);

    // ── FX note + disclaimer ──────────────────────────────────────────────────
    let fx_note = Text::new(LIVE_FORWARD_FX_NOTE)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let disclaimer = Text::new(LIVE_FORWARD_DISCLAIMER)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    Column::new()
        .spacing(space::XS)
        .push(running_caption)
        .push(metric_row)
        .push(fx_note)
        .push(disclaimer)
        .into()
}
