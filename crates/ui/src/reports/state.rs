//! cockpit-reports-viewer v0.1.0 — Reports-screen per-session state (D1).
//!
//! Holds the discovered `backtest-*.md` corpus, the active selection index,
//! and the active selection's load result. Mirrors `BaselineScreenState`
//! and `ModelsScreenState` one-for-one — the established list-detail shape.
//!
//! **Reuses `crate::viewer::ReportLoadResult` verbatim** as the `loaded`
//! payload (no parallel load-result type). Each field of `ReportLoadResult`
//! carries its own `PanelState`, so the KPI strip can be `Ready` while the
//! equity curve is `Empty` — the common corpus case (no companion CSV).
//!
//! [`ReportEntry`] holds the `PathBuf` **in state** so the selection
//! message (`Message::ReportsSelect(usize)`) is a typed index, never a raw
//! `String`/`PathBuf` payload (R1) — the Baseline typed-message discipline.

use std::path::PathBuf;

use smol_str::SmolStr;

use crate::reports::loader;
use crate::state::PanelState;
use crate::viewer::ReportLoadResult;

/// One discovered backtest report (R1 / D1).
///
/// `slug` + `file_stem` form the picker row label `"<slug> · <file_stem>"`;
/// `path` is the full on-disk path, held here so the selection message is a
/// typed index into the discovered list (R1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportEntry {
    /// Feature slug (the `spec/<slug>/` dir name).
    pub slug: SmolStr,
    /// Report filename stem `backtest-<YYYYMMDD-HHMMSS>-<scenario>` (carries
    /// both the date and the scenario).
    pub file_stem: SmolStr,
    /// Full path to the `.md` report — held in state, NEVER the message key.
    pub path: PathBuf,
}

/// Per-session Reports-screen state (D1). Sibling of `BaselineScreenState`.
///
/// `Default` = `discovered: Loading` (pre-boot, before [`load_into`] runs),
/// `selected: None`, `loaded: Loading`. The boot scan flips `discovered` to
/// `Ready(list)` (or `Empty` on zero results); the first selection flips
/// `loaded` to `Ready`/`Error`.
#[derive(Debug, Clone)]
pub struct ReportsScreenState {
    /// Discovered corpus. `Loading` pre-boot, `Ready(list)` after the boot
    /// scan, `Empty` when the scan finds zero `backtest-*.md`. `Error` is
    /// not produced by the scan (it degrades to an empty list per K2, so
    /// `Empty` is the normal no-reports surface) — the variant exists for
    /// completeness only.
    pub discovered: PanelState<Vec<ReportEntry>>,
    /// Index into the discovered list of the active selection. `None` = no
    /// selection yet (the detail pane shows the "pick a report" prompt).
    pub selected: Option<usize>,
    /// The active selection's load result. `Loading` until the first
    /// selection, then `Ready(ReportLoadResult)` (whose own fields carry
    /// per-panel states) or `Error` if the file vanished between discovery
    /// and selection.
    pub loaded: PanelState<ReportLoadResult>,
}

impl Default for ReportsScreenState {
    fn default() -> Self {
        Self {
            discovered: PanelState::Loading,
            selected: None,
            loaded: PanelState::Loading,
        }
    }
}

impl ReportsScreenState {
    /// Load the report at `idx` in the discovered list into `loaded` (R2 /
    /// R3). Pure aside from the synchronous one-file read; **never panics**:
    ///
    /// - `idx` out of range / `discovered` not `Ready` → `loaded: Error`
    ///   (defensive — the update arm only fires this for a valid index).
    /// - The file vanished between discovery and selection (or its body is
    ///   unreadable) → `loaded: Error` (the Error-on-detail surface, R3).
    /// - Otherwise → `loaded: Ready(result)`; a malformed `## Summary`
    ///   surfaces as `result.metrics: Error`, a missing companion CSV as
    ///   `result.equity: Empty` (field-level degrade, not a panic).
    pub fn load_selection(&mut self, idx: usize) {
        let path = match &self.discovered {
            PanelState::Ready(list) => list.get(idx).map(|e| e.path.clone()),
            _ => None,
        };
        let Some(path) = path else {
            self.loaded = PanelState::Error(crate::strings::REPORTS_LOAD_ERROR.into());
            return;
        };
        match loader::load_report(&path) {
            Ok(result) => self.loaded = PanelState::Ready(result),
            Err(e) => {
                tracing::debug!(
                    path = %path.display(),
                    error = %e,
                    "reports: load_selection failed (file vanished or unreadable) — Error state"
                );
                self.loaded = PanelState::Error(crate::strings::REPORTS_LOAD_ERROR.into());
            }
        }
    }

