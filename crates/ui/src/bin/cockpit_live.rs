//! `cockpit_live` — unified agent + iced cockpit binary (T904 /
//! [live-cockpit-unified](../../../../spec/features/live-cockpit-unified.md)).
//!
//! ## Why this binary exists
//!
//! Before T904, an operator needed two processes to see live trading:
//!
//! 1. `cargo run --bin trading -- --config config/agent.toml --mode paper`
//!    — the headless agent (data feed, strategy watcher, paper engine,
//!    audit ledger).
//! 2. `cargo run --bin cockpit --features live` — the iced cockpit, which
//!    constructed an *empty* `Arc<EventBus>` because there was no IPC
//!    layer to connect it to the running agent. Every panel sat in
//!    `Loading` forever (the v0 IPC handoff contract — see
//!    `spec/reports/dev-week2-broadcast-api-2026-04-18.md`).
//!
//! `cockpit_live` collapses both into a single process: the agent
//! runtime and the iced cockpit share one `Arc<EventBus>` + one
//! `Arc<KillSwitch>` + one `CancellationToken`, so the cockpit
//! subscription receives the same broadcast events the running agent
//! produces — no IPC, no empty-bus dead path.
//!
//! ## Runtime topology (architect's design — Q2)
//!
//! macOS hard-requires GUI work on the main thread, and `iced::run`
//! blocks until the window closes. tokio cannot also own the main
//! thread, so we:
//!
//! 1. Construct `config`, `ledger`, `kill_switch`, `registry`, `bus`,
//!    `boot_id` synchronously on the main thread (no tokio runtime
//!    needed; `audit::Ledger::open` is `async` so we drive it via a
//!    short-lived `current_thread` runtime).
//! 2. Open the audit uptime interval (T806 R7.1) on the same
//!    short-lived runtime.
//! 3. Build a `tokio::runtime::Builder::new_multi_thread().enable_all()
//!    .build()?` runtime and `std::thread::spawn` a side thread that
//!    `block_on(agent::runtime::run(handles, cancel))`.
//! 4. On the same side thread, before calling `runtime::run`, also spawn
//!    a `tokio::signal::ctrl_c()` listener that calls
//!    `cancel.cancel()` (Q3 — Ctrl-C path V3a).
//! 5. Run `iced::application(..).run()` on the main thread (Q2 — iced
//!    owns the main thread).
//! 6. When the iced window closes, `iced::run` returns. We then call
//!    `cancel.cancel()` (idempotent if Ctrl-C already fired), join the
//!    side thread with a 2 s wall-clock bound (Q3 — `shutdown_deadline`),
//!    then write the close-uptime row and exit 0 (Q3 — sequence step 6).
//!
//! ## Out of scope for T904 (skeleton only — see task list)
//!
//! - Window-close → side-thread `cancel` bridge via a `Message::ShutdownRequested`
//!   recipe (lands with T905 / T906 / T911 once `state::Message` adds the
//!   variant). For now, `iced::run` returns naturally when the operator
//!   closes the window, and the side-thread `cancel.cancel()` happens
//!   *after* `iced::run` returns. Ctrl-C while the window is open
//!   gracefully shuts down the agent side thread but does not
//!   auto-close the iced window — operator closes the window manually.
//!   The 2 s deadline only starts after the window actually closes.
//! - `Message::ShutdownRequested` Message variant — added by T905/T906.
//!
//! ## T906 stitch (Wave 3 follow-up — 2026-05-01)
//!
//! The kill-button trip wire (`Cockpit::kill_switch = Some(trip_closure)`)
//! is now installed below.  Per Q6 + the orchestrator's option (A), the
//! side-thread tokio runtime is constructed in `main()` (not inside
//! `agent::runtime::run`) so we capture `runtime.handle().clone()`
//! BEFORE moving the runtime into the side thread.  The closure
//! `Handle::spawn`s `KillSwitch::trip(reason)` onto that runtime — the
//! trip's internal `tokio::spawn` for the T809 audit dual-write therefore
//! lands inside a runtime context (which iced's main thread does not
//! have).  See `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`
//! for the integration proof.
//!
//! ## Determinism note
//!
//! No `SystemTime::now()` / `Instant::now()` reachable from any backtest
//! replay path: this is a binary that *runs the live agent*, not a
//! backtest harness. The boot id is a `Uuid::new_v4()` — random by
//! design — and lives in YAML front-matter when audit rows reference it
//! (T806 invariant), never in any anchored backtest body.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use agent::{EventBus, KillSwitch, RunHandles};

