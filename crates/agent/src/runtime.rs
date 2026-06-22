//! Agent runtime — extracted from `main.rs` for the unified
//! `cockpit_live` binary (T902 / live-cockpit-unified).
//!
//! The headless `trading` bin and the unified `cockpit_live` bin both
//! call into [`run`] after constructing a [`RunHandles`] bundle.  This
//! module owns the post-init task graph: data feed, strategy watcher,
//! funding poller, uptime heartbeat, kill-switch halt-file watcher,
//! and the optional in-process cron scheduler.
//!
//! ## Caller responsibilities
//!
//! The caller — not [`run`] — owns the items that depend on stable
//! shutdown ordering:
//!
//! 1. Parse CLI / load config / install tracing / install observability.
//! 2. Construct ledger, kill_switch, registry, bus, boot_id.
//! 3. **Open the uptime interval** (`audit::journal::open_uptime_interval`)
//!    *before* calling [`run`].  T806 invariant: exactly one open row
//!    per boot id, written before any heartbeat fires.
//! 4. Install a Ctrl-C handler that calls `cancel.cancel()`.  The
//!    headless `trading` bin spawns a tokio task on
//!    `tokio::signal::ctrl_c()`; the unified `cockpit_live` bin bridges
//!    the iced window-close event into the same call.
//! 5. Call [`run`] and `.await`.
//! 6. After [`run`] returns, call [`shutdown_writer`] **exactly once**
//!    to write the close-uptime row (T806 R7.1).  The caller orders
//!    the close *after* [`run`] returns so all in-flight writes have
//!    flushed before the close row lands.
//!
//! ## Determinism + shutdown invariants
//!
//! * Task spawn order matches the pre-extraction `main.rs` order so
//!   any timing-sensitive integration test (e.g.
//!   `kill_switch_audit_test`) keeps passing without modification.
//! * Every spawned task either holds a `cancel.child_token()` clone
//!   or terminates naturally when its upstream channel drops.  A
//!   single `cancel.cancel()` call therefore aborts the full task
//!   graph cooperatively.
//! * [`run`] returns `Ok(())` on graceful shutdown (cancel OR
//!   `AgentMode::Halted`).  The kill-switch dual-write (T809) and
//!   uptime open/heartbeat/close (T806) are *not* gated by [`run`]'s
//!   return — they run on their own task lifecycles inside the
//!   spawned subsystems.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use data::MarketDataSource;
use futures::StreamExt;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use trading_core::{MarketHealth, RiskTelemetry, Symbol, Tick, Timeframe, Timestamp, Venue};

use crate::config::Config;
use crate::{AgentMode, EventBus, KillSwitch};

/// T1409 — wall-clock injection for the stale-data watchdog.
///
/// The watchdog reads the current `Timestamp` via this trait object so
/// tests can supply a controllable clock (`OffsetDateTime` is not
/// affected by `tokio::time::pause`).  Live mode wires
/// `Arc::new(Timestamp::now)`; tests construct a fixture clock that
/// returns a value owned by the test harness.
pub type NowFn = Arc<dyn Fn() -> Timestamp + Send + Sync>;

/// T1409 — shared per-venue last-tick timestamp map.
///
/// Updated by the per-venue `ticks_tap` (paper-mode only) and read by
/// the stale-data watchdog every tick of its scan interval.  We use
/// `std::sync::Mutex` (not `tokio::sync::Mutex`) because the critical
/// section is a single map insert / read — no `.await` is held under
/// the lock.
pub type LastTickMap = Arc<Mutex<HashMap<Venue, Timestamp>>>;

/// T1409 — observer hook called by `spawn_feed_taps_with_observer` on
/// each tick before the tick is republished to the bus.  The paper-mode
/// supervisor wires a closure that records `tick.local_recv_ts` into
/// the shared [`LastTickMap`] under venue `V`.
pub type TickObserver = Arc<dyn Fn(&Tick) + Send + Sync>;

/// F5-LAUNCH — hot-swap command into the paper-loop supervisor (post-boot).
///
/// `core`-typed payload so the `ui` crate stays free of strategy/exec deps.
/// Built UI-side from the leaderboard crowned/picked row + the F3 budget
/// using `core` types only (`StrategyId`/`Symbol`/`Money`).
/// ADR-0060 § D6.
#[derive(Debug, Clone)]
pub enum ForwardCommand {
    /// Launch (or re-launch) the forward run with this selection — cancels the
    /// current trading-loop task and spawns a fresh one on the same bus/ledger.
    Launch(crate::config::ForwardRunConfig),
}

/// Subsystems handed to [`run`] as already-constructed handles.
///
/// Both the headless `trading` bin and the unified `cockpit_live` bin
/// construct these and pass ownership in.  The construction order
/// mirrors the pre-extraction `crates/agent/src/main.rs` flow.
pub struct RunHandles {
    /// Loaded `[agent]` config — read for mode + sub-system knobs.
    pub config: Arc<Config>,
    /// Audit ledger (T806 / T809).  `runtime::run` does NOT close
    /// it; the caller drops the `Arc` after `shutdown_writer`.
    pub ledger: Arc<audit::Ledger>,
    /// Broadcast bus (R1 / live-cockpit-unified).  `runtime::run`
    /// hands clones to each producer task; on shutdown all clones
    /// drop, the senders close, and downstream subscribers receive
    /// `RecvError::Closed` (T903d invariant).
    pub bus: Arc<EventBus>,
    /// Audit-wired kill switch (T809).  `runtime::run` calls
    /// `spawn_halt_file_watcher()` exactly once on this handle.
    pub kill_switch: Arc<KillSwitch>,
    /// Strategy registry — already populated with the SMA crossover
    /// strategy before [`run`] is called.
    pub registry: Arc<strategy::StrategyRegistry>,
    /// Boot UUID — passed straight to the heartbeat task; the caller
    /// also passes the same id to `open_uptime_interval` /
    /// `close_uptime_interval` so a single boot writes one open + N
    /// heartbeats + one close, all carrying the same id (R12 / T806).
    pub boot_id: String,
    /// Durable live-equity store (live-equity-history-durable ADR-0052 / A1).
    ///
    /// `Some` in **paper/live mode only** — research mode passes `None` so
    /// the replayed 2023 `bar_ts` ranges never pollute the series (A2 gate).
    /// Headless `trading` bin and `cockpit_live` both call `runtime::run`;
    /// both persist equity when `Some` (A7 — the series accrues regardless of
    /// whether the UI is attached).
    ///
    /// Constructed by the caller from `Arc::new(audit::LedgerEquityStore::new(ledger.clone()))`.
    pub equity_store: Option<Arc<dyn audit::LiveEquityStore>>,

    /// Producer side of the reflection mpsc (T1807 / lesson-card wiring).
    ///
    /// `Some` when `cfg.reflection.enable_writer = true` AND the runtime is
    /// in paper mode — the trading loop calls `try_enqueue` on each
    /// position-close fill.  Research mode always gets `None` (no durable
    /// fills, no lesson cards in replay).  `cockpit_live` leaves this `None`
    /// because it re-uses the same reflection DB but does not wire fills.
    pub reflection_writer: Option<reflection::ReflectionWriter>,

    /// F5-LAUNCH — receiver side of the forward-command channel (ADR-0060 § D6).
    ///
    /// `Some(rx)` only for the cockpit (interactive post-boot launch); the
    /// headless `trading` bin + the soak harness pass `None` → the paper branch
    /// spawns the initial loop and never enters the supervisor select-loop, so
    /// their behaviour is **byte-identical** to today.
    ///
    /// **Replaces** the old `forward: Option<ForwardRunConfig>` boot-config
    /// field (which could only carry a BOOT-time selection — structurally
    /// insufficient for the post-boot bake-off; that is the root cause of the
    /// two prior FAKE passes).  ADR-0060 § D6.
    pub forward_rx: Option<tokio::sync::mpsc::Receiver<ForwardCommand>>,

    /// F6-PLAN — sender side of the forward-plan channel (ADR-0062 § D4).
    ///
    /// `Some(tx)` only for the cockpit; the supervisor calls `try_send` after
    /// building the `ForwardPlan` on each `ForwardCommand::Launch`.  The `ui`
    /// thread holds the matching `Receiver` (the iced subscription pattern).
    ///
    /// `None` → no plan is produced; the headless `trading` bin + the soak
    /// harness pass `None` → byte-identical to pre-F6 behaviour (D6 invariant).
    pub plan_tx: Option<tokio::sync::mpsc::Sender<crate::config::ForwardPlan>>,

    /// F9-NARRATION — receiver side of the narration-request channel (ADR-0064 § D3).
    ///
    /// `Some(rx)` only for the cockpit; the iced thread sends a
    /// [`crate::narration::NarrationRequest`] when the operator clicks "Explain".
    /// The agent's narration task (spawned inside `run`) receives it and runs
    /// [`crate::narration::generate_narration`] on the async runtime, then sends
    /// the `NarrationOutcome` back via `narration_outcome_tx`.
    ///
    /// `None` → byte-identical to today (headless / soak unaffected — D6 invariant).
    pub narration_request_rx:
        Option<tokio::sync::mpsc::Receiver<crate::narration::NarrationRequest>>,

    /// F9-NARRATION — sender side of the narration-outcome return channel (ADR-0064 § D3).
    ///
    /// `Some(tx)` only for the cockpit; the narration task sends the
    /// [`crate::narration::NarrationOutcome`] back to the iced thread's recipe
    /// (symmetric with F6's `plan_tx`).
    ///
    /// `None` → byte-identical to today (headless / soak unaffected — D6 invariant).
    pub narration_outcome_tx: Option<tokio::sync::mpsc::Sender<crate::narration::NarrationOutcome>>,
}

/// Build a [`strategy::StrategyRegistry`] pre-seeded with the strategies
/// declared in `cfg`.
///
/// Factored out of both the headless `trading` binary and the
/// `cockpit_live` binary so that **neither the `ui` crate nor any other
/// downstream crate needs a direct `strategy` dependency**. The `agent`
/// crate already depends on `strategy`; callers obtain the registry
/// opaquely as `Arc<strategy::StrategyRegistry>` via this function.
///
/// Currently seeds:
/// - `SmaCrossover` with `cfg.strategies.sma_crossover.{fast_len,slow_len}`.
///
/// Additional strategies can be added here as they land without touching
/// any binary's `main` function.
pub fn build_registry(cfg: &Config) -> Arc<strategy::StrategyRegistry> {
    let registry = strategy::StrategyRegistry::new();
    registry.register(Box::new(strategy::SmaCrossover::new(
        cfg.strategies.sma_crossover.fast_len,
        cfg.strategies.sma_crossover.slow_len,
    )));
    tracing::info!(
        fast = cfg.strategies.sma_crossover.fast_len,
        slow = cfg.strategies.sma_crossover.slow_len,
        "strategy registry constructed",
    );
    Arc::new(registry)
}

