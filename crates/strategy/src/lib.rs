//! Strategy trait, registry, and implementations.
//!
//! v0 ships one strategy: `sma_crossover`.
//! v0.5 adds `ComposedStrategy` with hot-loadable TOML rules.
//! v1 adds `MomentumStrategy` (cross-sectional momentum, T606).
//! v1.5a adds `MeanReversionPairsStrategy` (mean-reversion pairs, T706).
//! v2.5 adds `TcnOverlayMomentumStrategy` (TCN forecast overlay on v1 momentum, T-D-14).
//! v2.5a adds `PatchTstOverlayMomentumStrategy` (PatchTST forecast overlay, T-D-N22).
//! v3.0.0-volatility adds 3 GARCH(1,1) vol builders (ADR-0038 § D5):
//!   - `with_garch_vol_overlay_momentum` (R6.a primary — vol-targeting overlay).
//!   - `with_garch_vol_kill_switch` (R6.b secondary — kill-switch overlay).
//!   - `with_garch_vol_strategy` (R6.c tertiary — standalone mean-reversion).

pub mod composed;
pub mod cross_sectional;
pub mod pairs;
pub mod patchtst_overlay_momentum;
pub mod patchtst_sync;
pub mod registry;
pub mod sma_crossover;
pub mod tcn_overlay_momentum;
pub mod traits;
pub mod vol_killswitch_overlay;
pub mod vol_meanreversion;
pub mod vol_targeting_overlay;

pub use composed::{ComposedStrategy, ComposedStrategyConfig, Sizing, Stage, StrategyLoadError};
pub use cross_sectional::{
    CrossSectionalLoadError, CrossSectionalMomentumConfig, MomentumStrategy, top_k_long,
};
pub use pairs::{MeanReversionPairsConfig, MeanReversionPairsStrategy, PairsLoadError};
#[cfg(feature = "forecast")]
pub use patchtst_overlay_momentum::PatchTstSyncForecaster;
pub use patchtst_overlay_momentum::{
    PatchTstOverlayMomentumConfig, PatchTstOverlayMomentumStrategy,
};
pub use registry::{
    PendingJournalEvent, RegistryEventKind, StrategyRegistry, StrategyTomlEntry,
    flush_pending_to_ledger,
};
pub use sma_crossover::SmaCrossover;
#[cfg(feature = "forecast")]
pub use tcn_overlay_momentum::TcnSyncForecaster;
pub use tcn_overlay_momentum::{
    ForecastDirection, ModulationStats, PassthroughForecaster, SyncForecaster,
    TcnOverlayMomentumConfig, TcnOverlayMomentumStrategy,
};
pub use traits::Strategy;
pub use vol_killswitch_overlay::{VolKillSwitchConfig, VolKillSwitchOverlay};
pub use vol_meanreversion::{VolMeanReversionConfig, VolMeanReversionStrategy};
pub use vol_targeting_overlay::{GarchParams, VolTargetingConfig, VolTargetingOverlay};

// ── v3.0.0-volatility GARCH builder fns (ADR-0038 § D5 / decomp.md T-AR-4) ─

/// Builder: standalone vol mean-reversion strategy (R6.c tertiary).
///
/// Emits Buy when Parkinson realized vol > GARCH predicted vol.
/// Unit-tested only in v0.1.0; no backtest scenario (per Q-anchors-sub = 3).
///
/// # Arguments
///
/// - `symbols`: which symbols to trade.
/// - `models`: GARCH(1,1) params keyed by symbol name.
/// - `config`: vol mean-reversion config (default: `VolMeanReversionConfig::default()`).
#[must_use]
pub fn with_garch_vol_strategy(
    symbols: &[&str],
    models: std::collections::BTreeMap<String, GarchParams>,
    config: VolMeanReversionConfig,
) -> VolMeanReversionStrategy {
    VolMeanReversionStrategy::new(symbols, models, config)
}

/// Builder: vol-targeting overlay on v1 momentum (R6.a primary — anchor target).
///
/// Wraps `inner` with a GARCH(1,1) vol-targeting scaler that adjusts
/// signal quantities by `clamp(target_vol / sigma_hat, [scale_clamp_min, scale_clamp_max])`.
///
/// # Arguments
///
/// - `inner`: the v1 `MomentumStrategy` to wrap.
/// - `models`: GARCH(1,1) params keyed by symbol name.
/// - `config`: vol-targeting config (default: `VolTargetingConfig::default()`).
#[must_use]
pub fn with_garch_vol_overlay_momentum(
    inner: MomentumStrategy,
    models: std::collections::BTreeMap<String, GarchParams>,
    config: VolTargetingConfig,
) -> VolTargetingOverlay {
    VolTargetingOverlay::new(inner, models, config)
}

/// Builder: kill-switch overlay on v1 momentum (R6.b secondary).
///
/// Wraps `inner` with a GARCH(1,1) kill-switch that holds signals flat
/// when `sigma_hat > threshold_multiplier × rolling_median(sigma_hat)`.
///
/// # Arguments
///
/// - `inner`: the v1 `MomentumStrategy` to wrap.
/// - `models`: GARCH(1,1) params keyed by symbol name.
/// - `config`: kill-switch config (default: `VolKillSwitchConfig::default()`).
#[must_use]
pub fn with_garch_vol_kill_switch(
    inner: MomentumStrategy,
    models: std::collections::BTreeMap<String, GarchParams>,
    config: VolKillSwitchConfig,
) -> VolKillSwitchOverlay {
    VolKillSwitchOverlay::new(inner, models, config)
}
