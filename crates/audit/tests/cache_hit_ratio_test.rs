#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1910 — `audit::query::cache_hit_ratio_since` acceptance.
//!
//! Acceptance criterion from `spec/v1/v2-llm-strategy/tasks.md`:
//! > fixture ledger with 3 LLM events (`tokens_in=1000,
//! > tokens_cached_in=500` each) returns ratio `0.5`; empty fixture
//! > returns `0.0`.
//!
//! Because T1917's extended `post_cost` signature (which writes
//! `tokens_in` / `tokens_cached_in` onto `journal_transactions.metadata`)
//! lands in M5 — pass 4+ — this test seeds the fixture rows directly via
//! raw `sqlx` INSERTs against the schema the reader actually queries.
//! Forward-compat: once T1917 lands, the same test will keep passing
//! with no change because the query reads the same JSON shape.

use audit::{Ledger, bootstrap, query};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::Timestamp;
use uuid::Uuid;

/// Open a fresh in-memory ledger with the chart of accounts bootstrapped.
async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

/// Seed one LLM-cost transaction with the given token-meta payload.
///
/// Writes the same row shape `post_cost` will produce post-T1917:
/// one `journal_transactions` row with token JSON in `metadata`, plus
/// a balanced pair of entries on `expense:llm:<tier>` and
/// `liabilities:llm_accrued`.
async fn seed_llm_event(
    ledger: &Ledger,
    tier: &str,
    usd: rust_decimal::Decimal,
    tokens_in: u64,
    tokens_cached_in: u64,
    ts_secs: i64,
) {
    let txn_id = Uuid::new_v4().to_string();
    let ts = (OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(ts_secs))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let metadata = serde_json::json!({
        "tokens_in": tokens_in,
        "tokens_cached_in": tokens_cached_in,
    })
    .to_string();
    let expense = format!("expense:llm:{tier}");

    // Make sure both accounts exist (the chart_of_accounts bootstrap may
    // not seed `expense:llm:*` and `liabilities:llm_accrued` yet — the
    // post_cost call site does this; we mirror it here).
    for (acct, kind) in [
        (expense.as_str(), "expense"),
        ("liabilities:llm_accrued", "liability"),
    ] {
        sqlx::query("INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES (?, ?, 'USDT')")
            .bind(acct)
            .bind(kind)
            .execute(ledger.pool())
            .await
            .expect("seed account");
    }

    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, metadata) VALUES (?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(&ts)
    .bind(format!("llm_cost:{tier}"))
    .bind(&metadata)
    .execute(ledger.pool())
    .await
    .expect("insert journal transaction");

    // Balanced dr/cr pair (same shape `post_cost` writes).
    for (acct, dr, cr) in [
        (expense.as_str(), usd, dec!(0)),
        ("liabilities:llm_accrued", dec!(0), usd),
    ] {
        sqlx::query(
            "INSERT INTO journal_entries (id, transaction_id, account_id, debit_amount, credit_amount, ts, memo) \
             VALUES (?, ?, ?, ?, ?, ?, '')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&txn_id)
        .bind(acct)
        .bind(dr.to_string())
        .bind(cr.to_string())
        .bind(&ts)
        .execute(ledger.pool())
        .await
        .expect("insert journal entry");
    }
}

/// T1910 acceptance: 3 events × (tokens_in=1000, tokens_cached_in=500) → 0.5.
#[tokio::test]
async fn t1910_three_events_returns_half() {
    let ledger = open_ledger().await;

    for i in 0..3 {
        seed_llm_event(
            &ledger,
            "deep_think",
            dec!(0.10),
            1_000,
            500,
            1_000 + i as i64,
        )
        .await;
    }

    let since = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
    let ratio = query::cache_hit_ratio_since(&ledger, since)
        .await
        .expect("cache_hit_ratio_since ok");
    assert_eq!(ratio, dec!(0.5));
}

/// T1910 acceptance: empty fixture → 0.0.
#[tokio::test]
async fn t1910_empty_fixture_returns_zero() {
    let ledger = open_ledger().await;
    let since = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
    let ratio = query::cache_hit_ratio_since(&ledger, since)
        .await
        .expect("cache_hit_ratio_since ok");
    assert_eq!(ratio, dec!(0));
}

/// `since` bound is honored — events before `since` are excluded.
#[tokio::test]
async fn t1910_since_window_excludes_older_events() {
    let ledger = open_ledger().await;

    // Old: 100% cache hit (would skew the ratio to 1.0 if included).
    seed_llm_event(&ledger, "deep_think", dec!(0.05), 1_000, 1_000, 100).await;
    // Recent: 0% cache hit.
    seed_llm_event(&ledger, "deep_think", dec!(0.10), 1_000, 0, 10_000).await;

    let since = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(5_000));
    let ratio = query::cache_hit_ratio_since(&ledger, since)
        .await
        .expect("cache_hit_ratio_since ok");
    assert_eq!(
        ratio,
        dec!(0),
        "older row outside the window must be excluded"
    );
}

/// Malformed metadata JSON is skipped (defensive — read-only query must
/// not block on a stray bad row).
#[tokio::test]
async fn t1910_malformed_metadata_skipped() {
    let ledger = open_ledger().await;

    // Seed one well-formed event so the result is non-zero.
    seed_llm_event(&ledger, "deep_think", dec!(0.10), 1_000, 500, 1_000).await;

    // Seed one row with malformed metadata directly.
    let bad_txn_id = Uuid::new_v4().to_string();
    let ts = (OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(2_000))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    sqlx::query("INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES ('expense:llm:deep_think', 'expense', 'USDT')")
        .execute(ledger.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, metadata) VALUES (?, ?, ?, ?)",
    )
    .bind(&bad_txn_id)
    .bind(&ts)
    .bind("llm_cost:deep_think")
    .bind("{not valid json")
    .execute(ledger.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO journal_entries (id, transaction_id, account_id, debit_amount, credit_amount, ts, memo) \
         VALUES (?, ?, 'expense:llm:deep_think', '0.10', '0', ?, '')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&bad_txn_id)
    .bind(&ts)
    .execute(ledger.pool())
    .await
    .unwrap();

    let since = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
    let ratio = query::cache_hit_ratio_since(&ledger, since)
        .await
        .expect("malformed row should not error the query");
    // Bad row contributes 0/0; the well-formed row contributes 500/1000.
    assert_eq!(ratio, dec!(0.5));
}
