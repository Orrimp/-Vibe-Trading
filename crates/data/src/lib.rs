//! Market data ingestion, storage, and replay.
//!
//! Exposes `MarketDataSource` trait with `BinanceFeed`, `ReplayFeed`, `FakeFeed`.

pub mod bar_stream;
pub mod binance;
pub mod clock_skew;
pub mod fake_feed;
pub mod replay_feed;
pub mod source;

pub use bar_stream::{bar_stream, bar_stream_with_cross_check};
pub use binance::BinanceFeed;
pub use clock_skew::{ClockSkewConfig, ClockSkewDetector, ObserveResult};
pub use fake_feed::{bar_cross_check_delta, trade_aggregation, FakeFeed};
pub use replay_feed::ReplayFeed;
pub use source::MarketDataSource;
