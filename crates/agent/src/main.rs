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
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // ── Tracing ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("trading=info".parse()?)
                .add_directive("agent=info".parse()?),
        )
        .json()
        .init();

    let args = Args::parse();

    info!("trading agent starting");

    // ── Config ────────────────────────────────────────────────────────────────
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
    let ledger = Arc::new(
        audit::Ledger::open(db_path)
            .await
            .context("open audit ledger")?,
    );
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .context("bootstrap chart of accounts")?;
    info!(db = %db_path, "audit ledger initialized");

    // ── Reflection store + writer task (T1807 / Q8) ────────────────────────────
    // Internal mpsc — not a bus channel (R8.3, hard constraint #4).
    // Gated by `cfg.reflection.enable_writer`; default false in
    // research / fixture profiles.  Producer side is held by the
    // executor's fill-handler tap; consumer task drains the queue
    // and persists via `SqliteReflectionStore::upsert`.
    let _reflection_writer = if cfg.reflection.enable_writer {
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
    // Construction (and the seeding tracing::info! call) is centralised in
    // `agent::runtime::build_registry` so neither the `ui` crate nor any
    // other downstream crate needs a direct `strategy` dependency.
    let registry = agent::runtime::build_registry(&cfg);

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
    let handles = RunHandles {
        config: Arc::new(cfg),
        ledger: Arc::clone(&ledger),
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        registry: Arc::clone(&registry),
        boot_id: boot_id.clone(),
    };
    agent::runtime::run(handles, cancel).await?;

    // ── T806 — close uptime interval on graceful shutdown ────────────────────
    agent::runtime::shutdown_writer(Arc::clone(&ledger), &boot_id).await;

    info!("agent stopped");
    Ok(())
}
