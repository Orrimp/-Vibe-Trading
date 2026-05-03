//! T1411 V2 — Coinbase Tick emission test (multi-venue feature v1.5b).
//!
//! Per [spec/features/v1-5b-multi-venue.md → V2] and
//! [spec/tasks/v1-5b-multi-venue.md → T1411 V2 acceptance]:
//!
//! > V2 — Coinbase Tick (NEW). Constructs a `MockFeed::new(scripted_coinbase_ticks,
//! > Duration::from_millis(10), Venue::Coinbase)` (the mock is venue-agnostic; the
//! > test scripts the venue field). Asserts `Tick { venue: Venue::Coinbase,
//! > symbol == "BTCUSDC", price > 0, qty > 0, side == Side::Buy,
//! > venue_ts.is_some(), local_recv_ts.is_some(), trade_id != 0 }` within 30s.
//!
//! Pure in-memory; deterministic. Uses `tokio::time::pause` so the test
//! does not actually wait 30s of wall-clock — it advances the paused
//! virtual clock past the `MockFeed`'s 10ms interval. Two runs against
//! the same fixture emit byte-identical Ticks (R5.3).
//!
//! **Feature gate.** `MockFeed` is gated behind the `fixtures` feature per
//! T1407 / Q10. Run with:
//!
//! ```bash
//! cargo test -p data --features fixtures --test coinbase_tick
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
    let recv_dt = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_us + 1_000) * 1_000)
        .expect("valid timestamp");
    Tick {
        symbol: symbol.clone(),
        venue_ts: Timestamp::new(dt),
        local_recv_ts: Timestamp::new(recv_dt),
        price: Price::new(dec!(60000.00)).unwrap(),
        qty: Quantity::new(dec!(0.001)).unwrap(),
        side: Side::Buy,
        trade_id: id,
        venue,
    }
}

/// T1411 V2 — `CoinbaseFeed` emits Tick with `venue: Venue::Coinbase`.
///
/// V2's spec talks about a live `CoinbaseFeed::subscribe_trades(...)`
/// emitting at least one Tick within 30s; the architect's design (per
/// T1411 acceptance) replaces the live WS dependency with a `MockFeed`
/// fixture (Q10) so the test is hermetic. The on-wire JSON parser unit
/// test (`coinbase::tests::t1403_parses_market_trades_event_to_tick`)
/// proves the same Tick-shape end-to-end on real Coinbase wire data;
/// this file proves the `MarketDataSource` surface returns Ticks with
/// the correct venue tag.
#[tokio::test(start_paused = true)]
async fn t1411_v2_coinbase_tick_emits_with_venue() {
    let symbol = Symbol::new("BTCUSDC");
    let scripted = vec![
        mk_tick(&symbol, 1_000_000, 1, Venue::Coinbase),
        mk_tick(&symbol, 1_500_000, 2, Venue::Coinbase),
        mk_tick(&symbol, 2_000_000, 3, Venue::Coinbase),
    ];
    let feed = MockFeed::new(scripted, Duration::from_millis(10), Venue::Coinbase);
    let mut stream = feed
        .subscribe_trades(symbol.clone())
        .await
        .expect("subscribe_trades ok");

    // V2 acceptance: at least one Tick within 30s. Advance the paused
    // tokio clock past the 10ms interval and consume a tick.
    tokio::time::advance(Duration::from_millis(11)).await;
    let first = stream
        .next()
        .await
        .expect("at least one tick available")
        .expect("Ok tick");

    // Field-by-field acceptance (V2):
    assert_eq!(first.venue, Venue::Coinbase, "venue == Venue::Coinbase");
    assert_eq!(
        first.symbol,
        Symbol::new("BTCUSDC"),
        "symbol round-trip — BTCUSDC"
    );
    assert!(first.price.get() > rust_decimal::Decimal::ZERO, "price > 0");
    assert!(first.qty.get() > rust_decimal::Decimal::ZERO, "qty > 0");
    assert_eq!(first.side, Side::Buy, "side == Side::Buy");
    // `Timestamp` is non-Optional and always populated by construction;
    // V2's "is_some()" requirement maps to "non-default" — assert the
    // venue_ts and local_recv_ts are not the UNIX_EPOCH default sentinel.
    assert_ne!(
        first.venue_ts.inner(),
        OffsetDateTime::UNIX_EPOCH,
        "venue_ts populated (non-epoch)"
    );
    assert_ne!(
        first.local_recv_ts.inner(),
        OffsetDateTime::UNIX_EPOCH,
        "local_recv_ts populated (non-epoch)"
    );
    assert_ne!(first.trade_id, 0, "trade_id non-zero");

    // Drain the remaining two scripted ticks; assert the venue tag is
    // honoured for the entire stream (defense-in-depth).
    for _ in 0..2 {
        tokio::time::advance(Duration::from_millis(11)).await;
        let t = stream
            .next()
            .await
            .expect("tick available")
            .expect("Ok tick");
        assert_eq!(t.venue, Venue::Coinbase);
        assert_eq!(t.symbol, symbol);
    }
}
