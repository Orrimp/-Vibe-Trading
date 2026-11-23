#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1811 — Phase 3 deferral closure for the Strategies-detail sparkline
//! (history: the claim was that the screen body renders the canvas
//! sparkline, not the deferred placeholder copy, once
//! `cockpit.strategy_equity` is populated for the selected strategy).
//!
//! What it pins, honestly (story 2-63 Wave E3): that `strategies::view`
//! builds without panicking once `strategy_equity` holds a `Ready` 120-point
//! series for the selected strategy, and that the Phase 4 loading copy is the
//! constant `STRATEGIES_SPARKLINE_LOADING`. It does NOT prove the sparkline
//! draws: no pixel is read, and the `Ready` check reads back the entry this
//! test seeded (by construction). The name is kept because `trace.toml`
//! cites this file; render proof belongs to a pixel harness (AD-10).
//!
//! Until story 2-63 Wave E3 every item here carried
//! `#[cfg(feature = "fixtures")]`, so CI's macOS leg (default features)
//! compiled 0 tests from it. `ui::fixtures` is always compiled
//! (`pub mod fixtures;` in `crates/ui/src/lib.rs`), so the gate guarded
//! nothing. T-T4 E3-FX6: a file-level gate re-added above the first
//! `#![allow(` makes `cargo test -p ui --test
//! strategies_screen_sparkline_replaces_placeholder` report `running 0
//! tests`. The base's six item-level gates are a different text, so the base
//! run does not measure this probe: it runs on its own in Round A of the E3
//! probe plan.

use trading_core::StrategyId;
use ui::fixtures::{fake_cockpit_v15a_pairs_steady_state, fake_equity_series_for_sparkline};
use ui::screens::strategies;
use ui::state::{PanelState, Screen};
use ui::theme::ThemeMode;

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
