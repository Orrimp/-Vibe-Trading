//! Position panel — open positions with cost basis, mark, P&L, exposure (R6.2).
//! Positions with zero qty are hidden; that's the "empty" state copy job.

use iced::widget::{Column, Row, Scrollable, Text};
use iced::Element;
use rust_decimal::Decimal;

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    PANEL_POSITIONS_TITLE, POS_COL_COST, POS_COL_EXPOSURE, POS_COL_MARK, POS_COL_PNL,
    POS_COL_PNL_PCT, POS_COL_QTY, POS_COL_SYMBOL, POS_EMPTY, POS_ERROR_PREFIX, POS_LOADING,
};
use crate::theme::{color, color_for_delta, space, text};

use super::frame::{col_header, error_body, muted_body, panel};
use super::num::{fmt_pct, fmt_price, fmt_qty, fmt_usdt_signed};

#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.positions {
        PanelState::Loading => muted_body(POS_LOADING),
        PanelState::Empty => muted_body(POS_EMPTY),
        PanelState::Error(e) => error_body(POS_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(positions) => ready_body(positions),
    };
    panel(PANEL_POSITIONS_TITLE, body)
}

fn ready_body(positions: &[trading_core::PositionView]) -> Element<'_, Message> {
    // Hide zero-quantity positions per task T17 acceptance.
    let visible: Vec<&trading_core::PositionView> =
        positions.iter().filter(|p| !p.base_qty.is_zero()).collect();

    if visible.is_empty() {
        return muted_body(POS_EMPTY);
    }

    let header = Row::new()
        .push(col_header(POS_COL_SYMBOL))
        .push(col_header(POS_COL_QTY))
        .push(col_header(POS_COL_COST))
        .push(col_header(POS_COL_MARK))
        .push(col_header(POS_COL_PNL))
        .push(col_header(POS_COL_PNL_PCT))
        .push(col_header(POS_COL_EXPOSURE))
        .spacing(space::M);

    let mut rows = Column::new().spacing(space::XS);
    for p in visible {
        rows = rows.push(row_for(p));
    }

    let scroll: Element<Message> = Scrollable::new(rows).into();

    Column::new()
        .spacing(space::S)
        .push(header)
        .push(scroll)
        .into()
}

fn row_for(p: &trading_core::PositionView) -> Element<'_, Message> {
    let pnl_color = color_for_delta(p.pnl.amount());
    let pnl_pct_color = color_for_delta(p.pnl_pct);
    Row::new()
        .push(cell(p.symbol.0.to_string()))
        .push(cell(fmt_qty(p.base_qty)))
        .push(cell(fmt_price(p.cost_basis.amount())))
        .push(cell(fmt_price(p.last_mark.get())))
        .push(colored_cell(fmt_usdt_signed(p.pnl.amount()), pnl_color))
        .push(colored_cell(fmt_pct(p.pnl_pct), pnl_pct_color))
        .push(cell(fmt_pct(p.exposure_pct)))
        .spacing(space::M)
        .into()
}

fn cell<'a>(s: String) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(color::FG).into()
}

fn colored_cell<'a>(s: String, c: iced::Color) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(c).into()
}

// Kept available for an exposure-pill or column sorting hook later.
#[allow(dead_code)]
fn warn_if_over<'a>(s: String, value: Decimal, cap: Decimal) -> Element<'a, Message> {
    let c = if value > cap { color::WARN } else { color::FG };
    Text::new(s).size(text::BODY).color(c).into()
}
