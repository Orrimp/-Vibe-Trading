#![deny(clippy::unwrap_used)]
//! Trading agent binary — T31 (refactored T902).
//!
//! Usage: `cargo run --bin trading -- --config config/agent.toml --mode research`
//!
//! After T902 (live-cockpit-unified), this binary is a thin wrapper around
//! `agent::runtime::run`.  The unified `cockpit_live` binary
//! (`crates/ui/src/bin/cockpit_live.rs`, lands in T904) calls into the
//! same `run` function from a side-thread tokio runtime; this binary
//! drives it from `#[tokio::main]` directly.
//!
//! Caller responsibilities here mirror the
//! `agent::runtime` module-doc:
//!
//! 1. parse CLI / load config / install tracing / install observability;
//! 2. construct ledger, kill_switch, registry, bus, boot_id;
//! 3. open the uptime interval (T806 R7.1);
//! 4. install Ctrl-C handler that calls `cancel.cancel()`;
//! 5. `agent::runtime::run(handles, cancel).await?`;
//! 6. `agent::runtime::shutdown_writer(ledger, &boot_id).await`;
//! 7. exit.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use agent::{EventBus, KillSwitch, RunHandles};
// T1931 (pass 6) — LLM factory wire-up at agent boot.
use cost::{CostBudget, CostSink, NoopCostSink};
use llm::factory::{LlmProviderFactory, Mode as LlmMode};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "trading", about = "v0 Trading Agent")]
struct Args {
    #[arg(long, default_value = "config/agent.toml")]
    config: PathBuf,

    /// Operating mode override (research | paper).
    #[arg(long)]
    mode: Option<String>,

    /// Override `replay_pace_ms` from config: force as-fast-as-possible replay
    /// regardless of the value in `config/agent.toml`.
    ///
    /// Use this flag for headless research runs and benchmarks when the config
    /// file has `replay_pace_ms = N` set for the cockpit live view.  Without
    /// this flag the headless bin respects whatever pace the config specifies.
    #[arg(long, default_value_t = false)]
    fast_replay: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // ── Tracing (T-RED-D10 / v2-1-tracing-layer-redactor) ────────────────────
    // Migrated from `tracing_subscriber::fmt().init()` to `install_global` to
    // wire the `RedactLayer` BEFORE the fmt sink (R1.4 ordering contract).
    // Secrets in structured log fields are redacted before reaching stdout/audit.
    llm::tracing_init::install_global(&["trading=info", "agent=info"], true)?;

    let args = Args::parse();

    info!("trading agent starting");

