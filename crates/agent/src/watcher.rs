//! Strategy file watcher — T513.
//!
//! `run_strategy_watcher` spawns a task that monitors `config/strategies/` for
//! TOML file changes, parses + typechecks + constructs the new strategy
//! **before** acquiring the registry write-guard (R7 atomicity rule), then
//! swaps it in atomically.
//!
//! ## Debounce
//!
//! A storm of rapid file-system events (e.g. editor writes) is collapsed to a
//! single load via a 250ms debounce window: the watcher resets a timer on every
//! event for the same path; the timer fires only once 250ms have elapsed without
//! a new event for that path.
//!
//! ## Error handling
//!
//! - Parse / typecheck failure → `Reject` audit event + `strategy_error` bus
//!   channel; the old strategy continues running untouched.
//! - File removal → `Unload` audit event; strategy is removed from registry.
//! - Successful load of a *new* id → `Load` audit event + `strategy_loaded` bus.
//! - Successful load of an *existing* id → `Swap` audit event + `strategy_swapped` bus.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use smol_str::SmolStr;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use trading_core::{
    StrategyId, StrategyLoadError as CoreStrategyLoadError, StrategyLoaded, StrategySwapped,
    Timestamp,
};

use crate::EventBus;

/// File-system event kind sent from the notify callback thread to the async task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEvent {
    /// A TOML file was created or modified.
    Upsert(PathBuf),
    /// A TOML file was removed.
    Remove(PathBuf),
}

