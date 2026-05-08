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

use iced::widget::{button, Button, Column, Row, Scrollable, Text};
use iced::{Border, Element};

use crate::state::{Cockpit, Message, PanelState, StrategyRow, StrategyStatus};
use crate::strings::{
    PANEL_STRATEGIES_TITLE, PLACEHOLDER_NONE, STRATEGIES_COL_HASH, STRATEGIES_COL_ID,
    STRATEGIES_COL_LAST_EVENT, STRATEGIES_COL_POSITION, STRATEGIES_COL_SIGNALS_60S,
    STRATEGIES_COL_STATUS, STRATEGIES_EMPTY, STRATEGIES_ERROR_PREFIX, STRATEGIES_EVENT_LOAD,
    STRATEGIES_EVENT_REJECT, STRATEGIES_EVENT_SWAP, STRATEGIES_EVENT_UNLOAD, STRATEGIES_LOADING,
    STRATEGIES_POSITION_FLAT, STRATEGIES_POSITION_HELD, STRATEGIES_STATUS_ERROR,
    STRATEGIES_STATUS_LOADING, STRATEGIES_STATUS_READY, STRATEGY_PAUSE_LABEL,
    STRATEGY_RESUME_LABEL,
};
use crate::theme::{color, radius, space, text, ThemeMode};

use super::focus_ring;
use super::frame::{active_row, col_header, error_body, muted_body, panel};
use trading_core::{StrategyEventKind, StrategyEventView, StrategyId};

/// Render the strategies panel.
#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.strategies {
        PanelState::Loading => muted_body(STRATEGIES_LOADING),
        PanelState::Empty => muted_body(STRATEGIES_EMPTY),
        PanelState::Error(e) => error_body(STRATEGIES_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(rows) => ready_body(
            rows,
            &model.strategies_recent_events,
            model.selected_strategy.as_ref(),
        ),
    };
    panel(PANEL_STRATEGIES_TITLE, body, ThemeMode::Dark)
}

fn ready_body<'a>(
    rows: &'a [StrategyRow],
    recent_events: &'a std::collections::VecDeque<StrategyEventView>,
    selected: Option<&'a trading_core::StrategyId>,
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
        let is_active = selected == Some(&r.id);
        table = table.push(row_for(r, is_active));
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

fn row_for(r: &StrategyRow, is_active: bool) -> Element<'_, Message> {
    let (status_label, status_color) = match &r.status {
        StrategyStatus::Ready => (
            STRATEGIES_STATUS_READY,
            color::UP_500.current(ThemeMode::Dark),
        ),
        StrategyStatus::Loading => (
            STRATEGIES_STATUS_LOADING,
            color::FG_3.current(ThemeMode::Dark),
        ),
        StrategyStatus::Error(_) => (
            STRATEGIES_STATUS_ERROR,
            color::DOWN_500.current(ThemeMode::Dark),
        ),
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

    let row_content = Row::new()
        .push(cell(r.id.to_string()))
        .push(cell(hash_label))
        .push(colored_cell(status_label.to_string(), status_color))
        .push(cell(last_event.to_string()))
        .push(cell(signals_label))
        .push(cell(position_label.to_string()))
        .spacing(space::M);

    // T1705 — Phase 3 cross-link: each row is a button that emits
    // `Message::SelectStrategy(row.id.clone())` on press. The binary's
    // update wrapper chains `Task::done(SwitchScreen(Strategies))` per
    // R5.2 / Q11b compound dispatch when the click came from Home.
    let row_button = Button::new(row_content)
        .on_press(Message::SelectStrategy(r.id.clone()))
        .padding(0)
        .style(move |_theme: &iced::Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => {
                    Some(color::PANEL_SUNKEN.current(ThemeMode::Dark).into())
                }
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: color::FG_1.current(ThemeMode::Dark),
                border: Border {
                    radius: radius::R2.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    // T1507: 2 px ACCENT left rule for the active (selected) row.
    active_row(row_button.into(), is_active, ThemeMode::Dark)
}

/// Red-tinted one-line error badge rendered under a `StrategyStatus::Error`
/// row. Reuses the semantic `NEG` color so it reads as "danger" without
/// competing with the header-level panel error state.
fn error_badge(summary: &str) -> Element<'_, Message> {
    Text::new(summary.to_string())
        .size(text::MICRO)
        .color(color::DOWN_500.current(ThemeMode::Dark))
        .into()
}

fn event_row(ev: &StrategyEventView) -> Element<'_, Message> {
    let (label, c) = match ev.kind {
        StrategyEventKind::Load => (
            STRATEGIES_EVENT_LOAD,
            color::ACCENT.current(ThemeMode::Dark),
        ),
        StrategyEventKind::Swap => (
            STRATEGIES_EVENT_SWAP,
            color::WARN_500.current(ThemeMode::Dark),
        ),
        StrategyEventKind::Unload => (
            STRATEGIES_EVENT_UNLOAD,
            color::FG_3.current(ThemeMode::Dark),
        ),
        StrategyEventKind::Reject | StrategyEventKind::RebalanceRejected => (
            STRATEGIES_EVENT_REJECT,
            color::DOWN_500.current(ThemeMode::Dark),
        ),
        // v1.5a Q8 — new strategy event kinds (MeanReversionStop /
        // PairShortObservation), and v1+ Q8 / R7.1 operator-success-report
        // sources (KillSwitchTripped / FeedReconnect); rendered as
        // informational events. Phase 5 (T1902) — operator-write
        // variants (StrategyPaused / RiskVetoOverridden) join the same
        // informational bucket — operator decisions, not strategy errors.
        StrategyEventKind::MeanReversionStop
        | StrategyEventKind::PairShortObservation
        | StrategyEventKind::KillSwitchTripped
        | StrategyEventKind::FeedReconnect
        | StrategyEventKind::StrategyPaused
        | StrategyEventKind::RiskVetoOverridden => {
            (STRATEGIES_EVENT_LOAD, color::FG_3.current(ThemeMode::Dark))
        }
    };
    let id = ev
        .strategy_id
        .as_ref()
        .map_or_else(|| PLACEHOLDER_NONE.to_string(), ToString::to_string);
    Row::new()
        .push(Text::new(label).size(text::MICRO).color(c))
        .push(
            Text::new(id)
                .size(text::MICRO)
                .color(color::FG_1.current(ThemeMode::Dark)),
        )
        .spacing(space::S)
        .into()
}

