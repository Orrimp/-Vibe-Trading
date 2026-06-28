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
//!
//! ## Historical note — regime-tag column removed 2026-05-29
//!
//! Wave D of `v3-regime-classifier` (commit `ced662d`) added a
//! `regime_tag_cell` + `regime_tag_column_header` pair to this module,
//! never wired into `view()`. v3 was operator-retired 2026-05-29 after
//! Wave E proved a -0.294 Sharpe-delta against the v1 momentum baseline
//! (see `spec/v1/v3-regime-classifier/feature.md` shipped_disposition).
//! Per the durable-over-quick contract the dead helpers + their snapshot
//! scaffolding were excised by
//! `post-v3-retirement-trail-ui-cleanup v0.1.0` — see
//! `spec/dev-notes/post-v3-trail-ui-cleanup-2026-05-29.md`.

use iced::widget::{Button, Column, Row, Text, button};
use iced::{Border, Element, Length};

use crate::state::{Cockpit, Message};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::trail_drawer::DrawerPayload;
use crate::widgets::trail_node::{self, TrailNode, TrailNodeKind};

/// Build the side-drawer payload for the selected `kind` from the hydrated
/// `reconstructed_trail`. Returns `Some(DrawerPayload::Fill { metadata_json })`
/// — the scrollable raw-JSON viewer — when the matching stage carries a
/// `raw_payload` (the serialised SQL row). Returns `None` (drawer renders the
/// placeholder body) when the trail is unhydrated or the stage has no payload.
///
/// Why `Fill` for every kind: see the data-plumbing follow-up note at the
/// drawer construction site. The structured per-kind variants require discrete
/// columns that `reflection::TrailStage` does not yet emit; the honest,
/// non-fabricating render today is the raw row under the correct per-kind
/// title (the drawer title comes from `kind`, not the payload variant).
fn drawer_payload_for(model: &Cockpit, kind: TrailNodeKind) -> Option<DrawerPayload> {
    let trail = model.trail_screen_state.reconstructed_trail.as_ref()?;
    let stage = match kind {
        TrailNodeKind::Forecast => &trail.forecast,
        TrailNodeKind::LlmDebate => &trail.debate,
        TrailNodeKind::Signal => &trail.signal,
        TrailNodeKind::Fill => &trail.fill,
    };
    stage.raw_payload.as_ref().map(|json| DrawerPayload::Fill {
        metadata_json: json.clone(),
    })
}

