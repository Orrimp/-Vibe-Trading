//! Phase 5 T1902 — `audit::journal::strategy_paused` integration test.
//!
//! Exercises the audit writer end-to-end against an in-memory ledger
//! and locks the `strategy_events` row format via an insta snapshot
//! baseline (`strategy_events__strategy_paused_row.snap`). Per Q10
//! ratification (unit + integration + audit-row snapshot baseline).
//!
//! The snapshot omits volatile fields (`id` UUID, `ts`) and locks the
//! deterministic columns (`kind`, `strategy_id`, `error_code`,
//! `error_summary`, `venue`).

use audit::query::strategy_events_since;
use audit::{bootstrap, journal, Ledger};
use time::OffsetDateTime;
use trading_core::{StrategyEventKind, Timestamp};

/// `(kind, strategy_id, error_code, error_summary, operator, venue)`
/// projection used by the snapshot baseline. Aliased to keep
/// `clippy::type_complexity` quiet for the test fixture.
type StrategyEventRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

#[tokio::test]
async fn strategy_paused_writes_memo_and_strategy_event() {
    let ledger = open_ledger().await;
    journal::strategy_paused(&ledger, "alpha", true, "operator")
        .await
        .expect("strategy_paused write");

    let events = strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");
    assert_eq!(events.len(), 1, "exactly one StrategyPaused row");
    let ev = &events[0];
    assert_eq!(ev.kind, StrategyEventKind::StrategyPaused);
    assert_eq!(ev.strategy_id.as_ref().map(|s| s.0.as_str()), Some("alpha"),);
    assert_eq!(ev.error_summary.as_deref(), Some("paused"));
    assert_eq!(ev.error_code.as_deref(), Some("strategy_paused"));
    assert_eq!(ev.operator.as_str(), "operator");
}

#[tokio::test]
async fn strategy_paused_row_format_snapshot() {
    let ledger = open_ledger().await;
    journal::strategy_paused(&ledger, "alpha", true, "operator")
        .await
        .expect("strategy_paused write");

    let rows: Vec<StrategyEventRow> = sqlx::query_as(
        "SELECT kind, strategy_id, error_code, error_summary, operator, venue \
             FROM strategy_events WHERE kind = 'StrategyPaused'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select row");
    assert_eq!(rows.len(), 1);
    let r = &rows[0];

    let summary = format!(
        "kind: {}\nstrategy_id: {:?}\nerror_code: {:?}\nerror_summary: {:?}\noperator: {}\nvenue: {:?}\n",
        r.0, r.1, r.2, r.3, r.4, r.5,
    );
    insta::assert_snapshot!("strategy_events__strategy_paused_row", summary);
}
