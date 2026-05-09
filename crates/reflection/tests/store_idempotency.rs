//! T1806 / R2.4 — store idempotency.
//!
//! Seed a fixture audit ledger with N=10 deliberate closed trades;
//! call `post_mortem_analyst::generate_card` for each; call
//! `store.upsert` 10 times; assert `count() == 10` and
//! `upsert(same_card_again)` returns `Ok(false)` for all 10.
//!
//! Second test: 10 cards from one fixture seeded at seed `0xC0FFEE`,
//! then 10 cards from the same fixture at the same seed → `count()`
//! stays at 10; all 10 second-run upserts return `false`.

use reflection::post_mortem_analyst::generate_card;
use reflection::store::sqlite::SqliteReflectionStore;
use reflection::store::ReflectionStore;
use reflection::types::{ClosedTrade, SymbolOrPair};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn day(n: i64) -> Timestamp {
    ts(1_700_000_000 + n * 86_400)
}

fn btc_closes() -> Vec<(Timestamp, Decimal)> {
    let closes: Vec<Decimal> = (0..30).map(|i| dec!(100) + Decimal::from(i)).collect();
    closes
        .into_iter()
        .enumerate()
        .map(|(i, c)| (day(i as i64), c))
        .collect()
}

fn fixture_trade(i: usize) -> ClosedTrade {
    // Each trade has a distinct (closed_at, signed_pnl) so card_id is unique.
    let strategies = [
        "sma_crossover",
        "macd_trend",
        "rsi_reversion",
        "bbands_mean_revert",
        "top10_momentum_h1",
    ];
    ClosedTrade {
        close_transaction_id: format!("close-{i}"),
        open_transaction_id: format!("open-{i}"),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new(strategies[i % strategies.len()]),
        signed_pnl: Money::<Usdt>::from_decimal(Decimal::from(i as i64) * dec!(10)),
        closed_at: day(8 + i as i64),
        opened_at: day(7 + i as i64),
        holding_period_bars: 1440,
    }
}

#[tokio::test]
async fn t1806_ten_trade_idempotency() {
    let store = SqliteReflectionStore::in_memory().await.expect("open");
    let cap = Money::<Usdt>::from_decimal(dec!(10000));
    let closes = btc_closes();

    let mut cards = Vec::with_capacity(10);
    for i in 0..10 {
        let t = fixture_trade(i);
        let c = generate_card(&t, cap, &closes).await.expect("gen");
        cards.push(c);
    }

    // First-run upserts: all 10 must insert.
    for c in &cards {
        let inserted = store.upsert(c).await.expect("upsert");
        assert!(inserted, "first-run insert for {} expected true", c.card_id);
    }
    assert_eq!(store.count().await.unwrap(), 10);

    // Second-run upserts of the SAME cards: all 10 must return false.
    for c in &cards {
        let inserted = store.upsert(c).await.expect("upsert again");
        assert!(
            !inserted,
            "second-run insert for {} expected false",
            c.card_id
        );
    }
    assert_eq!(store.count().await.unwrap(), 10);
}

#[tokio::test]
async fn t1806_seeded_fixture_idempotency_zero_inserts_on_second_run() {
    // Seed `0xC0FFEE` discipline — a deterministic fixture maps to a
    // deterministic card list.  Same seed in twice produces the same
    // 10 cards; the second-pass upserts must be 0 inserts.
    let store = SqliteReflectionStore::in_memory().await.expect("open");
    let cap = Money::<Usdt>::from_decimal(dec!(10000));
    let closes = btc_closes();

    // First seeded run.
    for i in 0..10 {
        let t = fixture_trade(i);
        let c = generate_card(&t, cap, &closes).await.expect("gen");
        let _ = store.upsert(&c).await.expect("upsert");
    }
    let first_count = store.count().await.unwrap();
    assert_eq!(first_count, 10);

    // Second seeded run — same fixture, same seed → same card_ids.
    let mut zero_inserts = 0;
    for i in 0..10 {
        let t = fixture_trade(i);
        let c = generate_card(&t, cap, &closes).await.expect("gen");
        let inserted = store.upsert(&c).await.expect("upsert 2nd");
        if !inserted {
            zero_inserts += 1;
        }
    }
    assert_eq!(zero_inserts, 10, "second pass must be entirely no-ops");
    assert_eq!(store.count().await.unwrap(), 10);
}
