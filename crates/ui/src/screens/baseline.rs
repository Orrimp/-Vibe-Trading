//! cockpit-baseline-panel v0.1.0 — Baseline screen body (T5).
//!
//! Surfaces the shipped passive buy-and-hold result for a selected year
//! (2023 | 2024, default 2024), reusing the existing render widgets
//! **verbatim**. Composition (top → bottom):
//!
//! ```text
//! headline (BASELINE_HEADLINE, text::H2)            year chips [2023][2024◀]
//! caption  (BASELINE_CAPTION — honest bounded scope, R3/A3)
//! kpi_strip::view(&Ready(baseline_metrics(active_year)))   ← 6 fixed cards
//! equity_curve::view(&active_curve)                        ← realized BH line
//! drawdown_band::view(&active_curve)                       ← free from from_points
//! BASELINE_RISK_DETAIL (Sortino/Calmar caption — A2, FG_3)
//! ```
//!
//! The three render widgets return `Element<'_, ViewerMessage>`; they are
//! bridged to the screen's `Message` with `.map(|_| Message::
//! ChartMarkerHoverEnded)` — a harmless never-fired no-op arm (these panels
//! emit no interactions for Baseline), exactly as `screens/live.rs` does.
//!
//! Metrics are pulled from the `const` `baseline::baseline_metrics(year)`
//! at view time (D1 = c) — not stored on the model, not recomputed from the
//! daily-sampled curve (the published Sharpe is hourly; re-derivation would
//! disagree with the characterization). So a missing CSV leaves the KPI
//! strip populated (honest degrade) while the curve + band show their
//! Error body.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget** (AC7).

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::widget::{Button, Column, Row, Text, button};
use iced::{Border, Length};

use crate::state::{BaselineYear, Cockpit, Message};
use crate::strings::{
    BASELINE_CAPTION, BASELINE_HEADLINE, BASELINE_RISK_DETAIL, BASELINE_YEAR_2023_LABEL,
    BASELINE_YEAR_2024_LABEL,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::{drawdown_band, equity_curve, kpi_strip};

/// Render the Baseline screen body (R1–R5).
///
/// Called by `shell::screen_body` when `current_screen == Screen::Baseline`.
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let active_year = model.baseline_screen_state.active_year;

    // ── Headline + year chips row ────────────────────────────────────────────
    let headline = Text::new(BASELINE_HEADLINE)
        .size(text::H2)
        .color(color::FG_1.current(mode));

    let year_chips = build_year_chips(active_year, mode);

    let headline_row = Row::new()
        .spacing(space::M)
        .align_y(iced::alignment::Vertical::Center)
        .push(headline)
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(year_chips);

    // ── Honest-bounded caption (R3 / A3) ─────────────────────────────────────
    let caption = Text::new(BASELINE_CAPTION)
        .size(text::BODY)
        .color(color::FG_2.current(mode));

    // ── KPI strip — sourced from the `const`, always populated (D1=c) ────────
    // `kpi_strip::view` takes `&PanelState<BacktestMetrics>` and ties its
    // returned element to that ref's lifetime, so we borrow the model-stored
    // `Ready(const)` block (materialized at boot from `baseline_metrics`) — the
    // viewer's `&self.model.metrics` pattern. The strip never errors; only the
    // curve can (so a missing CSV still leaves the strip populated — honest
    // degrade).
    let kpi = kpi_strip::view(model.baseline_screen_state.active_metrics(), mode)
        .map(|_| Message::ChartMarkerHoverEnded);

    // ── Equity curve + drawdown band — both from the active year's curve ─────
    // Bridge `ViewerMessage` → `Message` via the never-fired no-op arm
    // (Baseline panels emit no interactions), per `screens/live.rs`.
    let active_curve = model.baseline_screen_state.active_curve();
    let curve = equity_curve::view(active_curve, mode).map(|_| Message::ChartMarkerHoverEnded);
    let band = drawdown_band::view(active_curve, mode).map(|_| Message::ChartMarkerHoverEnded);

    // ── Caption-only Sortino / Calmar detail (A2 — no KPI slot) ──────────────
    let risk_detail = Text::new(BASELINE_RISK_DETAIL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    // ── Compose full-screen column ───────────────────────────────────────────
    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(headline_row)
        .push(caption)
        .push(kpi)
        .push(curve)
        .push(band)
        .push(risk_detail)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Build the `[2023] [2024]` year-toggle chip row (R2).
///
/// Established Compare/Lab chip pattern: `Button` + active/inactive token
/// styling, `on_press(Message::BaselineSelectYear(year))`, focusable +
/// Enter-activatable (iced buttons with an `on_press` are keyboard-
/// reachable). Active chip = `ACCENT` text on `PANEL_RAISED` bg with an
/// `ACCENT` border; inactive = `FG_3` text + `BORDER_1` border. Colour is
/// never the only active signal — the active chip also gets the raised
/// background (shape), satisfying the accessibility minimum.
fn build_year_chips(selected: BaselineYear, mode: ThemeMode) -> Row<'static, Message> {
    let years = [
        (BaselineYear::Y2023, BASELINE_YEAR_2023_LABEL),
        (BaselineYear::Y2024, BASELINE_YEAR_2024_LABEL),
    ];

    let mut row = Row::new().spacing(space::XS);
    for (year, label) in years {
        let is_active = selected == year;
        let btn = Button::new(Text::new(label).size(text::SMALL).color(if is_active {
            color::ACCENT.current(mode)
        } else {
            color::FG_3.current(mode)
        }))
        .on_press(Message::BaselineSelectYear(year))
        .padding([space::XS as u16, space::S as u16])
        .style(move |_: &iced::Theme, _: button::Status| button::Style {
            background: if is_active {
                Some(color::PANEL_RAISED.current(mode).into())
            } else {
                None
            },
            text_color: if is_active {
                color::ACCENT.current(mode)
            } else {
                color::FG_3.current(mode)
            },
            border: Border {
                color: if is_active {
                    color::ACCENT.current(mode)
                } else {
                    color::BORDER_1.current(mode)
                },
                width: 1.0,
                radius: radius::R1.into(),
            },
            ..Default::default()
        });
        row = row.push(btn);
    }
    row
}
