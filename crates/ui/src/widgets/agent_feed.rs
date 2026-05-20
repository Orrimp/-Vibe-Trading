#![allow(clippy::cast_possible_truncation)]
//! Live agent activity feed — last 200 fills, most recent on top (R6.2).
//!
//! Phase 5 (Lumen Q6 / R11) renamed this module from `tape.rs` to
//! `agent_feed.rs` to match the Lumen `AgentFeed.jsx` reference; the
//! widget body / state contract is unchanged. **Field on Cockpit is
//! preserved as `Cockpit::tape` per Phase 5 Q14 — see
//! `lumen-phase-5-humancontrol-agentfeed.md` / Cockpit state diff.**
//!
//! Supports empty / loading / error / ready states (R6.4). Pause button
//! buffers incoming fills without dropping them (see `state::update`
//! `Message::TapePauseToggled`).
//!
//! ## Row click → audit modal (`tape-row-audit-modal`)
//!
//! Each row is wrapped in a transparent `Button` that emits
//! `Message::TapeRowClicked(fill.transaction_id.clone())` on press. The
//! button's style strips the default chrome so the rendered feed is
//! visually identical to the pre-modal world (R11 / V7); the click
//! handler is a pure interaction wrapper.

use iced::Element;
use iced::Length;
use iced::widget::{Button, Column, Row, Scrollable, Text, button};

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    PANEL_AGENT_FEED_TITLE, SIDE_BUY, SIDE_SELL, TAPE_COL_FEE, TAPE_COL_PRICE, TAPE_COL_QTY,
    TAPE_COL_SIDE, TAPE_COL_SYMBOL, TAPE_COL_TIME, TAPE_EMPTY, TAPE_ERROR_PREFIX, TAPE_LOADING,
    TAPE_PAUSE_LABEL, TAPE_PAUSED_BANNER, TAPE_RESUME_LABEL,
};
use crate::theme::{ThemeMode, color, space, text};

use super::frame::{col_header, error_body, loading_with_spinner, muted_body, panel};
use super::num::{fmt_price, fmt_qty, fmt_usdt};

/// Render the live agent-activity feed panel.
#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.tape {
        PanelState::Loading => loading_with_spinner(TAPE_LOADING, ThemeMode::Dark),
        PanelState::Empty => muted_body(TAPE_EMPTY),
        PanelState::Error(e) => error_body(TAPE_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(fills) => ready_body(model, fills),
    };

    panel(PANEL_AGENT_FEED_TITLE, body, ThemeMode::Dark)
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
                .size(text::MICRO)
                .color(color::WARN_500.current(ThemeMode::Dark)),
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
        trading_core::Side::Buy => color::UP_500.current(ThemeMode::Dark),
        trading_core::Side::Sell => color::DOWN_500.current(ThemeMode::Dark),
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
    // `panel_snapshots__agent_feed_*` snapshots stay byte-identical (R11 / V7).
    let row_btn = Button::new(row_content)
        .on_press(Message::TapeRowClicked(fill.transaction_id.clone()))
        .padding(0)
        .style(transparent_row_button);

    // Phase D R5.1 — adjacent Trail chevron button (Q5 = every row).
    // Emits `Message::OpenTrailFor(audit_id)` which compound-dispatches
    // to `SwitchScreen(Trail)` + `SelectTrailRow(id)`.
    let trail_audit_id = fill.transaction_id.clone();
    let chevron = Button::new(
        Text::new("›")
            .size(text::BODY)
            .color(color::ACCENT.current(ThemeMode::Dark)),
    )
    .on_press(Message::OpenTrailFor(trail_audit_id))
    .padding([0, space::XS as u16])
    .style(transparent_row_button);

    Row::new()
        .push(row_btn)
        .push(chevron)
        .spacing(space::XS)
        .into()
}

/// Transparent row-button style — no background, no border. The button
/// is a click affordance only; the tape's rendered text shape is
/// unchanged from the pre-modal world.
fn transparent_row_button(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: color::FG_1.current(ThemeMode::Dark),
        border: iced::Border::default(),
        ..Default::default()
    }
}

fn cell<'a>(s: String) -> Element<'a, Message> {
    Text::new(s)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .into()
}

/// Render a timestamp as `HH:MM:SS` (UTC). Kept private to the tape — the
/// P&L "as of" label does its own formatting.
fn short_time(ts: trading_core::Timestamp) -> String {
    let dt = ts.inner();
    format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second())
}
