//! Test-only cockpit factory helpers — ui-test-harness-bootstrap v0.1.
//!
//! Hosts the **test-only** cockpit factory consumed by
//! [`iced_test::screenshot`] driven visual snapshots. Keeping this in a
//! sibling module (rather than expanding the production-reachable
//! `crate::fixtures` surface) isolates the hovered-marker fixture
//! (operator-locked Q9) from any future production-fixture refactor —
//! see [feature.md Design § Q6 resolution](../../../../spec/v1/ui-test-harness-bootstrap/feature.md#q1-q7-resolutions-architect-decide).
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
use crate::state::{Cockpit, Message, PanelState, Screen, update};
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
    cockpit.universe.clone_from(&universe);
    // Navigate the cockpit directly to the Charts screen — the visual
    // snapshot fires off this view, not Home.
    #[allow(deprecated)]
    // Screen::Charts is a backwards-compat alias for Screen::Lab; gallery fixtures keep the old name intentionally (T-D-1)
    {
        cockpit.current_screen = Screen::Charts;
    }
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
        .default_font(crate::theme::font::embedded())
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

/// lab-compare-equity-overlay T3 — render-layer harness for the **real Compare
/// screen body** (`screens::compare::view`), the production path that hydrates
/// the two-run equity overlay from each selected cell's `CachedCell`.
///
/// Unlike [`chart_overlay_program`] (which feeds the bare `chart::view` widget
/// directly), this renders `screens::compare::view(&cockpit, …)` — so it
/// exercises the ACTUAL screen code: `overlay_panel` reads
/// `compare_screen_state.overlay_selection`, resolves each slot's `CachedCell`,
/// builds `LabEquitySeries::from_samples` from the companion-CSV-hydrated
/// `equity_series_ts`, and calls `chart::view` with `equity`=slot 0 / `compare`
/// =[slot 1]. The matrix above the chart is omitted from the count by cropping
/// to the chart band; the bare body (no shell sidebar) keeps stray `ACCENT`
/// chrome out of the classifier.
#[must_use]
pub fn compare_screen_program(
    cockpit: Cockpit,
) -> iced::Application<
    impl iced::Program<State = CompareScreenApp, Message = Message, Theme = iced::Theme>,
> {
    let boot = move || {
        (
            CompareScreenApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(boot, CompareScreenApp::update, CompareScreenApp::view)
        .default_font(crate::theme::font::embedded())
        .title(CompareScreenApp::title)
        .theme(CompareScreenApp::theme)
}

/// Test app whose `view` is the bare Compare screen body (no shell chrome) so
/// the screenshot frames the matrix + overlay without the sidebar's `ACCENT`
/// active-item highlight leaking into the curve-pixel classifier.
pub struct CompareScreenApp {
    cockpit: Cockpit,
}

impl Default for CompareScreenApp {
    fn default() -> Self {
        Self {
            cockpit: Cockpit::new(),
        }
    }
}

impl CompareScreenApp {
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn update(&mut self, msg: Message) -> iced::Task<Message> {
        update(&mut self.cockpit, msg);
        iced::Task::none()
    }

    /// View — the real Compare screen body, full-bleed in a `PANEL`-background
    /// container.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let body = crate::screens::compare::view(&self.cockpit, ThemeMode::Dark);
        Container::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(crate::theme::color::PANEL.current(ThemeMode::Dark).into()),
                ..Default::default()
            })
            .into()
    }
}

