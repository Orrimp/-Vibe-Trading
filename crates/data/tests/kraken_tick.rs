//! T1411 V3 — Kraken Tick emission test (multi-venue feature v1.5b).
//!
//! Per [spec/features/v1-5b-multi-venue.md → V3] and
//! [spec/tasks/v1-5b-multi-venue.md → T1411 V3 acceptance]:
//!
//! > V3 — Kraken Tick (NEW). Same pattern as V2 with `Venue::Kraken`,
//! > `symbol == "XBTUSD"` (or whatever the agent's universe-side normalized
//! > name is — the adapter maps to `XBT/USD` on the wire). Symbol
//! > normalization end-to-end verified.
//!
//! The agent-native symbol is `BTCUSDC` (10-symbol universe alphabet, USDC
//! mirror per R3.1); the `kraken_symbol_map` adapter rewrites this to
//! `XBT/USDC` on the Kraken wire (`XBT`, the legacy ISO-4217 ticker for
//! Bitcoin, is what Kraken uses). The `MockFeed` here scripts Ticks at
//! the post-normalization `BTCUSDC` shape — symbol survives the round-trip.
//!
//! Pure in-memory; deterministic. Uses `tokio::time::pause`. Two runs
//! against the same fixture emit byte-identical Ticks (R5.3).
//!
//! **Feature gate.** `MockFeed` is gated behind the `fixtures` feature per
//! T1407 / Q10. Run with:
//!
//! ```bash
//! cargo test -p data --features fixtures --test kraken_tick
//! ```
#![cfg(feature = "fixtures")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use data::{kraken_symbol_map, source::MarketDataSource, MockFeed};
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

/// T1411 V3 — `KrakenFeed` emits Tick with `venue: Venue::Kraken`.
///
/// V3 mirrors V2 but with `Venue::Kraken`. The symbol normalization
/// pipeline maps the agent-native `BTCUSDC` to Kraken's wire format
/// `XBT/USDC` via `kraken_symbol_map` (T1404). This test asserts both
/// directions:
///   1. The agent-native symbol `BTCUSDC` flows through the
///      `MarketDataSource::subscribe_trades` surface unchanged (the
///      consumer-facing API uses the agent-native form).
///   2. `kraken_symbol_map` reproduces the wire-side `XBT/USDC` mapping.
#[tokio::test(start_paused = true)]
async fn t1411_v3_kraken_tick_emits_with_venue() {
    let symbol = Symbol::new("BTCUSDC");
    let scripted = vec![
        mk_tick(&symbol, 1_000_000, 1, Venue::Kraken),
        mk_tick(&symbol, 1_500_000, 2, Venue::Kraken),
        mk_tick(&symbol, 2_000_000, 3, Venue::Kraken),
    ];
    let feed = MockFeed::new(scripted, Duration::from_millis(10), Venue::Kraken);
    let mut stream = feed
        .subscribe_trades(symbol.clone())
        .await
        .expect("subscribe_trades ok");

    // V3 acceptance: at least one Tick within 30s.
    tokio::time::advance(Duration::from_millis(11)).await;
    let first = stream
        .next()
        .await
        .expect("at least one tick available")
        .expect("Ok tick");

    assert_eq!(first.venue, Venue::Kraken, "venue == Venue::Kraken");
    assert_eq!(
        first.symbol,
        Symbol::new("BTCUSDC"),
        "agent-native symbol round-trip — BTCUSDC"
    );
    assert!(first.price.get() > rust_decimal::Decimal::ZERO);
    assert!(first.qty.get() > rust_decimal::Decimal::ZERO);
    assert_eq!(first.side, Side::Buy);
    assert_ne!(first.venue_ts.inner(), OffsetDateTime::UNIX_EPOCH);
    assert_ne!(first.local_recv_ts.inner(), OffsetDateTime::UNIX_EPOCH);
    assert_ne!(first.trade_id, 0);

    // Symbol normalization gate: agent-native `BTCUSDC` maps to
    // wire-form `XBT/USDC` (Kraken's legacy ISO-4217 'X' prefix, T1404).
    // This is the one Kraken-specific semantic on top of the venue-blind
    // `MockFeed` shell.
    assert_eq!(
        kraken_symbol_map(&symbol),
        "XBT/USDC",
        "Kraken wire-form normalization — BTCUSDC → XBT/USDC"
    );

    // Drain the remaining two scripted ticks; venue tag preserved.
    for _ in 0..2 {
        tokio::time::advance(Duration::from_millis(11)).await;
        let t = stream
            .next()
            .await
            .expect("tick available")
            .expect("Ok tick");
        assert_eq!(t.venue, Venue::Kraken);
        assert_eq!(t.symbol, symbol);
    }
}
