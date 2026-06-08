//! Cockpit binary — fixtures-only ops view.
//!
//! Wires the `ui` crate panels into an iced `Application` using the
//! functional builder API (`iced::application` / `iced::run`).
//!
//! ## Why `--features live` no longer applies here (T908)
//!
//! Pre-T908, this binary accepted a `--features live` build that
//! constructed an *empty* `Arc<EventBus>` — every panel stayed in
//! `Loading` forever because nothing was publishing. That dead-end was
//! the exact failure mode the unified [`cockpit_live`] binary exists
//! to delete. Per
//! [`spec/features/live-cockpit-unified.md` Q7](../../../../spec/features/live-cockpit-unified.md#q7--keep-two-binary-path-alive)
//! the standalone `cockpit` binary is now fixtures-only; an explicit
//! [`compile_error!`] below fires if anyone tries
//! `cargo run --bin cockpit --features live`, redirecting them to
//! `cargo run --bin cockpit_live --features live` (the unified binary
//! that actually wires the bus + kill switch + audit ledger end-to-end).
//!
//! Feature flag still supported here:
//! - `fixtures` — boot against deterministic in-memory data from
//!   `ui::fixtures`; no `agent` process required. Best for layout smoke
//!   tests and demo runs.
//!
//! [`cockpit_live`]: ../../bin/cockpit_live/index.html

// T908 — deprecation shim. The standalone `cockpit` bin no longer
// honors `--features live` as the live entry point; the new home for
// live wiring is the `cockpit_live` bin (see Cargo.toml
// `[[bin]] cockpit_live`, `required-features = ["live"]`).
//
// Two layers of gating defend against the dead empty-bus path:
//
// 1. **Cargo-level**: this bin declares `required-features =
//    ["fixtures"]` in `Cargo.toml`, so `cargo run --bin cockpit
//    --features live` fails at resolve time with "target requires the
//    features: fixtures" — pointing the operator at the right call.
// 2. **Source-level (this `compile_error!`)**: fires only when
//    `live` is requested *without* `fixtures`. That combination is
//    impossible to hit through the cargo gate above, but if a future
//    edit ever drops the `required-features` line, this shim still
//    routes the operator to `cockpit_live` with a clear message
//    instead of silently re-introducing the empty-bus dead end.
//
// Workspace-wide `cargo build --workspace --all-features` (which
// activates both `live` and `fixtures`) compiles cleanly because the
// `not(feature = "fixtures")` half of the gate is false.
#[cfg(all(feature = "live", not(feature = "fixtures")))]
compile_error!(
    "The `cargo run --bin cockpit --features live` path was retired in \
     live-cockpit-unified (T908). Use `cargo run --bin cockpit_live --features live` \
     for the unified agent+cockpit binary; the headless agent still runs via \
     `cargo run --bin trading`. The standalone `cockpit` bin is fixtures-only."
);

use iced::Element;

use trading_core::{Symbol, Venue};
use ui::fixtures::{seed_for, synthetic_candles, synthetic_fills_for};
use ui::shell;
use ui::state::{Cockpit, Message, PanelState, Screen};
use ui::strings::APP_TITLE;
use ui::theme::ThemeMode;
use ui::widgets::journal_transaction_modal;

// ── 1 Hz server-time recipe (T1509) ──────────────────────────────────────────
//
// A custom iced `Recipe` that emits `Message::ServerTimeTick` every second.
// Implemented with a background OS thread + `std::sync::mpsc` so no tokio
// dep is needed in the `fixtures`-only bin. The thread sleeps for 1 s and
// sends the current `Timestamp`; the stream yields it and loops.

use iced::advanced::subscription::{EventStream, Hasher, Recipe};
use iced::futures;

struct ServerTimeRecipe;