/// lab-run-save-compare T7 — render-layer harness for the Lab/Compare equity
/// **overlay** widget (`widgets::chart`), the curve R5 diffs two persisted runs
/// on. Builds a one-screen iced `Program` whose `view` is exactly the
/// `chart::view(bars, …, equity, compare, …)` overlay the Lab screen renders,
/// so `iced_test::screenshot` drives the real `ChartProgram::draw` →
/// `tiny_skia` rasterization path (the same pixels the operator sees). The
/// caller supplies a primary `equity` series (drawn `ACCENT`) and optional
/// `compare` series (drawn `ACCENT_2..5`) — both hydrated from `lab-runs/`
/// reports via `equity_loader::load_equity` in the test — plus bars to anchor
/// the overlay's x-axis.
///
/// This is the project-law render-layer proof (MEMORY.md "verify UI at the
/// render layer"): model-Ready is necessary but not sufficient; this asserts
/// the polyline actually rasterizes.
#[must_use]
pub fn chart_overlay_program(
    bars: Vec<trading_core::Bar>,
    equity: Option<crate::lab::equity_loader::LabEquitySeries>,
    compare: Vec<crate::lab::equity_loader::LabEquitySeries>,
) -> iced::Application<
    impl iced::Program<State = ChartOverlayApp, Message = Message, Theme = iced::Theme>,
> {
    let scene = ChartOverlayScene {
        bars,
        equity,
        compare,
    };
    let boot = move || {
        (
            ChartOverlayApp {
                scene: scene.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(boot, ChartOverlayApp::update, ChartOverlayApp::view)
        .default_font(crate::theme::font::embedded())
        .title(ChartOverlayApp::title)
        .theme(ChartOverlayApp::theme)
}

/// Owned inputs for the [`chart_overlay_program`] render scene.
#[derive(Clone)]
struct ChartOverlayScene {
    bars: Vec<trading_core::Bar>,
    equity: Option<crate::lab::equity_loader::LabEquitySeries>,
    compare: Vec<crate::lab::equity_loader::LabEquitySeries>,
}

/// Test app whose `view` is the bare chart-overlay widget (no shell chrome) so
/// the screenshot crop frames the overlay canvas directly.
pub struct ChartOverlayApp {
    scene: ChartOverlayScene,
}

impl Default for ChartOverlayApp {
    fn default() -> Self {
        Self {
            scene: ChartOverlayScene {
                bars: Vec::new(),
                equity: None,
                compare: Vec::new(),
            },
        }
    }
}

impl ChartOverlayApp {
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn update(&mut self, _msg: Message) -> iced::Task<Message> {
        iced::Task::none()
    }

    /// View — the chart-overlay widget, full-bleed in a `PANEL`-background
    /// container so the crop window sees only the canvas.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let chart = crate::widgets::chart::view(
            self.scene.bars.clone(),
            Vec::new(),
            Vec::new(),
            None,
            self.scene.equity.clone(),
            self.scene.compare.clone(),
            ThemeMode::Dark,
        );
        Container::new(chart)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(crate::theme::color::PANEL.current(ThemeMode::Dark).into()),
                ..Default::default()
            })
            .into()
    }
}

// ── leaderboard render harness (advisor-leaderboard-screen v0.1.0) ────────────

/// Render-layer harness for the **real Leaderboard screen body**
/// (`screens::leaderboard::view`) — the production path that renders a
/// `backtest::bakeoff` result (mirrored into `BakeoffReportMirror`) as the
/// ranked table + recommendation + disclaimer.
///
/// Renders the BARE screen body (no shell sidebar) so the screenshot frames the
/// table + recommendation without the sidebar's `ACCENT` active-item highlight
/// leaking into the crowned-row `ACCENT` pixel classifier — exactly the
/// rationale `compare_screen_program` documents. The caller seeds the
/// `Cockpit`'s `leaderboard_screen_state` (e.g. via
/// `fixtures::fake_cockpit_leaderboard`).
#[must_use]
pub fn leaderboard_screen_program(
    cockpit: Cockpit,
) -> iced::Application<
    impl iced::Program<State = LeaderboardScreenApp, Message = Message, Theme = iced::Theme>,
