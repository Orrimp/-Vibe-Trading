//! Integration tests for strategy_events table (T508, T509, T510).

use audit::{bootstrap, journal, ledger::Ledger, query};
use time::OffsetDateTime;
use trading_core::{StrategyEventKind, StrategyId, Timestamp};

async fn open_test_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap accounts");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

#[tokio::test]
async fn t508_strategy_events_table_exists() {
    let ledger = open_test_ledger().await;
    // If the table doesn't exist, the query below will fail.
    let rows: Vec<(String,)> = sqlx::query_as::<_, (String,)>(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='strategy_events'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("query sqlite_master");
    assert_eq!(rows.len(), 1, "strategy_events table should exist");
}

#[tokio::test]
async fn t509_write_and_read_load_event() {
    let ledger = open_test_ledger().await;

    let write = journal::StrategyEventWrite {
        kind: "Load",
        strategy_id: Some("btc_macd_trend"),
        old_hash: None,
        new_hash: Some("aabbccdd"),
        source_path: "config/strategies/btc_macd_trend.toml",
        operator: "system",
        error_code: None,
        error_summary: None,
    };
    journal::strategy_event(&ledger, &write)
        .await
        .expect("write strategy event");

    let history = query::strategy_history(&ledger, StrategyId::new("btc_macd_trend"))
        .await
        .expect("read strategy history");

    assert_eq!(history.len(), 1, "expected exactly 1 event");
    assert_eq!(history[0].kind, StrategyEventKind::Load);
    assert_eq!(history[0].new_hash.as_deref(), Some("aabbccdd"));
    assert!(history[0].error_code.is_none());
}

#[tokio::test]
async fn t509_write_all_event_kinds_and_read_in_order() {
    let ledger = open_test_ledger().await;
    let id = "test_strategy";

    // Load
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Load",
            strategy_id: Some(id),
            old_hash: None,
            new_hash: Some("hash_v1"),
            source_path: "config/strategies/test.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
        },
    )
    .await
    .unwrap();

    // Swap
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Swap",
            strategy_id: Some(id),
            old_hash: Some("hash_v1"),
            new_hash: Some("hash_v2"),
            source_path: "config/strategies/test.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
        },
    )
    .await
    .unwrap();

    // Reject (invalid reload attempt)
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Reject",
            strategy_id: Some(id),
            old_hash: None,
            new_hash: None,
            source_path: "config/strategies/test.toml",
            operator: "system",
            error_code: Some("arity_mismatch"),
            error_summary: Some("macd_cross(12): expected 3 args"),
        },
    )
    .await
    .unwrap();

    // Unload
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Unload",
            strategy_id: Some(id),
            old_hash: Some("hash_v2"),
            new_hash: None,
            source_path: "config/strategies/test.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
        },
    )
    .await
    .unwrap();

    let history = query::strategy_history(&ledger, StrategyId::new(id))
        .await
        .expect("read strategy history");

    assert_eq!(history.len(), 4, "expected exactly 4 events");
    assert_eq!(history[0].kind, StrategyEventKind::Load);
    assert_eq!(history[1].kind, StrategyEventKind::Swap);
    assert_eq!(history[2].kind, StrategyEventKind::Reject);
    assert_eq!(history[3].kind, StrategyEventKind::Unload);

    // Hashes
    assert_eq!(history[0].new_hash.as_deref(), Some("hash_v1"));
    assert_eq!(history[1].old_hash.as_deref(), Some("hash_v1"));
    assert_eq!(history[1].new_hash.as_deref(), Some("hash_v2"));
    assert_eq!(history[3].old_hash.as_deref(), Some("hash_v2"));

    // Error fields only on Reject
    assert_eq!(history[2].error_code.as_deref(), Some("arity_mismatch"));
    assert!(history[0].error_code.is_none());
}

#[tokio::test]
async fn t510_strategy_events_do_not_affect_balance() {
    let ledger = open_test_ledger().await;

    // Record the global debit/credit sums before strategy events.
    let (debits_before, credits_before) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("global sum before");

    // Write several strategy events.
    for kind in &["Load", "Swap", "Unload", "Reject"] {
        journal::strategy_event(
            &ledger,
            &journal::StrategyEventWrite {
                kind,
                strategy_id: Some("test_strategy"),
                old_hash: None,
                new_hash: None,
                source_path: "config/strategies/test.toml",
                operator: "system",
                error_code: if *kind == "Reject" {
                    Some("empty_signal")
                } else {
                    None
                },
                error_summary: None,
            },
        )
        .await
        .unwrap();
    }

    // The global debit/credit sums should be UNCHANGED.
    let (debits_after, credits_after) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("global sum after");

    assert_eq!(
        debits_before, debits_after,
        "strategy_events must not affect total debits"
    );
    assert_eq!(
        credits_before, credits_after,
        "strategy_events must not affect total credits"
    );
}

#[tokio::test]
async fn t509_strategy_events_since() {
    let ledger = open_test_ledger().await;

    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Load",
            strategy_id: Some("strategy_a"),
            old_hash: None,
            new_hash: Some("hash_a"),
            source_path: "config/strategies/a.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
        },
    )
    .await
    .unwrap();

    // Query from epoch — should return 1 event.
    let events = query::strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");

    assert!(!events.is_empty(), "should have at least 1 event");
    assert_eq!(
        events[0].strategy_id.as_ref().map(|s| s.0.as_str()),
        Some("strategy_a")
    );
}
