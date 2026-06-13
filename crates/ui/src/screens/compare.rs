#![allow(clippy::cast_possible_truncation)]
//! Compare screen — Phase E (ui-rethink-phase-e-compare R1.1-R1.4).
//!
//! Body composition (top to bottom):
//!
//! 1. **Toolbar row** — date-range picker + KPI-axis dropdown + K7
//!    universe-aggregate subtitle (when ≥ 1 multi-symbol cell is in view).
//! 2. **Matrix body** — `widgets::matrix::view(model, mode)`.
//! 3. **(Reserved v0.2.0)** "Recompute all missing" toolbar button — Q2=(c)
//!    out-of-scope at v0.1.0.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new Lumen tokens** (R7.6).

use iced::widget::{Column, Container, Row, Text};
use iced::{Element, Length, Padding};

use crate::compare::state::{CompareKpiAxis, OverlaySlot};
use crate::lab::equity_loader::LabEquitySeries;
use crate::lab::state::DateRange;
use crate::state::{Cockpit, Message};
use crate::strings::{
    COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE, COMPARE_OVERLAY_EMPTY, COMPARE_OVERLAY_LEGEND_COMPARE,
    COMPARE_OVERLAY_LEGEND_PRIMARY, COMPARE_OVERLAY_LEGEND_SWATCH, COMPARE_OVERLAY_NO_SERIES,
    COMPARE_OVERLAY_TITLE, COMPARE_TOOLBAR_KPI_LABEL, COMPARE_TOOLBAR_RANGE_LABEL,
};
use crate::theme::{ThemeMode, color, space, text};
use crate::widgets::{chart, matrix};

/// Fixed height of the two-run equity-overlay chart panel (logical px). Sized
/// to match the Lab chart's vertical budget so the operator reads both at the
/// same scale.
const OVERLAY_CHART_HEIGHT_PX: f32 = 240.0;

/// Render the Compare screen body.
///
/// Called by `shell::screen_body` when `current_screen == Screen::Compare`
/// (replacing the Phase A `placeholder::view` route per R1.3).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_, Message> {
    // ── Toolbar row ──────────────────────────────────────────────────────────
    let range_label = Text::new(COMPARE_TOOLBAR_RANGE_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    // Date-range picker: same presets as Lab (R6.4 — no new DateRange variant).
    // Renders as a row of preset chips.
    let range_chips = build_range_chips(&model.compare_screen_state.range, mode);

    let kpi_label = Text::new(COMPARE_TOOLBAR_KPI_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    let kpi_chips = build_kpi_chips(model.compare_screen_state.kpi_axis, mode);

    let toolbar = Row::new()
        .spacing(space::M)
        .align_y(iced::alignment::Vertical::Center)
        .push(range_label)
        .push(range_chips)
        .push(Text::new(" ").size(text::SMALL)) // gutter
        .push(kpi_label)
        .push(kpi_chips);

    // ── K7 subtitle ──────────────────────────────────────────────────────────
    //
    // §1.4 of decomp.md: always-visible subtitle when ≥ 1 multi-symbol cell
    // is populated in the cache. At v0.1.0 we show it whenever v1.momentum or
    // v2.5.tcn have any cache hits (which is ~20/24 populated cells per the
    // H1 census).  Simple check: any CachedCell with `is_multi_symbol == true`.
    let has_multi_symbol = model
        .compare_screen_state
        .cache
        .values()
        .any(|c| c.is_multi_symbol);

    let maybe_subtitle: Option<Element<'_, Message>> = if has_multi_symbol {
        Some(
            Text::new(COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE)
                .size(text::MICRO)
                .color(color::FG_4.current(mode))
                .into(),
        )
    } else {
        None
    };

    // ── Matrix body ──────────────────────────────────────────────────────────
    let matrix_body = matrix::view(model, mode);

    // ── Compose ──────────────────────────────────────────────────────────────
    let mut col = Column::new()
        .spacing(space::S)
        .padding(Padding::from(space::M as u16))
        .push(toolbar);

    if let Some(subtitle) = maybe_subtitle {
        col = col.push(subtitle);
    }

    col = col.push(
        Container::new(matrix_body)
            .width(Length::Fill)
            .height(Length::Fill),
    );

    // ── Two-run equity overlay (lab-compare-equity-overlay T2 / Q1) ────────────
    // Always present so the operator always knows the overlay exists (no blank
    // surprise): an empty-state prompt until ≥ 1 run is selected, then the
    // chart with slot 0 in ACCENT + slot 1 in ACCENT_2.
    col = col.push(overlay_panel(model, mode));

    col.into()
}

