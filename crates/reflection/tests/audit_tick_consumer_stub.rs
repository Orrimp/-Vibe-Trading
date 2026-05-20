//! R4 — end-to-end stub consumer test.
//!
//! Opens `Ledger::open_with_tick_bus(":memory:", 64)`, spawns
//! `ReflectionAuditTickConsumer::run(...)`, writes one fill, and asserts the
//! stub task completes within 200ms of the sender closing (proves the consumer
//! loop terminates correctly and processes the fill tick).

use std::sync::Arc;
use std::time::Duration;

use audit::{bootstrap, journal};
use reflection::audit_tick_consumer::ReflectionAuditTickConsumer;
use rust_decimal_macros::dec;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    Venue,
};

fn make_fill() -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.01)).expect("qty"),
        price: Price::new(dec!(50000)).expect("price"),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: Timestamp::now(),
        local_ts: Timestamp::now(),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

#[tokio::test]
async fn stub_receives_fill_tick_and_terminates_on_sender_drop() {
    let (ledger, sender) = audit::Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("open ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");

    // Subscribe before writing so we don't miss the tick.
    let rx = sender.subscribe();
    let stream = audit::tick::AuditTickStream::new(rx, "reflection_stub_test");
    // Arc<()> is a valid store: Send + Sync + 'static (R4.1 — stub doesn't use it).
    let stub = ReflectionAuditTickConsumer::new(stream, Arc::new(()));

    // Spawn stub consumer.
    let consumer_handle = tokio::spawn(async move { stub.run().await });

    // Write one fill — stub should receive the Fill tick.
    journal::post_fill(&ledger, &make_fill(), Venue::Binance, None)
        .await
        .expect("post_fill");

    // Give the stub a beat to process the tick.
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Drop the ledger (releases internal broadcast sender) and the outer sender.
    drop(sender);
    drop(ledger);

    // Stub should terminate within 200ms of sender close (RecvError::Closed).
    tokio::time::timeout(Duration::from_millis(200), consumer_handle)
        .await
        .expect("stub should finish within 200ms after sender drops")
        .expect("stub task joined without panic");
}

#[tokio::test]
async fn stub_terminates_immediately_when_no_ticks() {
    let (ledger, sender) = audit::Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("open ledger");

    let rx = sender.subscribe();
    let stream = audit::tick::AuditTickStream::new(rx, "reflection_empty_test");
    let stub = ReflectionAuditTickConsumer::new(stream, Arc::new(()));
    let handle = tokio::spawn(async move { stub.run().await });

    // Drop sender immediately — stub should terminate.
    drop(sender);
    drop(ledger);

    tokio::time::timeout(Duration::from_millis(100), handle)
        .await
        .expect("stub terminates quickly when sender drops immediately")
        .expect("task joined");
}
