//! T803 — `audit::query::pnl_by_strategy` integration tests.
//!
//! Per the architect's task spec:
//!  - 12 fills across 4 strategies (3 trades each) with deliberate mix of
//!    wins/losses; assert
//!    (a) 4 rows returned in `realized DESC` order,
//!    (b) `Σ rows.realized == realized_pnl_since(period_start)` to the
//!    satoshi,
//!    (c) win-rate computed correctly,
//!    (d) one extra fill posted with `strategy_id = None` produces a 5th
//!    `(unattributed)` row,
//!    (e) running with `until = far_past` produces an empty `Vec`.

use audit::query::{pnl_by_strategy, realized_pnl_since, StrategyPnl};
use audit::{bootstrap, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{StrategyId, Timestamp};
use uuid::Uuid;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

fn ts_far_future() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(36500))
}

fn ts_far_past() -> Timestamp {
    // 1971-01-01 — earlier than any injected row.
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(365))
}

/// Inject a single closed-trade `income:realized_pnl` row tagged with the
/// given strategy id.  Bypasses `post_fill`'s simplified zero-realized cost
/// basis so the test can verify aggregation across realistic P&L values.
async fn inject_strategy_pnl(
    ledger: &Ledger,
    strategy_id: Option<&str>,
    pnl: Decimal,
    ts_str: &str,
) {
    let txn_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(ts_str)
    .bind("sell 1 BTCUSDT @ 1000")
    .bind(strategy_id)
    .execute(ledger.pool())
    .await
    .expect("insert transaction");

    // income:realized_pnl row — credit for profit, debit for loss.
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
    .bind(&txn_id)
    .bind("income:realized_pnl")
    .bind(debit.to_string())
    .bind(credit.to_string())
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert journal entry");

    // Balancing entry against assets:cash:USDT (mirror sign).
    let bal_id = Uuid::new_v4().to_string();
    let (bal_debit, bal_credit) = if pnl >= Decimal::ZERO {
        (pnl, Decimal::ZERO)
    } else {
        (Decimal::ZERO, pnl.abs())
    };
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&bal_id)
    .bind(&txn_id)
    .bind("assets:cash:USDT")
    .bind(bal_debit.to_string())
    .bind(bal_credit.to_string())
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert balancing entry");
}

// ── (a) + (b) + (c): 12 fills × 4 strategies, sum + ordering + win-rate ─────

#[tokio::test]
async fn t803_12_fills_4_strategies_sorted_with_correct_stats() {
    let ledger = open_ledger().await;

    // 4 strategies, 3 closed trades each (12 total).
    // Strategy A — strongest realized: 3 wins (200, 150, 100) → 450 realized,
    //                                    win-rate 3/3 = 100%.
    // Strategy B — mixed: 100, -50, 100 → 150 realized, 2/3 wins.
    // Strategy C — single-loss-dominated: 50, -200, 30 → -120 realized, 2/3.
    // Strategy D — moderate: 80, 70, -50 → 100 realized, 2/3 wins.
    //
    // Expected DESC ordering: A (450), B (150), D (100), C (-120).

    let trades = vec![
        ("strat_a", dec!(200)),
        ("strat_a", dec!(150)),
        ("strat_a", dec!(100)),
        ("strat_b", dec!(100)),
        ("strat_b", dec!(-50)),
        ("strat_b", dec!(100)),
        ("strat_c", dec!(50)),
        ("strat_c", dec!(-200)),
        ("strat_c", dec!(30)),
        ("strat_d", dec!(80)),
        ("strat_d", dec!(70)),
        ("strat_d", dec!(-50)),
    ];

    let ts_str = "2030-01-01T00:00:00Z";
    for (sid, pnl) in &trades {
        inject_strategy_pnl(&ledger, Some(*sid), *pnl, ts_str).await;
    }

    let rows = pnl_by_strategy(&ledger, ts_epoch(), ts_far_future())
        .await
        .expect("pnl_by_strategy");

    // (a) 4 rows in realized DESC order.
    assert_eq!(rows.len(), 4, "expected 4 strategy rows");
    assert_eq!(
        rows[0].strategy_id,
        StrategyId::new("strat_a"),
        "DESC #1 must be strat_a"
    );
    assert_eq!(rows[1].strategy_id, StrategyId::new("strat_b"));
    assert_eq!(rows[2].strategy_id, StrategyId::new("strat_d"));
    assert_eq!(rows[3].strategy_id, StrategyId::new("strat_c"));

    // (a) realized amounts.
    assert_eq!(rows[0].realized.amount(), dec!(450));
    assert_eq!(rows[1].realized.amount(), dec!(150));
    assert_eq!(rows[2].realized.amount(), dec!(100));
    assert_eq!(rows[3].realized.amount(), dec!(-120));

    // (c) closed/winning trade counts.
    assert_eq!(rows[0].closed_trade_count, 3, "strat_a closed");
    assert_eq!(rows[0].winning_trade_count, 3, "strat_a wins");
    assert_eq!(rows[1].closed_trade_count, 3, "strat_b closed");
    assert_eq!(rows[1].winning_trade_count, 2, "strat_b wins");
    assert_eq!(rows[2].closed_trade_count, 3, "strat_d closed");
    assert_eq!(rows[2].winning_trade_count, 2, "strat_d wins");
    assert_eq!(rows[3].closed_trade_count, 3, "strat_c closed");
    assert_eq!(rows[3].winning_trade_count, 2, "strat_c wins");

    // avg_trade_realized = realized / closed_trade_count
    assert_eq!(rows[0].avg_trade_realized.amount(), dec!(150)); // 450 / 3
    assert_eq!(rows[1].avg_trade_realized.amount(), dec!(50)); // 150 / 3
    assert_eq!(rows[2].avg_trade_realized.amount(), dec!(100) / dec!(3));
    assert_eq!(rows[3].avg_trade_realized.amount(), dec!(-40)); // -120 / 3

    // (b) Sum invariant: Σ rows.realized == realized_pnl_since(period_start)
    let sum: Decimal = rows.iter().map(|r| r.realized.amount()).sum();
    let scalar = realized_pnl_since(&ledger, ts_epoch())
        .await
        .expect("realized_pnl_since");
    assert_eq!(
        sum,
        scalar.amount(),
        "Σ pnl_by_strategy must equal realized_pnl_since to the satoshi"
    );
}

