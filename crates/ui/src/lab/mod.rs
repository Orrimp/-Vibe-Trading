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
//! ├── mod.rs         — this file, re-exports
//! ├── state.rs       — LabState struct + ops (T-D-4)
//! └── universe.rs    — XRP-first pair ordering (T-D-8)
//! ```
//!
//! Modules added in later milestones (not yet created at M1):
//! - `defaults.rs` — cold-start constant (M-FINAL / T-D-17)
//! - `persistence.rs` — JSON read/write + debounce (M-FINAL / T-D-17)
//! - `equity_loader.rs` — cached-report scanner (M2 / T-D-10)
//! - `runner.rs` — ADR-0030 invocation glue (M2.5 / T-D-14)

pub mod state;
pub mod universe;

pub use state::{DateRange, LabState, Preset, StrategyFamily, COMPARE_SET_CAP};
