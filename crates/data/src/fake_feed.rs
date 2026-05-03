//! Scriptable in-memory feed for unit tests (T10).
//!
//! `FakeFeed` drives `MarketDataSource` from in-memory `Vec<Bar>` / `Vec<Tick>`.
//! `trade_aggregation` aggregates ticks into bars for cross-checking.

use async_trait::async_trait;
use futures::stream::BoxStream;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
#[cfg(test)]
use trading_core::Venue;
use trading_core::{Bar, FeedError, Price, Quantity, Symbol, Tick, Timeframe, Timestamp};

use crate::source::{MarketDataSource, SymbolInfo};

// ── FakeFeed ──────────────────────────────────────────────────────────────────

/// Scriptable in-memory feed for unit tests.
pub struct FakeFeed {
    pub bars: Vec<Bar>,
    pub ticks: Vec<Tick>,
}

impl FakeFeed {
    /// Create a `FakeFeed` with specific bars and ticks.
    #[must_use]
    pub fn new(bars: Vec<Bar>, ticks: Vec<Tick>) -> Self {
        Self { bars, ticks }
    }

    /// Create an empty `FakeFeed`.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(vec![], vec![])
    }
}

#[async_trait]
impl MarketDataSource for FakeFeed {
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError> {
        Ok(SymbolInfo {
            symbol: symbol.clone(),
            base_asset: "BTC".into(),
            quote_asset: "USDT".into(),
            min_qty: Decimal::new(1, 5),
            lot_size: Decimal::new(1, 5),
            min_notional: Decimal::new(10, 0),
        })
    }

    async fn subscribe_bars(
        &self,
        _symbol: Symbol,
        _tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        use futures::stream;
        let bars: Vec<Result<Bar, FeedError>> = self.bars.iter().cloned().map(Ok).collect();
        Ok(Box::pin(stream::iter(bars)))
    }

    async fn subscribe_trades(
        &self,
        _symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        use futures::stream;
        let ticks: Vec<Result<Tick, FeedError>> = self.ticks.iter().cloned().map(Ok).collect();
        Ok(Box::pin(stream::iter(ticks)))
    }
}

// ── Trade aggregation ─────────────────────────────────────────────────────────

/// Aggregate a slice of ticks into an OHLCV bar.
///
/// `open_ts` and `close_ts` are taken from the first and last tick respectively.
/// `tf` is passed through; `local_recv_ts` is set to `Timestamp::now()`.
///
/// Returns `None` if `ticks` is empty.
///
/// # Panics
///
/// Will not panic (all errors are handled via `Result`/`Option`).
#[must_use]
pub fn trade_aggregation(ticks: &[Tick], symbol: Symbol, tf: Timeframe) -> Option<Bar> {
    if ticks.is_empty() {
        return None;
    }

    let first = &ticks[0];
    let last = &ticks[ticks.len() - 1];

    let open = first.price.get();
    let mut high = first.price.get();
    let mut low = first.price.get();
    let mut close = first.price.get();
    let mut volume = dec!(0);

    for tick in ticks {
        let p = tick.price.get();
        // open stays as the first tick's price
        if p > high {
            high = p;
        }
        if p < low {
            low = p;
        }
        close = p;
        volume += tick.qty.get();
    }

    // These are already validated when the ticks were constructed, so
    // we use the values directly; fallback to first.price if somehow invalid.
    let mk_price = |d: Decimal, fallback: Price| Price::new(d).unwrap_or(fallback);

    Some(Bar {
        symbol,
        tf,
        open_ts: first.venue_ts,
        close_ts: last.venue_ts,
        open: mk_price(open, first.price),
        high: mk_price(high, first.price),
        low: mk_price(low, first.price),
        close: mk_price(close, last.price),
        volume: Quantity::new(volume).unwrap_or_else(|_| Quantity::zero()),
        trade_count: u32::try_from(ticks.len()).unwrap_or(u32::MAX),
        local_recv_ts: Timestamp::now(),
        // Aggregated bars inherit the venue of their constituent ticks.
        venue: first.venue,
    })
}