/// Phase D (T-D-N22 / R6.5) — Construct the strategy registry with an
/// audit ledger threaded through to the TCN overlay strategy.
///
/// This is the paper-mode sibling of [`build_registry`]. It:
/// 1. Always registers `SmaCrossover` (same as `build_registry`).
/// 2. When `cfg.strategies.tcn_overlay_momentum.enabled = true`, attempts
///    to load the BS-1 TCN checkpoint and attach `ledger` for the Phase D
///    `forecast_events` SQL writer. On checkpoint load failure, logs a
///    warning and falls back to `SmaCrossover`-only (graceful degradation).
///
/// **Backtest determinism invariant (H2):** backtests continue to call
/// `build_registry(cfg)` (no ledger). This function is paper-mode only.
/// The ledger passed here must have been constructed via
/// `Ledger::open_with_tick_bus` (tick-bus armed) — the static-branch
/// tee at `crates/audit/src/tick.rs:104-107` stays dormant when
/// `tick_bus = None` (the `Ledger::open` backtest path).
///
/// The `forecast-audit-tick` feature gate on the TCN wiring is checked at
/// compile time via `#[cfg]`; when the feature is absent the TCN arm is
/// elided and the function degrades to an exact copy of `build_registry`.
pub fn build_registry_with_ledger(
    cfg: &Config,
    ledger: audit::Ledger,
) -> Arc<strategy::StrategyRegistry> {
    let registry = strategy::StrategyRegistry::new();

    // Always register SmaCrossover (baseline strategy).
    registry.register(Box::new(strategy::SmaCrossover::new(
        cfg.strategies.sma_crossover.fast_len,
        cfg.strategies.sma_crossover.slow_len,
    )));
    tracing::info!(
        fast = cfg.strategies.sma_crossover.fast_len,
        slow = cfg.strategies.sma_crossover.slow_len,
        "strategy registry: SmaCrossover registered",
    );

    // TCN overlay strategy — opt-in via [strategies.tcn_overlay_momentum] enabled = true.
    // Guard: cfg-gated on the `forecast-audit-tick` combined feature so that builds
    // without candle/audit-tick are unaffected.
    #[cfg(feature = "forecast-audit-tick")]
    if cfg.strategies.tcn_overlay_momentum.enabled {
        // Load the base momentum config from the canonical strategy config path.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let cfg_path = std::path::PathBuf::from(&manifest_dir)
            .join("../../config/strategies/top10_momentum_h1.toml");

        match strategy::CrossSectionalMomentumConfig::from_file(&cfg_path) {
            Ok(momentum_cfg) => {
                let base = strategy::MomentumStrategy::from_config(
                    momentum_cfg,
                    smol_str::SmolStr::new(cfg_path.to_string_lossy()),
                );
                match strategy::TcnOverlayMomentumStrategy::with_tcn_bs1_ledger(base, ledger) {
                    Ok(tcn_strategy) => {
                        registry.register(Box::new(tcn_strategy));
                        tracing::info!(
                            "strategy registry: TcnOverlayMomentum (BS-1 + ledger) registered"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "strategy registry: TcnOverlayMomentum skipped (checkpoint load failed)"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "strategy registry: TcnOverlayMomentum skipped (momentum config load failed)"
                );
            }
        }
    }

    // When `forecast-audit-tick` is not enabled, the TCN arm is elided and the
    // `ledger` param is unused. Suppress the unused-variable lint.
    #[cfg(not(feature = "forecast-audit-tick"))]
    let _ = ledger;

    Arc::new(registry)
}

/// F5b — build a [`strategy::StrategyRegistry`] for the **selected** strategy
/// from a [`crate::config::ForwardRunConfig`].
///
/// # Strategy id → concrete engine mapping
///
/// | `ForwardRunConfig::strategy` id | Engine loaded                                          |
/// |---------------------------------|--------------------------------------------------------|
/// | `"v0.sma"` / `"v0.5.sma"`      | `SmaCrossover` from `cfg.strategies.sma_crossover`     |
/// | `"v0.5.macd"`                   | `ComposedStrategy` from `config/strategies/btc_macd_trend.toml` |
/// | `"v0.5.rsi"`                    | `ComposedStrategy` from `config/strategies/btc_rsi_reversion.toml` |
/// | `"v0.5.bbands"`                 | `ComposedStrategy` from `config/strategies/btc_bbands_mean_revert.toml` |
/// | `"v0.buyhold"`                  | `AlwaysLongStrategy` (Buy on first bar, Hold thereafter) |
///
/// This uses **the same TOML files the bake-off scored**, so the forward engine
/// is byte-for-byte the strategy that won the ranking (F5b fidelity requirement).
///
/// # Errors
///
/// Returns `Err` when the TOML file for a composed strategy cannot be resolved
/// or parsed. The caller MUST NOT silently fall back to an SMA proxy — that is
/// the exact bug F5b kills. The supervisor at the call site logs the error and
/// does NOT swap the running loop to avoid silent degradation.
///
/// # No-forward path
///
/// When `forward` is `None`, delegates to `build_registry(cfg)` — byte-identical
/// to the pre-F5b headless / soak path.
pub fn build_registry_for(
    cfg: &Config,
    forward: Option<&crate::config::ForwardRunConfig>,
) -> anyhow::Result<Arc<strategy::StrategyRegistry>> {
    let Some(fwd) = forward else {
        return Ok(build_registry(cfg));
    };

    let registry = strategy::StrategyRegistry::new();
    let id = fwd.strategy.0.as_str();

    match id {
        "v0.sma" | "v0.5.sma" => {
            registry.register(Box::new(strategy::SmaCrossover::new(
                cfg.strategies.sma_crossover.fast_len,
                cfg.strategies.sma_crossover.slow_len,
            )));
            tracing::info!(
                strategy = id,
                fast = cfg.strategies.sma_crossover.fast_len,
                slow = cfg.strategies.sma_crossover.slow_len,
                "build_registry_for: SmaCrossover registered"
            );
        }
        // ── F5b: real ComposedStrategy for MACD / RSI / BBands ───────────────
        //
        // Uses the SAME TOML files the bake-off scored so the forward engine
        // is byte-for-byte the strategy that won the ranking.  The path
        // resolver walks up from CWD to find the workspace root — identical
        // to the pattern in `crates/backtest/src/scenarios/sma_composed_run.rs`
        // (Bug #56 fix).
        "v0.5.macd" => {
            let toml_name = "btc_macd_trend";
            let reg = load_composed_strategy_from_toml(toml_name).with_context(|| {
                format!(
                    "build_registry_for: failed to load ComposedStrategy \
                         for strategy id '{id}' from config/strategies/{toml_name}.toml"
                )
            })?;
            registry.register(Box::new(reg));
            tracing::info!(
                strategy = id,
                toml = toml_name,
                "build_registry_for: ComposedStrategy (btc_macd_trend) registered"
            );
        }
        "v0.5.rsi" => {
            let toml_name = "btc_rsi_reversion";
            let reg = load_composed_strategy_from_toml(toml_name).with_context(|| {
                format!(
                    "build_registry_for: failed to load ComposedStrategy \
                         for strategy id '{id}' from config/strategies/{toml_name}.toml"
                )
            })?;
            registry.register(Box::new(reg));
            tracing::info!(
                strategy = id,
                toml = toml_name,
                "build_registry_for: ComposedStrategy (btc_rsi_reversion) registered"
            );
        }
        "v0.5.bbands" => {
            let toml_name = "btc_bbands_mean_revert";
            let reg = load_composed_strategy_from_toml(toml_name).with_context(|| {
                format!(
                    "build_registry_for: failed to load ComposedStrategy \
                         for strategy id '{id}' from config/strategies/{toml_name}.toml"
                )
            })?;
            registry.register(Box::new(reg));
            tracing::info!(
                strategy = id,
                toml = toml_name,
                "build_registry_for: ComposedStrategy (btc_bbands_mean_revert) registered"
            );
        }
        // ── F5b: real AlwaysLong for buy-and-hold ────────────────────────────
        //
        // `bakeoff::buyhold::run_buyhold_path` is a standalone equity-curve
        // function, not a Strategy impl. `AlwaysLongStrategy` bridges the
        // two: same semantics (buy once at first bar, hold indefinitely)
        // expressed as a proper Strategy so it can be registered and driven
        // bar-by-bar by the paper-loop engine.
        "v0.buyhold" => {
            registry.register(Box::new(strategy::AlwaysLongStrategy::new()));
            tracing::info!(
                strategy = id,
                "build_registry_for: AlwaysLongStrategy (buy-and-hold) registered"
            );
        }
        // ── F8: EnsembleStrategy (ADR-0063 § D5) ─────────────────────────────
        //
        // `build_ensemble` builds the two pre-registered F8 ensembles using
        // the same TOML-loaded members as the bake-off arms (build_member).
        // The same anti-fake gate as F5b: no silent fallback on unknown ids.
        "v0.8.vote.majority" | "v0.8.vote.unanimous" => {
            let ensemble = strategy::build_ensemble(id).with_context(|| {
                format!(
                    "build_registry_for: failed to build EnsembleStrategy for id '{id}' \
                     (F8 anti-fake gate)"
                )
            })?;
            registry.register(Box::new(ensemble));
            tracing::info!(
                strategy = id,
                "build_registry_for: EnsembleStrategy registered"
            );
        }
        unknown => {
            // Unknown id — return a typed error so the supervisor can log and
            // surface it to the UI error path. NO silent SMA fallback.
            anyhow::bail!(
                "build_registry_for: unknown strategy id '{}' — \
                 refusing to silently fall back to SmaCrossover proxy (F5b anti-fake gate)",
                unknown
            );
        }
    }

    Ok(Arc::new(registry))
}

/// Helper: load a `ComposedStrategy` from `config/strategies/<toml_name>.toml`.
///
/// Uses `backtest::paths::resolve_workspace_path` (identical to the
/// `sma_composed_run` pattern, Bug #56 fix) so that the agent binary launched
/// from any CWD can still find the TOML.
fn load_composed_strategy_from_toml(toml_name: &str) -> anyhow::Result<strategy::ComposedStrategy> {
    use std::path::PathBuf;
    let rel_path = PathBuf::from(format!("config/strategies/{toml_name}.toml"));
    let toml_path = backtest::paths::resolve_workspace_path(&rel_path);
    let cfg = strategy::ComposedStrategyConfig::from_file(&toml_path)
        .with_context(|| format!("load ComposedStrategyConfig from {}", toml_path.display()))?;
    // source_path stored as rel_path (not absolute) for audit identity — same
    // convention as sma_composed_run (ADR-0038 § D6 anchor-preservation).
    let source_path = smol_str::SmolStr::new(rel_path.to_string_lossy());
    Ok(strategy::ComposedStrategy::from_config(cfg, source_path))
}

/// Run all agent tokio tasks until `cancel` is tripped or the kill
/// switch flips to [`AgentMode::Halted`].  Returns `Ok(())` on
/// graceful shutdown.
///
/// Spawns (in this order, matching the pre-extraction `main.rs`):
///
/// 1. **uptime heartbeat** — 30 s tick that writes
///    `audit::journal::heartbeat_uptime` rows.  Holds a child cancel
///    token so it exits before the close-uptime write fires.
/// 2. **strategy watcher** — file-watch on `config/strategies/`.
/// 3. **kill-switch halt-file watcher** — polls for the `.halt`
///    sentinel file (1 s cadence).
/// 4. **funding poller + persist sidecar** — only when
///    `cfg.funding.enabled = true`.
/// 5. **in-process cron scheduler** — only under
///    `--features in_process_cron` (off by default).
/// 6. **data source init** — research-mode replay or paper-mode
///    Binance feed.  `run` itself does NOT consume bars; the bar
///    feed is plumbed through the strategy registry already.  The
///    init lines exist to surface "feed initialized" log entries
///    used by ops dashboards.
///
/// After all tasks are spawned, `run` `await`s a `tokio::select!`
/// over (a) `cancel.cancelled()` and (b) the kill-switch
/// subscription receiving `AgentMode::Halted`.  Either branch
/// signals the JoinSet to drain, then returns `Ok(())`.
///
/// # Errors
///
/// Returns an error from initial subsystem setup (e.g. the strategies
/// directory can't be created and `flush_pending_to_ledger` fails);
/// task-internal failures are warn-logged and never propagated.
pub async fn run(handles: RunHandles, cancel: CancellationToken) -> Result<()> {
    let RunHandles {
        config,
        ledger,
        bus,
        kill_switch,
        registry,
        boot_id,
        equity_store,
        reflection_writer,
        forward_rx,
        plan_tx,
        narration_request_rx,
        narration_outcome_tx,
    } = handles;

    // ── Kill-switch halt-file watcher ─────────────────────────────────────────
    // T809 — the audit-wired KillSwitch was already constructed by the
    // caller; we only spawn the file watcher here.  spawn_halt_file_watcher
    // does an immediate file-exists check inside the spawned task, so a
    // halt-file present at startup will trip the switch within one
    // tokio yield.  The `select!` at the bottom of this function then
    // observes the Halted mode and returns Ok.
    Arc::clone(&kill_switch).spawn_halt_file_watcher();
    info!(
        halt_file = %config.kill_switch.halt_file,
        "kill switch halt-file watcher spawned",
    );

    // Race-honoring early bailout — if the halt file existed at startup
    // and the watcher task already trip()ped, return immediately.
    // Mirrors pre-extraction main.rs:107–110.
    //
    // We yield once first so the watcher's spawned task can run its
    // initial `check_halt_file()` before we observe `is_tripped()`.
    tokio::task::yield_now().await;
    if kill_switch.is_tripped() {
        warn!("halt file present at startup — agent entering Halted mode immediately");
        return Ok(());
    }

    // JoinSet hosts every long-running task spawned below so a single
    // `cancel.cancel()` propagates to every child token, every task
    // returns, and the runtime exits cooperatively.
    let mut set: JoinSet<()> = JoinSet::new();

    // ── Agent uptime heartbeat (T806 — operator success reports R7.1) ────────
    // Open / close are caller responsibilities (see module docs); the
    // heartbeat itself runs here.  30 s tick; failures warn-logged.
    {
        let ledger_hb = Arc::clone(&ledger);
        let boot_id_hb = boot_id.clone();
        let cancel_hb = cancel.child_token();
        set.spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
            // Skip the first immediate tick — the caller just opened the row.
            tick.tick().await;
            loop {
                tokio::select! {
                    () = cancel_hb.cancelled() => break,
                    _ = tick.tick() => {
                        if let Err(e) =
                            audit::journal::heartbeat_uptime(&ledger_hb, &boot_id_hb, None).await
                        {
                            warn!(error = %e, "heartbeat_uptime failed (non-fatal)");
                        }
                    }
                }
            }
        });
    }

    // ── In-process cron scheduler (T810 — operator success reports) ─────────
    // Behind the `in_process_cron` feature flag.  Default builds skip
    // entirely.  Failures (invalid cron expression, scheduler start
    // failure) are warn-logged — never fatal.  Keep the scheduler
    // alive for the duration of `run` by binding it to a guard local.
    #[cfg(feature = "in_process_cron")]
    let _cron_scheduler = {
        let cron_cfg = crate::cron::CronConfig {
            ledger_db_path: std::path::PathBuf::from(&config.audit.ledger_db_path),
            parquet_root: std::path::PathBuf::from(&config.data.historical.parquet_root),
            ..Default::default()
        };
        match crate::cron::start(cron_cfg).await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!(error = %e, "in-process cron scheduler failed to start (non-fatal)");
                None
            }
        }
    };

    // ── Cost budget ───────────────────────────────────────────────────────────
    let _cost_sink = cost::LedgerCostSink::new(Arc::clone(&ledger));
    let cost_budget = cost::CostBudget::new(
        rust_decimal::Decimal::try_from(config.cost.budget_usd_month)
            .unwrap_or(rust_decimal::Decimal::from(20u32)),
    );
    info!(budget_usd = %cost_budget.remaining(), "cost budget initialized");

    // ── Strategy load (flush pending to ledger) ───────────────────────────────
    // The registry was populated by the caller; the load events
    // themselves still need to be journaled here so the audit row
    // ordering matches pre-extraction main.rs.
    strategy::flush_pending_to_ledger(&registry, &ledger)
        .await
        .context("journal strategy load")?;
    info!("strategy registry initialized");

    // ── Strategy watcher (paper + research only) ──────────────────────────────
    let (watcher_shutdown_tx, watcher_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let strategies_dir = PathBuf::from("config/strategies");
        // Create the directory if it doesn't exist (non-fatal).
        let _ = std::fs::create_dir_all(&strategies_dir);
        let reg_clone = Arc::clone(&registry);
        let ledger_clone = Arc::clone(&ledger);
        let bus_clone = Arc::clone(&bus);
        set.spawn(async move {
            crate::run_strategy_watcher(
                strategies_dir,
                reg_clone,
                ledger_clone,
                bus_clone,
                watcher_shutdown_rx,
            )
            .await;
        });
    }
    info!("strategy_watcher started");

    // Bridge `cancel.cancelled()` to the watcher's oneshot so a
    // top-level cancel signals the watcher to drain.  Sender lives in
    // the bridge task; on cancel it is dropped, closing the channel,
    // which the watcher observes via `shutdown_rx`.
    {
        let cancel_w = cancel.child_token();
        set.spawn(async move {
            cancel_w.cancelled().await;
            drop(watcher_shutdown_tx);
        });
    }

    // ── Funding-rate poller (v1 T614) ─────────────────────────────────────────
    // Disabled by default (funding.enabled = false in config/agent.toml).
    // When enabled, spawns an hourly REST poller against Binance fapi.
    // Non-essential: if the spawned task panics the agent continues running.
    if config.funding.enabled {
        let universe: Vec<trading_core::Symbol> = config
            .funding
            .universe
            .iter()
            .map(|s| trading_core::Symbol::new(s.as_str()))
            .collect();
        let interval = std::time::Duration::from_secs(config.funding.interval_secs);
        let poller = data::funding::FundingPoller {
            universe: universe.clone(),
            interval,
        };
        let client = Arc::new(data::funding::BinanceFundingClient::new());
        let tx = bus.funding_obs_sender();
        let ledger_clone = Arc::clone(&ledger);
        let cancel_poll = cancel.child_token();
        let cancel_persist = cancel.child_token();

        info!(universe_size = universe.len(), "funding_poller_started");

        // Spawn the poller task.  Panic in the poller does NOT crash the agent.
        set.spawn(async move {
            poller.run(client.as_ref(), &tx, cancel_poll).await;
        });

        // Spawn a persistence sidecar: subscribe to funding_obs and write to ledger.
        let mut rx = bus.funding_obs();
        set.spawn(async move {
            loop {
                tokio::select! {
                    () = cancel_persist.cancelled() => break,
                    msg = rx.recv() => {
                        match msg {
                            Ok(obs) => {
                                if let Err(e) = audit::journal::insert_funding_obs(&ledger_clone, &obs).await {
                                    warn!(error = %e, "funding_obs persist failed (non-fatal)");
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!(skipped = n, "funding_obs channel lagged — rows skipped");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                }
            }
        });
    } else {
        info!("funding_poller_disabled");
    }

    // ── Data source init ──────────────────────────────────────────────────────
    // In research mode, use replay; in paper mode, use Binance WS.  The
    // init lines exist to surface "feed initialized" log entries used
    // by ops dashboards.  The full feed loop is host-bin-specific; the
    // backtest binary runs the replay loop, the cockpit_live binary
    // consumes the bar/tick streams via the T903b taps spawned below.
    //
    // T903b — bar/tick taps publish into the `bars` and `ticks`
    // channels of the EventBus so the cockpit (and any other bus
    // consumer) sees real-time market data.  The taps are *additive*
    // — they sit between the feed and any downstream consumer; if no
    // strategy is wired here yet (the strategy is registered via the
    // watcher and presently consumes its own subscription elsewhere),
    // the taps are still useful: the bus sees them.  Symbol +
    // timeframe: the default is hardcoded BTCUSDT (pre-F5L byte-identical);
    // the supervisor swaps the feed on Launch. ADR-0060 § D6.
    let feed_symbol = Symbol::new("BTCUSDT");
    let feed_tf = Timeframe::OneMinute;
    match config.mode {
        crate::config::Mode::Research => {
            info!("research mode — replay feed (no live orders)");
            let parquet_root = &config.data.historical.parquet_root;
            let replay_fast = config.data.historical.replay_fast;
            let replay_pace_ms = config.data.historical.replay_pace_ms;
            let feed: Arc<dyn MarketDataSource> = Arc::new(data::ReplayFeed::new_with_pace(
                parquet_root,
                replay_fast,
                replay_pace_ms,
            ));
            info!(
                parquet_root = %parquet_root,
                replay_fast,
                ?replay_pace_ms,
                "replay feed initialized"
            );
            spawn_feed_taps(
                feed.as_ref(),
                Arc::clone(&bus),
                feed_symbol.clone(),
                feed_tf,
                &mut set,
                &cancel,
            )
            .await;

            // ── Research-mode strategy + paper-engine trading loop ────────────
            // Now that bars flow onto the bus via `bars_tap`, we subscribe to
            // `bus.bars()` and run the strategy + paper engine for each bar.
            // Without this loop, fills/positions/pnl are never published and
            // the Live dashboard fills tape + positions panel hang forever on
            // "Connecting to the fill stream…" / "Loading positions from the
            // ledger…".
            //
            // Design notes:
            // - We reuse the same `registry` that `run_strategy_watcher` already
            //   holds — it carries `SmaCrossover` seeded from the config.
            // - Fill/position publication goes through `PaperEnginePublisher` →
            //   `EventBus::publish_fill` + `EventBus::publish_position` so the
            //   existing iced subscription (`ui::live::stream_fills`) picks them up.
            // - PnL snapshots are emitted every bar via `ReconcilerTask` → bus.
            // - Determinism: the paper engine seed is fixed (`0x00C0_FFEE`).
            //   Research mode is single-threaded from the perspective of bar
            //   processing (broadcast::Receiver is polled in one task).
            spawn_trading_loop(
                Arc::clone(&feed),
                Arc::clone(&bus),
                Arc::clone(&registry),
                &config.backtest,
                &config.risk,
                feed_symbol.clone(),
                feed_tf,
                None, // Research mode: equity_store = None (A2 gate — never write in research)
                "research",
                &mut set,
                &cancel,
                None,   // research: no journal persistence
                None,   // research: no lesson cards
                vec![], // research: no btc_closes seed needed
                None,   // research: no budget override
            );
            info!("agent subsystems initialized — research trading loop + replay feed running");
        }
        crate::config::Mode::Paper => {
            // ── Per-venue ingest topology (T1408) ────────────────────────────
            // Build the enabled-venue list deterministically: Binance always
            // (R10.2 / backwards compat), Coinbase + Kraken opt-in via
            // `[data.sources.<venue>] enabled = true`.  Iteration order is
            // fixed by Venue's `Ord` impl (Binance < Coinbase < Kraken)
            // so cross-run determinism holds (R7.4).
            //
            // Each enabled venue gets its OWN supervisor task spawned into
            // the runtime JoinSet via [`spawn_venue_supervisor`].  The
            // supervisor wraps the actual feed-consumption in
            // `tokio::task::spawn` and inspects the resulting `JoinError`:
            //   * `is_panic() == true` → log + emit a `FeedReconnect`
            //     audit event with `error_code = "task_panic"` (R14.3) so
            //     the failure is venue-tagged in the journal.  The other
            //     venues' tasks keep running — Q3 / R14.1.
            //   * Normal completion / cancel → log + return.
            //
            // T1409 will add the watchdog respawn loop on top of this
            // skeleton; T1408 only ensures a panic is *isolated* (does not
            // tear down `runtime::run`).
            info!("paper mode — multi-venue WS ingest (paper fills, no real orders)");
            let mut enabled: Vec<(Venue, Arc<dyn MarketDataSource>)> = Vec::new();

            // Binance — always enabled in paper mode (R10.2).
            {
                let ws_url = &config.data.sources.binance.ws_url;
                let feed: Arc<dyn MarketDataSource> =
                    Arc::new(data::BinanceFeed::new(ws_url, ws_url));
                info!(ws = %ws_url, "Binance feed initialized");
                enabled.push((Venue::Binance, feed));
            }

            // Coinbase — operator opts in via config.
            if config.data.sources.coinbase.enabled {
                let ws_url = &config.data.sources.coinbase.ws_url;
                let rest_url = &config.data.sources.coinbase.rest_url;
                let feed: Arc<dyn MarketDataSource> =
                    Arc::new(data::CoinbaseFeed::with_urls(ws_url, rest_url));
                info!(ws = %ws_url, "Coinbase feed initialized");
                enabled.push((Venue::Coinbase, feed));
            }

            // Kraken — operator opts in via config.
            if config.data.sources.kraken.enabled {
                let ws_url = &config.data.sources.kraken.ws_url;
                let rest_url = &config.data.sources.kraken.rest_url;
                let feed: Arc<dyn MarketDataSource> =
                    Arc::new(data::KrakenFeed::with_urls(ws_url, rest_url));
                info!(ws = %ws_url, "Kraken feed initialized");
                enabled.push((Venue::Kraken, feed));
            }

            // Sort by Venue's Ord impl so spawn order is deterministic
            // across runs (R7.4 / determinism non-negotiable).  The
            // ascending order is `Binance < Coinbase < Kraken`.
            enabled.sort_by_key(|(v, _)| *v);

            info!(
                venue_count = enabled.len(),
                "spawning per-venue ingest supervisors"
            );
            // T1409 — shared per-venue last-tick map; updated by every
            // tick observer in the paper-mode supervisors and read by
            // the stale-data watchdog every scan.
            let last_tick: LastTickMap = Arc::new(Mutex::new(HashMap::new()));
            let venue_list: Vec<Venue> = enabled.iter().map(|(v, _)| *v).collect();
            for (venue, feed) in enabled {
                spawn_venue_supervisor(
                    venue,
                    feed,
                    Arc::clone(&bus),
                    Arc::clone(&ledger),
                    feed_symbol.clone(),
                    feed_tf,
                    &mut set,
                    &cancel,
                    Some(Arc::clone(&last_tick)),
                );
            }

            // ── T1409 — stale-data watchdog ──────────────────────────────────
            // Live mode: scan every 1s, threshold 30s by default (Q7).
            // The clock is `Timestamp::now` (wall-clock) — backtest
            // replay never reaches here so the determinism gate
            // (`crates/backtest/` is `MarketHealth`-free) holds.
            //
            // T1410 will plumb the threshold from `[universe].stale_threshold_secs`;
            // for T1409 the default 30s constant lives here.
            spawn_market_health_watchdog(
                Arc::clone(&bus),
                Arc::clone(&last_tick),
                venue_list,
                30, // default stale_threshold_secs (Q7)
                Arc::new(Timestamp::now),
                Duration::from_secs(1),
                &mut set,
                &cancel,
            );

            // ── ADR-0053 / F5-LAUNCH (ADR-0060 § D6): paper-loop supervisor ──────
            // The unified `spawn_trading_loop` is the sole per-bar equity writer
            // in paper mode (Q2=a: retire the idle reconciler mint role).
            // It subscribes directly to the Binance feed, runs the strategy +
            // paper engine per bar, and fire-and-forget persists via
            // `equity_store` (loop-direct persist, Q4=a, ADR-0053 D2).
            //
            // F5-LAUNCH: when `forward_rx.is_some()` (the cockpit path), the
            // supervisor retains the spawn context and hot-swaps the loop on
            // `ForwardCommand::Launch`. When `forward_rx.is_none()` (headless bin /
            // soak), it degenerates to today's inline single spawn — BYTE-IDENTICAL.
            {
                let binance_ws_url = config.data.sources.binance.ws_url.clone();

                // ── R7 retention — nightly purge (closes the ADR-0052 D5 deferral,
                // operator-ratified 2026-06-12). Runs only where rows are minted
                // (paper/live, store = Some); research persists nothing → no task.
                // First tick fires at boot (catch-up after downtime), then daily.
                if let Some(ref store) = equity_store {
                    spawn_equity_purge_task(
                        Arc::clone(store),
                        EQUITY_PURGE_HORIZON_DAYS,
                        EQUITY_PURGE_INTERVAL,
                        &mut set,
                        &cancel,
                    );
                }

                // lesson-card-wiring: BTC daily closes for regime seed.
                //
                // The seed is loaded via `tokio::task::spawn_blocking` so polars
                // (which initialises BLAS + Metal GPU at first use on Apple Silicon)
                // runs on a dedicated blocking thread and never stalls the async
                // reactor.  We `.await` the join handle here — this is called BEFORE
                // `spawn_trading_loop`, i.e. before the paper feed connects to the
                // live WS feed, so the latency (typically < 1 s on warm disk) is
                // acceptable.  The trading loop starts only after the seed is ready.
                //
                // `./data/yahoo` is the conventional yahoo-cache root: the same root
                // used by the lab runner and the `fetch_yahoo_klines` binary.
                // `parquet_root` is `./data/binance`, so we derive yahoo as a sibling.
                let yahoo_root = {
                    let pr = std::path::PathBuf::from(&config.data.historical.parquet_root);
                    pr.parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("yahoo")
                };
                let btc_closes_seed: Vec<(Timestamp, rust_decimal::Decimal)> = {
                    let yahoo_root_clone = yahoo_root.clone();
                    match tokio::task::spawn_blocking(move || {
                        load_btc_daily_closes_for_regime(&yahoo_root_clone, 30)
                    })
                    .await
                    {
                        Ok(seed) => {
                            info!(
                                seed_len = seed.len(),
                                yahoo_root = %yahoo_root.display(),
                                "btc_closes_seed loaded off-hot-path via spawn_blocking"
                            );
                            seed
                        }
                        Err(join_err) => {
                            warn!(
                                error = %join_err,
                                "btc_closes_seed load panicked in blocking thread — using empty seed"
                            );
                            vec![]
                        }
                    }
                };

                if let Some(mut cmd_rx) = forward_rx {
                    // ── F5-LAUNCH supervisor path (cockpit only) ─────────────────
                    // Spawn the initial default loop (pre-F5 byte-identical: BTCUSDT,
                    // build_registry, initial_capital_usdt, no budget) then select!
                    // on cmd_rx for hot-swap commands. The supervisor lives in its
                    // own tokio task inside the shared JoinSet so the run-level
                    // cancel still drains it.
                    //
                    // All the spawn context (bus, ledger, equity_store, config,
                    // btc_closes_seed, reflection_writer, cancel) is moved into the
                    // supervisor task. The task holds the per-loop AbortHandle so
                    // it can drain the old loop before spawning the new one
                    // (no-double-equity-writer guarantee — ADR-0060 § D6 step 2b).

                    let supervisor_bus = Arc::clone(&bus);
                    let supervisor_ledger = Arc::clone(&ledger);
                    let supervisor_equity_store = equity_store;
                    let supervisor_config = Arc::clone(&config);
                    let supervisor_registry = Arc::clone(&registry);
                    let supervisor_reflection_writer = reflection_writer;
                    let supervisor_btc_closes_seed = btc_closes_seed;
                    let supervisor_cancel = cancel.clone();
                    let supervisor_binance_ws_url = binance_ws_url.clone();
                    // F6-PLAN: move plan_tx into the supervisor task so it can
                    // send a ForwardPlan on each Launch (ADR-0062 § D4).
                    let supervisor_plan_tx = plan_tx;

                    set.spawn(async move {
                        // ── Build the initial default loop ────────────────────────
                        // Pre-F5L inline spawn — byte-identical behaviour:
                        // BTCUSDT / build_registry / initial_capital_usdt / None budget.
                        let initial_feed: Arc<dyn MarketDataSource> =
                            Arc::new(data::BinanceFeed::new(
                                &supervisor_binance_ws_url,
                                &supervisor_binance_ws_url,
                            ));
                        let mut inner_set: JoinSet<()> = JoinSet::new();
                        let mut loop_cancel = supervisor_cancel.child_token();

                        let mut loop_abort = spawn_trading_loop(
                            Arc::clone(&initial_feed),
                            Arc::clone(&supervisor_bus),
                            Arc::clone(&supervisor_registry),
                            &supervisor_config.backtest,
                            &supervisor_config.risk,
                            Symbol::new("BTCUSDT"),
                            feed_tf,
                            supervisor_equity_store.as_ref().map(Arc::clone),
                            "paper",
                            &mut inner_set,
                            &loop_cancel,
                            Some(Arc::clone(&supervisor_ledger)),
                            supervisor_reflection_writer.clone(),
                            supervisor_btc_closes_seed.clone(),
                            None, // initial loop: no budget (pre-F5 default)
                        );
                        info!("paper_loop_supervisor: initial default loop spawned");

                        // ── Supervisor select-loop ────────────────────────────────
                        // Runs until the run-level cancel fires.
                        loop {
                            tokio::select! {
                                biased;
                                () = supervisor_cancel.cancelled() => {
                                    // Run-level shutdown — abort the current loop.
                                    loop_abort.abort();
                                    // Drain inner_set (best-effort, bounded).
                                    let drain = async {
                                        while inner_set.join_next().await.is_some() {}
                                    };
                                    let _ = tokio::time::timeout(
                                        std::time::Duration::from_secs(2),
                                        drain,
                                    ).await;
                                    break;
                                }
                                cmd = cmd_rx.recv() => {
                                    let Some(ForwardCommand::Launch(cfg)) = cmd else {
                                        // Channel closed — no more commands; keep the
                                        // current loop running until run-level cancel.
                                        supervisor_cancel.cancelled().await;
                                        loop_abort.abort();
                                        let drain = async {
                                            while inner_set.join_next().await.is_some() {}
                                        };
                                        let _ = tokio::time::timeout(
                                            std::time::Duration::from_secs(2),
                                            drain,
                                        ).await;
                                        break;
                                    };

                                    info!(
                                        strategy = %cfg.strategy.0,
                                        symbol = %cfg.symbol,
                                        budget = %cfg.budget.amount(),
                                        "paper_loop_supervisor: Launch received — hot-swapping loop"
                                    );

                                    // ── F5b REORDER (Task 2): build FIRST, tear down AFTER. ──
                                    //
                                    // OLD order: cancel → drain → build → if Err skip (stranded).
                                    // NEW order: build → if Ok: cancel + drain + spawn;
                                    //            if Err: keep old loop + error-log + continue.
                                    //
                                    // This prevents the Live view from going dead when the TOML
                                    // load fails — the old loop keeps running, so the operator
                                    // sees fills/equity until they retry with a valid strategy.
                                    //
                                    // No-double-equity-writer guarantee is preserved: the new
                                    // loop is spawned ONLY AFTER the old loop has been fully
                                    // drained (same timing as before; only the Err path changed).

                                    // Step (a): build registry for the selected strategy.
                                    // NO silent SMA fallback — that is the F5b anti-fake gate.
                                    let new_registry = match build_registry_for(
                                        &supervisor_config,
                                        Some(&cfg),
                                    ) {
                                        Ok(r) => r,
                                        Err(e) => {
                                            error!(
                                                strategy = %cfg.strategy.0,
                                                error = %e,
                                                "paper_loop_supervisor: strategy load FAILED — \
                                                 keeping OLD loop running (F5b anti-fake gate: no SMA proxy, \
                                                 no teardown on build failure)"
                                            );
                                            // The old loop is still alive — do NOT cancel it.
                                            // Continue waiting for the next Launch command.
                                            continue;
                                        }
                                    };

                                    // Step (b): cancel the current loop's child token AFTER
                                    // the build succeeded (no-teardown-on-failure guarantee).
                                    loop_cancel.cancel();

                                    // Step (c): abort + await old loop drain
                                    // (no-double-equity-writer serialisation — the loop
                                    // is the sole per-bar equity writer in paper mode).
                                    loop_abort.abort();
                                    let drain = async {
                                        while inner_set.join_next().await.is_some() {}
                                    };
                                    if tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        drain,
                                    )
                                    .await
                                    .is_err()
                                    {
                                        warn!(
                                            "paper_loop_supervisor: old loop drain \
                                             exceeded 5s — proceeding (abort already fired)"
                                        );
                                    }
                                    info!(
                                        "paper_loop_supervisor: old loop drained — \
                                         spawning selected-strategy loop"
                                    );

                                    // Step (d): fresh per-loop cancel token under the
                                    // run-level cancel (so the next abort is scoped to
                                    // this new loop, not the whole runtime).
                                    loop_cancel = supervisor_cancel.child_token();
                                    inner_set = JoinSet::new();

                                    // Step (d.5 → now e.5): F6-PLAN — produce and send ForwardPlan.
                                    //
                                    // We read the PlanDescribe from the NEW registry
                                    // (same resolved engine the loop will run — R7 by
                                    // construction). The latest-bar close/ts are
                                    // unavailable at Launch time (the live feed hasn't
                                    // consumed any bar yet), so we use a sentinel close of
                                    // Decimal::ONE (avoids division-by-zero; the plan
                                    // will update once a real bar lands).
                                    //
                                    // ADR-0062 § D3: "using the latest bar (close, ts) the
                                    // loop is about to consume" — for the live cockpit
                                    // path, we don't have a bar yet at launch time.  We
                                    // use a conservative sentinel and the UI displays
                                    // "at the last close (pending first bar)" until a real
                                    // bar updates the plan (T-D2.4 intent preserved;
                                    // the btc_closes_seed gives a usable close for paper
                                    // mode where we have historical data).
                                    if let Some(ref plan_tx_ref) = supervisor_plan_tx {
                                        // Use the last available close from the BTC seed if
                                        // present; otherwise fall back to a sentinel price.
                                        let seed_close = supervisor_btc_closes_seed
                                            .last()
                                            .and_then(|(ts, close)| {
                                                trading_core::Price::new(*close).ok().map(|p| (p, *ts))
                                            });
                                        let (last_close, last_bar_ts) = if let Some((p, ts)) = seed_close {
                                            (p, ts)
                                        } else {
                                            // Sentinel: 1 USDT (plan will be updated once a bar lands).
                                            let fallback_price = trading_core::Price::new(
                                                rust_decimal::Decimal::ONE
                                            ).unwrap_or(
                                                // Price::new(1) always succeeds but be defensive.
                                                trading_core::Price::new(rust_decimal::Decimal::from(50_000u32)).unwrap()
                                            );
                                            (fallback_price, Timestamp::now())
                                        };

                                        // Cast the new registry's strategy to &dyn PlanDescribe.
                                        // The registry exposes `Box<dyn Strategy>` per symbol;
                                        // we need to reach the concrete type. Since we hold
                                        // the fresh `new_registry` Arc, use a helper that
                                        // rebuilds a describe_plan from scratch for the id.
                                        let fwd_plan = crate::plan::build_forward_plan_from_registry(
                                            &supervisor_config,
                                            &cfg,
                                            last_close,
                                            last_bar_ts,
                                            7, // default horizon_days (display-only)
                                        );
                                        if let Some(plan) = fwd_plan
                                            && let Err(e) = plan_tx_ref.try_send(plan) {
                                            warn!(
                                                error = %e,
                                                "paper_loop_supervisor: ForwardPlan send failed \
                                                 (channel full or closed — non-fatal)"
                                            );
                                        }
                                    }

                                    // Step (f): fresh feed on the selected coin.
                                    let new_feed: Arc<dyn MarketDataSource> = Arc::new(
                                        data::BinanceFeed::new(
                                            &supervisor_binance_ws_url,
                                            &supervisor_binance_ws_url,
                                        ),
                                    );

                                    // OQ-5: wire Some(reflection_writer) for the forward
                                    // (swapped) loop — recommended default.
                                    // The writer derives Clone so re-use is free.
                                    let fwd_reflection = supervisor_reflection_writer.clone();

                                    // Step (f): spawn the new loop with the selection.
                                    // Reuses the same bus / ledger / equity_store /
                                    // boot_id so all downstream consumers (Live tape,
                                    // durable store, audit trail) carry forward.
                                    loop_abort = spawn_trading_loop(
                                        new_feed,
                                        Arc::clone(&supervisor_bus),
                                        new_registry,
                                        &supervisor_config.backtest,
                                        &supervisor_config.risk,
                                        cfg.symbol.clone(),
                                        feed_tf,
                                        supervisor_equity_store
                                            .as_ref()
                                            .map(Arc::clone),
                                        "paper",
                                        &mut inner_set,
                                        &loop_cancel,
                                        Some(Arc::clone(&supervisor_ledger)),
                                        fwd_reflection,
                                        supervisor_btc_closes_seed.clone(),
                                        Some(cfg.budget), // F5: budget cap + starting capital
                                    );
                                    info!(
                                        strategy = %cfg.strategy.0,
                                        symbol = %cfg.symbol,
                                        budget = %cfg.budget.amount(),
                                        "paper_loop_supervisor: forward loop spawned (real launch)"
                                    );
                                }
                            }
                        }
                    });
                    info!("paper_loop_supervisor spawned (cockpit forward-launch path)");
                } else {
                    // ── No supervisor (headless / soak path) — byte-identical ──────
                    // `forward_rx = None`: spawn the initial loop inline, exactly as
                    // the pre-F5L code did. No select-loop. ADR-0053 determinism +
                    // 119/119 anchors hold by construction.
                    let paper_feed: Arc<dyn MarketDataSource> =
                        Arc::new(data::BinanceFeed::new(&binance_ws_url, &binance_ws_url));
                    let _ = spawn_trading_loop(
                        paper_feed,
                        Arc::clone(&bus),
                        Arc::clone(&registry),
                        &config.backtest,
                        &config.risk,
                        feed_symbol.clone(),
                        feed_tf,
                        equity_store, // Some(store) — the loop-direct persist gate (ADR-0053 D2)
                        "paper",
                        &mut set,
                        &cancel,
                        Some(Arc::clone(&ledger)), // paper: persist fills to journal_transactions
                        reflection_writer,         // paper: enqueue lesson cards on close
                        btc_closes_seed,           // paper: seed for regime classification
                        None,                      // no budget override (legacy default)
                    );
                    info!("paper trading loop spawned (live feed → paper engine → equity store)");
                }
            }
        }
    }

    // ── F9-NARRATION — triggered narration task (ADR-0064 § D3) ─────────────────
    // Receives a `NarrationRequest` from the iced thread (via the cockpit's
    // "Explain" action), calls `generate_narration(provider, facts)` on the
    // async runtime, and returns the `NarrationOutcome` over the return channel.
    //
    // `None` channels → byte-identical to today (headless / soak unaffected —
    // the absent-channel path is structurally guaranteed by the Option guards).
    if let (Some(mut req_rx), Some(outcome_tx)) = (narration_request_rx, narration_outcome_tx) {
        // The LLM provider is built at agent boot (gated on cfg.llm.enabled).
        // We clone the config's provider handle here; if absent the task still
        // runs (receives requests + sends FellBack for each) so the UI never hangs.
        let llm_provider: Option<Arc<dyn llm::LlmProvider>> = if config.llm.enabled {
            // The production path: the factory was called at boot and stored
            // the provider on the config. For now we re-build a minimal provider
            // from the config so the task can operate. The preferred pattern is
            // to pass an `Option<Arc<dyn LlmProvider>>` into RunHandles (the F9
            // v0.3 hardening point); for MVP we use `None` and let the task
            // FellBack (the honest fallback — the bake-off result is always shown).
            //
            // TODO(F9-v0.3): plumb the boot-built `Arc<dyn LlmProvider>` into
            // RunHandles so the narration task uses the full BudgetedProvider stack.
            None
        } else {
            None
        };
        let narration_cancel = cancel.child_token();
        set.spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = narration_cancel.cancelled() => break,
                    req = req_rx.recv() => {
                        let Some(crate::narration::NarrationRequest { facts }) = req else {
                            // Channel closed — exit.
                            break;
                        };
                        let outcome = if let Some(ref provider) = llm_provider {
                            crate::narration::generate_narration(provider, &facts).await
                        } else {
                            tracing::debug!(
                                winner = %facts.winner_id,
                                "narration_task: provider not enabled — FellBack"
                            );
                            crate::narration::NarrationOutcome::FellBack
                        };
                        if outcome_tx.send(outcome).await.is_err() {
                            tracing::warn!("narration_task: outcome channel closed — stopping");
                            break;
                        }
                    }
                }
            }
        });
        info!("narration_task spawned (F9 — cockpit narration path)");
    }
    // else: headless / soak — byte-identical to pre-F9 (no narration task spawned).

    // ── Phase 3 T1707 — risk-telemetry publisher (Q3) ─────────────────────────
    // 1 Hz tick. v1.5b plumbing-only state — the snapshot returns a
    // deterministic placeholder; the actual risk-engine wiring lands in a
    // follow-up. The bus channel is the load-bearing surface tested by
    // the cockpit integration test.
    spawn_risk_telemetry_publisher(
        Arc::clone(&bus),
        Arc::new(default_risk_telemetry_stub),
        Duration::from_secs(1),
        &mut set,
        &cancel,
    );

    // ── Mode-broadcast forwarder (T905) ───────────────────────────────────────
    // Bridge `KillSwitch::subscribe()` (the kill-switch internal mode
    // channel) into the bus's `mode` channel so the cockpit's
    // mode-stream subscriber observes `AgentMode::Halted` after a
    // trip.  Kill switch is intentionally unaware of the bus; this
    // forwarder is the single writer to `bus.publish_mode(...)`.
    spawn_mode_forwarder(
        Arc::clone(&kill_switch),
        Arc::clone(&bus),
        &mut set,
        &cancel,
    );

    // ── Serve until cancelled or halted ───────────────────────────────────────
    info!("agent running — serving /metrics, watching for halt file");
    let mut mode_rx = kill_switch.subscribe();
    tokio::select! {
        () = cancel.cancelled() => {
            info!("cancel received — shutting down");
        }
        mode = mode_rx.recv() => {
            if let Ok(AgentMode::Halted { reason }) = mode {
                warn!(reason = %reason, "agent halted");
            }
        }
    }

    // Propagate cancellation to every spawned task, then drain.  We
    // use a wall-clock bound (2 s) so a misbehaving task can't pin the
    // shutdown — risk #3 in the architect's design.
    cancel.cancel();
    let drain = async { while set.join_next().await.is_some() {} };
    if (tokio::time::timeout(std::time::Duration::from_secs(2), drain).await).is_err() {
        warn!("shutdown_deadline_exceeded — JoinSet drain exceeded 2s; aborting remaining tasks");
        set.abort_all();
        // Best-effort drain of aborts (each abort returns JoinError; ignore).
        while set.join_next().await.is_some() {}
    }

    info!("agent stopped");
    Ok(())
}

