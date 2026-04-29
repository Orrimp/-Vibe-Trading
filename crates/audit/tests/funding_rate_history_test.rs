//! Integration tests for `audit::query::funding_rate_history` (T613 — v1 Q2).
//!
//! T613 acceptance:
//! - `funding_rate_history(symbol, since, until)` returns rows in chronological order.
//! - Rows for other symbols are excluded.
//! - Rows outside the `[since, until]` window are excluded.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use audit::{journal, ledger::Ledger, query};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{FundingObs, Symbol, Timestamp};

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn open_test_ledger() -> Ledger {
    Ledger::in_memory().await.expect("open in-memory ledger")
}

fn ts_at(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).unwrap())
}

fn make_obs(symbol: &str, rate: rust_decimal::Decimal, funding_unix: i64) -> FundingObs {
    FundingObs {
        symbol: Symbol::new(symbol),
        funding_rate: rate,
        funding_ts: ts_at(funding_unix),
        next_funding_ts: ts_at(funding_unix + 28800),
        poll_ts: ts_at(funding_unix + 1),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// T613 — `funding_rates` table exists after migration.
#[tokio::test]
async fn t613_funding_rates_table_exists() {
    let ledger = open_test_ledger().await;
    let rows: Vec<(String,)> = sqlx::query_as::<_, (String,)>(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='funding_rates'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("query sqlite_master");
    assert_eq!(
        rows.len(),
        1,
        "funding_rates table should exist after migration 003"
    );
}

/// T613 — Happy path: insert 3 rows for BTCUSDT, query by symbol and window.
#[tokio::test]
async fn t613_funding_rate_history_returns_rows_in_order() {
    let ledger = open_test_ledger().await;

    // Insert 3 observations for BTCUSDT at t=1000, t=2000, t=3000.
    let obs1 = make_obs("BTCUSDT", dec!(0.0001), 1000);
    let obs2 = make_obs("BTCUSDT", dec!(0.0002), 2000);
    let obs3 = make_obs("BTCUSDT", dec!(-0.0001), 3000);

    journal::insert_funding_obs(&ledger, &obs1)
        .await
        .expect("insert 1");
    journal::insert_funding_obs(&ledger, &obs2)
        .await
        .expect("insert 2");
    journal::insert_funding_obs(&ledger, &obs3)
        .await
        .expect("insert 3");

    // Query the full window.
    let history = query::funding_rate_history(
        &ledger,
        Symbol::new("BTCUSDT"),
        ts_at(0),
        ts_at(999_999_999),
    )
    .await
    .expect("query history");

    assert_eq!(history.len(), 3, "expected 3 rows");

    // Verify chronological order.
    assert_eq!(history[0].funding_rate, dec!(0.0001));
    assert_eq!(history[1].funding_rate, dec!(0.0002));
    assert_eq!(history[2].funding_rate, dec!(-0.0001));

    // Verify symbol is correct.
    for row in &history {
        assert_eq!(row.symbol, Symbol::new("BTCUSDT"));
    }
}

/// T613 — Rows for other symbols are excluded.
#[tokio::test]
async fn t613_funding_rate_history_excludes_other_symbols() {
    let ledger = open_test_ledger().await;

    journal::insert_funding_obs(&ledger, &make_obs("BTCUSDT", dec!(0.0001), 1000))
        .await
        .expect("insert BTC");
    journal::insert_funding_obs(&ledger, &make_obs("ETHUSDT", dec!(0.0002), 1000))
        .await
        .expect("insert ETH");

    let btc_history = query::funding_rate_history(
        &ledger,
        Symbol::new("BTCUSDT"),
        ts_at(0),
        ts_at(999_999_999),
    )
    .await
    .expect("query BTC");

    assert_eq!(btc_history.len(), 1);
    assert_eq!(btc_history[0].symbol, Symbol::new("BTCUSDT"));

    let eth_history = query::funding_rate_history(
        &ledger,
        Symbol::new("ETHUSDT"),
        ts_at(0),
        ts_at(999_999_999),
    )
    .await
    .expect("query ETH");

    assert_eq!(eth_history.len(), 1);
    assert_eq!(eth_history[0].symbol, Symbol::new("ETHUSDT"));
}

/// T613 — Window filtering: only rows within `[since, until]` are returned.
#[tokio::test]
async fn t613_funding_rate_history_window_filter() {
    let ledger = open_test_ledger().await;

    // Insert 3 rows at t=1000, t=2000, t=3000.
    journal::insert_funding_obs(&ledger, &make_obs("SOLUSDT", dec!(0.0001), 1000))
        .await
        .expect("insert 1");
    journal::insert_funding_obs(&ledger, &make_obs("SOLUSDT", dec!(0.0002), 2000))
        .await
        .expect("insert 2");
    journal::insert_funding_obs(&ledger, &make_obs("SOLUSDT", dec!(0.0003), 3000))
        .await
        .expect("insert 3");

    // Query only the middle row (since=1500, until=2500).
    let window =
        query::funding_rate_history(&ledger, Symbol::new("SOLUSDT"), ts_at(1500), ts_at(2500))
            .await
            .expect("query window");

    assert_eq!(window.len(), 1, "expected only row at t=2000");
    assert_eq!(window[0].funding_rate, dec!(0.0002));
}

/// T613 — Empty result when no rows match.
#[tokio::test]
async fn t613_funding_rate_history_empty_on_no_match() {
    let ledger = open_test_ledger().await;

    let history = query::funding_rate_history(
        &ledger,
        Symbol::new("BTCUSDT"),
        ts_at(0),
        ts_at(999_999_999),
    )
    .await
    .expect("empty query");

    assert!(history.is_empty(), "expected empty Vec when no rows");
}

/// T613 — Ledger balance invariant: inserting funding_obs rows does NOT affect
/// the double-entry balance (funding_rates is observation-only).
#[tokio::test]
async fn t613_insert_funding_obs_does_not_affect_ledger_balance() {
    let ledger = open_test_ledger().await;

    // Insert a funding observation.
    journal::insert_funding_obs(&ledger, &make_obs("BTCUSDT", dec!(0.0001), 1000))
        .await
        .expect("insert");

    // The global debit/credit sums should be zero (no journal_entries written).
    let (debits, credits) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("global sum");

    assert_eq!(debits, dec!(0), "no debits from funding_obs");
    assert_eq!(credits, dec!(0), "no credits from funding_obs");
}
