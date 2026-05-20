//! K4 — `run_id` correctness with concurrent backtests.
//!
//! Opens one `Ledger::open_with_tick_bus(":memory:", 64)`, clones via
//! `with_run_id(uuid_a)` and `with_run_id(uuid_b)`, writes one fill on each
//! clone, and asserts the two ticks on a single subscriber carry the two
//! distinct uuids.

use audit::{bootstrap, journal};
use rust_decimal_macros::dec;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    Venue,
};
use uuid::Uuid;

fn make_fill(price: u64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.01)).expect("qty"),
        price: Price::new(rust_decimal::Decimal::from(price)).expect("price"),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: Timestamp::now(),
        local_ts: Timestamp::now(),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

#[tokio::test]
async fn with_run_id_stamps_distinct_ids_per_clone() {
    let (base_ledger, sender) = audit::Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("open");
    bootstrap::chart_of_accounts(&base_ledger)
        .await
        .expect("bootstrap");

    let uuid_a = Uuid::new_v4();
    let uuid_b = Uuid::new_v4();
    assert_ne!(uuid_a, uuid_b);

    let ledger_a = base_ledger.with_run_id(uuid_a);
    let ledger_b = base_ledger.with_run_id(uuid_b);

    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "run_id_test");

    // Write one fill on each clone.
    journal::post_fill(&ledger_a, &make_fill(50_000), Venue::Binance, None)
        .await
        .expect("fill a");
    journal::post_fill(&ledger_b, &make_fill(51_000), Venue::Binance, None)
        .await
        .expect("fill b");

    let tick_a = rx.next().await.expect("tick a");
    let tick_b = rx.next().await.expect("tick b");

    assert_eq!(tick_a.context.run_id, uuid_a, "tick_a should have uuid_a");
    assert_eq!(tick_b.context.run_id, uuid_b, "tick_b should have uuid_b");
    assert_ne!(
        tick_a.context.run_id, tick_b.context.run_id,
        "distinct clones must produce distinct run_ids"
    );
}

#[tokio::test]
async fn base_ledger_run_id_is_nil() {
    let (base_ledger, sender) = audit::Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("open");
    bootstrap::chart_of_accounts(&base_ledger)
        .await
        .expect("bootstrap");

    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "nil_test");

    journal::post_fill(&base_ledger, &make_fill(50_000), Venue::Binance, None)
        .await
        .expect("fill");

    let tick = rx.next().await.expect("tick");
    assert_eq!(
        tick.context.run_id,
        Uuid::nil(),
        "base ledger (no with_run_id) should have nil run_id"
    );
}
