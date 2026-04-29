//! Strategies panel — loaded strategies + recent swap log (R5, Q4 resolution).
//!
//! Placement: right column, **above** Open positions. The table carries one
//! row per loaded strategy with the id, a short 7-char hash, a status pill,
//! the last strategy event, a rolling 60s signal count, and a "Holds position"
//! column. Below the table a compact footer lists the last ten
//! `StrategyEventView`s colored by event kind (Load → `ACCENT`,
//! Swap → `WARN`, Reject → `NEG`).
//!
//! State semantics (`PanelState`):
//! - `Loading` → "Loading active strategies…" in `FG_MUTED`.
//! - `Empty` → "No strategies loaded. Drop a TOML under config/strategies/ to
//!   begin." in `FG_MUTED`.
//! - `Error(e)` → `STRATEGIES_ERROR_PREFIX` + `e` in `NEG`.
//! - `Ready(rows)` → the table + footer. Rows whose own `status` is `Error`
//!   render a per-row error badge with the `error_summary` beneath them —
//!   this is the R8 "malformed TOML, old strategy keeps running" visual.
//!
//! Design-system contract: every string flows from `ui::strings`, every color
//! from `ui::theme`. Consistency tests in `crates/ui/tests/consistency.rs`
//! fail the build if a literal sneaks in here.

use iced::widget::{Column, Row, Scrollable, Text};
use iced::Element;

use crate::state::{Cockpit, Message, PanelState, StrategyRow, StrategyStatus};
use crate::strings::{
    PANEL_STRATEGIES_TITLE, PLACEHOLDER_NONE, STRATEGIES_COL_HASH, STRATEGIES_COL_ID,
    STRATEGIES_COL_LAST_EVENT, STRATEGIES_COL_POSITION, STRATEGIES_COL_SIGNALS_60S,
    STRATEGIES_COL_STATUS, STRATEGIES_EMPTY, STRATEGIES_ERROR_PREFIX, STRATEGIES_EVENT_LOAD,
    STRATEGIES_EVENT_REJECT, STRATEGIES_EVENT_SWAP, STRATEGIES_EVENT_UNLOAD, STRATEGIES_LOADING,
    STRATEGIES_POSITION_FLAT, STRATEGIES_POSITION_HELD, STRATEGIES_STATUS_ERROR,
    STRATEGIES_STATUS_LOADING, STRATEGIES_STATUS_READY,
};
use crate::theme::{color, space, text};

use super::frame::{col_header, error_body, muted_body, panel};
use trading_core::{StrategyEventKind, StrategyEventView};

/// Render the strategies panel.
#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.strategies {
        PanelState::Loading => muted_body(STRATEGIES_LOADING),
        PanelState::Empty => muted_body(STRATEGIES_EMPTY),
        PanelState::Error(e) => error_body(STRATEGIES_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(rows) => ready_body(rows, &model.strategies_recent_events),
    };
    panel(PANEL_STRATEGIES_TITLE, body)
}

fn ready_body<'a>(
    rows: &'a [StrategyRow],
    recent_events: &'a std::collections::VecDeque<StrategyEventView>,
) -> Element<'a, Message> {
    let header = Row::new()
        .push(col_header(STRATEGIES_COL_ID))
        .push(col_header(STRATEGIES_COL_HASH))
        .push(col_header(STRATEGIES_COL_STATUS))
        .push(col_header(STRATEGIES_COL_LAST_EVENT))
        .push(col_header(STRATEGIES_COL_SIGNALS_60S))
        .push(col_header(STRATEGIES_COL_POSITION))
        .spacing(space::M);

    let mut table = Column::new().spacing(space::XS);
    for r in rows {
        table = table.push(row_for(r));
        // Per-row error badge — beneath the main row so the table lines up.
        if let StrategyStatus::Error(summary) = &r.status {
            table = table.push(error_badge(summary.as_str()));
        }
    }

    let scroll: Element<Message> = Scrollable::new(table).into();

    // Footer: recent events, newest first. Keep it compact — caption-sized
    // monospace-ish so it scans like a log.
    let mut footer = Column::new().spacing(space::XS);
    for ev in recent_events {
        footer = footer.push(event_row(ev));
    }

    Column::new()
        .spacing(space::S)
        .push(header)
        .push(scroll)
        .push(footer)
        .into()
}

