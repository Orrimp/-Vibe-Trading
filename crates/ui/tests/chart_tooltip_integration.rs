#![allow(
    clippy::field_reassign_with_default,
    clippy::expect_used,
    clippy::bool_assert_comparison,
    clippy::unwrap_used
)]
//! T2011 — Chart tooltip integration test (chart-buy-sell-emphasis v1.9).
//!
//! Drives the new `Message::ChartMarkerHovered` /
//! `Message::ChartMarkerHoverEnded` / `Message::ChartSignalsLoaded` arms
//! through `ui::state::update` and asserts on the resulting
//! `Cockpit.chart_tooltip` + `Cockpit.chart_signals` state transitions.
//!
//! Pure unit-style integration test — no iced application, no rendered
//! widgets. Snapshot-style chart coverage lives in
//! `crates/ui/src/widgets/chart.rs` `#[test]`s; this file is the V3
//! cross-arm wiring gate.

use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{
    FeeTier, FillView, Money, Price, Quantity, Side, SignalView, StrategyId, Symbol, Timestamp,
};

use ui::state::{update, ChartMarkerIndex, ChartTooltipKind, Cockpit, Message, PanelState};

fn fixed_ts(offset_secs: i64) -> Timestamp {
    let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000 + offset_secs)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    Timestamp::new(dt)
}

fn make_fill(offset_secs: i64, side: Side, price_dec: rust_decimal::Decimal) -> FillView {
    FillView {
        symbol: Symbol::new("BTCUSDT"),
        side,
        price: Price::new(price_dec).unwrap(),
        qty: Quantity::new(dec!(0.1)).unwrap(),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: fixed_ts(offset_secs),
        transaction_id: SmolStr::new(format!("tx-{offset_secs}")),
    }
}

