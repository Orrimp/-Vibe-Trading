//! Live tape panel — last 200 fills, most recent on top (R6.2).
//!
//! Supports empty / loading / error / ready states (R6.4). Pause button
//! buffers incoming fills without dropping them (see `state::update`
//! `Message::TapePauseToggled`).
//!
//! ## Row click → audit modal (`tape-row-audit-modal`)
//!
//! Each row is wrapped in a transparent `Button` that emits
//! `Message::TapeRowClicked(fill.transaction_id.clone())` on press. The
//! button's style strips the default chrome so the rendered tape is
//! visually identical to the pre-modal world (R11 / V7); the click
//! handler is a pure interaction wrapper.

use iced::widget::{button, Button, Column, Row, Scrollable, Text};
use iced::Element;
use iced::Length;

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    PANEL_TAPE_TITLE, SIDE_BUY, SIDE_SELL, TAPE_COL_FEE, TAPE_COL_PRICE, TAPE_COL_QTY,
    TAPE_COL_SIDE, TAPE_COL_SYMBOL, TAPE_COL_TIME, TAPE_EMPTY, TAPE_ERROR_PREFIX, TAPE_LOADING,
    TAPE_PAUSED_BANNER, TAPE_PAUSE_LABEL, TAPE_RESUME_LABEL,
};
use crate::theme::{color, space, text};

use super::frame::{col_header, error_body, muted_body, panel};
use super::num::{fmt_price, fmt_qty, fmt_usdt};

/// Render the live tape panel.
#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.tape {
        PanelState::Loading => muted_body(TAPE_LOADING),
        PanelState::Empty => muted_body(TAPE_EMPTY),
        PanelState::Error(e) => error_body(TAPE_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(fills) => ready_body(model, fills),
    };

    panel(PANEL_TAPE_TITLE, body)
}

fn ready_body<'a>(
    model: &'a Cockpit,
    fills: &'a std::collections::VecDeque<trading_core::FillView>,
) -> Element<'a, Message> {
    let header = Row::new()
        .push(col_header(TAPE_COL_TIME))
        .push(col_header(TAPE_COL_SYMBOL))
        .push(col_header(TAPE_COL_SIDE))
        .push(col_header(TAPE_COL_PRICE))
        .push(col_header(TAPE_COL_QTY))
        .push(col_header(TAPE_COL_FEE))
        .spacing(space::M);

    let mut rows = Column::new().spacing(space::XS);
    for fill in fills {
        rows = rows.push(row_for(fill));
    }

    let scroll: Element<Message> = Scrollable::new(rows).height(Length::Fill).into();

    let pause_label = if model.tape_paused {
        TAPE_RESUME_LABEL
    } else {
        TAPE_PAUSE_LABEL
    };

    let controls = Row::new()
        .push(
            Button::new(Text::new(pause_label).size(text::BODY))
                .on_press(Message::TapePauseToggled),
        )
        .spacing(space::M);

    let mut col = Column::new()
        .spacing(space::S)
        .push(header)
        .push(scroll)
        .push(controls);

    if model.tape_paused {
        col = col.push(
            Text::new(TAPE_PAUSED_BANNER)
                .size(text::CAPTION)
                .color(color::WARN),
        );
    }

    col.into()
}

fn row_for(fill: &trading_core::FillView) -> Element<'_, Message> {
    let side_label = match fill.side {
        trading_core::Side::Buy => SIDE_BUY,
        trading_core::Side::Sell => SIDE_SELL,
    };
    let side_color = match fill.side {
        trading_core::Side::Buy => color::POS,
        trading_core::Side::Sell => color::NEG,
    };
    let row_content = Row::new()
        .push(cell(short_time(fill.venue_ts)))
        .push(cell(fill.symbol.0.to_string()))
        .push(Text::new(side_label).size(text::BODY).color(side_color))
        .push(cell(fmt_price(fill.price.get())))
        .push(cell(fmt_qty(fill.qty.get())))
        .push(cell(fmt_usdt(fill.fee.amount())))
        .spacing(space::M);

    // Wrap each row in a transparent `Button` so the operator can click
    // through to the journal-transaction audit modal. The button strips
    // its default chrome (no background, no border) so the rendered tape
    // is visually identical to the pre-modal world — existing
    // `panel_snapshots__tape_*` snapshots stay byte-identical (R11 / V7).
    Button::new(row_content)
        .on_press(Message::TapeRowClicked(fill.transaction_id.clone()))
        .padding(0)
        .style(transparent_row_button)
        .into()
}

/// Transparent row-button style — no background, no border. The button
/// is a click affordance only; the tape's rendered text shape is
/// unchanged from the pre-modal world.
fn transparent_row_button(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: color::FG,
        border: iced::Border::default(),
        ..Default::default()
    }
}

fn cell<'a>(s: String) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(color::FG).into()
}

/// Render a timestamp as `HH:MM:SS` (UTC). Kept private to the tape — the
/// P&L "as of" label does its own formatting.
fn short_time(ts: trading_core::Timestamp) -> String {
    let dt = ts.inner();
    format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second())
}
