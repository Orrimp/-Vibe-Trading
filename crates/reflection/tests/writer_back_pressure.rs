//! T1808 / R7.1 / Q8 — back-pressure smoke.
//!
//! Fill a 1024-capacity mpsc with synthetic `LessonCardWriteRequest`s;
//! assert the 1025th `try_enqueue` returns `Err(BackPressure)` and
//! the writer's `dropped_count` increments by 1.

use reflection::types::{ClosedTrade, LessonCardWriteRequest, SymbolOrPair};
use reflection::{ReflectionWriter, TryEnqueueError};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn synth_req(i: usize) -> LessonCardWriteRequest {
    LessonCardWriteRequest {
        closed_trade: ClosedTrade {
            close_transaction_id: format!("close-{i}"),
            open_transaction_id: format!("open-{i}"),
            symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
            strategy_id: StrategyId::new("sma_crossover"),
            signed_pnl: Money::<Usdt>::from_decimal(dec!(100)),
            closed_at: ts(1_700_000_000 + i as i64),
            opened_at: ts(1_700_000_000 + i as i64 - 86_400),
            holding_period_bars: 1440,
        },
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        btc_closes: Vec::new(),
    }
}

#[tokio::test]
async fn t1808_back_pressure_drop_counter_increments() {
    // for_test gives us a writer + a receiver we DON'T drain — the
    // bounded mpsc fills at the exact capacity boundary.
    let (writer, _rx) = ReflectionWriter::for_test(1024);

    // Fill the channel to capacity.
    for i in 0..1024 {
        writer.try_enqueue(synth_req(i)).expect("under capacity");
    }
    assert_eq!(writer.dropped_count(), 0, "no drops yet");

    // 1025th enqueue must hit back-pressure.
    let res = writer.try_enqueue(synth_req(9999));
    assert!(matches!(res, Err(TryEnqueueError::BackPressure)));
    assert_eq!(writer.dropped_count(), 1, "exactly one drop counted");

    // Subsequent failures keep counting.
    let res = writer.try_enqueue(synth_req(10000));
    assert!(matches!(res, Err(TryEnqueueError::BackPressure)));
    assert_eq!(writer.dropped_count(), 2);
}

#[tokio::test]
async fn t1808_closed_receiver_returns_closed_err() {
    let (writer, rx) = ReflectionWriter::for_test(8);
    drop(rx);
    let res = writer.try_enqueue(synth_req(0));
    assert!(matches!(res, Err(TryEnqueueError::Closed)));
    // Closed-channel drops do NOT count as back-pressure.
    assert_eq!(writer.dropped_count(), 0);
}
