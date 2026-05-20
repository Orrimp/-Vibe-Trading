#![allow(clippy::cast_possible_truncation)]
//! Trail screen — Phase D (ui-rethink-phase-d-trail R2.1-R2.5).
//!
//! Two modes:
//!
//! **List mode** (cold-start / `trail_screen_state.selected_audit_id == None`):
//! Byte-identical delegation to `screens::audit::view` (R2.2 / R10.1 gate).
//!
//! **Trail mode** (`selected_audit_id == Some(id)`):
//! Vertical stack of `widgets::trail_node` widgets + side-drawer for the
//! selected node + breadcrumb "back to list".
//!
//! The `Screen::Audit` deprecated alias routes here (R2.4).

use iced::widget::{Button, Column, Row, Text, button};
use iced::{Border, Element, Length};

use crate::state::{Cockpit, Message};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::trail_node::{self, TrailNode, TrailNodeKind};

/// Render the Trail screen body.
///
/// In list mode (`trail_screen_state.selected_audit_id == None`) delegates
/// verbatim to `screens::audit::view` for byte-identical rendering (R2.2).
/// In trail mode renders the upstream node stack + side-drawer.
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_, Message> {
    if model.trail_screen_state.selected_audit_id.is_none() {
        // ── List mode: byte-identical delegation to audit::view (R2.2) ───────
        return crate::screens::audit::view(model, mode);
    }

    // ── Trail mode: vertical node stack + side-drawer (R2.3) ──────────────

    let audit_id = model
        .trail_screen_state
        .selected_audit_id
        .as_deref()
        .unwrap_or("");

    // Back-to-list breadcrumb button.
    let back_btn = Button::new(
        Text::new("‹ Back to list")
            .size(text::SMALL)
            .color(color::ACCENT.current(mode)),
    )
    .on_press({
        // Clicking back clears selected_audit_id (returns to list mode).
        // We reuse SelectTrailRow with an empty id — this is handled in update
        // as setting selected_audit_id to Some("") which we treat as None in view.
        // Actually: we use a dedicated Message: OpenTrailFor clears by navigating to Trail with no id.
        // Cleanest: emit SwitchScreen(Trail) which does not change selected_audit_id,
        // so instead we send a direct clear via a compound: set screen + clear state.
        // For now emit TrailDrawerClosed which clears drawer but not the row.
        // The cleanest way per the design is to add the Message inline:
        // We'll use SelectTrailRow("") and handle empty = None in update.
        // Actually the decomp says drawer-close clears drawer_selected_node only.
        // A "back to list" button should clear selected_audit_id.
        // We handle this by repurposing SelectTrailRow with a special sentinel,
        // OR by adding a BackToTrailList message.
        // Decision: treat as SwitchScreen(Trail) which resets to list via the
        // current_screen routing + reset selected_audit_id in update arm.
        // We use the existing pattern: OpenTrailFor handles screen+row selection,
        // but we need back. Use SelectTrailRow with empty SmolStr as "clear" signal,
        // and handle in update: empty string → None.
        // Per the pure-function discipline, let's make SelectTrailRow("") → None.
        // This is the simplest approach without adding yet another Message variant.
        Message::SelectTrailRow(smol_str::SmolStr::default())
    })
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

    let breadcrumb = Row::new().spacing(space::M).push(back_btn).push(
        Text::new(format!("Trail · {audit_id}"))
            .size(text::SMALL)
            .color(color::FG_4.current(mode)),
    );

    // Build placeholder trail nodes for the four stages.
    // At v0.1.0 these are empty stubs — the trail-mirror backfill
    // (Wave F) populates them. Empty-stage rendering per R3.4.
    // Upstream-at-top (Q2 analyst default): Forecast → LLM → Signal → Fill.
    let drawer_selected = model.trail_screen_state.drawer_selected_node;

    // Build each node view inline to avoid borrowing from a local Vec
    // (the Element type's lifetime must not be bound to a local).
    let node_col = Column::new()
        .spacing(space::M)
        .push(trail_node::view(
            &TrailNode {
                kind: TrailNodeKind::Forecast,
                timestamp: None,
                actor: None,
                headline: None,
            },
            drawer_selected == Some(TrailNodeKind::Forecast),
            mode,
        ))
        .push(trail_node::view(
            &TrailNode {
                kind: TrailNodeKind::LlmDebate,
                timestamp: None,
                actor: None,
                headline: None,
            },
            drawer_selected == Some(TrailNodeKind::LlmDebate),
            mode,
        ))
        .push(trail_node::view(
            &TrailNode {
                kind: TrailNodeKind::Signal,
                timestamp: None,
                actor: None,
                headline: None,
            },
            drawer_selected == Some(TrailNodeKind::Signal),
            mode,
        ))
        .push(trail_node::view(
            &TrailNode {
                kind: TrailNodeKind::Fill,
                timestamp: None,
                actor: None,
                headline: None,
            },
            drawer_selected == Some(TrailNodeKind::Fill),
            mode,
        ));

    // Optional side-drawer.
    let main_area: Element<'_, Message> = if let Some(node_kind) = drawer_selected {
        // Drawer open — show node stack left, drawer right.
        Row::new()
            .spacing(space::L)
            .push(Column::new().push(node_col).width(Length::Fill))
            .push(crate::widgets::trail_drawer::view(node_kind, None, mode))
            .into()
    } else {
        Column::new().push(node_col).into()
    };

    Column::new()
        .spacing(space::M)
        .padding(space::M as u16)
        .push(breadcrumb)
        .push(main_area)
        .into()
}