> {
    let boot = move || {
        (
            LeaderboardScreenApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(
        boot,
        LeaderboardScreenApp::update,
        LeaderboardScreenApp::view,
    )
    .default_font(crate::theme::font::embedded())
    .title(LeaderboardScreenApp::title)
    .theme(LeaderboardScreenApp::theme)
}

/// Test app whose `view` is the bare Leaderboard screen body (no shell chrome)
/// so the screenshot frames the table + recommendation directly.
pub struct LeaderboardScreenApp {
    cockpit: Cockpit,
}

impl Default for LeaderboardScreenApp {
    fn default() -> Self {
        Self {
            cockpit: Cockpit::new(),
        }
    }
}

impl LeaderboardScreenApp {
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn update(&mut self, msg: Message) -> iced::Task<Message> {
        update(&mut self.cockpit, msg);
        iced::Task::none()
    }

    /// View — the real Leaderboard screen body, full-bleed on a `CANVAS`
    /// background (the shell's outer background) so the table's `PANEL` surfaces
    /// read against the same chrome the operator sees.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let body = crate::screens::leaderboard::view(&self.cockpit, ThemeMode::Dark);
        Container::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(crate::theme::color::CANVAS.current(ThemeMode::Dark).into()),
                ..Default::default()
            })
            .into()
    }
}

// ── Tune render harness (advisor-param-tuning, ADR-0069 T9) ───────────────────

/// Render-layer harness for the **real Tune screen body**
/// (`screens::tune::view`) — the gate-tied hyperparameter sweep editor: the
/// range form (family picker + SMA axes + presets + grid readout) + the result
/// grid (one row per swept config: params · verdict · return · Sharpe p5/p50/p95
/// · P(loss) · P(Sharpe>1) · Max-DD p95) with FRAGILE prominently flagged +
/// promotion-blocked.
///
/// Renders the BARE screen body (no shell sidebar) so the screenshot frames the
/// form + grid without the sidebar's `ACCENT` active-item highlight leaking into
/// the pixel classifiers — exactly the rationale `leaderboard_screen_program`
/// documents. The caller seeds the `Cockpit`'s `tune_screen_state` (e.g. via
/// `fixtures::fake_cockpit_tune`).
#[must_use]
pub fn tune_screen_program(
    cockpit: Cockpit,
) -> iced::Application<
    impl iced::Program<State = TuneScreenApp, Message = Message, Theme = iced::Theme>,
> {
    let boot = move || {
        (
            TuneScreenApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(boot, TuneScreenApp::update, TuneScreenApp::view)
        .default_font(crate::theme::font::embedded())
        .title(TuneScreenApp::title)
        .theme(TuneScreenApp::theme)
}

/// Test app whose `view` is the bare Tune screen body (no shell chrome) so the
/// screenshot frames the form + result grid directly.
pub struct TuneScreenApp {
    cockpit: Cockpit,
}

impl Default for TuneScreenApp {
    fn default() -> Self {
        Self {
            cockpit: Cockpit::new(),
        }
    }
}

impl TuneScreenApp {
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn update(&mut self, msg: Message) -> iced::Task<Message> {
        update(&mut self.cockpit, msg);
        iced::Task::none()
    }

    /// View — the real Tune screen body, full-bleed on a `CANVAS` background (the
    /// shell's outer background) so the form + grid's `PANEL` surfaces read
    /// against the same chrome the operator sees.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let body = crate::screens::tune::view(&self.cockpit, ThemeMode::Dark);
        Container::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(crate::theme::color::CANVAS.current(ThemeMode::Dark).into()),
                ..Default::default()
            })
            .into()
    }
}

// ── forward-plan render harness (advisor-forward-plan v0.1.0, F6) ─────────────

/// Render-layer harness for the **real Forward-plan screen body**
/// (`screens::forward_plan::view`) — the production path that renders a
/// `ForwardPlanView` (mirrored from the `core`-typed
/// `agent::config::ForwardPlan`) as the conditional plan: the dated stance
/// badge + the IF/THEN standing rules + the €200 projected sizing + the
/// horizon + the not-a-prediction / not-advice disclaimers.
///
/// Renders the BARE screen body (no shell sidebar) so the screenshot frames the
/// plan without the sidebar's `ACCENT` active-item highlight leaking into the
/// pixel classifiers — exactly the rationale `leaderboard_screen_program`
/// documents. The caller seeds the `Cockpit`'s `forward_plan_screen_state`
/// (e.g. via `fixtures::fake_cockpit_forward_plan`).
#[must_use]
pub fn forward_plan_screen_program(
    cockpit: Cockpit,
) -> iced::Application<
    impl iced::Program<State = ForwardPlanScreenApp, Message = Message, Theme = iced::Theme>,