fn cell<'a>(s: String) -> Element<'a, Message> {
    Text::new(s)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .into()
}

fn colored_cell<'a>(s: String, c: iced::Color) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(c).into()
}

/// Plain-language label for a strategy event kind. Exposed `pub(crate)` so
/// the snapshot helper in `tests/panel_snapshots.rs` renders the exact same
/// text the panel does.
pub(crate) fn event_kind_label(ev: &StrategyEventView) -> &'static str {
    match ev.kind {
        // v1.5a Q8 informational kinds (MeanReversionStop /
        // PairShortObservation) and v1+ Q8 / R7.1 operator-success-report
        // sources (KillSwitchTripped / FeedReconnect) all render under the
        // generic Load label in the cockpit (informational, no error
        // styling).
        StrategyEventKind::Load
        | StrategyEventKind::MeanReversionStop
        | StrategyEventKind::PairShortObservation
        | StrategyEventKind::KillSwitchTripped
        | StrategyEventKind::FeedReconnect
        | StrategyEventKind::StrategyPaused
        | StrategyEventKind::RiskVetoOverridden => STRATEGIES_EVENT_LOAD,
        StrategyEventKind::Swap => STRATEGIES_EVENT_SWAP,
        StrategyEventKind::Unload => STRATEGIES_EVENT_UNLOAD,
        StrategyEventKind::Reject | StrategyEventKind::RebalanceRejected => STRATEGIES_EVENT_REJECT,
    }
}

// ── Phase 5 — pause/resume per-strategy button (T1907) ──────────────────────

/// Render the Pause / Resume button for a single strategy (T1907 /
/// R4.3). Single-click both directions per Q8 — no typed-confirm gate
/// (pause is bounded-destructive: skips future signals; doesn't reverse
/// past decisions). Click emits `Message::StrategyPauseToggled(id)`.
///
/// Wraps in `focus_ring::wrap(...)` per TD-1 path b — the per-strategy
/// focus key is `strategy_pause::<strategy_id>` (see
/// `widgets::focus_ring::strategy_pause_id`).
// `cast_possible_truncation`: `space::*` constants are `u32` with bounded
// values 0..64; cast to `u16` padding is safe by construction.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn pause_button<'a>(
    id: &'a StrategyId,
    paused: bool,
    focused: Option<&'a str>,
) -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    let label = if paused {
        STRATEGY_RESUME_LABEL
    } else {
        STRATEGY_PAUSE_LABEL
    };
    let btn = Button::new(
        Text::new(label)
            .size(text::SMALL)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::StrategyPauseToggled(id.clone()))
    .padding([space::XS as u16, space::S as u16])
    .style(move |_theme: &iced::Theme, _status: button::Status| {
        let bg = if paused {
            color::PANEL_RAISED.current(mode)
        } else {
            color::PANEL.current(mode)
        };
        button::Style {
            background: Some(bg.into()),
            text_color: color::FG_1.current(mode),
            border: Border {
                color: color::BORDER_2.current(mode),
                width: 1.0,
                radius: radius::R2.into(),
            },
            ..Default::default()
        }
    });
    let focus_key = focus_ring::strategy_pause_id(id.0.as_str());
    let is_focused = focused == Some(focus_key.as_str());
    focus_ring::wrap(focus_key.as_str(), btn.into(), is_focused, mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_strategy_button_label_when_idle_reads_pause() {
        let id = StrategyId::new("alpha");
        let _: Element<'_, Message> = pause_button(&id, false, None);
        // Visual contract locked via panel snapshots at T1913. Compile-
        // time assertion + the state::tests::strategy_pause_toggled_*
        // tests cover round-trip behavior.
    }

    #[test]
    fn pause_strategy_button_toggles_via_state_round_trip() {
        // Locks the round-trip: click → membership flip → re-click →
        // membership flip back. Mirrors the existing strategy_pause
        // tests in state::tests; this lives here as a widget-side
        // smoke test to surface signature regressions early.
        let mut c = Cockpit::new();
        let id = StrategyId::new("alpha");
        crate::state::update(&mut c, Message::StrategyPauseToggled(id.clone()));
        assert!(c.paused_strategies.contains(&id));
        crate::state::update(&mut c, Message::StrategyPauseToggled(id.clone()));
        assert!(!c.paused_strategies.contains(&id));
    }
}
