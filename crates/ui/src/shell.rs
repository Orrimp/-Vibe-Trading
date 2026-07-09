//! Shell view — Phase 2 (T1603), extended Phase A (ui-rethink-phase-a-lab).
//!
//! Composes the screen-routed shell: `Row[sidebar | (body + status_bar) |
//! reserved-right-rail]`. Both bins (`cockpit`, `cockpit_live`) call
//! `shell::view` so the iced widget tree is identical pixel-for-pixel
//! across them.
//!
//! Phase 6 swaps the right-rail's `Length::Fixed(0.0)` to a real width
//! when the v2-LLM Assistant ships; Phase 2's job is just to leave the
//! spot. (Q7 ratification.)
//!
//! Halted-banner integration: rendered inside the right-side `Column`
//! between any chrome and the screen body so it remains visible across
//! every screen (R3.3 / R14.2). Phase 1's banner trip logic is in the
//! `kill::view` widget body and stays untouched.
//!
//! **Phase A (T-D-3):** `screen_body` gains 7-arm match covering the new
//! active routes (Lab, Live, Compare, Memory, Models, Trail, Settings)
//! plus the deprecated legacy aliases. Placeholder routes render the
//! `widgets::placeholder` empty-state card per Design § 6.
//!
//! **Zero string literals** — strings via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::Length;
use iced::widget::{Column, Container, Row, Stack};

use crate::assistant;
use crate::screens::{
    baseline, compare, forward_plan, lab, leaderboard, live, memory, models, reports, settings,
    strategy_registry, trail, tune,
};
use crate::state::{Cockpit, Screen};
use crate::theme::layout::{
    RIGHT_RAIL_OPEN_WIDTH_PX, RIGHT_RAIL_WIDTH_PX, SIDEBAR_ENTRIES_PHASE_A, SIDEBAR_GROUPS_PHASE_C,
};
use crate::theme::{ThemeMode, color};
use crate::widgets::{sidebar_nav, stage_stepper, status_bar, toast_tray};