> {
    let boot = move || {
        (
            ForwardPlanScreenApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(
        boot,
        ForwardPlanScreenApp::update,
        ForwardPlanScreenApp::view,
    )
    .default_font(crate::theme::font::embedded())
    .title(ForwardPlanScreenApp::title)
    .theme(ForwardPlanScreenApp::theme)
}

/// Test app whose `view` is the bare Forward-plan screen body (no shell chrome)
/// so the screenshot frames the plan surface directly.
pub struct ForwardPlanScreenApp {
    cockpit: Cockpit,
}

impl Default for ForwardPlanScreenApp {
    fn default() -> Self {
        Self {
            cockpit: Cockpit::new(),
        }
    }
}

impl ForwardPlanScreenApp {
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn update(&mut self, msg: Message) -> iced::Task<Message> {
        update(&mut self.cockpit, msg);
        iced::Task::none()
    }

    /// View — the real Forward-plan screen body, full-bleed on a `CANVAS`
    /// background (the shell's outer background) so the plan's `PANEL` surfaces
    /// read against the same chrome the operator sees.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let body = crate::screens::forward_plan::view(&self.cockpit, ThemeMode::Dark);
        Container::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(crate::theme::color::CANVAS.current(ThemeMode::Dark).into()),
                ..Default::default()
            })
            .into()
    }
}

// ── source_toggle render harness (simple-strategies-realdata T-B1 / AC7) ──────

/// Build a render program whose `view` is the bare Lab `source_toggle` widget,
/// for the three-way-toggle render proof (simple-strategies-realdata T-B1).
///
/// The toggle is rendered full-bleed on a `PANEL` background so the screenshot
/// crop frames only the chip row — the active chip's `ACCENT` background band
/// is then countable / locatable (mirrors `chart_overlay_program`'s pattern).
/// The Binance chip is itself `#[cfg(feature = "binance")]`, so a no-`binance`
/// build renders TWO chips and a `binance` build renders THREE (AC8).
#[must_use]
pub fn source_toggle_program(
    current: crate::lab::state::LabDataSource,
) -> iced::Application<
    impl iced::Program<State = SourceToggleApp, Message = Message, Theme = iced::Theme>,
> {
    let boot = move || (SourceToggleApp { current }, iced::Task::none());
    iced::application(boot, SourceToggleApp::update, SourceToggleApp::view)
        .default_font(crate::theme::font::embedded())
        .title(SourceToggleApp::title)
        .theme(SourceToggleApp::theme)
}

/// Test app whose `view` is the bare `source_toggle` widget (no shell chrome).
pub struct SourceToggleApp {
    current: crate::lab::state::LabDataSource,
}

impl Default for SourceToggleApp {
    fn default() -> Self {
        Self {
            current: crate::lab::state::LabDataSource::Synthetic,
        }
    }
}

impl SourceToggleApp {
    #[must_use]
    pub fn title(&self) -> String {
        crate::strings::APP_TITLE.to_string()
    }

    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    pub fn update(&mut self, _msg: Message) -> iced::Task<Message> {
        iced::Task::none()
    }

    /// View — the source-toggle chip row, top-left-anchored in a `PANEL`-
    /// background container so the crop window sees only the chips.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        use iced::Length;
        use iced::widget::{Container, container};
        let toggle = crate::widgets::source_toggle::view(self.current, ThemeMode::Dark);
        Container::new(toggle)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(crate::theme::color::PANEL.current(ThemeMode::Dark).into()),
                ..Default::default()
            })
            .into()
    }
}

/// ui-quality-gate-overhaul M1-C — pub(crate) widget accessors lifted
/// up to the always-compiled `pub` surface so the
/// `tests/layout_invariants.rs` proptest can fuzz the F1-fix widget
/// (`strategies::id_cell`) directly. Each helper is a thin wrapper
/// that constructs the underlying widget — production builds never
/// reach these accessors (they live in `test_support`, the same
/// always-compiled tests-only module that already houses
/// `program_from_cockpit`). See
/// `spec/v1/ui-quality-gate-overhaul/feature.md ## Q4` for the 6-widget
/// scope.
pub mod widgets_for_test {
    use trading_core::StrategyId;

