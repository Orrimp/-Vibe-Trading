//! Leaderboard "inspect this row's strategy in the Lab" — behavior relay test.
//!
//! ## The feature this pins (advisor-leaderboard-inspect-in-lab)
//!
//! The Leaderboard ranks strategies in a table; the operator wants to click any
//! data row to inspect how that strategy's trades would go, shown with the same
//! buy/sell overlay as the Lab. The chosen surface is a JUMP to the Lab screen,
//! preseeded — NOT an inline panel. Clicking a row fires
//! `Message::InspectStrategyFromLeaderboard { strategy, coin, lookback }`, handled
//! ENTIRELY in `ui::state::update` (pure — the `OpenLabFromCompare` /
//! `OpenStrategyInLab` precedent; no bin-layer `Task` chain).
//!
//! The handler must:
//! - navigate to `Screen::Lab`,
//! - preselect the row's strategy in `lab_state.strategy` + `selected_strategy`,
//! - load the leaderboard's CHOSEN coin as the Lab pair `(Binance, coin)`,
//!   OVERRIDING any prior Lab pair (the just-chosen coin is authoritative),
//! - load the leaderboard's CHOSEN lookback as the Lab `range`, and
//! - clear the stale run-report mirrors (the tuple changed) so the Lab does not
//!   paint a previous pick's equity curve against the new one.
//!
//! These tests drive `ui::state::update` directly and assert the post-update
//! model — they go RED if the handler stops doing any of the above (e.g. revert
//! the pair/range override, or drop the message). Reverting the handler body to
//! the bare `OpenStrategyInLab` (cold-start-only pair seed, no range load) FAILS
//! `inspect_overrides_existing_pair_with_chosen_coin` and `inspect_loads_lookback_range`.

#![cfg(feature = "fixtures")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::{DateRange, Preset};
use ui::leaderboard::LeaderboardLookback;
use ui::state::{Cockpit, Message, Screen, update};

/// Build the inspect message for a row, mirroring exactly what
/// `screens::leaderboard::data_row` constructs on a row click.
fn inspect(strategy: &str, coin: &str, lookback: LeaderboardLookback) -> Message {
    Message::InspectStrategyFromLeaderboard {
        strategy: StrategyId::new(strategy),
        coin: Symbol::new(coin),
        lookback,
    }
}

/// Core proof: from the Leaderboard screen, inspecting a row navigates to the
/// Lab AND preselects that row's strategy. This is the exact path a row click
/// drives — the rows were inert (no `.on_press`) before this feature.
#[test]
fn inspect_navigates_and_preselects_strategy() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;

    // A NON-SMA pick — the faithful-run case the feature exists to prove
    // (the Lab dispatches `v0.5.macd` to the real ComposedStrategy).
    update(
        &mut cockpit,
        inspect("v0.5.macd", "ETHUSDT", LeaderboardLookback::H1_2024),
    );

    assert_eq!(
        cockpit.current_screen,
        Screen::Lab,
        "inspecting a leaderboard row must switch the active screen to the Lab"
    );
    let expected = StrategyId::new("v0.5.macd");
    assert_eq!(
        cockpit.lab_state.strategy.as_ref(),
        Some(&expected),
        "inspect must preselect the row's strategy in lab_state.strategy"
    );
    assert_eq!(
        cockpit.selected_strategy.as_ref(),
        Some(&expected),
        "selected_strategy stays in sync with the inspected strategy"
    );
}

/// The chosen coin loads as the Lab pair `(Binance, coin)` — and OVERRIDES any
/// pre-existing Lab pair. The operator just chose this coin on the leaderboard,
/// so it is authoritative (this is the key difference from `OpenStrategyInLab`,
/// which only seeds a default pair when none is set).
#[test]
fn inspect_overrides_existing_pair_with_chosen_coin() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;
    // A stale prior Lab pair that must NOT survive — the leaderboard coin wins.
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));

    update(
        &mut cockpit,
        inspect("v0.5.rsi", "SOLUSDT", LeaderboardLookback::H2_2024),
    );

    assert_eq!(
        cockpit.lab_state.pair.as_ref(),
        Some(&(Venue::Binance, Symbol::new("SOLUSDT"))),
        "the leaderboard's chosen coin must override any prior Lab pair \
         (mapped to the single-venue (Binance, coin) pair)"
    );
    // selected_symbol mirrors the new pair (cross-link parity).
    assert_eq!(
        cockpit.selected_symbol.as_ref(),
        Some(&(Venue::Binance, Symbol::new("SOLUSDT"))),
        "selected_symbol mirrors the inspected coin"
    );
}

