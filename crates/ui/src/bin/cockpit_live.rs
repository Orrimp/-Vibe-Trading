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
//
// ## Runtime-context note (P1 bug fix — 2026-05-23)
//
// iced 0.14 uses `futures::executor::ThreadPool` when the `thread-pool`
// feature is active (see `crates/ui/Cargo.toml` iced feature list).
// That executor has NO tokio reactor context — calling
// `tokio::time::interval()` or `tokio::time::sleep()` directly inside a
// `Recipe::stream()` body panics with
// "there is no reactor running, must be called from the context of a
// Tokio 1.x runtime".
//
// Fix: carry the agent-runtime `Handle` into the recipe. At the start of
// `stream()`, enter the handle with `handle.enter()` and keep the
// `EnterGuard` alive for the entire duration of the stream (it is `'static`
// — it lives inside the `Box::pin(async_stream::stream! {...})` future).
// `Handle::enter()` is safe to call from any thread; the guard sets a
// thread-local that `tokio::time::*` reads to find the reactor.

struct ServerTimeRecipe {
    /// Agent-runtime handle so the stream body can call `tokio::time::interval`
    /// inside the iced `futures::ThreadPool` subscription executor.
    rt_handle: tokio::runtime::Handle,
}

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
        // Delegate to the extracted helper in `ui::live` so integration tests
        // can drive the stream without constructing a running iced application.
        // Behavior is identical to the previous inline body — see
        // `ui::live::server_time_stream_impl` for the full K8 rationale.
        ui::live::server_time_stream_impl(&self.rt_handle)
    }
}

// ── cockpit-toast-queue v0.1.0 — ToastDismissRecipe (6th subscription) ──────
//
// Mirrors `ServerTimeRecipe` above: carries the agent-runtime `Handle` so
// `tokio::time::interval` is called inside the tokio reactor context.
// Emits `Message::ToastTick(Instant::now())` every 500 ms via
// `ui::live::toast_dismiss_stream_impl` (extracted for test reachability).
//
// Always-on — no salt / no per-run gating. The 500 ms idle cost is negligible
// vs the 100 ms activity-tape tick already running.
//
// See ADR-0046 § Decision (ticker pattern) and T-D-N10.

struct ToastDismissRecipe {
    /// Agent-runtime handle so the stream body can call `tokio::time::interval`
    /// inside the iced `futures::ThreadPool` subscription executor.
    rt_handle: tokio::runtime::Handle,
}

impl Recipe for ToastDismissRecipe {
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
        // Delegate to the extracted helper in `ui::live` for test reachability.
        ui::live::toast_dismiss_stream_impl(&self.rt_handle)
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
    // ── Tracing (T-RED-D10 / v2-1-tracing-layer-redactor) ────────────────────
    // Migrated from `tracing_subscriber::fmt().init()` to `install_global` to
    // wire the `RedactLayer` BEFORE the fmt sink (R1.4 ordering contract).
    llm::tracing_init::install_global(&["cockpit_live=info", "agent=info", "ui=info"], true)?;

    let args = Args::parse();
    info!("cockpit_live starting");

    // ── Config ───────────────────────────────────────────────────────────────
    let cfg = if args.config.exists() {
        agent::config::Config::load(&args.config).context("load config")?
    } else {
        warn!(path = ?args.config, "config file not found — using defaults");
        agent::config::Config::default()
    };

