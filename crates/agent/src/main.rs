//! Trading agent binary — T31.
//!
//! Usage: `cargo run --bin trading -- --config config/agent.toml --mode research`
//!
//! Wires: `MarketDataSource` → `bar_stream` → `StrategyRegistry` → `risk` →
//! `ExecRouter` (paper) → `PaperEngine` → `audit`, plus reconciler, kill switch,
//! observability, broadcast buses the UI subscribes to.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};

use agent::{EventBus, KillSwitch};

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
    if let Err(e) =
        agent::observability::start_prometheus_exporter(&cfg.observability.prometheus_listen)
    {
        warn!(error = %e, "prometheus exporter failed to start (non-fatal)");
    }
    agent::observability::register_metrics();
    info!("observability initialized");

    // ── Kill switch ───────────────────────────────────────────────────────────
    let kill_switch = Arc::new(KillSwitch::new(&cfg.kill_switch.halt_file, 32));
    kill_switch.clone().spawn_halt_file_watcher();
    info!(halt_file = %cfg.kill_switch.halt_file, "kill switch initialized");

    if kill_switch.is_tripped() {
        warn!("halt file present at startup — agent entering Halted mode immediately");
        return Ok(());
    }

    // ── Audit ledger ──────────────────────────────────────────────────────────
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

    // ── Cost budget ───────────────────────────────────────────────────────────
    let _cost_sink = cost::LedgerCostSink::new(Arc::clone(&ledger));
    let cost_budget = cost::CostBudget::new(
        rust_decimal::Decimal::try_from(cfg.cost.budget_usd_month)
            .unwrap_or(rust_decimal::Decimal::from(20u32)),
    );
    info!(budget_usd = %cost_budget.remaining(), "cost budget initialized");

    // ── Strategy registry ─────────────────────────────────────────────────────
    let registry = strategy::StrategyRegistry::new();
    registry.register(Box::new(strategy::SmaCrossover::new(
        cfg.strategies.sma_crossover.fast_len,
        cfg.strategies.sma_crossover.slow_len,
    )));
    strategy::flush_pending_to_ledger(&registry, &ledger)
        .await
        .context("journal strategy load")?;
    info!(
        fast = cfg.strategies.sma_crossover.fast_len,
        slow = cfg.strategies.sma_crossover.slow_len,
        "strategy registry initialized"
    );

    // ── Broadcast bus ─────────────────────────────────────────────────────────
    let bus = Arc::new(EventBus::new(&cfg.bus));
    info!("broadcast event bus initialized");

    // ── Strategy watcher (paper + research only) ──────────────────────────────
    let registry = Arc::new(registry);
    let (watcher_shutdown_tx, watcher_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let strategies_dir = PathBuf::from("config/strategies");
        // Create the directory if it doesn't exist (non-fatal).
        let _ = std::fs::create_dir_all(&strategies_dir);
        let reg_clone = Arc::clone(&registry);
        let ledger_clone = Arc::clone(&ledger);
        let bus_clone = Arc::clone(&bus);
        tokio::spawn(agent::run_strategy_watcher(
            strategies_dir,
            reg_clone,
            ledger_clone,
            bus_clone,
            watcher_shutdown_rx,
        ));
    }
    info!("strategy_watcher started");

    // Keep the shutdown sender so it's dropped on shutdown, closing the watcher.
    let _watcher_shutdown = watcher_shutdown_tx;

    // ── Data source ───────────────────────────────────────────────────────────
    // In research mode, use replay; in paper mode, use Binance WS.
    match cfg.mode {
        agent::config::Mode::Research => {
            info!("research mode — replay feed (no live orders)");
            let parquet_root = &cfg.data.historical.parquet_root;
            let _feed = data::ReplayFeed::new(parquet_root, false); // wallclock pace
            info!(parquet_root = %parquet_root, "replay feed initialized");

            // Note: full replay loop would go here; for now just verify init.
            // The backtest binary (T25) runs the full loop.
            info!("agent subsystems initialized — entering idle (replay loop in backtest binary)");
        }
        agent::config::Mode::Paper => {
            info!("paper mode — Binance WS feed (paper fills, no real orders)");
            let ws_url = &cfg.data.sources.binance.ws_url;
            let _feed = data::BinanceFeed::new(ws_url, ws_url);
            info!(ws = %ws_url, "Binance feed initialized");
        }
    }

    // ── Serve until halted ────────────────────────────────────────────────────
    info!("agent running — serving /metrics, watching for halt file");
    let mut mode_rx = kill_switch.subscribe();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("ctrl-c received — shutting down");
        }
        mode = mode_rx.recv() => {
            if let Ok(agent::AgentMode::Halted { reason }) = mode {
                warn!(reason = %reason, "agent halted");
            }
        }
    }

    info!("agent stopped");
    Ok(())
}
