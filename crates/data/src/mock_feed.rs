//! `MockFeed` — scriptable in-memory `MarketDataSource` for tests (T1407).
//!
//! Per Q10 of the v1.5b multi-venue feature brief: `wiremock` does not
//! script WS frames cleanly, and spinning a real `tokio_tungstenite`
//! WS server per test is slow / flaky in CI. `MockFeed` impls
//! `MarketDataSource` directly (no WS frame parsing); tests construct
//! a feed from a `Vec<Tick>` script + a `tokio::time::interval` and
//! consume the resulting stream like any live feed.
//!
//! Gated behind `#[cfg(any(test, feature = "fixtures"))]` so production
//! builds don't include this harness. Mirrors the architect's gating
//! intent for test scaffolding.
//!
//! Useful for V1–V7 strategy / integration tests; see T1411 (V1–V3),
//! T1413 (V6 multi-symbol), T1414 (V7 outage isolation).
#![cfg(any(test, feature = "fixtures"))]
use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::BoxStream;
use rust_decimal::Decimal;
use tokio::time;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;
use trading_core::{Bar, FeedError, Symbol, Tick, Timeframe, Venue};

use crate::source::{MarketDataSource, SymbolInfo};

/// Scripted in-memory feed that publishes `Tick` events on a fixed
/// interval. Implements `MarketDataSource` so any test can swap a real
/// venue for `MockFeed` without changing the consumer code path.
pub struct MockFeed {
    /// Per-symbol scripted tick events.  Used by both `subscribe_trades`
    /// (to emit ticks directly) and the symbol-info default.
    events: HashMap<Symbol, Vec<Tick>>,
    /// Interval between successive emitted events. Driven by the test's
    /// tokio runtime; can be paused / advanced via `tokio::time::pause()`.
    interval: Duration,
    /// Originating venue stamped on every emitted Tick.  Allows V14
    /// outage-isolation tests to construct a 3-venue scenario.
    venue: Venue,
}

impl MockFeed {
    /// Construct a single-symbol `MockFeed`.
    ///
    /// All input ticks must carry the same `Symbol`; the constructor
    /// does NOT validate this — tests are expected to pass a coherent
    /// fixture.
    #[must_use]
    pub fn new(events: Vec<Tick>, interval: Duration, venue: Venue) -> Self {
        let mut by_symbol: HashMap<Symbol, Vec<Tick>> = HashMap::new();
        for tick in events {
            by_symbol.entry(tick.symbol.clone()).or_default().push(tick);
        }
        Self {
            events: by_symbol,
            interval,
            venue,
        }
    }

    /// Construct a multi-symbol `MockFeed`. Each symbol's ticks form
    /// an independent script consumed by its own `subscribe_trades`
    /// call. Used by V6 multi-symbol fan-out tests.
    #[must_use]
    pub fn new_multi(events: HashMap<Symbol, Vec<Tick>>, interval: Duration, venue: Venue) -> Self {
        Self {
            events,
            interval,
            venue,
        }
    }
}

#[async_trait]
impl MarketDataSource for MockFeed {
    /// Hard-coded `SymbolInfo` for tests. Override semantics aren't
    /// needed for V1–V7; if a future test needs different limits, add
    /// a builder method on `MockFeed`.
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError> {
        Ok(SymbolInfo {
            symbol: symbol.clone(),
            base_asset: "BASE".into(),
            quote_asset: "QUOTE".into(),
            min_qty: Decimal::new(1, 3),  // 0.001
            lot_size: Decimal::new(1, 3), // 0.001
            min_notional: Decimal::new(10, 0),
        })
    }

    /// Emit `Bar`s aggregated from the scripted Ticks for the given
    /// symbol. Pacing is driven by `self.interval`. The output `tf` is
    /// passed through; the input event timestamps drive the actual
    /// bucket boundaries via the same algorithm as live feeds.
    ///
    /// Note: this implementation is intentionally simple — for
    /// determinism-critical tests, prefer `aggregate_one_second_iter`
    /// directly. The harness here exists to satisfy the
    /// `MarketDataSource` trait contract end-to-end.
    async fn subscribe_bars(
        &self,
        symbol: Symbol,
        tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        let ticks = self.events.get(&symbol).cloned().unwrap_or_default();
        let venue = self.venue;
        // Build the bars synchronously then drip them on the interval.
        let bars = crate::bar_aggregator::aggregate_one_second_iter(ticks, venue);
        // If a non-1s tf is requested, override the tf field so consumer
        // assertions match — the values are still correct OHLCVs of the
        // (possibly re-bucketed) input, but for non-1s tf consumers
        // should construct their own fixture.  The simple path here is
        // adequate for V1–V7 tests where tf == OneSecond.
        let bars: Vec<Bar> = bars
            .into_iter()
            .map(|mut b| {
                b.tf = tf;
                b
            })
            .collect();
        let interval = self.interval;
        let stream = async_stream::stream! {
            let mut tick_stream = IntervalStream::new(time::interval(interval));
            // Consume the very first interval tick (fires immediately
            // by default) so we don't emit a bar at t=0 ahead of the
            // intended pacing.
            let _ = tick_stream.next().await;
            for bar in bars {
                let _ = tick_stream.next().await;
                yield Ok(bar);
            }
        };
        Ok(Box::pin(stream))
    }

