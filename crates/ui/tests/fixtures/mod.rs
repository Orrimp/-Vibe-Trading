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

/// Silence dead-code warning for fixtures only consumed via `mod`
/// glob in integration tests. (Cargo runs each `tests/*.rs` as a
/// separate crate, so some fixture exports look unused per-target
/// even when other targets exercise them.)
#[allow(dead_code)]
pub fn _ensure_decimal_dep() -> rust_decimal::Decimal {
    dec!(0)
}