fn make_signal(offset_secs: i64, side: Side, clamped: bool) -> SignalView {
    SignalView {
        signal_id: SmolStr::new(format!("sig-{offset_secs}")),
        symbol: Symbol::new("BTCUSDT"),
        side,
        intended_qty: Quantity::new(dec!(0.05)).unwrap(),
        signal_ts: fixed_ts(offset_secs),
        strategy_id: StrategyId::new("sma_crossover"),
        was_clamped: clamped,
        clamp_reason: if clamped {
            Some(SmolStr::new("per_symbol_cap"))
        } else {
            None
        },
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

/// T2011 V3 — `ChartMarkerHovered(Fill(0))` against a Ready marker slice
/// populates `chart_tooltip` with a Fill-variant view carrying the six
/// R4.2 fields.
#[test]
fn chart_marker_hovered_fill_populates_tooltip() {
    let mut cockpit = Cockpit::default();
    cockpit.chart_markers = PanelState::Ready(vec![make_fill(0, Side::Buy, dec!(40_010))]);

    // Cold-start: no tooltip.
    assert!(cockpit.chart_tooltip.is_none());

    // Hover on fill 0 → tooltip populated.
    update(
        &mut cockpit,
        Message::ChartMarkerHovered(ChartMarkerIndex::Fill(0)),
    );

    let tt = cockpit
        .chart_tooltip
        .expect("hover should populate tooltip");
    assert!(matches!(tt.kind, ChartTooltipKind::Fill));
    assert_eq!(tt.side, Side::Buy);
    assert_eq!(tt.price, Some(dec!(40_010)));
    assert_eq!(tt.qty, dec!(0.1));
    // Notional == price × qty.
    assert_eq!(tt.notional, Some(dec!(4_001)));
    assert_eq!(tt.was_clamped, false);
    assert!(tt.clamp_reason.is_none());
}

/// T2011 V3 — `ChartMarkerHoverEnded` clears the tooltip.
#[test]
fn chart_marker_hover_ended_clears_tooltip() {
    let mut cockpit = Cockpit::default();
    cockpit.chart_markers = PanelState::Ready(vec![make_fill(0, Side::Buy, dec!(40_010))]);
    update(
        &mut cockpit,
        Message::ChartMarkerHovered(ChartMarkerIndex::Fill(0)),
    );
    assert!(cockpit.chart_tooltip.is_some());

    update(&mut cockpit, Message::ChartMarkerHoverEnded);
    assert!(cockpit.chart_tooltip.is_none());
}

/// T2019 — `ChartMarkerHovered(Signal(0))` against a Ready signal slice
/// populates the tooltip with a Signal-variant view that omits price +
/// notional (R5.6).
#[test]
fn chart_marker_hovered_signal_populates_ghost_tooltip() {
    let mut cockpit = Cockpit::default();
    cockpit.chart_signals = PanelState::Ready(vec![make_signal(0, Side::Sell, true)]);

    update(
        &mut cockpit,
        Message::ChartMarkerHovered(ChartMarkerIndex::Signal(0)),
    );

    let tt = cockpit
        .chart_tooltip
        .expect("ghost hover populates tooltip");
    assert!(matches!(tt.kind, ChartTooltipKind::Signal));
    assert_eq!(tt.side, Side::Sell);
    assert!(tt.price.is_none(), "ghost variant has no price");
    assert!(tt.notional.is_none(), "ghost variant has no notional");
    assert_eq!(tt.qty, dec!(0.05));
    assert!(tt.was_clamped);
    assert_eq!(tt.clamp_reason.as_deref(), Some("per_symbol_cap"));
}

/// T2011 — Out-of-range index against a `Ready` slice clears the tooltip
/// (defence-in-depth across the async refresh boundary). The canvas
/// `update` could publish a stale index if a refresh swapped the slice;
/// the state update arm should not panic and should not retain a stale
/// tooltip.
#[test]
fn chart_marker_hovered_out_of_range_clears_tooltip() {
    let mut cockpit = Cockpit::default();
    cockpit.chart_markers = PanelState::Ready(vec![make_fill(0, Side::Buy, dec!(40_010))]);

    update(
        &mut cockpit,
        Message::ChartMarkerHovered(ChartMarkerIndex::Fill(99)),
    );
    assert!(cockpit.chart_tooltip.is_none());
}

/// T2011 — Hover against a `Loading` slice (cold-start: marker fetch in
/// flight) doesn't populate a tooltip.
#[test]
fn chart_marker_hovered_against_loading_slice_clears_tooltip() {
    let mut cockpit = Cockpit::default();
    assert!(matches!(cockpit.chart_markers, PanelState::Loading));

    update(
        &mut cockpit,
        Message::ChartMarkerHovered(ChartMarkerIndex::Fill(0)),
    );
    assert!(cockpit.chart_tooltip.is_none());
}

/// T2021 — `ChartSignalsLoaded(Ok(signals))` flips `chart_signals` from
/// `Loading` to `Ready(signals)`; `Err` flips to `Error`.
#[test]
fn chart_signals_loaded_flips_panel_state() {
    let mut cockpit = Cockpit::default();
    assert!(matches!(cockpit.chart_signals, PanelState::Loading));

    let signals = vec![make_signal(0, Side::Buy, false)];
    update(
        &mut cockpit,
        Message::ChartSignalsLoaded(Ok(signals.clone())),
    );
    match &cockpit.chart_signals {
        PanelState::Ready(v) => assert_eq!(v.len(), 1),
        other => panic!("expected Ready, got {other:?}"),
    }

    update(
        &mut cockpit,
        Message::ChartSignalsLoaded(Err(SmolStr::new("query failed"))),
    );
    match &cockpit.chart_signals {
        PanelState::Error(msg) => assert_eq!(msg.as_str(), "query failed"),
        other => panic!("expected Error, got {other:?}"),
    }
}

/// T2011 — `SelectSymbol` clears the tooltip and resets both marker and
/// signal panel states to `Loading` so a hover on the old symbol's
/// markers can't survive the symbol switch.
#[test]
fn select_symbol_clears_tooltip_and_resets_panels() {
    let mut cockpit = Cockpit::default();
    cockpit.chart_markers = PanelState::Ready(vec![make_fill(0, Side::Buy, dec!(40_010))]);
    cockpit.chart_signals = PanelState::Ready(vec![make_signal(0, Side::Buy, false)]);
    update(
        &mut cockpit,
        Message::ChartMarkerHovered(ChartMarkerIndex::Fill(0)),
    );
    assert!(cockpit.chart_tooltip.is_some());

    update(
        &mut cockpit,
        Message::SelectSymbol(trading_core::Venue::Binance, Symbol::new("ETHUSDT")),
    );

    assert!(
        cockpit.chart_tooltip.is_none(),
        "tooltip cleared on symbol switch"
    );
    assert!(matches!(cockpit.chart_markers, PanelState::Loading));
    assert!(matches!(cockpit.chart_signals, PanelState::Loading));
}
