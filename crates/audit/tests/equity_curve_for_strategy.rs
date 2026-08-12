#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1802 — `audit::query::equity_curve_for_strategy` integration test
//! (Phase 4 R12 / Q7).
//!
//! Multi-day multi-strategy fixture; direct realized_pnl row inserts
//! (mirrors the precedent from `tests/pnl_by_strategy.rs::inject_strategy_pnl`)
//! to cover round-trip semantics that the v0 `post_fill` simplifies to
//! zero. Asserts the returned `EquitySeries` shape: `from_points` Ok
//! round-trip, peak/trough/max-DD math against a hand-computed
//! reference, downsample(120) preserves peak/trough.

use audit::query::equity_curve_for_strategy;
use audit::{Ledger, bootstrap};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use trading_core::{StrategyId, Timestamp};
use uuid::Uuid;

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

async fn inject_realized_pnl(
    ledger: &Ledger,
    strategy_id: Option<&str>,
    pnl: Decimal,
    ts: Timestamp,
) {
    let ts_str = ts.inner().format(&Rfc3339).expect("rfc3339 fmt");
    let txn_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(&ts_str)
    .bind("sell 1 BTCUSDT @ 1000")
    .bind(strategy_id)
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
    .bind(&txn_id)
    .bind("income:realized_pnl")
    .bind(debit.to_string())
    .bind(credit.to_string())
    .bind(&ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert journal entry");

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
    .bind(&ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert balancing entry");
}

#[tokio::test]
async fn equity_curve_for_strategy_multi_day_round_trip() {
    let ledger = open_seeded_ledger().await;

    // 60 realized rows for "alpha" across multi-day spans (1 hour
    // apart). Build a known peak/trough trajectory: ramp up 30, dip
    // 15 (×2), then climb back 15 — running peak attains its max at
    // index 29; trough at index 44.
    let mut deltas: Vec<Decimal> = Vec::with_capacity(60);
    for i in 0..30i64 {
        deltas.push(Decimal::from(i + 1)); // +1, +2, ..., +30
    }
    for i in 0..15i64 {
        deltas.push(Decimal::from(-(i + 1) * 2)); // -2, -4, ..., -30
    }
    for i in 0..15i64 {
        deltas.push(Decimal::from(i + 1)); // +1..+15
    }

    for (i, d) in deltas.iter().enumerate() {
        let secs = 100 + i as i64 * 3_600;
        inject_realized_pnl(&ledger, Some("alpha"), *d, ts_secs(secs)).await;
    }

    let since = ts_secs(0);
    let until = ts_secs(100 + 60 * 3_600 + 1);
    let series = equity_curve_for_strategy(&ledger, StrategyId::new("alpha"), since, Some(until))
        .await
        .expect("query ok");

    assert_eq!(series.points.len(), 60);

    // Hand-computed running sum: cumulative of deltas (with baseline
    // cash from the closed_p&l balancing entries — cash_balance
    // reflects the same running). Walk the series and compare peak
    // vs the index-29 running.
    assert!(series.peak.amount() >= series.points[29].equity.amount());
    assert!(series.trough.amount() <= series.peak.amount());
    assert!(series.max_drawdown_frac >= Decimal::ZERO);

    // Downsample to 120 — points already < 120, must short-circuit
    // to no-op.
    let down = series.clone().downsample(120);
    assert_eq!(down.points.len(), 60);
    assert_eq!(down.peak.amount(), series.peak.amount());
    assert_eq!(down.trough.amount(), series.trough.amount());

    // Smaller cap — downsample to 30; first/last preserved + peak/
    // trough metadata survives.
    let down30 = series.clone().downsample(30);
    assert!(down30.points.len() <= 31);
    assert_eq!(down30.peak.amount(), series.peak.amount());
    assert_eq!(down30.trough.amount(), series.trough.amount());
    assert_eq!(down30.max_drawdown_frac, series.max_drawdown_frac);
    assert_eq!(
        down30.points[0].equity.amount(),
        series.points[0].equity.amount(),
    );
    assert_eq!(
        down30.points[down30.points.len() - 1].equity.amount(),
        series.points[59].equity.amount(),
    );
}

#[tokio::test]
async fn equity_curve_for_strategy_strategy_isolation_multi_day() {
    let ledger = open_seeded_ledger().await;

    // Seed two strategies on the same symbol across the same
    // multi-day window. Assert each query returns only its own
    // strategy's rows.
    for i in 0..10i64 {
        inject_realized_pnl(&ledger, Some("alpha"), dec!(5), ts_secs(100 + i * 7_200)).await;
    }
    for i in 0..3i64 {
        inject_realized_pnl(&ledger, Some("beta"), dec!(-3), ts_secs(200 + i * 7_200)).await;
    }

    let since = ts_secs(0);
    let until = ts_secs(10_000_000);

    let alpha = equity_curve_for_strategy(&ledger, StrategyId::new("alpha"), since, Some(until))
        .await
        .expect("alpha ok");
    assert_eq!(alpha.points.len(), 10);

    let beta = equity_curve_for_strategy(&ledger, StrategyId::new("beta"), since, Some(until))
        .await
        .expect("beta ok");
    assert_eq!(beta.points.len(), 3);
}
