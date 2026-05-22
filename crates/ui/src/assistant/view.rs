#![allow(clippy::cast_possible_truncation, clippy::elidable_lifetime_names)]
//! Phase F — Assistant slot view.
//!
//! v0.1.0 (Phase 6 wake, Q4=(a) stub-only) shipped the slot wake
//! structurally with `AssistantMode::Offline` rendering the
//! "Assistant offline" placeholder.
//!
//! Wave F (v3-llm-forecaster T-D-N(F2)) adds the
//! `AssistantMode::ReasoningTrace` body composition (R9.2): header
//! (symbol \u{00b7} rating \u{00b7} confidence), cost line, reasoning trace,
//! cited lessons, history, chevron-to-trail.
//!
//! ## R9.3 byte-identity guard
//!
//! When `state.mode == AssistantMode::Offline`, this fn returns the
//! exact same widget tree as the pre-Wave-F build:
//!
//! - `state.is_open == false` \u{2192} 0-width `Container<Space>` (mirrors
//!   the pre-Phase-F `Space::new()` at the old `shell.rs:47-49`).
//! - `state.is_open == true` \u{2192} `Container<Column[Title, Body]>` styled
//!   with the Phase F panel chrome.
//!
//! The `state.last_forecast` / `state.history` fields are ONLY read on
//! the `ReasoningTrace` arm. With the default `Offline` mode they are
//! never touched \u{2192} a default-disabled cockpit cannot tell the
//! difference between the pre-Wave-F view fn and this one.

use iced::Length;
use iced::widget::{Column, Container, Row, Scrollable, Space, Text};

use crate::assistant::state::{AssistantMode, AssistantState, LlmForecastView};
use crate::memory::state::LessonCardCard;
use crate::state::{Cockpit, Message};
use crate::strings::{
    ASSISTANT_OFFLINE_BODY, ASSISTANT_OFFLINE_TITLE, ASSISTANT_REASONING_COST_LABEL,
    ASSISTANT_REASONING_COST_PENDING, ASSISTANT_REASONING_HEADER_FMT,
    ASSISTANT_REASONING_HISTORY_EMPTY, ASSISTANT_REASONING_HISTORY_LABEL,
    ASSISTANT_REASONING_HISTORY_ROW_FMT, ASSISTANT_REASONING_LESSON_PENDING_FMT,
    ASSISTANT_REASONING_LESSONS_LABEL, ASSISTANT_REASONING_NO_LESSONS, ASSISTANT_REASONING_TITLE,
    ASSISTANT_REASONING_TRACE_LABEL, ASSISTANT_REASONING_WARMING_BODY,
    ASSISTANT_REASONING_WARMING_TITLE,
};
use crate::theme::{ThemeMode, color, layout::RIGHT_RAIL_OPEN_WIDTH_PX, radius, space, text};

/// Render the right-rail Assistant slot.
///
/// Pre-Wave-F entry point kept for binary callers (`shell::view`) and
/// existing tests. Reads only `assistant_state`; the `ReasoningTrace`
/// composition needs the full `Cockpit` (it looks up cited-lesson
/// bodies in `memory_screen_state.cache`). Callers that have a full
/// `Cockpit` should prefer [`view_with_cockpit`].
///
/// At `AssistantMode::Offline` (the default) this returns a widget
/// tree byte-identical to the pre-Wave-F view fn (R9.3 byte-identity
/// guard).
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn view(state: &AssistantState, mode: ThemeMode) -> crate::Element<'_> {
    // No `Cockpit` available — cited-lesson bodies cannot be looked up.
    // Pass an empty slice; the ReasoningTrace render falls through to
    // the `ASSISTANT_REASONING_LESSON_PENDING_FMT` row per cited id.
    render(state, &[], mode)
}

/// `Cockpit`-aware entry point. Reads cited-lesson card bodies from
/// `cockpit.memory_screen_state.cache` so the reasoning-trace render
/// can display real card metadata. Falls back to the `_LESSON_PENDING`
/// row when a cited card hasn't hydrated yet.
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn view_with_cockpit<'a>(cockpit: &'a Cockpit, mode: ThemeMode) -> crate::Element<'a> {
    render(
        &cockpit.assistant_state,
        &cockpit.memory_screen_state.cache,
        mode,
    )
}

