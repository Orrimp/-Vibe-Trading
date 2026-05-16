//! Client-side 1-second bar aggregator (T1406).
//!
//! Per Q5 of the v1.5b multi-venue feature brief: each venue's "1s bar"
//! is either undefined (Coinbase / Kraken don't expose 1s candles) or
//! has its own quirks (Binance's 1s WS is new-ish). Client-side
//! aggregation gives an **identical** algorithm across all venues.
//!
//! Bucketing key: `floor(tick.venue_ts.unix_micros() / 1_000_000)` —
//! deterministic on epoch microseconds. Empty seconds emit no bar
//! (R5.3). Two replays of the same Tick fixture emit byte-identical
//! Bars.
//!
//! Public surface:
//!
//! - `aggregate_one_second(stream, symbol, venue) -> stream` — async
//!   adapter from a `BoxStream<Result<Tick>>` to a
//!   `BoxStream<Result<Bar>>` with `tf == Timeframe::OneSecond`. Used
//!   by the live ingest path.
//! - `aggregate_one_second_iter(iter)` — pure synchronous variant
//!   that consumes any `Iterator<Item = Tick>` and returns a
//!   `Vec<Bar>`. Used by deterministic unit tests / fixtures.
//!
//! Both paths share the same per-second state machine and produce
//! identical Bars given identical Tick sequences.
use futures::stream::BoxStream;
use futures::StreamExt;
use rust_decimal::Decimal;
use tracing::warn;
use trading_core::{Bar, FeedError, Price, Quantity, Symbol, Tick, Timeframe, Timestamp, Venue};

/// Per-second OHLCV accumulator.
struct Bucket {
    /// Floor(venue_ts_us / 1e6). Identifies the 1-second bucket.
    second: i64,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
    trade_count: u32,
    /// Symbol of the first tick in this bucket — propagated to the bar.
    /// All ticks in a bucket are expected to share the symbol passed to
    /// the aggregator constructor; defensively recorded here.
    symbol: Symbol,
}

impl Bucket {
    fn from_tick(tick: &Tick, second: i64) -> Self {
        let p = tick.price.get();
        Self {
            second,
            open: p,
            high: p,
            low: p,
            close: p,
            volume: tick.qty.get(),
            trade_count: 1,
            symbol: tick.symbol.clone(),
        }
    }

    fn update(&mut self, tick: &Tick) {
        let p = tick.price.get();
        if p > self.high {
            self.high = p;
        }
        if p < self.low {
            self.low = p;
        }
        self.close = p;
        self.volume += tick.qty.get();
        self.trade_count += 1;
    }
}

/// Compute the 1-second bucket key for a `Tick.venue_ts`.
///
/// Bucket = `floor(venue_ts_micros / 1_000_000)` — identical to
/// `unix_seconds()` for non-negative timestamps.
fn bucket_second(ts: Timestamp) -> i64 {
    // `unix_timestamp_nanos` is i128 and safely covers 9999-12-31.  We
    // floor-divide by 1_000_000_000 to get whole UTC seconds.  Negative
    // timestamps (pre-epoch fixtures, unusual) floor toward -infinity to
    // keep monotone ordering — `i128::div_euclid` handles that.
    let nanos = ts.inner().unix_timestamp_nanos();
    let secs = nanos.div_euclid(1_000_000_000);
    i64::try_from(secs).unwrap_or(i64::MAX)
}

/// Convert a 1-second bucket key back to (open_ts, close_ts) microseconds.
///
/// `open_ts = second * 1_000_000` (microseconds since epoch),
/// `close_ts = open_ts + 999_999` (matches the existing 1m convention:
/// `open_ts + interval - 1µs`).
fn bucket_to_timestamps(second: i64) -> (Timestamp, Timestamp) {
    // Convert seconds → nanos.
    let open_nanos = i128::from(second) * 1_000_000_000;
    let close_nanos = open_nanos + 999_999_000;
    let open_dt = time::OffsetDateTime::from_unix_timestamp_nanos(open_nanos)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let close_dt = time::OffsetDateTime::from_unix_timestamp_nanos(close_nanos)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    (Timestamp::new(open_dt), Timestamp::new(close_dt))
}

