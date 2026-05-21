//! PatchTST sync-forecaster wrapper (v2.5a Wave D, T-D-N22).
//!
//! Provides `PatchTstSyncForecaster` — a synchronous wrapper around
//! `forecast::patchtst::PatchTstForecaster` that can be used from
//! `PatchTstOverlayMomentumStrategy` and other sync strategy contexts
//! without an async runtime.
//!
//! The implementation lives in `patchtst_overlay_momentum.rs` (Wave A.4)
//! and is re-exported here so that the module tree mirrors the TCN pattern
//! (where `tcn_sync.rs` was the planned home for the sync wrapper).
//!
//! ## Cross-references
//!
//! - `crates/strategy/src/patchtst_overlay_momentum.rs` — inline implementation
//! - `spec/v25a-patchtst-overlay/decomp.md § T-AR-4` — design decision
//! - `spec/architecture/adr/0036-patchtst-training-contract.md § D7`

/// Re-export `PatchTstSyncForecaster` from its Wave A implementation home.
#[cfg(feature = "forecast")]
pub use crate::patchtst_overlay_momentum::PatchTstSyncForecaster;
