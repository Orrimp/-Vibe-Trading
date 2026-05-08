//! Strategies-detail screen — Phase 3 (T1704, T1706).
//!
//! Layout (Phase 3 Design § Strategies-detail screen contract):
//!
//! 1. **Strategy chip row.** One chip per loaded strategy from
//!    `Cockpit::strategies` (Phase 1 R5). Each chip is a `button`
//!    carrying `Message::SelectStrategy(row.id.clone())` on press,
//!    wrapped in `frame::active_chip` so the active chip carries the
//!    T1609 bottom-edge accent rule. Top-right of the same row: the
//!    deferred sparkline placeholder per Q6 (T1706).
//! 2. **Params block.** Read-only key-value rows from the selected
//!    strategy's `[[strategy]]` block in `cockpit.strategies_config`.
//! 3. **Recent signal events table.** Newest-first, capped at 50 rows.
//!    Source: `Cockpit::strategies_recent_events` filtered at view time
//!    by `selected_strategy` (Q2 ratification — no new audit writer).
//!
//! Empty / loading states per Design.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

// `cast_possible_truncation`: `space::*` constants are `u32` with bounded
// values 0..64; cast to `u16` for iced padding is safe by construction.
// `elidable_lifetime_names`: explicit `'a` lifetimes document the
// borrow chain across the helper-call seam (`model.* → row` etc.) and
// match the Phase 2 charts/screens precedent.
#![allow(
    clippy::cast_possible_truncation,
    clippy::elidable_lifetime_names,
    clippy::needless_pass_by_value
)]

use iced::widget::{button, Button, Column, Container, Row, Text};
use iced::{Border, Length};

use crate::state::{Cockpit, Message, PanelState, StrategyRow};
use crate::strings::{
    OVERRIDE_RISK_VETO_BUTTON_LABEL, PLACEHOLDER_NONE,
    STRATEGIES_EQUITY_HISTORY_UNAVAILABLE_PREFIX, STRATEGIES_EVENTS_TITLE, STRATEGIES_LOADING,
    STRATEGIES_PANEL_TITLE, STRATEGIES_PARAMS_TITLE, STRATEGIES_SELECT_PROMPT,
    STRATEGIES_SPARKLINE_LOADING, VIEWER_NO_EQUITY_DATA,
};
use crate::theme::{color, layout, radius, space, text, ThemeMode};
use crate::widgets::frame::{self, active_chip, col_header, muted_body, panel};
use crate::widgets::{focus_ring, override_risk_veto, sparkline, strategies as strategies_widget};

/// Render the Strategies-detail screen body.
// `cast_possible_truncation`: `space::*` constants are `u32` with bounded
// values 0..64; cast to `u16` padding is safe.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let body: iced::Element<'_, Message> = match (&model.strategies, &model.strategies_config) {
        // Strategies-config still loading or panel still loading → muted body.
        (_, None) | (PanelState::Loading, _) => muted_body(STRATEGIES_LOADING),
        (PanelState::Error(e), _) => {
            frame::error_body(crate::strings::STRATEGIES_ERROR_PREFIX, e.as_str())
        }
        (PanelState::Empty, _) => muted_body(crate::strings::STRATEGIES_EMPTY),
        (PanelState::Ready(rows), Some(config)) => ready_body(model, rows, config, mode),
    };

    let panel_body = Container::new(body)
        .width(Length::Fill)
        .padding(layout::PANEL_PADDING as u16);

    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(panel(STRATEGIES_PANEL_TITLE, panel_body.into(), mode))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn ready_body<'a>(
    model: &'a Cockpit,
    rows: &'a [StrategyRow],
    config: &'a crate::state::StrategiesConfig,
    mode: ThemeMode,
) -> iced::Element<'a, Message> {
    // Chip row.
    let chip_row = chip_row_with_sparkline(model, rows, mode);

    // Params block (driven by the selected strategy).
    let params_section = params_block(model, config, mode);

    // Recent signal events table (filtered by selected strategy).
    let events_section = events_block(model, mode);

    // Phase 5 (T1908) — per-strategy pause/resume row.
    let pause_section = pause_section(model, rows);

    // Phase 5 (T1910) — surfaced risk-engine veto events with per-veto
    // override buttons.
    let veto_section = veto_section(model);

    // Phase 5 (T1909) — override-risk-veto modal (only renders when the
    // override state is non-Idle; otherwise pass-through).
    let modal_section = modal_section(model);

    let mut col = Column::new()
        .spacing(space::M)
        .push(chip_row)
        .push(params_section)
        .push(events_section)
        .push(pause_section)
        .push(veto_section);

    if let Some(modal) = modal_section {
        col = col.push(modal);
    }

    col.into()
}