/// Load recent BTC daily close prices from the Yahoo parquet cache for
/// regime classification in paper-mode lesson cards.
///
/// Returns up to `days` days of BTC-USD 1d closes sorted ascending by
/// timestamp.  Any I/O error is warn-logged and an empty vec returned —
/// the lesson-card generator then fails `classify_regime` gracefully
/// (warn-logged, no card generated) rather than blocking the loop.
///
/// `yahoo_cache_root` is typically `./data/yahoo` (from the agent config
/// `parquet_root` parent, or a conventional path).
///
/// If the current month is not in the cache yet (cache is updated monthly),
/// we fall back to the end-of-previous-month as `end_ms` so the load
/// succeeds with the most recently available data.
#[must_use]
pub fn load_btc_daily_closes_for_regime(
    yahoo_cache_root: &std::path::Path,
    days: u32,
) -> Vec<(Timestamp, rust_decimal::Decimal)> {
    use data::yahoo::{Interval, YahooBarSource};
    use time::OffsetDateTime;

    let source = YahooBarSource::new(yahoo_cache_root.to_path_buf());
    let now = OffsetDateTime::now_utc();
    let now_ms: i64 = (now.unix_timestamp_nanos() / 1_000_000)
        .try_into()
        .unwrap_or(i64::MAX);
    let start_ms: i64 = now_ms.saturating_sub(i64::from(days) * 86_400_000);

    // Helper: build epoch-ms for the 1st of a given year-month.
    let month_start_ms = |y: i32, m: u8| -> i64 {
        time::Date::from_calendar_date(
            y,
            time::Month::try_from(m).unwrap_or(time::Month::January),
            1,
        )
        .ok()
        .map(|d| time::PrimitiveDateTime::new(d, time::Time::MIDNIGHT).assume_utc())
        .map(|dt| (dt.unix_timestamp_nanos() / 1_000_000) as i64)
        .unwrap_or(now_ms)
    };

    // Try progressively: today → start of current month → start of previous month.
    // This ensures we land on a range covered by complete monthly parquet files.
    let cur_y = now.year();
    let cur_m = now.month() as u8;
    let (prev_y, prev_m) = if cur_m == 1 {
        (cur_y - 1, 12u8)
    } else {
        (cur_y, cur_m - 1)
    };

    let try_load = |end: i64| source.load_cached("BTC-USD", Interval::Days1, start_ms, end);

    let result = try_load(now_ms)
        .or_else(|_| try_load(month_start_ms(cur_y, cur_m)))
        .or_else(|_| try_load(month_start_ms(prev_y, prev_m)));

    match result {
        Ok(loaded) => {
            let mut out: Vec<(Timestamp, rust_decimal::Decimal)> = loaded
                .bars
                .iter()
                .map(|b| (b.close_ts, b.close.get()))
                .collect();
            out.sort_by_key(|(ts, _)| ts.inner());
            info!(count = out.len(), "btc_daily_closes loaded for regime seed");
            out
        }
        Err(e) => {
            warn!(
                error = %e,
                yahoo_root = %yahoo_cache_root.display(),
                "btc_daily_closes load failed — lesson cards will skip regime (warn-only)"
            );
            vec![]
        }
    }
}

