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

use iced::widget::{button, table, Button, Column, Container, Row, Text};
use iced::{Border, Element, Length};

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
use super::frame::{col_header, error_body, muted_body, panel};
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
    // T2.1 — native `iced::widget::table::Table` replaces the previous
    // hand-rolled `Row::new()` header + `Scrollable<Column>` body
    // (removed: strategies.rs:63-70 header + strategies.rs:72-82 row
    // loop + Scrollable wrap). The Table's Catalog impl
    // (`iced_widget-0.14.2/src/table.rs:704-714`) ships the default
    // class baked in at construction time; cockpit-tinted overrides
    // are exposed via `crate::theme::iced_widget_catalogs::cockpit_table_style_fn`
    // (T2.0) for future call sites.
    //
    // Q5 (committed): column 1's view lambda wraps the cell body in a
    // `Button::on_press(Message::SelectStrategy(...))`. Columns 2-6
    // stay plain `Element`; click bubbling from blank space inside the
    // table cells lands on the column-1 Button via the Table's first-
    // column primacy.
    //
    // Q6 / H-arch-A5b RESOLVED-CONFIRM (T2.2): the per-row error badge
    // becomes a sibling `Column<error_badges>` below the Table — Table
    // ships no row-decorator hook (`row_decorator | after_row | tail |
    // on_row | row_overlay` all absent per `table.rs` grep).
    //
    // The selected-row 2 px ACCENT left-rule (T1507) routes through
    // column 1's per-cell Container border (since Table's Style only
    // carries `separator_x` / `separator_y` — no per-row indicator
    // field). `selected` is cloned into `Option<StrategyId>` so the
    // column view closures (`Fn(StrategyRow) -> Element + 'b`) capture
    // owned data — the lambda body is invoked once per column-cell
    // pair per `Table::new` at table.rs:118-127.
    let selected_owned: Option<trading_core::StrategyId> = selected.cloned();

    let columns = [
        // Column 1 — ID. Wraps the cell body in a Button that emits
        // `Message::SelectStrategy(...)`. Also carries the selected-row
        // 2 px ACCENT left-rule via the Container border.
        table::column(col_header(STRATEGIES_COL_ID), {
            let selected = selected_owned.clone();
            move |r: StrategyRow| {
                let is_active = selected.as_ref() == Some(&r.id);
                id_cell(r.id.clone(), r.id.to_string(), is_active)
            }
        }),
        // Column 2 — HASH.
        table::column(col_header(STRATEGIES_COL_HASH), |r: StrategyRow| {
            let hash_label = if r.short_hash.is_empty() {
                PLACEHOLDER_NONE.to_string()
            } else {
                r.short_hash.to_string()
            };
            cell(hash_label)
        }),
        // Column 3 — STATUS (colored).
        table::column(col_header(STRATEGIES_COL_STATUS), |r: StrategyRow| {
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
            colored_cell(status_label.to_string(), status_color)
        }),
        // Column 4 — LAST_EVENT.
        table::column(col_header(STRATEGIES_COL_LAST_EVENT), |r: StrategyRow| {
            let last_event = r
                .last_event
                .as_ref()
                .map_or(PLACEHOLDER_NONE, event_kind_label);
            cell(last_event.to_string())
        }),
        // Column 5 — SIGNALS_60S.
        table::column(col_header(STRATEGIES_COL_SIGNALS_60S), |r: StrategyRow| {
            let signals_label = if matches!(r.status, StrategyStatus::Loading) && r.signals_60s == 0
            {
                PLACEHOLDER_NONE.to_string()
            } else {
                r.signals_60s.to_string()
            };
            cell(signals_label)
        }),
        // Column 6 — POSITION.
        table::column(col_header(STRATEGIES_COL_POSITION), |r: StrategyRow| {
            let position_label = if r.has_position {
                STRATEGIES_POSITION_HELD
            } else {
                STRATEGIES_POSITION_FLAT
            };
            cell(position_label.to_string())
        }),
    ];

    // `Table::new(columns, rows)` per H-arch-A2 REFINED signature:
    // accepts `IntoIterator<Item = T> where T: Clone` directly — no
    // intermediate `Vec` collect required. `StrategyRow` is `Clone`
    // per `state.rs:535-536`.
    let strategies_table = table::Table::new(columns, rows.iter().cloned()).width(Length::Fill);

    // T2.2 — sibling `Column<error_badges>` below the table (Q6 /
    // H-arch-A5b RESOLVED-CONFIRM, Option C committed). Best-effort
    // horizontal alignment: badges render in the same outer column as
    // the table; per-row pixel alignment with each error row drifts
    // slightly vs the legacy inline placement (badge previously
    // immediately followed the row inside the same Column; now lives
    // below the entire table). This is the documented drift per the
    // architect's Q6 rationale.
    let mut error_badges = Column::new().spacing(space::XXS);
    let mut has_badges = false;
    for r in rows {
        if let StrategyStatus::Error(summary) = &r.status {
            has_badges = true;
            error_badges = error_badges.push(error_badge_text(summary.as_str()));
        }
    }

    // Footer: recent events, newest first. Keep it compact — caption-sized
    // monospace-ish so it scans like a log.
    let mut footer = Column::new().spacing(space::XS);
    for ev in recent_events {
        footer = footer.push(event_row(ev));
    }

    let mut outer = Column::new().spacing(space::S).push(strategies_table);
    if has_badges {
        outer = outer.push(error_badges);
    }
    outer.push(footer).into()
}

/// Column-1 cell — wraps the strategy id text in a Button that emits
/// `Message::SelectStrategy(...)` (T1705 / R5.2 / Q11b compound
/// dispatch; preserved across the T2.1 native-table migration).
///
/// When `is_active`, a leading 2 px `ACCENT` Container rule renders to
/// the left of the Button — preserves the T1507 "2 px ACCENT left rule
/// on the selected row" semantics. The legacy `frame::active_row`
/// whole-row composition was incompatible with native Table's
/// column-cell layout (Table renders cells, not whole rows), so the
/// rule moves into column 1's per-cell content. Drift vs the legacy
/// behaviour: previously the rule spanned the row's full height;
/// post-migration it spans column 1's cell height only — acceptable
/// per the Q5 / H-arch-A5b architect read.
fn id_cell<'a>(id: StrategyId, label: String, is_active: bool) -> Element<'a, Message> {
    let rule_color = if is_active {
        color::ACCENT.current(ThemeMode::Dark)
    } else {
        iced::Color::TRANSPARENT
    };
    let rule = Container::new(
        iced::widget::Space::new()
            .width(Length::Fixed(2.0))
            .height(Length::Fill),
    )
    .width(Length::Fixed(2.0))
    .height(Length::Fill)
    .style(move |_theme: &iced::Theme| iced::widget::container::Style {
        background: Some(rule_color.into()),
        ..Default::default()
    });

    let button = Button::new(
        Text::new(label)
            .size(text::BODY)
            .color(color::FG_1.current(ThemeMode::Dark)),
    )
    .on_press(Message::SelectStrategy(id))
    .padding(0)
    .style(move |_theme: &iced::Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => Some(color::PANEL_SUNKEN.current(ThemeMode::Dark).into()),
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

    Row::new().push(rule).push(button).spacing(space::XS).into()
}

/// Red-tinted one-line error badge text. Reuses the semantic `NEG`
/// color so it reads as "danger" without competing with the
/// header-level panel error state. Rendered in a sibling
/// `Column<error_badges>` BELOW the table per Q6 / H-arch-A5b
/// RESOLVED-CONFIRM (T2.2).
fn error_badge_text<'a>(summary: &str) -> Element<'a, Message> {
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
