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

use trading_core::Timestamp;

use anyhow::{Context, Result};
use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use agent::{EventBus, KillSwitch, RunHandles};

use smol_str::SmolStr;
use trading_core::{Symbol as CoreSymbol, Venue as CoreVenue};
use ui::shell;
use ui::state::{Cockpit, JournalTransactionView, Message, Screen};
use ui::strings::{APP_TITLE, TAPE_AUDIT_MODAL_ERROR_PREFIX};
use ui::theme::ThemeMode;
use ui::widgets::journal_transaction_modal;

use iced::Element;
use iced::advanced::subscription::{EventStream, Hasher, Recipe};
use iced::futures;

// ── Server-time recipe (T1509) ────────────────────────────────────────────────
//
// Emits `Message::ServerTimeTick` every second using a `tokio::time::interval`.
// The `live` feature brings `tokio` as a direct dep, so
// `tokio::time::interval` is available here.

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
        Box::pin(async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            // Skip the first (immediate) tick so the first ServerTimeTick
            // arrives ~1 s after subscription, not immediately at boot.
            interval.tick().await;
            loop {
                interval.tick().await;
                yield Message::ServerTimeTick(Timestamp::now());
            }
        })
    }
}

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

// ui-session-journal-iced-tester v0.1 (T02 — REVISED) — there is no
// runtime `--record-tests` flag. iced 0.14's
// `iced::Application::run()` auto-wraps with `iced_tester::attach()`
// when the `tester` feature is enabled (see
// iced-0.14.0/src/application.rs:198). The recorder is therefore a
// compile-time choice via `--features record-tests`. Build the
// recorder binary with:
//
//   cargo build --features live,record-tests --bin cockpit_live
//
// and every run of that binary opens with the overlay visible. Use
// the default `--features live` build for production / non-recording
// sessions.

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

    // ── Phase 2 — chart universe (T1612) ─────────────────────────────────────
    // Build `Vec<(Venue, Symbol)>` from the loaded `Config` BEFORE the
    // `cfg` is moved into `RunHandles`. The cockpit chip row reads this
    // (capped to the first 3 entries to keep the row scannable in
    // Phase 2; Phase 3+ may grow it).
    let universe_pairs: Vec<(CoreVenue, CoreSymbol)> = {
        let mut out: Vec<(CoreVenue, CoreSymbol)> = Vec::new();
        // Binance is always present (no `enabled` toggle); Coinbase and
        // Kraken are operator-gated per v1.5b T1408/T1409.
        let mut venues: Vec<CoreVenue> = vec![CoreVenue::Binance];
        if cfg.data.sources.coinbase.enabled {
            venues.push(CoreVenue::Coinbase);
        }
        if cfg.data.sources.kraken.enabled {
            venues.push(CoreVenue::Kraken);
        }
        // Build the symbol list via the same toggle path the agent uses;
        // fall back to the Binance defaults if the loader rejects the
        // toggles (e.g. both disabled).
        let symbols: Vec<CoreSymbol> = trading_core::Universe::from_toggles(
            cfg.universe.usdt_enabled,
            cfg.universe.usdc_enabled,
        )
        .map(|u| u.symbols.iter().cloned().collect())
        .unwrap_or_else(|_| {
            vec![
                CoreSymbol::new("BTCUSDT"),
                CoreSymbol::new("ETHUSDT"),
                CoreSymbol::new("SOLUSDT"),
            ]
        });
        for v in &venues {
            for s in symbols.iter().take(3) {
                out.push((*v, s.clone()));
            }
        }
        if out.is_empty() {
            out.push((CoreVenue::Binance, CoreSymbol::new("BTCUSDT")));
        }
        out
    };

    // ── Strategy registry ────────────────────────────────────────────────────
    // Delegated to `agent::runtime::build_registry` so the `ui` crate has
    // no direct `strategy` dependency (architecture rule: ui depends only
    // on `core`, `audit`, and `agent`).
    let registry = agent::runtime::build_registry(&cfg);

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
    cockpit.current_screen = Screen::Home;
    cockpit.universe = universe_pairs.clone();
    if let Some(first) = universe_pairs.first() {
        cockpit.selected_symbol = Some(first.clone());
    }

    let app_state = AppState {
        cockpit,
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        ledger: Arc::clone(&ledger),
        rt_handle: rt_handle.clone(),
    };

    // ui-session-journal-iced-tester v0.1 (T03 — REVISED) — recorder
    // overlay is auto-attached by `iced::Application::run()` when the
    // `record-tests` feature pulls `iced/tester`. No manual
    // `iced_tester::attach()` call needed; no runtime gate.
    #[cfg(feature = "record-tests")]
    info!("iced_tester recorder overlay enabled (compile-time via --features record-tests)");

    let iced_result = iced::application(
        move || (app_state.clone(), iced::Task::none()),
        AppState::update,
        AppState::view,
    )
    .title(AppState::title)
    .theme(AppState::theme)
    .subscription(AppState::subscription)
    // T2028 + T2029 — Layout-β min-size floor + Lumen brand icon.
    .window(ui::window_icon::standard_window_settings())
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
        // Phase 2 — capture (venue, symbol) on `SelectSymbol` for the
        // marker re-fetch dispatch (T1610). The window is the last
        // 60 minutes against `server_time_now` (or current Timestamp
        // as a fallback when the 1 Hz tick has not landed yet).
        let select_pair = match &msg {
            Message::SelectSymbol(v, s) => Some((*v, s.clone())),
            _ => None,
        };
        // Phase 3 R5.2 / Q11b — compound dispatch: on a Home →
        // Strategies-summary row click (`SelectStrategy(id)` from a
        // non-Strategies screen), chain `Task::done(SwitchScreen(
        // Strategies))`. Capture before `update` so the marker reads
        // the pre-mutation `current_screen`.
        let cross_link_strategies = matches!(msg, Message::SelectStrategy(_))
            && self.cockpit.current_screen != Screen::Strategies;
        // Phase 4 (T1811) — capture the strategy id BEFORE `update`
        // mutates the cockpit so we can chain a Task::perform fetch
        // for the equity curve. `Loading` marker is inserted into
        // `model.strategy_equity` right after `update` runs.
        let select_strategy_id: Option<trading_core::StrategyId> = match &msg {
            Message::SelectStrategy(id) => Some(id.clone()),
            _ => None,
        };

        // T-D-N9: capture LabRunRequested BEFORE state::update mutates the
        // cockpit (lab_run_inflight flips to true inside state::update). We
        // need the pre-update LabState to build LabRunConfig.
        let lab_run_requested = matches!(msg, Message::LabRunRequested);
        let lab_run_cfg = if lab_run_requested {
            // Build LabRunConfig from current LabState before state::update.
            use smol_str::SmolStr;
            use ui::lab::runner::LabRunConfig;
            use ui::lab::state::DateRange as LabDateRange;
            use ui::lab::state::Preset;
            let ls = &self.cockpit.lab_state;
            if let (Some(strategy), Some((venue, symbol))) =
                (ls.strategy.as_ref(), ls.pair.as_ref())
            {
                let range_label = match &ls.range {
                    LabDateRange::Preset(Preset::Last30d) => SmolStr::new("Last30d"),
                    LabDateRange::Preset(Preset::Last90d) => SmolStr::new("Last90d"),
                    LabDateRange::Preset(Preset::H1_2024) => SmolStr::new("H1_2024"),
                    LabDateRange::Preset(Preset::H2_2024) => SmolStr::new("H2_2024"),
                    LabDateRange::Custom { start_raw, end_raw } => {
                        SmolStr::new(format!("Custom:{start_raw}:{end_raw}"))
                    }
                };
                Some(LabRunConfig {
                    strategy_id: SmolStr::new(&strategy.0),
                    symbol: SmolStr::new(symbol.0.as_str()),
                    venue: SmolStr::new(format!("{venue:?}")),
                    range_label,
                    seed: ui::lab::defaults::LAB_DEFAULT_SEED,
                    write_report: true,
                })
            } else {
                None
            }
        } else {
            None
        };

        ui::state::update(&mut self.cockpit, msg);

        if let Some(ref id) = select_strategy_id {
            // Phase 4 R13 — insert Loading marker so the screen
            // renders the loading copy until the fetch returns.
            self.cockpit
                .strategy_equity
                .insert(id.clone(), ui::state::PanelState::Loading);
        }

        if let Some((venue, symbol)) = select_pair {
            let now = self
                .cockpit
                .server_time_now
                .unwrap_or_else(trading_core::Timestamp::now);
            let now_ms = now.unix_millis();
            let since_ms = now_ms.saturating_sub(60 * 60 * 1_000);
            let since = trading_core::Timestamp::new(
                time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(since_ms) * 1_000_000)
                    .unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
            );
            let until = now;
            // Fan out two parallel fetches: existing markers (fills) +
            // chart-buy-sell-emphasis v1.9 (T2017) ghost signals. Each
            // dispatches its own typed `Message::Chart*Loaded` arm so the
            // state update is exhaustive and pure.
            let ledger_m = Arc::clone(&self.ledger);
            let rt_handle_m = self.rt_handle.clone();
            let venue_m = venue;
            let symbol_m = symbol.clone();
            let markers_task = iced::Task::perform(
                async move {
                    let join = rt_handle_m.spawn(async move {
                        audit::query::recent_fills_filtered(
                            &ledger_m, venue_m, symbol_m, since, until,
                        )
                        .await
                        .map_err(|e| SmolStr::new(format!("{e}")))
                    });
                    match join.await {
                        Ok(result) => result,
                        Err(e) => Err(SmolStr::new(format!("audit task join: {e}"))),
                    }
                },
                Message::ChartMarkersLoaded,
            );

            let ledger_s = Arc::clone(&self.ledger);
            let rt_handle_s = self.rt_handle.clone();
            let signals_task = iced::Task::perform(
                async move {
                    let join = rt_handle_s.spawn(async move {
                        audit::query::recent_signals(&ledger_s, venue, symbol, since, until)
                            .await
                            .map_err(|e| SmolStr::new(format!("{e}")))
                    });
                    match join.await {
                        Ok(result) => result,
                        Err(e) => Err(SmolStr::new(format!("audit task join: {e}"))),
                    }
                },
                Message::ChartSignalsLoaded,
            );

            return iced::Task::batch([markers_task, signals_task]);
        }

        if let Some(tx_id) = tx_id {
            let ledger = Arc::clone(&self.ledger);
            let rt_handle = self.rt_handle.clone();
            iced::Task::perform(
                async move {
                    // Bridge the iced thread's lack-of-runtime to the
                    // side-thread tokio runtime: spawn the audit reads on
                    // `rt_handle` and await its `JoinHandle`. The handle
                    // is `Send + 'static`, so the closure is `iced::Task`
                    // friendly.
                    //
                    // Per [Design § Q4](../../../../spec/features/journal-transactions-metadata.md#q4--sequential-await-not-tokiojoin),
                    // the chain is sequential — metadata first (with a
                    // `None` short-circuit on stale clicks), then entries.
                    // Per [Design § Q6](../../../../spec/features/journal-transactions-metadata.md#q6--partial-failure-semantics-any-err--error-state),
                    // every non-happy outcome maps to a single
                    // `Err(SmolStr)` payload that the modal renders as
                    // `TAPE_AUDIT_MODAL_ERROR_PREFIX + msg`.
                    let join = rt_handle.spawn(async move {
                        let tx_id_str = tx_id.as_str();
                        let meta =
                            match audit::query::journal_transaction_metadata(&ledger, tx_id_str)
                                .await
                            {
                                Ok(Some(m)) => m,
                                Ok(None) => {
                                    return Err(SmolStr::new(format!(
                                        "{TAPE_AUDIT_MODAL_ERROR_PREFIX}unknown transaction"
                                    )));
                                }
                                Err(e) => {
                                    return Err(SmolStr::new(format!(
                                        "{TAPE_AUDIT_MODAL_ERROR_PREFIX}{e}"
                                    )));
                                }
                            };
                        match audit::query::journal_entries_for_transaction(&ledger, tx_id_str)
                            .await
                        {
                            Ok(entries) => Ok(JournalTransactionView {
                                tx_id: meta.transaction_id,
                                ts: meta.ts,
                                description: meta.description,
                                strategy_id: meta.strategy_id,
                                entries,
                            }),
                            Err(e) => {
                                Err(SmolStr::new(format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}{e}")))
                            }
                        }
                    });
                    match join.await {
                        Ok(result) => result,
                        Err(e) => Err(SmolStr::new(format!(
                            "{TAPE_AUDIT_MODAL_ERROR_PREFIX}audit task join: {e}"
                        ))),
                    }
                },
                Message::TapeAuditEntriesLoaded,
            )
        } else if let Some(id) = select_strategy_id {
            // Phase 4 (T1811) — chain the equity-curve fetch + the
            // optional screen-switch. The fetched series is
            // `downsample(SPARKLINE_POINT_CAP)`-d at fetch time
            // (Q9 — cap-and-downsample at fetch, not at view time).
            let ledger = Arc::clone(&self.ledger);
            let rt_handle = self.rt_handle.clone();
            let id_for_task = id.clone();
            let id_for_msg = id.clone();
            let fetch_task = iced::Task::perform(
                async move {
                    let join = rt_handle.spawn(async move {
                        // `since`: 24h ago — Q9 keeps the cockpit
                        // surface offline-friendly without forcing
                        // a strategy-load timestamp lookup.
                        let now = trading_core::Timestamp::now();
                        let since_ms = now.unix_millis().saturating_sub(24 * 60 * 60 * 1_000);
                        let since = trading_core::Timestamp::new(
                            time::OffsetDateTime::from_unix_timestamp_nanos(
                                i128::from(since_ms) * 1_000_000,
                            )
                            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
                        );
                        audit::query::equity_curve_for_strategy(&ledger, id_for_task, since, None)
                            .await
                            .map(|s| s.downsample(ui::theme::layout::SPARKLINE_POINT_CAP))
                            .map_err(|e| SmolStr::new(format!("{e}")))
                    });
                    match join.await {
                        Ok(result) => result,
                        Err(e) => Err(SmolStr::new(format!("audit task join: {e}"))),
                    }
                },
                move |res| Message::StrategyEquityRefreshed(id_for_msg.clone(), res),
            );
            if cross_link_strategies {
                iced::Task::batch(vec![
                    iced::Task::done(Message::SwitchScreen(Screen::Strategies)),
                    fetch_task,
                ])
            } else {
                fetch_task
            }
        } else if cross_link_strategies {
            iced::Task::done(Message::SwitchScreen(Screen::Strategies))
        } else if let Some(run_cfg) = lab_run_cfg {
            // T-D-N9: LabRunRequested with a valid (strategy, pair) selection.
            // Spawn the real backtest engine call and post LabRunCompleted back
            // to the iced update loop.
            let (_, cancel_recv) = ui::lab::runner::cancellation_pair();
            ui::lab::runner::spawn_lab_run(Some(&self.rt_handle), run_cfg, cancel_recv)
        } else {
            iced::Task::none()
        }
    }

    /// Subscribe to the real bus — the entire point of `cockpit_live`.
    /// `ui::live::subscription` already exists (T32) and batches the
    /// six core channels + three v0.5 strategy lifecycle channels +
    /// the T1508 `MarketHealth` channel.
    ///
    /// T1509 additions:
    /// - The `MarketHealth` recipe is already included in
    ///   `ui::live::subscription` (T1508 shipped it in `live.rs`).
    /// - A 1 Hz `time::every` subscription emits `Message::ServerTimeTick`
    ///   so the status-bar server-time field advances each second.
    ///
    /// When the tape-row → audit modal is open, batch in an
    /// `iced::event::listen_with` recipe so `Esc` closes the modal
    /// (Q6 — modal-open-gated subscription). Other keys are not
    /// consumed; the live cockpit has no general keyboard navigation
    /// today, so nothing leaks to the tape beneath.
    fn subscription(&self) -> iced::Subscription<Message> {
        let bus_sub = ui::live::subscription(Arc::clone(&self.bus));

        // 1 Hz server-time tick — drives the status-bar clock (T1509).
        // Uses `ServerTimeRecipe` (tokio interval via tokio_stream) rather
        // than `iced::time::every` which requires iced's `tokio` feature flag.
        let time_sub = iced::advanced::subscription::from_recipe(ServerTimeRecipe);

        if self.cockpit.tape_audit_modal.is_some() {
            iced::Subscription::batch(vec![
                bus_sub,
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
            iced::Subscription::batch(vec![bus_sub, time_sub])
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Phase 2 — both bins compose the same shell so screenshots align
        // pixel-for-pixel. (T1603 / T1611.)
        let main_column: Element<'_, Message> = shell::view(&self.cockpit, ThemeMode::Dark);

        // Render the modal as a `Stack` overlay only when the modal is
        // open (tape-row-audit-modal Q1). The `Stack` base layer is
        // `main_column` (body + status bar), so the modal scrim overlays
        // both — the status bar stays visible behind the backdrop.
        // When closed, return `main_column` directly so the cockpit's
        // iced widget tree is byte-identical to the pre-modal world —
        // existing `panel_snapshots__*` stay green by construction
        // (V7 / R11).
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
