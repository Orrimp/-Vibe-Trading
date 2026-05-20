//! Phase E — Compare-screen per-session state.
//!
//! Sibling of `crates/ui/src/lab/state.rs` (Phase A/B) and
//! `crates/ui/src/state.rs::TrailScreenState` (Phase D). All fields
//! are session-scoped; no on-disk persistence at v0.1.0
//! (matches Q5=(a) sidebar-only-entry + Q2=(c) report-cache-only).

use std::collections::BTreeMap;

use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::Symbol;

use crate::lab::state::DateRange;

/// Phase E — KPI axis dropdown variants (R6.3).
///
/// v0.1.0 wires `Sharpe` only (Q3=(a)); selecting any other variant
/// at runtime falls back to `Sharpe` with a `tracing::warn!` in dev
/// builds. The full enum lives now so the dropdown widget can render
/// all 5 options (UI surface stable across the v0.1.0 → v0.2.0
/// transition).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompareKpiAxis {
    #[default]
    Sharpe,
    Sortino,
    TotalReturn,
    MaxDrawdown,
    WinRate,
}

impl CompareKpiAxis {
    /// Human-readable label for the dropdown.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            CompareKpiAxis::Sharpe => "Sharpe",
            CompareKpiAxis::Sortino => "Sortino",
            CompareKpiAxis::TotalReturn => "Total Return",
            CompareKpiAxis::MaxDrawdown => "Max Drawdown",
            CompareKpiAxis::WinRate => "Win Rate",
        }
    }
}

/// Phase E — one (strategy, pair) cell's cached KPI snapshot (R3.1).
///
/// Populated by `compare::cache::scan_spec_tree` at first Compare-screen
/// render (cold-boot-only at v0.1.0 per R3.5).
/// `is_multi_symbol` toggles the K7 disclaimer tooltip path (§1.4 of
/// the architect's M-T1 decomp).
#[derive(Debug, Clone, PartialEq)]
pub struct CachedCell {
    /// Sharpe ratio — the Q3=(a) default KPI displayed in the cell.
    pub sharpe: f64,
    /// Total return in percentage (e.g. 12.3 = 12.3 %).
    pub total_return_pct: f64,
    /// Max drawdown in percentage (negative convention: -15.0 = 15 % DD).
    pub max_drawdown_pct: f64,
    /// Number of closed trades in the backtest.
    pub trade_count: u32,
    /// Trailing equity-curve tail — at most 30 bars for the sparkline (R2.3).
    pub equity_curve_tail: Vec<f64>,
    /// Repo-relative path to the source backtest report (for drill-down).
    pub source_report_path: SmolStr,
    /// ISO-8601 `generated:` timestamp from the report frontmatter.
    /// Used as the most-recent tiebreaker (R3.3) in `scan_spec_tree`.
    pub generated_at: SmolStr,
    /// `true` when the source report covers a multi-symbol universe
    /// (e.g. `top10-2023-1h-momentum`). Drives the K7 disclaimer
    /// tooltip + subtitle in the matrix and compare screen (§1.4).
    pub is_multi_symbol: bool,
}

/// Phase E — Compare-screen per-session state (R6.1).
///
/// Added as `pub compare_screen_state: CompareScreenState` on `Cockpit`
/// at `state.rs:~880`, immediately after `trail_screen_state` (§1.6 of
/// the architect's decomp).
#[derive(Debug, Clone)]
pub struct CompareScreenState {
    /// R3.4 isolation: separate from `Cockpit::lab_state.range`.
    /// Toggling the Compare date-range picker MUST NOT mutate Lab state.
    pub range: DateRange,
    /// R6.3 — single-KPI v0.1.0 (Sharpe); dropdown reserves the
    /// option for v0.2.0 multi-KPI heatmap.
    pub kpi_axis: CompareKpiAxis,
    /// R3.1 lookup table — keyed by `(strategy_id, symbol, range)`.
    /// Empty until first view-render (R3.5 cold-boot-only at v0.1.0).
    /// `BTreeMap` chosen over `HashMap` for deterministic iteration
    /// order (snapshot baselines — see §1.6).
    pub cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>,
    /// R3.5 cold-boot tag — `None` until first scan completes.
    pub last_indexed_at: Option<OffsetDateTime>,
}

impl Default for CompareScreenState {
    fn default() -> Self {
        Self {
            range: DateRange::default(), // Preset::Last90d per lab::state
            kpi_axis: CompareKpiAxis::Sharpe,
            cache: BTreeMap::new(),
            last_indexed_at: None,
        }
    }
}
