//! Backtest engine: `MatchingEngine` trait, `PaperEngine`, backtest loop.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod engine;
pub mod paper;

pub use engine::MatchingEngine;
pub use paper::PaperEngine;
