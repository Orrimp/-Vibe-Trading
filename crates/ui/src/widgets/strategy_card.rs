//! Strategy registry card widget — Phase C (ui-rethink-phase-c-sidebar-ia).
//!
//! Tier-2 surface card composing existing Lumen primitives. One card per
//! registered strategy in the `Strategy registry` screen (R6.1 / Design § A11).
//!
//! Composition inside `frame::panel` (top → bottom):
//!
//! 1. Header row: strategy ID + status pill (`STRATEGY_REGISTRY_STATUS_SHIPPED`
//!    for all rows at Phase C per A6).
//! 2. Universe line: `STRATEGY_REGISTRY_UNIVERSE_PREFIX` + truncated symbols.
//! 3. Anchor line: `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX` + scenario + sha7,
//!    or `PLACEHOLDER_NONE` when `last_anchor` is `None` (all rows at Phase C).
//! 4. Last-run line: `STRATEGY_REGISTRY_LAST_RUN_PREFIX` + RFC-3339 timestamp,
//!    or `PLACEHOLDER_NONE` when `last_run_ts` is `None`.
//! 5. Footer button: `STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL` →
//!    `Message::OpenStrategyInLab(row.id.clone())` — a compound dispatch
//!    handled in `ui::state::update` (the `OpenLabFromCompare` precedent):
//!    switch to `Screen::Lab` + preselect the strategy in `lab_state`
//!    (seeding a default pair when none is set so the Lab opens runnable).
//!
//! **No new Lumen tokens.** Status pill reuses `frame::active_chip` (T1609
//! bottom-edge accent pattern). Card chrome reuses `frame::panel`.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

#![allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

use iced::widget::{Button, Column, Container, Row, Space, Text, button, container};
use iced::{Border, Length};
use trading_core::Timestamp;

use crate::state::{Message, StrategyConfigEntry, StrategyRow};
use crate::strings::{
    PLACEHOLDER_NONE, STATUS_BAR_UTC_SUFFIX, STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX,
    STRATEGY_REGISTRY_LAST_RUN_PREFIX, STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL,
    STRATEGY_REGISTRY_STATUS_SHIPPED, STRATEGY_REGISTRY_UNIVERSE_PREFIX,
};
use crate::theme::{ThemeMode, color, layout, radius, shadow, space, text};
use crate::widgets::frame;

/// Render a single strategy registry card.
///
/// Parameters follow Design § A11:
/// - `row` — the strategy-row carrying ID, status pill (unused at Phase C —
///   always "shipped"), event ring.
/// - `config` — optional lookup for universe / params. Pass `None` when
///   `Cockpit::strategies_config` has no entry for this strategy.
/// - `last_anchor` — `(scenario_name, sha256_prefix)` from `spec/anchors.toml`
///   lookup, or `None` when no anchor is recorded (all rows at Phase C).
/// - `last_run_ts` — newest `Run` event timestamp from
///   `Cockpit::strategies_recent_events`, or `None`.
/// - `mode` — theme mode.
#[must_use]
pub fn view<'a>(
    row: &'a StrategyRow,
    config: Option<&'a StrategyConfigEntry>,
    last_anchor: Option<(&'a str, &'a str)>,
    last_run_ts: Option<Timestamp>,
    mode: ThemeMode,
) -> crate::Element<'a> {
    // ── Header row: ID + status pill ────────────────────────────────────────
    let id_text = Text::new(row.id.to_string())
        .size(text::H3)
        .color(color::FG_1.current(mode));

    let pill_text = Text::new(STRATEGY_REGISTRY_STATUS_SHIPPED)
        .size(text::SMALL)
        .color(color::ACCENT.current(mode));
    // Active chip = always "active" so the accent rule renders for the pill.
    let pill = frame::active_chip(pill_text.into(), true, mode);

    let header_row = Row::new().spacing(space::M).push(id_text).push(pill);

    // ── Universe line ────────────────────────────────────────────────────────
    let universe_str: String = config.map_or_else(
        || PLACEHOLDER_NONE.to_string(),
        |cfg| {
            // Display params keys as a proxy for universe — strategy config
            // entries carry params but not a symbols list directly. At Phase C
            // we show the source_path as a universe proxy.
            cfg.source_path.to_string()
        },
    );
    let universe_text = Text::new(format!("{STRATEGY_REGISTRY_UNIVERSE_PREFIX}{universe_str}"))
        .size(text::SMALL)
        .color(color::FG_2.current(mode));

    // ── Anchor line ──────────────────────────────────────────────────────────
    let anchor_str = last_anchor.map_or_else(
        || PLACEHOLDER_NONE.to_string(),
        |(scenario, sha7)| format!("{scenario} @ {sha7}"),
    );
    let anchor_text = Text::new(format!(
        "{STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX}{anchor_str}"
    ))
    .size(text::SMALL)
    .color(color::FG_2.current(mode));

    // ── Last-run line ────────────────────────────────────────────────────────
    let run_str = last_run_ts.map_or_else(
        || PLACEHOLDER_NONE.to_string(),
        |ts| {
            let dt = ts.inner();
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {STATUS_BAR_UTC_SUFFIX}",
                dt.year(),
                dt.month() as u8,
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second(),
            )
        },
    );
    let run_text = Text::new(format!("{STRATEGY_REGISTRY_LAST_RUN_PREFIX}{run_str}"))
        .size(text::SMALL)
        .color(color::FG_2.current(mode));

    // ── "Open in Lab" button ─────────────────────────────────────────────────
    // Emits `Message::OpenStrategyInLab(id)` — a compound dispatch handled
    // entirely in `ui::state::update`: switch to `Screen::Lab` + preselect the
    // strategy in `lab_state` (seeding a default pair when unset so the Lab
    // opens runnable). Previously fired `SelectStrategy`, whose cross-link guard
    // (`current_screen != Screen::Strategies`) is false on the registry screen
    // — so the button was a no-op.
    let open_btn = Button::new(
        Text::new(STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL)
            .size(text::SMALL)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::OpenStrategyInLab(row.id.clone()))
    .padding([space::XS as u16, space::M as u16])
    .style(
        move |_theme: &iced::Theme, _status: button::Status| button::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            text_color: color::FG_1.current(mode),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R2.into(),
            },
            ..Default::default()
        },
    );

    // ── Compose card body ────────────────────────────────────────────────────
    let card_body = Column::new()
        .spacing(space::XS)
        .push(header_row)
        .push(universe_text)
        .push(anchor_text)
        .push(run_text)
        .push(open_btn);

    // Build the card chrome inline (equivalent to frame::panel but without
    // a dynamic `&'a str` title so no lifetime issues arise). The strategy ID
    // is already in the header_row above, so the card has no separate title bar.
    let body_container = Container::new(card_body)
        .width(Length::Fill)
        .padding(layout::PANEL_PADDING as u16);

    let separator = Container::new(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::BORDER_1.current(mode).into()),
            ..Default::default()
        });

    let stack = Column::new()
        .push(separator)
        .push(body_container)
        .spacing(0);

    Container::new(stack)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            shadow: shadow::shadow_1(mode),
            ..Default::default()
        })
        .into()
}
