//! Strategy trait, registry, and implementations.
//!
//! v0 ships one strategy: `sma_crossover`.

pub mod registry;
pub mod sma_crossover;
pub mod traits;

pub use registry::{
    flush_pending_to_ledger, PendingJournalEvent, RegistryEventKind, StrategyRegistry,
    StrategyTomlEntry,
};
pub use sma_crossover::SmaCrossover;
pub use traits::Strategy;
