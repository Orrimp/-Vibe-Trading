//! Viewer model + message types — Phase 4.
//!
//! Lives in the `ui` lib (not the `bin/viewer.rs` bin) because the
//! widgets `kpi_strip`, `equity_curve`, `drawdown_band` return
//! `Element<'_, ViewerMessage>` and bins can't import from each other.
//!
//! Sibling of [`crate::state::Cockpit`] / [`crate::state::Message`] —
//! the cockpit lives in `state.rs`; the viewer lives here. Both are
//! pure-data presentation models; `update` is a pure function.

use std::path::PathBuf;

use smol_str::SmolStr;
use trading_core::{BacktestMetrics, EquitySeries};

use crate::state::PanelState;
use crate::theme::ThemeMode;

/// Read-only mirror of the YAML front-matter fields the viewer's
/// title bar and body header surface. Only the load-bearing fields
/// (`scenario` for the title bar) are mirrored — anything else
/// becomes part of the markdown body and renders inline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportFrontMatter {
    /// `scenario:` value from the front-matter; populates the
    /// window title `"Backtest report — {scenario}"`. Empty string
    /// when the file has no front-matter or the field is absent.
    pub scenario: SmolStr,
}

/// One-shot load result — fired at boot from `fn main` after the CLI
/// arg parses and the file reads succeed. Carries the four
/// sub-states so the model lands fully-populated on success and the
/// curve / strip / body all render together.
#[derive(Debug, Clone)]
pub struct ReportLoadResult {
    pub front_matter: ReportFrontMatter,
    pub metrics: PanelState<BacktestMetrics>,
    pub equity: PanelState<EquitySeries>,
    pub body_markdown: String,
}

/// Root model for the viewer bin. Owned by the iced `Application`.
#[derive(Debug, Clone)]
pub struct ViewerModel {
    pub mode: ThemeMode,
    pub report_path: PathBuf,
    pub front_matter: ReportFrontMatter,
    pub metrics: PanelState<BacktestMetrics>,
    pub equity: PanelState<EquitySeries>,
    pub body_markdown: String,
}

impl ViewerModel {
    /// Construct a viewer model from a one-shot load result.
    #[must_use]
    pub fn new(report_path: PathBuf, result: ReportLoadResult) -> Self {
        Self {
            mode: ThemeMode::Dark,
            report_path,
            front_matter: result.front_matter,
            metrics: result.metrics,
            equity: result.equity,
            body_markdown: result.body_markdown,
        }
    }
}

/// Every possible state mutation for the viewer. Exhaustive by
/// construction — `update` matches with no catch-all arm.
#[derive(Debug, Clone)]
pub enum ViewerMessage {
    /// One-shot load result fired at boot. Field-level errors
    /// degrade to `PanelState::Error` independently — a missing
    /// equity CSV does not invalidate the KPI strip (R3.5 / R11.3
    /// missing-field tolerance).
    ReportLoaded(Box<ReportLoadResult>),
    /// Theme toggle — flips `mode`. No status bar so the toggle
    /// surfaces only via the bin's keyboard shim if/when wired
    /// (Phase 4 ships no keyboard handler).
    ToggleTheme,
}

/// Pure state-transition function. `update` is exhaustive over
/// `ViewerMessage`.
pub fn update(model: &mut ViewerModel, msg: ViewerMessage) {
    match msg {
        ViewerMessage::ReportLoaded(boxed) => {
            let ReportLoadResult {
                front_matter,
                metrics,
                equity,
                body_markdown,
            } = *boxed;
            model.front_matter = front_matter;
            model.metrics = metrics;
            model.equity = equity;
            model.body_markdown = body_markdown;
        }
        ViewerMessage::ToggleTheme => {
            model.mode = match model.mode {
                ThemeMode::Dark => ThemeMode::Light,
                ThemeMode::Light => ThemeMode::Dark,
            };
        }
    }
}
