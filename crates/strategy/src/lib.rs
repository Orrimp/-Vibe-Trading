//! Strategy trait, registry, and implementations.
//!
//! v0 ships one strategy: `sma_crossover`.
//! v0.5 adds `ComposedStrategy` with hot-loadable TOML rules.

pub mod composed;
pub mod registry;
pub mod sma_crossover;
pub mod traits;

pub use composed::{ComposedStrategy, ComposedStrategyConfig, Sizing, Stage, StrategyLoadError};
pub use registry::{
    flush_pending_to_ledger, PendingJournalEvent, RegistryEventKind, StrategyRegistry,
    StrategyTomlEntry,
};
pub use sma_crossover::SmaCrossover;
pub use traits::Strategy;