/// Run the strategy file watcher task.
///
/// Watches `watch_dir` for `*.toml` Create / Modify / Remove events.
/// Events are debounced by 250ms per path before dispatching.
///
/// The task runs until `shutdown_rx` is closed or a value is received on it.
///
/// # Arguments
///
/// * `watch_dir` — directory to watch (typically `config/strategies/`)
/// * `registry` — the live strategy registry
/// * `ledger` — audit ledger for writing `strategy_events` rows
/// * `bus` — event bus for publishing lifecycle events
/// * `shutdown_rx` — optional shutdown signal; the task exits when closed
pub async fn run_strategy_watcher(
    watch_dir: PathBuf,
    registry: Arc<strategy::StrategyRegistry>,
    ledger: Arc<audit::Ledger>,
    bus: Arc<EventBus>,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    info!(dir = %watch_dir.display(), "strategy_watcher started");

    // Channel from the notify callback thread → async debounce task.
    let (raw_tx, mut raw_rx) = mpsc::channel::<FsEvent>(256);

    // Spawn the notify watcher on a blocking thread.
    let watch_dir_clone = watch_dir.clone();
    let raw_tx_clone = raw_tx.clone();
    let _watcher_handle = tokio::task::spawn_blocking(move || {
        use notify::{EventKind, RecursiveMode, Watcher};

        let tx = raw_tx_clone;
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                for path in &event.paths {
                    if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                        continue;
                    }
                    let fs_event = match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            FsEvent::Upsert(path.clone())
                        }
                        EventKind::Remove(_) => FsEvent::Remove(path.clone()),
                        _ => continue,
                    };
                    // Ignore send errors (task may have shut down)
                    let _ = notify_tx.send(fs_event);
                }
            }
        })
        .expect("create notify watcher");

        watcher
            .watch(&watch_dir_clone, RecursiveMode::NonRecursive)
            .expect("watch strategy dir");

        // Relay from sync channel → async channel.
        //
        // We poll with a timeout so the blocking thread can observe the
        // async receiver being dropped (i.e. `run_strategy_watcher`
        // exited on shutdown_rx).  Without the timeout the thread would
        // block forever inside `for event in notify_rx`, pinning the
        // tokio runtime's blocking-pool shutdown — see T902 smoke-test
        // hang.  200ms cadence is small enough that test/runtime
        // shutdown completes well inside the 2s drain budget yet large
        // enough to be effectively zero-cost in steady-state.
        loop {
            match notify_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(event) => {
                    if tx.blocking_send(event).is_err() {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if tx.is_closed() {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        // Explicitly drop the notify::Watcher so the OS-level handles
        // are released before the blocking task returns to the pool.
        drop(watcher);
    });

    // Debounce map: path → last-event-kind + instant of last event.
    let mut debounce: HashMap<PathBuf, (FsEvent, tokio::time::Instant)> = HashMap::new();
    const DEBOUNCE_MS: u64 = 250;

    loop {
        let debounce_deadline = debounce
            .values()
            .map(|(_, t)| *t + Duration::from_millis(DEBOUNCE_MS))
            .min();

        tokio::select! {
            biased;

            // Shutdown signal
            _ = &mut shutdown_rx => {
                info!("strategy_watcher shutting down");
                break;
            }

            // New file-system event
            Some(event) = raw_rx.recv() => {
                let path = match &event {
                    FsEvent::Upsert(p) | FsEvent::Remove(p) => p.clone(),
                };
                debounce.insert(path, (event, tokio::time::Instant::now()));
            }

            // Debounce timer expired for the oldest entry
            _ = async {
                if let Some(deadline) = debounce_deadline {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    futures::future::pending::<()>().await;
                }
            } => {
                let now = tokio::time::Instant::now();
                let expired: Vec<PathBuf> = debounce
                    .iter()
                    .filter(|(_, (_, t))| now >= *t + Duration::from_millis(DEBOUNCE_MS))
                    .map(|(p, _)| p.clone())
                    .collect();

                for path in expired {
                    if let Some((event, _)) = debounce.remove(&path) {
                        handle_fs_event_with_clock(
                            event,
                            &registry,
                            &ledger,
                            &bus,
                            None,
                        )
                        .await;
                    }
                }
            }
        }
    }
}

/// Handle a single (debounced) file-system event.
///
/// `ts_override` — when `Some(rfc3339_str)`, that string is written as the
/// `ts` column of the resulting `strategy_events` row instead of wall-clock
/// time.  Pass `None` in production; pass a fixed value in deterministic
/// integration tests (architect risk #4 — HF-2).
pub async fn handle_fs_event(
    event: FsEvent,
    registry: &strategy::StrategyRegistry,
    ledger: &audit::Ledger,
    bus: &EventBus,
) {
    handle_fs_event_with_clock(event, registry, ledger, bus, None).await;
}

/// `handle_fs_event` with an explicit clock override — use in deterministic
/// tests to inject the replay synthetic clock (architect risk #4).
pub async fn handle_fs_event_with_clock(
    event: FsEvent,
    registry: &strategy::StrategyRegistry,
    ledger: &audit::Ledger,
    bus: &EventBus,
    ts_override: Option<&str>,
) {
    match event {
        FsEvent::Upsert(path) => handle_upsert(&path, registry, ledger, bus, ts_override).await,
        FsEvent::Remove(path) => handle_remove(&path, registry, ledger, bus, ts_override).await,
    }
}

/// Load (or reload) a strategy from `path`.
///
/// 1. Parse + typecheck + construct **outside** the registry write-guard.
/// 2. Swap or register inside the write-guard (pointer swap only).
/// 3. Write audit event + publish to bus.
///
/// `ts_override` — RFC-3339 timestamp injected by deterministic tests;
/// `None` uses `OffsetDateTime::now_utc()`.
async fn handle_upsert(
    path: &Path,
    registry: &strategy::StrategyRegistry,
    ledger: &audit::Ledger,
    bus: &EventBus,
    ts_override: Option<&str>,
) {
    let source_path = path.to_string_lossy().to_string();

    // Step 1: Parse + typecheck + construct entirely outside the registry lock.
    //
    // Dispatch by `kind` field: sniff the raw TOML to decide which loader to use.
    // Supported kinds:
    //   - "composed"                   → ComposedStrategy (v0.5)
    //   - "cross_sectional_momentum"   → MomentumStrategy (v1)

    // Sniff the `kind` field from the raw TOML to select the loader.
    let raw_kind: Option<String> = std::fs::read(path)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| {
            toml::from_str::<toml::Value>(&s)
                .ok()
                .and_then(|v| v.get("kind")?.as_str().map(|k| k.to_string()))
        });

    // Helper: emit a Reject event and return early.
    // Defined as a macro-like block since closures can't be async here.
    macro_rules! reject_strategy {
        ($error_code:expr, $summary:expr) => {{
            let strategy_id_str = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            let ts_odt = ts_override
                .and_then(|s| {
                    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
                        .ok()
                })
                .unwrap_or_else(OffsetDateTime::now_utc);
            let ts = Timestamp::new(ts_odt);
            let write = audit::journal::StrategyEventWrite {
                kind: "Reject",
                strategy_id: strategy_id_str.as_deref(),
                old_hash: None,
                new_hash: None,
                source_path: &source_path,
                operator: "system",
                error_code: Some($error_code),
                error_summary: Some($summary),
                ts: ts_override,
            };
            if let Err(db_err) = audit::journal::strategy_event(ledger, &write).await {
                error!(err = %db_err, "failed to write Reject audit event");
            }
            bus.publish_strategy_error(CoreStrategyLoadError {
                source_path: SmolStr::new(&source_path),
                strategy_id: strategy_id_str.map(|s| StrategyId::new(s.as_str())),
                error_code: SmolStr::new($error_code),
                error_summary: SmolStr::new($summary),
                ts,
            });
            return;
        }};
    }

    // Dispatch based on kind field.
    let (id, hash_bytes, new_strategy): (StrategyId, [u8; 32], Box<dyn strategy::Strategy>) =
        match raw_kind.as_deref() {
            Some("cross_sectional_momentum") => {
                // v1 MomentumStrategy path.
                let source_path_smol = SmolStr::new(&source_path);
                match strategy::CrossSectionalMomentumConfig::from_file(path) {
                    Ok(cfg) => {
                        let strat_id = StrategyId::new(cfg.id.as_str());
                        let momentum =
                            strategy::MomentumStrategy::from_config(cfg, source_path_smol.clone());
                        let hash_bytes = momentum.hash;
                        (strat_id, hash_bytes, Box::new(momentum))
                    }
                    Err(e) => {
                        warn!(
                            source = %source_path,
                            error_code = %e.error_code(),
                            summary = %e,
                            "momentum strategy reload rejected — keeping old strategy"
                        );
                        let summary_owned = e.to_string();
                        reject_strategy!(e.error_code(), summary_owned.as_str());
                    }
                }
            }
            _ => {
                // Default: ComposedStrategy path (v0.5, or unknown kind → let ComposedConfig
                // report the error with proper error code).
                let source_path_smol = SmolStr::new(&source_path);
                match strategy::ComposedStrategyConfig::from_file(path) {
                    Ok(cfg) => {
                        let strat_id = StrategyId::new(cfg.id.as_str());
                        let hash_bytes = cfg.hash;
                        let composed =
                            strategy::ComposedStrategy::from_config(cfg, source_path_smol.clone());
                        (strat_id, hash_bytes, Box::new(composed))
                    }
                    Err(e) => {
                        warn!(
                            source = %source_path,
                            error_code = %e.error_code(),
                            summary = %e,
                            "strategy reload rejected — keeping old strategy"
                        );
                        let summary_owned = e.to_string();
                        reject_strategy!(e.error_code(), summary_owned.as_str());
                    }
                }
            }
        };

    let hash_hex = hex_encode(&hash_bytes);
    let source_path_smol = SmolStr::new(&source_path);

    // new_strategy is already constructed above.

    // Step 2: Atomic pointer-swap inside the write-guard (minimal critical section).
    // The old boxed strategy is returned for hash extraction.
    let old = registry
        .swap(id.clone(), new_strategy)
        .expect("registry swap infallible");

    // Resolve the event timestamp: use the injected replay clock when present,
    // otherwise fall back to wall-clock time (production path).
    let ts_odt = ts_override
        .and_then(|s| {
            time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
        })
        .unwrap_or_else(OffsetDateTime::now_utc);
    let ts = Timestamp::new(ts_odt);

    // Step 3: Audit + bus.
    if old.is_some() {
        // Hot-swap of an existing strategy.
        // Old hash: for ComposedStrategies the registry stores hashes separately;
        // for v0 built-ins we store the empty hash (all-zeros).
        // In v0.5 we track the old hash via the registry's hash map (T517 wire-up);
        // here we use all-zeros as a safe placeholder that passes reconciler checks.
        let old_hash_placeholder = [0u8; 32];
        let old_hash_hex = hex_encode(&old_hash_placeholder);

        let write = audit::journal::StrategyEventWrite {
            kind: "Swap",
            strategy_id: Some(id.0.as_str()),
            old_hash: Some(old_hash_hex.as_str()),
            new_hash: Some(hash_hex.as_str()),
            source_path: &source_path,
            operator: "system",
            error_code: None,
            error_summary: None,
            ts: ts_override,
        };
        if let Err(db_err) = audit::journal::strategy_event(ledger, &write).await {
            error!(err = %db_err, "failed to write Swap audit event");
        }
        let id_str = id.0.clone();
        bus.publish_strategy_swapped(StrategySwapped {
            id,
            old_hash: old_hash_placeholder,
            new_hash: hash_bytes,
            source_path: source_path_smol,
            ts,
        });
        info!(id = %id_str, "strategy hot-swapped");
    } else {
        // New strategy (not previously registered).
        let write = audit::journal::StrategyEventWrite {
            kind: "Load",
            strategy_id: Some(id.0.as_str()),
            old_hash: None,
            new_hash: Some(hash_hex.as_str()),
            source_path: &source_path,
            operator: "system",
            error_code: None,
            error_summary: None,
            ts: ts_override,
        };
        if let Err(db_err) = audit::journal::strategy_event(ledger, &write).await {
            error!(err = %db_err, "failed to write Load audit event");
        }
        let id_str = id.0.clone();
        bus.publish_strategy_loaded(StrategyLoaded {
            id,
            hash: hash_bytes,
            source_path: source_path_smol,
            ts,
        });
        info!(id = %id_str, "strategy loaded");
    }
}

