//! T1803 / R2.5 — embedding determinism.
//!
//! - 1000 random `LessonCard`s embed-twice → byte-identical `[Decimal; 32]`.
//! - Cosine of parallel vectors == 1.0.
//! - Cosine of perpendicular vectors == 0.
//! - `embed(card_with_no_strategy)` puts 1.0 in slot 6 (`(unattributed)`).

use proptest::prelude::*;
use reflection::embedding::{cosine, embed, EMBEDDING_DIM, STRATEGY_SLOTS};
use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::types::{LessonCard, SymbolOrPair};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn arb_card() -> impl Strategy<Value = LessonCard> {
    (
        any::<u32>(),
        any::<u32>(),
        any::<i64>(),
        0u32..=10000,
        prop_oneof![
            Just(RegimeTag::Bull),
            Just(RegimeTag::Bear),
            Just(RegimeTag::Chop)
        ],
        prop_oneof![
            Just(OutcomeClass::Win),
            Just(OutcomeClass::Loss),
            Just(OutcomeClass::Scratch)
        ],
        0usize..STRATEGY_SLOTS.len(),
    )
        .prop_map(|(seed, _seed2, pnl, bars, regime, outcome, strategy_idx)| {
            let strategy = STRATEGY_SLOTS[strategy_idx];
            let pnl_dec = Decimal::from(pnl).abs() / dec!(100); // smaller magnitudes
            let pnl_signed = if seed % 2 == 0 { pnl_dec } else { -pnl_dec };
            LessonCard {
                card_id: format!("test-{seed}"),
                closed_at: ts(1_700_000_000 + i64::from(seed % 86_400)),
                symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
                strategy_id: StrategyId::new(strategy),
                signed_pnl: Money::<Usdt>::from_decimal(pnl_signed),
                opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
                holding_period_bars: bars,
                entry_regime: regime,
                exit_regime: regime,
                outcome_class: outcome,
                note: None,
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t1803_embed_byte_stable_over_random_cards(c in arb_card()) {
        let a = embed(&c);
        let b = embed(&c);
        prop_assert_eq!(a, b);
    }
}

#[test]
fn t1803_cosine_of_parallel_vectors_is_one() {
    let v: [Decimal; EMBEDDING_DIM] = {
        let mut x = [Decimal::ZERO; EMBEDDING_DIM];
        x[0] = Decimal::ONE;
        x[1] = dec!(2);
        x[2] = dec!(3);
        x
    };
    let r = cosine(&v, &v);
    // Allow small `decimal_sqrt` rounding; expect ~1.0 within 1e-6.
    let diff = (r - Decimal::ONE).abs();
    assert!(
        diff <= dec!(0.000001),
        "expected ~1.0, got {r} (diff {diff})"
    );
}

#[test]
fn t1803_cosine_of_perpendicular_vectors_is_zero() {
    let mut a = [Decimal::ZERO; EMBEDDING_DIM];
    let mut b = [Decimal::ZERO; EMBEDDING_DIM];
    a[0] = Decimal::ONE;
    b[1] = Decimal::ONE;
    let r = cosine(&a, &b);
    assert_eq!(r, Decimal::ZERO);
}

#[test]
fn t1803_unknown_strategy_falls_into_unattributed_slot() {
    let card = LessonCard {
        card_id: "u".into(),
        closed_at: ts(1_700_000_000),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new("(unattributed)"),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(0)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 0,
        entry_regime: RegimeTag::Chop,
        exit_regime: RegimeTag::Chop,
        outcome_class: OutcomeClass::Scratch,
        note: None,
    };
    let v = embed(&card);
    assert_eq!(v[6], Decimal::ONE);
    for (i, slot) in v.iter().enumerate().take(6) {
        assert_eq!(*slot, Decimal::ZERO, "slot {i} should be zero");
    }
}

#[test]
fn t1803_unknown_strategy_string_also_falls_into_unattributed_slot() {
    let card = LessonCard {
        card_id: "u".into(),
        closed_at: ts(1_700_000_000),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new("totally_new_strategy_v3"),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(0)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 0,
        entry_regime: RegimeTag::Chop,
        exit_regime: RegimeTag::Chop,
        outcome_class: OutcomeClass::Scratch,
        note: None,
    };
    let v = embed(&card);
    assert_eq!(v[6], Decimal::ONE);
}
