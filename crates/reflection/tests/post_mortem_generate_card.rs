//! T1804 / R1.1 + R2.3 — `post_mortem_analyst::generate_card` smoke.
//!
//! 3-trade fixture → 3 expected `LessonCard`s, byte-stable across
//! two calls. Outcome class matches Q3c. `note == None`.

use reflection::outcome::OutcomeClass;
use reflection::post_mortem_analyst::generate_card;
use reflection::regime::RegimeTag;
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
    // 9 daily samples — index 0..8.
    // We tune values so the open / close timestamps map to specific regimes.
    // open at day 7, close at day 9 → check both at points.
    // To make day 7 → +3% from day 0 (Bull at open):
    //   close[0] = 100, close[7] = 103.
    // To make day 9 → -3% from day 2 (Bear at close):
    //   close[2] = 100, close[9] = 97.
    // Build the series with explicit values:
    let closes: Vec<Decimal> = vec![
        dec!(100.0), // 0
        dec!(100.0), // 1
        dec!(100.0), // 2
        dec!(100.0), // 3
        dec!(100.0), // 4
        dec!(100.0), // 5
        dec!(100.0), // 6
        dec!(103.0), // 7  (Bull at open: 7d return = +3%)
        dec!(101.0), // 8
        dec!(97.0),  // 9  (Bear at close: 7d return from day 2 = -3%)
    ];
    closes
        .into_iter()
        .enumerate()
        .map(|(i, c)| (day(i as i64), c))
        .collect()
}

fn trade(
    suffix: &str,
    pnl: Decimal,
    bars: u32,
    strategy: &str,
    opened_at: Timestamp,
    closed_at: Timestamp,
) -> ClosedTrade {
    ClosedTrade {
        close_transaction_id: format!("close-{suffix}"),
        open_transaction_id: format!("open-{suffix}"),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new(strategy),
        signed_pnl: Money::<Usdt>::from_decimal(pnl),
        closed_at,
        opened_at,
        holding_period_bars: bars,
    }
}

#[tokio::test]
async fn t1804_generate_card_three_trade_fixture() {
    let closes = btc_closes();
    let opening_capital = Money::<Usdt>::from_decimal(dec!(10000));

    // Trade 1: Win (+1%) opened at day 7 (Bull), closed at day 9 (Bear).
    let t1 = trade(
        "win",
        dec!(100), // 1% of 10000 → > 0.5% → Win
        2880,      // 2 days × 1440 bars
        "sma_crossover",
        day(7),
        day(9),
    );
    let c1 = generate_card(&t1, opening_capital, &closes)
        .await
        .expect("c1");
    assert_eq!(c1.outcome_class, OutcomeClass::Win);
    assert_eq!(c1.entry_regime, RegimeTag::Bull);
    assert_eq!(c1.exit_regime, RegimeTag::Bear);
    assert_eq!(c1.note, None);

    // Trade 2: Loss (-1%).
    let t2 = trade("loss", dec!(-100), 2880, "macd_trend", day(7), day(9));
    let c2 = generate_card(&t2, opening_capital, &closes)
        .await
        .expect("c2");
    assert_eq!(c2.outcome_class, OutcomeClass::Loss);

    // Trade 3: Scratch (0.1% net).
    let t3 = trade("scratch", dec!(10), 2880, "rsi_reversion", day(7), day(9));
    let c3 = generate_card(&t3, opening_capital, &closes)
        .await
        .expect("c3");
    assert_eq!(c3.outcome_class, OutcomeClass::Scratch);
}

#[tokio::test]
async fn t1804_generate_card_byte_stable_across_two_calls() {
    let closes = btc_closes();
    let cap = Money::<Usdt>::from_decimal(dec!(10000));
    let t = trade("a", dec!(50), 1440, "sma_crossover", day(7), day(9));
    let a = generate_card(&t, cap, &closes).await.expect("a");
    let b = generate_card(&t, cap, &closes).await.expect("b");
    assert_eq!(a, b);
    assert_eq!(a.card_id, b.card_id);
}

#[tokio::test]
async fn t1804_generate_card_note_is_always_none_in_v1() {
    let closes = btc_closes();
    let cap = Money::<Usdt>::from_decimal(dec!(10000));
    let t = trade("n", dec!(50), 60, "sma_crossover", day(7), day(7));
    let c = generate_card(&t, cap, &closes).await.expect("c");
    assert_eq!(c.note, None, "v1 must always emit note=None (Q1=A)");
}
