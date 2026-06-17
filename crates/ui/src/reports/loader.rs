//! cockpit-reports-viewer v0.1.0 — shared report loader (D2 / R4 / AC5).
//!
//! Pure-`ui` data layer, **no new crate edge** (`ui` already depends on
//! `trading_core` + `reports` + `std::fs`). This module is the **one**
//! markdown/front-matter/CSV parse implementation that both the offline
//! `viewer` bin and the in-cockpit `Screen::Reports` call — it was lifted
//! verbatim out of `bin/viewer.rs` so the two surfaces can never drift
//! apart (the precondition for R4/AC5). The bin now `use`s these fns.
//!
//! Three responsibilities:
//!
//! 1. [`load_report`] — read a `backtest-*.md`, parse its `## Summary` KPI
//!    table (graceful `Error` on a malformed body, never a panic) + the
//!    companion equity CSV (`Empty` when absent — the common case today).
//! 2. [`load_equity_companion`] — scan `<dir>/artifacts/<run_id>/equity-*.csv`
//!    (the native 5-column schema), `Empty` when no companion exists.
//! 3. [`discover_reports`] — a **new** all-slug scan of
//!    `spec/*/reports/backtest-*.md`, the corpus the picker browses. K2
//!    never-panic: an unreadable dir is skipped with a `tracing` breadcrumb;
//!    an absent `spec/` yields an empty `Vec`. The `robustness-sweep-*.md`
//!    and `test-*.md` families are excluded by the `backtest-` filter.
//!
//! The `parse_front_matter` / `strip_front_matter` helpers round out the
//! lift so the bin keeps no local copy.

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use trading_core::{BacktestMetrics, EquitySeries, Money, Timestamp, Usdt};

use crate::reports::state::ReportEntry;
use crate::state::PanelState;
use crate::viewer::{ReportFrontMatter, ReportLoadResult};

/// Synchronous load. Reads the markdown body + parses the KPI table
/// + reads the companion equity CSV (when present).
///
/// **Lifted verbatim from `bin/viewer.rs:136` (D2 / AC5).** Field-level
/// errors degrade independently: a malformed `## Summary` flips the
/// `metrics` field to `PanelState::Error`, a missing companion CSV flips
/// the `equity` field to `PanelState::Empty` — neither invalidates the
/// other, and neither panics.
///
/// # Errors
///
/// Returns the underlying `std::io::Error` only when the report file
/// itself cannot be read (missing on disk / permissions). A malformed
/// body is NOT an error here — it surfaces as `metrics: Error` in the
/// returned [`ReportLoadResult`].
pub fn load_report(path: &Path) -> Result<ReportLoadResult, std::io::Error> {
    let raw = std::fs::read_to_string(path)?;

    let front_matter = parse_front_matter(&raw);

    // KPI metrics — graceful fallback to `PanelState::Error(msg)` when the
    // parser hits a malformed body (R3.5 / Q3 graceful fallback). The strip
    // then renders the unavailable state with the muted body.
    let metrics: PanelState<BacktestMetrics> = match reports::parse::parse_from_report(path) {
        Ok(m) => PanelState::Ready(m),
        Err(e) => PanelState::Error(smol_str::SmolStr::new(e.to_string())),
    };

    // Equity CSV — companion file at `<dir>/artifacts/<run_id>/equity-*.csv`.
    // When the companion is missing or unreadable, the equity curve /
    // drawdown band render their empty state independently of the KPI strip
    // (R11.3). No `backtest-*.md` in the current corpus ships this companion,
    // so this is `Empty` for every report today (the honest, accepted state
    // per § Data contract).
    let equity = load_equity_companion(path)
        .unwrap_or_else(|e| PanelState::Error(smol_str::SmolStr::new(e.as_str())));

    let body_markdown = strip_front_matter(&raw).to_string();

    Ok(ReportLoadResult {
        front_matter,
        metrics,
        equity,
        body_markdown,
    })
}

