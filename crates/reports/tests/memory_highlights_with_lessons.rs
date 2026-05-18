//! T1810 / R4.2 + R4.4 — `render_with_lessons` body byte-stable smoke.
//!
//! - K=5 fixture body byte-stable across two calls.
//! - Empty-store body equals `REFLECTION_MEMORY_EMPTY_STATE`.
//! - Decay-co-render body union when both fire.

use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::types::{LessonCard, SymbolOrPair};
use reports::render::memory_highlights::{
    REFLECTION_MEMORY_EMPTY_STATE, render_with_decay, render_with_lessons,
};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn k5_fixture() -> Vec<LessonCard> {
    let entries = [
        (
            "c1",
            "sma_crossover",
            OutcomeClass::Win,
            RegimeTag::Bull,
            60u32,
            dec!(120.50),
        ),
        (
            "c2",
            "macd_trend",
            OutcomeClass::Loss,
            RegimeTag::Bear,
            1440,
            dec!(-45.00),
        ),
        (
            "c3",
            "rsi_reversion",
            OutcomeClass::Scratch,
            RegimeTag::Chop,
            30,
            dec!(2.10),
        ),
        (
            "c4",
            "top10_momentum_h1",
            OutcomeClass::Win,
            RegimeTag::Bull,
            240,
            dec!(310.75),
        ),
        (
            "c5",
            "pairs_mr_h1",
            OutcomeClass::Loss,
            RegimeTag::Chop,
            180,
            dec!(-15.40),
        ),
    ];
    entries
        .iter()
        .enumerate()
        .map(|(i, (id, strat, oc, reg, bars, pnl))| LessonCard {
            card_id: (*id).into(),
            // Pin closed_at to a deterministic fixed date 2026-04-15.
            closed_at: ts(1_776_124_800 + i as i64 * 86_400),
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

#[test]
fn t1810_render_with_lessons_body_byte_stable() {
    let cards = k5_fixture();
    let a = render_with_lessons(&[], &cards);
    let b = render_with_lessons(&[], &cards);
    assert_eq!(a, b, "body must be byte-stable across two invocations");
    assert!(a.contains("Top 5 lesson cards retrieved this period:"));
    assert!(a.contains("[Win]"));
    assert!(a.contains("[Loss]"));
    assert!(a.contains("[Scratch]"));
    assert!(a.contains("regime=bull"));
    assert!(a.contains("pnl=120.50"));
    // Decay candidates section absent when empty.
    assert!(!a.contains("decay candidates:"));
}

#[test]
fn t1810_empty_store_body_equals_empty_state_constant() {
    let body = render_with_lessons(&[], &[]);
    assert!(body.starts_with("## Memory highlights\n\n"));
    assert!(body.contains(REFLECTION_MEMORY_EMPTY_STATE));
    // The byte-locked Q7 string must appear verbatim.
    assert!(
        body.contains(
            "_no closed trades yet — lesson cards will appear after the first closed trade._"
        ),
        "Q7 byte-lock invariant"
    );
}

#[test]
fn t1810_decay_co_render_body_union() {
    let cards = k5_fixture();
    let decayed = vec!["macd_trend".to_string(), "rsi_reversion".to_string()];
    let body = render_with_lessons(&decayed, &cards);
    // Lessons section.
    assert!(body.contains("Top 5 lesson cards retrieved this period:"));
    // Decay-candidates footer.
    assert!(body.contains("decay candidates: macd_trend, rsi_reversion"));
}

#[test]
fn t1810_render_with_decay_back_compat_no_lessons() {
    // Existing v1+ caller path — `render_with_decay(&[])` must keep
    // emitting the empty-state body now that the renderer is
    // upgraded.  R4.4 invariant.
    let body = render_with_decay(&[]);
    assert!(body.contains(REFLECTION_MEMORY_EMPTY_STATE));
}

#[test]
fn t1810_card_line_format_pinned() {
    // Locks the bullet shape in R4.2.  Any change re-anchors
    // `report-sample-*`.
    let cards = vec![LessonCard {
        card_id: "fixed".into(),
        closed_at: ts(1_700_000_000), // 2023-11-14 22:13:20 UTC
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new("sma_crossover"),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(123.45)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 60,
        entry_regime: RegimeTag::Bull,
        exit_regime: RegimeTag::Chop,
        outcome_class: OutcomeClass::Win,
        note: None,
    }];
    let body = render_with_lessons(&[], &cards);
    assert!(
        body.contains(
            "- 2023-11-14 [Win] sma_crossover BTCUSDT regime=chop held=60 bars pnl=123.45"
        ),
        "card line format drifted: {body}"
    );
}