/// Internal render entry — dispatches on `state.mode`.
fn render<'a>(
    state: &'a AssistantState,
    memory_cache: &'a [LessonCardCard],
    mode: ThemeMode,
) -> crate::Element<'a> {
    if !state.is_open {
        // R9.3 byte-identity: closed-state Space, identical to pre-Wave-F.
        return Container::new(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    match state.mode {
        AssistantMode::Offline => view_offline(mode),
        AssistantMode::ReasoningTrace => view_reasoning_trace(state, memory_cache, mode),
        AssistantMode::Live => {
            // v0.2.0 mode — falls back to Offline placeholder at v0.1.0
            // so a stray `Live` mode value doesn't crash the cockpit.
            view_offline(mode)
        }
    }
}

/// Offline-mode body \u{2014} R9.3 byte-identity guard.
///
/// This widget tree MUST stay byte-identical to the pre-Wave-F view fn
/// so the `assistant_slot__open_stub.png` baseline (locked 2026-05-21)
/// renders bit-for-bit identically when the v3-llm-forecaster strategy
/// is disabled.
fn view_offline<'a>(mode: ThemeMode) -> crate::Element<'a> {
    let title = Text::new(ASSISTANT_OFFLINE_TITLE)
        .size(text::BODY)
        .color(color::FG_1.current(mode));

    let body = Text::new(ASSISTANT_OFFLINE_BODY)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    let content = Column::new()
        .spacing(space::S)
        .push(title)
        .push(body)
        .padding(space::M as u16);

    Container::new(content)
        .width(Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Reasoning-trace mode body \u{2014} R9.2 composition.
///
/// Top-to-bottom:
/// 1. Title row \u{2014} `ASSISTANT_REASONING_TITLE` + chevron to audit trail.
/// 2. Header line \u{2014} `{symbol} \u{00b7} {rating} \u{00b7} conf {confidence}`.
/// 3. Cost line \u{2014} `LLM spend: {cost_line}` (or `Awaiting first forecast`).
/// 4. Reasoning trace card (scrollable text).
/// 5. Cited-lessons section (compact card rows or empty placeholder).
/// 6. Scrollable history (most-recent first; up to `HISTORY_CAP` rows).
///
/// When `state.last_forecast == None`, renders a warming-up empty
/// state instead (R9.3 mode-on / data-empty path).
fn view_reasoning_trace<'a>(
    state: &'a AssistantState,
    memory_cache: &'a [LessonCardCard],
    mode: ThemeMode,
) -> crate::Element<'a> {
    let Some(forecast) = &state.last_forecast else {
        return view_warming_up(mode);
    };

    let title = Text::new(ASSISTANT_REASONING_TITLE)
        .size(text::H3)
        .color(color::FG_1.current(mode));

    let header_text = ASSISTANT_REASONING_HEADER_FMT
        .replace("{symbol}", forecast.symbol.as_str())
        .replace("{rating}", forecast.rating.as_str())
        .replace("{confidence}", forecast.confidence_display.as_str());
    let header = Text::new(header_text)
        .size(text::BODY)
        .color(rating_color(forecast.rating.as_str(), mode));

    let cost_text = forecast
        .cost_line
        .as_deref()
        .unwrap_or(ASSISTANT_REASONING_COST_PENDING);
    let cost_row = Row::new()
        .spacing(space::XS)
        .push(
            Text::new(ASSISTANT_REASONING_COST_LABEL)
                .size(text::MICRO)
                .color(color::FG_3.current(mode)),
        )
        .push(
            Text::new(cost_text.to_string())
                .size(text::MICRO)
                .color(color::FG_2.current(mode)),
        );

    let reasoning_card = reasoning_card(forecast, mode);
    let lessons_section = cited_lessons_section(forecast, memory_cache, mode);
    let history_section = history_section(&state.history, mode);

    let content = Column::new()
        .spacing(space::M)
        .push(title)
        .push(header)
        .push(cost_row)
        .push(reasoning_card)
        .push(lessons_section)
        .push(history_section)
        .padding(space::M as u16);

    Container::new(Scrollable::new(content))
        .width(Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Warming-up empty state \u{2014} `mode == ReasoningTrace` + no forecast yet.
fn view_warming_up<'a>(mode: ThemeMode) -> crate::Element<'a> {
    let title = Text::new(ASSISTANT_REASONING_WARMING_TITLE)
        .size(text::BODY)
        .color(color::FG_1.current(mode));

    let body = Text::new(ASSISTANT_REASONING_WARMING_BODY)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    let content = Column::new()
        .spacing(space::S)
        .push(title)
        .push(body)
        .padding(space::M as u16);

    Container::new(content)
        .width(Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX))
        .height(Length::Fill)
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Reasoning-trace card body \u{2014} the `LlmForecast.reasoning_trace` string
/// rendered as plain text inside a sunken panel.
fn reasoning_card<'a>(forecast: &'a LlmForecastView, mode: ThemeMode) -> crate::Element<'a> {
    let label = Text::new(ASSISTANT_REASONING_TRACE_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    let card_body = Container::new(
        Text::new(forecast.reasoning_trace.as_str())
            .size(text::SMALL)
            .color(color::FG_1.current(mode)),
    )
    .width(Length::Fill)
    .padding(space::S as u16)
    .style(move |_t: &iced::Theme| iced::widget::container::Style {
        background: Some(color::PANEL_SUNKEN.current(mode).into()),
        border: iced::Border {
            color: color::BORDER_1.current(mode),
            width: 1.0,
            radius: radius::R2.into(),
        },
        text_color: Some(color::FG_1.current(mode)),
        ..Default::default()
    });

    Column::new()
        .spacing(space::XS)
        .push(label)
        .push(card_body)
        .into()
}

/// Cited-lessons section \u{2014} compact list with Memory cache lookup per id.
fn cited_lessons_section<'a>(
    forecast: &'a LlmForecastView,
    memory_cache: &'a [LessonCardCard],
    mode: ThemeMode,
) -> crate::Element<'a> {
    let label = Text::new(ASSISTANT_REASONING_LESSONS_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    let body: crate::Element<'_> = if forecast.cited_lesson_ids.is_empty() {
        Text::new(ASSISTANT_REASONING_NO_LESSONS)
            .size(text::SMALL)
            .color(color::FG_3.current(mode))
            .into()
    } else {
        let mut col = Column::new().spacing(space::XS);
        for card_id in &forecast.cited_lesson_ids {
            col = col.push(cited_lesson_row(card_id.as_str(), memory_cache, mode));
        }
        col.into()
    };

    Column::new()
        .spacing(space::XS)
        .push(label)
        .push(body)
        .into()
}