    /// Emit Ticks one-per-interval for the given symbol. The order is
    /// the order they were inserted into the script.
    async fn subscribe_trades(
        &self,
        symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        let ticks = self.events.get(&symbol).cloned().unwrap_or_default();
        let interval = self.interval;
        let stream = async_stream::stream! {
            let mut tick_stream = IntervalStream::new(time::interval(interval));
            // Drop the immediate first tick (Tokio's interval fires at
            // t=0 by default).
            let _ = tick_stream.next().await;
            for tick in ticks {
                let _ = tick_stream.next().await;
                yield Ok(tick);
            }
        };
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;
    use ::time::OffsetDateTime;
    use rust_decimal_macros::dec;
    use trading_core::{Price, Quantity, Side, Timestamp};

    fn mk_tick(symbol: &Symbol, ts_us: i64, id: u64, venue: Venue) -> Tick {
        let dt = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_us) * 1_000)
            .expect("valid timestamp");
        Tick {
            symbol: symbol.clone(),
            venue_ts: Timestamp::new(dt),
            local_recv_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
            price: Price::new(dec!(60000)).unwrap(),
            qty: Quantity::new(dec!(0.001)).unwrap(),
            side: Side::Buy,
            trade_id: id,
            venue,
        }
    }

    /// T1407 — scripted ticks emerge in script order on the trades stream.
    #[tokio::test(start_paused = true)]
    async fn t1407_mock_feed_emits_scripted_ticks_in_order() {
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
            .expect("subscribe ok");

        let mut received = Vec::new();
        for _ in 0..3 {
            // Advance the paused-tokio clock past the next interval tick.
            tokio::time::advance(Duration::from_millis(11)).await;
            // First poll consumes the deferred immediate tick; subsequent
            // polls return one Tick per advance.
            let item = stream.next().await.expect("item available");
            received.push(item.expect("ok"));
        }

        let ids: Vec<u64> = received.iter().map(|t| t.trade_id).collect();
        assert_eq!(ids, vec![1, 2, 3], "ticks emerge in script order");
        for t in &received {
            assert_eq!(t.venue, Venue::Coinbase);
            assert_eq!(t.symbol, symbol);
        }
    }

    /// `MockFeed::new` partitions ticks by symbol — a multi-symbol script
    /// works without `new_multi` if the input is heterogeneous.
    #[tokio::test(start_paused = true)]
    async fn t1407_mock_feed_partitions_by_symbol() {
        let btc = Symbol::new("BTCUSDC");
        let eth = Symbol::new("ETHUSDC");
        let scripted = vec![
            mk_tick(&btc, 1_000_000, 1, Venue::Coinbase),
            mk_tick(&eth, 1_000_000, 100, Venue::Coinbase),
            mk_tick(&btc, 2_000_000, 2, Venue::Coinbase),
        ];
        let feed = MockFeed::new(scripted, Duration::from_millis(10), Venue::Coinbase);

        // Take ETH's stream — should see only id=100.
        let mut eth_stream = feed.subscribe_trades(eth.clone()).await.expect("ok");
        tokio::time::advance(Duration::from_millis(11)).await;
        let t = eth_stream.next().await.expect("some").expect("ok");
        assert_eq!(t.symbol, eth);
        assert_eq!(t.trade_id, 100);
    }

    /// `new_multi` accepts a pre-partitioned map and behaves identically.
    #[tokio::test(start_paused = true)]
    async fn t1407_mock_feed_new_multi() {
        let btc = Symbol::new("BTCUSDC");
        let mut events: HashMap<Symbol, Vec<Tick>> = HashMap::new();
        events.insert(
            btc.clone(),
            vec![
                mk_tick(&btc, 1_000_000, 1, Venue::Kraken),
                mk_tick(&btc, 2_000_000, 2, Venue::Kraken),
            ],
        );
        let feed = MockFeed::new_multi(events, Duration::from_millis(10), Venue::Kraken);
        let mut stream = feed.subscribe_trades(btc.clone()).await.expect("ok");
        tokio::time::advance(Duration::from_millis(11)).await;
        let t1 = stream.next().await.expect("some").expect("ok");
        tokio::time::advance(Duration::from_millis(11)).await;
        let t2 = stream.next().await.expect("some").expect("ok");
        assert_eq!(t1.trade_id, 1);
        assert_eq!(t2.trade_id, 2);
        assert_eq!(t1.venue, Venue::Kraken);
    }

    /// `exchange_info` returns a hard-coded shape suitable for tests.
    #[tokio::test]
    async fn t1407_mock_feed_exchange_info() {
        let feed = MockFeed::new(vec![], Duration::from_millis(10), Venue::Binance);
        let info = feed
            .exchange_info(Symbol::new("BTCUSDT"))
            .await
            .expect("ok");
        assert_eq!(info.min_qty, dec!(0.001));
        assert_eq!(info.lot_size, dec!(0.001));
        assert_eq!(info.min_notional, Decimal::from(10));
    }
}
