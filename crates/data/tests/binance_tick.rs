//! T1411 V1 — Binance Tick regression test (multi-venue feature v1.5b).
//!
//! Per [spec/features/v1-5b-multi-venue.md → V1] and
//! [spec/tasks/v1-5b-multi-venue.md → T1411 V1 acceptance]:
//!
//! > V1 — Binance feed regression. Existing single-venue Binance feed tests
//! > pass unchanged. The new `subscribe_*_multi` methods are tested in T1413;
//! > the single-symbol path stays unchanged.
//!
//! This file is the architect-designed regression smoke: a `MockFeed` stamped
//! with `Venue::Binance` emits scripted Ticks through the
//! `MarketDataSource::subscribe_trades` surface. Asserts the venue field
//! survives the round-trip (the v0–v1.5a Binance code path is exercised by
//! the in-crate `binance::*` unit tests; this integration test is a
//! defense-in-depth gate that the venue tag is honoured end-to-end at the
//! `MarketDataSource` boundary).
//!
//! Pure in-memory; no wall-clock dependency (uses `tokio::time::pause` /
//! `advance`). Determinism (R5.3): two runs against the same fixture emit
//! byte-identical Ticks.
//!
//! **Feature gate.** Uses `MockFeed` from `crates/data/src/mock_feed.rs`,
//! which is itself gated behind `#[cfg(any(test, feature = "fixtures"))]`
//! per Q10 / T1407. Run with:
//!
//! ```bash
//! cargo test -p data --features fixtures --test binance_tick
//! ```
#![cfg(feature = "fixtures")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use data::{source::MarketDataSource, MockFeed};
use futures::StreamExt;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Price, Quantity, Side, Symbol, Tick, Timestamp, Venue};

fn mk_tick(symbol: &Symbol, ts_us: i64, id: u64, venue: Venue) -> Tick {
    let dt = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_us) * 1_000)
        .expect("valid timestamp");
    Tick {
        symbol: symbol.clone(),
        venue_ts: Timestamp::new(dt),
        local_recv_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
        price: Price::new(dec!(60000.50)).unwrap(),
        qty: Quantity::new(dec!(0.001)).unwrap(),
        side: Side::Buy,
        trade_id: id,
        venue,
    }
}

/// T1411 V1 — Binance feed still emits Tick events with `venue: Venue::Binance`.
///
/// Uses `MockFeed` (the v1.5b test harness per Q10) to keep CI deterministic;
/// the live-WS Binance path is exercised by the existing
/// `binance_ws_integration` test (gated `#[ignore]`).
#[tokio::test(start_paused = true)]
async fn t1411_v1_binance_tick_regression() {
    let symbol = Symbol::new("BTCUSDT");
    let scripted = vec![
        mk_tick(&symbol, 1_000_000, 1, Venue::Binance),
        mk_tick(&symbol, 1_500_000, 2, Venue::Binance),
        mk_tick(&symbol, 2_000_000, 3, Venue::Binance),
    ];
    let feed = MockFeed::new(scripted, Duration::from_millis(10), Venue::Binance);
    let mut stream = feed
        .subscribe_trades(symbol.clone())
        .await
        .expect("subscribe_trades ok");

    let mut received: Vec<Tick> = Vec::new();
    for _ in 0..3 {
        // Advance the paused-tokio clock past the next interval tick.
        tokio::time::advance(Duration::from_millis(11)).await;
        let item = stream.next().await.expect("item available");
        received.push(item.expect("Ok tick"));
    }

    assert_eq!(received.len(), 3, "three scripted ticks emerge");
    let ids: Vec<u64> = received.iter().map(|t| t.trade_id).collect();
    assert_eq!(ids, vec![1, 2, 3], "ticks emerge in script order");

    for t in &received {
        // V1 acceptance: the `venue` field is `Venue::Binance` on every Tick
        // emitted from a Binance-stamped feed.
        assert_eq!(t.venue, Venue::Binance, "venue survives round-trip");
        assert_eq!(t.symbol, symbol, "symbol round-trips");
        assert!(t.price.get() > rust_decimal::Decimal::ZERO);
        assert!(t.qty.get() > rust_decimal::Decimal::ZERO);
        assert_eq!(t.side, Side::Buy);
        assert_ne!(t.trade_id, 0, "trade_id non-zero per V1/V2/V3 contract");
    }
}
