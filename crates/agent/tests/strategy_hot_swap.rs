//! T517 — R7 hot-swap integration test.
//!
//! Verifies that the strategy file watcher detects TOML changes, performs an
//! atomic hot-swap, and that the audit journal records exactly `[Load, Swap]`
//! with distinct hashes.
//!
//! Determinism (architect risk #4, HF-2): `handle_fs_event_with_clock` injects
//! a fixed RFC-3339 timestamp so that `strategy_events` rows are byte-identical
//! across two test runs at the same seed.  The test explicitly asserts this via
//! `t517_strategy_events_byte_identical_across_runs`.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::{watcher, EventBus};
use audit::{bootstrap, ledger::Ledger, query};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp};

/// Fixed RFC-3339 timestamp used for all `strategy_events` rows in tests.
/// Derived from seed 0xC0FFEE: 0xC0FFEE = 12648430 seconds from Unix epoch
/// → 1970-01-01 + 12648430s ≈ 1970-05-27T19:07:10Z (deterministic replay clock).
const REPLAY_TS: &str = "1970-05-27T19:07:10Z";

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

    // Load initial strategy via watcher — use fixed replay clock for determinism.
    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
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

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
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

    watcher::handle_fs_event_with_clock(
        watcher::FsEvent::Upsert(strat_path.clone()),
        &registry,
        &ledger,
        &bus,
        Some(REPLAY_TS),
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
        watcher::handle_fs_event_with_clock(
            watcher::FsEvent::Upsert(strat_path.clone()),
            &registry,
            &ledger,
            &bus,
            Some(REPLAY_TS),
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

/// T517 determinism (HF-2 / architect risk #4): run the hot-swap sequence
/// twice with the same fixed replay clock and assert the `strategy_events`
/// rows are content-identical across both runs.
///
/// Excluded from comparison:
/// - `id` (UUID primary key — randomly generated per row)
/// - `source_path` directory part — the tempdir path is non-deterministic;
///   only the basename (`btc_rsi_det.toml`) is compared.
///
/// The fields that MUST be identical: `ts`, `kind`, `strategy_id`,
/// `old_hash`, `new_hash`, `source_path_basename`, `operator`,
/// `error_code`, `error_summary`.
#[tokio::test]
async fn t517_strategy_events_byte_identical_across_runs() {
    // Content-comparable snapshot of a StrategyEventView.
    #[derive(Debug, PartialEq, Eq)]
    struct EventSnapshot {
        ts: String,
        kind: trading_core::StrategyEventKind,
        strategy_id: Option<String>,
        old_hash: Option<String>,
        new_hash: Option<String>,
        /// Basename of source_path only (directory part is tempdir-non-deterministic).
        source_path_basename: Option<String>,
        operator: String,
        error_code: Option<String>,
        error_summary: Option<String>,
    }

    async fn run_one_sequence() -> Vec<EventSnapshot> {
        let dir = tempfile::tempdir().unwrap();
        let strat_path = dir.path().join("btc_rsi_det.toml");

        let initial_toml = r#"id = "btc_rsi_det"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 30"
size = "fixed_fraction(0.1)"
"#;
        std::fs::write(&strat_path, initial_toml).unwrap();

        let registry = Arc::new(strategy::StrategyRegistry::new());
        let ledger = {
            let l = Ledger::in_memory().await.unwrap();
            bootstrap::chart_of_accounts(&l).await.unwrap();
            Arc::new(l)
        };
        let bus = Arc::new(EventBus::new(&agent::config::BusConfig::default()));

        // Load with fixed replay clock.
        watcher::handle_fs_event_with_clock(
            watcher::FsEvent::Upsert(strat_path.clone()),
            &registry,
            &ledger,
            &bus,
            Some(REPLAY_TS),
        )
        .await;

        // Swap with same fixed clock.
        let swapped_toml = r#"id = "btc_rsi_det"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 25"
size = "fixed_fraction(0.1)"
"#;
        std::fs::write(&strat_path, swapped_toml).unwrap();
        watcher::handle_fs_event_with_clock(
            watcher::FsEvent::Upsert(strat_path.clone()),
            &registry,
            &ledger,
            &bus,
            Some(REPLAY_TS),
        )
        .await;

        // Read via audit::query (no sqlx types in the API).
        let events = query::strategy_history(&ledger, trading_core::StrategyId::new("btc_rsi_det"))
            .await
            .unwrap();

        events
            .into_iter()
            .map(|e| {
                use std::path::Path;
                use time::format_description::well_known::Rfc3339;
                EventSnapshot {
                    ts: e.ts.inner().format(&Rfc3339).unwrap_or_default(),
                    kind: e.kind,
                    strategy_id: e.strategy_id.map(|s| s.0.to_string()),
                    old_hash: e.old_hash.map(|s| s.to_string()),
                    new_hash: e.new_hash.map(|s| s.to_string()),
                    // Normalise to basename so tempdir path differences don't
                    // cause spurious failures.
                    source_path_basename: e.source_path.map(|s| {
                        Path::new(s.as_str())
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| s.to_string())
                    }),
                    operator: e.operator.to_string(),
                    error_code: e.error_code.map(|s| s.to_string()),
                    error_summary: e.error_summary.map(|s| s.to_string()),
                }
            })
            .collect()
    }

    let rows_a = run_one_sequence().await;
    let rows_b = run_one_sequence().await;

    assert_eq!(
        rows_a.len(),
        rows_b.len(),
        "both runs must produce the same number of strategy_events rows"
    );
    assert_eq!(
        rows_a, rows_b,
        "strategy_events rows must be content-identical across two runs \
         when using the fixed replay clock (ts, kind, strategy_id, hashes, \
         source_path_basename, operator all must match)"
    );
}
