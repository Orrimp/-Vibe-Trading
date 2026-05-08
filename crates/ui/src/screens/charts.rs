//! Charts screen — Phase 2 (T1610).
//!
//! Composes the symbol-selector chip row + price-chart canvas. Reads
//! `model.universe` for the chip row, `model.chart_buffer` for the
//! line series, and `model.chart_markers` for the buy/sell triangles.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{button, Button, Column, Row, Text};
use iced::{Border, Length};

use crate::state::{Cockpit, Message, PanelState};
use crate::theme::{color, radius, space, text, ThemeMode};
use crate::widgets::{chart, frame};

/// Render the Charts screen body.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let active = model
        .selected_symbol
        .clone()
        .or_else(|| model.universe.first().cloned());

    let mut chip_row = Row::new().spacing(space::S);
    for (venue, symbol) in &model.universe {
        let pair_active = match &active {
            Some((av, asym)) => av == venue && asym == symbol,
            None => false,
        };
        let label = format!("{venue} \u{00b7} {symbol}");
        let text_widget = Text::new(label).size(text::SMALL).color(if pair_active {
            color::FG_1.current(mode)
        } else {
            color::FG_2.current(mode)
        });
        let chip_button = Button::new(text_widget)
            .on_press(Message::SelectSymbol(*venue, symbol.clone()))
            .padding([space::XS as u16, space::M as u16])
            .style(move |_theme: &iced::Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                    _ => None,
                };
                button::Style {
                    background: bg,
                    text_color: if pair_active {
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
        chip_row = chip_row.push(frame::active_chip(chip_button.into(), pair_active, mode));
    }

    let chart_body = if let Some((venue, symbol)) = active {
        let bars: Vec<_> = model.chart_buffer.bars(venue, &symbol).cloned().collect();
        let markers: Vec<_> = match &model.chart_markers {
            PanelState::Ready(v) => v.clone(),
            _ => Vec::new(),
        };
        chart::view(bars, markers, mode)
    } else {
        chart::view(Vec::new(), Vec::new(), mode)
    };

    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(chip_row)
        .push(chart_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