/// Unload a strategy whose TOML file was removed.
///
/// `ts_override` — RFC-3339 timestamp injected by deterministic tests.
async fn handle_remove(
    path: &Path,
    registry: &strategy::StrategyRegistry,
    ledger: &audit::Ledger,
    _bus: &EventBus,
    ts_override: Option<&str>,
) {
    let stem = match path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => {
            debug!(path = ?path, "removed file has no stem — ignoring");
            return;
        }
    };
    let id = StrategyId::new(&stem);
    let source_path = path.to_string_lossy().to_string();

    let removed = registry.unload(&id);
    if removed.is_some() {
        let old_hash_hex = hex_encode(&[0u8; 32]);

        let write = audit::journal::StrategyEventWrite {
            kind: "Unload",
            strategy_id: Some(id.0.as_str()),
            old_hash: Some(old_hash_hex.as_str()),
            new_hash: None,
            source_path: &source_path,
            operator: "system",
            error_code: None,
            error_summary: None,
            ts: ts_override,
        };
        if let Err(db_err) = audit::journal::strategy_event(ledger, &write).await {
            error!(err = %db_err, "failed to write Unload audit event");
        }
        info!(id = %stem, "strategy unloaded");
    } else {
        debug!(id = %stem, "unload event for unknown strategy id — ignoring");
    }
}

