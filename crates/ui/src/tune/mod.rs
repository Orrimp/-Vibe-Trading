//! Gate-tied hyperparameter sweep Tune screen — state mirror and future runner.
//!
//! # Architecture (ADR-0069)
//!
//! This module is the `ui`-side of the sweep feature. The engine seam is
//! `backtest::run_param_sweep` (homed in `crates/backtest`). The ONE boundary
//! is `state::SweepReportMirror::from_report` — the only place a
//! `backtest::SweepReport` is read.
//!
//! ## Module layout
//!
//! ```text
//! tune/
//! ├── mod.rs           — this file, re-exports
//! ├── state.rs         — SweepReportMirror + SweepCellRow + the from_report seam (T5)
//! ├── screen_state.rs  — TuneScreenState (the range form + run lifecycle) (T6/T10)
//! └── runner.rs        — spawn_sweep + sweep_config_from_state (T10)
//! ```
//!
//! ## Phase discipline
//!
//! T5: engine mirror + from_report seam (`state.rs`).
//! T6: screen body (`crate::screens::tune`) + the form state (`screen_state.rs`).
//! T10: runner glue (`runner::spawn_sweep`).
//!
//! `cargo tree -p ui` is UNCHANGED — no new crate edge (backtest is already a dep).

pub mod runner;
pub mod screen_state;
pub mod state;

pub use screen_state::{
    AxisField, AxisInput, AxisPreset, BOLLINGER_K_PRESETS, BollingerGridForm, GridEstimate,
    MacdGridForm, RsiGridForm, SmaGridForm, TuneAxisKind, TuneFamily, TuneScreenState,
};
pub use state::{
    PromoteParams, SweepBenchmarkKpis, SweepCellRow, SweepDistributionMirror, SweepReportMirror,
    SweepVerdictLabel,
};