/// Phase 5 (T1908) — per-strategy pause/resume row. Renders a row of
/// pause/resume buttons keyed on `Cockpit::paused_strategies`
/// membership. Hidden when no strategies are loaded.
fn pause_section<'a>(model: &'a Cockpit, rows: &'a [StrategyRow]) -> iced::Element<'a, Message> {
    let mut row_el = Row::new().spacing(space::S);
    for r in rows {
        let paused = model.paused_strategies.contains(&r.id);
        row_el = row_el.push(strategies_widget::pause_button(
            &r.id,
            paused,
            model.focused_widget.as_deref(),
        ));
    }
    row_el.into()
}

/// Phase 5 (T1910) — surfaced risk-engine veto rows. Each entry in
/// `Cockpit::risk_veto_events` renders a row with the veto reason +
/// `Override` button. Click emits
/// `Message::OverrideRiskVetoPressed(veto_id)`. Hidden when no vetoes
/// are surfaced (Phase 5 ships over a placeholder feed per Q13).
fn veto_section<'a>(model: &'a Cockpit) -> iced::Element<'a, Message> {
    let mode = ThemeMode::Dark;
    if model.risk_veto_events.is_empty() {
        return iced::widget::Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into();
    }
    let mut col = Column::new().spacing(space::XS);
    for veto in &model.risk_veto_events {
        let reason = Text::new(veto.reason.to_string())
            .size(text::SMALL)
            .color(color::FG_2.current(mode));
        let strategy_label = Text::new(veto.strategy_id.to_string())
            .size(text::SMALL)
            .color(color::FG_3.current(mode));
        let button: iced::Element<'_, Message> = Button::new(
            Text::new(OVERRIDE_RISK_VETO_BUTTON_LABEL)
                .size(text::SMALL)
                .color(color::FG_1.current(mode)),
        )
        .on_press(Message::OverrideRiskVetoPressed(veto.veto_id.clone()))
        .padding([space::XS as u16, space::S as u16])
        .style(
            move |_theme: &iced::Theme, _status: button::Status| button::Style {
                background: Some(color::PANEL_RAISED.current(mode).into()),
                text_color: color::FG_1.current(mode),
                border: Border {
                    color: color::WARN_500.current(mode),
                    width: 1.0,
                    radius: radius::R2.into(),
                },
                ..Default::default()
            },
        )
        .into();
        let focus_key = focus_ring::override_veto_button_id(veto.veto_id.as_str());
        let is_focused = model.focused_widget.as_deref() == Some(focus_key.as_str());
        let wrapped = focus_ring::wrap(focus_key.as_str(), button, is_focused, mode);
        col = col.push(
            Row::new()
                .spacing(space::M)
                .push(strategy_label)
                .push(reason)
                .push(wrapped),
        );
    }
    col.into()
}

fn modal_section<'a>(model: &'a Cockpit) -> Option<iced::Element<'a, Message>> {
    override_risk_veto::modal_view(&model.override_risk_veto, model.focused_widget.as_deref())
}

