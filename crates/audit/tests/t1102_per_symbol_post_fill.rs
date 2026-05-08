#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1102 — Per-symbol post_fill writer + Q4 reader cross-check.
//!
//! Per `spec/features/per-symbol-position-accounts.md` Design § Q2 + Q4 and
//! `spec/tasks/per-symbol-position-accounts.md` T1102:
//!
//! - **`t1102_post_fill_writes_per_symbol_account`** — Writer flip (Q2).
//!   Posting an ETHUSDT Buy via `audit::journal::post_fill` writes a
//!   `journal_entries` row with `account_id = "assets:position:ETHUSDT"`,
//!   NOT the legacy `"assets:position:BTC"`. Asserted alongside symmetric
//!   cases for BTCUSDT and SOLUSDT.
//! - **`t1102_open_positions_at_handles_legacy_rows`** — Reader Q4
//!   cross-check tolerates legacy rows. The fixture mixes pre-T1102 shape
//!   (legacy `"assets:position:BTC"` row whose description carries
//!   `ETHUSDT`) and post-T1102 shape (per-pair account-id). The reader
//!   returns the correct positions for both — description-parse remains
//!   the primary symbol source (Q4 primary), and the cross-check warns
//!   silently in tests when the legacy row's account_id doesn't match
//!   the description-parsed symbol.

use audit::query::open_positions_at;
use audit::{bootstrap, journal, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, StrategyId, Symbol,
    Timestamp, Venue,
};

// ── helpers ───────────────────────────────────────────────────────────────────

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_offset_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

fn ts_far_future() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(36_500))
}

fn make_fill(symbol: &str, side: Side, qty: Decimal, price: Decimal, venue_ts_secs: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new(symbol),
        side,
        qty: Quantity::new(qty).expect("qty ok"),
        price: Price::new(price).expect("price ok"),
        fee: Money::from_decimal(dec!(0)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_offset_secs(venue_ts_secs),
        local_ts: ts_offset_secs(venue_ts_secs),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Writer flip — `post_fill` writes to per-pair account, not the legacy BTC bucket.
#[tokio::test]
async fn t1102_post_fill_writes_per_symbol_account() {
    let ledger = open_ledger().await;

    // Post one fill per symbol so we can assert account-id distribution.
    journal::post_fill(
        &ledger,
        &make_fill("ETHUSDT", Side::Buy, dec!(0.5), dec!(2_000), 100),
        Venue::Binance,
        Some("strat_eth"),
    )
    .await
    .expect("post ETHUSDT fill");
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 200),
        Venue::Binance,
        Some("strat_btc"),
    )
    .await
    .expect("post BTCUSDT fill");
    journal::post_fill(
        &ledger,
        &make_fill("SOLUSDT", Side::Buy, dec!(10), dec!(100), 300),
        Venue::Binance,
        Some("strat_sol"),
    )
    .await
    .expect("post SOLUSDT fill");

    // ETHUSDT row — exactly one journal_entries row with the per-pair account.
    let eth_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM journal_entries WHERE account_id = 'assets:position:ETHUSDT'",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("count ETHUSDT position rows");
    assert_eq!(
        eth_count.0, 1,
        "expected exactly one journal_entries row on `assets:position:ETHUSDT`; got {}",
        eth_count.0
    );

    // BTCUSDT row — must hit the per-pair account, NOT the legacy `assets:position:BTC` row.
    let btc_pair_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM journal_entries WHERE account_id = 'assets:position:BTCUSDT'",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("count BTCUSDT position rows");
    assert_eq!(
        btc_pair_count.0, 1,
        "expected exactly one journal_entries row on `assets:position:BTCUSDT`; got {}",
        btc_pair_count.0
    );

    // SOLUSDT row.
    let sol_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM journal_entries WHERE account_id = 'assets:position:SOLUSDT'",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("count SOLUSDT position rows");
    assert_eq!(
        sol_count.0, 1,
        "expected exactly one journal_entries row on `assets:position:SOLUSDT`; got {}",
        sol_count.0
    );

    // Legacy BTC bucket — zero post-T1102 rows. The chart-of-accounts row
    // still EXISTS (bootstrap.rs seeds it for backwards compat) but no
    // journal_entries rows from the three post_fill calls above target it.
    let legacy_btc_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM journal_entries WHERE account_id = 'assets:position:BTC'",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("count legacy BTC bucket rows");
    assert_eq!(
        legacy_btc_count.0, 0,
        "expected zero post-T1102 journal_entries rows on the legacy \
         `assets:position:BTC` bucket; got {} (regression: the BTC hardcode \
         is back at journal.rs:82 / journal.rs:135)",
        legacy_btc_count.0
    );
}

