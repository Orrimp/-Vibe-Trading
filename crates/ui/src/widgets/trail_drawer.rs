#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
//! Trail side-drawer widget — raw artifact viewer for a selected trail node.
//!
//! Phase D (ui-rethink-phase-d-trail R4.1-R4.4).
//!
//! Renders the raw payload for the selected trail stage:
//!   - Fill → `journal_transactions.metadata` JSON pretty-print
//!   - Signal → strategy_signals row dump
//!   - Forecast → forecast_events row dump + one-line summary
//!   - LLM debate → "(no transcript recorded)" placeholder (R1.5)
//!
//! Uses `RIGHT_RAIL_OPEN_WIDTH_PX` (= 320 px) — the OPEN right-rail width
//! (the drawer only exists in the open state; `RIGHT_RAIL_WIDTH_PX` is the
//! 0-px CLOSED-state token and would render an invisible drawer).
//! Trigger: chevron-click on a trail node (Q3 = chevron-click).
//! Dismissal: `Message::TrailDrawerClosed`.

use iced::widget::{Button, Column, Container, Row, Scrollable, Text, button};
use iced::{Border, Element, Length};

use crate::state::Message;
use crate::theme::{ThemeMode, color, layout, radius, space, text};
use crate::widgets::trail_node::TrailNodeKind;

// ── Drawer payload types ──────────────────────────────────────────────────────

/// Raw payload for one trail-drawer render. All variants carry human-readable
/// strings derived from the SQL row. Empty body renders the R3.4 placeholder.
#[derive(Debug, Clone)]
pub enum DrawerPayload {
    /// Fill row: `journal_transactions.metadata` JSON (pretty-printed).
    Fill { metadata_json: String },
    /// Signal row dump (`strategy_signals` columns).
    Signal {
        side: String,
        intended_qty: String,
        intended_price: Option<String>,
        was_clamped: bool,
        clamp_reason: Option<String>,
    },
    /// Forecast row dump + single-line summary.
    Forecast {
        direction: String,
        confidence: String,
        model_revision: String,
        cache_hit: bool,
    },
    /// LLM debate transcript — always the v0.1.0 placeholder (R1.5).
    LlmDebate,
}

// Silent-quarantine-fix-2026-05-26: all user-visible copy now lives in
// `crate::strings` per the `consistency::no_inline_user_visible_strings_in_widgets`
// hygiene test contract.
use crate::strings::{
    TRAIL_BOOL_NO, TRAIL_BOOL_YES, TRAIL_DRAWER_CLOSE_LABEL, TRAIL_DRAWER_FILL_TITLE,
    TRAIL_DRAWER_FORECAST_TITLE, TRAIL_DRAWER_LLM_PLACEHOLDER, TRAIL_DRAWER_LLM_TITLE,
    TRAIL_DRAWER_SIGNAL_TITLE, TRAIL_FORECAST_CACHE_HIT_LABEL, TRAIL_FORECAST_CONFIDENCE_LABEL,
    TRAIL_FORECAST_DIRECTION_LABEL, TRAIL_FORECAST_MODEL_LABEL, TRAIL_SIGNAL_CLAMP_REASON_LABEL,
    TRAIL_SIGNAL_CLAMPED_LABEL, TRAIL_SIGNAL_PRICE_LABEL, TRAIL_SIGNAL_PRICE_MARKET,
    TRAIL_SIGNAL_QTY_LABEL, TRAIL_SIGNAL_SIDE_LABEL, trail_forecast_summary,
};

fn title_for_kind(kind: TrailNodeKind) -> &'static str {
    match kind {
        TrailNodeKind::Fill => TRAIL_DRAWER_FILL_TITLE,
        TrailNodeKind::Signal => TRAIL_DRAWER_SIGNAL_TITLE,
        TrailNodeKind::Forecast => TRAIL_DRAWER_FORECAST_TITLE,
        TrailNodeKind::LlmDebate => TRAIL_DRAWER_LLM_TITLE,
    }
}

// ── Public view function ──────────────────────────────────────────────────────

