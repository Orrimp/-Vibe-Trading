//! Strategy registry screen — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! List-of-cards layout — one `widgets::strategy_card` per registered strategy
//! from `Cockpit::strategies` (`PanelState::Ready(rows)`) (R3.1 / R3.2).
//!
//! States:
//! - `Loading` → `frame::loading_with_spinner(STRATEGIES_LOADING, mode)`.
//! - `Empty` → `frame::muted_body(STRATEGY_REGISTRY_EMPTY)` (R3.6).
//! - `Error(e)` → `frame::error_body(STRATEGIES_ERROR_PREFIX, e)`.
//! - `Ready(rows)` → vertical card stack (R3.2).
//!
//! Panel chrome: `widgets::frame::panel(STRATEGY_REGISTRY_PANEL_TITLE, …, mode)`.
//!
//! Anchor lookup: `None` for all rows at Phase C (A11 — no `Cockpit` field
//! mirrors `evidence/anchors.toml`; Phase D adds the lookup).
//!
//! Last-run timestamp: newest entry in `Cockpit::strategies_recent_events`
//! with matching `strategy_id` (existing field at `state.rs:698`).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names
)]

use iced::Length;
use iced::widget::{Column, Container, Scrollable};
use trading_core::{StrategyId, Timestamp};

use crate::state::{Cockpit, PanelState, StrategyConfigEntry, StrategyRow};
use crate::strings::{
    STRATEGIES_ERROR_PREFIX, STRATEGIES_LOADING, STRATEGY_REGISTRY_EMPTY,
    STRATEGY_REGISTRY_PANEL_TITLE,
};
use crate::theme::{ThemeMode, layout, space};
use crate::widgets::frame::{self, panel};
use crate::widgets::strategy_card;

/// Render the Strategy registry screen body (R3.1).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let body: iced::Element<'_, crate::state::Message> = match &model.strategies {
        PanelState::Loading => frame::loading_with_spinner(STRATEGIES_LOADING, mode),
        PanelState::Error(e) => frame::error_body(STRATEGIES_ERROR_PREFIX, e.as_str()),
        PanelState::Empty => frame::muted_body(STRATEGY_REGISTRY_EMPTY),
        PanelState::Ready(rows) => ready_body(model, rows, mode),
    };

    let panel_body = Container::new(body)
        .width(Length::Fill)
        .padding(layout::PANEL_PADDING as u16);

    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(panel(
            STRATEGY_REGISTRY_PANEL_TITLE,
            panel_body.into(),
            mode,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn ready_body<'a>(
    model: &'a Cockpit,
    rows: &'a [StrategyRow],
    mode: ThemeMode,
) -> iced::Element<'a, crate::state::Message> {
    let mut col = Column::new().spacing(space::M);

    for row in rows {
        // Config lookup — find the matching entry from `strategies_config`.
        let config: Option<&StrategyConfigEntry> = model
            .strategies_config
            .as_ref()
            .and_then(|cfg| cfg.strategies.iter().find(|e| e.id == row.id));

        // Anchor lookup: None at Phase C (no Cockpit mirror of anchors.toml).
        let last_anchor: Option<(&str, &str)> = None;

        // Last-run timestamp: newest event in strategies_recent_events for this id.
        let last_run_ts: Option<Timestamp> = last_run_for_strategy(&row.id, model);

        let card = strategy_card::view(row, config, last_anchor, last_run_ts, mode);
        col = col.push(card);
    }

    Scrollable::new(col).into()
}

/// Scan `Cockpit::strategies_recent_events` for the newest event with
/// matching `strategy_id` and return its timestamp. Returns `None` when
/// no matching event exists.
fn last_run_for_strategy(id: &StrategyId, model: &Cockpit) -> Option<Timestamp> {
    model
        .strategies_recent_events
        .iter()
        .find(|ev| ev.strategy_id.as_ref() == Some(id))
        .map(|ev| ev.ts)
}
