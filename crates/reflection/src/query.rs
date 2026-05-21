//! Phase F (ui-rethink-phase-f-memory-models-assistant T-D-N8) — read-path
//! for the Memory screen.
//!
//! Lives as a **sibling of `store/`** per architect Q8=(b) refinement:
//! the SQL query bypasses the `ReflectionStore` trait (whose surface stays
//! at 3 methods: `upsert / top_k / count`) while still living inside the
//! reflection crate where the DB schema is canonical.
//!
//! The UI crate never calls this module directly — `cockpit_live` owns the
//! async/sync boundary, opens the pool, calls `list_recent_lesson_cards`,
//! and forwards the result via `Message::MemoryHydrate`.
//!
//! **Fail-soft contract:** returns `Ok(vec![])` if the table is empty
//! (cold-empty boot path); returns `Err(...)` only on DB connection /
//! encoding errors. The cockpit_live caller logs via `tracing::warn!` and
//! surfaces the R1.4 empty-state placeholder to the operator.

use std::path::Path;

use crate::store::ReflectionStoreError;
use crate::store::sqlite::{PersistedRow, decode_row};
use crate::types::LessonCard;

/// Phase F T-D-N10 — open a one-shot pool against `db_path` and list the N
/// most-recent lesson cards.
///
/// Convenience wrapper for callers (e.g. `cockpit_live`) that do not own
/// a `SqlitePool` — they pass the filesystem path and this function opens
/// a short-lived single-connection pool, queries, then returns.
///
/// **Fail-soft contract:** if `db_path` does not exist or cannot be opened,
/// returns `Ok(vec![])` — not an error — so the Memory screen renders the
/// R1.4 empty-state placeholder rather than crashing. The caller should log
/// the path-not-found case via `tracing::warn!` at the call site.
///
/// # Errors
///
/// Returns [`ReflectionStoreError::Database`] only on I/O errors that are
/// NOT a missing-file condition (e.g. permission denied, corrupt file).
/// Returns [`ReflectionStoreError::Encoding`] on row decode failure.
pub async fn open_and_list_recent(
    db_path: &Path,
    limit: usize,
) -> Result<Vec<LessonCard>, ReflectionStoreError> {
    use sqlx::ConnectOptions;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    // Return empty-state immediately when the file doesn't exist yet.
    // This is the dominant first-open path (H1 enumeration — reflection.db
    // ABSENT on a fresh workstation).
    if !db_path.exists() {
        tracing::debug!(
            path = %db_path.display(),
            "reflection DB not on disk yet — cold-empty boot path"
        );
        return Ok(vec![]);
    }

    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false)
        .disable_statement_logging();

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_with(opts)
        .await
        .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

    list_recent_lesson_cards(&pool, limit).await
}

/// Phase F — list the N most-recent lesson cards ordered by `closed_at DESC`.
///
/// Mirrors the schema declared at
/// `crates/reflection/migrations/001_lesson_cards.sql:8-24` and the
/// row materialisation at `store/sqlite.rs` (`decode_row`).
///
/// **Q8=(b) refinement:** SQL lives in a sibling module of `store/`,
/// not on the `ReflectionStore` trait (per operator-decide + architect
/// refinement). The trait surface stays at 3 methods.
///
/// # Errors
///
/// Returns [`ReflectionStoreError::Database`] on connection failure or
/// [`ReflectionStoreError::Encoding`] on row decode failure.
pub async fn list_recent_lesson_cards(
    pool: &sqlx::SqlitePool,
    limit: usize,
) -> Result<Vec<LessonCard>, ReflectionStoreError> {
    let rows: Vec<PersistedRow> = sqlx::query_as::<_, PersistedRow>(
        "SELECT card_id, closed_at, symbol_or_pair, strategy_id, signed_pnl_usdt, \
                opening_capital_usdt, holding_period_bars, entry_regime, exit_regime, \
                outcome_class, embedding_blob, note \
         FROM lesson_cards \
         ORDER BY closed_at DESC \
         LIMIT ?",
    )
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(pool)
    .await
    .map_err(|e| ReflectionStoreError::Database(e.to_string()))?;

    rows.into_iter().map(decode_row).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    /// Build a fresh in-memory SQLite pool with migrations + N test cards.
    ///
    /// Inserts cards with `closed_at` timestamps at `base_secs + i*100` for
    /// `i = 0..count`. Card ids are `"card_<i>"`. All other fields use
    /// valid fixed values. Returns the raw `SqlitePool`.
    async fn build_test_pool(cards: &[(&str, i64)]) -> sqlx::SqlitePool {
        use sqlx::ConnectOptions;
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

        let opts = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true)
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect_with(opts)
            .await
            .expect("pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrate");

        for (card_id, secs) in cards {
            let ts = OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(*secs);
            let closed_at_str = ts
                .format(&time::format_description::well_known::Rfc3339)
                .expect("format");
            // Use a fixed 32-component embedding blob of all zeros.
            let embedding = "0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0";
            sqlx::query(
                "INSERT INTO lesson_cards \
                 (card_id, closed_at, symbol_or_pair, strategy_id, \
                  signed_pnl_usdt, opening_capital_usdt, holding_period_bars, \
                  entry_regime, exit_regime, outcome_class, embedding_blob, note) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(*card_id)
            .bind(&closed_at_str)
            .bind("BTCUSDT")
            .bind("v1.test")
            .bind("10")
            .bind("1000")
            .bind(5_i64)
            .bind("bull")
            .bind("chop")
            .bind("Win")
            .bind(embedding)
            .bind(Option::<&str>::None)
            .execute(&pool)
            .await
            .expect("insert card");
        }
        pool
    }

    /// H4 falsification — `list_recent_lesson_cards` returns the N most-recent
    /// rows ordered by `closed_at DESC`.
    ///
    /// Populates an in-memory sqlite with 5 fixture rows at known timestamps;
    /// calls `list_recent_lesson_cards(pool, 3)`;
    /// asserts the returned 3 rows are the 3 most-recent by `closed_at DESC`.
    #[tokio::test]
    async fn list_recent_lesson_cards_returns_n_recent() {
        // Timestamps (seconds since UNIX_EPOCH): a=100, b=200, c=300, d=400, e=500.
        let cards: &[(&str, i64)] = &[
            ("card_a", 100),
            ("card_b", 200),
            ("card_c", 300),
            ("card_d", 400),
            ("card_e", 500),
        ];
        let pool = build_test_pool(cards).await;

        let result = list_recent_lesson_cards(&pool, 3).await.expect("query ok");
        assert_eq!(result.len(), 3, "should return exactly 3 cards (limit=3)");

        // Verify DESC order: card_e (500s) first, then card_d (400s), card_c (300s).
        let ids: Vec<&str> = result.iter().map(|c| c.card_id.as_str()).collect();
        assert_eq!(
            ids[0], "card_e",
            "first card should be most-recent (card_e @ 500s)"
        );
        assert_eq!(ids[1], "card_d", "second should be card_d @ 400s");
        assert_eq!(ids[2], "card_c", "third should be card_c @ 300s");
    }
}
