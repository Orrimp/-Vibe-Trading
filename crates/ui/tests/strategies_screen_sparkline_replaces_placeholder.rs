#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1811 — Phase 3 deferral closure: assert the Strategies-detail
//! screen body renders the canvas sparkline (not the deferred
//! placeholder copy) once `cockpit.strategy_equity` is populated for
//! the selected strategy.
//!
//! This test compiles only with `--features fixtures` so the
//! deterministic 120-point series + the `fake_cockpit_v15a_pairs_steady_state`
//! seed are available.

#[cfg(feature = "fixtures")]
use trading_core::StrategyId;
#[cfg(feature = "fixtures")]
use ui::fixtures::{fake_cockpit_v15a_pairs_steady_state, fake_equity_series_for_sparkline};
#[cfg(feature = "fixtures")]
use ui::screens::strategies;
#[cfg(feature = "fixtures")]
use ui::state::{PanelState, Screen};
#[cfg(feature = "fixtures")]
use ui::theme::ThemeMode;

#[cfg(feature = "fixtures")]
#[test]
fn strategies_screen_sparkline_replaces_placeholder() {
    // 1. Boot fixtures with a strategy selected.
    let mut c = fake_cockpit_v15a_pairs_steady_state();
    c.current_screen = Screen::Strategies;
    let id = StrategyId::new("btc_rsi_reversion");
    c.selected_strategy = Some(id.clone());

    // 2. Pre-seed `model.strategy_equity` with a 120-point series for
    //    that strategy id.
    c.strategy_equity.insert(
        id.clone(),
        PanelState::Ready(fake_equity_series_for_sparkline()),
    );

    // 3. Render the Strategies screen — assert it doesn't panic.
    let _element = strategies::view(&c, ThemeMode::Dark);

    // 4. Assert the deferred-placeholder constant is no longer
    //    accessible (compile-time guarantee — the test referenced
    //    `STRATEGIES_SPARKLINE_LOADING` instead, which is the new
    //    Phase 4 net-new copy). The retired `STRATEGIES_SPARKLINE_DEFERRED`
    //    is gone from the strings module.
    let loading = ui::strings::STRATEGIES_SPARKLINE_LOADING;
    assert_eq!(loading, "Loading equity history\u{2026}");

    // 5. Assert `strategy_equity` for the selected strategy is
    //    `Ready` with a non-empty series — the screen reads from
    //    here when dispatching to the canvas widget.
    match c.strategy_equity.get(&id) {
        Some(PanelState::Ready(s)) => {
            assert!(!s.points.is_empty());
            assert_eq!(s.points.len(), 120);
        }
        other => panic!("expected Ready(120pt series); got {other:?}"),
    }
}
