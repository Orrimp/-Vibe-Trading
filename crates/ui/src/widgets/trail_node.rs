#![allow(clippy::cast_possible_truncation)]
//! Trail node widget — one node per pipeline stage in the trail view.
//!
//! Phase D (ui-rethink-phase-d-trail R3.1-R3.5).
//!
//! Visual layout: vertical stack, top→bottom upstream→downstream
//! (Forecast at top, LLM next, Signal mid, Fill at bottom) per Q2
//! analyst recommendation.
//!
//! Each node renders:
//!   - timestamp (`HH:MM:SS.μμμ`)
//!   - actor label ("strategy:<id>" / "tcn:<rev short>" / "llm:<tier>")
//!   - headline (one-line summary)
//!   - chevron button → `Message::TrailNodeChevronClicked(node_kind)`
//!
//! Selected node gets Lumen `ACCENT` border ring. Empty-stage rendering
//! via `frame::muted_body` when stage row is absent (R3.4).

use iced::widget::{Button, Column, Row, Text, button};
use iced::{Border, Element, Length};

use crate::state::Message;
use crate::theme::{ThemeMode, color, radius, space, text};

// ── Public types ──────────────────────────────────────────────────────────────

/// The four pipeline stages for trail node rendering (R3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrailNodeKind {
    /// The final trade execution fill.
    Fill,
    /// The strategy-emitted signal preceding the fill.
    Signal,
    /// The TCN forecast that influenced the signal.
    Forecast,
    /// LLM debate transcript (placeholder at v0.1.0 — R1.5).
    LlmDebate,
}

/// Data for one trail node stage. All fields are `Option` — absent when the
/// stage has no audit row (R3.4 empty-stage rendering).
#[derive(Debug, Clone)]
pub struct TrailNode {
    pub kind: TrailNodeKind,
    /// Timestamp string already formatted as `HH:MM:SS.μμμ`.
    pub timestamp: Option<String>,
    /// Actor label: "strategy:\<id\>", "tcn:\<`rev_short`\>", etc.
    pub actor: Option<String>,
    /// One-line headline summarising this stage's payload.
    pub headline: Option<String>,
}

// ── Constants ─────────────────────────────────────────────────────────────────

const CHEVRON_LABEL: &str = "›";
const NO_UPSTREAM_FILL: &str = "(no upstream fill recorded)";
const NO_UPSTREAM_SIGNAL: &str = "(no upstream signal recorded)";
const NO_UPSTREAM_FORECAST: &str = "(no upstream forecast recorded)";
const NO_LLM_TRANSCRIPT: &str = "(no transcript recorded)";

fn empty_label(kind: TrailNodeKind) -> &'static str {
    match kind {
        TrailNodeKind::Fill => NO_UPSTREAM_FILL,
        TrailNodeKind::Signal => NO_UPSTREAM_SIGNAL,
        TrailNodeKind::Forecast => NO_UPSTREAM_FORECAST,
        TrailNodeKind::LlmDebate => NO_LLM_TRANSCRIPT,
    }
}

fn kind_label(kind: TrailNodeKind) -> &'static str {
    match kind {
        TrailNodeKind::Fill => "Fill",
        TrailNodeKind::Signal => "Signal",
        TrailNodeKind::Forecast => "Forecast",
        TrailNodeKind::LlmDebate => "LLM Debate",
    }
}

// ── Public view function ──────────────────────────────────────────────────────

