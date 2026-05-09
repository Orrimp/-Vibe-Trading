#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]
//! T1811 / Q3g — sibling reflection-store builder for the 7d
//! `report-sample-7d` scenario.
//!
//! Seeds **3 lesson cards across 2 strategies** (1 Win + 1 Loss + 1
//! Scratch; 1 Bull + 1 Bear + 1 Chop regime), aligned with the
//! existing `build_ledger_7d` window timestamps.  Same `FIXTURE_SEED
//! = 0xC0FFEE` discipline.

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

/// Build a sibling `reflection.db` at `path` for the 7d scenario.
///
/// Returns the 3-card list for assertion in tests.  Pure over the
/// pinned timestamps + signed-pnl values; no clock, no RNG.
pub async fn build_reflection_store_7d(path: &Path) -> Vec<LessonCard> {
    let store = SqliteReflectionStore::open(path).await.expect("open store");
    let cards = synthetic_cards();
    for c in &cards {
        let _ = store.upsert(c).await.expect("upsert");
    }
    cards
}

/// Build the same store in-memory and return both store + cards.
pub async fn build_reflection_store_7d_in_memory() -> (SqliteReflectionStore, Vec<LessonCard>) {
    let store = SqliteReflectionStore::in_memory().await.expect("in_memory");
    let cards = synthetic_cards();
    for c in &cards {
        let _ = store.upsert(c).await.expect("upsert");
    }
    (store, cards)
}

/// 3 cards × 2 strategies × 3 outcomes × 3 regimes per Q3g.
fn synthetic_cards() -> Vec<LessonCard> {
    // 3 closed_at timestamps inside 2026-04-21 .. 2026-04-28.
    let entries: [(&str, &str, OutcomeClass, RegimeTag, u32, Decimal, &str); 3] = [
        (
            "card-7d-win",
            "sma_crossover",
            OutcomeClass::Win,
            RegimeTag::Bull,
            120,
            dec!(150.00),
            "2026-04-22T08:00:00Z",
        ),
        (
            "card-7d-loss",
            "macd_trend",
            OutcomeClass::Loss,
            RegimeTag::Bear,
            240,
            dec!(-80.00),
            "2026-04-24T12:00:00Z",
        ),
        (
            "card-7d-scratch",
            "sma_crossover",
            OutcomeClass::Scratch,
            RegimeTag::Chop,
            60,
            dec!(2.50),
            "2026-04-26T18:00:00Z",
        ),
    ];

    entries
        .iter()
        .map(|(id, strat, oc, reg, bars, pnl, ts_str)| LessonCard {
            card_id: (*id).into(),
            closed_at: parse_ts(ts_str),
            symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
            strategy_id: StrategyId::new(*strat),
            signed_pnl: Money::<Usdt>::from_decimal(*pnl),
            opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
            holding_period_bars: *bars,
            entry_regime: *reg,
            exit_regime: *reg,
            outcome_class: *oc,
            note: None,
        })
        .collect()
}

fn parse_ts(s: &str) -> Timestamp {
    Timestamp::new(OffsetDateTime::parse(s, &Rfc3339).expect("parse ts"))
}
