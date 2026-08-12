//! cockpit-baseline-panel v0.1.0 — passive-BH baseline feature module.
//!
//! Pure-`ui` over `trading_core` + `std::fs` — **no new crate edge** (AC7).
//! Surfaces the shipped passive buy-and-hold result (the program's headline
//! deliverable) inside the cockpit shell, reusing the existing
//! `equity_curve` / `drawdown_band` / `kpi_strip` widgets verbatim.
//!
//! Module layout:
//!
//! ```text
//! baseline/
//! ├── mod.rs     — this file, re-exports
//! ├── loader.rs  — CSV → EquitySeries loader + embedded §7.1 metrics const (T2/T3)
//! └── state.rs   — BaselineScreenState + boot-load helper (T4)
//! ```
//!
//! The screen body lives at `crate::screens::baseline` (T5); the sidebar
//! IA + `Screen::Baseline` routing live in `crate::theme` / `crate::state`
//! / `crate::shell` (T1/T7).

pub mod loader;
pub mod state;

pub use loader::{
    BaselineRiskFacts, baseline_csv_path, baseline_metrics, baseline_risk_detail,
    baseline_risk_facts, baseline_sampling_note, baseline_sharpe_note, curve_max_drawdown_pct,
    load_baseline_curve,
};
pub use state::{BaselineScreenState, load_into};