/// Build a closed `Bar` from a fully-accumulated `Bucket`.
///
/// `local_recv_ts` is set to the bucket close time (deterministic — no
/// `SystemTime::now()`). `venue` is propagated from the caller.
fn finalize_bucket(bucket: &Bucket, venue: Venue) -> Result<Bar, FeedError> {
    let (open_ts, close_ts) = bucket_to_timestamps(bucket.second);
    let open = Price::new(bucket.open).map_err(|e| FeedError::Parse(e.to_string()))?;
    let high = Price::new(bucket.high).map_err(|e| FeedError::Parse(e.to_string()))?;
    let low = Price::new(bucket.low).map_err(|e| FeedError::Parse(e.to_string()))?;
    let close = Price::new(bucket.close).map_err(|e| FeedError::Parse(e.to_string()))?;
    let volume = Quantity::new(bucket.volume).map_err(|e| FeedError::Parse(e.to_string()))?;
    Ok(Bar {
        symbol: bucket.symbol.clone(),
        tf: Timeframe::OneSecond,
        open_ts,
        close_ts,
        open,
        high,
        low,
        close,
        volume,
        trade_count: bucket.trade_count,
        local_recv_ts: close_ts,
        venue,
    })
}

/// Pure (synchronous) form of the 1s aggregator. Consumes any iterator of
/// `Tick`s and returns the resulting `Bar`s.
///
/// **Determinism (R5.3).** Pure integer arithmetic on `i64` epoch
/// microseconds + `Decimal` math. Two replays of the same Tick stream
/// emit byte-identical Bars. No `f64`. No `SystemTime::now()`.
///
/// Out-of-order ticks (`tick_second < current_bucket.second`) are
/// dropped with a `warn!` — this should not happen on a single venue's
/// ordered stream, but the behaviour is defined explicitly to make the
/// determinism guarantee total.
///
/// Empty seconds emit no bar (R5.3): the stream `[1.0@s=0, 1.0@s=2]`
/// produces 2 bars (s=0 and s=2), not 3.
pub fn aggregate_one_second_iter<I: IntoIterator<Item = Tick>>(ticks: I, venue: Venue) -> Vec<Bar> {
    let mut bars: Vec<Bar> = Vec::new();
    let mut current: Option<Bucket> = None;
    for tick in ticks {
        let s = bucket_second(tick.venue_ts);
        match current.as_mut() {
            None => {
                current = Some(Bucket::from_tick(&tick, s));
            }
            Some(b) if b.second == s => {
                b.update(&tick);
            }
            Some(b) if s < b.second => {
                warn!(
                    bucket = b.second,
                    tick_second = s,
                    "out-of-order tick; dropping"
                );
            }
            Some(_b) => {
                // s > b.second — flush the current bucket and start fresh.
                let prev = current.take().expect("current is Some");
                if let Ok(bar) = finalize_bucket(&prev, venue) {
                    bars.push(bar);
                }
                current = Some(Bucket::from_tick(&tick, s));
            }
        }
    }
    if let Some(prev) = current.take()
        && let Ok(bar) = finalize_bucket(&prev, venue)
    {
        bars.push(bar);
    }
    bars
}