/// Static fallback nodes (all-`None`) used when `reconstructed_trail` is
/// `None` (SQL backfill not yet completed). All-`None` fields resolve to
/// `'static` string literals inside `trail_node::view` — no lifetime issue.
///
/// Module-level placement required by `clippy::items_after_statements`
/// (items inside fn bodies after any statement are prohibited under
/// `-D warnings`).
static FALLBACK_NODES: std::sync::LazyLock<Vec<TrailNode>> = std::sync::LazyLock::new(|| {
    vec![
        TrailNode {
            kind: TrailNodeKind::Forecast,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::LlmDebate,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::Signal,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::Fill,
            timestamp: None,
            actor: None,
            headline: None,
        },
    ]
});

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

    let audit_id = model
        .trail_screen_state
        .selected_audit_id
        .as_deref()
        .unwrap_or("");

    // Phase D+ (T-D-N10) — while `pending_trail_audit_id` is set (chevron
    // clicked but mirror hasn't responded yet), render a loading placeholder
    // per R3.4 of the predecessor.  We construct the `Text` widget inline
    // (not via `frame::loading_with_spinner`) so the owned `String` does not
    // produce an E0515 "borrows local" lifetime error on early return.
    if model.trail_screen_state.pending_trail_audit_id.is_some()
        && model.trail_screen_state.reconstructed_trail.is_none()
    {
        let loading_text = format!("Loading trail for {audit_id}\u{2026}");
        return Text::new(loading_text)
            .size(text::BODY)
            .color(color::FG_3.current(mode))
            .into();
    }

    // ── Trail mode: vertical node stack + side-drawer (R2.3) ──────────────

    // Back-to-list breadcrumb button.
    let back_btn = Button::new(
        Text::new("\u{2039} Back to list")
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
        Text::new(format!("Trail \u{00b7} {audit_id}"))
            .size(text::SMALL)
            .color(color::FG_4.current(mode)),
    );

    // Build trail nodes for the four stages.
    //
    // Phase D+ (T-D-N10): `ReconstructedTrailUi::nodes` is a `Vec<TrailNode>`
    // stored WITHIN `Cockpit`, so borrowing `&nodes[i]` gives a reference with
    // the Cockpit's lifetime (`'_`). This avoids E0515: `trail_node::view<'a>`
    // returns `Element<'a>` bound to `&'a TrailNode`; the borrow must outlive
    // the returned `Element<'_>` from this function, which it does when the
    // nodes live inside `model`.
    //
    // When `reconstructed_trail` is `None` (SQL backfill not yet completed),
    // fall back to the four static empty-stub nodes per R3.4. The static nodes
    // use all-`None` string fields which resolve to `'static` string literals
    // inside `trail_node::view` — no lifetime issue.
    let drawer_selected = model.trail_screen_state.drawer_selected_node;

    // Borrow the pre-built node slice from `reconstructed_trail` if hydrated;
    // fall back to the module-level `FALLBACK_NODES` (all-`None`) per R3.4.
    let nodes: &[TrailNode] = model
        .trail_screen_state
        .reconstructed_trail
        .as_ref()
        .map_or(FALLBACK_NODES.as_slice(), |t| t.nodes.as_slice());

    // Build node views. Each `trail_node::view(&nodes[i])` returns
    // `Element<'a>` where `'a` is the Cockpit's lifetime (correct).
    let mut node_col = Column::new().spacing(space::M);
    for (i, kind) in [
        TrailNodeKind::Forecast,
        TrailNodeKind::LlmDebate,
        TrailNodeKind::Signal,
        TrailNodeKind::Fill,
    ]
    .iter()
    .enumerate()
    {
        if let Some(node) = nodes.get(i) {
            node_col = node_col.push(trail_node::view(node, drawer_selected == Some(*kind), mode));
        }
    }

    // Optional side-drawer.
    let main_area: Element<'_, Message> = if let Some(node_kind) = drawer_selected {
        // Drawer open — show node stack left, drawer right.
        //
        // Build the drawer payload from the SELECTED stage's `raw_payload`
        // (the serialised SQL row), which lives in `reconstructed_trail`.
        // The drawer takes the payload BY VALUE, so we can construct it from
        // an owned clone here with no E0515 lifetime concern.
        //
        // NOTE (data-plumbing follow-up): the structured `DrawerPayload`
        // variants (`Forecast { direction, confidence, .. }`, `Signal { side,
        // intended_qty, .. }`) carry discrete fields that are NOT yet plumbed
        // onto `TrailStageUi`/`reflection::TrailStage` — that mirror is a
        // default-only stub at v0.1.0 (only the opaque `raw_payload: Option
        // <String>` exists). Until the reflection layer parses the four
        // correlation rows into discrete columns, we render the REAL raw-JSON
        // payload via the scrollable `Fill` body under the correct per-kind
        // title (NOT the LLM "(no transcript)" placeholder for every kind).
        // When a stage has no `raw_payload` (e.g. the always-`None` LLM debate
        // stage, or an unhydrated stage), the drawer falls back to the
        // placeholder body.
        let payload = drawer_payload_for(model, node_kind);
        Row::new()
            .spacing(space::L)
            .push(Column::new().push(node_col).width(Length::Fill))
            .push(crate::widgets::trail_drawer::view(node_kind, payload, mode))
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