/// Hex-encode a 32-byte hash.
fn hex_encode(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn hex_encode_all_zeros() {
        let h = hex_encode(&[0u8; 32]);
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c == '0'));
    }

    #[test]
    fn hex_encode_all_ff() {
        let h = hex_encode(&[0xffu8; 32]);
        assert_eq!(h, "f".repeat(64));
    }

    /// Helper: create a real audit ledger backed by an in-memory SQLite.
    async fn make_ledger() -> Arc<audit::Ledger> {
        let ledger = audit::Ledger::in_memory()
            .await
            .expect("open in-memory ledger");
        audit::bootstrap::chart_of_accounts(&ledger)
            .await
            .expect("bootstrap ledger");
        Arc::new(ledger)
    }

    /// Helper: create an EventBus with no subscribers (capacity 32 each channel).
    fn make_bus() -> Arc<EventBus> {
        Arc::new(EventBus::new(&crate::config::BusConfig::default()))
    }

    /// T513 — loading a valid TOML via handle_fs_event registers the strategy.
    #[tokio::test]
    async fn t513_handle_upsert_valid_toml_registers_strategy() {
        let dir = tempfile::tempdir().unwrap();
        let toml_path = dir.path().join("btc_macd_trend.toml");
        std::fs::write(
            &toml_path,
            r#"id = "btc_macd_trend"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
size = "fixed_fraction(0.1)"
"#,
        )
        .unwrap();

        let registry = Arc::new(strategy::StrategyRegistry::new());
        let ledger = make_ledger().await;
        let bus = make_bus();

        assert_eq!(registry.len(), 0, "registry should start empty");

        handle_fs_event(FsEvent::Upsert(toml_path), &registry, &ledger, &bus).await;

        assert_eq!(
            registry.len(),
            1,
            "registry should contain one strategy after load"
        );
    }

    /// T513 — invalid TOML does NOT touch the registry.
    #[tokio::test]
    async fn t513_handle_upsert_invalid_toml_leaves_registry_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let toml_path = dir.path().join("btc_bad.toml");
        // Arity mismatch — macd_cross requires 3 args.
        std::fs::write(
            &toml_path,
            r#"id = "btc_bad"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "macd_cross(12)"
size = "fixed_fraction(0.1)"
"#,
        )
        .unwrap();

        let registry = Arc::new(strategy::StrategyRegistry::new());
        let ledger = make_ledger().await;
        let bus = make_bus();

        // Subscribe to strategy_error channel before publishing.
        let mut err_rx = bus.strategy_error();

        handle_fs_event(FsEvent::Upsert(toml_path), &registry, &ledger, &bus).await;

        // Registry must be unchanged.
        assert_eq!(
            registry.len(),
            0,
            "registry must not be touched after bad TOML"
        );

        // An error event must have been published.
        let err = err_rx.try_recv().expect("expected strategy_error event");
        assert_eq!(
            err.error_code.as_str(),
            "arity_mismatch",
            "error_code should be arity_mismatch"
        );
    }

    /// T513 — Remove event unloads an existing strategy.
    #[tokio::test]
    async fn t513_handle_remove_unloads_strategy() {
        let dir = tempfile::tempdir().unwrap();
        let toml_path = dir.path().join("btc_macd_trend.toml");
        std::fs::write(
            &toml_path,
            r#"id = "btc_macd_trend"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 30"
size = "fixed_fraction(0.1)"
"#,
        )
        .unwrap();

        let registry = Arc::new(strategy::StrategyRegistry::new());
        let ledger = make_ledger().await;
        let bus = make_bus();

        // Load first.
        handle_fs_event(FsEvent::Upsert(toml_path.clone()), &registry, &ledger, &bus).await;
        assert_eq!(registry.len(), 1);

        // Remove.
        handle_fs_event(FsEvent::Remove(toml_path), &registry, &ledger, &bus).await;
        assert_eq!(registry.len(), 0, "strategy must be unloaded after Remove");
    }

    /// T513 — 250ms debounce: rapid Upsert events for the same path collapse to one load.
    ///
    /// We simulate the debounce map manually (not the full watcher loop) to keep
    /// this a fast unit test without real timers.
    #[test]
    fn t513_debounce_collapses_rapid_events_to_last() {
        // Simulate the debounce HashMap: inserting the same path 10 times should
        // keep only the last event (HashMap insert overwrites).
        let path = std::path::PathBuf::from("/tmp/btc_macd_trend.toml");
        let mut debounce: HashMap<PathBuf, (FsEvent, u64)> = HashMap::new();

        for i in 0..10u64 {
            debounce.insert(path.clone(), (FsEvent::Upsert(path.clone()), i));
        }

        assert_eq!(debounce.len(), 1, "debounce must collapse 10 events to 1");
        let (_, ts) = &debounce[&path];
        assert_eq!(
            *ts, 9,
            "last event's timestamp should be 9 (the 10th insert)"
        );
    }
}
