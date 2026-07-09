//! Market data ingestion, storage, and replay.
//!
//! Exposes `MarketDataSource` trait with `BinanceFeed`, `ReplayFeed`, `FakeFeed`.
//! v1 adds `funding::FundingPoller` (T613).
//! C1 adds `synth` — Monte-Carlo stationary-block-bootstrap path generator.

/// Market-calendar layer — ADR-0073 D1.
///
/// Resolves a Yahoo ticker to its [`calendar::MarketCalendar`] and counts
/// trading days for the `load_cached` 95% coverage gate.  This is the
/// durable "v0.2.0 market-calendar layer" the `yahoo.rs` comment named.
/// Available unconditionally (not gated behind `yahoo` feature) so tests
/// and other callers can use `classify_ticker` without enabling parquet deps.
pub mod calendar;

pub mod bar_aggregator;
pub mod bar_stream;
pub mod binance;
pub mod binance_klines;
pub mod clock_skew;
pub mod coinbase;
pub mod coinbase_klines;
pub mod daily_volume;
pub mod dynamic_cache;
pub mod fake_feed;
pub mod funding;
pub mod kraken;
#[cfg(any(test, feature = "fixtures"))]
pub mod mock_feed;
pub mod replay_feed;
pub mod revision;
pub mod source;
pub mod synth;
#[cfg(feature = "yahoo")]
pub mod yahoo;

pub use bar_aggregator::{aggregate_one_second, aggregate_one_second_iter};
pub use bar_stream::{bar_stream, bar_stream_with_cross_check};
pub use binance::BinanceFeed;
pub use clock_skew::{ClockSkewConfig, ClockSkewDetector, ObserveResult};
pub use coinbase::{CoinbaseFeed, coinbase_symbol_map};
pub use coinbase_klines::{
    CoinbaseKlineFetcher, HttpCoinbaseKlineFetcher, build_coinbase_candles_url,
    coinbase_product_id_for_symbol, paginate_coinbase_candles, parse_coinbase_candle,
};
pub use daily_volume::{
    DailyVolumeError, daily_volume_usd_trailing, universe_avg_daily_volume_usd_trailing,
};
pub use fake_feed::{FakeFeed, bar_cross_check_delta, trade_aggregation};
pub use funding::{BinanceFundingClient, FundingPollError, FundingPoller};
pub use kraken::{KrakenFeed, kraken_symbol_map};
#[cfg(any(test, feature = "fixtures"))]
pub use mock_feed::MockFeed;
pub use replay_feed::ReplayFeed;
pub use source::MarketDataSource;
pub use synth::{
    BlockBootstrapPathGen, BlockLengthPolicy, GbmPathGen, GeneratedPath, MonteCarloPathGen,
    SynthError,
};
