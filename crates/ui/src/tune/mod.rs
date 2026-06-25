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
//! ├── mod.rs    — this file, re-exports
//! └── state.rs  — SweepReportMirror + SweepCellRow + the from_report seam
//! ```
//!
//! ## Phase discipline
//!
//! T5 (this module): engine mirror + from_report seam. No view, no runner.
//! T6/T8/T9: screen body + fixtures + render-pixel guards (later phases).
//! T10: runner glue (`spawn_sweep`).
//!
//! `cargo tree -p ui` is UNCHANGED — no new crate edge (backtest is already a dep).

pub mod state;

pub use state::{
    SweepBenchmarkKpis, SweepCellRow, SweepDistributionMirror, SweepReportMirror, SweepVerdictLabel,
};
