//! T804 — `audit::query::ledger_inception_ts` integration test.
//!
//! Acceptance: returns the earliest `ts` from a fixture with three
//! transactions.

use audit::query::ledger_inception_ts;
use audit::{bootstrap, Ledger};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use trading_core::Timestamp;
use uuid::Uuid;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    ledger
}

async fn insert_txn(ledger: &Ledger, ts_str: &str) {
    let txn_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind(ts_str)
        .bind("test")
        .execute(ledger.pool())
        .await
        .expect("insert txn");
}

#[tokio::test]
async fn t804_inception_ts_returns_earliest_of_three() {
    let ledger = open_ledger().await;

    // Insert three transactions out of order.  ledger_inception_ts must
    // return the earliest ts (string-min on ISO-8601-Z is the same as
    // chronological-min for the values we use).
    insert_txn(&ledger, "2030-06-01T00:00:00Z").await;
    insert_txn(&ledger, "2030-01-01T00:00:00Z").await; // earliest
    insert_txn(&ledger, "2030-12-31T00:00:00Z").await;

    let inception = ledger_inception_ts(&ledger)
        .await
        .expect("ledger_inception_ts");

    let expected =
        Timestamp::new(OffsetDateTime::parse("2030-01-01T00:00:00Z", &Rfc3339).expect("parse"));
    assert_eq!(inception, expected, "inception must be the earliest ts");
}

#[tokio::test]
async fn t804_inception_ts_errors_on_empty_ledger() {
    let ledger = open_ledger().await;
    let err = ledger_inception_ts(&ledger)
        .await
        .expect_err("empty ledger should error");
    let msg = err.to_string();
    assert!(
        msg.contains("ledger_inception_ts: no transactions"),
        "expected no-transactions error, got: {msg}"
    );
}
