//! Execution routing: paper-mode and live-mode stubs.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod live;
pub mod paper;
pub mod publisher;
pub mod router;

/// Deterministic latency simulation for backtest fills (v5-latency-slippage-sim R2).
/// Backtest-only — not called from live-mode paths (ADR-0043 § D5).
pub mod latency;

pub use latency::apply_latency;
pub use live::cap::check_notional_cap;
pub use live::endpoint::Network;
pub use live::error::ExecError;
pub use live::types::{AccountSnapshot, Balance, OrderAck, OrderRef, OrderStatus};
pub use live::{AccountReader, BinanceSpotExecClient, LiveExecRouter};
pub use paper::PaperEnginePublisher;
pub use publisher::{FillPublisher, NullPublisher};
pub use router::{ExecRouter, PaperExecRouter};