/// Locate and read the companion equity CSV. Returns
/// `Ok(PanelState::Ready(series))` on success, `Ok(PanelState::Empty)`
/// when no companion exists, `Err(...)` on read / parse failure.
///
/// **Lifted verbatim from `bin/viewer.rs:172` (D2 / AC5).**
///
/// # Errors
///
/// Returns `Err(String)` when the `artifacts/` directory is unreadable or
/// the discovered CSV fails to parse / yields a curve `from_points`
/// rejects. A missing `artifacts/` dir or zero matching files is **not**
/// an error — it returns `Ok(PanelState::Empty)`.
pub fn load_equity_companion(report_path: &Path) -> Result<PanelState<EquitySeries>, String> {
    let parent = report_path
        .parent()
        .ok_or_else(|| "report has no parent directory".to_string())?;
    // The reports binary writes the companion under
    // `<parent>/artifacts/<run_id>/equity-*.csv`. We don't have the
    // run_id here; scan for the first `equity-*.csv` under any
    // run-id folder. If none, return Empty.
    let artifacts_root = parent.join("artifacts");
    if !artifacts_root.exists() {
        return Ok(PanelState::Empty);
    }
    let mut candidate: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&artifacts_root).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        if let Ok(inner) = std::fs::read_dir(&p) {
            for inner_entry in inner.flatten() {
                let ip = inner_entry.path();
                let name = ip.file_name().and_then(|n| n.to_str()).unwrap_or("");
                // Equity companion files are always written with a lowercase
                // `.csv` extension (the `reports` crate's writer contract).
                #[allow(clippy::case_sensitive_file_extension_comparisons)]
                let is_equity_csv = name.starts_with("equity-") && name.ends_with(".csv");
                if is_equity_csv {
                    candidate = Some(ip);
                    break;
                }
            }
        }
        if candidate.is_some() {
            break;
        }
    }
    let Some(csv_path) = candidate else {
        return Ok(PanelState::Empty);
    };
    let samples = reports::csv_artifacts::read_equity_csv(&csv_path).map_err(|e| e.to_string())?;
    if samples.is_empty() {
        return Ok(PanelState::Empty);
    }
    let points: Vec<(Timestamp, Money<Usdt>)> = samples
        .into_iter()
        .map(|s| (s.ts, Money::<Usdt>::from_decimal(s.equity_total)))
        .collect();
    let series = EquitySeries::from_points(points).map_err(|e| e.to_string())?;
    // Q5 — cap at 2000 points for paint budget.
    let series = series.downsample(2000);
    Ok(PanelState::Ready(series))
}

/// Parse the `scenario:` field out of the YAML front-matter.
///
/// **Lifted verbatim from `bin/viewer.rs:223` (D2 / AC5).**
#[must_use]
pub fn parse_front_matter(raw: &str) -> ReportFrontMatter {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        return ReportFrontMatter::default();
    }
    let after = match trimmed.find('\n') {
        Some(n) => &trimmed[n + 1..],
        None => return ReportFrontMatter::default(),
    };
    let end = after.find("\n---").unwrap_or(after.len());
    let yaml = &after[..end];
    let mut scenario = smol_str::SmolStr::default();
    for line in yaml.lines() {
        if let Some(rest) = line.strip_prefix("scenario:") {
            scenario = smol_str::SmolStr::new(rest.trim());
            break;
        }
    }
    ReportFrontMatter { scenario }
}

/// Strip the YAML front-matter block, returning the markdown body.
///
/// **Lifted verbatim from `bin/viewer.rs:244` (D2 / AC5).**
#[must_use]
pub fn strip_front_matter(raw: &str) -> &str {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        return trimmed;
    }
    let after_first = match trimmed.find('\n') {
        Some(n) => &trimmed[n + 1..],
        None => return trimmed,
    };
    if let Some(rel) = after_first.find("\n---") {
        let rest = &after_first[rel + 1..];
        if let Some(nl) = rest.find('\n') {
            return &rest[nl + 1..];
        }
    }
    trimmed
}