/// The chosen lookback loads as the Lab `range`. The two fixed corpus presets
/// (H1/H2 2024) map to the Lab's matching named `Preset` so the Lab's date-range
/// picker shows the right chip and the run uses the byte-identical window the
/// bake-off scored.
#[test]
fn inspect_loads_lookback_range() {
    // H1_2024 → the Lab's H1_2024 preset.
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;
    update(
        &mut cockpit,
        inspect("v0.sma", "BTCUSDT", LeaderboardLookback::H1_2024),
    );
    assert_eq!(
        cockpit.lab_state.range,
        DateRange::Preset(Preset::H1_2024),
        "H1_2024 lookback must load the Lab's H1_2024 preset range"
    );

    // H2_2024 → the Lab's H2_2024 preset.
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;
    update(
        &mut cockpit,
        inspect("v0.sma", "BTCUSDT", LeaderboardLookback::H2_2024),
    );
    assert_eq!(
        cockpit.lab_state.range,
        DateRange::Preset(Preset::H2_2024),
        "H2_2024 lookback must load the Lab's H2_2024 preset range"
    );
}

/// A relative lookback (e.g. 3 months) maps to a `Custom` ISO-date window
/// (`now - N days` → `now`). The Lab run path accepts the date-only `Custom`
/// form, so a relative window runs faithfully. We assert the SHAPE (Custom with
/// two non-empty `YYYY-MM-DD` strings) rather than exact dates (wall-clock now).
#[test]
fn inspect_relative_lookback_loads_custom_iso_range() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;
    update(
        &mut cockpit,
        inspect("v0.5.bbands", "BTCUSDT", LeaderboardLookback::ThreeMonths),
    );
    match &cockpit.lab_state.range {
        DateRange::Custom { start_raw, end_raw } => {
            // YYYY-MM-DD is 10 chars; both must be present and ordered.
            assert_eq!(start_raw.len(), 10, "start_raw must be a YYYY-MM-DD date");
            assert_eq!(end_raw.len(), 10, "end_raw must be a YYYY-MM-DD date");
            assert!(
                start_raw.as_str() < end_raw.as_str(),
                "the 3-month window's start ({start_raw}) must precede its end ({end_raw})"
            );
        }
        other => panic!("a relative lookback must load a Custom range, got {other:?}"),
    }
}

/// Regression guard for the tuple-change trap: inspecting a row must clear the
/// stale run-report mirrors (parity with `LabSelectPrimaryStrategy` /
/// `OpenStrategyInLab`) so the Lab does not render a previous strategy's equity
/// curve against the freshly-inspected pick.
#[test]
fn inspect_clears_stale_run_reports() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;

    update(
        &mut cockpit,
        inspect("v0.5.macd", "ETHUSDT", LeaderboardLookback::H1_2024),
    );

    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "inspect must clear last_run_report (tuple changed)"
    );
    assert!(
        cockpit.lab_state.prev_run_report.is_none(),
        "inspect must clear prev_run_report (tuple changed)"
    );
}

/// The Lab run gate (`screens::lab` Run button) requires BOTH a strategy AND a
/// pair. After an inspect, both halves are populated straight away — the Lab
/// opens runnable on the inspected coin (the operator presses Run to execute).
#[test]
fn inspect_lands_in_a_runnable_lab() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Leaderboard;

    update(
        &mut cockpit,
        inspect("v0.buyhold", "BTCUSDT", LeaderboardLookback::H1_2024),
    );

    assert!(
        cockpit.lab_state.strategy.is_some() && cockpit.lab_state.pair.is_some(),
        "after an inspect both halves of the Lab run gate (strategy + pair) are set"
    );
}
