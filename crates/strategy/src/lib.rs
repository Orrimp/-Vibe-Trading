//! Strategy trait, registry, and implementations.
//!
//! v0 ships one strategy: `sma_crossover`.
//! v0.5 adds `ComposedStrategy` with hot-loadable TOML rules.
//! v1 adds `MomentumStrategy` (cross-sectional momentum, T606).
//! v1.5a adds `MeanReversionPairsStrategy` (mean-reversion pairs, T706).
//! v2.5 adds `TcnOverlayMomentumStrategy` (TCN forecast overlay on v1 momentum, T-D-14).
//! v2.5a adds `PatchTstOverlayMomentumStrategy` (PatchTST forecast overlay, T-D-N22).

pub mod composed;
pub mod cross_sectional;
pub mod pairs;
pub mod patchtst_overlay_momentum;
pub mod registry;
pub mod sma_crossover;
pub mod tcn_overlay_momentum;
pub mod traits;

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