impl Recipe for ServerTimeRecipe {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        use std::any::TypeId;
        use std::hash::Hash;
        TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: EventStream,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        let (tx, rx) = std::sync::mpsc::channel::<Message>();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if tx
                    .send(Message::ServerTimeTick(trading_core::Timestamp::now()))
                    .is_err()
                {
                    break;
                }
            }
        });
        // Convert the blocking mpsc receiver into a futures Stream.
        Box::pin(futures::stream::unfold(rx, |rx| async move {
            // `recv` blocks until a message arrives (or channel drops).
            // This blocks the iced thread-pool thread, but for a 1 Hz
            // subscription this is acceptable.
            match rx.recv() {
                Ok(msg) => Some((msg, rx)),
                Err(_) => None,
            }
        }))
    }
}

fn main() -> iced::Result {
    // ui-quality-gate-overhaul M2-A (T-M2-A-3): under `render-debug` the
    // M2-A spans on `frame::panel`, `frame::loading_with_spinner`, and
    // `strategies::id_cell` need a subscriber to actually surface on
    // stderr. Initialise the workspace-default `tracing_subscriber::fmt`
    // with `RUST_LOG`-driven filtering so operators run
    // `RUST_LOG=ui=trace cargo run -p ui --bin cockpit --features
    // fixtures,render-debug` to triage a render panic. Default builds
    // (no feature) compile this away — the standalone fixtures cockpit
    // does not need its own subscriber for the normal smoke path.
    // Stderr-only per architect Q2 (no audit-ledger sink ships).
    #[cfg(feature = "render-debug")]
    {
        // `try_init` so a host that already installed a subscriber
        // (e.g. cockpit_live or a wrapping test harness) doesn't panic
        // here. The fixtures cockpit is the dominant user under this
        // feature, but defence-in-depth.
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("ui=trace")),
            )
            .with_writer(std::io::stderr)
            .try_init();
    }

    // T2028 + T2029 — Layout-β min-size floor + Lumen brand icon.
    // Shared with `cockpit_live` and `viewer` via
    // `ui::window_icon::standard_window_settings`.
    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .window(ui::window_icon::standard_window_settings())
        .run()
}

#[derive(Default)]
struct App {
    cockpit: Cockpit,
}