/// Compare a venue bar and a trade-aggregated bar, returning the max absolute
/// delta across OHLCV prices.  Used for cross-checking (R1.2).
///
/// Returns `None` if the bars have different symbols or timeframes.
#[must_use]
pub fn bar_cross_check_delta(venue: &Bar, aggregated: &Bar) -> Option<Decimal> {
    if venue.symbol != aggregated.symbol || venue.tf != aggregated.tf {
        return None;
    }
    let deltas = [
        (venue.open.get() - aggregated.open.get()).abs(),
        (venue.high.get() - aggregated.high.get()).abs(),
        (venue.low.get() - aggregated.low.get()).abs(),
        (venue.close.get() - aggregated.close.get()).abs(),
        (venue.volume.get() - aggregated.volume.get()).abs(),
    ];
    Some(*deltas.iter().max_by(|a, b| a.cmp(b)).unwrap_or(&dec!(0)))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Side, Timestamp};

    fn ts(offset_secs: i64) -> Timestamp {
        Timestamp::new(
            OffsetDateTime::from_unix_timestamp(1_700_000_000 + offset_secs).expect("valid ts"),
        )
    }

    fn make_tick(price: Decimal, qty: Decimal, t: i64) -> Tick {
        Tick {
            symbol: Symbol::new("BTCUSDT"),
            venue_ts: ts(t),
            local_recv_ts: ts(t),
            price: Price::new(price).expect("price ok"),
            qty: Quantity::new(qty).expect("qty ok"),
            side: Side::Buy,
            trade_id: u64::try_from(t).unwrap_or(0),
            venue: Venue::Binance,
        }
    }

    #[test]
    fn t10_trade_aggregation_ohlcv() {
        // Known ticks: prices 100, 120, 90, 110; quantities 1, 2, 3, 4
        let ticks = vec![
            make_tick(dec!(100), dec!(1), 0),
            make_tick(dec!(120), dec!(2), 1),
            make_tick(dec!(90), dec!(3), 2),
            make_tick(dec!(110), dec!(4), 3),
        ];
        let bar = trade_aggregation(&ticks, Symbol::new("BTCUSDT"), Timeframe::OneMinute)
            .expect("non-empty");

        assert_eq!(bar.open.get(), dec!(100), "open");
        assert_eq!(bar.high.get(), dec!(120), "high");
        assert_eq!(bar.low.get(), dec!(90), "low");
        assert_eq!(bar.close.get(), dec!(110), "close");
        assert_eq!(bar.volume.get(), dec!(10), "volume (1+2+3+4)");
        assert_eq!(bar.trade_count, 4, "trade_count");
    }

    #[test]
    fn t10_trade_aggregation_single_tick() {
        let ticks = vec![make_tick(dec!(50000), dec!(0.1), 0)];
        let bar = trade_aggregation(&ticks, Symbol::new("BTCUSDT"), Timeframe::OneMinute)
            .expect("non-empty");
        assert_eq!(bar.open.get(), dec!(50000));
        assert_eq!(bar.high.get(), dec!(50000));
        assert_eq!(bar.low.get(), dec!(50000));
        assert_eq!(bar.close.get(), dec!(50000));
    }

    #[test]
    fn t10_trade_aggregation_empty() {
        assert!(trade_aggregation(&[], Symbol::new("BTCUSDT"), Timeframe::OneMinute).is_none());
    }

    #[test]
    fn t10_bar_cross_check_within_satoshi() {
        let ticks = vec![
            make_tick(dec!(50000), dec!(1), 0),
            make_tick(dec!(50001), dec!(1), 1),
        ];
        let agg_bar =
            trade_aggregation(&ticks, Symbol::new("BTCUSDT"), Timeframe::OneMinute).expect("ok");
        // Venue bar identical to aggregated — delta should be 0
        let venue_bar = agg_bar.clone();
        let delta = bar_cross_check_delta(&venue_bar, &agg_bar).expect("same symbol+tf");
        assert!(delta <= dec!(0.00000001), "delta {delta} exceeds 1 satoshi");
    }
}
