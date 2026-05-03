//! T1002 — `audit::query::open_positions_at` integration tests.
//!
//! Per the architect's task spec
//! (`spec/tasks/real-mtm-unrealized-pnl.md` T1002, with semantics from
//! `spec/features/real-mtm-unrealized-pnl.md` Design § Q1, Q3, Q7, Q8 + R10):
//!
//! - Q1 — snapshot-vec signature `open_positions_at(ledger, ts)
//!   -> Result<Vec<OpenPosition>, LedgerError>`.
//! - Q7 — weighted-average cost basis with proportional release on Sells.
//! - Q8 — long-only at v1+; net-negative qty raises
//!   `LedgerError::Database`.
//! - R10 — symbol parsed from the description (every row writes the
//!   literal `"assets:position:BTC"` account, see `journal.rs:82,135`),
//!   so the reader must NOT consume the account_id.
//! - V7 (determinism gate) is exercised separately under T1005; the
//!   8 tests below cover the function's algorithmic surface so the
//!   architect's V1/V4/V7 gates have something to lean on.
//!
//! Tests inject fills via `journal::post_fill` so the description-parser
//! contract is exercised end-to-end (rather than synthesizing
//! `journal_transactions` rows directly).

use audit::query::open_positions_at;
use audit::{bootstrap, journal, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, LedgerError, Liquidity, Money, OrderId, Price, Quantity, Side,
    StrategyId, Symbol, Timestamp,
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
    // 100 years past epoch — well past every fixture timestamp below.
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(36500))
}

/// Build a `Fill` with deterministic seconds-from-epoch venue_ts.
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
    }
}

async fn post(ledger: &Ledger, fill: Fill, strategy_id: Option<&str>) {
    journal::post_fill(ledger, &fill, strategy_id)
        .await
        .expect("post_fill");
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Empty ledger → empty Vec.
#[tokio::test]
async fn t1002_empty_ledger_returns_empty_vec() {
    let ledger = open_ledger().await;
    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");
    assert!(
        positions.is_empty(),
        "expected empty vec, got {positions:?}"
    );
}

/// 1 Buy with no Sell → 1 OpenPosition with the Buy's (qty, price, ts, sid).
#[tokio::test]
async fn t1002_single_open_position() {
    let ledger = open_ledger().await;
    let buy = make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 100);
    post(&ledger, buy, Some("strat_alpha")).await;

    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");

    assert_eq!(positions.len(), 1, "expected 1 open position");
    let p = &positions[0];
    assert_eq!(p.symbol, Symbol::new("BTCUSDT"));
    assert_eq!(p.qty, dec!(0.01));
    assert_eq!(p.avg_cost_basis, Money::from_decimal(dec!(60_000)));
    assert_eq!(p.opened_at, ts_offset_secs(100));
    assert_eq!(p.strategy_id, Some(StrategyId::new("strat_alpha")));
}

/// 1 Buy + matching Sell of same qty → empty Vec.
#[tokio::test]
async fn t1002_closed_position_excluded() {
    let ledger = open_ledger().await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 100),
        Some("strat_alpha"),
    )
    .await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Sell, dec!(0.01), dec!(70_000), 200),
        Some("strat_alpha"),
    )
    .await;

    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");
    assert!(
        positions.is_empty(),
        "closed position should not surface, got {positions:?}"
    );
}

/// 2 Buys at different prices → cost_basis = (qty1*p1 + qty2*p2) / (qty1+qty2).
#[tokio::test]
async fn t1002_weighted_avg_cost_basis() {
    let ledger = open_ledger().await;
    // Buy 1 BTCUSDT @ 60_000  → notional 60_000, qty 1
    // Buy 3 BTCUSDT @ 70_000  → notional 210_000, qty 3
    // Combined: notional 270_000 / qty 4 = 67_500
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Buy, dec!(1), dec!(60_000), 100),
        Some("strat_alpha"),
    )
    .await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Buy, dec!(3), dec!(70_000), 200),
        Some("strat_alpha"),
    )
    .await;

    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");

    assert_eq!(positions.len(), 1);
    let p = &positions[0];
    assert_eq!(p.qty, dec!(4));
    assert_eq!(
        p.avg_cost_basis,
        Money::from_decimal(dec!(67_500)),
        "weighted-avg should be (1*60_000 + 3*70_000) / 4 = 67_500"
    );
    // opened_at = ts of the FIRST Buy (the lot is still open).
    assert_eq!(p.opened_at, ts_offset_secs(100));
}

