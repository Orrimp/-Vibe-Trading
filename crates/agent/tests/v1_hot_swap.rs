//! T619 — v1 momentum strategy hot-swap integration test.
//!
//! Verifies that the file watcher correctly:
//! 1. Loads a `cross_sectional_momentum` TOML config (StrategyLoaded event).
//! 2. Detects a TOML change (new k_long) and performs an atomic hot-swap
//!    (StrategySwapped event with distinct hashes).
//! 3. Rejects an invalid config (StrategyLoadError event, old strategy retained).
//!
//! The test uses `handle_fs_event_with_clock` to inject a fixed timestamp for
//! deterministic audit rows (architect risk #4).
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use agent::{watcher, EventBus};
use audit::{bootstrap, ledger::Ledger, query};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp};

const REPLAY_TS: &str = "1970-05-27T19:07:10Z";

async fn open_ledger() -> Arc<Ledger> {
    let ledger = Ledger::in_memory().await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();
    Arc::new(ledger)
}

fn make_bus() -> Arc<EventBus> {
    Arc::new(EventBus::new(&agent::config::BusConfig::default()))
}

/// Write a valid cross_sectional_momentum TOML with the given k_long.
fn momentum_toml(k_long: u32) -> String {
    format!(
        r#"id = "test_momentum_v1"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
lookback_minutes = 10
rebalance_minutes = 10
k_long = {k_long}
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#
    )
}

/// Make a synthetic bar for one symbol.
#[allow(dead_code)]
fn make_bar(symbol: &str, close: f64, offset_minutes: i64) -> Bar {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    let base = time::OffsetDateTime::UNIX_EPOCH;
    let ts = Timestamp::new(base + time::Duration::minutes(offset_minutes));
    let price = Price::new(Decimal::try_from(close).unwrap()).unwrap();
    Bar {
        symbol: Symbol::new(symbol),
        tf: Timeframe::OneMinute,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: Quantity::new(dec!(1)).unwrap(),
        trade_count: 1,
        local_recv_ts: ts,
        open_ts: ts,
        close_ts: ts,
    }
}

// ── T619-A: load momentum strategy → StrategyLoaded audit event ───────────────

#[tokio::test]
async fn t619_momentum_load_records_strategy_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let strat_path = dir.path().join("test_momentum_v1.toml");
    std::fs::write(&strat_path, momentum_toml(2)).unwrap();

    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
    )
    .await;

    assert_eq!(registry.len(), 1, "strategy should be loaded after Upsert");

    // Verify audit trail.
    let history =
        query::strategy_history(&ledger, trading_core::StrategyId::new("test_momentum_v1"))
            .await
            .unwrap();

    assert_eq!(history.len(), 1, "should have exactly one event");
    assert_eq!(
        history[0].kind,
        trading_core::StrategyEventKind::Load,
        "first event must be Load"
    );
    assert!(
        history[0].new_hash.is_some(),
        "load event must record config hash"
    );
}

// ── T619-B: hot-swap → new hash, StrategySwapped event ───────────────────────

#[tokio::test]
async fn t619_momentum_hot_swap_records_swapped_event() {
    let dir = tempfile::tempdir().unwrap();
    let strat_path = dir.path().join("test_momentum_v1.toml");

    // Initial load with k_long=2.
    std::fs::write(&strat_path, momentum_toml(2)).unwrap();

    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
    )
    .await;

    let history_before =
        query::strategy_history(&ledger, trading_core::StrategyId::new("test_momentum_v1"))
            .await
            .unwrap();
    assert_eq!(history_before.len(), 1);
    let load_hash = history_before[0].new_hash.clone().unwrap_or_default();

    // Hot-swap: change k_long from 2 → 3.
    std::fs::write(&strat_path, momentum_toml(3)).unwrap();

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
    )
    .await;

    // Should still have exactly one registered strategy.
    assert_eq!(
        registry.len(),
        1,
        "registry must have exactly one strategy after swap"
    );

    let history_after =
        query::strategy_history(&ledger, trading_core::StrategyId::new("test_momentum_v1"))
            .await
            .unwrap();

    assert_eq!(history_after.len(), 2, "should have [Load, Swap] events");
    assert_eq!(history_after[0].kind, trading_core::StrategyEventKind::Load);
    assert_eq!(history_after[1].kind, trading_core::StrategyEventKind::Swap);

    let swap_new_hash = history_after[1].new_hash.clone().unwrap_or_default();

    // The watcher uses an all-zeros placeholder for old_hash in the Swap event
    // (the registry does not yet track per-strategy hashes at audit time).
    // The important invariant: old and new hashes are different (config changed).
    assert_ne!(
        load_hash, swap_new_hash,
        "swap event new_hash must differ from the initial load hash (k_long changed)"
    );
}

// ── T619-C: invalid TOML → StrategyLoadError, old strategy retained ───────────

#[tokio::test]
async fn t619_invalid_toml_records_error_and_retains_old_strategy() {
    let dir = tempfile::tempdir().unwrap();
    let strat_path = dir.path().join("test_momentum_v1.toml");

    // Initial valid load.
    std::fs::write(&strat_path, momentum_toml(2)).unwrap();

    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
    )
    .await;
    assert_eq!(registry.len(), 1);

    // Overwrite with invalid TOML (k_short=1 → rejected per Q3).
    let bad_toml = r#"id = "test_momentum_v1"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 10
rebalance_minutes = 10
k_long = 2
k_short = 1
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#;
    std::fs::write(&strat_path, bad_toml).unwrap();

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
    )
    .await;

    // Old strategy must still be registered.
    assert_eq!(
        registry.len(),
        1,
        "old strategy must be retained after rejection"
    );

    // Audit trail: [Load, Reject].
    let history =
        query::strategy_history(&ledger, trading_core::StrategyId::new("test_momentum_v1"))
            .await
            .unwrap();

    assert_eq!(history.len(), 2, "should have [Load, Reject] events");
    assert_eq!(history[0].kind, trading_core::StrategyEventKind::Load);
    assert_eq!(history[1].kind, trading_core::StrategyEventKind::Reject);
}

// ── T619-D: bus receives the correct event type ───────────────────────────────

#[tokio::test]
async fn t619_bus_receives_strategy_loaded_event() {
    let dir = tempfile::tempdir().unwrap();
    let strat_path = dir.path().join("test_momentum_v1.toml");
    std::fs::write(&strat_path, momentum_toml(1)).unwrap();

    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = Arc::new(EventBus::new(&agent::config::BusConfig::default()));

    // Subscribe before the event fires.
    let mut loaded_rx = bus.strategy_loaded();

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
    )
    .await;

    // The bus should have received a StrategyLoaded event.
    let event = loaded_rx.try_recv();
    assert!(
        event.is_ok(),
        "bus must receive StrategyLoaded event on initial load"
    );
    let event = event.unwrap();
    assert_eq!(
        event.id.to_string(),
        "test_momentum_v1",
        "StrategyLoaded event must carry the correct strategy id"
    );
}
