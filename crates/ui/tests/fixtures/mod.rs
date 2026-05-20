//! Test-only fixture builders for the ui-test-harness-bootstrap v0.1
//! visual-snapshot suite.
//!
//! ## Why a tests-tree module?
//!
//! Operator-locked Q9 (see
//! `spec/ui-test-harness-bootstrap/feature.md`) requires a richer
//! Charts-screen scene that includes a hovered marker
//! (`Cockpit.chart_tooltip = Some(ChartTooltipView{...})`). The
//! existing production `ui::fixtures` builders don't carry that state
//! because the cockpit binary itself never pre-populates a tooltip —
//! the tooltip is canvas-driven at runtime. Authoring this fixture in
//! `ui::fixtures` would expand the production-reachable fixture
//! surface for a test-only need; per Q6 we instead keep it in the
//! tests tree.
//!
//! ## Include from an integration test
//!
//! ```ignore
//! #[path = "fixtures/mod.rs"]
//! mod fixtures;
//!
//! let cockpit = fixtures::charts_screen_with_hovered_marker();
//! ```

// Clippy: tests can use expect / unwrap for fixture construction.
#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

pub mod visual_diff;

use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Quantity, Side, SignalView, StrategyId, Symbol, Timestamp, Venue};

use ui::state::{ChartTooltipKind, ChartTooltipView, Cockpit, PanelState};

/// Construct the Q9 hovered-marker Charts-screen fixture.
///
/// Builds on top of [`ui::test_support::charts_screen_cockpit`] (which
/// seeds the same scene the cockpit binary's `App::boot` would render)
/// and additionally pre-populates `cockpit.chart_tooltip` against the
/// first fill marker so the canvas tooltip card paints on first frame
/// — no live cursor input required.
///
/// The signal layer is seeded with two ghost signals (Buy + clamped
/// Sell) so the chart's full layer stack is exercised by the snapshot.
/// Position-mirror data is left to `test_support::charts_screen_cockpit`'s
/// pairs-steady-state seed; we don't override it here.
#[must_use]
pub fn charts_screen_with_hovered_marker() -> Cockpit {
    let mut cockpit = ui::test_support::charts_screen_cockpit();

    // Seed the ghost-signal layer (R5.4) so the visual snapshot
    // captures both marker kinds. Timestamps deliberately use the
    // same fixed `0` offset that `synthetic_candles` anchors to, so
    // the snapshot has a stable position for these markers.
    let sig_ts = Timestamp::new(
        time::OffsetDateTime::from_unix_timestamp(1_705_320_000)
            .expect("static unix timestamp must parse"),
    );
    cockpit.chart_signals = PanelState::Ready(vec![
        SignalView {
            signal_id: SmolStr::new("sig-1"),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            intended_qty: Quantity::new(dec!(0.05)).expect("fixture qty must be > 0"),
            signal_ts: sig_ts,
            strategy_id: StrategyId::new("sma_crossover"),
            was_clamped: false,
            clamp_reason: None,
        },
        SignalView {
            signal_id: SmolStr::new("sig-2"),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Sell,
            intended_qty: Quantity::new(dec!(0.04)).expect("fixture qty must be > 0"),
            signal_ts: sig_ts,
            strategy_id: StrategyId::new("sma_crossover"),
            was_clamped: true,
            clamp_reason: Some(SmolStr::new("per_symbol_cap")),
        },
    ]);

    // Seed the hovered-marker tooltip against the first fill (Q9
    // operator lock). Mirrors `state::build_tooltip_view` for a
    // Fill(0) hover: same six fields, derived from the fill the
    // fixture seeded via `synthetic_fills_for`.
    if let PanelState::Ready(fills) = &cockpit.chart_markers
        && let Some(first) = fills.first()
    {
        let price = first.price.get();
        let qty = first.qty.get();
        cockpit.chart_tooltip = Some(ChartTooltipView {
            kind: ChartTooltipKind::Fill,
            side: first.side,
            price: Some(price),
            qty,
            notional: Some(price.saturating_mul(qty)),
            ts: first.venue_ts,
            strategy_id: None,
            was_clamped: false,
            clamp_reason: None,
        });
    }

    // Defence-in-depth: BTC universe + Binance is the architect's
    // committed default for the floor / typical / operator
    // baselines. The factory already sets these, but assert here
    // so a future refactor of `charts_screen_cockpit` doesn't
    // silently shift the baseline scene.
    debug_assert_eq!(
        cockpit.selected_symbol,
        Some((Venue::Binance, Symbol::new("BTCUSDT"))),
        "Q10 baselines expect BTC selected"
    );
    // Notional should land at 0.1 * price_first_fill — sanity-check
    // the cockpit hasn't been re-seeded with empty fills.
    debug_assert!(
        cockpit.chart_tooltip.is_some(),
        "Q9 hovered-marker fixture must set chart_tooltip"
    );

    cockpit
}

// ── Phase D+ snapshot fixtures (ui-rethink-phase-d-trail-followup Wave C) ─────