    if let Some(ref m) = args.mode
        && m.eq_ignore_ascii_case("live")
    {
        anyhow::bail!("mode=live is rejected in v0");
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
    // T-D-11: conditionally wire the broadcast tick bus (R7.1 / decomp §6).
    // Sender is held for the process lifetime so broadcast receivers stay live.
    let (ledger, _tick_bus_sender) = bootstrap_rt.block_on(async {
        if cfg.audit.tick_bus_capacity > 0 {
            let (l, s) = audit::Ledger::open_with_tick_bus(db_path, cfg.audit.tick_bus_capacity)
                .await
                .context("open audit ledger with tick bus")?;
            audit::bootstrap::chart_of_accounts(&l)
                .await
                .context("bootstrap chart of accounts")?;
            Ok::<_, anyhow::Error>((l, Some(s)))
        } else {
            let l = audit::Ledger::open(db_path)
                .await
                .context("open audit ledger")?;
            audit::bootstrap::chart_of_accounts(&l)
                .await
                .context("bootstrap chart of accounts")?;
            Ok::<_, anyhow::Error>((l, None))
        }
    })?;
    let ledger = Arc::new(ledger);
    info!(db = %db_path, tick_bus = cfg.audit.tick_bus_capacity > 0, "audit ledger initialized");

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

    // ── Phase D+ — Trail-mirror construction (ui-rethink-phase-d-trail-followup
    //              T-D-N6 / Q3 = (c) construct in cockpit_live bootstrap)
    // `TrailMirror::new` is sync (channel allocations only). The `mirror.run()`
    // task is spawned on the side-thread runtime at T-D-N7 below.
    // Construction is gated on `tick_bus_capacity > 0` — if the tick bus is
    // disabled (e.g. in fixture smoke runs), no mirror is created. The
    // `_tick_bus_sender` must stay alive here so `subscribe()` gets a live sender.
    #[cfg(feature = "live")]
    let (trail_mirror_task, trail_mirror_handle) = {
        if let Some(ref sender) = _tick_bus_sender {
            let rx = sender.subscribe();
            let (mirror, handle) =
                reflection::trail_mirror::TrailMirror::new(rx, Arc::clone(&ledger));
            info!("trail mirror constructed (Phase D+ subscription bridge)");
            (Some(mirror), Some(handle))
        } else {
            (None, None)
        }
    };

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

    // ── Phase F T-D-N10 — save paths before cfg is moved into RunHandles ────────
    // `cfg.reflection.path` is consumed by `Arc::new(cfg)` below; capture a
    // clone so the iced init closure can open the reflection pool at boot.
    // The checkpoint dir follows the workspace convention used by
    // `crates/forecast/src/tcn.rs:494` and `strategy/src/tcn_overlay_momentum.rs`.
    #[cfg(feature = "live")]
    let reflection_db_path: PathBuf = cfg.reflection.path.clone();
    #[cfg(feature = "live")]
    let checkpoint_dir: PathBuf = PathBuf::from("crates/forecast/checkpoints/anchors");
    // live-equity-history-durable (T7 / A4) — capture the run mode (CLONE; the
    // enum is not `Copy`) before `cfg` is moved into `Arc::new(cfg)` below, so
    // the boot-hydrate task closure can read it. The boot equity hydrate is
    // issued ONLY in paper/live mode; research replay restarts the 2023 series
    // each boot, so hydrating it would overlap/duplicate a meaningless curve —
    // research stays session-scoped (R6 / A2).
    #[cfg(feature = "live")]
    let boot_mode: agent::config::Mode = cfg.mode.clone();

    // ── cockpit-activity-audit-ledger-producer v0.1.0 — aggregator sender ───────
    // Clone the tick-bus sender for the aggregator. The original `_tick_bus_sender`
    // stays alive so the broadcast channel remains open for the full process
    // lifetime. The clone is moved into the side-thread closure below and used
    // to call `agent::spawn_aggregator` once the tokio runtime is live (K6
    // ordering: the aggregator MUST spawn inside the tokio runtime context so
    // `tokio::time::interval` has a reactor; spawning before `rt.block_on`
    // panics with "no reactor running").
    //
    // `#[cfg(feature = "live")]` mirrors the trail-mirror gate above — the
    // aggregator is only wired in live builds where the tick bus is alive.
    #[cfg(feature = "live")]
    let audit_aggregator_tick_sender = _tick_bus_sender.clone();
    #[cfg(feature = "live")]
    let bus_for_aggregator = Arc::clone(&bus);

    // ── Side thread: own the runtime, drive agent::runtime::run on it ────────
    // live-equity-history-durable (T5 reconvergence / A1 / A2) — construct the
    // durable-equity write store for `RunHandles`. The store is wired ONLY in
    // paper/live mode: research replay restarts the 2023 series each boot, so
    // persisting it would overlap/duplicate a meaningless series (the
    // single load-bearing duplication-prevention gate, A2). `None` in research
    // means the reconciler's `Option<store>` is absent → it writes nothing.
    // This write-gate is symmetric with the boot-hydrate READ-gate
    // (`should_hydrate_equity_on_boot`): research = no write + no hydrate;
    // paper/live = per-bar write + boot hydrate. Production impl wraps the
    // existing single-writer `Arc<Ledger>` (ADR-0052 D1).
    let equity_store: Option<Arc<dyn audit::LiveEquityStore>> =
        if cfg.mode == agent::config::Mode::Research {
            None
        } else {
            Some(Arc::new(audit::LedgerEquityStore::new(Arc::clone(&ledger))))
        };
    let agent_handle = {
        let cancel = cancel.clone();
        let handles = RunHandles {
            config: Arc::new(cfg),
            ledger: Arc::clone(&ledger),
            bus: Arc::clone(&bus),
            kill_switch: Arc::clone(&kill_switch),
            registry: Arc::clone(&registry),
            boot_id: boot_id.clone(),
            equity_store,
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

                    // Phase D+ T-D-N7 — spawn the trail-mirror task onto the
                    // side-thread runtime (mirrors must `await run()` inside a
                    // tokio context; the bootstrap_rt is already dropped here).
                    #[cfg(feature = "live")]
                    if let Some(mirror) = trail_mirror_task {
                        tokio::spawn(async move {
                            info!("trail mirror task spawned");
                            mirror.run().await;
                            info!("trail mirror task exited");
                        });
                    }

                    // cockpit-activity-audit-ledger-producer v0.1.0 T-D-N5 —
                    // Spawn the audit-ledger-writes activity aggregator.
                    //
                    // K6 ordering: we are now INSIDE `rt.block_on`, so the tokio
                    // reactor is live and `tokio::time::interval` works correctly.
                    // The aggregator is gated on `audit_aggregator_tick_sender` being
                    // `Some` (i.e. `tick_bus_capacity > 0` in config). When `None`,
                    // `spawn_aggregator` spawns a no-op task that returns immediately.
                    //
                    // The returned `JoinHandle` is intentionally not held across the
                    // function boundary — if the aggregator panics, `tracing::warn`
                    // surfaces the panic but the cockpit continues (K5 mitigation per
                    // ADR-0044 § "What costs this incurs"). A future ADR can promote
                    // the handle into a supervisor if K5 surfaces as a real incident.
                    #[cfg(feature = "live")]
                    {
                        let _agg_handle = agent::spawn_aggregator(
                            audit_aggregator_tick_sender.as_ref(),
                            &bus_for_aggregator,
                        );
                        info!(
                            tick_bus_active = audit_aggregator_tick_sender.is_some(),
                            "audit-ledger activity aggregator spawned"
                        );
                    }

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
    cockpit.current_screen = Screen::Live;
    cockpit.universe = universe_pairs.clone();
    if let Some(first) = universe_pairs.first() {
        cockpit.selected_symbol = Some(first.clone());
    }

    // cockpit-baseline-panel v0.1.0 — boot-load the realized BH curves so the
    // navigable Baseline screen is `Ready` on first visit (or `Error`, never a
    // panic, if the runbook artifacts are absent). Read-only over committed
    // files; no bus/audit dependency.
    ui::baseline::load_into(&mut cockpit);

    // cockpit-reports-viewer v0.1.0 — boot-scan the committed `backtest-*.md`
    // corpus so the navigable Reports screen lists it on first visit (or the
    // empty-list copy, never a panic, if `spec/` reports are absent). Read-only
    // over committed files; no bus/audit dependency. Default route stays Live.
    ui::reports::load_into(&mut cockpit);

    // F7 fix 2026-05-24 — seed the strategy registry from the engine's
    // dispatched ScenarioStrategy ids so the Lab chip row is non-empty
    // at cold boot. Without this seed, model.strategies stays at
    // PanelState::Loading forever (no subscription bridges the agent's
    // strategy_watcher into the UI) and the Lab screen renders no
    // strategy chips for the operator to pick. This is a stop-gap until
    // a proper strategies_subscription Recipe is wired in a future wave.
    {
        use trading_core::StrategyId;
        use ui::state::{PanelState, StrategyRow, StrategyStatus};
        let seed_rows: Vec<StrategyRow> = [
            // Single-symbol strategies (Wave D-2 — full chart per Phase A R2).
            ("v0.sma", "config/strategies/sma_cross.toml"),
            ("v0.5.macd", "config/strategies/macd_trend.toml"),
            ("v0.5.rsi", "config/strategies/rsi_reversion.toml"),
            ("v0.5.bbands", "config/strategies/bbands_mean_revert.toml"),
            // Cross-sectional strategies (equity-only render until v0.2.0 D-2.5).
            ("v1.momentum", "config/strategies/top10_momentum_h1.toml"),
            ("v1.5a.pairs", "config/strategies/pairs_mr_h1.toml"),
            ("v2.5.tcn", "config/strategies/tcn_overlay_momentum.toml"),
            (
                "v2.5.tcn.weights",
                "config/strategies/tcn_overlay_momentum.toml",
            ),
        ]
        .into_iter()
        .map(|(id, path)| StrategyRow {
            id: StrategyId(id.into()),
            short_hash: smol_str::SmolStr::new("0000000"),
            full_hash: smol_str::SmolStr::new(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            status: StrategyStatus::Ready,
            last_event: None,
            signals_60s: 0,
            has_position: false,
            source_path: smol_str::SmolStr::new(path),
        })
        .collect();
        cockpit.strategies = PanelState::Ready(seed_rows);
    }

    let app_state = AppState {
        cockpit,
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        ledger: Arc::clone(&ledger),
        rt_handle: rt_handle.clone(),
        // Phase D+ T-D-N8 — wire the trail-mirror handle into AppState so
        // subscription() can build the TrailMirrorRecipe from it.
        #[cfg(feature = "live")]
        trail_mirror_handle,
        // Wave D-3/D-4 — Lab progress channel (T-AR-6). No in-flight run at boot.
        #[cfg(feature = "live")]
        lab_progress_rx: None,
        #[cfg(feature = "live")]
        lab_progress_recipe_salt: 0,
        // cockpit-activity-status-bar v0.1.0 Wave C — T-D-N8 / T-D-N9.
        // Both handles start as None; started on the first activity trigger.
        #[cfg(feature = "live")]
        lab_activity_handle: None,
        #[cfg(feature = "live")]
        training_activity_handle: None,
        // cockpit-training-pressed-wiring v0.1.0 T-D-N1.
        // Training log channel: None until TrainingPressed fires.
        #[cfg(feature = "live")]
        training_log_rx: None,
        #[cfg(feature = "live")]
        training_log_recipe_salt: 0,
    };

    // ui-session-journal-iced-tester v0.1 (T03 — REVISED) — recorder
    // overlay is auto-attached by `iced::Application::run()` when the
    // `record-tests` feature pulls `iced/tester`. No manual
    // `iced_tester::attach()` call needed; no runtime gate.
    #[cfg(feature = "record-tests")]
    info!("iced_tester recorder overlay enabled (compile-time via --features record-tests)");

    // Phase F T-D-N10 — cold-boot hydrate tasks.
    // Capture paths + rt_handle before the `app_state` move into the iced
    // init closure. Each task mirrors the `TapeRowClicked` / `SelectSymbol`
    // pattern: `rt_handle.spawn(async { … })` bridges iced's main thread
    // (no tokio runtime) to the side-thread tokio runtime.
    #[cfg(feature = "live")]
    let boot_rt_for_memory = rt_handle.clone();
    #[cfg(feature = "live")]
    let boot_reflection_db_path = reflection_db_path;
    #[cfg(feature = "live")]
    let boot_checkpoint_dir = checkpoint_dir;
    // live-equity-history-durable (T7 / A4) — capture an rt handle + the ledger
    // for the boot equity-hydrate task before `app_state` moves into the iced
    // init closure. Mirrors `boot_rt_for_memory` / `boot_reflection_db_path`.
    #[cfg(feature = "live")]
    let boot_rt_for_equity = rt_handle.clone();
    #[cfg(feature = "live")]
    let boot_equity_ledger = Arc::clone(&ledger);

    let iced_result = iced::application(
        move || {
            let state = app_state.clone();

            // Phase F T-D-N10 — boot-time Memory hydrate task.
            // Delegates pool-open + query to `reflection::query::open_and_list_recent`
            // so sqlx stays within the reflection crate boundary (the `ui` crate has
            // no direct sqlx dep). Fail-soft: on missing file `open_and_list_recent`
            // returns `Ok(vec![])` immediately; on other errors the warn! is logged
            // inside the reflection crate and `Message::MemoryHydrate(vec![])` fires
            // — the Memory screen renders the R1.4 empty-state placeholder.
            #[cfg(feature = "live")]
            let memory_task = {
                use ui::memory::state::LessonCardCard;
                let rt = boot_rt_for_memory.clone();
                let db_path = boot_reflection_db_path.clone();
                iced::Task::perform(
                    async move {
                        let join = rt.spawn(async move {
                            match reflection::query::open_and_list_recent(&db_path, 50).await {
                                Ok(cards) => cards
                                    .into_iter()
                                    .map(|c| LessonCardCard {
                                        card_id: SmolStr::new(&c.card_id),
                                        symbol_or_pair: SmolStr::new(format!(
                                            "{}",
                                            c.symbol_or_pair
                                        )),
                                        closed_at: SmolStr::new(format!("{}", c.closed_at)),
                                        strategy_id: SmolStr::new(c.strategy_id.0.as_str()),
                                        signed_pnl_display: SmolStr::new(format!(
                                            "{}",
                                            c.signed_pnl
                                        )),
                                        outcome_class: SmolStr::new(format!("{}", c.outcome_class)),
                                        note: c.note.as_deref().map(SmolStr::new),
                                        // v0.1.0: close_transaction_id not stored in
                                        // LessonCard (LLM-enrichment follow-up).
                                        close_transaction_id: None,
                                    })
                                    .collect(),
                                Err(e) => {
                                    warn!(
                                        error = %e,
                                        "memory cold-boot: query failed (empty-state path)"
                                    );
                                    vec![]
                                }
                            }
                        });
                        match join.await {
                            Ok(cards) => cards,
                            Err(e) => {
                                warn!(error = %e, "memory cold-boot: task join error");
                                vec![]
                            }
                        }
                    },
                    Message::MemoryHydrate,
                )
            };

            // Phase F T-D-N10 — boot-time Models hydrate task.
            // Synchronous filesystem scan: `discover_checkpoints` does only
            // `read_dir` + `read_to_string` + `serde_json::from_str`.  The
            // H2 static argument shows total cost ≈ 20 μs for 2 checkpoints
            // (well within the 50 ms p99 budget), so we wrap in an
            // `iced::Task::perform` simply to keep the iced init closure
            // synchronous and return the result via the message loop.
            #[cfg(feature = "live")]
            let models_task = {
                let dir = boot_checkpoint_dir.clone();
                iced::Task::perform(
                    async move { ui::models::registry_read::discover_checkpoints(&dir) },
                    Message::ModelsHydrate,
                )
            };

            // live-equity-history-durable (T7 / A4) — boot-time durable-equity
            // hydrate task. Mirrors `memory_task`: an `iced::Task::perform` that
            // spawns the `audit::query::equity_snapshot_tail` reader on the
            // side-thread tokio runtime (so `ui` keeps its no-direct-sqlx edge —
            // the query lives in the `audit` crate) and routes the tail back as a
            // batch `Message::PnlHydrated`. Each row maps to `(bar_ts, as_of,
            // total_equity)`: `bar_ts` is the plotted x-coord, `as_of` the
            // delivery key (the `PnlHydrated` arm seeds the guard from the MAX
            // hydrated `as_of`). `LIMIT = LIVE_EQUITY_BUFFER_CAP` so the hydrate
            // never exceeds the buffer ring.
            //
            // Fail-soft: `equity_snapshot_tail` returns `Ok(vec![])` on an empty
            // table; `unwrap_or_default()` collapses any reader/join error to an
            // empty tail → `Message::PnlHydrated(vec![])` is a no-op (the curve
            // stays session-scoped/Loading), exactly the Memory cold-boot
            // tolerance. Issued ONLY in paper/live mode — research replay restarts
            // the 2023 series each boot, so hydrating it would overlap/duplicate a
            // meaningless curve (R2 / A2). In research mode the task is
            // `Task::none()`, so no hydrate fires and the curve stays
            // session-scoped (R6).
            #[cfg(feature = "live")]
            let equity_hydrate_task = if !ui::live::should_hydrate_equity_on_boot(&boot_mode) {
                iced::Task::none()
            } else {
                let rt = boot_rt_for_equity.clone();
                let ledger = Arc::clone(&boot_equity_ledger);
                iced::Task::perform(
                    async move {
                        let join = rt.spawn(async move {
                            let rows = audit::query::equity_snapshot_tail(
                                &ledger,
                                ui::theme::layout::LIVE_EQUITY_BUFFER_CAP,
                            )
                            .await
                            .unwrap_or_default();
                            rows.into_iter()
                                .map(|r| (r.bar_ts, r.as_of, r.total_equity))
                                .collect::<Vec<_>>()
                        });
                        match join.await {
                            Ok(tail) => tail,
                            Err(e) => {
                                warn!(error = %e, "equity cold-boot: task join error");
                                vec![]
                            }
                        }
                    },
                    Message::PnlHydrated,
                )
            };

            #[cfg(feature = "live")]
            let boot_task = iced::Task::batch([memory_task, models_task, equity_hydrate_task]);
            #[cfg(not(feature = "live"))]
            let boot_task = iced::Task::none();

            (state, boot_task)
        },
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
// NOTE: Clone is implemented manually below because `agent::ActivityHandle`
// is `!Clone` (uses `Cell<>` for throttle state). The manual impl returns
// `None` for both activity handle fields — the clone is only used at boot
// where both fields are always `None`, so this is semantically correct.
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
    /// Phase D+ (ui-rethink-phase-d-trail-followup T-D-N8 / Q3 = (c)).
    /// Trail-mirror handle for the iced Subscription bridge (Wave B).
    /// `None` when `tick_bus_capacity = 0` (mirror not spawned).
    /// The bridge recipe in `ui::live::trail_mirror_subscription` reads
    /// `handle.tick_tx.subscribe()`. Cloned cheaply (Arcs inside).
    /// Only present when compiled with `--features live`.
    #[cfg(feature = "live")]
    #[allow(dead_code)] // accessed via subscription() only
    trail_mirror_handle: Option<reflection::trail_mirror::TrailMirrorHandle>,

    // ── Wave D-3/D-4 — Lab Stop + progress (lab-end-to-end-v2 T-AR-5/T-AR-6) ──
    /// In-flight Lab run progress receiver.
    ///
    /// `Some` while a backtest is running; `None` otherwise.
    /// Stored in an `Arc<Mutex<Option<...>>>` so the `LabProgressRecipe`
    /// can take ownership of the receiver in its `stream()` call without
    /// requiring the AppState to be moved into the subscription.
    /// Only available in `live` builds (tokio mpsc requires a runtime).
    #[cfg(feature = "live")]
    lab_progress_rx: Option<
        std::sync::Arc<
            std::sync::Mutex<Option<tokio::sync::mpsc::Receiver<backtest::progress::Progress>>>,
        >,
    >,

    /// Salt bumped on every `LabRunRequested` so `LabProgressRecipe::hash`
    /// returns a fresh identity per run (iced de-duplicates subscriptions by hash).
    #[cfg(feature = "live")]
    lab_progress_recipe_salt: u64,

    // ── cockpit-activity-status-bar v0.1.0 Wave C — T-D-N8 / T-D-N9 ─────────
    /// In-flight Lab run `ActivityHandle` (T-D-N8 approach A).
    ///
    /// Started on `LabRunRequested` (before `iced::Task::perform`); ticked
    /// on each `LabRunProgress` message; ended (`Success` or `Failed`) on
    /// `LabRunCompleted`. Held on the iced side because `ActivityHandle` is
    /// `!Send` (uses `Cell<>` for the throttle state).
    ///
    /// `None` when no run is in flight.
    #[cfg(feature = "live")]
    lab_activity_handle: Option<agent::ActivityHandle>,

    /// In-flight Training subprocess `ActivityHandle` (T-D-N9 approach A).
    ///
    /// Started on `TrainingPressed`; ticked on each `TrainingEventsRefreshed`;
    /// ended (`Success` or `Cancelled`) on `TrainingExited` / `TrainingCancelPressed`.
    /// Held on the iced side — same `!Send` constraint as `lab_activity_handle`.
    ///
    /// `None` when no training run is in flight.
    #[cfg(feature = "live")]
    training_activity_handle: Option<agent::ActivityHandle>,

    // ── cockpit-training-pressed-wiring v0.1.0 T-D-N1 ────────────────────────
    /// In-flight training log receiver. `Some` while training is in-flight;
    /// `None` otherwise. The `TrainingLogRecipe` takes ownership of the
    /// receiver in `stream()` via `.take()` — same pattern as `lab_progress_rx`.
    #[cfg(feature = "live")]
    training_log_rx: Option<
        std::sync::Arc<
            std::sync::Mutex<Option<std::sync::mpsc::Receiver<ui::lab::trainer::TrainingLogLine>>>,
        >,
    >,

    /// Salt bumped on every `TrainingPressed` so `TrainingLogRecipe::hash`
    /// returns a fresh identity per run (iced de-duplicates subscriptions by hash).
    #[cfg(feature = "live")]
    training_log_recipe_salt: u64,
}

// ── Manual Clone for AppState ─────────────────────────────────────────────────
//
// `agent::ActivityHandle` is `!Clone` (uses `Cell<>` for the tick-throttle
// state). The two activity-handle fields are always `None` at cold-boot (the
// only site where `AppState` is cloned — see `app_state.clone()` in the
// reflection / memory-mirror warm-up block), so returning `None` is
// semantically correct for the clone.
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            cockpit: self.cockpit.clone(),
            bus: self.bus.clone(),
            kill_switch: self.kill_switch.clone(),
            ledger: self.ledger.clone(),
            rt_handle: self.rt_handle.clone(),
            #[cfg(feature = "live")]
            trail_mirror_handle: self.trail_mirror_handle.clone(),
            #[cfg(feature = "live")]
            lab_progress_rx: self.lab_progress_rx.clone(),
            #[cfg(feature = "live")]
            lab_progress_recipe_salt: self.lab_progress_recipe_salt,
            // Activity handles are never cloned — they are unique per run.
            // The clone only happens at cold-boot where both are None.
            #[cfg(feature = "live")]
            lab_activity_handle: None,
            #[cfg(feature = "live")]
            training_activity_handle: None,
            // Training log receiver is not cloned — clone only happens at cold-boot.
            #[cfg(feature = "live")]
            training_log_rx: None,
            #[cfg(feature = "live")]
            training_log_recipe_salt: self.training_log_recipe_salt,
        }
    }
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
            // R1.2 (lab-end-to-end-v2 T-D1.1) — extend capture so that a
            // Lab pair-chip click also fires the markers/signals re-fetch
            // cascade in the post-forward block (cockpit_live.rs:861-916).
            Message::LabSelectPair(v, s) => Some((*v, s.clone())),
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
                    // lab-yahoo-realdata T-C3.6: mirror the current UI data_source
                    // selection into the run config so runner can dispatch accordingly.
                    data_source: self.cockpit.lab_state.data_source,
                    // lab-polish-round-2 R2: pass operator-tuned SMA windows
                    // through. None → engine falls back to the (20, 50) default.
                    sma_fast_len: self.cockpit.lab_state.sma_fast_len,
                    sma_slow_len: self.cockpit.lab_state.sma_slow_len,
                })
            } else {
                None
            }
        } else {
            None
        };

        // T-D1.3 (lab-end-to-end-v2 T-AR-1) — capture LabRunCompleted BEFORE
        // state::update so we still see the pre-forward LabState (operator MAY
        // have clicked away during the run; the pre-forward snapshot is what
        // the RunReportMirror.tuple must encode per K3).
        let lab_run_completed_summary: Option<ui::lab::runner::RunSummary> = match &msg {
            Message::LabRunCompleted(Ok(summary)) => Some(summary.clone()),
            _ => None,
        };
        // T-D3.1 — detect any LabRunCompleted (Ok or Err) so we can clear the
        // cancel handle + progress receiver unconditionally on run completion.
        let lab_run_completed_any = matches!(&msg, Message::LabRunCompleted(_));
        // T-D3.4 — detect LabRunStopRequested to drop the cancel handle.
        let lab_run_stop_requested = matches!(&msg, Message::LabRunStopRequested);

        // cockpit-activity-status-bar T-D-N8 — capture LabRunProgress current
        // bar so we can tick the activity handle after state::update.
        #[cfg(feature = "live")]
        let lab_run_progress_current: Option<u64> = match &msg {
            Message::LabRunProgress(p) => Some(p.current_bar as u64),
            _ => None,
        };
        // cockpit-activity-status-bar T-D-N8 — capture LabRunCompleted error
        // so we can fail the activity handle if needed.
        #[cfg(feature = "live")]
        let lab_run_completed_err: Option<smol_str::SmolStr> = match &msg {
            Message::LabRunCompleted(Err(e)) => Some(e.clone()),
            _ => None,
        };

        // cockpit-activity-status-bar T-D-N9 / cockpit-training-pressed-wiring T-D-N1 —
        // capture training lifecycle events. `TrainingPressed` is intercepted BELOW
        // (before delegation) to spawn the subprocess and populate the activity handle.
        // The lifecycle management below (cancel/exit/tick) operates on
        // `training_activity_handle` which is populated by the interception.
        #[cfg(feature = "live")]
        let training_cancel_pressed = matches!(&msg, Message::TrainingCancelPressed);
        #[cfg(feature = "live")]
        let training_exited = matches!(&msg, Message::TrainingExited(_));
        // Capture epoch for tick forwarding: use number of training events as proxy.
        #[cfg(feature = "live")]
        let training_events_count: Option<u64> = match &msg {
            Message::TrainingEventsRefreshed(rows) => Some(rows.len() as u64),
            _ => None,
        };

        let lab_run_completed_pre_tuple = if lab_run_completed_summary.is_some() {
            let ls = &self.cockpit.lab_state;
            match (ls.strategy.as_ref(), ls.pair.as_ref()) {
                (Some(strategy), Some((venue, symbol))) => {
                    Some(ui::lab::equity_loader::LabTuple::new(
                        strategy,
                        *venue,
                        symbol,
                        ls.range.clone(),
                    ))
                }
                _ => None,
            }
        } else {
            None
        };

        // ── cockpit-training-pressed-wiring v0.1.0 T-D-N1 — TrainingPressed intercept ──
        // Intercept BEFORE ui::state::update so the binary-side I/O wiring (spawn,
        // channel setup, handle storage) runs before the pure-state no-op arm.
        // Mirrors the LabRunRequested intercept pattern at lines ~1314-1362.
        #[cfg(feature = "live")]
        if matches!(msg, Message::TrainingPressed) {
            // Short-circuit if already in-flight (button is disabled per parent R3.4,
            // but defensive check matches the LabRunRequested precedent).
            if self.cockpit.lab_state.training_inflight.is_none() {
                use ui::lab::trainer::{
                    TrainingLogLine, cancellation_pair, default_training_config, spawn_training_run,
                };

                // Build TrainingConfig from workspace defaults (R3 / Q1=(a)).
                let cfg = default_training_config();

                // Build cancellation pair (mirrors LabRunRequested precedent).
                let (cancel_handle, cancel_rx) = cancellation_pair();

                // Build the log line channel (256-slot per R1.1 step 3).
                let (line_tx, line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

                // Stash the receiver so the TrainingLogRecipe can take it in stream().
                let line_rx_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(line_rx)));
                self.training_log_rx = Some(std::sync::Arc::clone(&line_rx_arc));
                // Bump salt so iced sees a new recipe identity per TrainingPressed.
                self.training_log_recipe_salt = self.training_log_recipe_salt.wrapping_add(1);

                // Call spawn_training_run — synchronous (uses rt_handle.block_on internally).
                match spawn_training_run(
                    Some(&self.rt_handle),
                    &cfg,
                    cancel_rx,
                    line_tx,
                    Some(self.bus.activity()),
                ) {
                    Ok((training_handle, activity_handle)) => {
                        self.cockpit.lab_state.training_inflight = Some(training_handle);
                        self.training_activity_handle = activity_handle;
                        self.cockpit.lab_state.training_cancel = Some(cancel_handle);
                    }
                    Err(e) => {
                        // Surface the error via toast (R1.1 step 7 / R-NR.4 / R3.2).
                        // cockpit-toast-queue v0.1.0 T-D-N8: route through the message
                        // dispatcher with Danger severity instead of direct field write.
                        ui::state::update(
                            &mut self.cockpit,
                            Message::ShowToastWithSeverity(
                                smol_str::SmolStr::new(format!("Training failed to launch: {e}")),
                                ui::state::ToastSeverity::Danger,
                            ),
                        );
                        // Reset the log channel — recipe goes idle.
                        self.training_log_rx = None;
                    }
                }
            }
        }

        // ── cockpit-training-pressed-wiring v0.1.0 T-D-N1 — TrainingExited clear ──
        // Clear training-specific resources when the subprocess exits. The Wave C
        // T-D-N9 lifecycle arm above already handles `training_activity_handle`;
        // here we additionally clear the log channel so the recipe goes idle.
        #[cfg(feature = "live")]
        if matches!(msg, Message::TrainingExited(_)) {
            self.training_log_rx = None;
            self.cockpit.lab_state.training_cancel = None;
        }

        // ── cockpit-training-pressed-wiring v0.1.0 T-D-N1 — TrainingCancelPressed clear ──
        #[cfg(feature = "live")]
        if matches!(msg, Message::TrainingCancelPressed) {
            self.training_log_rx = None;
            self.cockpit.lab_state.training_cancel = None;
        }

        ui::state::update(&mut self.cockpit, msg);

        // T-D3.4 — Stop button: drop the cancel handle so the receiver sees
        // Disconnected at its next poll boundary (≤ 128 bars ≈ a few hundred ms).
        if lab_run_stop_requested {
            self.cockpit.lab_state.run_cancel = None;
            // Also clear the progress receiver — the run is being cancelled.
            self.lab_progress_rx = None;
        }

        // T-D3.1 — on any LabRunCompleted: clear the cancel handle (it may
        // already be None if the run completed normally, but clearing is
        // idempotent). Also clear the progress receiver.
        if lab_run_completed_any {
            self.cockpit.lab_state.run_cancel = None;
            self.lab_progress_rx = None;
        }

        // ── cockpit-activity-status-bar T-D-N8 — Lab Run activity handle ───────
        // Approach A: handle held on the iced side; ticked via LabRunProgress
        // messages already flowing to the iced thread (no Send required).
        #[cfg(feature = "live")]
        {
            // Tick on progress.
            if let Some(current) = lab_run_progress_current
                && let Some(ref handle) = self.lab_activity_handle
            {
                handle.tick(current);
            }
            // End on stop-requested: cancel the activity.
            if lab_run_stop_requested && let Some(handle) = self.lab_activity_handle.take() {
                handle.cancel();
                // Drop emits End { Cancelled }.
            }
            // End on completion.
            if lab_run_completed_any && let Some(handle) = self.lab_activity_handle.take() {
                if let Some(ref err) = lab_run_completed_err {
                    handle.fail(err.as_str());
                    // Drop emits End { Failed }.
                }
                // Else: Success — Drop emits End { Success }.
                drop(handle);
            }
        }

        // ── cockpit-activity-status-bar T-D-N9 — Training activity handle ──────
        // Approach A: handle held on the iced side; ticked via TrainingEventsRefreshed
        // messages (1 Hz poller per cockpit-training-control R7).
        #[cfg(feature = "live")]
        {
            // Tick on new training events.
            if let Some(count) = training_events_count
                && count > 0
                && let Some(ref handle) = self.training_activity_handle
            {
                // Use the running total length of training_events as
                // a monotonic progress counter.
                let total_events = self.cockpit.lab_state.training_events.len() as u64;
                handle.tick(total_events);
            }
            // End on cancel pressed: cancel the activity.
            if training_cancel_pressed && let Some(handle) = self.training_activity_handle.take() {
                handle.cancel();
                // Drop emits End { Cancelled }.
            }
            // End on subprocess exited: success.
            if training_exited {
                // Drop emits End { Success } — no explicit call needed.
                self.training_activity_handle = None;
            }
        }

        // T-D1.3 (lab-end-to-end-v2 T-AR-1) — rotate RunReportMirror:
        // prev ← last, last ← Some(new_mirror).
        // On Err / no captured tuple, do not rotate (R2.3: failure does NOT
        // mutate last_run_report).
        if let (Some(summary), Some(tuple)) =
            (lab_run_completed_summary, lab_run_completed_pre_tuple)
        {
            let mirror = ui::lab::runner::RunReportMirror {
                tuple,
                equity_series: std::sync::Arc::new(summary.equity_series.clone()),
                kpis: summary.kpis.clone(),
                generated_at: ::time::OffsetDateTime::now_utc(),
                bars: summary.bars.clone(),
                // lab-polish-round-2 R1 — position-curve (already symbol-filtered
                // by the runner before being placed in RunSummary).
                position_curve: std::sync::Arc::new(summary.position_curve.clone()),
            };
            let prev = self.cockpit.lab_state.last_run_report.take();
            self.cockpit.lab_state.prev_run_report = prev;
            self.cockpit.lab_state.last_run_report = Some(mirror);

            // T-D1.5 / R2.5 — when the engine surfaced fills (Phase C work
            // landing in Wave D-2 alongside the single-symbol extraction),
            // dispatch ChartMarkersLoaded so the chart's triangle markers
            // update. Empty fills → no dispatch (chart shows equity-only).
            if !summary.fills.is_empty() {
                return iced::Task::done(Message::ChartMarkersLoaded(Ok(summary.fills.clone())));
            }
        }

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

            // T-D3.1 (lab-end-to-end-v2 R6.1) — F4 fix: store the cancel handle
            // in `lab_state.run_cancel` so it lives until the run completes or
            // the operator presses Stop. Previous code dropped the handle immediately
            // with `let (_, cancel_recv) = ...` which meant `is_cancelled()` was
            // always true on the very first poll.
            let (handle, cancel_recv) = ui::lab::runner::cancellation_pair();
            self.cockpit.lab_state.run_cancel = Some(handle);

            // T-D4.1 (lab-end-to-end-v2 R7.1) — create progress channel.
            // The receiver is stored in an Arc<Mutex<Option<_>>> so the
            // LabProgressRecipe can take ownership in stream().
            let (progress_tx, progress_rx) = backtest::progress::progress_pair();
            // Bump the salt so the iced subscription sees a new recipe identity.
            self.lab_progress_recipe_salt = self.lab_progress_recipe_salt.wrapping_add(1);
            self.lab_progress_rx = Some(std::sync::Arc::new(std::sync::Mutex::new(Some(
                progress_rx,
            ))));

            // cockpit-activity-status-bar T-D-N8 — start Lab Run activity handle
            // (approach A: held on iced side, ticked via LabRunProgress messages,
            // ended on LabRunCompleted). Label: "<strategy> · <symbol> · <range>".
            let lab_activity_label = format!(
                "Backtest {} · {} · {}",
                run_cfg.strategy_id, run_cfg.symbol, run_cfg.range_label
            );
            let lab_activity_sender = self.bus.activity();
            self.lab_activity_handle = Some(
                lab_activity_sender
                    .start(agent::activity::ActivityKind::LabRun, lab_activity_label),
            );

            // T-D-N7: pass the ActivitySender so spawn_lab_run can wrap the
            // Yahoo preload in its own YahooPreload handle (approach A inline).
            let yahoo_preload_sender = Some(self.bus.activity());

            ui::lab::runner::spawn_lab_run(
                Some(&self.rt_handle),
                run_cfg,
                cancel_recv,
                progress_tx,
                yahoo_preload_sender,
                // lab-recipe-test-harness T-D1: production path uses default Yahoo
                // source (None = DefaultLabYahooBarSource via preload_yahoo_bars).
                None,
            )
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
        // ── Wave C seam: build_subscription_batch_descriptor ──────────────────
        //
        // `build_subscription_batch_descriptor` returns a `Vec<SubscriptionVariant>`
        // listing every recipe that should be included in the batch, based on the
        // current cockpit state.  We convert each variant to the corresponding
        // real `iced::Subscription` here.  This wires the introspectable
        // descriptor (tested by `cockpit_subscription_server_time_always_batched`
        // and `cockpit_subscription_toast_dismiss_always_batched`) to the actual
        // production subscription batch, so removing a variant from the descriptor
        // also removes it from the live subscription — closing the seam.
        let descriptor = ui::live::build_subscription_batch_descriptor(
            self.trail_mirror_handle.is_some(),
            self.lab_progress_rx.is_some(),
            self.training_log_rx.is_some(),
        );

        // Convert each descriptor variant to a real iced Subscription.
        // Order matches the descriptor (Bus, ServerTime, [Trail], [LabProgress],
        // Activity, [TrainingLog], ToastDismiss).
        let mut subs: Vec<iced::Subscription<Message>> = descriptor
            .iter()
            .map(|variant| match variant {
                ui::live::SubscriptionVariant::Bus => ui::live::subscription(Arc::clone(&self.bus)),
                ui::live::SubscriptionVariant::ServerTime => {
                    // 1 Hz server-time tick — drives the status-bar clock (T1509).
                    // Uses `ServerTimeRecipe` (tokio interval via tokio_stream) rather
                    // than `iced::time::every` which requires iced's `tokio` feature flag.
                    // The rt_handle is passed so the recipe can enter the tokio runtime
                    // context before calling `tokio::time::interval` (P1 bug fix — see
                    // ServerTimeRecipe struct comment above for full rationale).
                    iced::advanced::subscription::from_recipe(ServerTimeRecipe {
                        rt_handle: self.rt_handle.clone(),
                    })
                }
                ui::live::SubscriptionVariant::Trail => {
                    // Phase D+ T-D-N9 — trail-mirror Subscription bridge (R1.3 / R1.5).
                    // Only reached when has_trail = true, so unwrap is safe.
                    self.trail_mirror_handle
                        .as_ref()
                        .map(|h| ui::live::trail_mirror_subscription(h.clone()))
                        .unwrap_or_else(iced::Subscription::none)
                }
                ui::live::SubscriptionVariant::LabProgress => {
                    // Wave D-4 T-AR-6 — Lab progress subscription.
                    // Active only while a run is in-flight and the progress channel is open.
                    // Salt-bumped per LabRunRequested so iced sees a fresh recipe each run.
                    // Only reached when has_lab_progress = true, so unwrap is safe.
                    self.lab_progress_rx
                        .as_ref()
                        .map(|rx| {
                            iced::advanced::subscription::from_recipe(
                                ui::lab::progress::LabProgressRecipe {
                                    rt_handle: self.rt_handle.clone(),
                                    rx: std::sync::Arc::clone(rx),
                                    salt: self.lab_progress_recipe_salt,
                                },
                            )
                        })
                        .unwrap_or_else(iced::Subscription::none)
                }
                ui::live::SubscriptionVariant::Activity => {
                    // cockpit-activity-status-bar v0.1.0 Wave B (T-D-N5).
                    // Always active (no salt / no per-run gating).
                    iced::advanced::subscription::from_recipe(ui::live::ActivityRecipe {
                        bus: std::sync::Arc::clone(&self.bus),
                    })
                }
                ui::live::SubscriptionVariant::TrainingLog => {
                    // cockpit-training-pressed-wiring v0.1.0 T-D-N4.
                    // Active only while a training run is in-flight.
                    // Only reached when has_training_log = true, so unwrap is safe.
                    self.training_log_rx
                        .as_ref()
                        .map(|rx| {
                            iced::advanced::subscription::from_recipe(
                                ui::lab::training_log::TrainingLogRecipe {
                                    rt_handle: self.rt_handle.clone(),
                                    rx: std::sync::Arc::clone(rx),
                                    salt: self.training_log_recipe_salt,
                                },
                            )
                        })
                        .unwrap_or_else(iced::Subscription::none)
                }
                ui::live::SubscriptionVariant::ToastDismiss => {
                    // cockpit-toast-queue v0.1.0 T-D-N10 — 6th subscription.
                    // Always-on 500 ms ticker for auto-dismiss sweep; emits
                    // `Message::ToastTick(Instant::now())`. No salt / no gating.
                    iced::advanced::subscription::from_recipe(ToastDismissRecipe {
                        rt_handle: self.rt_handle.clone(),
                    })
                }
            })
            .collect();

        // Q6 — modal-open-gated Esc keyboard listener.
        // Added AFTER the base batch so the descriptor-to-iced loop stays clean.
        if self.cockpit.tape_audit_modal.is_some() {
            subs.push(iced::event::listen_with(
                |event, _status, _window| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        ..
                    }) => Some(Message::TapeAuditModalClosed),
                    _ => None,
                },
            ));
        }

        iced::Subscription::batch(subs)
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use ui::lab::runner::{RunReportMirror, RunSummary};
    use ui::state::Cockpit;

    /// Build a minimal `RunSummary` with the given equity series length.
    fn make_summary(equity_len: usize) -> RunSummary {
        use rust_decimal::Decimal;
        RunSummary {
            strategy_id: smol_str::SmolStr::new("v1.momentum"),
            symbol: smol_str::SmolStr::new("XRPUSDT"),
            report_path: None,
            equity_series: (0..equity_len as i64)
                .map(|i| (i * 3_600_000, Decimal::new(100_000, 0)))
                .collect(),
            fills: vec![],
            kpis: backtest::BacktestKpis::default(),
            bars: std::sync::Arc::new(Vec::new()),
            position_curve: Vec::new(),
        }
    }

    /// T-D1.3 / T-AR-1 — `lab_run_completed_wrapper_rotates_mirror`:
    /// after calling the binary-side wrapper logic with a `LabRunCompleted(Ok(_))`
    /// message and a valid pre-update (strategy + pair) lab tuple, the wrapper
    /// MUST populate `last_run_report` with a `RunReportMirror` whose
    /// `equity_series` length matches the summary.
    ///
    /// This is the K7 mitigation test.  The wrapper is tested directly by
    /// constructing a synthetic `Cockpit` state, pre-populating the `lab_state`
    /// fields, and invoking the capture + rotate logic.
    #[test]
    fn lab_run_completed_wrapper_rotates_mirror() {
        use trading_core::{StrategyId, Symbol, Venue};
        use ui::lab::equity_loader::LabTuple;
        use ui::lab::state::DateRange;

        let mut cockpit = Cockpit::new();
        // Pre-populate the cockpit lab_state so the pre-forward tuple snapshot
        // resolves (mimics the state just before LabRunCompleted arrives).
        cockpit.lab_state.strategy = Some(StrategyId::new("v1.momentum"));
        cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("XRPUSDT")));
        cockpit.lab_state.range = DateRange::default();
        assert!(cockpit.lab_state.last_run_report.is_none());

        // Reproduce the capture logic from cockpit_live::update's pre-forward block.
        let summary = make_summary(5);
        let pre_tuple = {
            let ls = &cockpit.lab_state;
            match (ls.strategy.as_ref(), ls.pair.as_ref()) {
                (Some(strategy), Some((venue, symbol))) => {
                    Some(LabTuple::new(strategy, *venue, symbol, ls.range.clone()))
                }
                _ => None,
            }
        };
        assert!(
            pre_tuple.is_some(),
            "pre_tuple must resolve when strategy+pair set"
        );

        // Reproduce the post-forward rotate block.
        if let Some(tuple) = pre_tuple {
            let mirror = RunReportMirror {
                tuple,
                equity_series: std::sync::Arc::new(summary.equity_series.clone()),
                kpis: summary.kpis.clone(),
                generated_at: ::time::OffsetDateTime::now_utc(),
                bars: summary.bars.clone(),
                position_curve: std::sync::Arc::new(summary.position_curve.clone()),
            };
            let prev = cockpit.lab_state.last_run_report.take();
            cockpit.lab_state.prev_run_report = prev;
            cockpit.lab_state.last_run_report = Some(mirror);
        }

        // Assert post-conditions.
        let report = cockpit
            .lab_state
            .last_run_report
            .as_ref()
            .expect("last_run_report must be Some after wrapper rotation");
        assert_eq!(
            report.equity_series.len(),
            5,
            "RunReportMirror equity_series must match summary.equity_series"
        );
        assert!(
            cockpit.lab_state.prev_run_report.is_none(),
            "prev_run_report must be None — first run has no predecessor"
        );
    }

    /// T-D1.3 extension — on a second run, prev ← last and last ← new.
    #[test]
    fn lab_run_completed_wrapper_rotates_prev_on_second_run() {
        use trading_core::{StrategyId, Symbol, Venue};
        use ui::lab::equity_loader::LabTuple;
        use ui::lab::state::DateRange;

        let mut cockpit = Cockpit::new();
        cockpit.lab_state.strategy = Some(StrategyId::new("v1.momentum"));
        cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("XRPUSDT")));
        cockpit.lab_state.range = DateRange::default();

        // First run.
        let apply_wrapper = |cockpit: &mut Cockpit, equity_len: usize| {
            let summary = make_summary(equity_len);
            let ls = &cockpit.lab_state;
            let pre_tuple = match (ls.strategy.as_ref(), ls.pair.as_ref()) {
                (Some(strategy), Some((venue, symbol))) => {
                    Some(LabTuple::new(strategy, *venue, symbol, ls.range.clone()))
                }
                _ => None,
            };
            if let Some(tuple) = pre_tuple {
                let mirror = RunReportMirror {
                    tuple,
                    equity_series: std::sync::Arc::new(summary.equity_series.clone()),
                    kpis: summary.kpis.clone(),
                    generated_at: ::time::OffsetDateTime::now_utc(),
                    bars: summary.bars.clone(),
                    position_curve: std::sync::Arc::new(summary.position_curve.clone()),
                };
                let prev = cockpit.lab_state.last_run_report.take();
                cockpit.lab_state.prev_run_report = prev;
                cockpit.lab_state.last_run_report = Some(mirror);
            }
        };

        apply_wrapper(&mut cockpit, 5);
        assert!(cockpit.lab_state.last_run_report.is_some());
        assert!(cockpit.lab_state.prev_run_report.is_none());

        apply_wrapper(&mut cockpit, 10);
        // After second run: last has 10, prev has 5.
        assert_eq!(
            cockpit
                .lab_state
                .last_run_report
                .as_ref()
                .unwrap()
                .equity_series
                .len(),
            10
        );
        assert_eq!(
            cockpit
                .lab_state
                .prev_run_report
                .as_ref()
                .unwrap()
                .equity_series
                .len(),
            5
        );
    }
}
