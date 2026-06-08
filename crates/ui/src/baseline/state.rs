//! cockpit-baseline-panel v0.1.0 — Baseline-screen per-session state (T4).
//!
//! Holds the two realized BH equity curves (one per year), the two §7.1
//! KPI-metrics blocks, and the active-year toggle.
//!
//! The KPI scalars are **sourced from the `const`**
//! [`crate::baseline::baseline_metrics`] (D1 = c — never recomputed from
//! the daily-sampled curve, never parsed from markdown). They are
//! materialized into `metrics_*` fields here so the screen `view` can
//! borrow a `&PanelState<BacktestMetrics>` with the model's lifetime — the
//! same pattern the `viewer` binary uses for its metrics strip
//! (`bin/viewer.rs:102` borrows `&self.model.metrics`). The KPI-strip
//! widget ties its returned `Element<'a>` to the input ref's lifetime, so
//! the input must outlive the element; storing the const-derived value on
//! the model satisfies that without a per-frame `Box::leak`. The single
//! source of truth (`baseline_metrics`) is unchanged — these fields are
//! just its boot-time materialization. The re-sync test still guards the
//! `const`.
//!
//! Curves + metrics are populated **once at boot** via [`load_into`]
//! (mirroring how `cockpit.rs` pre-seeds fixtures). Boot-load keeps the
//! `update` arm trivial (a pure year assignment) and means the first
//! Baseline visit already shows `Ready` (or `Error`, in a fixtures-only
//! checkout where the CSVs are absent — the metrics half still populates).

use trading_core::{BacktestMetrics, EquitySeries};

use crate::baseline::loader;
use crate::state::{BaselineYear, PanelState};

/// Per-session Baseline-screen state. Sibling of `CompareScreenState`.
///
/// `Default` = `active_year: Y2024`, both curves `Loading` (the pre-boot
/// state, before [`load_into`] runs), and both metrics blocks already
/// populated from the `const` (the metrics half never errors / loads).
#[derive(Debug, Clone)]
pub struct BaselineScreenState {
    /// Realized BH equity curve for 2023 (loaded at boot from
    /// `bh-equity-curve-2023.csv`).
    pub curve_2023: PanelState<EquitySeries>,
    /// Realized BH equity curve for 2024 (loaded at boot from
    /// `bh-equity-curve-2024.csv`).
    pub curve_2024: PanelState<EquitySeries>,
    /// §7.1 realized KPI metrics for 2023 — `Ready`-wrapped const value
    /// (D1 = c). Always populated; the metrics half never errors.
    pub metrics_2023: PanelState<BacktestMetrics>,
    /// §7.1 realized KPI metrics for 2024 — `Ready`-wrapped const value.
    pub metrics_2024: PanelState<BacktestMetrics>,
    /// Operator-selected year. Cold-start `Y2024` (most recent, R2).
    pub active_year: BaselineYear,
}

impl Default for BaselineScreenState {
    fn default() -> Self {
        Self {
            curve_2023: PanelState::Loading,
            curve_2024: PanelState::Loading,
            // Metrics come from the const — populate immediately so the KPI
            // strip is `Ready` even before / without the curve CSVs.
            metrics_2023: PanelState::Ready(loader::baseline_metrics(BaselineYear::Y2023)),
            metrics_2024: PanelState::Ready(loader::baseline_metrics(BaselineYear::Y2024)),
            active_year: BaselineYear::default(),
        }
    }
}

impl BaselineScreenState {
    /// The curve `PanelState` for the active year — the single accessor
    /// the screen view reads for both the equity-curve and the drawdown
    /// band (both derive from the same `EquitySeries`).
    #[must_use]
    pub fn active_curve(&self) -> &PanelState<EquitySeries> {
        match self.active_year {
            BaselineYear::Y2023 => &self.curve_2023,
            BaselineYear::Y2024 => &self.curve_2024,
        }
    }

    /// The §7.1 KPI metrics `PanelState` for the active year (always
    /// `Ready` — sourced from the `const`). Borrowed by the screen so the
    /// KPI-strip element gets the model's lifetime.
    #[must_use]
    pub fn active_metrics(&self) -> &PanelState<BacktestMetrics> {
        match self.active_year {
            BaselineYear::Y2023 => &self.metrics_2023,
            BaselineYear::Y2024 => &self.metrics_2024,
        }
    }
}

/// Boot-load both years' curves into `model.baseline_screen_state` (R1 /
/// R4). Called once from each bin's boot path (the fixtures `cockpit` bin
/// and `cockpit_live`), mirroring the fixture pre-seed pattern.
///
/// Synchronous + **never panics** — the loader degrades a missing/malformed
/// CSV to `PanelState::Error` (R7), so this is safe to call unconditionally
/// at boot regardless of whether the runbook artifacts are present in the
/// checkout.
pub fn load_into(model: &mut crate::state::Cockpit) {
    model.baseline_screen_state.curve_2023 =
        loader::load_baseline_curve(&loader::baseline_csv_path(BaselineYear::Y2023));
    model.baseline_screen_state.curve_2024 =
        loader::load_baseline_curve(&loader::baseline_csv_path(BaselineYear::Y2024));
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    // Tests mutate individual fields of a `default()` instance to set up
    // each scenario — clearer than a full struct literal here.
    clippy::field_reassign_with_default
)]
mod tests {
    use super::*;

    #[test]
    fn default_is_y2024_curves_loading_metrics_ready() {
        let s = BaselineScreenState::default();
        assert_eq!(s.active_year, BaselineYear::Y2024);
        assert!(matches!(s.curve_2023, PanelState::Loading));
        assert!(matches!(s.curve_2024, PanelState::Loading));
        // `active_curve` follows the active year.
        assert!(matches!(s.active_curve(), PanelState::Loading));
        // Metrics are sourced from the const — populated immediately (D1=c).
        assert!(matches!(s.metrics_2023, PanelState::Ready(_)));
        assert!(matches!(s.metrics_2024, PanelState::Ready(_)));
        assert!(matches!(s.active_metrics(), PanelState::Ready(_)));
    }

    #[test]
    fn active_metrics_follows_year() {
        let mut s = BaselineScreenState::default();
        s.active_year = BaselineYear::Y2023;
        match s.active_metrics() {
            PanelState::Ready(m) => assert_eq!(m.sharpe, rust_decimal_macros::dec!(1.8417)),
            _ => panic!("2023 metrics must be Ready"),
        }
        s.active_year = BaselineYear::Y2024;
        match s.active_metrics() {
            PanelState::Ready(m) => assert_eq!(m.sharpe, rust_decimal_macros::dec!(0.8925)),
            _ => panic!("2024 metrics must be Ready"),
        }
    }

    #[test]
    fn active_curve_follows_year() {
        let mut s = BaselineScreenState::default();
        s.curve_2023 = PanelState::Empty;
        s.curve_2024 = PanelState::Error("x".into());
        s.active_year = BaselineYear::Y2023;
        assert!(matches!(s.active_curve(), PanelState::Empty));
        s.active_year = BaselineYear::Y2024;
        assert!(matches!(s.active_curve(), PanelState::Error(_)));
    }
}