// ── Two-run equity overlay panel ────────────────────────────────────────────

/// Build the equity-overlay panel below the matrix (T2 / Q1).
///
/// Reads `compare_screen_state.overlay_selection` (≤ 2 slots), resolves each
/// slot's `CachedCell` from the cache, and feeds the timestamped series to the
/// render-proven `chart::view` overlay (`equity` = slot 0 → `ACCENT`;
/// `compare[0]` = slot 1 → `ACCENT_2`). The chart renders with EMPTY bars, so
/// it takes the standalone equity path that self-scales the x-axis from the
/// series' own timestamps (`chart.rs:480` no-bars branch) — no per-run price
/// bars needed.
///
/// States (no blank screens):
/// - **Empty** (nothing selected): a prompt telling the operator to pick runs.
/// - **Selected-but-no-series**: a note that the run has no saved curve.
/// - **Ready**: the overlay chart + a colour legend.
fn overlay_panel<'a>(model: &Cockpit, mode: ThemeMode) -> Element<'a, Message> {
    let title = Text::new(COMPARE_OVERLAY_TITLE)
        .size(text::SMALL)
        .color(color::FG_2.current(mode));

    let selection = &model.compare_screen_state.overlay_selection;

    if selection.is_empty() {
        // Empty state — prompt the next action.
        let body = Container::new(
            Text::new(COMPARE_OVERLAY_EMPTY)
                .size(text::MICRO)
                .color(color::FG_3.current(mode)),
        )
        .width(Length::Fill)
        .height(Length::Fixed(OVERLAY_CHART_HEIGHT_PX))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
        return Column::new()
            .spacing(space::XS)
            .push(title)
            .push(body)
            .into();
    }

    // Resolve each selected slot to its hydrated timestamped series (if any).
    let series: Vec<Option<LabEquitySeries>> = selection
        .iter()
        .map(|slot| resolve_slot_series(model, slot))
        .collect();

    // slot 0 → primary (ACCENT); slot 1 (if present) → compare[0] (ACCENT_2).
    let primary = series.first().cloned().flatten();
    let compare: Vec<LabEquitySeries> = series.get(1).cloned().flatten().into_iter().collect();

    // If a selected run resolved to no series (older report, no companion CSV),
    // surface a note so the missing line is explained — not silently absent.
    let any_missing = series.iter().any(Option::is_none);

    let chart_body = chart::view(
        Vec::new(), // no price bars — standalone equity path self-scales x-axis
        Vec::new(),
        Vec::new(),
        None,
        primary,
        compare,
        mode,
    );

    let chart_container = Container::new(chart_body)
        .width(Length::Fill)
        .height(Length::Fixed(OVERLAY_CHART_HEIGHT_PX));

    // Colour legend — pairs each run's label with its curve colour. Built from
    // the selection order so it always matches the drawn curves.
    let legend = overlay_legend(selection, mode);

    let mut panel = Column::new().spacing(space::XS).push(title).push(legend);

    if any_missing {
        panel = panel.push(
            Text::new(COMPARE_OVERLAY_NO_SERIES)
                .size(text::MICRO)
                .color(color::WARN_500.current(mode)),
        );
    }

    panel.push(chart_container).into()
}

