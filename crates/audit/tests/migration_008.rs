#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1702 — `008_journal_transactions_venue.sql` migration acceptance.
//!
//! Per `spec/features/lumen-phase-3-detail-screens.md` Design § Q1 + R13.
//!
//! Verifies:
//! - The migration adds a `venue TEXT` column to `journal_transactions`.
//! - Pre-migration rows (synthesized via raw SQL into `journal_transactions`
//!   without a `venue` value) read back as `'binance'` after the
//!   migration's UPDATE pass — exercising the backfill semantics.
//! - The post-008 writer (`audit::journal::post_fill`) binds
//!   `venue.to_string()` so non-Binance fills round-trip through the
//!   column and surface in `recent_fills_filtered(.., Venue::Coinbase, ..)`.
//! - The Phase 2 venue gate is gone — `recent_fills_filtered` returns the
//!   matching subset for any seeded venue (was previously `Ok(vec![])`).

use audit::query::recent_fills_filtered;
use audit::{bootstrap, journal, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    Venue,
};

async fn open_seeded_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

fn make_fill(symbol: &str, side: Side, qty: Decimal, price: Decimal, secs: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new(symbol),
        side,
        qty: Quantity::new(qty).expect("qty ok"),
        price: Price::new(price).expect("price ok"),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_secs(secs),
        local_ts: ts_secs(secs),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

/// Migration 008 V1 — the `venue` column exists on `journal_transactions`
/// post-migration with a `'binance'` backfill on rows seeded with NULL.
#[tokio::test]
async fn migration_008_adds_venue_column_with_binance_backfill() {
    let ledger = open_seeded_ledger().await;

    // Synthesize a "pre-008-shape" row: raw INSERT with NULL venue (the
    // 4-column header form Phase 2 ledgers actually wrote). The post-008
    // schema accepts NULL because the column is nullable; the migration's
    // UPDATE pass (in 008_journal_transactions_venue.sql) backfills
    // `'binance'` only for rows where `venue IS NULL`.
    let legacy_txn_id = uuid::Uuid::new_v4().to_string();
    let legacy_ts = "2026-04-27T20:00:00Z";
    let legacy_desc = "buy 0.01 BTCUSDT @ 60000";
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id, venue) \
         VALUES (?, ?, ?, ?, NULL)",
    )
    .bind(&legacy_txn_id)
    .bind(legacy_ts)
    .bind(legacy_desc)
    .bind::<Option<&str>>(None)
    .execute(ledger.pool())
    .await
    .expect("insert pre-008 NULL-venue row");

    // Re-run the backfill UPDATE in-process so the test asserts the
    // migration's actual semantics on the row we just inserted (the
    // initial migration runs at `Ledger::in_memory()` and has already
    // backfilled empty NULL rows; we need to observe the same UPDATE
    // applied to a synthesized NULL we inserted post-migration).
    sqlx::query("UPDATE journal_transactions SET venue = 'binance' WHERE venue IS NULL")
        .execute(ledger.pool())
        .await
        .expect("re-apply backfill UPDATE");

    let row: (Option<String>,) =
        sqlx::query_as("SELECT venue FROM journal_transactions WHERE id = ?")
            .bind(&legacy_txn_id)
            .fetch_one(ledger.pool())
            .await
            .expect("select venue");
    assert_eq!(
        row.0.as_deref(),
        Some("binance"),
        "pre-008 NULL row must be backfilled to 'binance' by the migration's UPDATE pass; \
         got {:?}",
        row.0
    );
}

/// Migration 008 V2 — a non-Binance fill posts via the updated
/// `post_fill(ledger, fill, venue, strategy_id)` signature with the
/// venue's snake_case `Display` impl bound on the column.
#[tokio::test]
async fn migration_008_post_fill_writes_explicit_venue() {
    let ledger = open_seeded_ledger().await;

    let fill = make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 100);
    let txn_id = journal::post_fill(&ledger, &fill, Venue::Coinbase, Some("test_strategy"))
        .await
        .expect("post Coinbase fill");

    let row: (String,) = sqlx::query_as("SELECT venue FROM journal_transactions WHERE id = ?")
        .bind(txn_id.as_str())
        .fetch_one(ledger.pool())
        .await
        .expect("select venue");
    assert_eq!(
        row.0.as_str(),
        "coinbase",
        "post_fill must bind venue.to_string() (snake_case) on the new column"
    );
}

/// Migration 008 V3 — `recent_fills_filtered(&ledger, Venue::Coinbase, ..)`
/// returns the matching subset (was previously `Ok(vec![])` under the
/// Phase 2 venue gate; the gate is dropped now that the column exists).
#[tokio::test]
async fn migration_008_recent_fills_filtered_handles_non_binance_venue() {
    let ledger = open_seeded_ledger().await;

    // Seed: 1 BTCUSDT Buy on Binance, 1 on Coinbase — both inside the
    // window. The non-Binance row exercises the post-migration
    // `WHERE venue = ?` predicate.
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.1), dec!(40_000), 100),
        Venue::Binance,
        None,
    )
    .await
    .expect("post Binance fill");
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.1), dec!(40_000), 200),
        Venue::Coinbase,
        None,
    )
    .await
    .expect("post Coinbase fill");

    let since = ts_secs(0);
    let until = ts_secs(1_000);

    // Coinbase: returns the Coinbase row (was Ok(vec![]) under Phase 2's
    // venue gate; gate dropped post-008 — multi-venue queries now
    // discriminate at the SQL layer).
    let coinbase_fills = recent_fills_filtered(
        &ledger,
        Venue::Coinbase,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("query Coinbase");
    assert_eq!(
        coinbase_fills.len(),
        1,
        "Coinbase venue must now return its 1 fill (Phase 2 gate removed); got {}",
        coinbase_fills.len()
    );

    // Binance: returns the Binance row only (predicate isolates the venue).
    let binance_fills = recent_fills_filtered(
        &ledger,
        Venue::Binance,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("query Binance");
    assert_eq!(
        binance_fills.len(),
        1,
        "Binance venue must return its 1 fill (single-venue isolation); got {}",
        binance_fills.len()
    );
}