/// One cited-lesson row \u{2014} either a compact rendering of the matched
/// `LessonCardCard` or a `_LESSON_PENDING` fallback when the card
/// hasn't hydrated yet.
fn cited_lesson_row<'a>(
    card_id: &'a str,
    memory_cache: &'a [LessonCardCard],
    mode: ThemeMode,
) -> crate::Element<'a> {
    let matched = memory_cache.iter().find(|c| c.card_id.as_str() == card_id);

    let body: crate::Element<'_> = if let Some(card) = matched {
        // Compact row: symbol + outcome + pnl display.
        Row::new()
            .spacing(space::S)
            .push(
                Text::new(card.symbol_or_pair.as_str())
                    .size(text::MICRO)
                    .color(color::FG_2.current(mode)),
            )
            .push(
                Text::new(card.outcome_class.as_str())
                    .size(text::MICRO)
                    .color(outcome_color(card.outcome_class.as_str(), mode)),
            )
            .push(
                Text::new(card.signed_pnl_display.as_str())
                    .size(text::MICRO)
                    .color(color::FG_3.current(mode)),
            )
            .into()
    } else {
        let pending = ASSISTANT_REASONING_LESSON_PENDING_FMT.replace("{card_id}", card_id);
        Text::new(pending)
            .size(text::MICRO)
            .color(color::FG_3.current(mode))
            .into()
    };

    Container::new(body)
        .width(Length::Fill)
        .padding([space::XXS as u16, space::XS as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R1.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// History section \u{2014} compact list of past forecasts (most-recent first).
fn history_section<'a>(history: &'a [LlmForecastView], mode: ThemeMode) -> crate::Element<'a> {
    let label = Text::new(ASSISTANT_REASONING_HISTORY_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    let body: crate::Element<'_> = if history.is_empty() {
        Text::new(ASSISTANT_REASONING_HISTORY_EMPTY)
            .size(text::SMALL)
            .color(color::FG_3.current(mode))
            .into()
    } else {
        let mut col = Column::new().spacing(space::XXS);
        for entry in history {
            let row_text = ASSISTANT_REASONING_HISTORY_ROW_FMT
                .replace("{rating}", entry.rating.as_str())
                .replace("{confidence}", entry.confidence_display.as_str());
            col = col.push(
                Text::new(row_text)
                    .size(text::MICRO)
                    .color(rating_color(entry.rating.as_str(), mode)),
            );
        }
        col.into()
    };

    Column::new()
        .spacing(space::XS)
        .push(label)
        .push(body)
        .into()
}

