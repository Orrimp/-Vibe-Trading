//! T1805 — store smoke + durability gate.
//!
//! Open a store at a fixture path, upsert 3 cards, assert
//! `count() == 3`, close + reopen, assert `count() == 3` (durability).

use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::store::ReflectionStore;
use reflection::store::sqlite::SqliteReflectionStore;
use reflection::types::{LessonCard, SymbolOrPair};
use rust_decimal_macros::dec;
use tempfile::tempdir;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn mk_card(card_id: &str, secs_offset: i64, strategy: &str) -> LessonCard {
    LessonCard {
        card_id: card_id.into(),
        closed_at: ts(1_700_000_000 + secs_offset),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new(strategy),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(100)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 60,
        entry_regime: RegimeTag::Bull,
        exit_regime: RegimeTag::Chop,
        outcome_class: OutcomeClass::Win,
        note: None,
    }
}

#[tokio::test]
async fn t1805_store_smoke_in_memory() {
    let store = SqliteReflectionStore::in_memory().await.expect("open");
    assert_eq!(store.count().await.unwrap(), 0);

    let inserted_a = store
        .upsert(&mk_card("a", 0, "sma_crossover"))
        .await
        .unwrap();
    let inserted_b = store.upsert(&mk_card("b", 60, "macd_trend")).await.unwrap();
    let inserted_c = store
        .upsert(&mk_card("c", 120, "rsi_reversion"))
        .await
        .unwrap();
    assert!(inserted_a);
    assert!(inserted_b);
    assert!(inserted_c);

    assert_eq!(store.count().await.unwrap(), 3);

    // Same card again → idempotent skip.
    let again = store
        .upsert(&mk_card("a", 0, "sma_crossover"))
        .await
        .unwrap();
    assert!(!again);
    assert_eq!(store.count().await.unwrap(), 3);
}

#[tokio::test]
async fn t1805_store_smoke_durability() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reflection.db");

    // First open: write 3 cards, drop the store.
    {
        let store = SqliteReflectionStore::open(&path).await.expect("open 1");
        store
            .upsert(&mk_card("a", 0, "sma_crossover"))
            .await
            .unwrap();
        store.upsert(&mk_card("b", 60, "macd_trend")).await.unwrap();
        store
            .upsert(&mk_card("c", 120, "rsi_reversion"))
            .await
            .unwrap();
        assert_eq!(store.count().await.unwrap(), 3);
    }

    // Reopen: count must persist.
    {
        let store = SqliteReflectionStore::open(&path).await.expect("open 2");
        assert_eq!(store.count().await.unwrap(), 3);
    }
}