// ── (d): one extra fill with strategy_id = None → 5th `(unattributed)` row ──

#[tokio::test]
async fn t803_unattributed_bucket_when_strategy_id_null() {
    let ledger = open_ledger().await;

    let ts_str = "2030-01-01T00:00:00Z";
    inject_strategy_pnl(&ledger, Some("strat_a"), dec!(100), ts_str).await;
    inject_strategy_pnl(&ledger, Some("strat_b"), dec!(50), ts_str).await;
    // Pre-migration row simulation — strategy_id is NULL.
    inject_strategy_pnl(&ledger, None, dec!(75), ts_str).await;

    let rows = pnl_by_strategy(&ledger, ts_epoch(), ts_far_future())
        .await
        .expect("pnl_by_strategy");

    assert_eq!(rows.len(), 3, "expected 3 strategy buckets");
    let unattributed = rows
        .iter()
        .find(|r| r.strategy_id == StrategyId::new("(unattributed)"))
        .expect("(unattributed) row must be present");
    assert_eq!(unattributed.realized.amount(), dec!(75));
    assert_eq!(unattributed.closed_trade_count, 1);
    assert_eq!(unattributed.winning_trade_count, 1);
}

// ── (e): until = far_past produces an empty Vec ──────────────────────────────

#[tokio::test]
async fn t803_empty_when_window_excludes_all_rows() {
    let ledger = open_ledger().await;

    // Inject a row in the future — but query with a far-past `until`.
    inject_strategy_pnl(&ledger, Some("strat_a"), dec!(100), "2030-01-01T00:00:00Z").await;

    let rows = pnl_by_strategy(&ledger, ts_epoch(), ts_far_past())
        .await
        .expect("pnl_by_strategy");

    assert!(rows.is_empty(), "no rows in the [epoch, 1971-01-01] window");
}

// ── tie-break: when two strategies have identical realized, sort by id ASC ──

#[tokio::test]
async fn t803_tie_break_by_strategy_id_asc() {
    let ledger = open_ledger().await;

    let ts_str = "2030-01-01T00:00:00Z";
    inject_strategy_pnl(&ledger, Some("strat_z"), dec!(100), ts_str).await;
    inject_strategy_pnl(&ledger, Some("strat_a"), dec!(100), ts_str).await;
    inject_strategy_pnl(&ledger, Some("strat_m"), dec!(100), ts_str).await;

    let rows: Vec<StrategyPnl> = pnl_by_strategy(&ledger, ts_epoch(), ts_far_future())
        .await
        .expect("pnl_by_strategy");

    // All three have realized = 100; ordering is by strategy_id ASC.
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].strategy_id, StrategyId::new("strat_a"));
    assert_eq!(rows[1].strategy_id, StrategyId::new("strat_m"));
    assert_eq!(rows[2].strategy_id, StrategyId::new("strat_z"));
}
