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
    /// Whether a stem-matched equity companion CSV exists on disk for this
    /// report — i.e. `<report_dir>/artifacts/<file_stem>/equity-*.csv` is
    /// present (backtest-equity-companion UX follow-on). Computed cheaply at
    /// discovery time (existence-only; the CSV is **not** parsed here — that
    /// happens lazily in [`load_selection`]). Drives the picker's "has-curve"
    /// marker (so the operator can see at a glance which reports will paint a
    /// populated curve) and the boot auto-select (newest companion-bearing
    /// report is selected on entry).
    pub has_companion: bool,
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
    /// Picker-rail filter (reports-picker-curve-filter). `false` (the
    /// default) shows ONLY companion-bearing reports — the `has_companion`
    /// rows that paint a populated equity curve. `true` reveals the full
    /// discovered corpus. The operator kept landing on companion-less
    /// "no equity data" reports (only 14 of 117 ship a curve), so curve-only
    /// is the default and a compact toggle reveals all.
    ///
    /// **Filter affects the picker LIST only, never the selection.** The
    /// rows are filtered at render time by iterating the FULL discovered list
    /// with `.enumerate()` and skipping `!has_companion` rows when this is
    /// `false`; the row that survives still carries its TRUE full-list index
    /// for `Message::ReportsSelect(idx)`, so `load_selection(idx)` always
    /// resolves the right report. A non-companion report that happens to be
    /// the current `selected` (e.g. boot auto-select can only pick a companion,
    /// but a future flow might) still renders in the detail pane regardless of
    /// this flag — the toggle never clears or re-points the selection.
    pub show_all_reports: bool,
}

impl Default for ReportsScreenState {
    fn default() -> Self {
        Self {
            discovered: PanelState::Loading,
            selected: None,
            loaded: PanelState::Loading,
            // reports-picker-curve-filter — default to the curve-only view so
            // the operator does not land on "no equity data" reports.
            show_all_reports: false,
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
///
/// **Auto-select the newest companion-bearing report (backtest-equity-
/// companion UX follow-on).** Only one report in the corpus today ships a
/// stem-matched equity companion, and it sorts deep in the `(slug, stem)`
/// list with a no-companion near-duplicate right above it, so the operator
/// could not find the row whose curve actually populates — the screen looked
/// empty on entry. To fix discoverability, when the discovered list first
/// becomes `Ready` we default the selection to the **newest** entry whose
/// `has_companion == true` (newest = highest `file_stem`, which carries the
/// `YYYYMMDD-HHMMSS` stamp) and load it synchronously, so a populated curve
/// renders the moment the operator opens Reports. If no entry has a companion
/// the selection is left unset (the cold-start "pick a report" prompt — the
/// pre-follow-on behaviour). An operator selection already in place is never
/// overridden (guarded on `selected.is_none()`); `load_into` runs once at
/// boot, before any interaction, so this guard holds by construction and is
/// belt-and-braces against a future re-call.
pub fn load_into(model: &mut crate::state::Cockpit) {
    let discovered = loader::discover_reports();
    let st = &mut model.reports_screen_state;
    if discovered.is_empty() {
        st.discovered = PanelState::Empty;
        return;
    }

    let auto = newest_companion_index(&discovered);
    st.discovered = PanelState::Ready(discovered);

    // Only auto-select on a cold screen (no operator choice yet). `load_into`
    // is a boot-time one-shot, so `selected` is `None` here in practice.
    if st.selected.is_none()
        && let Some(idx) = auto
    {
        st.selected = Some(idx);
        st.load_selection(idx);
    }
}

/// Index of the **newest** companion-bearing entry in `entries`, or `None`
/// when none has a companion (backtest-equity-companion UX follow-on).
///
/// "Newest" = the lexicographically-greatest `file_stem` among
/// `has_companion == true` rows — the stem embeds the `YYYYMMDD-HHMMSS` stamp
/// (`backtest-<stamp>-<scenario>`), so a string `max` is a chronological max.
/// Pure + total (no panic, no I/O); factored out of [`load_into`] so the
/// auto-select decision is unit-testable without disk discovery.
#[must_use]
pub fn newest_companion_index(entries: &[ReportEntry]) -> Option<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.has_companion)
        .max_by(|(_, a), (_, b)| a.file_stem.cmp(&b.file_stem))
        .map(|(idx, _)| idx)
}

/// Number of companion-bearing (`has_companion == true`) entries in `entries`
/// (reports-picker-curve-filter). Drives the "Curve only (N)" chip count.
/// Pure + total; the full discovered count is just `entries.len()` ("All (M)").
#[must_use]
pub fn companion_count(entries: &[ReportEntry]) -> usize {
    entries.iter().filter(|e| e.has_companion).count()
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
        // reports-picker-curve-filter — curve-only is the default surface.
        assert!(
            !s.show_all_reports,
            "Reports picker must default to curve-only (show_all_reports = false)"
        );
    }

