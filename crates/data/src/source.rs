//! `MarketDataSource` trait.
use async_trait::async_trait;
use futures::stream::BoxStream;
use trading_core::{Bar, FeedError, Symbol, Tick, Timeframe};

/// Exchange metadata for a symbol.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub symbol: Symbol,
    pub base_asset: String,
    pub quote_asset: String,
    pub min_qty: rust_decimal::Decimal,
    pub lot_size: rust_decimal::Decimal,
    pub min_notional: rust_decimal::Decimal,
}

/// Abstraction over a market data provider (live venue, replay, or fake).
#[async_trait]
pub trait MarketDataSource: Send + Sync {
    /// Symbol metadata fetched at startup.
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError>;

    /// Bar stream (kline, venue-closed bars only).
    async fn subscribe_bars(
        &self,
        symbol: Symbol,
        tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError>;

    /// Raw trade stream.
    async fn subscribe_trades(
        &self,
        symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError>;
}