    use crate::state::Message;

    /// The strategies-table id-cell — the canonical F1 case
    /// (`Length::Fill` collapses to 0 inside a Table cell). The
    /// proptest at `tests/layout_invariants.rs` fuzzes the inputs
    /// and asserts the resulting `Widget::layout` Node tree has no
    /// zero-dim Node (per architect Q4 + M1-C-2 acceptance
    /// criteria).
    #[must_use]
    pub fn strategies_id_cell<'a>(
        id: StrategyId,
        label: String,
        is_active: bool,
    ) -> iced::Element<'a, Message> {
        crate::widgets::strategies::id_cell(id, label, is_active)
    }
}

/// bug-log #205 — the ONE live-corpus oracle the `reports` /
/// `lab` lib tests and `tests/reports_populated_curve_render.rs` share: which
/// files under `evidence/` exist according to git, independent of the
/// discovery code under test. Always compiled for the same reason as the rest
/// of this module (integration tests only see `pub` items); nothing in a
/// production path calls it.
///
/// **What it lists.** `git ls-files -z --cached --others -- evidence`: the
/// index (so a tracked report deleted from disk is still listed, and a test
/// comparing discovery against it fails — #66) PLUS every untracked file on
/// disk, ignored ones included (no `--exclude-standard`), so a report a
/// backtest-CLI run writes under `evidence/` on a dev box is in the oracle as
/// well as in discovery. The discovery-coverage tests (the `reports` loader's
/// D2/D3/D5 and the boot auto-select render test) therefore cannot turn red
/// for a reason outside their subject (#189). The Lab's E3 is strict the other
/// way on purpose: it requires exactly ONE git-listed momentum 2024-h1 report,
/// because a second one can change which report the Lab resolves.
pub mod evidence_oracle {
    use std::path::Path;
    use std::process::Command;

    /// A `git` command for the LIVE oracle on the real workspace: `current_dir`
    /// = `workspace`, environment NOT scrubbed — under a hook the exported
    /// `GIT_INDEX_FILE` is the index being committed, which is the one to read.
    /// A hermetic fixture builds its own `Command` with every `GIT_*` removed.
    #[must_use]
    pub fn live_git(workspace: &Path) -> Command {
        let mut git = Command::new("git");
        git.current_dir(workspace);
        git
    }

