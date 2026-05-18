//! Phase 5 T1902 — `audit::journal::risk_veto_overridden` integration test.
//!
//! Exercises the audit writer end-to-end against an in-memory ledger
//! and locks the `strategy_events` row format via an insta snapshot
//! baseline (`strategy_events__risk_veto_overridden_row.snap`). Per
//! Q10 ratification (unit + integration + audit-row snapshot baseline).
//!
//! The snapshot omits volatile fields (`id` UUID, `ts`) and locks the
//! deterministic columns (`kind`, `strategy_id`, `error_code`,
//! `error_summary`, `venue`).

use audit::query::strategy_events_since;
use audit::{Ledger, bootstrap, journal};
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
async fn risk_veto_overridden_writes_memo_and_strategy_event() {
    let ledger = open_ledger().await;
    journal::risk_veto_overridden(&ledger, "veto-1", "alpha", "daily_loss_cap", "operator")
        .await
        .expect("risk_veto_overridden write");

    let events = strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");
    assert_eq!(events.len(), 1);
    let ev = &events[0];
    assert_eq!(ev.kind, StrategyEventKind::RiskVetoOverridden);
    assert_eq!(ev.strategy_id.as_ref().map(|s| s.0.as_str()), Some("alpha"),);
    assert_eq!(ev.error_summary.as_deref(), Some("daily_loss_cap"));
    assert_eq!(ev.error_code.as_deref(), Some("risk_veto_overridden"));
    assert_eq!(ev.operator.as_str(), "operator");
}

#[tokio::test]
async fn risk_veto_overridden_row_format_snapshot() {
    let ledger = open_ledger().await;
    journal::risk_veto_overridden(&ledger, "veto-1", "alpha", "daily_loss_cap", "operator")
        .await
        .expect("risk_veto_overridden write");

    let rows: Vec<StrategyEventRow> = sqlx::query_as(
        "SELECT kind, strategy_id, error_code, error_summary, operator, venue \
             FROM strategy_events WHERE kind = 'RiskVetoOverridden'",
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
    insta::assert_snapshot!("strategy_events__risk_veto_overridden_row", summary);
}