    /// The selected entry, if any (`selected` index resolved against the
    /// `Ready` discovered list). Borrowed by the screen view so the active
    /// row can be styled.
    #[must_use]
    pub fn selected_entry(&self) -> Option<&ReportEntry> {
        let idx = self.selected?;
        match &self.discovered {
            PanelState::Ready(list) => list.get(idx),
            _ => None,
        }
    }
}

/// Boot-load the discovered corpus into `model.reports_screen_state` (R3 /
/// R7 / AC4). Called once from each bin's boot path next to
/// `ui::baseline::load_into`.
///
/// Discovery is filename-only (no per-file parse), so it is cheap +
/// synchronous — the Baseline-style boot-load fits the "synchronous +
/// cheap" Loading→Ready contract (R3). **Never panics**: the scan degrades
/// to an empty list (K2), which lands as `PanelState::Empty` — the
/// deterministic empty-list surface in a fixtures-only checkout.
pub fn load_into(model: &mut crate::state::Cockpit) {
    let discovered = loader::discover_reports();
    model.reports_screen_state.discovered = if discovered.is_empty() {
        PanelState::Empty
    } else {
        PanelState::Ready(discovered)
    };
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]
mod tests {
    use super::*;

    #[test]
    fn default_is_loading_none_loading() {
        let s = ReportsScreenState::default();
        assert!(matches!(s.discovered, PanelState::Loading));
        assert_eq!(s.selected, None);
        assert!(matches!(s.loaded, PanelState::Loading));
        assert!(s.selected_entry().is_none());
    }

    /// A vanished-path index → `loaded: Error`, no panic (AC3). We build a
    /// `Ready` discovered list whose single entry points at a path that
    /// does not exist, then load it.
    #[test]
    fn load_selection_vanished_path_yields_error_no_panic() {
        let mut s = ReportsScreenState::default();
        s.discovered = PanelState::Ready(vec![ReportEntry {
            slug: SmolStr::new("slug"),
            file_stem: SmolStr::new("backtest-20260101-000000-x"),
            path: PathBuf::from("/definitely/not/real/backtest-20260101-000000-x.md"),
        }]);
        s.selected = Some(0);
        s.load_selection(0);
        assert!(
            matches!(s.loaded, PanelState::Error(_)),
            "vanished path → Error, got {}",
            s.loaded.variant_name()
        );
    }

    /// An out-of-range index → `loaded: Error`, no panic (defensive). Also
    /// covers `discovered` not being `Ready`.
    #[test]
    fn load_selection_out_of_range_or_not_ready_yields_error() {
        // Empty discovered, any index.
        let mut s = ReportsScreenState::default();
        s.discovered = PanelState::Empty;
        s.load_selection(0);
        assert!(matches!(s.loaded, PanelState::Error(_)));

        // Ready but index past the end.
        let mut s2 = ReportsScreenState::default();
        s2.discovered = PanelState::Ready(Vec::new());
        s2.load_selection(5);
        assert!(matches!(s2.loaded, PanelState::Error(_)));
    }

    /// A valid temp fixture loads to `Ready`, and `selected_entry` resolves
    /// the active row.
    #[test]
    fn load_selection_valid_fixture_is_ready() {
        let dir = std::env::temp_dir().join(format!("reports_state_ok_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("backtest-20260101-000000-fixture.md");
        let body = "---\nscenario: fixture\n---\n\
                    # Report\n\n\
                    ## Summary\n\n\
                    | Metric | Value |\n\
                    |--------|-------|\n\
                    | Total return | 12.34% |\n\
                    | Trades | 7 |\n";
        std::fs::write(&path, body).expect("write");

        let mut s = ReportsScreenState::default();
        s.discovered = PanelState::Ready(vec![ReportEntry {
            slug: SmolStr::new("slug"),
            file_stem: SmolStr::new("backtest-20260101-000000-fixture"),
            path: path.clone(),
        }]);
        s.selected = Some(0);
        s.load_selection(0);
        assert!(
            matches!(s.loaded, PanelState::Ready(_)),
            "valid fixture → Ready, got {}",
            s.loaded.variant_name()
        );
        assert_eq!(
            s.selected_entry().map(|e| e.file_stem.as_str()),
            Some("backtest-20260101-000000-fixture")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
