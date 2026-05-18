//! P&L card — cash, unrealized, realized, total equity, daily return (R6.2).
//!
//! Numbers come from `audit::query` (R3.6), never from a cockpit-local
//! accumulator. Under fixtures, the feed is in `ui::fixtures`.

use iced::Element;
use iced::widget::{Column, Row, Text};

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    PANEL_PNL_TITLE, PNL_EMPTY, PNL_ERROR_PREFIX, PNL_LABEL_CASH, PNL_LABEL_DAILY_RETURN,
    PNL_LABEL_EQUITY, PNL_LABEL_REALIZED, PNL_LABEL_UNREALIZED, PNL_LOADING,
};
use crate::theme::{ThemeMode, color, color_for_delta, space, text};

use super::frame::{error_body, loading_with_spinner, muted_body, panel};
use super::num::{fmt_usdt, fmt_usdt_signed};

#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.pnl {
        PanelState::Loading => loading_with_spinner(PNL_LOADING, ThemeMode::Dark),
        PanelState::Empty => muted_body(PNL_EMPTY),
        PanelState::Error(e) => error_body(PNL_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(snap) => ready_body(snap),
    };
    panel(PANEL_PNL_TITLE, body, ThemeMode::Dark)
}

fn ready_body(snap: &trading_core::PnlSnapshot) -> Element<'_, Message> {
    // The display row is the eye-grabber.
    let daily_color = color_for_delta(snap.daily_return.amount());
    let equity_row = Row::new()
        .push(
            Text::new(PNL_LABEL_EQUITY)
                .size(text::MICRO)
                .color(color::FG_3.current(ThemeMode::Dark)),
        )
        .push(
            Text::new(fmt_usdt(snap.total_equity.amount()))
                .size(text::DISPLAY)
                .color(color::FG_1.current(ThemeMode::Dark)),
        )
        .spacing(space::M);

    Column::new()
        .spacing(space::S)
        .push(equity_row)
        .push(row(
            PNL_LABEL_DAILY_RETURN,
            fmt_usdt_signed(snap.daily_return.amount()),
            Some(daily_color),
        ))
        .push(row(PNL_LABEL_CASH, fmt_usdt(snap.cash.amount()), None))
        .push(row(
            PNL_LABEL_UNREALIZED,
            fmt_usdt_signed(snap.unrealized.amount()),
            Some(color_for_delta(snap.unrealized.amount())),
        ))
        .push(row(
            PNL_LABEL_REALIZED,
            fmt_usdt_signed(snap.realized.amount()),
            Some(color_for_delta(snap.realized.amount())),
        ))
        .into()
}

fn row(label: &str, value: String, value_color: Option<iced::Color>) -> Element<'_, Message> {
    let value_el = Text::new(value)
        .size(text::BODY)
        .color(value_color.unwrap_or_else(|| color::FG_1.current(ThemeMode::Dark)));
    Row::new()
        .push(
            Text::new(label)
                .size(text::BODY)
                .color(color::FG_3.current(ThemeMode::Dark)),
        )
        .push(value_el)
        .spacing(space::M)
        .into()
}