use smol_str::SmolStr;
use ui::state::{Cockpit, JournalTransactionView, Message};
use ui::strings::APP_TITLE;
use ui::theme::{color, layout, space};
use ui::widgets::{journal_transaction_modal, kill, latency, pnl, positions, strategies, tape};

use iced::widget::{Column, Row};
use iced::{Element, Length};

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Mirror of the `trading` bin CLI so operator muscle memory carries
/// across (`--config`, `--mode`).
#[derive(Parser, Debug)]
#[command(
    name = "cockpit_live",
    about = "Unified agent + iced cockpit (live-cockpit-unified / T904)"
)]
struct Args {
    /// Path to `config/agent.toml`.
    #[arg(long, default_value = "config/agent.toml")]
    config: PathBuf,

    /// Operating mode override (`research` | `paper`). Defaults to whatever
    /// the loaded config specifies.
    #[arg(long)]
    mode: Option<String>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Hard wall-clock bound for the side-thread join after the iced window
/// closes. Architect's Q3 sets this at 2 s; exceeding it logs
/// `shutdown_deadline_exceeded` and force-exits the process.
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(2);

fn main() -> Result<()> {
    // ── Tracing ──────────────────────────────────────────────────────────────
    // Mirrors `crates/agent/src/main.rs` so audit / ops dashboards see the
    // same log lines whether the operator boots the headless agent or the
    // unified cockpit_live bin.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(
                    "cockpit_live=info"
                        .parse()
                        .expect("static directive parses"),
                )
                .add_directive("agent=info".parse().expect("static directive parses"))
                .add_directive("ui=info".parse().expect("static directive parses")),
        )
        .json()
        .init();

    let args = Args::parse();
    info!("cockpit_live starting");

    // ── Config ───────────────────────────────────────────────────────────────
    let cfg = if args.config.exists() {
        agent::config::Config::load(&args.config).context("load config")?
    } else {
        warn!(path = ?args.config, "config file not found — using defaults");
        agent::config::Config::default()
    };

    if let Some(ref m) = args.mode {
        if m.eq_ignore_ascii_case("live") {
            anyhow::bail!("mode=live is rejected in v0");
        }
    }
    info!(mode = %cfg.mode, "config loaded");

    // ── Short-lived bootstrap runtime ────────────────────────────────────────
    // `audit::Ledger::open`, the chart-of-accounts bootstrap, and
    // `open_uptime_interval` are all `async fn`s but need to run *before*
    // we hand ownership of the side-thread runtime away. Spin up a tiny
    // single-threaded runtime here, drive the bootstrap to completion,
    // then drop it. This keeps the side-thread runtime exclusively
    // responsible for the long-running task graph.
    let bootstrap_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build bootstrap runtime")?;

    // ── Audit ledger ─────────────────────────────────────────────────────────
    // Opened BEFORE the kill switch so the trip handler can dual-write
    // (T809). Mirrors crates/agent/src/main.rs.
    let db_path = &cfg.audit.ledger_db_path;
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let ledger = bootstrap_rt
        .block_on(async {
            let l = audit::Ledger::open(db_path)
                .await
                .context("open audit ledger")?;
            audit::bootstrap::chart_of_accounts(&l)
                .await
                .context("bootstrap chart of accounts")?;
            Ok::<_, anyhow::Error>(l)
        })
        .map(Arc::new)?;
    info!(db = %db_path, "audit ledger initialized");

    // ── Observability ────────────────────────────────────────────────────────
    // Same gate as the headless `trading` bin: the prometheus exporter
    // is started here so /metrics surfaces from the unified process.
    // T901 added `prometheus_enabled` so an operator running the cockpit
    // on a laptop can disable the exporter when binding :9100 is wrong.
    if let Err(e) = agent::observability::start_prometheus_exporter(&cfg.observability) {
        warn!(error = %e, "prometheus exporter failed to start (non-fatal)");
    }
    agent::observability::register_metrics();
    info!("observability initialized");

    // ── Kill switch ──────────────────────────────────────────────────────────
    // T809-wired (audit ledger + production incident spawner). The
    // halt-file watcher is spawned *inside* `agent::runtime::run` on the
    // side-thread runtime — see crates/agent/src/runtime.rs.
    let incident_spawner: Arc<dyn agent::IncidentSpawner> = Arc::new(agent::CommandIncidentSpawner);
    let kill_switch = Arc::new(KillSwitch::with_audit(
        &cfg.kill_switch.halt_file,
        32,
        Arc::clone(&ledger),
        incident_spawner,
    ));
    info!(halt_file = %cfg.kill_switch.halt_file, "kill switch initialized (audit-wired)");

    // ── Strategy registry ────────────────────────────────────────────────────
    let registry = strategy::StrategyRegistry::new();
    registry.register(Box::new(strategy::SmaCrossover::new(
        cfg.strategies.sma_crossover.fast_len,
        cfg.strategies.sma_crossover.slow_len,
    )));
    info!(
        fast = cfg.strategies.sma_crossover.fast_len,
        slow = cfg.strategies.sma_crossover.slow_len,
        "strategy registry constructed",
    );
    let registry = Arc::new(registry);

    // ── Broadcast bus ────────────────────────────────────────────────────────
    let bus = Arc::new(EventBus::new(&cfg.bus));
    info!("broadcast event bus initialized");

    // ── Open uptime interval (T806 R7.1) ─────────────────────────────────────
    // Same caller-side ordering as the headless trading bin: open BEFORE
    // entering the runtime so the heartbeat task spawned inside
    // `runtime::run` writes against the same boot id.
    let boot_id = uuid::Uuid::new_v4().to_string();
    bootstrap_rt.block_on(async {
        if let Err(e) = audit::journal::open_uptime_interval(&ledger, &boot_id, None).await {
            warn!(error = %e, "open_uptime_interval failed (non-fatal)");
        } else {
            info!(boot_id = %boot_id, "agent uptime interval opened");
        }
    });

    // Drop the bootstrap runtime — the side thread builds its own.
    drop(bootstrap_rt);

    // ── Cancellation token ───────────────────────────────────────────────────
    // Single token shared three ways:
    // 1. iced thread: cancels after `iced::run` returns (window closed).
    // 2. side-thread runtime: every spawned task observes via child tokens.
    // 3. side-thread Ctrl-C listener: cancels on SIGINT.
    let cancel = CancellationToken::new();

    // ── Side-thread tokio runtime ────────────────────────────────────────────
    // Q2 topology — iced needs the main thread (macOS GUI), so the
    // tokio runtime hosting the agent task graph runs on a side thread.
    //
    // T906 stitch: the runtime is built HERE (not inside the side thread)
    // so we can capture `runtime.handle().clone()` BEFORE handing
    // ownership of the runtime to the spawned thread.  The handle is
    // injected into the kill-button trip closure (see further below) so
    // `KillSwitch::trip`'s internal `tokio::spawn` for the T809 audit
    // dual-write lands inside a runtime context — iced's main thread has
    // no tokio runtime of its own.  Option (A) per the orchestrator's
    // brief; keeps `agent::runtime::run` agnostic of whether its caller
    // built the runtime in-place or hands one in.
    let agent_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("agent-rt")
        .build()
        .context("build side-thread tokio runtime")?;
    let rt_handle = agent_runtime.handle().clone();

    // ── Side thread: own the runtime, drive agent::runtime::run on it ────────
    let agent_handle = {
        let cancel = cancel.clone();
        let handles = RunHandles {
            config: Arc::new(cfg),
            ledger: Arc::clone(&ledger),
            bus: Arc::clone(&bus),
            kill_switch: Arc::clone(&kill_switch),
            registry: Arc::clone(&registry),
            boot_id: boot_id.clone(),
        };
        let ledger_for_close = Arc::clone(&ledger);
        let boot_id_for_close = boot_id.clone();

        std::thread::Builder::new()
            .name("agent-runtime".to_string())
            .spawn(move || {
                let rt = agent_runtime;

                rt.block_on(async move {
                    // Ctrl-C bridge — V3a path. macOS routes SIGINT to the
                    // process; the side-thread runtime can `await` it just
                    // as easily as the main thread. On SIGINT, trip the
                    // shared cancel token; the agent's internal `select!`
                    // observes its child tokens and drains.
                    let cancel_signal = cancel.clone();
                    tokio::spawn(async move {
                        if tokio::signal::ctrl_c().await.is_ok() {
                            info!("ctrl-c received — shutting down agent runtime");
                            cancel_signal.cancel();
                        }
                    });

                    if let Err(e) = agent::runtime::run(handles, cancel).await {
                        warn!(error = %e, "agent runtime exited with error");
                    }

                    // T806 close-uptime row — written on the side-thread
                    // runtime so `audit::journal` can use the runtime's
                    // sqlite executor. Mirrors the headless trading bin's
                    // ordering (caller writes the close row *after*
                    // `runtime::run` returns).
                    agent::runtime::shutdown_writer(ledger_for_close, &boot_id_for_close).await;
                });

                info!("agent side thread exiting");
            })
            .context("spawn agent side thread")?
    };

    // ── Main thread: iced cockpit ────────────────────────────────────────────
    // The iced subscription is wired against `Arc<EventBus>` via the
    // existing `ui::live::subscription` (T32). Any bus producer the agent
    // runtime spins up (data feed taps T903b, paper engine T903a,
    // reconciler T903c, mode forwarder T905) immediately reaches the
    // cockpit panels — no IPC.
    //
    // T906 stitch: build the trip closure that the cockpit's
    // `Message::KillConfirmed` arm invokes.  The closure captures
    // (a) `Arc<KillSwitch>` (the same instance held by `RunHandles` — so
    // a trip propagates through `kill_switch.subscribe()` →
    // `spawn_mode_forwarder` (T905) → `bus.publish_mode(Halted)` → the
    // cockpit's mode subscription, and via the audit dual-write into the
    // ledger), and (b) the side-thread runtime's `Handle` so
    // `KillSwitch::trip`'s internal `tokio::spawn` for the T809 audit
    // dual-write executes end-to-end (see kill_switch.rs:283-298).
    let trip: ui::state::KillTripFn = {
        let kill_for_trip = Arc::clone(&kill_switch);
        let rt_handle_for_trip = rt_handle.clone();
        Arc::new(move |reason| {
            let kill = Arc::clone(&kill_for_trip);
            // Spawn onto the side-thread runtime so the trip's internal
            // `tokio::spawn` (the T809 audit dual-write) succeeds —
            // iced's main thread has no tokio runtime context.
            rt_handle_for_trip.spawn(async move {
                kill.trip(reason);
            });
        })
    };

    let mut cockpit = Cockpit::new();
    cockpit.kill_switch = Some(trip);

    let app_state = AppState {
        cockpit,
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        ledger: Arc::clone(&ledger),
        rt_handle: rt_handle.clone(),
    };

    let iced_result = iced::application(
        move || (app_state.clone(), iced::Task::none()),
        AppState::update,
        AppState::view,
    )
    .title(AppState::title)
    .theme(AppState::theme)
    .subscription(AppState::subscription)
    .run();

    // ── Shutdown ─────────────────────────────────────────────────────────────
    // V3b sequence — iced has returned (window closed). Trip the shared
    // cancel token (idempotent if Ctrl-C already fired), then join the
    // side thread with a 2 s wall-clock bound. On timeout we log
    // `shutdown_deadline_exceeded` and exit anyway (force-abort) — Q3
    // explicitly accepts a missed close-uptime row over a hung process.
    cancel.cancel();
    info!("iced window closed — joining agent side thread");

    if !join_with_deadline(agent_handle, SHUTDOWN_DEADLINE) {
        warn!(
            deadline_ms = SHUTDOWN_DEADLINE.as_millis() as u64,
            "shutdown_deadline_exceeded — agent side thread did not join in bound; force-exiting"
        );
        // Q3 explicitly mandates force-exit on deadline exceedance. The
        // close-uptime row may be missing from this boot's audit trail
        // — that is observability degradation, not control-flow loss.
        std::process::exit(0);
    }

    info!("cockpit_live exited cleanly");
    iced_result.map_err(Into::into)
}