/// Construct the `trail__steady_state` fixture: Trail screen in list mode
/// (byte-identical to `audit::view` per R2.2).
///
/// The cockpit is set to `Screen::Trail` with `selected_audit_id = None`
/// so the trail screen delegates to `screens::audit::view` (list mode).
/// Seeded with 5 journal rows so the audit table renders `PanelState::Ready`
/// (avoids the `ThrottledSpinner` whose frame counter is non-deterministic
/// across consecutive `iced_test::screenshot` calls).
#[must_use]
pub fn trail_steady_state_cockpit() -> Cockpit {
    use ui::state::{AuditScreenState, Screen};
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Trail;
    // Ensure list mode (no row selected).
    cockpit.trail_screen_state = Default::default();
    // Seed Ready rows — prevents the loading spinner (non-deterministic
    // frame position) from appearing and invalidating the baseline.
    cockpit.audit_screen_state = AuditScreenState {
        rows: PanelState::Ready(ui::fixtures::fake_journal_rows(5)),
        total_count: Some(5),
        ..Default::default()
    };
    cockpit
}

/// Construct the `trail__side_drawer_open` fixture: Trail screen in trail
/// mode with a Forecast-stage payload and the side-drawer open.
///
/// Uses a deterministic `ReconstructedTrailUi` fixture (fixed-seed strings).
/// Drawer is open to `TrailNodeKind::Forecast`.
#[must_use]
pub fn trail_side_drawer_open_cockpit() -> Cockpit {
    use smol_str::SmolStr;
    use ui::state::{ReconstructedTrailUi, Screen, TrailScreenState, TrailStageUi};
    use ui::widgets::trail_node::{TrailNode, TrailNodeKind};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Trail;

    // Build a deterministic reconstructed trail fixture.
    let fill_ts = SmolStr::new("12:34:56.789");
    let sig_ts = SmolStr::new("12:34:55.123");
    let fc_ts = SmolStr::new("12:34:54.001");

    let fill = TrailStageUi {
        timestamp: Some(fill_ts.to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy 0.05 BTCUSDT @ 42000.00".to_string()),
        raw_payload: Some(r#"{"fill_id":"abc123","qty":0.05}"#.to_string()),
    };
    let signal = TrailStageUi {
        timestamp: Some(sig_ts.to_string()),
        actor: Some("strategy:sma_crossover".to_string()),
        headline: Some("Buy signal triggered (SMA crossover)".to_string()),
        raw_payload: Some(r#"{"signal_id":"sig001"}"#.to_string()),
    };
    let forecast = TrailStageUi {
        timestamp: Some(fc_ts.to_string()),
        actor: Some("tcn:abc12345".to_string()),
        headline: Some("Bullish p=0.72 horizon=15m".to_string()),
        raw_payload: Some(r#"{"forecast_id":"fc001","confidence":0.72}"#.to_string()),
    };
    let debate = TrailStageUi::default();

    // Pre-build nodes (upstream-first: Forecast, LlmDebate, Signal, Fill).
    let nodes = vec![
        TrailNode {
            kind: TrailNodeKind::Forecast,
            timestamp: forecast.timestamp.clone(),
            actor: forecast.actor.clone(),
            headline: forecast.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::LlmDebate,
            timestamp: None,
            actor: None,
            headline: None,
        },
        TrailNode {
            kind: TrailNodeKind::Signal,
            timestamp: signal.timestamp.clone(),
            actor: signal.actor.clone(),
            headline: signal.headline.clone(),
        },
        TrailNode {
            kind: TrailNodeKind::Fill,
            timestamp: fill.timestamp.clone(),
            actor: fill.actor.clone(),
            headline: fill.headline.clone(),
        },
    ];

    let trail = ReconstructedTrailUi {
        audit_id: SmolStr::new("fixture-audit-id-001"),
        fill,
        signal,
        forecast,
        debate,
        nodes,
    };

    cockpit.trail_screen_state = TrailScreenState {
        selected_audit_id: Some(SmolStr::new("fixture-audit-id-001")),
        drawer_selected_node: Some(TrailNodeKind::Forecast),
        reconstructed_trail: Some(trail),
        pending_trail_audit_id: None,
    };
    cockpit
}

/// Construct the `live__recent_activity_with_chevron` fixture: Live screen
/// with 5 rows in `agent_feed::ready_body` (the recent-activity tape) and
/// the universal chevron rendered on every row (Phase D R5.1).
///
/// The cockpit is set to `Screen::Live` with 5 fill rows in `tape`.
#[must_use]
pub fn live_recent_activity_with_chevron_cockpit() -> Cockpit {
    use ui::state::Screen;
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = Screen::Live;
    // 5-row tape — matches the R2.3 fixture spec.
    cockpit.tape =
        ui::state::PanelState::Ready(ui::fixtures::fake_fill_feed(5).into_iter().collect());
    cockpit
}

/// Silence dead-code warning for fixtures only consumed via `mod`
/// glob in integration tests. (Cargo runs each `tests/*.rs` as a
/// separate crate, so some fixture exports look unused per-target
/// even when other targets exercise them.)
#[allow(dead_code)]
pub fn _ensure_decimal_dep() -> rust_decimal::Decimal {
    dec!(0)
}