/// Render the full cockpit shell.
#[allow(clippy::needless_pass_by_value, clippy::cast_possible_truncation)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let sidebar = sidebar_nav::view(
        model.current_screen,
        SIDEBAR_ENTRIES_PHASE_A,
        SIDEBAR_GROUPS_PHASE_C,
        mode,
    );
    let body = screen_body(model.current_screen, model, mode);
    let bar = status_bar::view(model);

    // advisor-calibrate-stage (R3-3a / ADR-0083 D1) — the DATA → CALIBRATE →
    // ANALYZE → SUGGEST spine orientation band, pushed at the TOP of the centre
    // Column (above `body`, mirroring the halted-banner placement) so it spans
    // every advisor-journey screen consistently. It is NOT a router: the
    // highlighted stage is resolved by the pure `stage_for` over the current
    // screen + the EXISTING leaderboard result substate (Leaderboard+Empty →
    // DATA, Leaderboard+Ready → ANALYZE; Tune → CALIBRATE; ForwardPlan →
    // SUGGEST), and the band is elided (pixel-silent) off the journey.
    let spine_stage =
        stage_stepper::stage_for(model.current_screen, &model.leaderboard_screen_state.result);
    let stepper = stage_stepper::view(spine_stage, mode);

    // Phase F T-D-N17 — Assistant slot wake (K6 Option A).
    // When `assistant_state.is_open == false`, `assistant::view::view` returns a
    // 0-width Container (byte-identical to the old `Space::new()` path) using the
    // shell's outer Fixed width (set below). When open it returns a rendered stub
    // placeholder at `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`.
    //
    // The outer Container picks the width based on `is_open`; the inner
    // `assistant::view::view` element fills its parent. This preserves the
    // `RIGHT_RAIL_WIDTH_PX = 0.0` constant (the shell_grid invariant test
    // reads the constant — it does NOT measure the rendered pixel width at
    // runtime — so K6 is satisfied).
    let rail_width = if model.assistant_state.is_open {
        Length::Fixed(RIGHT_RAIL_OPEN_WIDTH_PX)
    } else {
        Length::Fixed(RIGHT_RAIL_WIDTH_PX)
    };
    // v3-llm-forecaster Wave F (T-D-N(F2)) — switch to the Cockpit-aware
    // entry point so the reasoning-trace body can look up cited-lesson
    // bodies in `memory_screen_state.cache`. The `Offline` mode (R9.3
    // default-disabled) short-circuits before touching the cache, so the
    // existing `assistant_slot__open_stub` baseline stays byte-identical.
    let right_track = Container::new(assistant::view::view_with_cockpit(model, mode))
        .width(rail_width)
        .height(Length::Fill);

    let centre = Column::new()
        .push(stepper)
        .push(body)
        .push(bar)
        .width(Length::Fill)
        .height(Length::Fill);

    let shell_row = Row::new()
        .push(sidebar)
        .push(centre)
        .push(right_track)
        .width(Length::Fill)
        .height(Length::Fill);

    // cockpit-toast-queue v0.1.0 (ADR-0046 § Shell wiring / T-D-N6).
    // Wrap the shell body in a two-layer `Stack`:
    //   Layer 0 (lowest z): the existing `shell_row` Container — untouched chrome.
    //   Layer 1 (highest z): `toast_tray::view` overlay — pixel-silent when empty.
    //
    // The outer Container retains the existing style closure (CANVAS background +
    // FG_1 text color) so the shell-grid invariant test stays green.
    //
    // Q4=(a): tray is positioned above the 24 px activity tape via
    // `TOAST_TRAY_BOTTOM_OFFSET_PX = 28` inside `toast_tray::view`.
    //
    // T-D-N6 open Q: if hit-test bleed-through surfaces on the 0-sized top
    // layer, fall back to conditional Stack-vs-bare-Container at view-time.
    // Architect's expectation: bleed-through won't happen with an empty-queue
    // path that returns a shrink Container rather than a fill Container.
    let shell_with_toasts = Stack::new()
        .push(
            Container::new(shell_row)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(toast_tray::view(&model.toast_queue, mode));

    Container::new(shell_with_toasts)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(color::CANVAS.current(mode).into()),
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Dispatch on `Cockpit::current_screen` to pick the screen body.
///
/// Phase A (T-D-3) — 7-arm match for the new routes. Deprecated alias
/// variants auto-route to their successors so the test harness doesn't
/// need to migrate in one cycle (R9.3 / Design § 6).
///
/// Phase C (ui-rethink-phase-c-sidebar-ia) — Live routes to `live::view`;
/// Settings/Risk/Debug/Control route to `settings::view` (with tab
/// pre-selected by `update`'s `SwitchScreen` arm per Design § A4);
/// Strategies routes to `strategy_registry::view`.
///
/// `home::view` is retained as a source file for one cycle (R2.4 / Q1a).
#[allow(clippy::needless_pass_by_value, deprecated)]
#[must_use]
pub fn screen_body(screen: Screen, model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    match screen {
        // ── Phase C active routes ─────────────────────────────────────────
        Screen::Lab | Screen::Charts => lab::view(model, mode),
        // Phase C: Live routes to the new §J6 layout; Home is the compat alias.
        Screen::Live | Screen::Home => live::view(model, mode),
        // Phase E: Compare routes to the matrix screen (replaces Phase A placeholder).
        Screen::Compare => compare::view(model, mode),
        // cockpit-baseline-panel v0.1.0: passive-BH baseline screen (D2 —
        // navigable, not default-routed; the smoke's default stays on Live).
        Screen::Baseline => baseline::view(model, mode),
        // advisor-leaderboard-screen v0.1.0: the strategy bake-off leaderboard
        // (single-coin advisor journey step 3 — navigable via the Work group,
        // not default-routed).
        Screen::Leaderboard => leaderboard::view(model, mode),
        // advisor-forward-plan v0.1.0 (F6): the forward buy/sell plan
        // (single-coin advisor journey step 4 — navigable via the Work group,
        // between the crowned Leaderboard pick and the Live view).
        Screen::ForwardPlan => forward_plan::view(model, mode),
        // advisor-param-tuning (ADR-0069): the gate-tied hyperparameter sweep
        // editor ("Tune") — a power-user drill-down off a Leaderboard row's
        // "Tune…" affordance (navigable, NOT sidebar-default-routed).
        Screen::Tune => tune::view(model, mode),
        // Phase F: Memory routes to the full memory screen (replaces Phase A placeholder).
        Screen::Memory => memory::view(model, mode),
        // Phase F: Models routes to the full models screen (replaces Phase A placeholder).
        Screen::Models => models::view(model, mode),
        // cockpit-reports-viewer v0.1.0: browse + render committed backtest
        // reports (D5 — navigable via the Library group, not default-routed).
        Screen::Reports => reports::view(model, mode),
        // Phase D: Trail routes to the new trail::view which delegates to
        // audit::view in list mode (R2.2 byte-identity gate) and renders
        // the upstream node stack in trail mode (R2.3).
        // Screen::Audit is the deprecated alias (R2.4) — routes to the same body.
        Screen::Trail | Screen::Audit => trail::view(model, mode),
        // Phase C: Settings rollup wraps risk/control/debug sub-tabs.
        // R5.2: deprecated Risk/Debug/Control aliases route here too —
        // the active tab is pre-selected by the `SwitchScreen` arm in `update`.
        Screen::Settings | Screen::Risk | Screen::Debug | Screen::Control => {
            settings::view(model, mode)
        }

        // Phase C: Strategy registry replaces the old detail panel.
        Screen::Strategies => strategy_registry::view(model, mode),
    }
}