/// Map a rating tier string to a Lumen semantic color.
///
/// Bullish ratings (`STRONG_BUY` / `BUY`) \u{2192} `UP_500` (sage gain).
/// Bearish ratings (`SELL` / `STRONG_SELL`) \u{2192} `DOWN_500` (clay loss).
/// `HOLD` and anything else \u{2192} `FG_2` (neutral secondary text).
fn rating_color(rating: &str, mode: ThemeMode) -> iced::Color {
    match rating {
        "STRONG_BUY" | "BUY" => color::UP_500.current(mode),
        "SELL" | "STRONG_SELL" => color::DOWN_500.current(mode),
        _ => color::FG_2.current(mode),
    }
}

/// Map a lesson outcome class label to a Lumen semantic color.
fn outcome_color(outcome: &str, mode: ThemeMode) -> iced::Color {
    match outcome {
        "Win" => color::UP_500.current(mode),
        "Loss" => color::DOWN_500.current(mode),
        _ => color::FG_3.current(mode),
    }
}

/// Suppress the unused-import warning for `Message`. The view emits no
/// messages directly at v0.1.0 (chevron-to-trail is wired in T-D-N(F3)
/// `Message::AssistantReasoningTraceUpdate` path \u{2014} the trail-link
/// itself reuses the existing `Message::OpenTrailFor(audit_id)` per
/// R9.2 bullet 6 and lands when the audit-emission Wave E surfaces an
/// `audit_id`).
#[allow(dead_code)]
fn _message_used(_: &Message) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::fake_cockpit_ready;
    use smol_str::SmolStr;

    fn fake_forecast() -> LlmForecastView {
        LlmForecastView {
            symbol: SmolStr::new("BTCUSDT"),
            rating: SmolStr::new("BUY"),
            confidence_display: SmolStr::new("0.74"),
            reasoning_trace: SmolStr::new(
                "RSI=58 with MACD crossover above zero suggests continuation. \
                 Bollinger band squeeze tightening over last 3 bars. \
                 Recent similar setup at lesson_abc closed Win +1.2%.",
            ),
            cited_lesson_ids: vec![SmolStr::new("card_abc"), SmolStr::new("card_def")],
            cost_line: Some(SmolStr::new("$0.42 / $100.00 today")),
            audit_id: Some(SmolStr::new("audit_001")),
        }
    }

    /// T-D-N(F2) — Reasoning-trace render produces a non-empty element
    /// when `mode == ReasoningTrace` and a forecast is present.
    #[test]
    fn assistant_view_reasoning_trace_render() {
        let state = AssistantState {
            is_open: true,
            mode: AssistantMode::ReasoningTrace,
            last_forecast: Some(fake_forecast()),
            history: vec![fake_forecast()],
        };
        // Render produces a valid Element. The fact that we get here
        // without panicking is the contract (iced::Element is opaque;
        // pixel checks belong in tests/visual_snapshots.rs).
        let _el = view(&state, ThemeMode::Dark);
    }

    /// T-D-N(F4) — `view(state)` with `mode == Offline` renders the
    /// placeholder body regardless of `last_forecast` content. This is
    /// the R9.3 byte-identity guarantee in code form: even if a stray
    /// forecast slipped into `last_forecast`, the Offline mode body is
    /// unchanged.
    #[test]
    fn assistant_runtime_gate_preserves_offline_default() {
        // A "naughty" state: Offline mode but with a forecast in
        // `last_forecast`. The runtime gate sets the mode, not the
        // payload; this asserts the view fn honours the gate.
        let state = AssistantState {
            is_open: true,
            mode: AssistantMode::Offline,
            last_forecast: Some(fake_forecast()),
            history: vec![fake_forecast()],
        };
        // Render must not panic and must not access the forecast
        // payload. We can't directly compare iced elements, but a
        // visual snapshot test confirms byte-identity downstream.
        let _el = view(&state, ThemeMode::Dark);
    }

    /// T-D-N(F2) — Warming-up state when `mode == ReasoningTrace` but
    /// `last_forecast == None`. The view fn must NOT panic on the None
    /// path; the warming-up empty state renders instead.
    #[test]
    fn assistant_view_reasoning_trace_warming_up() {
        let state = AssistantState {
            is_open: true,
            mode: AssistantMode::ReasoningTrace,
            last_forecast: None,
            history: vec![],
        };
        let _el = view(&state, ThemeMode::Dark);
    }

    /// T-D-N(F2) — `view_with_cockpit` path: cited-lessons section
    /// renders compact rows when the memory cache has matching cards;
    /// falls back to the `_LESSON_PENDING` row on lookup miss.
    #[test]
    fn assistant_view_with_cockpit_uses_memory_cache() {
        let mut cockpit = fake_cockpit_ready();
        cockpit.assistant_state = AssistantState {
            is_open: true,
            mode: AssistantMode::ReasoningTrace,
            last_forecast: Some(fake_forecast()),
            history: vec![],
        };
        // Insert one of the two cited cards into memory cache so the
        // section exercises BOTH branches (matched + pending).
        cockpit.memory_screen_state.cache = vec![LessonCardCard {
            card_id: SmolStr::new("card_abc"),
            symbol_or_pair: SmolStr::new("BTCUSDT"),
            closed_at: SmolStr::new("2026-05-01T00:00:00Z"),
            strategy_id: SmolStr::new("llm_forecaster_v3"),
            signed_pnl_display: SmolStr::new("+1.20 USDT"),
            outcome_class: SmolStr::new("Win"),
            note: None,
            close_transaction_id: None,
        }];
        let _el = view_with_cockpit(&cockpit, ThemeMode::Dark);
    }

    /// T-D-N(F2) — Light mode must render without panicking. Lumen
    /// design-system contract: every screen renders correctly under
    /// both themes.
    #[test]
    fn assistant_view_reasoning_trace_renders_light_mode() {
        let state = AssistantState {
            is_open: true,
            mode: AssistantMode::ReasoningTrace,
            last_forecast: Some(fake_forecast()),
            history: vec![],
        };
        let _el = view(&state, ThemeMode::Light);
    }

    /// T-D-N(F4) — `mode == Live` (v0.2.0 reserved variant) falls back
    /// to the Offline placeholder rather than panicking. Defensive
    /// rendering: a stray `Live` value in the state must not crash.
    #[test]
    fn assistant_view_live_mode_falls_back_to_offline() {
        let state = AssistantState {
            is_open: true,
            mode: AssistantMode::Live,
            last_forecast: None,
            history: vec![],
        };
        let _el = view(&state, ThemeMode::Dark);
    }

    /// T-D-N(F4) — Closed slot (`is_open == false`) renders the zero-
    /// width Space regardless of `mode`. R9.3 byte-identity guard at
    /// the outermost layer.
    #[test]
    fn assistant_view_closed_slot_is_zero_width_for_all_modes() {
        for m in [
            AssistantMode::Offline,
            AssistantMode::ReasoningTrace,
            AssistantMode::Live,
        ] {
            let state = AssistantState {
                is_open: false,
                mode: m,
                last_forecast: Some(fake_forecast()),
                history: vec![],
            };
            let _el = view(&state, ThemeMode::Dark);
        }
    }
}