impl App {
    fn boot() -> (Self, iced::Task<Message>) {
        // Fixtures boot populates every panel so the layout smoke covers the
        // full column stack without a running agent. v1 (T623) extends this
        // to a top-3 momentum portfolio so the positions panel renders the
        // multi-row steady state for the V8 smoke (R11 negative confirmation
        // — same widget, more rows). v1.5a (T719) extends it again to the
        // mean-reversion-pairs steady state: 3 long-leg position rows +
        // `pairs_mr_h1` strategy row + a recent-events footer carrying both
        // new v1.5a kinds (`MeanReversionStop`, `PairShortObservation`).
        // Operators see the most recent feature set when they fixtures-boot
        // the cockpit — earlier presets stay available for snapshot tests.
        #[cfg(feature = "fixtures")]
        let mut cockpit = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
        #[cfg(not(feature = "fixtures"))]
        let mut cockpit = Cockpit::new();

        // Phase 2 boot — sidebar shell + universe + chart-buffer seed.
        // Q6=(a) (lab-end-to-end-v2 T-D1.2): pre-load bars for ALL 10
        // XRP_FIRST_UNIVERSE pairs so the Lab pair-chip picker has bars for
        // every pair on first click (~12 KB memory cost; R1.4 closure).
        // Previously only 3 symbols (BTC/ETH/SOL) were seeded.
        let universe: Vec<(Venue, Symbol)> = ui::lab::universe::XRP_FIRST_UNIVERSE
            .iter()
            .map(|(v, s)| (*v, Symbol::new(*s)))
            .collect();
        cockpit.universe = universe.clone();
        cockpit.current_screen = Screen::Home;
        // Default selected symbol: BTCUSDT (index 2 in XRP-first order).
        let default_pair = universe
            .iter()
            .find(|(_, s)| s.0.as_str() == "BTCUSDT")
            .cloned()
            .unwrap_or_else(|| universe[0].clone());
        cockpit.selected_symbol = Some(default_pair.clone());
        for (venue, symbol) in &universe {
            let seed = seed_for(*venue, symbol);
            for bar in synthetic_candles(seed, *venue, symbol.clone(), 60) {
                ui::state::update(&mut cockpit, Message::BarReceived(bar));
            }
        }
        // Pre-seed chart markers for the default symbol — Q3 + R8.5.
        cockpit.chart_markers =
            PanelState::Ready(synthetic_fills_for(default_pair.0, &default_pair.1, 4));

        // Phase 3 (Lumen detail screens) — pre-seed Risk / Strategies-config /
        // Audit fixtures so the three new screens render their `Ready` body
        // on first paint (T1707, T1704, T1710 / V5, V6 acceptance).
        #[cfg(feature = "fixtures")]
        {
            cockpit.risk_state = PanelState::Ready(ui::fixtures::fake_risk_state());
            cockpit.strategies_config = Some(ui::fixtures::fake_strategies_config());
            // V6: ≥ 5 visible rows by default — fixtures bin seeds 12 rows so
            // the snapshot baseline can pick its first 5 deterministically;
            // pagination tests separately seed 250 + 5 to exercise page 2.
            let rows = ui::fixtures::fake_journal_rows(12);
            let total = u64::try_from(rows.len()).unwrap_or(0);
            cockpit.audit_screen_state.rows = PanelState::Ready(rows);
            cockpit.audit_screen_state.total_count = Some(total);

            // Phase 4 (T1811) — pre-seed strategy_equity for every
            // loaded strategy so the Strategies-detail sparkline
            // renders the canvas on first paint (no audit ledger
            // query in fixtures mode).
            if let PanelState::Ready(rows) = &cockpit.strategies {
                for row in rows {
                    cockpit.strategy_equity.insert(
                        row.id.clone(),
                        PanelState::Ready(ui::fixtures::fake_equity_series_for_sparkline()),
                    );
                }
            }
        }

        // cockpit-baseline-panel v0.1.0 — boot-load both realized BH curves
        // into `baseline_screen_state` so the navigable Baseline screen shows
        // `Ready` (or `Error`, never a blank/panic, in a fixtures-only checkout
        // where the runbook CSVs are absent). The default screen stays on
        // `Live`/`Home` (D2 — navigable, not default-routed), so this does not
        // change the deterministic first-frame smoke baseline.
        ui::baseline::load_into(&mut cockpit);

        (Self { cockpit }, iced::Task::none())
    }

    fn title(&self) -> String {
        APP_TITLE.to_string()
    }