/// Discover every committed `backtest-*.md` report under
/// `spec/*/reports/`, across **all** feature slugs (R1 / R7 / AC1).
///
/// This is the corpus the Reports picker browses. It is a **new** top-level
/// scan — the existing `lab::equity_loader::discover_reports` is private +
/// per-slug, so it is not directly reusable; this walks all slugs. (This is
/// not the R4/AC5 parse — that concern is the markdown/CSV parse, shared via
/// [`load_report`]; discovery is a distinct filename-only concern.)
///
/// **K2 never-panic contract** (mirrors `models::registry_read` +
/// `baseline::loader`): an unreadable `spec/` root → empty `Vec` + a
/// `tracing::debug!` breadcrumb; an unreadable per-slug `reports/` dir →
/// skipped with a breadcrumb. Never panics.
///
/// The `robustness-sweep-*.md` and `test-*.md` families are excluded by the
/// `starts_with("backtest-") && ends_with(".md")` filter (the same filter
/// `lab::equity_loader` uses) — they never match, so they are excluded by
/// construction, not silently dropped.
///
/// Result is sorted deterministically by `(slug, file_stem)` for stable
/// list ordering + reproducible snapshots.
#[must_use]
pub fn discover_reports() -> Vec<ReportEntry> {
    let spec_root = workspace_root().join("spec");
    let Ok(slug_dirs) = std::fs::read_dir(&spec_root) else {
        tracing::debug!(
            path = %spec_root.display(),
            "discover_reports: spec/ not found or unreadable — returning empty list"
        );
        return Vec::new();
    };

    let mut out: Vec<ReportEntry> = Vec::new();
    for slug_entry in slug_dirs.flatten() {
        let slug_path = slug_entry.path();
        if !slug_path.is_dir() {
            continue;
        }
        let Some(slug) = slug_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let reports_dir = slug_path.join("reports");
        if !reports_dir.is_dir() {
            continue;
        }
        let Ok(report_files) = std::fs::read_dir(&reports_dir) else {
            tracing::debug!(
                path = %reports_dir.display(),
                "discover_reports: reports/ dir unreadable — skipping slug"
            );
            continue;
        };
        for file_entry in report_files.flatten() {
            let file_path = file_entry.path();
            let Some(name) = file_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !is_backtest_report(name) {
                continue;
            }
            // `file_stem` = `backtest-<YYYYMMDD-HHMMSS>-<scenario>` (drop `.md`).
            let stem = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            out.push(ReportEntry {
                slug: smol_str::SmolStr::new(slug),
                file_stem: smol_str::SmolStr::new(stem),
                path: file_path.clone(),
            });
        }
    }

    // Deterministic order: by slug, then by file_stem (carries the date).
    out.sort_by(|a, b| {
        a.slug
            .cmp(&b.slug)
            .then_with(|| a.file_stem.cmp(&b.file_stem))
    });
    out
}

/// The established `backtest-*.md` filter (per `lab/equity_loader.rs:261`):
/// only files whose name `starts_with("backtest-")` and ends with the
/// lowercase `.md` extension. Excludes `robustness-sweep-*.md` +
/// `test-*.md` by construction.
fn is_backtest_report(name: &str) -> bool {
    name.starts_with("backtest-") && {
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let ok = name.ends_with(".md");
        ok
    }
}

