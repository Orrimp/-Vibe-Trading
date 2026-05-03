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

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use data::MarketDataSource;
use futures::StreamExt;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use trading_core::{Symbol, Timeframe};

use crate::config::Config;
use crate::{AgentMode, EventBus, KillSwitch};

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
    // timeframe are hardcoded BTCUSDT / 1m to match the SMA crossover
    // strategy that ships in `config/agent.toml`.
    let feed_symbol = Symbol::new("BTCUSDT");
    let feed_tf = Timeframe::OneMinute;
    match config.mode {
        crate::config::Mode::Research => {
            info!("research mode — replay feed (no live orders)");
            let parquet_root = &config.data.historical.parquet_root;
            let feed: Arc<dyn MarketDataSource> =
                Arc::new(data::ReplayFeed::new(parquet_root, false)); // wallclock pace
            info!(parquet_root = %parquet_root, "replay feed initialized");
            spawn_feed_taps(
                feed.as_ref(),
                Arc::clone(&bus),
                feed_symbol.clone(),
                feed_tf,
                &mut set,
                &cancel,
            )
            .await;
            info!("agent subsystems initialized — entering idle (replay loop in backtest binary)");
        }
        crate::config::Mode::Paper => {
            info!("paper mode — Binance WS feed (paper fills, no real orders)");
            let ws_url = &config.data.sources.binance.ws_url;
            let feed: Arc<dyn MarketDataSource> = Arc::new(data::BinanceFeed::new(ws_url, ws_url));
            info!(ws = %ws_url, "Binance feed initialized");
            spawn_feed_taps(
                feed.as_ref(),
                Arc::clone(&bus),
                feed_symbol.clone(),
                feed_tf,
                &mut set,
                &cancel,
            )
            .await;
        }
    }

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
            set.spawn(async move {
                info!(symbol = %symbol_t, "ticks_tap started");
                loop {
                    tokio::select! {
                        () = cancel_t.cancelled() => break,
                        next = stream.next() => match next {
                            Some(Ok(tick)) => bus_t.publish_tick(tick),
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
    use trading_core::{Bar, Price, Quantity, Side, Tick, Timestamp};

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
}
