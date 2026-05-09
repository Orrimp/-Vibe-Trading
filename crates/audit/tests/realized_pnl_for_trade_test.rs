//! T1801 / R2.2 — `audit::query::realized_pnl_for_trade` smoke.
//!
//! 3 closed-trade fixture rows → 3 expected `Money<Usdt>` values; sum
//! equals `realized_pnl_since(period_start)` to the satoshi.

use audit::bootstrap;
use audit::query::{realized_pnl_for_trade, realized_pnl_since};
use audit::Ledger;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::Timestamp;
use uuid::Uuid;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_far_past() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(365))
}

async fn inject_pnl(ledger: &Ledger, txn_id: &str, pnl: Decimal, ts_str: &str) {
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(txn_id)
    .bind(ts_str)
    .bind("sell 1 BTCUSDT @ 1000")
    .bind::<Option<&str>>(Some("sma_crossover"))
    .execute(ledger.pool())
    .await
    .expect("insert transaction");

    let (debit, credit) = if pnl >= Decimal::ZERO {
        (Decimal::ZERO, pnl)
    } else {
        (pnl.abs(), Decimal::ZERO)
    };
    let entry_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry_id)
    .bind(txn_id)
    .bind("income:realized_pnl")
    .bind(debit.to_string())
    .bind(credit.to_string())
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert entry");
}

#[tokio::test]
async fn t1801_realized_pnl_for_trade_sums_match() {
    let ledger = open_ledger().await;

    // 3 closed trades.
    let txn_a = "11111111-1111-4111-8111-111111111111";
    let txn_b = "22222222-2222-4222-8222-222222222222";
    let txn_c = "33333333-3333-4333-8333-333333333333";

    inject_pnl(&ledger, txn_a, dec!(10.50), "2026-01-01T00:00:00.000000Z").await;
    inject_pnl(&ledger, txn_b, dec!(-3.25), "2026-01-02T00:00:00.000000Z").await;
    inject_pnl(&ledger, txn_c, dec!(0.75), "2026-01-03T00:00:00.000000Z").await;

    let a = realized_pnl_for_trade(&ledger, txn_a).await.expect("a");
    let b = realized_pnl_for_trade(&ledger, txn_b).await.expect("b");
    let c = realized_pnl_for_trade(&ledger, txn_c).await.expect("c");

    assert_eq!(a.amount(), dec!(10.50));
    assert_eq!(b.amount(), dec!(-3.25));
    assert_eq!(c.amount(), dec!(0.75));

    let total = a.amount() + b.amount() + c.amount();
    let since = realized_pnl_since(&ledger, ts_far_past())
        .await
        .expect("since")
        .amount();
    assert_eq!(
        total, since,
        "sum of per-trade PnL must equal realized_pnl_since"
    );
}

#[tokio::test]
async fn t1801_realized_pnl_for_trade_returns_zero_for_unknown_trade() {
    let ledger = open_ledger().await;
    // Forward-compat: unknown trade id → Money::from_decimal(dec!(0))
    let unknown = "deadbeef-dead-4ead-8ead-deadbeefdead";
    let res = realized_pnl_for_trade(&ledger, unknown)
        .await
        .expect("unknown trade ok");
    assert_eq!(res.amount(), Decimal::ZERO);
}