/// Build a [`exec::PaperEnginePublisher`] backed by the agent's
/// [`EventBus`] (T903a-glue — live-cockpit-unified).
///
/// `crates/exec/src/paper.rs::PaperEnginePublisher::with_publisher`
/// accepts an `Arc<dyn FillPublisher>`.  The `EventBus` impls
/// `FillPublisher` (see `crates/agent/src/bus.rs`); this helper
/// constructs the publisher with one allocation so the live-mode
/// caller hands a single value into whatever paper-engine task graph
/// emerges.  Backtest callers continue to use
/// `PaperEnginePublisher::new()` (the no-op `NullPublisher` path) so
/// the deterministic backtest report bytes stay byte-identical (R15 /
/// V5 anchor invariant).
///
/// Returning the publisher rather than threading the bus directly
/// keeps the dep-graph boundary the architect's design Q-resolution
/// row Q6 calls for: `crates/exec/` knows nothing about
/// `crates/agent/`; the only place those two crates meet is here +
/// the `impl FillPublisher for EventBus` block.
#[must_use]
pub fn paper_engine_publisher(bus: Arc<EventBus>) -> exec::PaperEnginePublisher {
    // Unsize-coerce via `as`; a bare assignment doesn't unsize, and
    // `Arc::clone` would infer the wrong source type.
    let publisher: Arc<dyn exec::FillPublisher> = bus as Arc<dyn exec::FillPublisher>;
    exec::PaperEnginePublisher::with_publisher(publisher)
}

/// Spawn the bar/tick "tap" tasks that re-publish each item from the
/// market-data feed onto the corresponding [`EventBus`] channel
/// (T903b — live-cockpit-unified).
///
/// Two tasks are spawned into `set`:
/// 1. **bars tap** — subscribes to `feed.subscribe_bars(symbol, tf)`
///    and calls `bus.publish_bar(bar.clone())` for each.
/// 2. **ticks tap** — subscribes to `feed.subscribe_trades(symbol)`
///    and calls `bus.publish_tick(tick.clone())` for each.
///
/// Both tasks honor `cancel.cancelled()` and exit cleanly on shutdown
/// or when the upstream stream ends (e.g. replay finished).  Stream
/// errors are warn-logged and the task continues so a transient feed
/// hiccup never tears down the runtime.
///
/// If `feed.subscribe_bars` / `subscribe_trades` returns an error at
/// startup (e.g. replay parquet not on disk in research mode), the
/// failure is warn-logged and that tap is silently skipped — the
/// runtime stays up so other subsystems (heartbeat, watcher, kill
/// switch) can keep functioning.
pub(crate) async fn spawn_feed_taps<S: MarketDataSource + ?Sized>(
    feed: &S,
    bus: Arc<EventBus>,
    symbol: Symbol,
    tf: Timeframe,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
) {
    spawn_feed_taps_with_observer(feed, bus, symbol, tf, set, cancel, None).await;
}

/// Variant of [`spawn_feed_taps`] that calls an optional `tick_observer`
/// closure on each tick *before* re-publishing it on the bus.
///
/// T1409 wires this from the per-venue paper-mode supervisor with a
/// closure that records the tick's `local_recv_ts` into the shared
/// [`LastTickMap`] so the stale-data watchdog can see venue activity.
/// Research mode keeps using [`spawn_feed_taps`] (observer = `None`).
pub(crate) async fn spawn_feed_taps_with_observer<S: MarketDataSource + ?Sized>(
    feed: &S,
    bus: Arc<EventBus>,
    symbol: Symbol,
    tf: Timeframe,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
    tick_observer: Option<TickObserver>,
) {
    // ── bars tap ──────────────────────────────────────────────────────────────
    match feed.subscribe_bars(symbol.clone(), tf).await {
        Ok(mut stream) => {
            let bus_b = Arc::clone(&bus);
            let cancel_b = cancel.child_token();
            let symbol_b = symbol.clone();
            set.spawn(async move {
                info!(symbol = %symbol_b, tf = %tf, "bars_tap started");
                loop {
                    tokio::select! {
                        () = cancel_b.cancelled() => break,
                        next = stream.next() => match next {
                            Some(Ok(bar)) => bus_b.publish_bar(bar),
                            Some(Err(e)) => {
                                warn!(error = %e, "bars_tap stream error (continuing)");
                            }
                            None => {
                                debug!("bars_tap stream ended");
                                break;
                            }
                        }
                    }
                }
                info!("bars_tap stopped");
            });
        }
        Err(e) => warn!(error = %e, "bars_tap subscribe failed (skipped)"),
    }

    // ── ticks tap ─────────────────────────────────────────────────────────────
    match feed.subscribe_trades(symbol.clone()).await {
        Ok(mut stream) => {
            let bus_t = Arc::clone(&bus);
            let cancel_t = cancel.child_token();
            let symbol_t = symbol;
            let observer = tick_observer.clone();
            set.spawn(async move {
                info!(symbol = %symbol_t, "ticks_tap started");
                loop {
                    tokio::select! {
                        () = cancel_t.cancelled() => break,
                        next = stream.next() => match next {
                            Some(Ok(tick)) => {
                                // T1409 — observe the tick (e.g. update the
                                // per-venue last-tick map) before broadcasting.
                                if let Some(obs) = observer.as_ref() {
                                    obs(&tick);
                                }
                                bus_t.publish_tick(tick);
                            }
                            Some(Err(e)) => {
                                debug!(error = %e, "ticks_tap stream error (continuing)");
                            }
                            None => {
                                debug!("ticks_tap stream ended");
                                break;
                            }
                        }
                    }
                }
                info!("ticks_tap stopped");
            });
        }
        Err(e) => warn!(error = %e, "ticks_tap subscribe failed (skipped)"),
    }
}

