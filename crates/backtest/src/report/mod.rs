//! Extracted report-writer modules — Phase B (ADR-0035).
//!
//! Each sub-module contains the extracted body of one `write_*_report`
//! function from `main.rs`. Writers are called from scenario modules after
//! the backtest run completes.
//!
//! # Determinism contract
//!
//! The bytes produced by each writer must be byte-identical to those
//! produced by the original `main.rs` functions when given the same inputs.
//! The 22 body-SHA-256 anchors in `spec/anchors.toml` guard this contract.

pub mod momentum;
pub mod pairs;
pub mod sma;
pub mod tcn_overlay;