fn chip_row_with_sparkline<'a>(
    model: &'a Cockpit,
    rows: &'a [StrategyRow],
    mode: ThemeMode,
) -> iced::Element<'a, Message> {
    let mut chips_row = Row::new().spacing(space::S);
    for r in rows {
        let active = model.selected_strategy.as_ref() == Some(&r.id);
        let label = Text::new(r.id.to_string())
            .size(text::SMALL)
            .color(if active {
                color::FG_1.current(mode)
            } else {
                color::FG_2.current(mode)
            });
        let chip_button = Button::new(label)
            .on_press(Message::SelectStrategy(r.id.clone()))
            .padding([space::XS as u16, space::M as u16])
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
                        radius: radius::R3.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });
        chips_row = chips_row.push(active_chip(chip_button.into(), active, mode));
    }
    // Phase 4 (T1811) — sparkline replaces the Phase 3 deferred
    // placeholder. Dispatch on `cockpit.strategy_equity` for the
    // selected strategy.
    let sparkline_slot: iced::Element<'_, Message> = match (
        model.selected_strategy.as_ref(),
        model
            .selected_strategy
            .as_ref()
            .and_then(|id| model.strategy_equity.get(id)),
    ) {
        (Some(_), Some(PanelState::Ready(series))) if !series.points.is_empty() => {
            sparkline::view(series, mode)
        }
        (Some(_), Some(PanelState::Loading) | None) | (None, _) => {
            muted_body(STRATEGIES_SPARKLINE_LOADING)
        }
        (Some(_), Some(PanelState::Empty | PanelState::Ready(_))) => {
            muted_body(VIEWER_NO_EQUITY_DATA)
        }
        (Some(_), Some(PanelState::Error(msg))) => {
            let body = format!("{STRATEGIES_EQUITY_HISTORY_UNAVAILABLE_PREFIX}{msg}");
            iced::widget::Text::new(body)
                .size(text::BODY)
                .color(color::FG_3.current(mode))
                .into()
        }
    };
    let slot = Container::new(sparkline_slot).width(Length::Fixed(160.0));
    Row::new()
        .push(chips_row)
        .push(iced::widget::Space::new().width(Length::Fill).height(0))
        .push(slot)
        .width(Length::Fill)
        .into()
}

fn params_block<'a>(
    model: &'a Cockpit,
    config: &'a crate::state::StrategiesConfig,
    mode: ThemeMode,
) -> iced::Element<'a, Message> {
    let header = Text::new(STRATEGIES_PARAMS_TITLE)
        .size(text::H3)
        .color(color::FG_1.current(mode));

    let body: iced::Element<'_, Message> = match &model.selected_strategy {
        None => muted_body(STRATEGIES_SELECT_PROMPT),
        Some(id) => {
            let entry = config.strategies.iter().find(|e| &e.id == id);
            match entry {
                None => muted_body(STRATEGIES_SELECT_PROMPT),
                Some(entry) => {
                    let mut col = Column::new().spacing(space::XS);
                    let header_row = Row::new()
                        .spacing(space::M)
                        .push(col_header("Key"))
                        .push(col_header("Value"));
                    col = col.push(header_row);
                    for (key, value) in &entry.params {
                        let kv = Row::new()
                            .spacing(space::M)
                            .push(
                                Text::new(key.to_string())
                                    .size(text::BODY)
                                    .color(color::FG_2.current(mode)),
                            )
                            .push(
                                Text::new(value.to_string())
                                    .size(text::BODY)
                                    .color(color::FG_1.current(mode)),
                            );
                        col = col.push(kv);
                    }
                    col.into()
                }
            }
        }
    };

    Column::new()
        .spacing(space::S)
        .push(header)
        .push(body)
        .into()
}

fn events_block<'a>(model: &'a Cockpit, mode: ThemeMode) -> iced::Element<'a, Message> {
    let header = Text::new(STRATEGIES_EVENTS_TITLE)
        .size(text::H3)
        .color(color::FG_1.current(mode));
    let mut col = Column::new().spacing(space::XS);
    let mut count = 0usize;
    for ev in &model.strategies_recent_events {
        if let Some(selected) = &model.selected_strategy {
            if ev.strategy_id.as_ref() != Some(selected) {
                continue;
            }
        }
        if count >= 50 {
            break;
        }
        count += 1;
        let kind_label = format!("{:?}", ev.kind);
        let id_label = ev
            .strategy_id
            .as_ref()
            .map_or_else(|| PLACEHOLDER_NONE.to_string(), ToString::to_string);
        let row = Row::new()
            .spacing(space::M)
            .push(
                Text::new(kind_label)
                    .size(text::SMALL)
                    .color(color::FG_3.current(mode)),
            )
            .push(
                Text::new(id_label)
                    .size(text::SMALL)
                    .color(color::FG_2.current(mode)),
            );
        col = col.push(row);
    }
    if count == 0 {
        col = col.push(muted_body(PLACEHOLDER_NONE));
    }
    Column::new()
        .spacing(space::S)
        .push(header)
        .push(col)
        .into()
}
