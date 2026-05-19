//! Lab feature module — ui-rethink-phase-a-lab.
//!
//! Houses all Lab-screen-specific logic that is not a widget or a screen
//! view: state shape, cold-start defaults, persistence (M-FINAL), equity
//! loader (M2), and the backtest runner glue (M2.5).
//!
//! Module layout per Design § 1:
//!
//! ```text
//! lab/
//! ├── mod.rs           — this file, re-exports
//! ├── state.rs         — LabState struct + ops (T-D-4)
//! ├── universe.rs      — XRP-first pair ordering (T-D-8)
//! ├── equity_loader.rs — cached-report scanner (M2 / T-D-10)
//! ├── defaults.rs      — cold-start constant (M-FINAL / T-D-17)
//! ├── persistence.rs   — JSON read/write + debounce (M-FINAL / T-D-17)
//! └── runner.rs        — ADR-0030 invocation glue (M2.5 / T-D-14)
//! ```

pub mod defaults;
pub mod equity_loader;
pub mod persistence;
/// cockpit-training-control T-D-N14 — cross-platform PID liveness check.
pub mod pid_alive;
pub mod runner;
pub mod state;
pub mod trainer;
/// cockpit-training-control T-D-N11 — 1 Hz audit-DB poller subscription.
/// Gated behind `live` because it requires the `audit` crate + `async-stream`.
#[cfg(feature = "live")]
pub mod training_subscription;
pub mod universe;

pub use equity_loader::{EquityCache, EquityLoadError, Fidelity, LabEquitySeries, LabTuple};
pub use state::{COMPARE_SET_CAP, DateRange, LabState, Preset, StrategyFamily};
