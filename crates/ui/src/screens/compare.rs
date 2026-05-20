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

use crate::compare::state::CompareKpiAxis;
use crate::lab::state::DateRange;
use crate::state::{Cockpit, Message};
use crate::strings::{
    COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE, COMPARE_TOOLBAR_KPI_LABEL, COMPARE_TOOLBAR_RANGE_LABEL,
};
use crate::theme::{ThemeMode, color, space, text};
use crate::widgets::matrix;

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

    col.into()
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