fn row_for(r: &StrategyRow) -> Element<'_, Message> {
    let (status_label, status_color) = match &r.status {
        StrategyStatus::Ready => (STRATEGIES_STATUS_READY, color::POS),
        StrategyStatus::Loading => (STRATEGIES_STATUS_LOADING, color::FG_MUTED),
        StrategyStatus::Error(_) => (STRATEGIES_STATUS_ERROR, color::NEG),
    };
    let last_event = r
        .last_event
        .as_ref()
        .map_or(PLACEHOLDER_NONE, event_kind_label);
    let position_label = if r.has_position {
        STRATEGIES_POSITION_HELD
    } else {
        STRATEGIES_POSITION_FLAT
    };
    let hash_label = if r.short_hash.is_empty() {
        PLACEHOLDER_NONE.to_string()
    } else {
        r.short_hash.to_string()
    };
    let signals_label = if matches!(r.status, StrategyStatus::Loading) && r.signals_60s == 0 {
        PLACEHOLDER_NONE.to_string()
    } else {
        r.signals_60s.to_string()
    };

    Row::new()
        .push(cell(r.id.to_string()))
        .push(cell(hash_label))
        .push(colored_cell(status_label.to_string(), status_color))
        .push(cell(last_event.to_string()))
        .push(cell(signals_label))
        .push(cell(position_label.to_string()))
        .spacing(space::M)
        .into()
}

/// Red-tinted one-line error badge rendered under a `StrategyStatus::Error`
/// row. Reuses the semantic `NEG` color so it reads as "danger" without
/// competing with the header-level panel error state.
fn error_badge(summary: &str) -> Element<'_, Message> {
    Text::new(summary.to_string())
        .size(text::CAPTION)
        .color(color::NEG)
        .into()
}

fn event_row(ev: &StrategyEventView) -> Element<'_, Message> {
    let (label, c) = match ev.kind {
        StrategyEventKind::Load => (STRATEGIES_EVENT_LOAD, color::ACCENT),
        StrategyEventKind::Swap => (STRATEGIES_EVENT_SWAP, color::WARN),
        StrategyEventKind::Unload => (STRATEGIES_EVENT_UNLOAD, color::FG_MUTED),
        StrategyEventKind::Reject | StrategyEventKind::RebalanceRejected => {
            (STRATEGIES_EVENT_REJECT, color::NEG)
        }
    };
    let id = ev
        .strategy_id
        .as_ref()
        .map_or_else(|| PLACEHOLDER_NONE.to_string(), ToString::to_string);
    Row::new()
        .push(Text::new(label).size(text::CAPTION).color(c))
        .push(Text::new(id).size(text::CAPTION).color(color::FG))
        .spacing(space::S)
        .into()
}

fn cell<'a>(s: String) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(color::FG).into()
}

fn colored_cell<'a>(s: String, c: iced::Color) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(c).into()
}

/// Plain-language label for a strategy event kind. Exposed `pub(crate)` so
/// the snapshot helper in `tests/panel_snapshots.rs` renders the exact same
/// text the panel does.
pub(crate) fn event_kind_label(ev: &StrategyEventView) -> &'static str {
    match ev.kind {
        StrategyEventKind::Load => STRATEGIES_EVENT_LOAD,
        StrategyEventKind::Swap => STRATEGIES_EVENT_SWAP,
        StrategyEventKind::Unload => STRATEGIES_EVENT_UNLOAD,
        StrategyEventKind::Reject | StrategyEventKind::RebalanceRejected => STRATEGIES_EVENT_REJECT,
    }
}