/// Render the trail side-drawer.
///
/// Width is `RIGHT_RAIL_OPEN_WIDTH_PX` (= 320 px). The drawer is only ever
/// built in the OPEN state (`screens::trail::view` constructs it inside
/// `if let Some(node_kind) = drawer_selected`), so it uses the *open* rail
/// width — NOT the `RIGHT_RAIL_WIDTH_PX = 0.0` closed-state token (which would
/// render an invisible 0-px-wide drawer). All four body variants (Fill /
/// Signal / Forecast / LLM-placeholder) are covered per R4.2.
/// Dismissal via `Message::TrailDrawerClosed` (R4.4).
///
/// `payload` is taken BY VALUE so the body's `Text` widgets can own their
/// strings: `screens::trail::view` builds the payload from the selected
/// stage's data (which lives in `model`) and moves it in, avoiding an E0515
/// "borrows local" lifetime error on the returned `Element`.
#[must_use]
pub fn view<'a>(
    kind: TrailNodeKind,
    payload: Option<DrawerPayload>,
    mode: ThemeMode,
) -> Element<'a, Message> {
    let title = title_for_kind(kind);

    let close_btn = Button::new(
        Text::new(TRAIL_DRAWER_CLOSE_LABEL)
            .size(text::SMALL)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::TrailDrawerClosed)
    .padding([space::XS as u16, space::S as u16])
    .style(move |_theme: &iced::Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: color::FG_1.current(mode),
            border: Border {
                radius: radius::R2.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let header = Row::new()
        .spacing(space::M)
        .push(
            Text::new(title)
                .size(text::SMALL)
                .color(color::ACCENT.current(mode)),
        )
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(close_btn);

    let body: Element<'a, Message> = match payload {
        // Raw-JSON viewer (the production path today): the selected stage's
        // serialised `raw_payload` row, scrollable. `metadata_json` is owned
        // here, so move it into the `Text` (owning `Cow`) — no local borrow.
        Some(DrawerPayload::Fill { metadata_json }) => Scrollable::new(
            Text::new(metadata_json)
                .size(text::MICRO)
                .color(color::FG_1.current(mode)),
        )
        .height(Length::Fill)
        .into(),
        Some(DrawerPayload::Signal {
            side,
            intended_qty,
            intended_price,
            was_clamped,
            clamp_reason,
        }) => {
            let mut col = Column::new()
                .spacing(space::XS)
                .push(kv(TRAIL_SIGNAL_SIDE_LABEL, side, mode))
                .push(kv(TRAIL_SIGNAL_QTY_LABEL, intended_qty, mode))
                .push(kv(
                    TRAIL_SIGNAL_PRICE_LABEL,
                    intended_price.unwrap_or_else(|| TRAIL_SIGNAL_PRICE_MARKET.to_string()),
                    mode,
                ))
                .push(kv(
                    TRAIL_SIGNAL_CLAMPED_LABEL,
                    if was_clamped {
                        TRAIL_BOOL_YES
                    } else {
                        TRAIL_BOOL_NO
                    }
                    .to_string(),
                    mode,
                ));
            if let Some(reason) = clamp_reason {
                col = col.push(kv(TRAIL_SIGNAL_CLAMP_REASON_LABEL, reason, mode));
            }
            col.into()
        }
        Some(DrawerPayload::Forecast {
            direction,
            confidence,
            model_revision,
            cache_hit,
        }) => {
            let summary = trail_forecast_summary(&direction, &confidence);
            Column::new()
                .spacing(space::XS)
                .push(
                    Text::new(summary)
                        .size(text::BODY)
                        .color(color::FG_1.current(mode)),
                )
                .push(kv(TRAIL_FORECAST_DIRECTION_LABEL, direction, mode))
                .push(kv(TRAIL_FORECAST_CONFIDENCE_LABEL, confidence, mode))
                .push(kv(TRAIL_FORECAST_MODEL_LABEL, model_revision, mode))
                .push(kv(
                    TRAIL_FORECAST_CACHE_HIT_LABEL,
                    if cache_hit {
                        TRAIL_BOOL_YES
                    } else {
                        TRAIL_BOOL_NO
                    }
                    .to_string(),
                    mode,
                ))
                .into()
        }
        None | Some(DrawerPayload::LlmDebate) => Text::new(TRAIL_DRAWER_LLM_PLACEHOLDER)
            .size(text::BODY)
            .color(color::FG_4.current(mode))
            .into(),
    };

    let content = Column::new()
        .spacing(space::M)
        .padding(space::M as u16)
        .push(header)
        .push(body)
        .width(Length::Fixed(layout::RIGHT_RAIL_OPEN_WIDTH_PX));

    Container::new(content)
        .height(Length::Fill)
        .width(Length::Fixed(layout::RIGHT_RAIL_OPEN_WIDTH_PX))
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: color::PANEL_SUNKEN.current(mode),
                width: 1.0,
                radius: radius::R3.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render a key-value row for the drawer body. `value` is taken by owned
/// `String` (moved into the value `Text`) so the row borrows nothing local —
/// the caller builds these from a by-value `DrawerPayload`. `key` is a
/// `'static` string constant from `crate::strings`.
fn kv<'a>(key: &'static str, value: String, mode: ThemeMode) -> Element<'a, Message> {
    Row::new()
        .spacing(space::S)
        .push(
            Text::new(key)
                .size(text::MICRO)
                .color(color::FG_4.current(mode)),
        )
        .push(
            Text::new(value)
                .size(text::SMALL)
                .color(color::FG_1.current(mode)),
        )
        .into()
}
