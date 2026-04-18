//! Risk engine: position sizing, limit enforcement, pre-trade checks.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod sizing;

pub use sizing::{size_and_validate, FixedFractionSizer, SizingError};