    /// reports-picker-curve-filter — `companion_count` matches the number of
    /// `has_companion` rows (the "Curve only (N)" chip count). `entries.len()`
    /// is the "All (M)" count.
    #[test]
    fn companion_count_matches_has_companion_rows() {
        let entries = vec![
            entry("backtest-20260101-000000-a", true),
            entry("backtest-20260202-000000-b", false),
            entry("backtest-20260303-000000-c", true),
            entry("backtest-20260404-000000-d", false),
        ];
        assert_eq!(
            companion_count(&entries),
            2,
            "two of four rows have a curve"
        );
        assert_eq!(entries.len(), 4, "the full corpus is the All count");
        assert_eq!(companion_count(&[]), 0);
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
            has_companion: false,
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
            has_companion: false,
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

    // ── auto-select the newest companion-bearing report (UX follow-on) ───────

    fn entry(stem: &str, has_companion: bool) -> ReportEntry {
        ReportEntry {
            slug: SmolStr::new("v0-paper-sma"),
            file_stem: SmolStr::new(stem),
            path: PathBuf::from(format!("/fixture/v0-paper-sma/reports/{stem}.md")),
            has_companion,
        }
    }

    /// The picker's real shape: a no-companion near-duplicate sorts ABOVE the
    /// one companion-bearing report (lexicographically `…20260527…` <
    /// `…20260617…`, but the older one has no companion). Auto-select MUST
    /// pick the companion-bearing row, never the higher-sorting no-companion
    /// one — the exact discoverability bug this fixes.
    #[test]
    fn newest_companion_index_skips_higher_sorting_no_companion() {
        let entries = vec![
            entry("backtest-20260527-000000-btc-2024-h1-sma-cross", false),
            entry("backtest-20260617-180015-btc-2024-h1-sma-cross", true),
        ];
        assert_eq!(
            newest_companion_index(&entries),
            Some(1),
            "must select the companion-bearing row, not the higher-sorting \
             no-companion near-duplicate above it"
        );
    }

    /// Among MULTIPLE companion-bearing rows, the newest (greatest stem) wins.
    #[test]
    fn newest_companion_index_picks_greatest_stem_among_companions() {
        let entries = vec![
            entry("backtest-20260101-000000-old", true),
            entry("backtest-20260901-000000-newest", true),
            entry("backtest-20260301-000000-mid", true),
            // A still-newer stem but NO companion — must be ignored.
            entry("backtest-20261231-235959-no-curve", false),
        ];
        assert_eq!(
            newest_companion_index(&entries),
            Some(1),
            "the greatest-stem companion-bearing row wins; a newer no-companion \
             row is ignored"
        );
    }

    /// No companion anywhere → `None` (the screen stays on the cold-start
    /// prompt — the pre-follow-on behaviour).
    #[test]
    fn newest_companion_index_none_when_no_companions() {
        let entries = vec![
            entry("backtest-20260101-000000-a", false),
            entry("backtest-20260202-000000-b", false),
        ];
        assert_eq!(newest_companion_index(&entries), None);
        assert_eq!(newest_companion_index(&[]), None);
    }
}
