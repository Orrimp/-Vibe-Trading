//! Bar stream with optional trade-aggregation cross-check (R1.2, T10).
//!
//! `bar_stream` wraps a raw venue bar stream and, if a tick collector is
//! provided, cross-checks each closed bar against the tick-aggregated OHLCV.
//! On mismatch > 1 satoshi a warning is emitted but the bar is still forwarded.

use futures::stream::BoxStream;
use futures::StreamExt;
use tracing::warn;
use trading_core::{Bar, FeedError, Symbol, Timeframe};

use crate::fake_feed::{bar_cross_check_delta, trade_aggregation};

/// Wrap a raw venue bar stream as a `BoxStream`.
/// In v0 this is a pass-through; the cross-check is wired when a tick
/// buffer is provided via `bar_stream_with_cross_check`.
#[must_use]
pub fn bar_stream(
    raw: BoxStream<'static, Result<Bar, FeedError>>,
) -> BoxStream<'static, Result<Bar, FeedError>> {
    raw
}

/// Wrap a bar stream and cross-check each bar against a trade-aggregated bar.
///
/// `tick_collector` is called for each bar and should return the ticks that
/// arrived within that bar's `[open_ts, close_ts]` interval.
///
/// If the OHLCV delta between venue bar and aggregated bar exceeds 1 satoshi
/// (`0.00000001`), a `WARN` log is emitted.  The venue bar is always forwarded.
pub fn bar_stream_with_cross_check<F>(
    raw: BoxStream<'static, Result<Bar, FeedError>>,
    tick_collector: F,
) -> BoxStream<'static, Result<Bar, FeedError>>
where
    F: Fn(&Bar) -> Vec<trading_core::Tick> + Send + 'static,
{
    use rust_decimal_macros::dec;

    let stream = raw.map(move |res| {
        if let Ok(bar) = &res {
            let ticks = tick_collector(bar);
            if !ticks.is_empty() {
                let symbol: Symbol = bar.symbol.clone();
                let tf: Timeframe = bar.tf;
                if let Some(agg) = trade_aggregation(&ticks, symbol, tf) {
                    if let Some(delta) = bar_cross_check_delta(bar, &agg) {
                        if delta > dec!(0.00000001) {
                            warn!(
                                symbol = %bar.symbol,
                                open_ts = %bar.open_ts,
                                delta = %delta,
                                "bar/trade-agg mismatch > 1 satoshi"
                            );
                        }
                    }
                }
            }
        }
        res
    });

    Box::pin(stream)
}
