//! T1402 — multi-venue strategy_events.venue column.
//!
//! Acceptance per `spec/tasks/v1-5b-multi-venue.md` T1402:
//!  - V1: `feed_reconnect(.., Venue::Binance, ..)` writes
//!    `strategy_events.venue = "binance"` (the new typed column added by
//!    migration `007_strategy_events_venue.sql`).
//!  - V2: pre-migration rows (rows that existed in `strategy_events`
//!    before migration `007` ran) have `venue = NULL`, and reads against
//!    those rows do not crash. Verifies the architect's Q11 / R8 design
//!    that the column is purely additive (NULLABLE, no default).

use audit::{Ledger, bootstrap, journal};
use sqlx::Row;
use sqlx::sqlite::SqlitePoolOptions;
use trading_core::Venue;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    ledger
}

/// V1 — A `feed_reconnect(.., Venue::Binance, ..)` call writes
/// `venue = "binance"` to the new typed column.
#[tokio::test]
async fn t1402_v1_writes_venue_column() {
    let ledger = open_ledger().await;

    let ts_str = "2030-06-01T00:00:00.111111Z";
    journal::feed_reconnect(&ledger, "BTCUSDT", Venue::Binance, Some(ts_str))
        .await
        .expect("feed_reconnect write");

    // Raw SELECT to confirm the new column carries the venue snake_case.
    let rows: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT venue, error_summary FROM strategy_events WHERE kind = 'FeedReconnect'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select venue");

    assert_eq!(rows.len(), 1, "expected exactly 1 FeedReconnect row");
    assert_eq!(
        rows[0].0.as_deref(),
        Some("binance"),
        "venue column must be the snake_case Venue::Display value"
    );
    // error_summary still carries the symbol literal verbatim — the
    // architect's Q11 explicitly rejected encoding venue inline in
    // error_summary; both fields are independent.
    assert_eq!(
        rows[0].1.as_deref(),
        Some("BTCUSDT"),
        "error_summary must keep the bare symbol literal (Q11 (a) over (b))"
    );

    // All three venues round-trip through Venue::Display.
    journal::feed_reconnect(&ledger, "BTCUSDC", Venue::Coinbase, None)
        .await
        .expect("coinbase write");
    journal::feed_reconnect(&ledger, "BTCUSDC", Venue::Kraken, None)
        .await
        .expect("kraken write");

    let venues: Vec<(String,)> = sqlx::query_as(
        "SELECT venue FROM strategy_events WHERE kind = 'FeedReconnect' \
         AND venue IS NOT NULL ORDER BY venue ASC",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select venues");
    let venue_list: Vec<&str> = venues.iter().map(|(s,)| s.as_str()).collect();
    assert_eq!(
        venue_list,
        vec!["binance", "coinbase", "kraken"],
        "all three venues stamp their snake_case display value"
    );
}

/// V2 — Pre-migration rows have `venue = NULL`.
///
/// Replays the migration history manually: applies migrations 001..006
/// (no `venue` column yet), inserts a `strategy_events` row directly via
/// raw SQL (mirroring what production rows look like before T1402
/// landed), then applies migration `007_strategy_events_venue.sql`.
/// The pre-existing row must have `venue = NULL` post-migration, and
/// reads must not crash. Confirms the architect's Q11 promise that
/// migration `007` is purely additive.
#[tokio::test]
async fn t1402_v2_pre_migration_rows_have_null_venue() {
    // Open an in-memory pool *without* using `Ledger::in_memory()` so we
    // can run migrations one slot at a time.
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("connect in-memory");

    // Apply 001..006 in order (the current pre-T1402 set).
    let pre_migrations: [(&str, &str); 6] = [
        (
            "001",
            include_str!("../migrations/001_chart_of_accounts.sql"),
        ),
        ("002", include_str!("../migrations/002_strategy_events.sql")),
        ("003", include_str!("../migrations/003_funding_rates.sql")),
        (
            "004",
            include_str!("../migrations/004_journal_transactions_strategy_id.sql"),
        ),
        (
            "005",
            include_str!("../migrations/005_uptime_intervals.sql"),
        ),
        (
            "006",
            include_str!("../migrations/006_per_symbol_position_accounts.sql"),
        ),
    ];
    for (slot, sql) in pre_migrations {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("apply migration {slot}: {e}"));
    }

    // Insert a pre-T1402 strategy_events row directly. Schema does *not*
    // yet have the `venue` column — the INSERT mirrors what
    // `journal::strategy_event` would have written before T1402.
    sqlx::query(
        "INSERT INTO strategy_events \
         (id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("00000000-0000-0000-0000-000000000001")
    .bind("2025-01-01T00:00:00.000001Z")
    .bind("FeedReconnect")
    .bind::<Option<&str>>(None)
    .bind::<Option<&str>>(None)
    .bind::<Option<&str>>(None)
    .bind("")
    .bind("system")
    .bind(Some("feed_reconnect"))
    .bind(Some("BTCUSDT"))
    .execute(&pool)
    .await
    .expect("insert pre-T1402 row");

    // Now apply migration 007 — purely additive ALTER TABLE.
    let m007 = include_str!("../migrations/007_strategy_events_venue.sql");
    sqlx::raw_sql(m007)
        .execute(&pool)
        .await
        .expect("apply migration 007");

    // The pre-existing row must have venue = NULL.
    let rows: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT id, venue FROM strategy_events WHERE id = ?")
            .bind("00000000-0000-0000-0000-000000000001")
            .fetch_all(&pool)
            .await
            .expect("select pre-migration row");
    assert_eq!(
        rows.len(),
        1,
        "the pre-migration row survives the ALTER TABLE"
    );
    assert_eq!(rows[0].0, "00000000-0000-0000-0000-000000000001");
    assert!(
        rows[0].1.is_none(),
        "pre-migration row must have venue = NULL (NULLABLE column, no default)"
    );

    // And `IS NULL` queries don't crash.
    let null_count: i64 = sqlx::query("SELECT COUNT(*) FROM strategy_events WHERE venue IS NULL")
        .fetch_one(&pool)
        .await
        .expect("count null venues")
        .get(0);
    assert_eq!(null_count, 1, "exactly one row, with venue NULL");

    // Index exists and is queryable (smoke — `WHERE venue = ?` plans).
    let by_venue: i64 = sqlx::query("SELECT COUNT(*) FROM strategy_events WHERE venue = ?")
        .bind("binance")
        .fetch_one(&pool)
        .await
        .expect("count by venue")
        .get(0);
    assert_eq!(by_venue, 0, "no rows have venue = 'binance' yet");
}
