//! Integration tests for T05 (chart of accounts) and T06 (journal balance).
//!
//! T05 acceptance: in-memory SQLite ledger, bootstrap, `account_list()` returns
//!                 all 13 v0 accounts.
//! T06 acceptance: post 100 synthetic fills; `Σ debits == Σ credits` for every
//!                 transaction.

use audit::{bootstrap, journal, query, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    Venue,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_fill(side: Side, qty: Decimal, price: Decimal, fee: Decimal) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side,
        qty: Quantity::new(qty).expect("qty ok"),
        price: Price::new(price).expect("price ok"),
        fee: Money::from_decimal(fee),
        fee_tier: FeeTier::Taker,
        venue_ts: Timestamp::now(),
        local_ts: Timestamp::now(),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

// ── T05: chart of accounts ────────────────────────────────────────────────────

#[tokio::test]
async fn t05_account_list_returns_all_v0_accounts() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    let accounts = query::account_list(&ledger).await.expect("account_list");

    let expected: &[&str] = &[
        "assets:cash:USDT",
        "assets:position:BTC",
        "assets:position_mark:BTC",
        "equity:opening_balance",
        "expense:data",
        "expense:fees:maker",
        "expense:fees:taker",
        "expense:infra",
        "expense:llm:deep_think",
        "expense:llm:quick_think",
        "income:realized_pnl",
        "income:unrealized_pnl",
        "liabilities:llm_accrued",
        // T1101 — per-symbol position accounts seeded by migration 006
        // (config/agent.toml [funding].universe).
        "assets:position:BTCUSDT",
        "assets:position:ETHUSDT",
        "assets:position:BNBUSDT",
        "assets:position:SOLUSDT",
        "assets:position:XRPUSDT",
        "assets:position:ADAUSDT",
        "assets:position:DOGEUSDT",
        "assets:position:AVAXUSDT",
        "assets:position:DOTUSDT",
        "assets:position:LINKUSDT",
    ];

    assert_eq!(
        accounts.len(),
        expected.len(),
        "expected {} accounts, got {}: {:?}",
        expected.len(),
        accounts.len(),
        accounts,
    );

    for e in expected {
        assert!(accounts.contains(&e.to_string()), "missing account: {e}");
    }
}

#[tokio::test]
async fn t05_bootstrap_is_idempotent() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("first bootstrap");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("second bootstrap — must not fail");

    let accounts = query::account_list(&ledger).await.expect("account_list");
    // 13 v0 accounts + 10 per-symbol position accounts seeded by migration 006
    // (T1101). `chart_of_accounts` is idempotent (INSERT OR IGNORE) and the
    // migration's own INSERTs are also idempotent, so the total stays 23.
    assert_eq!(accounts.len(), 23, "idempotent: still 23 accounts");
}

// ── T06: journal balance ──────────────────────────────────────────────────────

/// Post 100 alternating buy/sell fills and assert every transaction balances.
#[tokio::test]
async fn t06_100_fills_all_transactions_balance() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    for i in 0u64..100 {
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        let price = dec!(50000) + Decimal::from(i % 10) * dec!(100);
        let qty = dec!(0.01) + Decimal::from(i % 5) * dec!(0.001);
        let fee = qty * price * dec!(0.001);
        let fill = make_fill(side, qty, price, fee);
        journal::post_fill(&ledger, &fill, Venue::Binance, None)
            .await
            .expect("post_fill");
    }

    let txn_ids = query::all_transaction_ids(&ledger)
        .await
        .expect("all_transaction_ids");
    assert_eq!(txn_ids.len(), 100, "expected 100 transactions");

    for txn_id in &txn_ids {
        query::verify_transaction_balance(&ledger, txn_id)
            .await
            .unwrap_or_else(|e| panic!("balance check failed for {txn_id}: {e}"));
    }
}

/// Global balance: Σ debits == Σ credits across all entries.
#[tokio::test]
async fn t06_global_debit_credit_equality() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    for i in 0u64..20 {
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        let price = dec!(48000) + Decimal::from(i) * dec!(200);
        let qty = dec!(0.05);
        let fee = qty * price * dec!(0.001);
        let fill = make_fill(side, qty, price, fee);
        journal::post_fill(&ledger, &fill, Venue::Binance, None)
            .await
            .expect("post_fill");
    }

    let (total_dr, total_cr) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("global_debit_credit_sum");

    assert!(
        (total_dr - total_cr).abs() <= dec!(0.00000001),
        "global imbalance: Σdr={total_dr} Σcr={total_cr}"
    );
}

// ── T05 extra: cash_balance reads from journal ─────────────────────────────────

#[tokio::test]
async fn t05_cash_balance_after_buy_fill() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    let price = dec!(50000);
    let qty = dec!(0.1);
    let fee = dec!(5);
    let fill = make_fill(Side::Buy, qty, price, fee);
    journal::post_fill(&ledger, &fill, Venue::Binance, None)
        .await
        .expect("post_fill");

    let cash = query::cash_balance(&ledger).await.expect("cash_balance");
    // Buy: Cr assets:cash notional + Cr assets:cash fee → cr-dr = notional+fee
    // (positive means more credits than debits, i.e. cash was paid out)
    let expected = qty * price + fee;
    assert!(
        (cash.amount() - expected).abs() <= dec!(0.00000001),
        "cash_balance {} != {}",
        cash.amount(),
        expected
    );
}

// ── T802 — strategy_id column populated by post_fill ─────────────────────────

#[tokio::test]
async fn t802_post_fill_populates_strategy_id_when_some() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    let fill = make_fill(Side::Buy, dec!(0.1), dec!(50000), dec!(5));
    journal::post_fill(&ledger, &fill, Venue::Binance, Some("sma_crossover"))
        .await
        .expect("post_fill with strategy_id");

    // Read the column directly — verify it stores the strategy id verbatim.
    let rows: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT strategy_id FROM journal_transactions")
            .fetch_all(ledger.pool())
            .await
            .expect("select strategy_id");

    assert_eq!(rows.len(), 1, "expected 1 transaction row");
    assert_eq!(
        rows[0].0.as_deref(),
        Some("sma_crossover"),
        "strategy_id must be populated verbatim"
    );
}

#[tokio::test]
async fn t802_post_fill_leaves_strategy_id_null_when_none() {
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    let fill = make_fill(Side::Buy, dec!(0.1), dec!(50000), dec!(5));
    journal::post_fill(&ledger, &fill, Venue::Binance, None)
        .await
        .expect("post_fill without strategy_id");

    let rows: Vec<(Option<String>,)> =
        sqlx::query_as("SELECT strategy_id FROM journal_transactions")
            .fetch_all(ledger.pool())
            .await
            .expect("select strategy_id");

    assert_eq!(rows.len(), 1, "expected 1 transaction row");
    assert_eq!(rows[0].0, None, "strategy_id must be NULL when None passed");
}

#[tokio::test]
async fn t802_migration_004_creates_index() {
    // Verify the migration created the (strategy_id, ts) index — required
    // for sub-millisecond `pnl_by_strategy` queries at v1+ scale.
    let ledger = Ledger::in_memory().await.expect("open");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='index' \
         AND name='journal_transactions_sid_idx'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select index");

    assert_eq!(
        rows.len(),
        1,
        "migration 004 must create journal_transactions_sid_idx"
    );
}
