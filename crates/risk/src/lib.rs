//! Risk engine: position sizing, limit enforcement, pre-trade checks.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod portfolio;
pub mod sizing;

pub use portfolio::{PortfolioSizeError, TargetLeg, size_portfolio_target};
pub use sizing::{FixedFractionSizer, SizingError, size_and_validate};