/// Workspace root, derived from this crate's manifest dir
/// (`<root>/crates/ui` → `<root>`). Single source of the base path so the
/// discovery scan resolves `spec/` workspace-relative, never an absolute
/// hard-code (mirrors `baseline/loader.rs:234`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // ── parse_front_matter (moved from bin/viewer.rs:343 — AC5) ──────────────

    /// The `scenario:` field is extracted from the YAML front-matter. This
    /// assertion is **moved verbatim** from the `viewer` bin's
    /// `parse_front_matter_extracts_scenario` test — it must survive the
    /// lift (AC5).
    #[test]
    fn parse_front_matter_extracts_scenario() {
        let raw = "---\nscenario: btc-2023-1m-rsi-reversion\nseed: 0xC0FFEE\n---\n# Body\n";
        let fm = parse_front_matter(raw);
        assert_eq!(fm.scenario.as_str(), "btc-2023-1m-rsi-reversion");
    }

    /// No front-matter → default (empty scenario); body returned intact.
    #[test]
    fn parse_and_strip_no_front_matter() {
        let raw = "# Just a heading\nbody line\n";
        assert_eq!(parse_front_matter(raw).scenario.as_str(), "");
        assert_eq!(strip_front_matter(raw), raw);
    }

    /// Front-matter is stripped; the body after the closing `---` survives.
    #[test]
    fn strip_front_matter_returns_body() {
        let raw = "---\nscenario: x\n---\n# Summary\nrow\n";
        assert_eq!(strip_front_matter(raw), "# Summary\nrow\n");
    }

    // ── discover_reports (AC1 / AC3 / K2) ────────────────────────────────────

    /// Discovery finds `backtest-*.md` and **excludes** the
    /// `robustness-sweep-*.md` + `test-*.md` families (AC1). Gated on
    /// `spec/` being present so a minimal checkout that omits the reports
    /// tree does not fail this unit test (the absent-root path is covered
    /// separately below) — mirrors `baseline/loader.rs`'s
    /// `committed_csvs_load_to_ready` skip-if-absent guard.
    #[test]
    fn discover_finds_backtest_excludes_other_families() {
        let spec_root = workspace_root().join("spec");
        if !spec_root.is_dir() {
            // Minimal checkout — skip; the absent-root path is tested below.
            return;
        }
        let entries = discover_reports();
        // Every discovered entry is a `backtest-*` stem (never sweep/test).
        for e in &entries {
            assert!(
                e.file_stem.starts_with("backtest-"),
                "only backtest-* reports are listed; got {}",
                e.file_stem
            );
            assert!(
                !e.file_stem.starts_with("robustness-sweep-"),
                "robustness-sweep family must be excluded"
            );
            assert!(
                !e.file_stem.starts_with("test-"),
                "test-report family must be excluded"
            );
            // The held PathBuf points at the discovered file on disk.
            assert!(
                e.path.exists(),
                "discovered path must exist: {}",
                e.path.display()
            );
        }
    }

    /// Discovery is deterministically sorted by `(slug, file_stem)` so the
    /// list order + snapshots are stable across runs (AC1). Gated on
    /// `spec/` present.
    #[test]
    fn discover_is_deterministically_sorted() {
        let spec_root = workspace_root().join("spec");
        if !spec_root.is_dir() {
            return;
        }
        let entries = discover_reports();
        let mut sorted = entries.clone();
        sorted.sort_by(|a, b| {
            a.slug
                .cmp(&b.slug)
                .then_with(|| a.file_stem.cmp(&b.file_stem))
        });
        let keys: Vec<(&str, &str)> = entries
            .iter()
            .map(|e| (e.slug.as_str(), e.file_stem.as_str()))
            .collect();
        let sorted_keys: Vec<(&str, &str)> = sorted
            .iter()
            .map(|e| (e.slug.as_str(), e.file_stem.as_str()))
            .collect();
        assert_eq!(
            keys, sorted_keys,
            "discover_reports must be (slug, stem)-sorted"
        );
    }

    /// The `backtest-` filter excludes the other families by construction.
    #[test]
    fn is_backtest_report_filter() {
        assert!(is_backtest_report("backtest-20260420-152017-btc.md"));
        assert!(!is_backtest_report("robustness-sweep-20260420.md"));
        assert!(!is_backtest_report("test-week2-smoke.md"));
        assert!(!is_backtest_report("backtest-foo.txt"));
        assert!(!is_backtest_report("feature.md"));
    }

    // ── load_report behaviour (AC2 / AC3) ────────────────────────────────────

    /// A valid `## Summary` fixture → `metrics: Ready`; no companion CSV →
    /// `equity: Empty`; the body strips the front-matter (AC2). Writes a
    /// temp fixture so the test is checkout-independent.
    #[test]
    fn load_report_valid_summary_ready_no_companion_empty() {
        let dir = std::env::temp_dir().join(format!("reports_loader_ok_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("backtest-20260101-000000-fixture.md");
        // A `## Summary` table `reports::parse::parse_from_report` reads.
        let body = "---\nscenario: fixture\n---\n\
                    # Report\n\n\
                    ## Summary\n\n\
                    | Metric | Value |\n\
                    |--------|-------|\n\
                    | Total return | 12.34% |\n\
                    | Sharpe | 1.10 |\n\
                    | Max drawdown | 5.00% |\n\
                    | Trades | 7 |\n\n\
                    ## Notes\n\nsome prose\n";
        std::fs::write(&path, body).expect("write fixture");

        let r = load_report(&path).expect("load must succeed for a readable file");
        assert!(
            matches!(r.metrics, PanelState::Ready(_)),
            "valid ## Summary → metrics Ready, got {}",
            r.metrics.variant_name()
        );
        // No `artifacts/` dir → equity Empty (the common corpus case).
        assert!(
            matches!(r.equity, PanelState::Empty),
            "no companion CSV → equity Empty, got {}",
            r.equity.variant_name()
        );
        // Body has the front-matter stripped.
        assert!(r.body_markdown.starts_with("# Report"));
        assert!(!r.body_markdown.contains("scenario: fixture"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `## Summary`-less body → `metrics: Error(NoSummaryHeading)`, **no
    /// panic** (AC3). The KPI strip then renders its muted Error body.
    #[test]
    fn load_report_no_summary_yields_metrics_error_no_panic() {
        let dir = std::env::temp_dir().join(format!("reports_loader_bad_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("backtest-20260101-000001-nosummary.md");
        std::fs::write(
            &path,
            "---\nscenario: x\n---\n# No summary table here\nbody\n",
        )
        .expect("write fixture");

        let r = load_report(&path).expect("load reads the file even with a bad body");
        assert!(
            matches!(r.metrics, PanelState::Error(_)),
            "missing ## Summary → metrics Error, got {}",
            r.metrics.variant_name()
        );
        // Equity is still independently Empty (no companion) — field-level
        // degrade, not a cascade.
        assert!(matches!(r.equity, PanelState::Empty));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A report missing on disk → `load_report` returns the io `Err` (the
    /// caller maps this to the screen's Error state); never panics (AC3).
    #[test]
    fn load_report_missing_file_is_err_no_panic() {
        let bogus = workspace_root().join("definitely/not/a/real/backtest-x.md");
        assert!(
            load_report(&bogus).is_err(),
            "a missing report file must surface as Err, not a panic"
        );
    }
}
