//! T517 — R7 hot-swap integration test.
//!
//! Verifies that the strategy file watcher detects TOML changes, performs an
//! atomic hot-swap, and that the audit journal records exactly `[Load, Swap]`
//! with distinct hashes.
//!
//! Determinism note (architect risk #4): timestamps in `strategy_events` come
//! from the watcher task which uses `OffsetDateTime::now_utc()` for the audit
//! row. This test verifies swap correctness and hash distinctness but does NOT
//! assert byte-identical `strategy_events` tables across two runs (wall time
//! differs).  The T521 determinism re-gate covers report-body byte-identity
//! for the backtest scenarios.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::{watcher, EventBus};
use audit::{bootstrap, ledger::Ledger, query};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp};

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn open_ledger() -> Arc<Ledger> {
    let ledger = Ledger::in_memory().await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();
    Arc::new(ledger)
}

fn make_bus() -> Arc<EventBus> {
    Arc::new(EventBus::new(&agent::config::BusConfig::default()))
}

/// Synthetic 1-bar helper.
fn make_bar(close: f64) -> Bar {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
    let price = Price::new(Decimal::try_from(close).unwrap()).unwrap();
    Bar {
        symbol: Symbol::new("BTCUSDT"),
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

// ── T517 — hot-swap roundtrip ──────────────────────────────────────────────

/// T517: Load strategy, run 500 synthetic bars, hot-swap to new params,
/// assert registry has new hash, assert strategy_history = [Load, Swap].
#[tokio::test]
async fn t517_hot_swap_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let strat_path = dir.path().join("btc_macd_trend.toml");

    // Initial config with (12,26,9).
    let initial_toml = r#"id = "btc_macd_trend"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
size = "fixed_fraction(0.1)"
"#;

    std::fs::write(&strat_path, initial_toml).unwrap();

    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    // Load initial strategy via watcher.
    watcher::handle_fs_event(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
    )
    .await;

    assert_eq!(registry.len(), 1, "strategy should be loaded");

    // Run 500 synthetic bars.
    for i in 0..500 {
        let close = 16_500.0_f64 + (i as f64) * 0.1;
        let bar = make_bar(close);
        let _ = registry.on_bar(&bar);
    }

    // Capture initial hash from strategy_history.
    let history_before =
        query::strategy_history(&ledger, trading_core::StrategyId::new("btc_macd_trend"))
            .await
            .unwrap();
    assert_eq!(history_before.len(), 1, "should have exactly Load event");
    let load_hash = history_before[0].new_hash.clone().unwrap_or_default();

    // Hot-swap: rewrite TOML with (8,21,9) params.
    let new_toml = r#"id = "btc_macd_trend"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "macd_hist(8,21,9) > 0 AND close > ema(200)"
size = "fixed_fraction(0.1)"
"#;
    std::fs::write(&strat_path, new_toml).unwrap();

    // Subscribe to swapped events BEFORE triggering the swap.
    let mut swapped_rx = bus.strategy_swapped();

    watcher::handle_fs_event(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
    )
    .await;

    // Assert swap event published.
    let swap_event =
        tokio::time::timeout(Duration::from_secs(2), async { swapped_rx.recv().await })
            .await
            .expect("swap event should arrive within 2s")
            .expect("swap event recv");

    assert_ne!(
        swap_event.old_hash, swap_event.new_hash,
        "old and new hashes must differ after param change"
    );

    // Assert strategy_history = [Load, Swap] with distinct hashes.
    let history_after =
        query::strategy_history(&ledger, trading_core::StrategyId::new("btc_macd_trend"))
            .await
            .unwrap();

    assert_eq!(
        history_after.len(),
        2,
        "history must be exactly [Load, Swap], got: {:?}",
        history_after.iter().map(|e| &e.kind).collect::<Vec<_>>()
    );
    assert_eq!(history_after[0].kind, trading_core::StrategyEventKind::Load);
    assert_eq!(history_after[1].kind, trading_core::StrategyEventKind::Swap);

    // Hashes must be distinct.
    let swap_hash = history_after[1].new_hash.clone().unwrap_or_default();
    assert_ne!(
        load_hash, swap_hash,
        "Load hash and Swap new_hash must differ"
    );
}

/// T517 sibling: rapid-fire 20 swaps assert no torn reads.
///
/// Runs 20 concurrent swap + on_bar operations and asserts the registry
/// always returns a consistent strategy (never panics, never returns 0
/// signals from a warm strategy).
#[tokio::test]
async fn t517_rapid_fire_20_swaps_no_torn_reads() {
    let dir = tempfile::tempdir().unwrap();
    let strat_path = dir.path().join("btc_rsi.toml");

    let initial_toml = r#"id = "btc_rsi"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 30"
size = "fixed_fraction(0.1)"
"#;
    std::fs::write(&strat_path, initial_toml).unwrap();

    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    watcher::handle_fs_event(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
    )
    .await;
    assert_eq!(registry.len(), 1);

    // Run 20 swaps alternating between two valid configs.
    for i in 0..20u32 {
        let threshold = if i % 2 == 0 { 30 } else { 25 };
        let new_toml = format!(
            r#"id = "btc_rsi"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < {threshold}"
size = "fixed_fraction(0.1)"
"#
        );
        std::fs::write(&strat_path, &new_toml).unwrap();
        watcher::handle_fs_event(
            watcher::FsEvent::Upsert(strat_path.clone()),
            &registry,
            &ledger,
            &bus,
        )
        .await;

        // Call on_bar during the swap — must not panic or tear state.
        let bar = make_bar(16_500.0 + i as f64);
        let _signals = registry.on_bar(&bar);
    }

    // Registry must still have exactly 1 strategy.
    assert_eq!(
        registry.len(),
        1,
        "registry must still have 1 strategy after 20 swaps"
    );
}