/// Spawn the research-mode strategy + paper-engine trading loop.
///
/// Subscribes **directly to the feed** (not through the broadcast bus) via
/// `feed.subscribe_bars(...)`, then runs each bar through the strategy
/// registry, converts signals to orders using the risk sizer, executes them
/// through the paper matching engine, and publishes fills + positions + PnL
/// snapshots to the bus so the Live dashboard panels have data to render.
///
/// ## Why direct feed subscription (not bus.bars())
///
/// In fast-replay mode (`replay_fast = true`), `bars_tap` publishes all
/// ~500 k bars in a single tokio turn before any other task gets scheduled.
/// The broadcast channel capacity (`bars_capacity = 1024`) is far smaller
/// than the bar count, so `bus.bars()` receivers that subscribe after
/// `bars_tap` has started will lag and miss almost every bar.  Subscribing
/// directly to the feed's `BoxStream` avoids the broadcast channel entirely
/// — the trading loop and `bars_tap` both call `subscribe_bars` independently
/// (they receive separate, independent streams) and each processes every bar.
///
/// ## Why this task exists
///
/// Before this fix, research mode only published raw bars to the bus
/// but never executed strategies or generated fills. The fills tape and
/// positions panels in the Live view therefore hung forever on their
/// "Connecting…" / "Loading positions…" placeholders. The pattern is
/// taken directly from `backtest::scenarios::sma_composed_run::run`.
///
/// ## Determinism
///
/// - Paper engine seeded with `0x00C0_FFEE` (same as `PaperEngine::with_default_seed`).
/// - No `SystemTime::now()` in the trade-decision path; `bar.close_ts` is
///   used as the fill timestamp (same as `backtest::paper::PaperEngine::step`).
/// - This task is NOT reached by backtest replays
///   (`crates/backtest/` never calls `runtime::run`), so anchor reports
///   are unaffected (determinism gate holds).
///
/// ## Risk sizing
///
/// Uses `risk::size_and_validate` with the agent config's `fixed_fraction`
/// and `per_symbol_exposure_cap` — same as the Lab backtest runner.
///
/// The task exits when the `cancel` token fires OR when the bar stream ends
/// (replay completed / feed stopped).
/// Spawn the research-mode trading loop with an explicitly-provided feed.
///
/// This overload is `pub` for integration tests that need to inject a
/// synthetic paced feed (e.g. `data::MockFeed` with a fixed interval) to
/// verify that a **late bus subscriber** — one that subscribes AFTER the feed
/// starts emitting — still receives fills/positions/pnl events (the core
/// late-subscriber regression guard for the cockpit UI).
///
/// Production code calls this from `runtime::run`; tests call it directly
/// alongside a controlled tokio runtime (see
/// `crates/agent/tests/paced_replay_late_subscriber.rs`).
///
/// The `doc(hidden)` marker prevents this from cluttering the public API
/// surface in `cargo doc`; it is still `pub` for the integration test crate.
// `too_many_arguments` is intentional — the function mirrors the
// backtest::scenarios::sma_composed_run pattern and all parameters are
// required.  No builder pattern overhead is warranted for a spawn helper.
//
// ADR-0053 (paper-mode-equity-wiring): renamed from `spawn_research_trading_loop`
// and widened with `equity_store: Option<Arc<dyn audit::LiveEquityStore>>` +
// `mode_label: &'static str`.  Research passes `None` (byte-identical behavior);
// paper passes `Some(store)` + `"paper"` so the loop-direct persist seam (A2)
// fires per bar without any `if mode != Research` inside the body.
//
// lesson-card-wiring: extended with `ledger`, `reflection_writer`, and
// `btc_closes_seed` so paper mode writes `journal_transactions` on every fill
// and enqueues a `LessonCardWriteRequest` on every position-close fill.
// Research mode passes `None, None, vec![]` — byte-identical with pre-extension.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::let_and_return)] // abort_handle binding spans an async block too large to inline
#[doc(hidden)]
pub fn spawn_trading_loop(
    feed: Arc<dyn MarketDataSource>,
    bus: Arc<EventBus>,
    registry: Arc<strategy::StrategyRegistry>,
    backtest_cfg: &crate::config::BacktestConfig,
    risk_cfg: &crate::config::RiskConfig,
    feed_symbol: Symbol,
    feed_tf: Timeframe,
    equity_store: Option<Arc<dyn audit::LiveEquityStore>>, // NEW (ADR-0053 A1/A2)
    mode_label: &'static str,                              // NEW ("paper" | "research")
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
    // lesson-card-wiring additions: None/None/vec![] for research (no-op)
    ledger: Option<Arc<audit::Ledger>>,
    reflection_writer: Option<reflection::ReflectionWriter>,
    btc_closes_seed: Vec<(Timestamp, rust_decimal::Decimal)>,
    // F5 budget override (ADR-0060 § D5): Some(b) → starting capital = b +
    // FixedFractionSizer::with_budget_cap(fraction, b). None → pre-F5 byte-
    // identical path (initial_capital_usdt + FixedFractionSizer::new).
    budget: Option<trading_core::Money<trading_core::Usdt>>,
) -> tokio::task::AbortHandle {
    use backtest::MatchingEngine as _;
    use backtest::paper::{FillPriceMode, MatchConfig, PaperEngine};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use trading_core::{
        Money, Order, OrderKind, PnlSnapshot, Position, Quantity, RiskLimits, Side, SignalKind,
        TimeInForce, Timestamp, Usdt,
    };

    // Build the paper engine config from the agent config.
    let match_config = MatchConfig {
        slippage_bps: backtest_cfg.slippage_bps,
        taker_fee_bps: backtest_cfg.taker_fee_bps,
        maker_fee_bps: backtest_cfg.maker_fee_bps,
        fill_price_mode: FillPriceMode::BarClose,
    };

    // Build the risk sizer + limits from the agent config.
    // F5: when `budget` is `Some(b)`, use `with_budget_cap` so the deployed
    // notional is capped at the budget (the F4 hard-cap modifier). When
    // `None`, `new(fraction)` is used — byte-identical to the pre-F5 path.
    let fraction = Decimal::try_from(risk_cfg.sizing.fixed_fraction).unwrap_or(dec!(0.10));
    let sizer = if let Some(b) = budget {
        risk::FixedFractionSizer::with_budget_cap(fraction, b)
    } else {
        risk::FixedFractionSizer::new(fraction)
    };
    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: Decimal::try_from(risk_cfg.per_symbol_exposure_cap)
            .unwrap_or(dec!(0.40)),
        price_sanity_band: dec!(0.10), // 10% band — same default as RiskLimits::default()
        portfolio_exposure_cap: None,  // no portfolio cap in research mode
    };

    // F5 initial capital: `Some(b)` → start cash at the budget amount so the
    // published equity IS the budget equity (every downstream consumer —
    // LiveEquityStore, EventBus PnL, the Live screen — naturally tracks the
    // €200 run). `None` → legacy `initial_capital_usdt` (byte-identical).
    // ADR-0060 § D5.
    let initial_capital = if let Some(b) = budget {
        b.amount()
    } else {
        Decimal::try_from(backtest_cfg.initial_capital_usdt).unwrap_or(dec!(100_000))
    };

    // Build the publisher that routes fills → bus.fills() + bus.positions().
    let publisher = paper_engine_publisher(Arc::clone(&bus));

    // Clone bus for PnL publishing inside the async task.
    let pnl_bus = Arc::clone(&bus);

    let cancel_loop = cancel.child_token();
    // F5L.2: return the AbortHandle so the paper_loop_supervisor can drain the old
    // loop before spawning the new one (no-double-equity-writer guarantee).
    // `JoinSet::spawn` returns `AbortHandle` directly.
    let abort_handle = set.spawn(async move {
        info!(mode = mode_label, "trading_loop started");

        // Subscribe directly to the feed stream (bypasses the broadcast bus so
        // fast-replay bars are never dropped due to channel lag).
        let mut bar_stream = match feed.subscribe_bars(feed_symbol.clone(), feed_tf).await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, mode = mode_label, "trading_loop: subscribe_bars failed — no trading");
                return;
            }
        };

        let mut engine = PaperEngine::new(match_config, 0x00C0_FFEE);
        let mut position = Position::empty(feed_symbol.clone());
        let mut cash = initial_capital;
        let mut realized_pnl = Decimal::ZERO;
        let mut cost_basis = Decimal::ZERO;
        let mut fill_count = 0usize;
        let mut bar_count = 0u32;

        // lesson-card-wiring: track open-side trade state so we can build
        // a ClosedTrade when a sell closes the position.
        let mut open_tx_id: Option<String> = None;
        let mut opened_at: Option<Timestamp> = None;
        let mut open_capital: Option<Money<Usdt>> = None;
        let mut open_strategy_id: Option<trading_core::StrategyId> = None;
        let mut bar_count_at_open: u32 = 0;

        // Accumulate live bars for btc_closes (regime classifier).
        // Pre-seeded with the caller-supplied historical seed so that
        // classify_regime has at least 7d of data even on a short soak.
        let mut btc_closes: Vec<(Timestamp, Decimal)> = btc_closes_seed;

        loop {
            tokio::select! {
                () = cancel_loop.cancelled() => break,
                next = bar_stream.next() => {
                    let bar = match next {
                        Some(Ok(b)) => b,
                        Some(Err(e)) => {
                            warn!(error = %e, mode = mode_label, "trading_loop bar stream error (continuing)");
                            continue;
                        }
                        None => {
                            debug!(mode = mode_label, "trading_loop bar stream ended");
                            break;
                        }
                    };

                    let mark = bar.close.get();
                    position.last_mark = bar.close;
                    bar_count += 1;

                    // Accumulate bar close prices for regime classification.
                    btc_closes.push((bar.close_ts, mark));

                    // Equity at start of bar (for risk sizing).
                    let equity = cash + position.base_qty * mark;

                    let signals = registry.on_bar(&bar);
                    let mut orders: Vec<Order> = Vec::new();

                    for sig in &signals {
                        let desired_side: Option<Side> = match sig.kind {
                            SignalKind::Buy if position.base_qty <= Decimal::ZERO => {
                                Some(Side::Buy)
                            }
                            SignalKind::Sell if position.base_qty > Decimal::ZERO => {
                                Some(Side::Sell)
                            }
                            _ => None,
                        };

                        if let Some(side) = desired_side {
                            let order_opt = match side {
                                Side::Buy => risk::size_and_validate(
                                    &sizer,
                                    sig.strategy_id.clone(),
                                    sig.symbol.clone(),
                                    side,
                                    Money::<Usdt>::from_decimal(equity),
                                    bar.close,
                                    &position,
                                    &risk_limits,
                                )
                                .ok(),
                                Side::Sell => Quantity::new(position.base_qty)
                                    .ok()
                                    .filter(|q| q.get() > Decimal::ZERO)
                                    .and_then(|q| {
                                        Order::new(
                                            sig.strategy_id.clone(),
                                            sig.symbol.clone(),
                                            Side::Sell,
                                            q,
                                            OrderKind::Market,
                                            TimeInForce::Ioc,
                                            &position,
                                            bar.close,
                                            &risk_limits,
                                            equity,
                                        )
                                        .ok()
                                    }),
                            };
                            if let Some(ord) = order_opt {
                                orders.push(ord);
                            }
                        }
                    }

                    // lesson-card-wiring: capture strategy_ids before orders are moved.
                    // There is at most one order per bar (SMA crossover); we take
                    // the first strategy_id (or "(unattributed)" if absent).
                    let orders_strategy_id: Option<trading_core::StrategyId> = orders
                        .first()
                        .map(|o| o.strategy_id().clone());

                    if !orders.is_empty()
                        && let Ok(fills) = engine.step(&bar, orders).await
                    {
                        for fill in &fills {
                            // lesson-card-wiring: capture pre-update position qty
                            // to detect position-close on sell fills.
                            let pre_fill_qty = position.base_qty;

                            // Update cash + position (mirror of sma_composed_run.rs).
                            match fill.side {
                                Side::Buy => {
                                    let cost = fill.qty.get() * fill.price.get();
                                    cash -= cost + fill.fee.amount();
                                    position.base_qty += fill.qty.get();
                                    cost_basis += cost;
                                }
                                Side::Sell => {
                                    let proceeds = fill.qty.get() * fill.price.get();
                                    cash += proceeds - fill.fee.amount();
                                    let closed =
                                        fill.qty.get().min(position.base_qty);
                                    position.base_qty =
                                        (position.base_qty - closed).max(Decimal::ZERO);
                                    // Proportional cost-basis reduction.
                                    let avg_cost = if position.base_qty == Decimal::ZERO {
                                        cost_basis
                                    } else {
                                        cost_basis
                                            * (closed / (position.base_qty + closed))
                                    };
                                    realized_pnl +=
                                        proceeds - avg_cost - fill.fee.amount();
                                    cost_basis =
                                        (cost_basis - avg_cost).max(Decimal::ZERO);
                                }
                            }
                            // Publish fill + position → Live tape / positions panels.
                            publisher.on_fill(fill, &position);
                            fill_count += 1;
                            {
                                let notional = fill.qty.get() * fill.price.get();
                                let running_equity = cash + position.base_qty * fill.price.get();
                                // Detailed fill log: `debug!` for every bar, `info!` every 100.
                                // notional_usdt = qty × price (the real clip size, ~$10k for
                                // 10%-of-$100k fixed fraction); fee_usdt ≈ 4bps × notional.
                                tracing::debug!(
                                    fill_count,
                                    mode = mode_label,
                                    side = ?fill.side,
                                    price = %fill.price.get(),
                                    qty = %fill.qty.get(),
                                    notional_usdt = %notional,
                                    fee_usdt = %fill.fee.amount(),
                                    running_equity_usdt = %running_equity,
                                    "trading_loop fill detail"
                                );
                                if fill_count <= 5 || fill_count.is_multiple_of(100) {
                                    info!(
                                        fill_count,
                                        mode = mode_label,
                                        side = ?fill.side,
                                        price = %fill.price.get(),
                                        qty = %fill.qty.get(),
                                        notional_usdt = %notional,
                                        fee_usdt = %fill.fee.amount(),
                                        "trading_loop fill"
                                    );
                                }
                            }

                            // lesson-card-wiring: persist fill to journal_transactions.
                            // Fire-and-forget (spawned) — never blocks the loop.
                            if let Some(ref jledger) = ledger {
                                let fill_clone = fill.clone();
                                let ledger_clone = Arc::clone(jledger);
                                let strat_id = orders_strategy_id
                                    .as_ref()
                                    .map(|s| s.0.to_string());
                                tokio::spawn(async move {
                                    if let Err(e) = audit::journal::post_fill_with_signal(
                                        &ledger_clone,
                                        &fill_clone,
                                        trading_core::Venue::Binance,
                                        strat_id.as_deref(),
                                        None,
                                    ).await {
                                        warn!(error = %e, "trading_loop: post_fill_with_signal failed (non-fatal)");
                                    }
                                });
                            }

                            // Capture open-side state on buy fills.
                            if fill.side == Side::Buy
                                && pre_fill_qty <= Decimal::ZERO
                            {
                                // Opening a new position.
                                open_tx_id = Some(fill.id.to_string());
                                opened_at = Some(fill.venue_ts);
                                open_capital = Some(Money::<Usdt>::from_decimal(
                                    cash + (position.base_qty * fill.price.get()),
                                ));
                                open_strategy_id = orders_strategy_id.clone();
                                bar_count_at_open = bar_count;
                            }

                            // Enqueue lesson card on position-close sell fills.
                            if fill.side == Side::Sell
                                && pre_fill_qty > Decimal::ZERO
                                && position.base_qty <= Decimal::ZERO
                            {
                                // Position just closed — construct and enqueue a lesson card req.
                                let strategy_for_card = orders_strategy_id
                                    .clone()
                                    .or_else(|| open_strategy_id.clone())
                                    .unwrap_or_else(|| trading_core::StrategyId::new("(unattributed)"));
                                if let (
                                    Some(rw),
                                    Some(oat),
                                    Some(ocap),
                                    Some(oid),
                                ) = (
                                    &reflection_writer,
                                    &opened_at,
                                    &open_capital,
                                    &open_tx_id,
                                ) {
                                    let symbol_or_pair =
                                        reflection::SymbolOrPair::Single(fill.symbol.clone());
                                    let signed_pnl = Money::<Usdt>::from_decimal(
                                        realized_pnl,
                                    );
                                    let holding_bars = bar_count.saturating_sub(bar_count_at_open);
                                    let closed_trade = reflection::ClosedTrade {
                                        close_transaction_id: fill.id.to_string(),
                                        open_transaction_id: oid.clone(),
                                        symbol_or_pair,
                                        strategy_id: strategy_for_card,
                                        signed_pnl,
                                        closed_at: fill.venue_ts,
                                        opened_at: *oat,
                                        holding_period_bars: holding_bars,
                                    };
                                    let req = reflection::LessonCardWriteRequest {
                                        closed_trade,
                                        opening_capital: *ocap,
                                        btc_closes: btc_closes.clone(),
                                    };
                                    match rw.try_enqueue(req) {
                                        Ok(()) => debug!(mode = mode_label, "lesson card enqueued"),
                                        Err(e) => warn!(error = %e, mode = mode_label, "lesson card enqueue failed (non-fatal)"),
                                    }
                                }
                                // Reset open-side state.
                                open_tx_id = None;
                                opened_at = None;
                                open_capital = None;
                                open_strategy_id = None;
                            }
                        }
                    }

                    // Publish PnL snapshot every bar → Live equity/pnl chart.
                    //
                    // `as_of` stays wallclock `now()` — the UI equity buffer's
                    // out-of-order-delivery guard + freshness/latency rely on it
                    // being monotone (a clock never goes back). Stamping `as_of`
                    // with `bar.close_ts` (data time) to get a historical x-axis
                    // BROKE the live render and was reverted (I1, 2026-06-11).
                    //
                    // The historical data time the chart plots on its x-axis now
                    // rides a SEPARATE field, `bar_ts = bar.close_ts`
                    // (cockpit-live-equity-render-guard, approach A): the curve
                    // shows real 2023-24 dates during a fast replay while the
                    // delivery guard still uses the wallclock `as_of`. Verified
                    // render-side by `crates/ui/tests/live_equity_render.rs`.
                    let unrealized = position.base_qty * mark - cost_basis;
                    let total_equity = cash + position.base_qty * mark;
                    let snap = PnlSnapshot {
                        cash: Money::from_decimal(cash),
                        unrealized: Money::from_decimal(unrealized),
                        realized: Money::from_decimal(realized_pnl),
                        total_equity: Money::from_decimal(total_equity),
                        daily_return: Money::from_decimal(Decimal::ZERO),
                        as_of: Timestamp::now(),
                        bar_ts: Some(bar.close_ts),
                    };
                    pnl_bus.publish_pnl(snap.clone());

                    // ADR-0053 A2 — loop-direct persist, gated by Some(store).
                    // The Some/None IS the mode gate — no if mode != Research inside the loop.
                    // Research passes None → this branch never executes → byte-identical outputs.
                    // Paper passes Some(store) → fire-and-forget persist per bar (A6 — never
                    // blocks or panics the loop; write errors are logged and discarded).
                    if let Some(ref store) = equity_store {
                        let store = Arc::clone(store);
                        let row = crate::reconciler::build_snapshot_row(&snap, mode_label);
                        tokio::spawn(async move {
                            if let Err(e) = store.append_equity_snapshot(&row).await {
                                tracing::warn!(
                                    error = %e,
                                    mode = mode_label,
                                    "equity_snapshot write failed (non-fatal — loop continues)"
                                );
                            }
                        });
                    }
                }
            }
        }

        let total_equity = cash + position.base_qty * position.last_mark.get();
        info!(
            total_equity = %total_equity,
            fills = fill_count,
            mode = mode_label,
            "trading_loop stopped"
        );
    });
    abort_handle
}

