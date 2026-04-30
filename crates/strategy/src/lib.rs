//! Strategy trait, registry, and implementations.
//!
//! v0 ships one strategy: `sma_crossover`.
//! v0.5 adds `ComposedStrategy` with hot-loadable TOML rules.
//! v1 adds `MomentumStrategy` (cross-sectional momentum, T606).
//! v1.5a adds `MeanReversionPairsStrategy` (mean-reversion pairs, T706).

pub mod composed;
pub mod cross_sectional;
pub mod pairs;
pub mod registry;
pub mod sma_crossover;
pub mod traits;

pub use composed::{ComposedStrategy, ComposedStrategyConfig, Sizing, Stage, StrategyLoadError};
pub use cross_sectional::{
    top_k_long, CrossSectionalLoadError, CrossSectionalMomentumConfig, MomentumStrategy,
};
pub use pairs::{MeanReversionPairsConfig, MeanReversionPairsStrategy, PairsLoadError};
pub use registry::{
    flush_pending_to_ledger, PendingJournalEvent, RegistryEventKind, StrategyRegistry,
    StrategyTomlEntry,
};
pub use sma_crossover::SmaCrossover;
pub use traits::Strategy;
