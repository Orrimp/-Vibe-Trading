//! Mean-reversion pairs strategy module (v1.5a T701–T706).
//!
//! ## Sub-modules
//!
//! - [`config`] — TOML config loader/validator ([`MeanReversionPairsConfig`],
//!   [`PairsLoadError`]).
//! - [`pair_state`] — Per-pair state machine ([`PairState`], [`SyncSlot`],
//!   decision logic).
//! - [`mean_reversion`] — [`MeanReversionPairsStrategy`] (implements [`crate::Strategy`]).

pub mod config;
pub mod mean_reversion;
pub mod pair_state;

pub use config::{MeanReversionPairsConfig, PairsLoadError};
pub use mean_reversion::MeanReversionPairsStrategy;
pub use pair_state::{LegRole, PairState, PositionState, SyncSlot, PAIR_SYNC_DROPPED_TOTAL};
