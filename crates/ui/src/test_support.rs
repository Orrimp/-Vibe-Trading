//! Test-only cockpit factory helpers — ui-test-harness-bootstrap v0.1.
//!
//! Hosts the **test-only** cockpit factory consumed by
//! [`iced_test::screenshot`] driven visual snapshots. Keeping this in a
//! sibling module (rather than expanding the production-reachable
//! `crate::fixtures` surface) isolates the hovered-marker fixture
//! (operator-locked Q9) from any future production-fixture refactor —
//! see [feature.md Design § Q6 resolution](../../../../spec/ui-test-harness-bootstrap/feature.md#q1-q7-resolutions-architect-decide).
//!
//! ## Why always-compiled?
//!
//! Integration tests in `crates/ui/tests/*.rs` only see the library's
//! public API; they cannot import a `#[cfg(test)]`-only item. The
//! `crate::fixtures` module is also unconditionally compiled (see
//! `lib.rs`) for the same reason. H5 (in feature.md) requires the
//! factory to compile under default features — no `--features fixtures`
//! /  `--features live` opt-in — so the integration tests can run via
//! the standard `cargo test -p ui --tests` invocation.
//!
//! Production builds incur a negligible compile cost: the factory is one
//! function that delegates to `crate::fixtures` builders already in the
//! default build.

use trading_core::{Symbol, Venue};

use crate::fixtures::{
    fake_cockpit_v15a_pairs_steady_state, seed_for, synthetic_candles, synthetic_fills_for,
};
use crate::state::{update, Cockpit, Message, PanelState, Screen};
use crate::theme::ThemeMode;

/// Construct a Charts-screen seeded `Cockpit` mirroring the
/// `cockpit` binary's fixtures boot (see `src/bin/cockpit.rs:132`) but
/// navigated directly to `Screen::Charts` with the chart markers
/// pre-loaded for BTCUSDT.
///
/// This is the base scene shared by every visual snapshot — individual
/// tests may further augment the returned `Cockpit` (e.g. seeding
/// `chart_tooltip` for the Q9 hovered-marker fixture).
#[must_use]
pub fn charts_screen_cockpit() -> Cockpit {
    let mut cockpit = fake_cockpit_v15a_pairs_steady_state();

    let universe: Vec<(Venue, Symbol)> = vec![
        (Venue::Binance, Symbol::new("BTCUSDT")),
        (Venue::Binance, Symbol::new("ETHUSDT")),
        (Venue::Binance, Symbol::new("SOLUSDT")),
    ];
    cockpit.universe = universe.clone();
    // Navigate the cockpit directly to the Charts screen — the visual
    // snapshot fires off this view, not Home.
    cockpit.current_screen = Screen::Charts;
    let default_pair = universe[0].clone();
    cockpit.selected_symbol = Some(default_pair.clone());

    // Seed the chart buffer with 60 deterministic synthetic bars for
    // every symbol (same seed function the cockpit bin uses, so the
    // visual snapshot's price series matches what an operator sees
    // when booting the fixtures cockpit).
    for (venue, symbol) in &universe {
        let seed = seed_for(*venue, symbol);
        for bar in synthetic_candles(seed, *venue, symbol.clone(), 60) {
            update(&mut cockpit, Message::BarReceived(bar));
        }
    }
    // Pre-seed chart markers for the active symbol so the Charts
    // screen renders fills on first paint.
    cockpit.chart_markers =
        PanelState::Ready(synthetic_fills_for(default_pair.0, &default_pair.1, 4));

    cockpit
}

/// Cockpit-shaped iced `Application` instance ready for
/// [`iced_test::screenshot`]. Returns an `impl iced::Program` so the
/// caller can pass `&program` straight into the free function.
///
/// The boot closure captures the supplied `Cockpit` by clone so this
/// helper is safe to call once per `#[test]` — different tests can
/// seed different `Cockpit` states (e.g. with vs. without
/// `chart_tooltip`).
#[must_use]
pub fn program_from_cockpit(
    cockpit: Cockpit,
) -> iced::Application<impl iced::Program<State = TestApp, Message = Message, Theme = iced::Theme>>
{
    // `boot` captures the seeded cockpit by clone — the iced runtime
    // calls `boot` once on startup, so the clone happens at most once
    // per `screenshot` invocation. No `unwrap` / `panic!` on the path.
    let boot = move || {
        (
            TestApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };

    iced::application(boot, TestApp::update, TestApp::view)
        .title(TestApp::title)
        .theme(TestApp::theme)
}

/// Test wrapper mirroring the production `App` struct in
/// `src/bin/cockpit.rs`. Carries the seeded `Cockpit` and routes
/// `update` / `view` to the same code paths the real binary uses.
///
/// `Default` is required by `iced::Application`'s `State: 'static`
/// bound but is intentionally unreachable — the visual-snapshot
/// `boot` closure always supplies a fixture-seeded value.
pub struct TestApp {
    cockpit: Cockpit,
}

impl Default for TestApp {
    fn default() -> Self {
        Self {
            cockpit: charts_screen_cockpit(),
        }
    }
}

impl TestApp {
    /// Title — fed to `iced::application(...).title(...)`.
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    /// Theme — `Dark` to match the production cockpit. The visual
    /// baselines under `crates/ui/tests/visual-baselines/` are
    /// committed at Dark; switching to Light would require a separate
    /// baseline set.
    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    /// Update — delegates to the library's `state::update`. The
    /// snapshot path doesn't run `update` (single render at
    /// `Duration::ZERO`), but iced's `Program` trait requires it.
    pub fn update(&mut self, msg: Message) -> iced::Task<Message> {
        update(&mut self.cockpit, msg);
        iced::Task::none()
    }

    /// View — routes to the shell (sidebar + screen body + status
    /// bar) the same way the production cockpit binary does. Dark
    /// theme is locked in `theme()`.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        crate::shell::view(&self.cockpit, ThemeMode::Dark)
    }
}