    // ── Config ────────────────────────────────────────────────────────────────
    let mut cfg = if args.config.exists() {
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

    // `--fast-replay` overrides `replay_pace_ms` from config so the headless
    // `trading` bin can run as-fast-as-possible even when the config file has
    // `replay_pace_ms = N` set for the cockpit live view.
    if args.fast_replay {
        cfg.data.historical.replay_pace_ms = None;
        info!("--fast-replay: replay_pace_ms overridden to None (full-speed replay)");
    }

    info!(mode = %cfg.mode, replay_pace_ms = ?cfg.data.historical.replay_pace_ms, "config loaded");

    // ── Observability ─────────────────────────────────────────────────────────
    // Install recorder before registering metrics — otherwise names never surface on /metrics.
    if let Err(e) = agent::observability::start_prometheus_exporter(&cfg.observability) {
        warn!(error = %e, "prometheus exporter failed to start (non-fatal)");
    }
    agent::observability::register_metrics();
    info!("observability initialized");

    // ── Audit ledger ──────────────────────────────────────────────────────────
    // Opened BEFORE the kill switch so the trip handler can dual-write
    // (T809 — operator success reports Q8).
    let db_path = &cfg.audit.ledger_db_path;
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // T-D-11: conditionally wire the broadcast tick bus (R7.1 / decomp §6).
    // `tick_bus_capacity = 0` → tee dormant (Ledger::open); any positive
    // value → Ledger::open_with_tick_bus. The sender is held for the
    // process lifetime so broadcast receivers stay live (K6).
    let (ledger, tick_bus_sender) = if cfg.audit.tick_bus_capacity > 0 {
        let (l, s) = audit::Ledger::open_with_tick_bus(db_path, cfg.audit.tick_bus_capacity)
            .await
            .context("open audit ledger with tick bus")?;
        (l, Some(s))
    } else {
        let l = audit::Ledger::open(db_path)
            .await
            .context("open audit ledger")?;
        (l, None)
    };
    let ledger = Arc::new(ledger);
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .context("bootstrap chart of accounts")?;
    info!(db = %db_path, tick_bus = cfg.audit.tick_bus_capacity > 0, "audit ledger initialized");

    // ── Reflection store + writer task (T1807 / Q8) ────────────────────────────
    // Internal mpsc — not a bus channel (R8.3, hard constraint #4).
    // Gated by `cfg.reflection.enable_writer`; default false in
    // research / fixture profiles.  Producer side is held by the
    // executor's fill-handler tap; consumer task drains the queue
    // and persists via `SqliteReflectionStore::upsert`.
    // lesson-card-wiring: create writer only in paper mode when enabled.
    // Research mode never writes lesson cards (no durable fills, no closed trades).
    let reflection_writer_for_runtime = if cfg.reflection.enable_writer {
        if let Some(parent) = cfg.reflection.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let store: std::sync::Arc<dyn reflection::ReflectionStore> = std::sync::Arc::new(
            reflection::store::sqlite::SqliteReflectionStore::open(&cfg.reflection.path)
                .await
                .context("open reflection store")?,
        );
        let (writer, task) =
            reflection::ReflectionWriter::new(store, cfg.reflection.channel_capacity);
        tokio::spawn(async move {
            task.run().await;
        });
        info!(path = %cfg.reflection.path.display(), "reflection writer task spawned");
        Some(writer)
    } else {
        info!("reflection writer disabled (cfg.reflection.enable_writer = false)");
        None
    };

    // ── Reflection audit-tick consumer stub (T-D-16 / R4.3) ─────────────────────
    // Spawned only when `[reflection].audit_tick_consumer_enabled = true` (default
    // false). Uses the broadcast sender from T-D-11; no-ops when tick bus is
    // disabled or config gate is off. Observation-only at v0.1.0 (R4.1).
    if cfg.reflection.audit_tick_consumer_enabled {
        if let Some(ref sender) = tick_bus_sender {
            let rx = sender.subscribe();
            let stream = audit::tick::AuditTickStream::new(rx, "reflection");
            // Store is shared with the existing ReflectionWriter task if enabled.
            // For the stub we open a separate in-memory-compat store reference;
            // observation-only so we can share a placeholder unit type.
            let stub = reflection::audit_tick_consumer::ReflectionAuditTickConsumer::new(
                stream,
                std::sync::Arc::new(()),
            );
            tokio::spawn(async move { stub.run().await });
            info!("reflection audit-tick consumer stub spawned (observation-only)");
        } else {
            warn!("audit_tick_consumer_enabled = true but tick_bus_capacity = 0; stub skipped");
        }
    }

    // ── Trail-mirror task (Phase D T-D-N24 / R6.1-R6.3) ─────────────────────────
    // Spawned only when the tick bus is active (tick_bus_capacity > 0). The
    // trail-mirror subscribes to the broadcast bus via the same sender used
    // by the reflection audit-tick consumer (R7.7 — subscriber-side only, no
    // producer change). Mirrors the `audit_tick_consumer_enabled` cfg-gate
    // convention: here we gate on `tick_bus_capacity > 0` (the tick bus must
    // be armed for the stream to carry events).
    //
    // The `TrailMirrorHandle` is kept as `_trail_mirror_handle` at v0.1.0
    // (the iced Subscription bridge T-D-N26 wires it into the cockpit's
    // subscription batch in a follow-up). The handle's `req_tx` / `tick_tx`
    // are accessible if the cockpit binary needs them.
    let _trail_mirror_handle = if let Some(ref sender) = tick_bus_sender {
        let rx = sender.subscribe();
        let (mirror, handle) = reflection::trail_mirror::TrailMirror::new(rx, Arc::clone(&ledger));
        tokio::spawn(async move { mirror.run().await });
        info!("trail mirror task spawned");
        Some(handle)
    } else {
        // Tick bus disabled → trail mirror is a no-op; cockpit Trail view
        // degrades to SQL-backfill-only mode (R3.4 empty-stage fallback).
        None
    };

    // ── Kill switch ───────────────────────────────────────────────────────────
    // T809 — wire the audit ledger + production incident spawner.  On
    // trip the kill switch dual-writes the audit memo + `strategy_events`
    // row and spawns the reports binary out-of-process.  The halt-file
    // watcher itself is spawned inside `agent::runtime::run`.
    let incident_spawner: Arc<dyn agent::IncidentSpawner> = Arc::new(agent::CommandIncidentSpawner);
    let kill_switch = Arc::new(KillSwitch::with_audit(
        &cfg.kill_switch.halt_file,
        32,
        Arc::clone(&ledger),
        incident_spawner,
    ));
    info!(halt_file = %cfg.kill_switch.halt_file, "kill switch initialized (audit-wired)");

    // ── Strategy registry ─────────────────────────────────────────────────────
    // Phase D (T-D-N22): paper mode with tcn_overlay_momentum.enabled = true
    // uses `build_registry_with_ledger` so the TCN overlay strategy receives
    // the tick-bus-armed ledger for `forecast_events` SQL durability. All
    // other modes / configs use the baseline `build_registry`.
    //
    // `Ledger` is `Clone` (SQLite pool is Arc-backed; tick_bus sender is cheap).
    // We clone before the Arc-wrap since `build_registry_with_ledger` takes
    // ownership. Backtest call sites NEVER reach this arm (they exit through
    // `agent::backtest_entry`, not here) — H2 anchor invariant preserved.
    let registry =
        if cfg.mode == agent::config::Mode::Paper && cfg.strategies.tcn_overlay_momentum.enabled {
            agent::runtime::build_registry_with_ledger(&cfg, (*ledger).clone())
        } else {
            agent::runtime::build_registry(&cfg)
        };

    // ── Broadcast bus ─────────────────────────────────────────────────────────
    let bus = Arc::new(EventBus::new(&cfg.bus));
    info!("broadcast event bus initialized");

    // ── Agent uptime interval — open BEFORE entering the runtime ─────────────
    // T806 R7.1: the open row carries the same boot id that the
    // heartbeat task (spawned inside `runtime::run`) writes against and
    // the close row written via `shutdown_writer` matches.  Failures
    // are warn-logged, never fatal — uptime is observability, not
    // control flow.
    let boot_id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = audit::journal::open_uptime_interval(&ledger, &boot_id, None).await {
        warn!(error = %e, "open_uptime_interval failed (non-fatal)");
    } else {
        info!(boot_id = %boot_id, "agent uptime interval opened");
    }

    // ── LLM provider (T1931, pass 6) ─────────────────────────────────────────
    // Gated on `cfg.llm.enabled` (default false in v2.0.0). When true,
    // construct the provider stack once at boot; the resulting
    // `Arc<dyn LlmProvider>` is stored on the runtime context for
    // future consumer briefs to pluck. **No bus channel added** (R8.3,
    // hard constraint #4).
    //
    // When false (the foundation-only default), no provider is
    // constructed, no key files are read, no `.local` is required —
    // a fresh checkout boots cleanly.
    let llm_provider: Option<Arc<dyn llm::LlmProvider>> = if cfg.llm.enabled {
        let llm_cfg = Arc::new(cfg.llm.clone());
        let budget = Arc::new(CostBudget::new(cfg.llm.budget_usd_month));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
        let llm_mode = match cfg.mode {
            agent::config::Mode::Research => LlmMode::Research,
            agent::config::Mode::Paper => LlmMode::Paper,
        };
        match LlmProviderFactory::build(Arc::clone(&llm_cfg), llm_mode, budget, sink, &args.config)
            .await
        {
            Ok(p) => {
                info!(
                    provider = %p.name(),
                    mode = %cfg.mode,
                    "llm provider constructed",
                );
                Some(p)
            }
            Err(e) => {
                // Auth / Provider errors at startup are non-fatal in
                // v2.0.0 (foundation-only — no consumer wired up). The
                // operator sees the error; the agent boots without LLM.
                warn!(error = %e, "llm factory build failed (non-fatal); subsystem disabled");
                None
            }
        }
    } else {
        info!("llm subsystem disabled (cfg.llm.enabled = false)");
        None
    };
    // Suppress unused warning until a consumer brief plucks the provider.
    let _llm_provider = llm_provider;

    // ── Cancellation token + Ctrl-C bridge ───────────────────────────────────
    // Single CancellationToken shared with `runtime::run`.  A
    // background task awaits SIGINT and trips the same token; the
    // runtime's internal `select!` observes the cancellation via its
    // child tokens and drains the JoinSet.
    let cancel = CancellationToken::new();
    {
        let cancel_signal = cancel.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                info!("ctrl-c received — shutting down");
                cancel_signal.cancel();
            }
        });
    }

    // ── Run the agent runtime ─────────────────────────────────────────────────
    // live-equity-history-durable ADR-0052 A2: equity store is Some only in
    // paper/live mode — research replay must NOT persist (repeating 2023
    // bar_ts ranges would produce a meaningless hydrated curve).
    let equity_store: Option<Arc<dyn audit::LiveEquityStore>> =
        if cfg.mode != agent::config::Mode::Research {
            Some(Arc::new(audit::LedgerEquityStore::new(Arc::clone(&ledger))))
        } else {
            None
        };

    let handles = RunHandles {
        config: Arc::new(cfg),
        ledger: Arc::clone(&ledger),
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        registry: Arc::clone(&registry),
        boot_id: boot_id.clone(),
        equity_store,
        reflection_writer: reflection_writer_for_runtime,
        forward_rx: None, // headless bin: no forward-command channel (byte-identical legacy path)
    };
    agent::runtime::run(handles, cancel).await?;

    // ── T806 — close uptime interval on graceful shutdown ────────────────────
    agent::runtime::shutdown_writer(Arc::clone(&ledger), &boot_id).await;

    info!("agent stopped");
    Ok(())
}