    fn update(&mut self, msg: Message) -> iced::Task<Message> {
        // Phase 2 — capture (venue, symbol) of `SelectSymbol` BEFORE the
        // model is mutated so we can re-seed the fixtures-mode marker
        // panel after `update` flips it to `Loading`.
        let select_pair = if let Message::SelectSymbol(v, s) = &msg {
            Some((*v, s.clone()))
        } else {
            None
        };
        // Phase 3 R5.2 / Q11b — compound dispatch: when a Home →
        // Strategies-summary row click emits `SelectStrategy(id)`, chain
        // `Task::done(SwitchScreen(Strategies))` if the operator wasn't
        // already on the Strategies screen. Capture the marker before
        // `update` runs so the screen-switch decision uses the
        // pre-mutation `current_screen` value.
        let cross_link_strategies = matches!(msg, Message::SelectStrategy(_))
            && self.cockpit.current_screen != Screen::Strategies;

        // Phase B / cockpit-training-control follow-up — fixtures mode
        // does NOT carry a tokio runtime so the real engine is unreachable
        // (`cockpit_live.rs` owns `--features live`). Without a synthetic
        // completion the Lab Run button hangs on "Running" forever because
        // `LabRunCompleted` is never dispatched. Capture the request marker
        // BEFORE `state::update` flips `lab_run_inflight = true`, then chain
        // an immediate synthetic `LabRunCompleted(Ok(empty_summary))` so
        // the UI returns to idle. Real backtest execution requires
        // `cargo run -p ui --bin cockpit_live --features live`.
        let lab_run_requested = matches!(msg, Message::LabRunRequested);

        ui::state::update(&mut self.cockpit, msg);

        if let Some((v, s)) = select_pair {
            let fills = synthetic_fills_for(v, &s, 4);
            ui::state::update(&mut self.cockpit, Message::ChartMarkersLoaded(Ok(fills)));
        }
        if cross_link_strategies {
            return iced::Task::done(Message::SwitchScreen(Screen::Strategies));
        }
        if lab_run_requested {
            // Synthetic completion — fixtures binary has no engine wired.
            // Returns the same empty `RunSummary` shape `spawn_lab_run`
            // produces in its no-runtime branch (`runner.rs:267-275`).
            let strategy = self
                .cockpit
                .lab_state
                .strategy
                .as_ref()
                .map(|s| s.0.clone())
                .unwrap_or_else(|| smol_str::SmolStr::new_static(""));
            let symbol = self
                .cockpit
                .lab_state
                .pair
                .as_ref()
                .map(|(_, s)| s.0.clone())
                .unwrap_or_else(|| smol_str::SmolStr::new_static(""));
            let summary = ui::lab::runner::RunSummary {
                strategy_id: strategy,
                symbol,
                report_path: None,
                equity_series: Vec::new(),
                fills: Vec::new(),
                kpis: backtest::BacktestKpis::default(),
                bars: std::sync::Arc::new(Vec::new()),
                position_curve: Vec::new(),
            };
            return iced::Task::done(Message::LabRunCompleted(Ok(summary)));
        }
        iced::Task::none()
    }

    /// Cockpit subscription — fixtures path + modal-open keyboard recipe.
    ///
    /// - `fixtures` or default → no live bus; the `fixtures` boot already
    ///   populates every panel.  We DO add a 1 Hz `time::every` recipe
    ///   so the status-bar server-time field advances each second
    ///   (T1509). `MarketHealth` is NOT subscribed here — fixtures boot
    ///   with `Fresh` per-venue state and never update.
    /// - The retired `live` arm now lives in `cockpit_live` (T908).
    /// - When the tape-row → audit modal is open, batch in an
    ///   `iced::event::listen_with` recipe that translates the keyboard
    ///   `Esc` press into `Message::TapeAuditModalClosed` (Q6 —
    ///   modal-open-gated subscription). Other keys are not consumed
    ///   (the fixtures cockpit has no keyboard navigation today, so
    ///   nothing leaks).
    fn subscription(&self) -> iced::Subscription<Message> {
        // 1 Hz server-time tick — drives the status-bar clock (T1509).
        // Use iced::advanced::subscription::from_recipe with a custom Recipe
        // that uses an OS thread + std::sync::mpsc for the timer, so no
        // tokio dep is required in the fixtures bin.
        let time_sub = iced::advanced::subscription::from_recipe(ServerTimeRecipe);

        if self.cockpit.tape_audit_modal.is_some() {
            iced::Subscription::batch(vec![
                time_sub,
                iced::event::listen_with(|event, _status, _window| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        ..
                    }) => Some(Message::TapeAuditModalClosed),
                    _ => None,
                }),
            ])
        } else {
            time_sub
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Phase 2 — shell composes sidebar + screen body + status bar +
        // reserved right-rail. The screen-routed body dispatches off
        // `current_screen`. (T1603 / T1611.)
        let main_column: Element<'_, Message> = shell::view(&self.cockpit, ThemeMode::Dark);

        // Render the modal as a `Stack` overlay only when the modal is open
        // (`tape-row-audit-modal` Q1). When closed, return `main_column`
        // directly so the cockpit's iced widget tree is byte-identical to
        // the pre-modal world — existing `panel_snapshots__*` stay green
        // by construction (V7 / R11).
        if let Some(modal_state) = self.cockpit.tape_audit_modal.as_ref() {
            journal_transaction_modal::view(modal_state, main_column, Message::TapeAuditModalClosed)
        } else {
            main_column
        }
    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }
}