/// Resolve a selection slot to its hydrated `LabEquitySeries` (or `None` when
/// the cell is missing from the cache or carries no companion-CSV series).
fn resolve_slot_series(model: &Cockpit, slot: &OverlaySlot) -> Option<LabEquitySeries> {
    let cell = model.compare_screen_state.cache.get(slot)?;
    LabEquitySeries::from_samples(
        cell.equity_series_ts.clone(),
        cell.source_report_path.clone(),
    )
}

/// Build the overlay colour legend (T2). Slot 0 → `ACCENT` "Run A"; slot 1 →
/// `ACCENT_2` "Run B", each annotated with its `strategy × symbol` identity so
/// the operator knows which curve is which.
fn overlay_legend<'a>(selection: &[OverlaySlot], mode: ThemeMode) -> Element<'a, Message> {
    let mut row = Row::new().spacing(space::M);
    for (i, (strategy, symbol, _range)) in selection.iter().enumerate() {
        let (swatch_color, slot_label) = if i == 0 {
            (color::ACCENT.current(mode), COMPARE_OVERLAY_LEGEND_PRIMARY)
        } else {
            (
                color::ACCENT_2.current(mode),
                COMPARE_OVERLAY_LEGEND_COMPARE,
            )
        };
        // A short colour swatch (a coloured bullet) + the run identity.
        let swatch = Text::new(COMPARE_OVERLAY_LEGEND_SWATCH)
            .size(text::MICRO)
            .color(swatch_color);
        let label = Text::new(format!("{slot_label}: {strategy} \u{00b7} {symbol}"))
            .size(text::MICRO)
            .color(color::FG_3.current(mode));
        row = row.push(
            Row::new()
                .spacing(space::XXS)
                .align_y(iced::alignment::Vertical::Center)
                .push(swatch)
                .push(label),
        );
    }
    row.into()
}

// ── Toolbar helpers ───────────────────────────────────────────────────────────

/// Build the date-range preset chip row (R6.4 — same presets as Lab).
fn build_range_chips(selected: &DateRange, mode: ThemeMode) -> Element<'_, Message> {
    use crate::lab::state::Preset;
    use crate::theme::radius;
    use iced::Border;
    use iced::widget::{Button, button};

    let presets = [
        (Preset::Last30d, DateRange::Preset(Preset::Last30d)),
        (Preset::Last90d, DateRange::Preset(Preset::Last90d)),
        (Preset::H1_2024, DateRange::Preset(Preset::H1_2024)),
        (Preset::H2_2024, DateRange::Preset(Preset::H2_2024)),
    ];

    let mut row = Row::new().spacing(space::XS);
    for (preset, range) in presets {
        let is_active = selected == &range;
        let label = preset.label();
        let range_clone = range;
        let btn = Button::new(Text::new(label).size(text::MICRO).color(if is_active {
            color::ACCENT.current(mode)
        } else {
            color::FG_3.current(mode)
        }))
        .on_press(Message::CompareSelectRange(range_clone))
        .padding([2u16, space::XS as u16])
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
    row.into()
}

/// Build the KPI-axis chip row (R6.3 — 5 chips; only Sharpe wired at v0.1.0).
fn build_kpi_chips(selected: CompareKpiAxis, mode: ThemeMode) -> Element<'static, Message> {
    use crate::theme::radius;
    use iced::Border;
    use iced::widget::{Button, button};

    let axes = [
        CompareKpiAxis::Sharpe,
        CompareKpiAxis::Sortino,
        CompareKpiAxis::TotalReturn,
        CompareKpiAxis::MaxDrawdown,
        CompareKpiAxis::WinRate,
    ];

    let mut row = Row::new().spacing(space::XS);
    for axis in axes {
        let is_active = selected == axis;
        let label = axis.label();
        let btn = Button::new(Text::new(label).size(text::MICRO).color(if is_active {
            color::ACCENT.current(mode)
        } else {
            color::FG_3.current(mode)
        }))
        .on_press(Message::CompareSelectKpiAxis(axis))
        .padding([2u16, space::XS as u16])
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
    row.into()
}
