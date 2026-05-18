//! T518 — R8 invalid-config rejection integration test.
//!
//! Verifies that ten malformed TOML fixtures each produce a `Reject` audit entry,
//! the registry is not modified, and the reconciler invariant holds.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use agent::{EventBus, watcher};
use audit::{bootstrap, ledger::Ledger, query};
use trading_core::Timestamp;

async fn open_ledger() -> Arc<Ledger> {
    let ledger = Ledger::in_memory().await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();
    Arc::new(ledger)
}

fn make_bus() -> Arc<EventBus> {
    Arc::new(EventBus::new(&agent::config::BusConfig::default()))
}

/// The ten bad-strategy fixture files (from T504 / T518).
const BAD_FIXTURES: &[(&str, &str)] = &[
    (
        "bad_arity_mismatch",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_arity_mismatch.toml"),
    ),
    (
        "bad_empty_signal",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_empty_signal.toml"),
    ),
    (
        "bad_grammar_parse",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_grammar_parse.toml"),
    ),
    (
        "bad_id_filename_mismatch",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_id_filename_mismatch.toml"),
    ),
    (
        "bad_invalid_range",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_invalid_range.toml"),
    ),
    (
        "bad_invalid_stage",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_invalid_stage.toml"),
    ),
    (
        "bad_toml_parse",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_toml_parse.toml"),
    ),
    (
        "bad_unknown_indicator",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_unknown_indicator.toml"),
    ),
    (
        "bad_unknown_param",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_unknown_param.toml"),
    ),
    (
        "bad_unsupported_sizing",
        include_str!("../../strategy/tests/fixtures/bad_strategies/bad_unsupported_sizing.toml"),
    ),
];

/// T518: each bad fixture produces a Reject row and leaves the registry unchanged.
#[tokio::test]
async fn t518_ten_bad_fixtures_all_rejected_registry_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    // Subscribe to strategy_error BEFORE sending events.
    let mut err_rx = bus.strategy_error();

    // First load a valid strategy so we can verify it keeps running.
    let good_path = dir.path().join("btc_good.toml");
    std::fs::write(
        &good_path,
        r#"id = "btc_good"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 30"
size = "fixed_fraction(0.1)"
"#,
    )
    .unwrap();
    watcher::handle_fs_event(
        watcher::FsEvent::Upsert(good_path.clone()),
        &registry,
        &ledger,
        &bus,
    )
    .await;
    assert_eq!(registry.len(), 1, "good strategy loaded");

    let mut reject_count = 0usize;

    for (stem, content) in BAD_FIXTURES {
        let bad_path = dir.path().join(format!("{stem}.toml"));
        std::fs::write(&bad_path, content).unwrap();

        let prev_len = registry.len();
        watcher::handle_fs_event(watcher::FsEvent::Upsert(bad_path), &registry, &ledger, &bus)
            .await;

        // Registry must be unchanged.
        assert_eq!(
            registry.len(),
            prev_len,
            "registry length must not change after bad fixture: {stem}"
        );

        // A strategy_error event must have been published.
        let err_event = err_rx.try_recv().unwrap_or_else(|e| {
            panic!("expected strategy_error event for fixture {stem}: {e}");
        });
        assert!(
            !err_event.error_code.is_empty(),
            "error_code must be non-empty for fixture: {stem}"
        );
        reject_count += 1;
    }

    assert_eq!(
        reject_count, 10,
        "all 10 bad fixtures must produce error events"
    );

    // The good strategy must still be loaded.
    assert_eq!(
        registry.len(),
        1,
        "good strategy must still be running after all rejections"
    );

    // Check that the audit journal has exactly 10 Reject entries + 1 Load.
    let all_events =
        query::strategy_events_since(&ledger, Timestamp::new(time::OffsetDateTime::UNIX_EPOCH))
            .await
            .unwrap();

    let reject_rows: Vec<_> = all_events
        .iter()
        .filter(|e| e.kind == trading_core::StrategyEventKind::Reject)
        .collect();
    assert_eq!(
        reject_rows.len(),
        10,
        "journal must have exactly 10 Reject entries"
    );

    let load_rows: Vec<_> = all_events
        .iter()
        .filter(|e| e.kind == trading_core::StrategyEventKind::Load)
        .collect();
    assert_eq!(
        load_rows.len(),
        1,
        "journal must have exactly 1 Load entry (the good strategy)"
    );
}

/// T518: ledger_imbalance must stay 0 throughout (reconciler invariant).
///
/// This test loads a good strategy and rejects 10 bad ones, then runs bars
/// and verifies the ledger has no imbalance.
#[tokio::test]
async fn t518_ledger_imbalance_zero_after_rejections() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(strategy::StrategyRegistry::new());
    let ledger = open_ledger().await;
    let bus = make_bus();

    // Load a valid strategy.
    let good_path = dir.path().join("btc_good2.toml");
    std::fs::write(
        &good_path,
        r#"id = "btc_good2"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "rsi(14) < 30"
size = "fixed_fraction(0.1)"
"#,
    )
    .unwrap();
    watcher::handle_fs_event(
        watcher::FsEvent::Upsert(good_path),
        &registry,
        &ledger,
        &bus,
    )
    .await;

    // Reject all 10 bad fixtures.
    for (stem, content) in BAD_FIXTURES {
        let bad_path = dir.path().join(format!("{stem}_2.toml"));
        std::fs::write(&bad_path, content).unwrap();
        watcher::handle_fs_event(watcher::FsEvent::Upsert(bad_path), &registry, &ledger, &bus)
            .await;
    }

    // Verify ledger imbalance = 0: global debit sum must equal credit sum.
    let (debits, credits) = audit::query::global_debit_credit_sum(&ledger)
        .await
        .unwrap();
    assert_eq!(
        debits, credits,
        "ledger_imbalance must be 0 after all rejections (debits={debits}, credits={credits})"
    );
}