/// Join a `JoinHandle` with a wall-clock deadline. Returns `true` on a
/// clean join inside `deadline`, `false` on timeout. The polling loop
/// uses 10 ms ticks — short enough that the 2 s budget feels
/// instantaneous, long enough that we don't burn CPU on idle waits.
///
/// Why poll instead of spawn-a-joiner-thread: the deadline path needs
/// to *return* (so `main` can decide to log + force-exit); a joiner
/// thread would leak. `JoinHandle::is_finished()` is stable since 1.61
/// and this is the idiomatic way to bound a join.
fn join_with_deadline(handle: std::thread::JoinHandle<()>, deadline: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        if handle.is_finished() {
            // Drain the join now that the thread is finished. `.join()`
            // returns immediately at this point; an Err means the side
            // thread panicked, which we log and treat as "joined".
            if let Err(e) = handle.join() {
                warn!(?e, "agent side thread panicked during shutdown");
            }
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

// ── iced Application state ────────────────────────────────────────────────────

/// iced application state. Mirrors `cockpit.rs::App` but carries real
/// `Arc<EventBus>` + `Arc<KillSwitch>` instead of the empty-bus
/// placeholder. `Clone` because iced's functional-builder API moves the
/// `boot` closure into the runtime; cloning once at boot is cheap (Arcs
/// are refcount bumps, `Cockpit::new()` allocates a few empty
/// collections).
#[derive(Clone)]
struct AppState {
    cockpit: Cockpit,
    bus: Arc<EventBus>,
    /// Held on the iced side to keep the Arc alive for the lifetime of
    /// the iced application (the trip closure inside `cockpit.kill_switch`
    /// also holds an `Arc<KillSwitch>`, so technically this field is
    /// redundant for liveness — but keeping it named makes the
    /// shared-ownership story explicit and keeps the option open for
    /// future iced-side reads, e.g. an `is_tripped()` poll for an
    /// already-halted banner on app cold-boot).
    #[allow(dead_code)]
    kill_switch: Arc<KillSwitch>,
    /// Shared audit-ledger handle. Used by the tape-row → audit-modal
    /// click path (`Message::TapeRowClicked`) to read journal entries
    /// for the clicked transaction id via
    /// `audit::query::journal_entries_for_transaction`. The query is
    /// dispatched on the side-thread tokio runtime (`rt_handle`) — iced's
    /// main thread has no tokio runtime context.
    ledger: Arc<audit::Ledger>,
    /// Side-thread tokio runtime handle. Used to drive `audit::query`
    /// fetches issued by the cockpit's iced thread (where there is no
    /// tokio runtime). `iced::Task::perform` requires an executor; we
    /// route through `rt_handle.spawn(...)` so the future runs on the
    /// agent runtime.
    rt_handle: tokio::runtime::Handle,
}

impl AppState {
    fn title(&self) -> String {
        APP_TITLE.to_string()
    }

    /// Pure-state update + side-effect dispatch.
    ///
    /// `ui::state::update` mutates the cockpit model deterministically
    /// (no I/O). On `Message::TapeRowClicked`, we additionally issue an
    /// async fetch against the audit ledger via `iced::Task::perform`
    /// — the result returns to the cockpit as
    /// `Message::TapeAuditEntriesLoaded`. The state mutation that
    /// flips the modal sub-state into `Loading` is owned by
    /// `ui::state::update`; the binary owns only the I/O wiring
    /// (Q5 — separation of pure state from side-channel I/O).
    fn update(&mut self, msg: Message) -> iced::Task<Message> {
        // Capture the tx_id before delegating so we can dispatch the
        // async fetch after `update` mutates the model.
        let tx_id = match &msg {
            Message::TapeRowClicked(tx) => Some(tx.clone()),
            _ => None,
        };

        ui::state::update(&mut self.cockpit, msg);

        if let Some(tx_id) = tx_id {
            let ledger = Arc::clone(&self.ledger);
            let rt_handle = self.rt_handle.clone();
            iced::Task::perform(
                async move {
                    // Bridge the iced thread's lack-of-runtime to the
                    // side-thread tokio runtime: spawn the audit read on
                    // `rt_handle` and await its `JoinHandle`. The handle
                    // is `Send + 'static`, so the closure is `iced::Task`
                    // friendly.
                    let join = rt_handle.spawn(async move {
                        let tx_id_str = tx_id.as_str();
                        match audit::query::journal_entries_for_transaction(&ledger, tx_id_str)
                            .await
                        {
                            Ok(entries) => {
                                // Best-effort header — the dedicated
                                // `journal_transactions` metadata reader
                                // is a follow-up. Use the first entry's
                                // `ts` as a proxy until then; description
                                // and strategy_id default to empty / None
                                // so the modal still renders.
                                let ts = entries
                                    .first()
                                    .map_or_else(trading_core::Timestamp::now, |e| e.ts);
                                Ok(JournalTransactionView {
                                    tx_id: SmolStr::new(tx_id_str),
                                    ts,
                                    description: SmolStr::default(),
                                    strategy_id: None,
                                    entries,
                                })
                            }
                            Err(e) => Err(SmolStr::new(e.to_string())),
                        }
                    });
                    match join.await {
                        Ok(result) => result,
                        Err(e) => Err(SmolStr::new(format!("audit task join: {e}"))),
                    }
                },
                Message::TapeAuditEntriesLoaded,
            )
        } else {
            iced::Task::none()
        }
    }

    /// Subscribe to the real bus — the entire point of `cockpit_live`.
    /// `ui::live::subscription` already exists (T32) and batches the
    /// six core channels + three v0.5 strategy lifecycle channels.
    ///
    /// When the tape-row → audit modal is open, batch in an
    /// `iced::event::listen_with` recipe so `Esc` closes the modal
    /// (Q6 — modal-open-gated subscription). Other keys are not
    /// consumed; the live cockpit has no general keyboard navigation
    /// today, so nothing leaks to the tape beneath.
    fn subscription(&self) -> iced::Subscription<Message> {
        let bus_sub = ui::live::subscription(Arc::clone(&self.bus));
        if self.cockpit.tape_audit_modal.is_some() {
            iced::Subscription::batch(vec![
                bus_sub,
                iced::event::listen_with(|event, _status, _window| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        ..
                    }) => Some(Message::TapeAuditModalClosed),
                    _ => None,
                }),
            ])
        } else {
            bus_sub
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Layout mirrors `cockpit.rs` so a screenshot taken against the
        // unified bin matches the fixtures cockpit's pixel positions.
        let left = Column::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(pnl::view(&self.cockpit))
            .push(latency::view(&self.cockpit))
            .push(kill::view(&self.cockpit))
            .width(Length::FillPortion(1));

        let right = Column::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(strategies::view(&self.cockpit))
            .push(positions::view(&self.cockpit))
            .push(tape::view(&self.cockpit))
            .width(Length::FillPortion(2));

        let body = Row::new()
            .spacing(layout::PANEL_OUTER_GAP)
            .push(left)
            .push(right);

        let main_column: Element<'_, Message> = iced::widget::container(body)
            .padding(space::L as u16)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &iced::Theme| iced::widget::container::Style {
                background: Some(color::BG.into()),
                text_color: Some(color::FG),
                ..Default::default()
            })
            .into();

        // Render the modal as a `Stack` overlay only when the modal is
        // open (tape-row-audit-modal Q1). When closed, return
        // `main_column` directly so the cockpit's iced widget tree is
        // byte-identical to the pre-modal world — existing
        // `panel_snapshots__*` stay green by construction (V7 / R11).
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
