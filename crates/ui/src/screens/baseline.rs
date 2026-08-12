//! cockpit-baseline-panel v0.1.0 — Baseline screen body (T5).
//!
//! Surfaces the shipped passive buy-and-hold result for a selected year
//! (2023 | 2024, default 2024), reusing the existing render widgets
//! **verbatim**. Composition (top → bottom):
//!
//! ```text
//! headline (BASELINE_HEADLINE, text::H2)            year chips [2023][2024◀]
//! caption  (BASELINE_CAPTION — honest bounded scope + gross-of-costs, R3/A3)
//! kpi_strip::view(&Ready(baseline_metrics(active_year)))   ← 6 fixed cards
//! sharpe note   (realized single-path vs bootstrap p50 — review H3)
//! equity_curve::view(&active_curve)                        ← realized BH line
//! drawdown_band::view(&active_curve)                       ← free from from_points
//! sampling note (hourly card vs daily-sampled line — review H2)
//! risk detail   (Sortino/Calmar for the ACTIVE year — A2, FG_3)
//! ```
//!
//! The whole stack sits in a `Scrollable` (review M-scroll): the content is
//! ~600 px of fixed-height panels plus four text blocks, so on a short window
//! the honesty captions — the parts that qualify the numbers above them — were
//! clipped and unreachable. Every sibling screen with stacked panels
//! (`forward_plan`, `leaderboard`, `tune`, `models`, `memory`) already wraps.
//!
//! The three render widgets return `Element<'_, ViewerMessage>`; they are
//! bridged to the screen's `Message` with `.map(|_| Message::
//! ChartMarkerHoverEnded)` — a harmless never-fired no-op arm (these panels
//! emit no interactions for Baseline), exactly as `screens/live.rs` does.
//!
//! Metrics come from the `const` `baseline::baseline_metrics(year)` (D1 = c),
//! materialized on the model at boot — not recomputed from the daily-sampled
//! curve (the published metrics are hourly; re-derivation would disagree with
//! the characterization *and* with every artifact that cites it). So a missing
//! CSV leaves the KPI strip populated (honest degrade) while the curve + band
//! show their Error body.
//!
//! **That choice is exactly why this screen carries two provenance captions.**
//! Keeping the published scalars means the card and the line beneath it are
//! measured on different samplings (hourly vs daily; up to 7.13 points apart
//! on max drawdown), and that the Sharpe on the card is the realized
//! single-path figure rather than the bootstrap p50 the PRD headlines. Both
//! are defensible; neither was *disclosed*, on the one screen whose purpose is
//! "this is the bar active must clear". The captions are the disclosure, and
//! `baseline_sampling_note` derives its curve figure from the loaded series so
//! the reconciliation cannot silently rot.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget** (AC7).

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::widget::{Button, Column, Row, Scrollable, Text, button};
use iced::{Border, Length};

use crate::baseline::loader;
use crate::state::{BaselineYear, Cockpit, Message, PanelState};
use crate::strings::{
    BASELINE_CAPTION, BASELINE_HEADLINE, BASELINE_YEAR_2023_LABEL, BASELINE_YEAR_2024_LABEL,
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

    // ── Provenance captions (review H2 / H3) + Sortino / Calmar (A2) ─────────
    // All three are FG_3 `SMALL` — they qualify the numbers above them without
    // competing with the cards. Their text comes from production seams in
    // `baseline::loader`, which the snapshot mirror also calls (so the mirror
    // cannot claim copy the screen does not render).
    let sharpe_note = muted_line(&sharpe_note_text(active_year), mode);
    let risk_detail = muted_line(&risk_detail_text(active_year), mode);

    // ── Compose full-screen column ───────────────────────────────────────────
    let mut stack = Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(headline_row)
        .push(caption)
        .push(kpi)
        .push(sharpe_note)
        .push(curve)
        .push(band);
    // The sampling note is data-dependent: it reconciles the card's hourly
    // max drawdown against the DRAWN curve's, so it only exists when a curve
    // is drawn.
    if let Some(note) = sampling_note_text(model) {
        stack = stack.push(muted_line(&note, mode));
    }
    let stack = stack.push(risk_detail).width(Length::Fill);

    Scrollable::new(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// One muted caption line. Owns its text (the notes are built per-view), so
/// it cannot borrow from a temporary.
fn muted_line<'a>(body: &str, mode: ThemeMode) -> Text<'a> {
    Text::new(body.to_string())
        .size(text::SMALL)
        .color(color::FG_3.current(mode))
}

/// **Production seam** — the Sharpe-provenance caption the screen renders
/// (review H3). Exposed so the panel-snapshot mirror reads the same string
/// `view` does instead of re-deriving it.
#[must_use]
pub fn sharpe_note_text(year: BaselineYear) -> String {
    loader::baseline_sharpe_note(year)
}

/// **Production seam** — the Sortino / Calmar caption for the ACTIVE year
/// (review M-static: this line used to be a static const printing both years
/// regardless of the toggle).
#[must_use]
pub fn risk_detail_text(year: BaselineYear) -> String {
    loader::baseline_risk_detail(year)
}

/// **Production seam** — the card-vs-curve sampling reconciliation (review
/// H2), or `None` when the active curve is not `Ready`.
///
/// This is the branch decision the screen makes; the snapshot mirror calls it
/// rather than re-implementing the match (2-15 review M4's rule).
#[must_use]
pub fn sampling_note_text(model: &Cockpit) -> Option<String> {
    let year = model.baseline_screen_state.active_year;
    match model.baseline_screen_state.active_curve() {
        PanelState::Ready(series) if !series.points.is_empty() => {
            Some(loader::baseline_sampling_note(year, series))
        }
        _ => None,
    }
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
