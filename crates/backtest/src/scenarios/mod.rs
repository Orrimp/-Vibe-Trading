//! Extracted scenario execution modules — Phase B (ADR-0035).
//!
//! Each sub-module contains the extracted body of one `run_*_backtest`
//! function from `main.rs`. The dispatcher in `engine::run_scenario`
//! delegates to these modules.
//!
//! # Anchor preservation discipline
//!
//! Every body is a behaviour-preserving extraction: same seed handling,
//! same RNG construction, same bar-iteration order, same fill/equity/KPI
//! compute. The 22 body-SHA-256 anchors in `spec/anchors.toml` must stay
//! byte-identical after this extraction (H2 / H4 / R10.1).

pub mod garch_vol_target_overlay;
pub mod momentum;
pub mod pairs;
pub mod patchtst_overlay_weights;
pub mod regime_dispatcher;
pub mod sim;
pub mod sma_composed;
pub mod sma_composed_run;
pub mod tcn_overlay;
pub mod tcn_overlay_weights;
pub mod threshold_sweep;