/// Reader Q4 cross-check — handles legacy rows (account_id = `assets:position:BTC`
/// with description carrying BTCUSDT or ETHUSDT) AND new per-pair rows in the same ledger.
#[tokio::test]
async fn t1102_open_positions_at_handles_legacy_rows() {
    let ledger = open_ledger().await;

    // (1) Post-T1102 row — the updated `post_fill` writes
    //     `account_id = 'assets:position:ETHUSDT'` for an ETHUSDT Buy.
    journal::post_fill(
        &ledger,
        &make_fill("ETHUSDT", Side::Buy, dec!(0.5), dec!(2_000), 100),
        Venue::Binance,
        Some("strat_eth"),
    )
    .await
    .expect("post per-pair ETHUSDT fill");

    // (2) Legacy row — synthesise a pre-T1102-shape transaction by hand:
    //     description carries `BTCUSDT` but the journal_entries row writes
    //     to the legacy `'assets:position:BTC'` account-id (the hardcode
    //     this feature retires). The Q4 cross-check tolerates this shape.
    //
    //     Description format mirrors `journal.rs::post_fill`:
    //       "<side> <qty> <symbol> @ <price>"
    let legacy_txn_id = uuid::Uuid::new_v4().to_string();
    let legacy_ts = "2026-04-27T20:00:00Z";
    let legacy_desc = "buy 0.01 BTCUSDT @ 60000";
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&legacy_txn_id)
    .bind(legacy_ts)
    .bind(legacy_desc)
    .bind("strat_btc_legacy")
    .execute(ledger.pool())
    .await
    .expect("insert legacy txn header");
    // Position-side leg: Dr `assets:position:BTC` 600  (legacy hardcode).
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, 'assets:position:BTC', '600', '0', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&legacy_txn_id)
    .bind(legacy_ts)
    .execute(ledger.pool())
    .await
    .expect("insert legacy position-side entry");
    // Cash-side leg: Cr `assets:cash:USDT` 600  (keeps Σ Dr == Σ Cr).
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, 'assets:cash:USDT', '0', '600', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&legacy_txn_id)
    .bind(legacy_ts)
    .execute(ledger.pool())
    .await
    .expect("insert legacy cash-side entry");

    // Reader output — both rows surface as separate (symbol, strategy_id) lots.
    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at on mixed ledger");

    assert_eq!(
        positions.len(),
        2,
        "expected 2 open positions (BTCUSDT legacy + ETHUSDT per-pair); got {}",
        positions.len()
    );

    // Sorted alphabetically — BTCUSDT first, ETHUSDT second (R6).
    let btc = &positions[0];
    assert_eq!(
        btc.symbol,
        Symbol::new("BTCUSDT"),
        "row 0 symbol — legacy row description carries BTCUSDT despite \
         account_id == 'assets:position:BTC' (Q4 description-parse primary)"
    );
    assert_eq!(btc.qty, dec!(0.01), "row 0 qty");
    assert_eq!(
        btc.avg_cost_basis,
        Money::from_decimal(dec!(60_000)),
        "row 0 avg_cost_basis"
    );
    assert_eq!(
        btc.strategy_id,
        Some(StrategyId::new("strat_btc_legacy")),
        "row 0 strategy_id from legacy txn"
    );

    let eth = &positions[1];
    assert_eq!(
        eth.symbol,
        Symbol::new("ETHUSDT"),
        "row 1 symbol — per-pair row, description-parse and account-id \
         agree (cross-check passes silently)"
    );
    assert_eq!(eth.qty, dec!(0.5), "row 1 qty");
    assert_eq!(
        eth.avg_cost_basis,
        Money::from_decimal(dec!(2_000)),
        "row 1 avg_cost_basis"
    );
    assert_eq!(
        eth.strategy_id,
        Some(StrategyId::new("strat_eth")),
        "row 1 strategy_id"
    );
}
