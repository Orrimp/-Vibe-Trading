#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T2013 — `009_strategy_signals.sql` migration acceptance.
//!
//! Per `spec/chart-buy-sell-emphasis/feature.md` Design § Q1.
//!
//! Verifies:
//! - A fresh in-memory ledger applies the migration as part of
//!   `Ledger::open()` — the `strategy_signals` table and its three
//!   indexes exist post-open.
//! - Re-running the migration is a no-op (sqlx tracks the version; the
//!   `IF NOT EXISTS` guards re-application against a hand-touched DB).
//! - The pre-migration row count is exactly zero (the migration ships
//!   no seed data — operator opts in via `agent.toml [signal_log]
//!   enabled = true` before any row lands; V11c hard-asserts the
//!   gate-off path returns `Ok(vec![])`).

use audit::Ledger;

/// T2013 V1 — migration applies cleanly on an empty in-memory ledger.
#[tokio::test]
async fn migrations_apply_clean() {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");

    // The `strategy_signals` table exists post-migration (sqlite_master
    // is the SQLite catalog table).
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name = 'strategy_signals'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("query sqlite_master for strategy_signals table");
    assert_eq!(
        tables.len(),
        1,
        "strategy_signals table must exist after migration 009 applies; \
         found {} matching rows",
        tables.len()
    );

    // All three indexes from 009_strategy_signals.sql exist.
    let expected_indexes = [
        "strategy_signals_ts_idx",
        "strategy_signals_vs_idx",
        "strategy_signals_sid_idx",
    ];
    for idx in expected_indexes {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master \
             WHERE type = 'index' AND name = ?",
        )
        .bind(idx)
        .fetch_all(ledger.pool())
        .await
        .expect("query sqlite_master for index");
        assert_eq!(
            rows.len(),
            1,
            "index {idx} must exist after migration 009 applies"
        );
    }

    // No seed data — the table is empty on a fresh open. V11c asserts
    // the gate-off reader returns `Ok(vec![])` on this empty table.
    let row_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_signals")
        .fetch_one(ledger.pool())
        .await
        .expect("count strategy_signals rows");
    assert_eq!(
        row_count.0, 0,
        "fresh ledger must have zero strategy_signals rows; \
         migration is additive-schema-only, no data backfill"
    );
}

/// T2013 V2 — re-running the migration is idempotent.
///
/// Re-applies the same `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF
/// NOT EXISTS` statements directly against the same pool. The IF NOT
/// EXISTS clauses make the second apply a no-op; no error, no row
/// duplication.
#[tokio::test]
async fn migration_009_is_idempotent() {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");

    // Re-apply the migration's DDL statements in-process. The
    // IF NOT EXISTS clauses must make this a no-op.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS strategy_signals (\
            id                 TEXT PRIMARY KEY,\
            ts                 TEXT NOT NULL,\
            strategy_id        TEXT NOT NULL,\
            venue              TEXT NOT NULL,\
            symbol             TEXT NOT NULL,\
            side               TEXT NOT NULL,\
            intended_qty_str   TEXT NOT NULL,\
            intended_price_str TEXT,\
            was_clamped        INTEGER NOT NULL DEFAULT 0,\
            clamp_reason       TEXT\
         )",
    )
    .execute(ledger.pool())
    .await
    .expect("re-apply CREATE TABLE IF NOT EXISTS");

    sqlx::query("CREATE INDEX IF NOT EXISTS strategy_signals_ts_idx ON strategy_signals(ts)")
        .execute(ledger.pool())
        .await
        .expect("re-apply ts index");
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS strategy_signals_vs_idx ON strategy_signals(venue, symbol, ts)",
    )
    .execute(ledger.pool())
    .await
    .expect("re-apply vs index");
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS strategy_signals_sid_idx ON strategy_signals(strategy_id, ts)",
    )
    .execute(ledger.pool())
    .await
    .expect("re-apply sid index");

    // Row count is still zero.
    let row_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_signals")
        .fetch_one(ledger.pool())
        .await
        .expect("count after re-apply");
    assert_eq!(row_count.0, 0, "re-apply must not seed rows");
}
