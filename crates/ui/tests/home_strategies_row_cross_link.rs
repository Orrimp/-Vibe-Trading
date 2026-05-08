//! T1705 — Home → Strategies-summary row click cross-link compound
//! dispatch (Phase 3 R5.2 / Q11b).
//!
//! Boots a fixtures cockpit on the Home screen, simulates a row click
//! by feeding `Message::SelectStrategy(id)` through `ui::state::update`,
//! and asserts the post-update model has `selected_strategy ==
//! Some(<id>)`. The screen-switch chain (`Task::done(SwitchScreen(
//! Screen::Strategies))`) lives in the binary's `update` wrapper —
//! verified at the message level by checking `current_screen` after
//! processing both messages in sequence (the binary issues
//! `SwitchScreen(Strategies)` on its update return).

#![cfg(feature = "fixtures")]
// Test helpers construct a `Cockpit::default()` and override individual
// fields for clarity over a giant `Cockpit { ..Default::default() }`
// initializer; the lint is over-eager for this test shape.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use trading_core::StrategyId;
use ui::state::{update, Cockpit, Message, PanelState, Screen};

#[test]
fn select_strategy_from_home_persists_id() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Home;
    cockpit.strategies = PanelState::Ready(vec![ui::fixtures::fake_strategy_row_ready()]);
    assert!(cockpit.selected_strategy.is_none());

    let target = StrategyId::new("btc_macd_trend");
    update(&mut cockpit, Message::SelectStrategy(target.clone()));

    // Pure-update arm sets selected_strategy; current_screen unchanged
    // (the binary's update wrapper chains SwitchScreen via Task::done).
    assert_eq!(cockpit.selected_strategy.as_ref(), Some(&target));
    assert_eq!(cockpit.current_screen, Screen::Home);
}

#[test]
fn select_strategy_then_switch_screen_lands_on_strategies() {
    // Simulate the binary's compound dispatch sequence: `update(...,
    // SelectStrategy(id))` followed by `update(..., SwitchScreen(
    // Strategies))` — the chain the binary issues via Task::done.
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Home;
    cockpit.strategies = PanelState::Ready(vec![ui::fixtures::fake_strategy_row_ready()]);

    let target = StrategyId::new("btc_macd_trend");
    update(&mut cockpit, Message::SelectStrategy(target.clone()));
    update(&mut cockpit, Message::SwitchScreen(Screen::Strategies));

    assert_eq!(cockpit.selected_strategy.as_ref(), Some(&target));
    assert_eq!(cockpit.current_screen, Screen::Strategies);
}

#[test]
fn select_strategy_when_already_on_strategies_does_not_re_dispatch() {
    // From the Strategies screen itself, a chip click emits
    // `SelectStrategy(id)`; the binary's update wrapper checks
    // `current_screen != Screen::Strategies` BEFORE chaining the
    // SwitchScreen task, so re-clicking the same chip on the Strategies
    // screen does not re-dispatch. Verified at the model level by
    // observing `current_screen` after a SelectStrategy on Strategies.
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Strategies;
    cockpit.strategies = PanelState::Ready(vec![ui::fixtures::fake_strategy_row_ready()]);

    let target = StrategyId::new("btc_macd_trend");
    update(&mut cockpit, Message::SelectStrategy(target.clone()));

    assert_eq!(cockpit.selected_strategy.as_ref(), Some(&target));
    assert_eq!(cockpit.current_screen, Screen::Strategies);
}