/// Async stream adapter over a `BoxStream<Result<Tick>>`. Per Q5: takes
/// a raw Tick stream from any `MarketDataSource` impl and emits closed
/// 1-second `Bar`s aligned to UTC second boundaries.
///
/// Errors in the input stream are forwarded to the output stream.
/// Tick parse errors do NOT corrupt the per-second accumulator: only
/// `Ok(tick)` items advance the state machine.
///
/// `symbol` is captured for the resulting Bars; the input ticks SHOULD
/// already match this symbol (the aggregator is constructed per-stream).
/// `venue` is propagated to every emitted Bar.
#[must_use]
pub fn aggregate_one_second(
    ticks: BoxStream<'static, Result<Tick, FeedError>>,
    symbol: Symbol,
    venue: Venue,
) -> BoxStream<'static, Result<Bar, FeedError>> {
    let stream = async_stream::stream! {
        let mut current: Option<Bucket> = None;
        let mut input = ticks;
        while let Some(item) = input.next().await {
            match item {
                Err(e) => { yield Err(e); continue; }
                Ok(mut tick) => {
                    // Defensively normalize the symbol on every tick to
                    // match the aggregator's contract.
                    tick.symbol = symbol.clone();
                    let s = bucket_second(tick.venue_ts);
                    match current.as_mut() {
                        None => { current = Some(Bucket::from_tick(&tick, s)); }
                        Some(b) if b.second == s => { b.update(&tick); }
                        Some(b) if s < b.second => {
                            warn!(bucket = b.second, tick_second = s, "out-of-order tick; dropping");
                        }
                        Some(_b) => {
                            let prev = current.take().expect("current is Some");
                            yield finalize_bucket(&prev, venue);
                            current = Some(Bucket::from_tick(&tick, s));
                        }
                    }
                }
            }
        }
        if let Some(prev) = current.take() {
            yield finalize_bucket(&prev, venue);
        }
    };
    Box::pin(stream)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::str::FromStr;
    use trading_core::Side;

    fn mk_tick(symbol: &Symbol, ts_micros: i64, price: Decimal, qty: Decimal, id: u64) -> Tick {
        let dt = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_micros) * 1_000)
            .expect("valid timestamp");
        Tick {
            symbol: symbol.clone(),
            venue_ts: Timestamp::new(dt),
            local_recv_ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
            price: Price::new(price).unwrap(),
            qty: Quantity::new(qty).unwrap(),
            side: Side::Buy,
            trade_id: id,
            venue: Venue::Binance,
        }
    }

    /// V5 — synthetic stream of 60 ticks at 100ms intervals → 6 bars.
    #[test]
    fn t1406_v5_synthetic_stream_aggregates_to_n_bars() {
        let symbol = Symbol::new("BTCUSDT");
        let mut ticks = Vec::new();
        // 60 ticks across 6 seconds (10 ticks per second), starting at s=0.
        for i in 0..60_i64 {
            // Stride: 100ms = 100_000 microseconds.
            let ts_us = i * 100_000;
            let price = Decimal::from(60_000) + Decimal::from(i);
            ticks.push(mk_tick(&symbol, ts_us, price, dec!(0.001), i as u64));
        }
        let bars = aggregate_one_second_iter(ticks, Venue::Binance);
        assert_eq!(bars.len(), 6, "expected 6 bars from 60 ticks across 6s");
        // First bar covers s=0 → tick prices 60000..60009. open=60000, close=60009,
        // high=60009, low=60000.
        let b0 = &bars[0];
        assert_eq!(b0.tf, Timeframe::OneSecond);
        assert_eq!(b0.venue, Venue::Binance);
        assert_eq!(b0.symbol, symbol);
        assert_eq!(b0.open.get(), Decimal::from(60_000));
        assert_eq!(b0.high.get(), Decimal::from(60_009));
        assert_eq!(b0.low.get(), Decimal::from(60_000));
        assert_eq!(b0.close.get(), Decimal::from(60_009));
        assert_eq!(b0.trade_count, 10);
        assert_eq!(b0.volume.get(), dec!(0.01)); // 10 × 0.001
                                                 // Each subsequent bar has trade_count == 10.
        for b in &bars {
            assert_eq!(b.trade_count, 10);
        }
        // Bars are in ascending `open_ts` order with 1s strides.
        for w in bars.windows(2) {
            let delta_us = w[1].open_ts.inner().unix_timestamp_nanos() / 1_000
                - w[0].open_ts.inner().unix_timestamp_nanos() / 1_000;
            assert_eq!(delta_us, 1_000_000);
        }
    }

    /// V5 — empty seconds emit no bar (R5.3).
    #[test]
    fn t1406_empty_seconds_emit_no_bar() {
        let symbol = Symbol::new("BTCUSDT");
        let ticks = vec![
            mk_tick(&symbol, 0, dec!(60000), dec!(0.001), 1),
            // skip s=1 entirely
            mk_tick(&symbol, 2_500_000, dec!(60100), dec!(0.001), 2),
        ];
        let bars = aggregate_one_second_iter(ticks, Venue::Binance);
        assert_eq!(bars.len(), 2, "two non-empty seconds → two bars");
        // Bars are at s=0 and s=2.
        let s0 = bars[0].open_ts.inner().unix_timestamp_nanos() / 1_000_000_000;
        let s1 = bars[1].open_ts.inner().unix_timestamp_nanos() / 1_000_000_000;
        assert_eq!(s0, 0);
        assert_eq!(s1, 2);
    }

    /// Determinism — same input twice → identical output.
    #[test]
    fn t1406_aggregator_is_deterministic() {
        let symbol = Symbol::new("BTCUSDT");
        let make_input = || {
            let mut v = Vec::new();
            for i in 0..120_i64 {
                let ts_us = i * 50_000; // 50ms stride → 6s × 20 ticks
                let price = Decimal::from(60_000) + Decimal::from(i % 13);
                v.push(mk_tick(&symbol, ts_us, price, dec!(0.001), i as u64));
            }
            v
        };
        let bars_a = aggregate_one_second_iter(make_input(), Venue::Binance);
        let bars_b = aggregate_one_second_iter(make_input(), Venue::Binance);
        assert_eq!(bars_a.len(), bars_b.len());
        // Compare every field — Decimal exact, Timestamp exact, etc.
        for (a, b) in bars_a.iter().zip(bars_b.iter()) {
            assert_eq!(a.symbol, b.symbol);
            assert_eq!(a.tf, b.tf);
            assert_eq!(a.open_ts, b.open_ts);
            assert_eq!(a.close_ts, b.close_ts);
            assert_eq!(a.open.get(), b.open.get());
            assert_eq!(a.high.get(), b.high.get());
            assert_eq!(a.low.get(), b.low.get());
            assert_eq!(a.close.get(), b.close.get());
            assert_eq!(a.volume.get(), b.volume.get());
            assert_eq!(a.trade_count, b.trade_count);
            assert_eq!(a.venue, b.venue);
        }
    }

    /// Out-of-order ticks are dropped, bucket boundary unchanged.
    #[test]
    fn t1406_drops_out_of_order_ticks() {
        let symbol = Symbol::new("BTCUSDT");
        // Build a stream that goes s=2, then back to s=1 (should drop), then s=2.
        let ticks = vec![
            mk_tick(&symbol, 2_000_000, dec!(60000), dec!(0.001), 1),
            // Out-of-order — must not retroactively create an s=1 bar.
            mk_tick(&symbol, 1_500_000, dec!(60050), dec!(0.001), 2),
            mk_tick(&symbol, 2_500_000, dec!(60100), dec!(0.001), 3),
        ];
        let bars = aggregate_one_second_iter(ticks, Venue::Binance);
        assert_eq!(bars.len(), 1);
        let b = &bars[0];
        assert_eq!(b.trade_count, 2); // first + third only
        assert_eq!(b.open.get(), Decimal::from(60_000));
        assert_eq!(b.close.get(), Decimal::from(60_100));
    }

    /// Bucket boundary math: open_ts at exact UTC second, close_ts at +999_999µs.
    #[test]
    fn t1406_bucket_to_timestamps_alignment() {
        let (open_ts, close_ts) = bucket_to_timestamps(1_714_579_200);
        let open_us = open_ts.inner().unix_timestamp_nanos() / 1_000;
        let close_us = close_ts.inner().unix_timestamp_nanos() / 1_000;
        assert_eq!(open_us, 1_714_579_200_000_000);
        assert_eq!(close_us, 1_714_579_200_999_999);
    }

    /// `bucket_second` is `floor(unix_micros / 1_000_000)`.
    #[test]
    fn t1406_bucket_key_floor() {
        // 1.999s → bucket 1.
        let dt = time::OffsetDateTime::from_unix_timestamp_nanos(1_999_000_000).unwrap();
        assert_eq!(bucket_second(Timestamp::new(dt)), 1);
        // 0us → bucket 0.
        let dt0 = time::OffsetDateTime::from_unix_timestamp_nanos(0).unwrap();
        assert_eq!(bucket_second(Timestamp::new(dt0)), 0);
        // 999_999us → bucket 0.
        let dt999 = time::OffsetDateTime::from_unix_timestamp_nanos(999_999_000).unwrap();
        assert_eq!(bucket_second(Timestamp::new(dt999)), 0);
    }

    /// Async path matches sync path exactly.
    #[tokio::test]
    async fn t1406_async_path_matches_sync_path() {
        use futures::stream;
        let symbol = Symbol::new("ETHUSDT");
        let mut ticks = Vec::new();
        for i in 0..30_i64 {
            ticks.push(mk_tick(
                &symbol,
                i * 100_000,
                Decimal::from_str("3000.00").unwrap() + Decimal::from(i),
                dec!(0.5),
                i as u64,
            ));
        }
        let sync_bars = aggregate_one_second_iter(ticks.clone(), Venue::Coinbase);
        let async_input: BoxStream<'static, Result<Tick, FeedError>> =
            Box::pin(stream::iter(ticks.into_iter().map(Ok)));
        let async_stream = aggregate_one_second(async_input, symbol.clone(), Venue::Coinbase);
        let async_bars: Vec<Bar> = async_stream
            .collect::<Vec<Result<Bar, FeedError>>>()
            .await
            .into_iter()
            .map(|r| r.expect("ok"))
            .collect();
        assert_eq!(sync_bars.len(), async_bars.len());
        for (a, b) in sync_bars.iter().zip(async_bars.iter()) {
            assert_eq!(a.open.get(), b.open.get());
            assert_eq!(a.close.get(), b.close.get());
            assert_eq!(a.trade_count, b.trade_count);
            assert_eq!(a.venue, b.venue);
            assert_eq!(a.tf, b.tf);
        }
    }
}