    /// Run `git ls-files -z --cached --others -- evidence` through `git` (a
    /// `Command::new("git")` with `current_dir` set to the workspace, see
    /// [`live_git`]) and split every listed path into its `/`-separated
    /// components (git prints `/` on every OS).
    ///
    /// # Errors
    ///
    /// A `String` naming the failure when `git` cannot run, exits non-zero,
    /// or prints a non-UTF-8 path. Callers panic on it: the oracle never
    /// skips.
    pub fn git_listed_files(git: &mut Command) -> Result<Vec<Vec<String>>, String> {
        let out = git
            .args(["ls-files", "-z", "--cached", "--others", "--", "evidence"])
            .output()
            .map_err(|e| format!("git ls-files could not run: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git ls-files exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let text = String::from_utf8(out.stdout)
            .map_err(|e| format!("git printed a non-UTF-8 path under evidence/: {e}"))?;
        Ok(text
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.split('/').map(str::to_string).collect())
            .collect())
    }

    /// The `backtest-*.md` name rule, restated here (not borrowed from the
    /// code under test).
    #[must_use]
    pub fn is_backtest_name(name: &str) -> bool {
        // Writers emit a lowercase `.md` extension.
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let md = name.ends_with(".md");
        name.starts_with("backtest-") && md
    }

    /// The `equity-*.csv` companion name rule (lowercase extension).
    #[must_use]
    pub fn is_equity_csv_name(name: &str) -> bool {
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let csv = name.ends_with(".csv");
        name.starts_with("equity-") && csv
    }

    /// An in-contract report path, by components: `[evidence, <slug>, reports,
    /// backtest-*.md]` (container depth 0) or `[evidence, <container>, <slug>,
    /// reports, backtest-*.md]` (container depth 1), with no middle component
    /// named `reports`. Stated independently of `reports::layout`.
    #[must_use]
    pub fn is_contract_report(c: &[String]) -> bool {
        let n = c.len();
        (n == 4 || n == 5)
            && c[0] == "evidence"
            && c[n - 2] == "reports"
            && is_backtest_name(&c[n - 1])
            && c[1..n - 2].iter().all(|m| m != "reports")
    }

    /// The in-contract reports among `files` whose stem-matched
    /// `reports/artifacts/<stem>/equity-*.csv` is also among `files` — the
    /// companion rule, derived from the listing alone. Sorted.
    #[must_use]
    pub fn companion_reports(files: &[Vec<String>]) -> Vec<Vec<String>> {
        let companion_dirs: std::collections::HashSet<&[String]> = files
            .iter()
            .filter(|c| {
                let n = c.len();
                n >= 4 && c[n - 3] == "artifacts" && is_equity_csv_name(&c[n - 1])
            })
            .map(|c| &c[..c.len() - 1])
            .collect();
        let mut out: Vec<Vec<String>> = files
            .iter()
            .filter(|c| is_contract_report(c))
            .filter(|r| {
                let n = r.len();
                let stem = r[n - 1].strip_suffix(".md").unwrap_or(&r[n - 1]);
                let mut d = r[..n - 1].to_vec();
                d.push("artifacts".to_string());
                d.push(stem.to_string());
                companion_dirs.contains(d.as_slice())
            })
            .cloned()
            .collect();
        out.sort();
        out
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    mod tests {
        use super::*;

        fn workspace_root() -> std::path::PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
        }

        /// A `git` for the HERMETIC fixture repo at `dir`: every inherited
        /// `GIT_*` variable removed (a hook's absolute `GIT_INDEX_FILE` would
        /// otherwise point a fixture at the real index), global and system
        /// config replaced by an empty file, identity and dates pinned.
        fn fixture_git(dir: &Path, empty_config: &Path) -> Command {
            let mut git = Command::new("git");
            git.current_dir(dir);
            for (k, _) in std::env::vars_os() {
                if k.to_string_lossy().starts_with("GIT_") {
                    git.env_remove(&k);
                }
            }
            git.env("GIT_CONFIG_GLOBAL", empty_config)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "fixture")
                .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
                .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
                .env("GIT_COMMITTER_NAME", "fixture")
                .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
                .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z");
            git
        }

        fn run(git: &mut Command, args: &[&str]) {
            let out = git.args(args).output().unwrap();
            assert!(
                out.status.success(),
                "fixture git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }

        /// OR1 — HERMETIC, two-sided: in a fixture repo the oracle lists a
        /// committed report, a committed report deleted from disk (#66), an
        /// untracked report and an ignored report (#189), and nothing outside
        /// `evidence/`. Expected values come from the fixture table below.
        /// Dropping `--others` (probe P12) loses the untracked and ignored rows.
        #[test]
        fn git_listed_files_lists_committed_deleted_untracked_and_ignored() {
            let tmp = tempfile::tempdir().unwrap();
            let repo = tmp.path().join("repo");
            std::fs::create_dir_all(&repo).unwrap();
            let empty_config = tmp.path().join("empty.gitconfig");
            std::fs::write(&empty_config, "").unwrap();
            let rd = "evidence/v1/s/reports";
            // (path, committed, deleted from disk after the commit, listed)
            let table: [(String, bool, bool, bool); 6] = [
                (
                    format!("{rd}/backtest-20260101-000000-committed.md"),
                    true,
                    false,
                    true,
                ),
                (
                    format!("{rd}/backtest-20260102-000000-deleted.md"),
                    true,
                    true,
                    true,
                ),
                (
                    format!("{rd}/backtest-20260103-000000-untracked.md"),
                    false,
                    false,
                    true,
                ),
                (
                    format!("{rd}/backtest-20260104-000000-ignored.md"),
                    false,
                    false,
                    true,
                ),
                (
                    "outside/backtest-20260105-000000-x.md".to_string(),
                    false,
                    false,
                    false,
                ),
                (".gitignore".to_string(), true, false, false),
            ];
            for (path, _, _, _) in &table {
                let p = repo.join(path);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, "ts\n").unwrap();
            }
            std::fs::write(
                repo.join(".gitignore"),
                format!("{rd}/backtest-20260104-000000-ignored.md\n"),
            )
            .unwrap();
            run(&mut fixture_git(&repo, &empty_config), &["init", "-q"]);
            let mut add = vec!["add", "--"];
            add.extend(table.iter().filter(|r| r.1).map(|r| r.0.as_str()));
            run(&mut fixture_git(&repo, &empty_config), &add);
            run(
                &mut fixture_git(&repo, &empty_config),
                &[
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "-q",
                    "-m",
                    "fixture",
                ],
            );
            for (path, _, deleted, _) in &table {
                if *deleted {
                    std::fs::remove_file(repo.join(path)).unwrap();
                }
            }

            let mut want: Vec<Vec<String>> = table
                .iter()
                .filter(|r| r.3)
                .map(|r| r.0.split('/').map(str::to_string).collect())
                .collect();
            want.sort();
            assert!(
                table.iter().any(|r| !r.1 && r.3) && table.iter().any(|r| r.2 && r.3),
                "the table must hold an untracked AND a deleted listed row, else OR1 pins nothing"
            );
            let mut got = git_listed_files(&mut fixture_git(&repo, &empty_config)).unwrap();
            got.sort();
            assert_eq!(
                got, want,
                "the oracle must list committed, deleted-but-tracked, untracked and ignored \
                 reports under evidence/, and nothing outside it (#205)"
            );
        }

        /// OR2 — the oracle is not partial (falsifier O5b): every in-contract
        /// report in the index, read HERE with `git ls-files -z --cached` and
        /// NO pathspec, then filtered to `evidence/` in Rust (so narrowing the
        /// oracle's pathspec cannot narrow this side), is in the oracle's
        /// listing. Both sides read the same index, so staging a move or
        /// deletion of a committed report keeps them in step (#189). Only the
        /// `evidence/` entries are decoded, so a non-UTF-8 file name elsewhere
        /// in the repo cannot fail it (#189). Live and read-only, so the
        /// environment is not scrubbed.
        #[test]
        fn git_listed_files_contains_every_indexed_contract_report() {
            let ws = workspace_root();
            let out = live_git(&ws)
                .args(["ls-files", "-z", "--cached"])
                .output()
                .unwrap_or_else(|e| panic!("git ls-files must run for OR2: {e}"));
            assert!(
                out.status.success(),
                "git ls-files failed for OR2: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let indexed: Vec<Vec<String>> = out
                .stdout
                .split(|b| *b == 0)
                .filter(|s| s.starts_with(b"evidence/"))
                .map(|s| {
                    std::str::from_utf8(s)
                        .unwrap_or_else(|e| panic!("an indexed evidence/ path must be UTF-8: {e}"))
                        .split('/')
                        .map(str::to_string)
                        .collect::<Vec<String>>()
                })
                .filter(|c| is_contract_report(c))
                .collect();
            assert!(
                indexed.iter().any(|c| c.len() == 4) && indexed.iter().any(|c| c.len() == 5),
                "the index must hold contract reports at container depth 0 AND 1, else OR2 pins \
                 nothing"
            );
            let listed: std::collections::HashSet<Vec<String>> =
                git_listed_files(&mut live_git(&ws))
                    .unwrap()
                    .into_iter()
                    .collect();
            let missing: Vec<String> = indexed
                .iter()
                .filter(|c| !listed.contains(*c))
                .map(|c| c.join("/"))
                .collect();
            assert!(
                missing.is_empty(),
                "{} of {} indexed contract reports are missing from the evidence oracle (a \
                 partial oracle lets a discovery gap pass); first: {:?}",
                missing.len(),
                indexed.len(),
                missing.first()
            );
        }
    }
}
