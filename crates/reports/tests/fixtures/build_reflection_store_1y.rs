#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]
//! T1811 — perf-smoke fixture (R7.2).  Seeds ≥500 lesson cards into a
//! sibling `reflection.db` for the perf gate.

use std::path::Path;

use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::store::sqlite::SqliteReflectionStore;
use reflection::store::ReflectionStore;
use reflection::types::{LessonCard, SymbolOrPair};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

/// Seed 500 lesson cards across the 1-year window for perf-smoke.
pub async fn build_reflection_store_1y(path: &Path) -> usize {
    let store = SqliteReflectionStore::open(path).await.expect("open store");
    seed_cards(&store).await
}

/// In-memory variant for tests.
pub async fn build_reflection_store_1y_in_memory() -> (SqliteReflectionStore, usize) {
    let store = SqliteReflectionStore::in_memory().await.expect("in_memory");
    let n = seed_cards(&store).await;
    (store, n)
}

async fn seed_cards(store: &SqliteReflectionStore) -> usize {
    let strategies = [
        "sma_crossover",
        "macd_trend",
        "rsi_reversion",
        "bbands_mean_revert",
        "top10_momentum_h1",
        "pairs_mr_h1",
    ];
    let regimes = [RegimeTag::Bull, RegimeTag::Bear, RegimeTag::Chop];
    let outcomes = [OutcomeClass::Win, OutcomeClass::Loss, OutcomeClass::Scratch];
    let base = OffsetDateTime::parse("2025-04-29T00:00:00Z", &Rfc3339).expect("parse base ts");

    let mut count = 0;
    for i in 0..500usize {
        let s = strategies[i % strategies.len()];
        let r = regimes[i % regimes.len()];
        let o = outcomes[i % outcomes.len()];
        // 500 cards spread across ~365 days, staggered.
        let secs = (i as i64) * (365 * 86_400 / 500);
        let pnl_sign: Decimal = if i % 2 == 0 {
            Decimal::ONE
        } else {
            -Decimal::ONE
        };
        let pnl = pnl_sign * Decimal::from(100 + (i % 50) as i64);
        let card = LessonCard {
            card_id: format!("card-1y-{i:04}"),
            closed_at: Timestamp::new(base + time::Duration::seconds(secs)),
            symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
            strategy_id: StrategyId::new(s),
            signed_pnl: Money::<Usdt>::from_decimal(pnl),
            opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
            holding_period_bars: u32::try_from(60 + (i % 200)).unwrap_or(60),
            entry_regime: r,
            exit_regime: r,
            outcome_class: o,
            note: None,
        };
        let inserted = store.upsert(&card).await.expect("upsert");
        if inserted {
            count += 1;
        }
    }
    count
}