/// Buy 10, Sell 4 → leaves qty=6 with cost_basis unchanged on Sell (Q7
/// proportional release leaves per-unit basis invariant).
#[tokio::test]
async fn t1002_partial_close() {
    let ledger = open_ledger().await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Buy, dec!(10), dec!(50_000), 100),
        Some("strat_alpha"),
    )
    .await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Sell, dec!(4), dec!(80_000), 200),
        Some("strat_alpha"),
    )
    .await;

    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");

    assert_eq!(positions.len(), 1);
    let p = &positions[0];
    assert_eq!(p.qty, dec!(6), "10 buy − 4 sell = 6 remaining");
    // Per-unit cost basis is preserved on a Sell because we release the
    // proportional share of running_notional. Pre-Sell notional = 500_000;
    // released = (500_000 / 10) * 4 = 200_000. Post: notional 300_000 / qty 6
    // = 50_000 — same as the original Buy price.
    assert_eq!(
        p.avg_cost_basis,
        Money::from_decimal(dec!(50_000)),
        "Q7: cost basis unchanged on Sell (proportional release)"
    );
    assert_eq!(p.opened_at, ts_offset_secs(100));
}

/// Fills across 3 symbols → 3 OpenPositions sorted by symbol ASC.
#[tokio::test]
async fn t1002_multi_symbol_sorted() {
    let ledger = open_ledger().await;
    // Insert in non-alphabetical order to prove the sort fires.
    post(
        &ledger,
        make_fill("SOLUSDT", Side::Buy, dec!(10), dec!(150), 100),
        Some("strat_gamma"),
    )
    .await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 200),
        Some("strat_alpha"),
    )
    .await;
    post(
        &ledger,
        make_fill("ETHUSDT", Side::Buy, dec!(0.20), dec!(3_000), 300),
        Some("strat_beta"),
    )
    .await;

    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");

    assert_eq!(positions.len(), 3, "expected 3 distinct open positions");
    assert_eq!(
        positions[0].symbol,
        Symbol::new("BTCUSDT"),
        "alphabetical first"
    );
    assert_eq!(positions[1].symbol, Symbol::new("ETHUSDT"));
    assert_eq!(positions[2].symbol, Symbol::new("SOLUSDT"));
}

/// Buy with strategy_id Some(...) → returned OpenPosition carries that id.
#[tokio::test]
async fn t1002_strategy_id_preserved() {
    let ledger = open_ledger().await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 100),
        Some("sma_crossover"),
    )
    .await;

    let positions = open_positions_at(&ledger, ts_far_future())
        .await
        .expect("open_positions_at");

    assert_eq!(positions.len(), 1);
    assert_eq!(
        positions[0].strategy_id,
        Some(StrategyId::new("sma_crossover")),
        "strategy_id from journal_transactions.strategy_id column should propagate"
    );
}

/// One Sell with no prior Buy → net-negative qty → LedgerError::Database (Q8).
#[tokio::test]
async fn t1002_net_negative_returns_err() {
    let ledger = open_ledger().await;
    post(
        &ledger,
        make_fill("BTCUSDT", Side::Sell, dec!(1), dec!(70_000), 100),
        Some("strat_alpha"),
    )
    .await;

    let result = open_positions_at(&ledger, ts_far_future()).await;

    match result {
        Err(LedgerError::Database(msg)) => {
            assert!(
                msg.contains("net-negative qty"),
                "error message should mention net-negative qty, got: {msg}"
            );
        }
        Err(other) => panic!("expected LedgerError::Database, got {other:?}"),
        Ok(positions) => panic!("expected Err for net-negative qty, got Ok({positions:?})"),
    }
}
