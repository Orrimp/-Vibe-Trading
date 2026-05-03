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
                .add_directive("trading=info".parse().unwrap())
                .add_directive("agent=info".parse().unwrap()),
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

    if let Some(ref m) = args.mode {
        if m.eq_ignore_ascii_case("live") {
            anyhow::bail!("mode=live is rejected in v0");
        }
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
