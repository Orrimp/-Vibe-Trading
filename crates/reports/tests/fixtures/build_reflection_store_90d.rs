#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]
//! T1811 / Q3g — sibling reflection-store builder for the 90d
//! `report-sample-90d` scenario.
//!
//! Seeds **10 lesson cards across 3 strategies** with the **9-cell
//! coverage matrix** (Win/Loss/Scratch × Bull/Bear/Chop) plus 1 pair-MR
//! pair-leg trade.  Pure over the pinned values; no RNG, no clock.

use std::path::Path;

use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::store::ReflectionStore;
use reflection::store::sqlite::SqliteReflectionStore;
use reflection::types::{LessonCard, SymbolOrPair};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use trading_core::{Money, PairKey, StrategyId, Symbol, Timestamp, Usdt};

/// Build a sibling `reflection.db` at `path` for the 90d scenario.
pub async fn build_reflection_store_90d(path: &Path) -> Vec<LessonCard> {
    let store = SqliteReflectionStore::open(path).await.expect("open store");
    let cards = synthetic_cards();
    for c in &cards {
        let _ = store.upsert(c).await.expect("upsert");
    }
    cards
}

/// In-memory variant for tests.
pub async fn build_reflection_store_90d_in_memory() -> (SqliteReflectionStore, Vec<LessonCard>) {
    let store = SqliteReflectionStore::in_memory().await.expect("in_memory");
    let cards = synthetic_cards();
    for c in &cards {
        let _ = store.upsert(c).await.expect("upsert");
    }
    (store, cards)
}

/// 10 cards × 3 strategies × 9-cell outcome×regime matrix + 1 pair-leg.
fn synthetic_cards() -> Vec<LessonCard> {
    let mut out = Vec::with_capacity(10);

    // 9-cell matrix using sma_crossover + top10_momentum_h1 alternating.
    let strategies = ["sma_crossover", "top10_momentum_h1"];
    let cells: [(OutcomeClass, RegimeTag, Decimal); 9] = [
        (OutcomeClass::Win, RegimeTag::Bull, dec!(200.00)),
        (OutcomeClass::Win, RegimeTag::Bear, dec!(180.00)),
        (OutcomeClass::Win, RegimeTag::Chop, dec!(150.00)),
        (OutcomeClass::Loss, RegimeTag::Bull, dec!(-90.00)),
        (OutcomeClass::Loss, RegimeTag::Bear, dec!(-120.00)),
        (OutcomeClass::Loss, RegimeTag::Chop, dec!(-60.00)),
        (OutcomeClass::Scratch, RegimeTag::Bull, dec!(3.00)),
        (OutcomeClass::Scratch, RegimeTag::Bear, dec!(-4.00)),
        (OutcomeClass::Scratch, RegimeTag::Chop, dec!(1.50)),
    ];

    for (i, (oc, reg, pnl)) in cells.iter().enumerate() {
        let s = strategies[i % strategies.len()];
        // Spread across 90 days starting 2026-01-29 (= 2026-04-29 minus 90d).
        let day_offset = (i as i64) * 9; // 9 days apart
        out.push(LessonCard {
            card_id: format!("card-90d-{i:02}-{}", oc).to_lowercase(),
            closed_at: parse_ts_offset_days("2026-01-29T08:00:00Z", day_offset),
            symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
            strategy_id: StrategyId::new(s),
            signed_pnl: Money::<Usdt>::from_decimal(*pnl),
            opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
            holding_period_bars: 60 + i as u32 * 30,
            entry_regime: *reg,
            exit_regime: *reg,
            outcome_class: *oc,
            note: None,
        });
    }

    // 1 pair-MR card — `pairs_mr_h1` strategy, BTC/ETH pair.
    let pair = PairKey::new(Symbol::new("BTCUSDT"), Symbol::new("ETHUSDT")).expect("valid pair");
    out.push(LessonCard {
        card_id: "card-90d-pair-mr-win".into(),
        closed_at: parse_ts_offset_days("2026-04-15T10:00:00Z", 0),
        symbol_or_pair: SymbolOrPair::Pair(pair),
        strategy_id: StrategyId::new("pairs_mr_h1"),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(75.00)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 480,
        entry_regime: RegimeTag::Chop,
        exit_regime: RegimeTag::Chop,
        outcome_class: OutcomeClass::Win,
        note: None,
    });

    out
}

fn parse_ts_offset_days(base: &str, days: i64) -> Timestamp {
    let t = OffsetDateTime::parse(base, &Rfc3339).expect("parse ts") + time::Duration::days(days);
    Timestamp::new(t)
}
