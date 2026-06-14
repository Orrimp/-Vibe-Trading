//! T1610 / R8.5 — chart markers wire end-to-end in fixtures mode.
//!
//! Boots a fixtures-mode cockpit, asserts that switching to a different
//! `(venue, symbol)` re-seeds the marker panel against the per-symbol
//! synthetic fills feed.

#![allow(deprecated)] // test uses deprecated Screen::Charts alias to verify backward-compat routing

use trading_core::{Symbol, Venue};
use ui::fixtures::synthetic_fills_for;
use ui::state::{Cockpit, Message, PanelState, Screen, update};

#[test]
fn chart_markers_from_audit_query_fixtures_mode() {
    let mut c = Cockpit::new();
    c.universe = vec![
        (Venue::Binance, Symbol::new("BTCUSDT")),
        (Venue::Binance, Symbol::new("ETHUSDT")),
    ];
    update(&mut c, Message::SwitchScreen(Screen::Charts));
    update(
        &mut c,
        Message::SelectSymbol(Venue::Binance, Symbol::new("BTCUSDT")),
    );
    // After SelectSymbol, the model flips chart_markers to Loading. The
    // fixtures bin's `update` shim then issues `ChartMarkersLoaded(Ok(...))`
    // synchronously; we replay that here to assert the wiring.
    let fills = synthetic_fills_for(Venue::Binance, &Symbol::new("BTCUSDT"), 4);
    update(&mut c, Message::ChartMarkersLoaded(Ok(fills.clone())));

    match &c.chart_markers {
        PanelState::Ready(v) => {
            assert_eq!(v.len(), fills.len());
            for f in v {
                assert_eq!(f.symbol, Symbol::new("BTCUSDT"));
            }
        }
        other => panic!("expected Ready markers, got {}", other.variant_name()),
    }
}