/// Spawn the mode-broadcast forwarder task (T905 — live-cockpit-unified).
///
/// Subscribes to `kill_switch.subscribe()` (the internal mode channel
/// owned by `KillSwitch` itself) and forwards every received
/// [`AgentMode`] event onto the bus's `mode` channel via
/// [`EventBus::publish_mode`].  This is the *single* writer to
/// `bus.publish_mode(...)`; the kill switch never publishes to the
/// bus directly (Q6 design — keeps the kill-switch boundary clean).
///
/// The trip is sticky / idempotent (`KillSwitch::trip` short-circuits
/// the second caller via CAS), so a duplicate trip emits exactly one
/// `AgentMode::Halted` on the kill-switch channel and therefore exactly
/// one event on the bus.
///
/// Task exits cleanly on `cancel.cancelled()` or when the kill-switch
/// channel closes (`RecvError::Closed`).
pub fn spawn_mode_forwarder(
    kill_switch: Arc<KillSwitch>,
    bus: Arc<EventBus>,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
) {
    let mut mode_rx = kill_switch.subscribe();
    let cancel_m = cancel.child_token();
    set.spawn(async move {
        info!("mode_forwarder started");
        loop {
            tokio::select! {
                () = cancel_m.cancelled() => break,
                msg = mode_rx.recv() => match msg {
                    Ok(mode) => bus.publish_mode(mode),
                    Err(RecvError::Lagged(n)) => {
                        warn!(skipped = n, "mode forwarder lagged");
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        }
        info!("mode_forwarder stopped");
    });
}

/// Spawn a per-venue ingest **supervisor** task (T1408 — v1.5b
/// multi-venue / Q3 / R14).
///
/// The supervisor task is the panic-isolation boundary: it spawns the
/// actual feed-consumption work (the bar/tick taps via
/// `spawn_feed_taps`) inside an inner `tokio::task::spawn` and
/// inspects the resulting `JoinError`.  A panic in any venue's task
/// surfaces here as `JoinError::is_panic() == true` and is logged +
/// audit-journaled with `error_code = "task_panic"`; **the panic does
/// not propagate into `runtime::run` itself, so the other venues'
/// supervisors keep running** (R14.1 / R14.3).
///
/// ## Topology
///
/// ```text
/// runtime::run::set: JoinSet<()>
///    │
///    ├── supervisor_task[Venue::Binance]   ← this fn spawns one of these per venue
///    │      │
///    │      └── inner_handle = tokio::spawn(consume_streams())
///    │             │  bars_tap + ticks_tap (via spawn_feed_taps,
///    │             │  but spawned into a *local* JoinSet whose
///    │             │  drain happens before the supervisor returns)
///    │             ▼
///    │             on panic → JoinError::is_panic() → feed_reconnect
///    │             on cancel / clean exit → log + return
///    ├── supervisor_task[Venue::Coinbase]   (if enabled)
///    └── supervisor_task[Venue::Kraken]     (if enabled)
/// ```
///
/// ## Caller contract
///
/// `cancel.child_token()` is forwarded into the inner consumption
/// task; on `cancel.cancel()` from the top of `runtime::run`, the
/// inner task drains and the supervisor returns cleanly.
///
/// ## Watchdog respawn (T1409)
///
/// T1408 implements **panic isolation only**: a panicked task is
/// logged + audit-journaled but is NOT respawned by the supervisor
/// itself.  T1409 will layer the stale-data watchdog on top — it
/// detects a venue's silence (no Tick within `stale_threshold_secs`)
/// and re-runs the supervisor's spawn body.
#[allow(clippy::too_many_arguments)]
pub fn spawn_venue_supervisor(
    venue: Venue,
    feed: Arc<dyn MarketDataSource>,
    bus: Arc<EventBus>,
    ledger: Arc<audit::Ledger>,
    symbol: Symbol,
    tf: Timeframe,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
    last_tick: Option<LastTickMap>,
) {
    let cancel_sup = cancel.child_token();
    set.spawn(async move {
        info!(venue = %venue, "venue_supervisor started");

        // The inner task owns the actual bar/tick stream consumption.
        // Wrapping it in `tokio::spawn` is the panic-isolation
        // boundary: a panic inside `spawn_feed_taps` (or anywhere
        // in the bar/tick stream poll loop) surfaces as
        // `JoinError::is_panic() == true` on the join handle below;
        // it does NOT unwind the supervisor.
        let bus_inner = Arc::clone(&bus);
        let cancel_inner = cancel_sup.clone();
        let symbol_inner = symbol.clone();
        // T1409 — when a `LastTickMap` is supplied (paper mode), build
        // a tick observer that records `local_recv_ts` per venue so the
        // stale-data watchdog can detect silence.
        let tick_observer: Option<TickObserver> = last_tick.map(|map| {
            let v = venue;
            Arc::new(move |tick: &Tick| {
                if let Ok(mut guard) = map.lock() {
                    guard.insert(v, tick.local_recv_ts);
                }
            }) as TickObserver
        });
        let inner = tokio::spawn(async move {
            let mut inner_set: JoinSet<()> = JoinSet::new();
            spawn_feed_taps_with_observer(
                feed.as_ref(),
                Arc::clone(&bus_inner),
                symbol_inner,
                tf,
                &mut inner_set,
                &cancel_inner,
                tick_observer,
            )
            .await;
            // Drain the tap tasks: cancel is the natural stop signal,
            // streams ending naturally also drains.  We bound nothing
            // here (the outer `runtime::run` shutdown enforces a 2s
            // wall-clock cap on the whole JoinSet).
            while inner_set.join_next().await.is_some() {}
        });

        match inner.await {
            Ok(()) => {
                info!(venue = %venue, "venue_supervisor inner task completed cleanly");
            }
            Err(join_err) if join_err.is_panic() => {
                // R14.3 — a panic in venue X's task is logged +
                // audit-journaled with `error_code = "task_panic"`;
                // crucially, it does NOT unwind the supervisor itself
                // (we caught the JoinError) so other venues keep
                // running.  The panic message is best-effort
                // extracted via `into_panic` + downcast.
                let panic_msg = panic_message(join_err.into_panic());
                error!(
                    venue = %venue,
                    panic = %panic_msg,
                    "venue {} crashed: {} ; restarting via watchdog",
                    venue,
                    panic_msg,
                );
                // Audit-journal the failure with venue context
                // (R8 / R14.3).  Failure to write is non-fatal —
                // observability, not control flow.
                if let Err(e) =
                    audit::journal::feed_reconnect(ledger.as_ref(), "unknown", venue, None).await
                {
                    warn!(
                        venue = %venue,
                        error = %e,
                        "feed_reconnect audit write failed (non-fatal)",
                    );
                }
                // T1408 stops here (panic isolated).  T1409 will add
                // the respawn loop on top of the watchdog.
            }
            Err(join_err) if join_err.is_cancelled() => {
                debug!(venue = %venue, "venue_supervisor inner task cancelled");
            }
            Err(join_err) => {
                warn!(
                    venue = %venue,
                    error = %join_err,
                    "venue_supervisor inner task ended with non-panic JoinError",
                );
            }
        }

        info!(venue = %venue, "venue_supervisor stopped");
    });
}

/// Best-effort extraction of a panic payload's message.
///
/// `JoinError::into_panic()` returns a `Box<dyn Any + Send>` whose
/// concrete type depends on what was panicked with.  The two common
/// cases (`String` and `&'static str`) cover almost every
/// `panic!("…")` invocation; anything else falls back to the literal
/// string `"non-string panic payload"`.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        "non-string panic payload".into()
    }
}

/// Spawn the v1.5b stale-data watchdog (T1409 — Q7 / R14.4).
///
/// The watchdog is the **single producer** of `MarketHealth` events on
/// `EventBus::market_health`.  It scans the shared per-venue last-tick
/// map every `interval` and, for each venue in `venues`:
///
/// - When the venue is **not yet seen** (no entry in the map) it is in
///   the "Unseen" state — no event is emitted until the first tick
///   arrives.  The first tick transitions Unseen → Fresh and publishes
///   `MarketHealth::Fresh { venue, last_tick_ts }`.
/// - When `now - last_tick > threshold_secs` and the previous state was
///   `Fresh`, transitions Fresh → Stale and publishes
///   `MarketHealth::Stale { venue, last_tick_ts, threshold_secs }`.
/// - When a tick arrives after a `Stale` window (i.e. a stored
///   `last_tick` updates and is now within threshold while the watchdog
///   recorded `Stale`), transitions Stale → Fresh and publishes
///   `MarketHealth::Recovered { venue, recovered_ts, gap_secs }`.
///
/// The clock is injected via [`NowFn`] so tests can drive the wall-clock
/// deterministically (`tokio::time::pause` only controls tokio sleeps,
/// not `OffsetDateTime::now_utc`).  Live mode wires `Timestamp::now`.
///
/// Determinism: the watchdog is live-only (paper-mode), never reachable
/// from `crates/backtest/` replay — see Q7 in the feature brief.  The
/// state machine itself is deterministic given a fixed input sequence.
#[allow(clippy::too_many_arguments)]
pub fn spawn_market_health_watchdog(
    bus: Arc<EventBus>,
    last_tick: LastTickMap,
    venues: Vec<Venue>,
    threshold_secs: u32,
    now_fn: NowFn,
    interval: Duration,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
) {
    let cancel_w = cancel.child_token();
    set.spawn(async move {
        info!(
            venue_count = venues.len(),
            threshold_secs, "market_health_watchdog started"
        );
        // Per-venue tracked state.  None = Unseen (no tick observed
        // yet); Some(MarketHealthState::Fresh) / Some(Stale) once seen.
        let mut state: HashMap<Venue, MarketHealthState> = HashMap::new();
        let mut tick = tokio::time::interval(interval);
        // Skip the immediate first tick so the watchdog's first scan
        // happens after `interval`, matching wall-clock cadence.
        tick.tick().await;
        loop {
            tokio::select! {
                () = cancel_w.cancelled() => break,
                _ = tick.tick() => {
                    let now = now_fn();
                    // Snapshot the last-tick map under the lock; do not
                    // hold the mutex across `await` (we don't await
                    // anything inside this scan, but the snapshot keeps
                    // the lock window O(venue_count)).
                    let snapshot: HashMap<Venue, Timestamp> = match last_tick.lock() {
                        Ok(g) => g.clone(),
                        Err(p) => {
                            // Another task panicked while holding the
                            // map; recover the inner value so we keep
                            // running (best-effort observability).
                            warn!("last_tick map poisoned — recovering inner state");
                            p.into_inner().clone()
                        }
                    };

                    // Iterate venues in deterministic Ord order so the
                    // emitted event sequence is reproducible across
                    // runs (matches Venue::Ord in core::venue).
                    let mut sorted = venues.clone();
                    sorted.sort();
                    for venue in sorted {
                        match snapshot.get(&venue) {
                            None => {
                                // Unseen: no event until the first tick
                                // (state remains absent from `state`).
                            }
                            Some(&last_tick_ts) => {
                                let age = saturating_secs_between(last_tick_ts, now);
                                let prev = state.get(&venue).copied();
                                let is_stale = age > i64::from(threshold_secs);
                                match (prev, is_stale) {
                                    (None, false) => {
                                        // First-ever observation: Unseen → Fresh.
                                        bus.publish_market_health(MarketHealth::Fresh {
                                            venue,
                                            last_tick_ts,
                                        });
                                        state.insert(venue, MarketHealthState::Fresh);
                                    }
                                    (None, true) => {
                                        // First observation but already
                                        // older than threshold (e.g. an
                                        // ancient tick replayed).  Emit
                                        // Stale directly so subscribers
                                        // see the worst-case state.
                                        bus.publish_market_health(MarketHealth::Stale {
                                            venue,
                                            last_tick_ts,
                                            threshold_secs,
                                        });
                                        state.insert(venue, MarketHealthState::Stale);
                                    }
                                    (Some(MarketHealthState::Fresh), true) => {
                                        // Fresh → Stale transition.
                                        bus.publish_market_health(MarketHealth::Stale {
                                            venue,
                                            last_tick_ts,
                                            threshold_secs,
                                        });
                                        state.insert(venue, MarketHealthState::Stale);
                                    }
                                    (Some(MarketHealthState::Stale), false) => {
                                        // Stale → Fresh (Recovered): a
                                        // newer tick has arrived inside
                                        // the threshold window.
                                        let gap_secs = u32::try_from(age.max(0)).unwrap_or(u32::MAX);
                                        bus.publish_market_health(MarketHealth::Recovered {
                                            venue,
                                            recovered_ts: last_tick_ts,
                                            gap_secs,
                                        });
                                        state.insert(venue, MarketHealthState::Fresh);
                                    }
                                    (Some(MarketHealthState::Fresh), false)
                                    | (Some(MarketHealthState::Stale), true) => {
                                        // Steady state — no event.
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        info!("market_health_watchdog stopped");
    });
}

// ── T1707 — RiskTelemetry publisher (Phase 3 Q3) ───────────────────────────────
//
// Sibling of `spawn_market_health_watchdog` — single-producer, periodic
// publisher of `RiskTelemetry` snapshots on `EventBus::risk_telemetry`.
// The cockpit's `Subscription::batch` recipe in `crates/ui/src/live.rs`
// maps incoming events to `Message::RiskStateRefreshed(RiskState)`.
//
// v1.5b plumbing-only state — the snapshot carries deterministic
// placeholder numbers (zero exposures, zero loss, large heartbeat
// timeout) until the actual risk-engine wiring lands. The bus
// channel is the load-bearing surface tested by the Phase 3 cockpit
// integration test.

/// Snapshot provider for the risk-telemetry publisher. Returns the
/// current `RiskTelemetry` view; live mode wires the actual risk-engine
/// state, fixtures wire a deterministic stub.
pub type RiskSnapshotFn = Arc<dyn Fn() -> RiskTelemetry + Send + Sync>;

/// Phase 3 T1707 — placeholder risk-telemetry snapshot for the live
/// runtime's 1 Hz publisher. v1.5b plumbing-only state — the
/// snapshot returns deterministic zero exposures + a wide heartbeat
/// timeout so the cockpit's Risk / Limits screen renders all-green
/// bands. The actual risk-engine wiring (per-symbol exposures,
/// daily-loss accumulator, kill-threshold proximity) lands as a
/// follow-up.
fn default_risk_telemetry_stub() -> RiskTelemetry {
    RiskTelemetry {
        per_symbol_exposure: HashMap::new(),
        per_symbol_caps: HashMap::new(),
        daily_loss_used_pct: rust_decimal::Decimal::ZERO,
        daily_loss_cap_pct: rust_decimal::Decimal::from(100),
        heartbeat_age_ms: 0,
        heartbeat_timeout_ms: 30_000,
    }
}

/// Spawn the risk-telemetry publisher (Phase 3 T1707 / Q3).
///
/// Ticks at `interval` (1 Hz in production) and publishes the latest
/// `RiskTelemetry` snapshot via `bus.publish_risk_telemetry(...)`.
/// `cancel` stops the loop on shutdown.
/// R7 retention horizon — rows older than this are purged. Matches the
/// `purge_old_equity_snapshots` production default and the 30-day
/// ledger-snapshot horizon (ADR-0052 A3).
const EQUITY_PURGE_HORIZON_DAYS: u32 = 30;

/// Purge cadence — nightly. The first tick fires immediately at boot so a
/// long-downtime restart catches up without waiting a day.
const EQUITY_PURGE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Spawn the nightly equity-snapshot retention purge (R7 — closes the
/// ADR-0052 D5 scheduling deferral, operator-ratified 2026-06-12).
///
/// Spawned ONLY where equity rows are minted (paper/live mode with
/// `equity_store = Some`); research mode persists nothing (A2) so it gets
/// no purge task. Fire-and-forget per tick: an `Err` is logged and the
/// task keeps its cadence — a failed purge never crashes or blocks
/// anything (the same A6 tolerance as the writer).
///
/// Unlike the telemetry publishers, the FIRST tick is NOT skipped:
/// `tokio::time::interval`'s immediate first fire is the boot catch-up
/// (a store that grew past the horizon during downtime is trimmed right
/// away, then daily).
pub fn spawn_equity_purge_task(
    store: Arc<dyn audit::LiveEquityStore>,
    horizon_days: u32,
    interval: Duration,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
) {
    let cancel_w = cancel.child_token();
    set.spawn(async move {
        info!(
            horizon_days,
            interval_secs = interval.as_secs(),
            "equity_purge_task started"
        );
        let mut tick = tokio::time::interval(interval);
        loop {
            tokio::select! {
                () = cancel_w.cancelled() => break,
                _ = tick.tick() => {
                    match store.purge_older_than(horizon_days).await {
                        Ok(0) => debug!("equity purge: nothing past the horizon"),
                        Ok(deleted) => info!(deleted, horizon_days, "equity purge: rows removed"),
                        // Fire-and-forget (A6): log + keep the cadence.
                        Err(e) => warn!(error = %e, "equity purge failed; will retry next tick"),
                    }
                }
            }
        }
        info!("equity_purge_task stopped");
    });
}

pub fn spawn_risk_telemetry_publisher(
    bus: Arc<EventBus>,
    snapshot_fn: RiskSnapshotFn,
    interval: Duration,
    set: &mut JoinSet<()>,
    cancel: &CancellationToken,
) {
    let cancel_w = cancel.child_token();
    set.spawn(async move {
        info!(
            interval_ms = interval.as_millis() as u64,
            "risk_telemetry_publisher started"
        );
        let mut tick = tokio::time::interval(interval);
        // Skip the immediate first tick so the cadence matches wall-clock.
        tick.tick().await;
        loop {
            tokio::select! {
                () = cancel_w.cancelled() => break,
                _ = tick.tick() => {
                    let snapshot = snapshot_fn();
                    bus.publish_risk_telemetry(snapshot);
                }
            }
        }
        info!("risk_telemetry_publisher stopped");
    });
}

/// Tracked per-venue health state inside the watchdog (private — the
/// public surface is the `MarketHealth` enum on the bus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarketHealthState {
    Fresh,
    Stale,
}

/// Whole-second age between `earlier` and `later`, saturating at
/// `i64::MAX`.  Negative when `earlier > later` (clock-skew defensive).
fn saturating_secs_between(earlier: Timestamp, later: Timestamp) -> i64 {
    let diff_ns = later.inner().unix_timestamp_nanos() - earlier.inner().unix_timestamp_nanos();
    i64::try_from(diff_ns / 1_000_000_000_i128).unwrap_or(i64::MAX)
}

/// Close the uptime interval — call exactly once after [`run`]
/// returns (T806 R7.1).
///
/// Failures are warn-logged, never returned: a missed close row is an
/// observability issue, not a control-flow failure.  Both shutdown
/// paths (Ctrl-C in the headless bin, window-close in the unified
/// `cockpit_live` bin) call this helper so the close-write site has
/// exactly one home.
pub async fn shutdown_writer(ledger: Arc<audit::Ledger>, boot_id: &str) {
    if let Err(e) = audit::journal::close_uptime_interval(&ledger, boot_id, None).await {
        warn!(error = %e, "close_uptime_interval failed (non-fatal)");
    } else {
        info!(boot_id = %boot_id, "agent uptime interval closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BusConfig;
    use crate::kill_switch::{HaltReason, MockIncidentSpawner};
    use rust_decimal_macros::dec;
    use trading_core::{Bar, Price, Quantity, Side, Tick, Timestamp, Venue};

    fn ts(offset_secs: i64) -> Timestamp {
        Timestamp::new(
            time::OffsetDateTime::from_unix_timestamp(1_700_000_000 + offset_secs)
                .expect("valid ts"),
        )
    }

    fn make_bar(close: rust_decimal::Decimal, t: i64) -> Bar {
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open_ts: ts(t),
            close_ts: ts(t + 60),
            open: Price::new(close).expect("open"),
            high: Price::new(close).expect("high"),
            low: Price::new(close).expect("low"),
            close: Price::new(close).expect("close"),
            volume: Quantity::new(dec!(1)).expect("volume"),
            trade_count: 1,
            local_recv_ts: ts(t + 60),
            venue: Venue::Binance,
        }
    }

    fn make_tick(price: rust_decimal::Decimal, t: i64) -> Tick {
        Tick {
            symbol: Symbol::new("BTCUSDT"),
            venue_ts: ts(t),
            local_recv_ts: ts(t),
            price: Price::new(price).expect("price"),
            qty: Quantity::new(dec!(1)).expect("qty"),
            side: Side::Buy,
            trade_id: u64::try_from(t).unwrap_or(0),
            venue: Venue::Binance,
        }
    }

    /// Smoke test for [`run`]: build a minimal RunHandles, await
    /// `runtime::run`, send `cancel.cancel()` after a short delay,
    /// assert clean Ok return inside 2 s.  Mitigates risk #5.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn t902_runtime_run_returns_clean_on_cancel() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("test_ledger.db");
        let halt_file = temp.path().join(".halt");

        // Build a research-mode config pointing at the tempdir.
        let mut cfg = Config::default();
        cfg.audit.ledger_db_path = db_path.to_string_lossy().into_owned();
        cfg.kill_switch.halt_file = halt_file.to_string_lossy().into_owned();
        cfg.data.historical.parquet_root = temp.path().to_string_lossy().into_owned();

        let ledger = Arc::new(
            audit::Ledger::open(&cfg.audit.ledger_db_path)
                .await
                .expect("open ledger"),
        );
        audit::bootstrap::chart_of_accounts(&ledger)
            .await
            .expect("chart");

        let spawner: Arc<dyn crate::IncidentSpawner> = Arc::new(MockIncidentSpawner::new());
        let kill_switch = Arc::new(KillSwitch::with_audit(
            &cfg.kill_switch.halt_file,
            32,
            Arc::clone(&ledger),
            spawner,
        ));

        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let registry = Arc::new(strategy::StrategyRegistry::new());

        let boot_id = uuid::Uuid::new_v4().to_string();
        audit::journal::open_uptime_interval(&ledger, &boot_id, None)
            .await
            .expect("open_uptime");

        let handles = RunHandles {
            config: Arc::new(cfg),
            ledger: Arc::clone(&ledger),
            bus: Arc::clone(&bus),
            kill_switch: Arc::clone(&kill_switch),
            registry,
            boot_id: boot_id.clone(),
            equity_store: None,         // tests use no equity store
            reflection_writer: None,    // tests do not exercise lesson-card wiring
            forward_rx: None,           // tests: no forward-command channel (byte-identical path)
            plan_tx: None,              // tests: no plan channel (F6 gate; ADR-0062 § D6)
            narration_request_rx: None, // tests: no narration channel (F9 byte-identical gate)
            narration_outcome_tx: None, // tests: no narration outcome channel
        };

        let cancel = CancellationToken::new();
        let cancel_for_run = cancel.clone();
        let run_handle = tokio::spawn(async move { run(handles, cancel_for_run).await });

        // Let the runtime warm up.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        // Trigger graceful shutdown.
        cancel.cancel();

        // Bound the wait so a regression that hangs the runtime is a
        // hard test failure rather than a hung test process.
        let res = tokio::time::timeout(std::time::Duration::from_secs(2), run_handle)
            .await
            .expect("runtime did not return inside 2 s");
        res.expect("join error").expect("runtime returned Err");

        // T806 invariant: caller closes the interval after run returns.
        shutdown_writer(Arc::clone(&ledger), &boot_id).await;

        // Sanity-check that a kill-switch trip from outside `run`
        // also behaves correctly even after shutdown (the trip is
        // sticky / idempotent — it must not panic post-shutdown).
        kill_switch.trip(HaltReason::Test);
        assert!(kill_switch.is_tripped());
    }

    /// T903b — bar/tick taps re-publish each item from the feed onto
    /// the bus.  Drives [`spawn_feed_taps`] against a `FakeFeed`
    /// emitting 5 bars + 20 ticks; subscribers on `bus.bars()` and
    /// `bus.ticks()` must observe all 5 bars and all 20 ticks within
    /// 2 s.  Also asserts that the spawned tasks exit cleanly on
    /// `cancel.cancel()`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn t903b_taps_publish_bars_and_ticks() {
        let bars: Vec<Bar> = (0..5)
            .map(|i| {
                make_bar(
                    dec!(50_000) + dec!(10) * rust_decimal::Decimal::from(i),
                    i * 60,
                )
            })
            .collect();
        let ticks: Vec<Tick> = (0..20)
            .map(|i| make_tick(dec!(50_000) + rust_decimal::Decimal::from(i), i))
            .collect();
        let feed = data::FakeFeed::new(bars.clone(), ticks.clone());

        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        // Subscribe BEFORE spawning the tap tasks so we don't drop
        // events.  broadcast::Receiver buffers up to channel capacity.
        let mut bars_rx = bus.bars();
        let mut ticks_rx = bus.ticks();

        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();
        spawn_feed_taps(
            &feed,
            Arc::clone(&bus),
            Symbol::new("BTCUSDT"),
            Timeframe::OneMinute,
            &mut set,
            &cancel,
        )
        .await;

        // Collect events with a 2 s overall budget.
        let collect = async {
            let mut got_bars: Vec<Bar> = Vec::new();
            let mut got_ticks: Vec<Tick> = Vec::new();
            while got_bars.len() < bars.len() || got_ticks.len() < ticks.len() {
                tokio::select! {
                    b = bars_rx.recv() => {
                        if let Ok(bar) = b { got_bars.push(bar); }
                    }
                    t = ticks_rx.recv() => {
                        if let Ok(tick) = t { got_ticks.push(tick); }
                    }
                }
            }
            (got_bars, got_ticks)
        };
        let (got_bars, got_ticks) =
            tokio::time::timeout(std::time::Duration::from_secs(2), collect)
                .await
                .expect("taps did not deliver all events inside 2 s");

        assert_eq!(got_bars.len(), bars.len(), "bar count mismatch");
        assert_eq!(got_ticks.len(), ticks.len(), "tick count mismatch");

        // Cancel and drain — must complete without leaking tasks.
        cancel.cancel();
        let drain = async { while set.join_next().await.is_some() {} };
        tokio::time::timeout(std::time::Duration::from_secs(2), drain)
            .await
            .expect("tap tasks did not drain inside 2 s");
    }

    /// T905 — mode-broadcast forwarder: a `KillSwitch::trip` event
    /// surfaces on the bus's `mode` channel exactly once (sticky).
    /// Confirms the forwarder bridges `kill_switch.subscribe()` →
    /// `bus.publish_mode(...)` and respects shutdown.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn t905_kill_switch_trip_emits_to_bus_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let halt_file = temp.path().join(".halt");

        // KillSwitch v0 (no audit) — sufficient for the forwarder
        // wiring test; T809's dual-write is exercised by
        // `kill_switch_trip_writes_both`.
        let kill_switch = Arc::new(KillSwitch::new(&halt_file, 32));
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut mode_rx = bus.mode();

        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();
        spawn_mode_forwarder(
            Arc::clone(&kill_switch),
            Arc::clone(&bus),
            &mut set,
            &cancel,
        );

        // Give the forwarder a tick to subscribe.
        tokio::task::yield_now().await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        kill_switch.trip(HaltReason::Test);

        let mode = tokio::time::timeout(std::time::Duration::from_millis(500), mode_rx.recv())
            .await
            .expect("forwarder did not deliver mode event inside 500 ms")
            .expect("mode channel closed unexpectedly");
        assert!(matches!(mode, AgentMode::Halted { .. }), "expected Halted");

        // Sticky-trip: a second trip is a no-op on the kill switch
        // and must NOT produce a second event on the bus.
        kill_switch.trip(HaltReason::ManualOperator);
        let second =
            tokio::time::timeout(std::time::Duration::from_millis(200), mode_rx.recv()).await;
        assert!(
            second.is_err(),
            "sticky-trip violated: second mode event arrived = {:?}",
            second
        );

        // Clean shutdown.
        cancel.cancel();
        let drain = async { while set.join_next().await.is_some() {} };
        tokio::time::timeout(std::time::Duration::from_secs(2), drain)
            .await
            .expect("forwarder did not drain inside 2 s");
    }

    /// T903a-glue — the [`paper_engine_publisher`] helper constructs a
    /// `PaperEnginePublisher` whose backing `FillPublisher` is the
    /// agent's [`EventBus`]; calling `on_fill` on the engine therefore
    /// fans out to `bus.fills()` + `bus.positions()` subscribers.
    /// This is the agent-side closure of the bus-wiring loop Dev A
    /// landed in `crates/exec/`.
    #[tokio::test(flavor = "current_thread")]
    async fn t903a_glue_paper_engine_publisher_routes_to_bus() {
        use exec::PaperEnginePublisher;
        use rust_decimal_macros::dec;
        use trading_core::{
            FeeTier, Fill, FillId, Liquidity, Money, OrderId, Position, Quantity, Side,
        };

        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut fills_rx = bus.fills();
        let mut pos_rx = bus.positions();

        let engine: PaperEnginePublisher = paper_engine_publisher(Arc::clone(&bus));

        let fill = Fill {
            id: FillId::new(),
            order_id: OrderId::new(),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            qty: Quantity::new(dec!(0.1)).expect("qty"),
            price: Price::new(dec!(40_000)).expect("price"),
            fee: Money::from_decimal(dec!(1.6)),
            fee_tier: FeeTier::Taker,
            venue_ts: ts(0),
            local_ts: ts(0),
            liquidity: Liquidity::Taker,
            transaction_id: None,
        };
        let pos = Position::empty(Symbol::new("BTCUSDT"));

        engine.on_fill(&fill, &pos);

        let got_fill = tokio::time::timeout(std::time::Duration::from_millis(500), fills_rx.recv())
            .await
            .expect("fill recv timed out")
            .expect("fill channel closed");
        assert_eq!(got_fill.id, fill.id);

        let got_pos = tokio::time::timeout(std::time::Duration::from_millis(500), pos_rx.recv())
            .await
            .expect("pos recv timed out")
            .expect("pos channel closed");
        assert_eq!(got_pos.symbol, pos.symbol);
    }

    // ── T1408 — per-venue ingest topology + panic isolation ───────────────────

    /// T1408 / R10.2 — backwards compatibility.  The default `Config`
    /// has Coinbase + Kraken disabled; a `runtime::run` build against
    /// the default config therefore enables Binance only.  This test
    /// asserts the **config flags** that drive the per-venue spawn loop
    /// in [`run`] line up with the v1.5a single-venue behaviour.  An
    /// integration test that spins up `run` would also cover this; the
    /// flag-level test is the cheap unit gate that fires on every
    /// `cargo test -p agent` run.
    #[test]
    fn t1408_default_config_spawns_only_binance() {
        let cfg = Config::default();
        assert!(
            !cfg.data.sources.coinbase.enabled,
            "default Coinbase must be disabled (R10.2 backwards compat)"
        );
        assert!(
            !cfg.data.sources.kraken.enabled,
            "default Kraken must be disabled (R10.2 backwards compat)"
        );

        // Mirror the per-venue enabled-set construction in `run`'s
        // `Mode::Paper` arm.  Binance is unconditionally enabled; the
        // other two follow the config flags.  The resulting list MUST
        // contain exactly Venue::Binance for the default config.
        let mut enabled: Vec<Venue> = vec![Venue::Binance];
        if cfg.data.sources.coinbase.enabled {
            enabled.push(Venue::Coinbase);
        }
        if cfg.data.sources.kraken.enabled {
            enabled.push(Venue::Kraken);
        }
        enabled.sort();

        assert_eq!(
            enabled,
            vec![Venue::Binance],
            "default config must enable Binance only (1.5a parity)"
        );
    }

    /// T1408 — three-venue config produces three sorted supervisor
    /// tasks.  We don't stand up the real WS feeds (those would touch
    /// the network); we instead drive [`spawn_venue_supervisor`] with
    /// `FakeFeed` instances and assert (a) three tasks land in the
    /// JoinSet, (b) `cancel.cancel()` drains them all cleanly, and
    /// (c) the spawn order is deterministic by `Venue`'s `Ord`
    /// (Binance < Coinbase < Kraken — R7.4 / determinism).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn t1408_three_venue_config_spawns_all_three() {
        // In-memory ledger (every supervisor is given a ledger arc;
        // we don't expect any feed_reconnect writes on the happy path).
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("test_ledger.db");
        let ledger = Arc::new(
            audit::Ledger::open(db_path.to_str().expect("path str"))
                .await
                .expect("open ledger"),
        );
        audit::bootstrap::chart_of_accounts(&ledger)
            .await
            .expect("chart");

        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();

        // Three venues, each with a FakeFeed that emits one bar +
        // one tick and then ends naturally (stream completes).
        let venues = [Venue::Binance, Venue::Coinbase, Venue::Kraken];
        for v in venues {
            let bar = make_bar(rust_decimal_macros::dec!(50_000), 0);
            let tick = make_tick(rust_decimal_macros::dec!(50_000), 0);
            let feed: Arc<dyn MarketDataSource> =
                Arc::new(data::FakeFeed::new(vec![bar], vec![tick]));
            spawn_venue_supervisor(
                v,
                feed,
                Arc::clone(&bus),
                Arc::clone(&ledger),
                Symbol::new("BTCUSDT"),
                Timeframe::OneMinute,
                &mut set,
                &cancel,
                None,
            );
        }

        // Three supervisor tasks must be queued.  `JoinSet::len`
        // reports tasks that have not yet been polled to completion.
        assert_eq!(
            set.len(),
            3,
            "expected one supervisor per enabled venue (3)"
        );

        // Cancel and drain — every supervisor must exit cleanly.
        cancel.cancel();
        let drain = async { while set.join_next().await.is_some() {} };
        tokio::time::timeout(std::time::Duration::from_secs(2), drain)
            .await
            .expect("supervisors did not drain inside 2 s");
    }

    /// T1408 / R14.1 + R14.3 — a venue-task panic must NOT propagate
    /// into `runtime::run`.  We construct a `PanickingFeed` whose
    /// `subscribe_bars` panics on the first poll; spawn the
    /// supervisor; await the JoinSet drain.  The test passes iff the
    /// JoinSet drain completes (no panic surfaces out of
    /// `spawn_venue_supervisor`).  If panic isolation regresses,
    /// `JoinSet::join_next` would surface a `JoinError::is_panic()`
    /// from the supervisor task and the assertion below would fail.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn t1408_venue_panic_isolated_does_not_kill_runtime() {
        use async_trait::async_trait;
        use futures::stream::BoxStream;
        use trading_core::FeedError;

        struct PanickingFeed;

        #[async_trait]
        impl MarketDataSource for PanickingFeed {
            async fn exchange_info(
                &self,
                _symbol: Symbol,
            ) -> Result<data::source::SymbolInfo, FeedError> {
                // Not exercised by the supervisor path; surface an
                // error to keep the trait satisfied.
                Err(FeedError::Parse("PanickingFeed::exchange_info".into()))
            }

            async fn subscribe_bars(
                &self,
                _symbol: Symbol,
                _tf: Timeframe,
            ) -> Result<BoxStream<'static, Result<trading_core::Bar, FeedError>>, FeedError>
            {
                // Synthetic crash inside the venue's stream subscribe
                // path — exactly the kind of bug Q3 / R14.3 calls out
                // (a parser bug poisoning the venue's stream).
                panic!("synthetic venue parser crash");
            }

            async fn subscribe_trades(
                &self,
                _symbol: Symbol,
            ) -> Result<BoxStream<'static, Result<trading_core::Tick, FeedError>>, FeedError>
            {
                panic!("synthetic venue parser crash (trades)");
            }
        }

        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("test_ledger.db");
        let ledger = Arc::new(
            audit::Ledger::open(db_path.to_str().expect("path str"))
                .await
                .expect("open ledger"),
        );
        audit::bootstrap::chart_of_accounts(&ledger)
            .await
            .expect("chart");

        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();

        // The Coinbase supervisor's inner task will panic.  The
        // supervisor MUST catch it via `JoinError::is_panic()` and
        // return cleanly so the surrounding JoinSet drains.
        spawn_venue_supervisor(
            Venue::Coinbase,
            Arc::new(PanickingFeed),
            Arc::clone(&bus),
            Arc::clone(&ledger),
            Symbol::new("BTCUSDT"),
            Timeframe::OneMinute,
            &mut set,
            &cancel,
            None,
        );

        // A second, healthy supervisor confirms cross-venue isolation:
        // the Coinbase panic must NOT poison the Binance supervisor.
        let bar = make_bar(rust_decimal_macros::dec!(50_000), 0);
        let tick = make_tick(rust_decimal_macros::dec!(50_000), 0);
        let healthy: Arc<dyn MarketDataSource> =
            Arc::new(data::FakeFeed::new(vec![bar], vec![tick]));
        spawn_venue_supervisor(
            Venue::Binance,
            healthy,
            Arc::clone(&bus),
            Arc::clone(&ledger),
            Symbol::new("BTCUSDT"),
            Timeframe::OneMinute,
            &mut set,
            &cancel,
            None,
        );

        // Drive the supervisors: drain join_next.  Critically, NO
        // join_next call should ever return `Err(JoinError)` — the
        // supervisor catches panics internally and returns Ok.  We
        // give the supervisor up to 2 s to detect + log + return; the
        // FakeFeed-backed Binance supervisor exits naturally as soon
        // as cancel fires.
        cancel.cancel();
        let collected = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let mut joins = Vec::new();
            while let Some(res) = set.join_next().await {
                joins.push(res);
            }
            joins
        })
        .await
        .expect("supervisors did not drain inside 2 s");

        assert_eq!(
            collected.len(),
            2,
            "expected both supervisors to drain (panic + healthy)"
        );
        for res in collected {
            // Panic isolation invariant: the supervisor task itself
            // never panics, even when its inner task does.
            assert!(
                res.is_ok(),
                "supervisor task surfaced a JoinError — panic isolation regressed: {res:?}"
            );
        }
    }

    // ── T1409 — MarketHealth bus channel + stale-data watchdog ────────────────

    /// Helper: a fake injected wall-clock the watchdog reads via [`NowFn`].
    /// Tests advance this independently of `tokio::time::advance`, since
    /// `OffsetDateTime::now_utc()` is NOT controllable through tokio
    /// pausing — only the watchdog's `interval` cadence is.  Pairing the
    /// two gives full deterministic control over (a) when the watchdog
    /// scans (`tokio::time::advance`) and (b) what `now()` returns at that
    /// scan (`fake_clock.set(...)`).
    #[derive(Clone)]
    struct FakeClock(Arc<Mutex<Timestamp>>);

    impl FakeClock {
        fn new(t: Timestamp) -> Self {
            Self(Arc::new(Mutex::new(t)))
        }
        fn set(&self, t: Timestamp) {
            // Best-effort: a poisoned mutex is recovered (test-only path).
            let mut guard = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *guard = t;
        }
        fn into_now_fn(self) -> NowFn {
            Arc::new(move || {
                self.0
                    .lock()
                    .map(|g| *g)
                    .unwrap_or_else(|p| *p.into_inner())
            })
        }
    }

    /// T1409 V1 — first observation of a venue's tick lands on the bus
    /// as `MarketHealth::Fresh { venue, last_tick_ts }`.  Drives the
    /// watchdog directly (no per-venue feed needed) so the test is
    /// strictly state-machine focused.
    #[tokio::test(start_paused = true, flavor = "current_thread")]
    async fn t1409_v1_health_publishes_fresh_on_first_tick() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut health_rx = bus.market_health();

        let last_tick: LastTickMap = Arc::new(Mutex::new(HashMap::new()));
        let t0 = ts(0);
        let clock = FakeClock::new(t0);

        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();
        spawn_market_health_watchdog(
            Arc::clone(&bus),
            Arc::clone(&last_tick),
            vec![Venue::Binance],
            30,
            clock.clone().into_now_fn(),
            std::time::Duration::from_secs(1),
            &mut set,
            &cancel,
        );

        // Inject the venue's first tick into the last-tick map.  This
        // simulates the per-venue ticks_tap observer (the production
        // wiring in `spawn_venue_supervisor`).
        last_tick.lock().expect("lock").insert(Venue::Binance, t0);

        // Advance both clocks by 1s — the watchdog interval fires once.
        clock.set(ts(1));
        tokio::time::advance(std::time::Duration::from_secs(1)).await;

        // Recv the Fresh event.  Use a generous tokio-time timeout
        // (start_paused = true makes wall-clock irrelevant).
        let evt = tokio::time::timeout(std::time::Duration::from_secs(5), health_rx.recv())
            .await
            .expect("watchdog did not publish Fresh inside the budget")
            .expect("channel closed unexpectedly");
        match evt {
            MarketHealth::Fresh {
                venue,
                last_tick_ts,
            } => {
                assert_eq!(venue, Venue::Binance);
                assert_eq!(last_tick_ts, t0);
            }
            other => panic!("expected Fresh, got {other:?}"),
        }

        cancel.cancel();
        let drain = async { while set.join_next().await.is_some() {} };
        tokio::time::timeout(std::time::Duration::from_secs(5), drain)
            .await
            .expect("watchdog did not drain inside budget");
    }

    /// T1409 V2 — after the fixture clock advances 30s past the last
    /// recorded tick, the watchdog publishes `MarketHealth::Stale`.
    #[tokio::test(start_paused = true, flavor = "current_thread")]
    async fn t1409_v2_publishes_stale_after_30s_silence() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut health_rx = bus.market_health();

        let last_tick: LastTickMap = Arc::new(Mutex::new(HashMap::new()));
        let t0 = ts(0);
        let clock = FakeClock::new(t0);

        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();
        spawn_market_health_watchdog(
            Arc::clone(&bus),
            Arc::clone(&last_tick),
            vec![Venue::Coinbase],
            30,
            clock.clone().into_now_fn(),
            std::time::Duration::from_secs(1),
            &mut set,
            &cancel,
        );

        // Inject a tick at t=0 — drives the Unseen → Fresh transition.
        last_tick.lock().expect("lock").insert(Venue::Coinbase, t0);

        // Advance 1s → first scan publishes Fresh.
        clock.set(ts(1));
        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        let first = tokio::time::timeout(std::time::Duration::from_secs(5), health_rx.recv())
            .await
            .expect("Fresh event missing")
            .expect("channel closed");
        assert!(
            matches!(first, MarketHealth::Fresh { .. }),
            "expected Fresh, got {first:?}"
        );

        // Advance another 31s with no new tick — watchdog scans on every
        // interval, but we jump in one large advance to keep the test
        // deterministic.  After the jump, the "now" clock reads t=32 and
        // last_tick is still t=0, so age = 32s > threshold = 30s.
        clock.set(ts(32));
        tokio::time::advance(std::time::Duration::from_secs(31)).await;

        // The next emitted event MUST be `Stale` for Coinbase at t=0.
        // Drain until we see a non-Fresh event (the watchdog fires
        // multiple intervals in the advance window but only the first
        // Stale-transition publishes; subsequent scans see no state
        // change).
        let stale = loop {
            let evt = tokio::time::timeout(std::time::Duration::from_secs(5), health_rx.recv())
                .await
                .expect("Stale event did not arrive")
                .expect("channel closed");
            if !matches!(evt, MarketHealth::Fresh { .. }) {
                break evt;
            }
        };
        match stale {
            MarketHealth::Stale {
                venue,
                last_tick_ts,
                threshold_secs,
            } => {
                assert_eq!(venue, Venue::Coinbase);
                assert_eq!(last_tick_ts, t0);
                assert_eq!(threshold_secs, 30);
            }
            other => panic!("expected Stale, got {other:?}"),
        }

        cancel.cancel();
        let drain = async { while set.join_next().await.is_some() {} };
        tokio::time::timeout(std::time::Duration::from_secs(5), drain)
            .await
            .expect("watchdog did not drain inside budget");
    }

    /// T1409 V3 — after the venue is `Stale`, the next fresh tick (and
    /// the next watchdog scan) publishes `MarketHealth::Recovered`.
    #[tokio::test(start_paused = true, flavor = "current_thread")]
    async fn t1409_v3_publishes_recovered_on_first_tick_after_stale() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut health_rx = bus.market_health();

        let last_tick: LastTickMap = Arc::new(Mutex::new(HashMap::new()));
        let t0 = ts(0);
        let clock = FakeClock::new(t0);

        let mut set: JoinSet<()> = JoinSet::new();
        let cancel = CancellationToken::new();
        spawn_market_health_watchdog(
            Arc::clone(&bus),
            Arc::clone(&last_tick),
            vec![Venue::Kraken],
            30,
            clock.clone().into_now_fn(),
            std::time::Duration::from_secs(1),
            &mut set,
            &cancel,
        );

        // Step 1: inject a tick at t=0 → Fresh.
        last_tick.lock().expect("lock").insert(Venue::Kraken, t0);
        clock.set(ts(1));
        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        let first = tokio::time::timeout(std::time::Duration::from_secs(5), health_rx.recv())
            .await
            .expect("Fresh event missing")
            .expect("channel closed");
        assert!(
            matches!(first, MarketHealth::Fresh { .. }),
            "expected Fresh, got {first:?}"
        );

        // Step 2: advance 31s past last tick → Stale.
        clock.set(ts(32));
        tokio::time::advance(std::time::Duration::from_secs(31)).await;
        let stale = loop {
            let evt = tokio::time::timeout(std::time::Duration::from_secs(5), health_rx.recv())
                .await
                .expect("Stale event did not arrive")
                .expect("channel closed");
            if !matches!(evt, MarketHealth::Fresh { .. }) {
                break evt;
            }
        };
        assert!(
            matches!(stale, MarketHealth::Stale { .. }),
            "expected Stale, got {stale:?}"
        );

        // Step 3: a fresh tick lands and the watchdog runs again.  The
        // tick `local_recv_ts` is "now" (t=32); after the next scan the
        // age is 0s (well under threshold) so the venue transitions
        // Stale → Fresh and emits `Recovered`.
        let recovery_ts = ts(32);
        last_tick
            .lock()
            .expect("lock")
            .insert(Venue::Kraken, recovery_ts);
        clock.set(ts(33));
        tokio::time::advance(std::time::Duration::from_secs(1)).await;

        let recovered = tokio::time::timeout(std::time::Duration::from_secs(5), health_rx.recv())
            .await
            .expect("Recovered event did not arrive")
            .expect("channel closed");
        match recovered {
            MarketHealth::Recovered {
                venue,
                recovered_ts: rec_ts,
                gap_secs,
            } => {
                assert_eq!(venue, Venue::Kraken);
                assert_eq!(rec_ts, recovery_ts);
                // gap_secs is the age at scan time (now=t33, last=t32 → 1s).
                assert!(gap_secs <= 1, "expected gap_secs ~0..1, got {gap_secs}");
            }
            other => panic!("expected Recovered, got {other:?}"),
        }

        cancel.cancel();
        let drain = async { while set.join_next().await.is_some() {} };
        tokio::time::timeout(std::time::Duration::from_secs(5), drain)
            .await
            .expect("watchdog did not drain inside budget");
    }
}
