//! `MatchingEngine` trait — per architecture.md matching-engine section.
use async_trait::async_trait;
use thiserror::Error;
use trading_core::{Bar, Fill, Order};

use crate::paper::MatchConfig;

/// Error from the matching engine.
#[derive(Debug, Error)]
pub enum MatchError {
    #[error("fill computation error: {0}")]
    FillError(String),
    #[error("no liquidity")]
    NoLiquidity,
}

/// The matching engine abstraction.
///
/// v0 ships `PaperEngine` (simple bps slippage + taker fee).
/// The trait signature is limit-order-friendly even though v0 only uses market orders.
/// v0.5 may swap in `orderbook-rs` / `matchcore` / `rust_ob` without changing callers.
#[async_trait]
pub trait MatchingEngine: Send + Sync {
    /// Process bar-aligned orders and return fills.
    async fn step(&mut self, bar: &Bar, orders: Vec<Order>) -> Result<Vec<Fill>, MatchError>;

    fn config(&self) -> MatchConfig;
}