/// Render one trail node.
///
/// Pure render — no state, no side effects. The `selected` flag renders the
/// Lumen `ACCENT` border ring (R3.5). Empty stage renders the muted placeholder
/// per R3.4 using `frame::muted_body` style.
#[must_use]
pub fn view<'a>(node: &'a TrailNode, selected: bool, mode: ThemeMode) -> Element<'a, Message> {
    let header_row = Row::new()
        .spacing(space::S)
        .push(
            Text::new(kind_label(node.kind))
                .size(text::SMALL)
                .color(color::ACCENT.current(mode)),
        )
        .push(
            Text::new(node.timestamp.as_deref().unwrap_or("--:--:--.---"))
                .size(text::MICRO)
                .color(color::FG_4.current(mode)),
        );

    let body: Element<'a, Message> = if node.headline.is_none() && node.actor.is_none() {
        // Empty stage — R3.4 muted placeholder.
        Text::new(empty_label(node.kind))
            .size(text::BODY)
            .color(color::FG_4.current(mode))
            .into()
    } else {
        let actor_text = node.actor.as_deref().unwrap_or("");
        let headline_text = node.headline.as_deref().unwrap_or("");

        Column::new()
            .spacing(space::XS)
            .push(
                Text::new(actor_text)
                    .size(text::MICRO)
                    .color(color::FG_4.current(mode)),
            )
            .push(
                Text::new(headline_text)
                    .size(text::BODY)
                    .color(color::FG_1.current(mode)),
            )
            .into()
    };

    let chevron_kind = node.kind;
    let chevron = Button::new(
        Text::new(CHEVRON_LABEL)
            .size(text::BODY)
            .color(color::ACCENT.current(mode)),
    )
    .on_press(Message::TrailNodeChevronClicked(chevron_kind))
    .padding([space::XS as u16, space::S as u16])
    .style(move |_theme: &iced::Theme, status: button::Status| {
        let bg = match status {
            button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: color::ACCENT.current(mode),
            border: Border {
                radius: radius::R2.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let content_row = Row::new()
        .spacing(space::M)
        .push(Column::new().spacing(space::XS).push(header_row).push(body))
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(chevron);

    let border_color = if selected {
        color::ACCENT.current(mode)
    } else {
        color::PANEL_SUNKEN.current(mode)
    };
    let border_width: f32 = if selected { 1.5 } else { 1.0 };

    iced::widget::Container::new(content_row)
        .padding([space::S as u16, space::M as u16])
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: border_color,
                width: border_width,
                radius: radius::R3.into(),
            },
            ..Default::default()
        })
        .into()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Helper: build a trail node with all fields populated.
    fn populated_node(kind: TrailNodeKind) -> TrailNode {
        TrailNode {
            kind,
            timestamp: Some("12:34:56.789".to_string()),
            actor: Some(match kind {
                TrailNodeKind::Fill => "strategy:sma_crossover".to_string(),
                TrailNodeKind::Signal => "strategy:sma_crossover".to_string(),
                TrailNodeKind::Forecast => "tcn:d1c3696d".to_string(),
                TrailNodeKind::LlmDebate => "llm:tier-1".to_string(),
            }),
            headline: Some(match kind {
                TrailNodeKind::Fill => "Buy 0.05 BTCUSDT @ 45000".to_string(),
                TrailNodeKind::Signal => "Buy signal emitted".to_string(),
                TrailNodeKind::Forecast => "Up 0.72 confidence".to_string(),
                TrailNodeKind::LlmDebate => "(no transcript recorded)".to_string(),
            }),
        }
    }

    /// Helper: build an empty (no-data) trail node.
    fn empty_node(kind: TrailNodeKind) -> TrailNode {
        TrailNode {
            kind,
            timestamp: None,
            actor: None,
            headline: None,
        }
    }

    /// T-D-N9: each node kind renders without panicking (dark + unselected).
    #[test]
    fn each_kind_renders_dark_unselected() {
        for kind in [
            TrailNodeKind::Fill,
            TrailNodeKind::Signal,
            TrailNodeKind::Forecast,
            TrailNodeKind::LlmDebate,
        ] {
            let node = populated_node(kind);
            let _el: Element<'_, Message> = view(&node, false, ThemeMode::Dark);
        }
    }

    /// T-D-N9: each node kind renders without panicking (light + selected).
    #[test]
    fn each_kind_renders_light_selected() {
        for kind in [
            TrailNodeKind::Fill,
            TrailNodeKind::Signal,
            TrailNodeKind::Forecast,
            TrailNodeKind::LlmDebate,
        ] {
            let node = populated_node(kind);
            let _el: Element<'_, Message> = view(&node, true, ThemeMode::Light);
        }
    }

    /// T-D-N9: empty-stage nodes render the muted placeholder in both themes.
    #[test]
    fn empty_stage_renders_both_themes() {
        for kind in [
            TrailNodeKind::Fill,
            TrailNodeKind::Signal,
            TrailNodeKind::Forecast,
            TrailNodeKind::LlmDebate,
        ] {
            let node = empty_node(kind);
            let _dark: Element<'_, Message> = view(&node, false, ThemeMode::Dark);
            let _light: Element<'_, Message> = view(&node, false, ThemeMode::Light);
        }
    }

    /// T-D-N9: `TrailNodeKind` equality used for chevron dispatch.
    #[test]
    fn trail_node_kind_eq() {
        assert_eq!(TrailNodeKind::Fill, TrailNodeKind::Fill);
        assert_ne!(TrailNodeKind::Fill, TrailNodeKind::Signal);
        assert_ne!(TrailNodeKind::Forecast, TrailNodeKind::LlmDebate);
    }
}
