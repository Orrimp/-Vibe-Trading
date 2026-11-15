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
//! 2. [`load_equity_companion`] — resolve the **stem-matched**
//!    `<dir>/artifacts/<report-file-stem>/equity-*.csv` (the native
//!    5-column schema), `Empty` when no companion exists. The run-id dir
//!    name is the report's own file stem, so the pairing is 1:1 — see the
//!    fn doc for why a first-match scan was the wrong contract.
//! 3. [`discover_reports`] — the all-slug scan of
//!    `evidence/[<container>/]<slug>/reports/backtest-*.md` (the
//!    [`crate::reports::layout`] contract, ADR-0055 D8 / bug-log #205), the
//!    corpus the picker browses; [`discover_reports_in`] takes the root. K2
//!    never-panic: an unreadable dir is skipped with a `tracing` breadcrumb;
//!    an absent `evidence/` yields an empty `Vec`. The `robustness-sweep-*.md`
//!    and `test-*.md` families are excluded by the `backtest-` filter.
//!
//! The `parse_front_matter` / `strip_front_matter` helpers round out the
//! lift so the bin keeps no local copy.

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use trading_core::{BacktestMetrics, EquitySeries, Money, Timestamp, Usdt};

use crate::reports::layout;
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
    // A parse that found a `## Summary` but recognised NO metric rows returns
    // the `BacktestMetrics::all_absent()` sentinel — genuinely-no-data. Since
    // the 2-15 review H2 fix that is carried by the STATE (`Empty`), not by a
    // zero-shaped `Ready`: `kpi_strip::view` now renders every `Ready` payload
    // verbatim, so a healthy-but-flat live strip can no longer be mistaken for
    // an empty one. Classifying here — at the seam that actually knows the
    // parse found nothing — keeps the viewer/Reports strip rendering its
    // honest "Backtest metrics unavailable" body for such a report.
    let metrics: PanelState<BacktestMetrics> = match reports::parse::parse_from_report(path) {
        Ok(m) if m.is_all_absent() => PanelState::Empty,
        Ok(m) => PanelState::Ready(m),
        Err(e) => PanelState::Error(smol_str::SmolStr::new(e.to_string())),
    };

    // Equity CSV — companion file at `<dir>/artifacts/<run_id>/equity-*.csv`.
    // When the companion is missing or unreadable, the equity curve /
    // drawdown band render their empty state independently of the KPI strip
    // (R11.3). Most committed reports ship no companion, so this is `Empty`
    // for them (the honest, accepted state per § Data contract).
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
/// **Stem-matched companion resolution (backtest-equity-companion v0.1.0).**
/// The emitter (`backtest::report::write_equity_companion`) writes the
/// companion at `<report_dir>/artifacts/<REPORT-FILE-STEM>/equity-*.csv` —
/// the run-id directory name **is** the report's own file stem
/// (`backtest-<stamp>-<scenario>`), per the architect's design. So we
/// resolve **only** that one matching-stem directory rather than scanning
/// every `artifacts/<X>/` and taking the first hit. The old first-match
/// scan paired the wrong companion to a report whenever an `artifacts/`
/// tree held more than one run-id directory (a correctness bug in the
/// shipped cockpit-reports-viewer loader); stem-matching makes the pairing
/// 1:1 with the report.
///
/// # Errors
///
/// Returns `Err(String)` when the matching-stem directory is unreadable or
/// the discovered CSV fails to parse / yields a curve `from_points`
/// rejects. A missing `artifacts/` dir, a missing matching-stem dir, or a
/// matching-stem dir with no `equity-*.csv` is **not** an error — it
/// returns `Ok(PanelState::Empty)`.
pub fn load_equity_companion(report_path: &Path) -> Result<PanelState<EquitySeries>, String> {
    let parent = report_path
        .parent()
        .ok_or_else(|| "report has no parent directory".to_string())?;
    // The emitter writes the companion under
    // `<parent>/artifacts/<report-file-stem>/equity-*.csv`. The run-id
    // directory name is the report's own file stem, so resolve ONLY that
    // matching-stem directory (never a first-match across all subdirs).
    let Some(stem) = report_path.file_stem().and_then(|s| s.to_str()) else {
        // A report path with no UTF-8 file stem can't have a stem-matched
        // companion — treat as Empty, never panic.
        return Ok(PanelState::Empty);
    };
    let stem_dir = parent.join("artifacts").join(stem);
    if !stem_dir.is_dir() {
        // No matching-stem artifacts dir → no companion for this report.
        return Ok(PanelState::Empty);
    }
    let mut candidate: Option<PathBuf> = None;
    let entries = std::fs::read_dir(&stem_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let ip = entry.path();
        if !ip.is_file() {
            continue;
        }
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

/// Cheap existence-only probe: does a stem-matched equity companion CSV exist
/// on disk for the report at `report_path`? (backtest-equity-companion UX
/// follow-on — drives the picker's "has-curve" marker + boot auto-select.)
///
/// Uses the **exact same stem-match convention** as
/// [`load_equity_companion`] — the companion lives at
/// `<report_dir>/artifacts/<report-file-stem>/equity-*.csv` — but stops at
/// *existence*: it never reads or parses the CSV (no `read_equity_csv`, no
/// `EquitySeries::from_points`), just a directory `is_dir()` check plus a
/// single `read_dir` scan for the first `equity-*.csv` filename. This keeps
/// discovery filename-cheap (one extra `stat` + at most one `read_dir` per
/// report) so the boot scan stays synchronous.
///
/// **K2 never-panic**: a path with no UTF-8 stem, an absent `artifacts/` dir,
/// an absent matching-stem dir, or an unreadable matching-stem dir all return
/// `false` — never an error, never a panic. (An unreadable dir is treated as
/// "no companion" rather than surfaced; the marker is an at-a-glance hint, so
/// a false negative degrades gracefully to "no marker", and the lazy
/// [`load_equity_companion`] path would still surface a real read error if the
/// row were selected.)
#[must_use]
pub fn report_has_companion(report_path: &Path) -> bool {
    let Some(parent) = report_path.parent() else {
        return false;
    };
    let Some(stem) = report_path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let stem_dir = parent.join("artifacts").join(stem);
    if !stem_dir.is_dir() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(&stem_dir) else {
        // Unreadable matching-stem dir → treat as no companion (no panic). The
        // lazy load path surfaces a real read error on selection if needed.
        tracing::debug!(
            path = %stem_dir.display(),
            "report_has_companion: matching-stem artifacts dir unreadable — treating as no companion"
        );
        return false;
    };
    for entry in entries.flatten() {
        let ip = entry.path();
        if !ip.is_file() {
            continue;
        }
        let name = ip.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Same equity-companion filename contract as `load_equity_companion`.
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let is_equity_csv = name.starts_with("equity-") && name.ends_with(".csv");
        if is_equity_csv {
            return true;
        }
    }
    false
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

/// The committed report root, `<workspace>/evidence` — the root
/// [`discover_reports`] and `reports::state::load_into` read.
pub(crate) fn evidence_root() -> PathBuf {
    workspace_root().join("evidence")
}

/// Discover every committed `backtest-*.md` report under `evidence/`, across
/// **all** slugs and both container depths (R1 / R7 / AC1; bug-log #205).
/// A thin wrapper over [`discover_reports_in`] at [`evidence_root`].
#[must_use]
pub fn discover_reports() -> Vec<ReportEntry> {
    discover_reports_in(&evidence_root())
}

/// Discover every `backtest-*.md` report under `root`, walked through the ONE
/// layout contract ([`crate::reports::layout`], ADR-0055 D8):
/// `<root>[/<container>]/<slug>/reports/backtest-*.md`, container depth ≤
/// [`crate::reports::layout::MAX_CONTAINER_DEPTH`]. Since `1395520629dc` the
/// committed corpus sits at container depth 1 (`evidence/<container>/<slug>/reports/`)
/// as well as container depth 0 (`evidence/<slug>/reports/`); a one-level walk
/// listed only the container-depth-0 reports, none of which ships an equity
/// companion (#205).
///
/// This is the corpus the Reports picker browses. (This is not the R4/AC5
/// parse — that concern is the markdown/CSV parse, shared via
/// [`load_report`]; discovery is a distinct filename-only concern.) Each
/// entry's `slug` is the leaf directory above `reports/`; its `path` carries
/// the container.
///
/// **K2 never-panic contract** (mirrors `models::registry_read` +
/// `baseline::loader`): an absent or unreadable root → empty `Vec` + a
/// `tracing` breadcrumb; an unreadable `reports/` dir → skipped with a
/// breadcrumb. Never panics.
///
/// The `robustness-sweep-*.md` and `test-*.md` families are excluded by the
/// `starts_with("backtest-") && ends_with(".md")` filter
/// ([`crate::reports::layout::is_backtest_report`], the same filter the Lab
/// uses) — they never match, so they are excluded by construction, not
/// silently dropped.
///
/// Result is sorted deterministically by `(slug, file_stem, path)` for stable
/// list ordering + reproducible snapshots (the `path` key orders the same stem
/// under two containers). This re-sort replaces the layout's (container depth,
/// path) order, so the D8 tie rule does not decide anything here.
#[must_use]
pub fn discover_reports_in(root: &Path) -> Vec<ReportEntry> {
    let mut out: Vec<ReportEntry> = Vec::new();
    for rd in layout::report_dirs(root) {
        for path in layout::backtest_files(&rd.dir) {
            // `file_stem` = `backtest-<YYYYMMDD-HHMMSS>-<scenario>` (drop `.md`).
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let file_stem = smol_str::SmolStr::new(stem);
            // Cheap existence-only companion probe (no CSV parse) so the
            // picker can mark which rows paint a curve + the boot can
            // auto-select the newest companion-bearing report.
            let has_companion = report_has_companion(&path);
            out.push(ReportEntry {
                slug: rd.slug.clone(),
                file_stem,
                path,
                has_companion,
            });
        }
    }
    out.sort_by(|a, b| {
        a.slug
            .cmp(&b.slug)
            .then_with(|| a.file_stem.cmp(&b.file_stem))
            .then_with(|| a.path.cmp(&b.path))
    });
    out
}

/// Workspace root, derived from this crate's manifest dir
/// (`<root>/crates/ui` → `<root>`). Single source of the base path so the
/// discovery scan resolves `evidence/` workspace-relative, never an absolute
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

    // ── discover_reports (AC1 / AC3 / K2; bug-log #205) ──────────────────────

    use std::collections::HashSet;

    use crate::reports::layout::test_tree::{self, TREE};

    use crate::test_support::evidence_oracle;

    /// The committed no-companion control: a `v0-paper-sma` report with NO
    /// `artifacts/<stem>/` path in the git listing (asserted, not assumed).
    const NO_COMPANION_STEM: &str = "backtest-20260527-143549-btc-2024-h1-sma-cross";

    /// D1 — on the shared fixture TREE, `discover_reports_in` lists exactly the
    /// rows the table marks listed (slug = the leaf dir, `has_companion` from
    /// the stem-matched artifacts dir), in `(slug, stem, path)` order. Replaces
    /// the old live-corpus property tests, which asserted only properties of
    /// whatever was found — 108 container-depth-0 entries satisfied them under
    /// #205.
    #[test]
    fn discover_reports_in_lists_exactly_the_declared_rows() {
        let tmp = tempfile::tempdir().unwrap();
        test_tree::build(tmp.path());
        let mut expected: Vec<(String, String, PathBuf, bool)> = TREE
            .iter()
            .filter_map(|r| {
                r.listed.map(|(slug, has_companion)| {
                    let path = tmp.path().join(r.path);
                    let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
                    (slug.to_string(), stem, path, has_companion)
                })
            })
            .collect();
        // Tuple order = (slug, stem, path); the flag never decides (the three
        // keys are unique per row).
        expected.sort();
        assert!(
            expected.len() >= 3,
            "the TREE must declare ≥3 listed rows, else D1 pins nothing"
        );
        let got: Vec<(String, String, PathBuf, bool)> = discover_reports_in(tmp.path())
            .into_iter()
            .map(|e| {
                (
                    e.slug.to_string(),
                    e.file_stem.to_string(),
                    e.path,
                    e.has_companion,
                )
            })
            .collect();
        assert_eq!(
            got, expected,
            "discover_reports_in must list exactly the declared rows in (slug, stem, path) order (#205)"
        );
    }

    /// K2: an absent root discovers nothing and never panics; the fixture
    /// root discovers something (two-sided).
    #[test]
    fn discover_reports_in_absent_root_is_empty_fixture_root_is_not() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(discover_reports_in(&tmp.path().join("absent")).is_empty());
        test_tree::build(tmp.path());
        assert!(!discover_reports_in(tmp.path()).is_empty());
    }

    /// The live-corpus oracle: every file under `evidence/` that git lists,
    /// tracked OR untracked on disk (`evidence_oracle::git_listed_files`, the
    /// ONE copy shared with the integration tests; bug-log #205). Read from
    /// git, never from the subject. A failing `git` PANICS — the oracle never
    /// skips (the #66 skip-as-pass shape that hid #205).
    fn git_listed_evidence_files() -> Vec<Vec<String>> {
        let ws = workspace_root();
        evidence_oracle::git_listed_files(&mut evidence_oracle::live_git(&ws))
            .unwrap_or_else(|e| panic!("the #205 evidence oracle must run: {e}"))
    }

    /// Repo-relative components of a discovered path (Windows-safe: compared
    /// component-wise, never as a `/`-joined string).
    fn rel_components(p: &Path) -> Vec<String> {
        p.strip_prefix(workspace_root())
            .unwrap_or_else(|_| {
                panic!(
                    "discovered path {} is not under the workspace root",
                    p.display()
                )
            })
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect()
    }

    /// D2 — every git-listed (tracked, or untracked on disk) in-contract report
    /// is discovered by the production `discover_reports()`, at container depth
    /// 0 AND 1 (#205: the container-depth-1 reports — every one with an equity
    /// companion — were not). Subset, not equality, so a stray file discovery
    /// lists cannot turn it red (#189); a deleted tracked report is still in the
    /// listing (`--cached`) and does turn it red (#66).
    #[test]
    fn discover_reports_covers_every_git_listed_report() {
        let listed: Vec<Vec<String>> = git_listed_evidence_files()
            .into_iter()
            .filter(|c| evidence_oracle::is_contract_report(c))
            .collect();
        let d0 = listed.iter().filter(|c| c.len() == 4).count();
        let d1 = listed.iter().filter(|c| c.len() == 5).count();
        assert!(
            d0 >= 1,
            "the oracle must hold ≥1 container-depth-0 report — else D2 pins nothing"
        );
        assert!(
            d1 >= 1,
            "the oracle must hold ≥1 container-depth-1 report — else D2 pins nothing"
        );
        let mut names: Vec<&str> = listed.iter().map(|c| c[c.len() - 1].as_str()).collect();
        names.sort_unstable();
        let n_names = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            n_names,
            "git-listed report file names must be unique (the stem is the picker's lookup key)"
        );
        let discovered: HashSet<Vec<String>> = discover_reports()
            .iter()
            .map(|e| rel_components(&e.path))
            .collect();
        let missing: Vec<String> = listed
            .iter()
            .filter(|c| !discovered.contains(*c))
            .map(|c| c.join("/"))
            .collect();
        assert!(
            missing.is_empty(),
            "{} of {} git-listed (tracked or on-disk) reports not discovered (#205); container \
             depth 0: {d0}, container depth 1: {d1}; first: {:?}",
            missing.len(),
            listed.len(),
            missing.first()
        );
    }

    /// D3 — the production `has_companion` flag equals the git-derived
    /// companion set: a git-listed report whose stem-matched
    /// `reports/artifacts/<stem>/equity-*.csv` is git-listed beside it
    /// (`evidence_oracle::companion_reports`, the rule R6 uses too). Both
    /// sides are asserted non-empty first.
    #[test]
    fn discover_reports_companion_flags_match_git_listed_companions() {
        let files = git_listed_evidence_files();
        let listed_set: HashSet<Vec<String>> = files.iter().cloned().collect();
        let reports = files
            .iter()
            .filter(|c| evidence_oracle::is_contract_report(c))
            .count();
        let derived: HashSet<Vec<String>> = evidence_oracle::companion_reports(&files)
            .into_iter()
            .collect();
        assert!(
            !derived.is_empty(),
            "the oracle must hold ≥1 derived companion — else D3 pins nothing"
        );
        assert!(
            reports > derived.len(),
            "the oracle must hold ≥1 report WITHOUT a companion — D3's other side"
        );
        let flagged: HashSet<Vec<String>> = discover_reports()
            .iter()
            .filter(|e| e.has_companion)
            .map(|e| rel_components(&e.path))
            .filter(|c| listed_set.contains(c))
            .collect();
        assert_eq!(
            flagged,
            derived,
            "the git-listed reports flagged has_companion must be exactly the {} git-derived \
             companions (#205)",
            derived.len()
        );
    }

    /// The `backtest-` filter excludes the other families by construction.
    #[test]
    fn is_backtest_report_filter() {
        use crate::reports::layout::is_backtest_report;
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

    /// **2-15 review H2.** A `## Summary` whose rows the parser recognises
    /// NONE of yields the `all_absent` sentinel — genuinely-no-data — and must
    /// surface as `metrics: Empty`, the state `kpi_strip::view` renders as
    /// "Backtest metrics unavailable". This is the seam that keeps the
    /// honest-dashes render alive now that a `Ready` payload always draws its
    /// values (so a healthy flat LIVE strip is no longer swallowed by it).
    #[test]
    fn load_report_unrecognised_summary_yields_metrics_empty() {
        let dir =
            std::env::temp_dir().join(format!("reports_loader_absent_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("backtest-20260101-000002-absent.md");
        let body = "---\nscenario: fixture\n---\n\
                    # Report\n\n\
                    ## Summary\n\n\
                    | Metric | Value |\n\
                    |--------|-------|\n\
                    | Scenario | btc-2023 |\n\n\
                    ## Notes\n\nno numeric rows at all\n";
        std::fs::write(&path, body).expect("write fixture");

        let r = load_report(&path).expect("load must succeed for a readable file");
        assert!(
            matches!(r.metrics, PanelState::Empty),
            "an all-absent parse → metrics Empty (not a zero-shaped Ready), got {}",
            r.metrics.variant_name()
        );
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

    // ── load_equity_companion stem-matching (backtest-equity-companion) ───────

    /// The minimal 5-column equity CSV body the canonical
    /// `reports::csv_artifacts::read_equity_csv` parses (RFC3339 `ts`,
    /// `Decimal` amount columns). Three bars is enough for
    /// `EquitySeries::from_points` to build a curve.
    const EQUITY_CSV_FIXTURE: &str = "ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt\n\
         2024-01-01T00:00:00Z,100000,0,0,0\n\
         2024-01-01T01:00:00Z,100500,0,0,0\n\
         2024-01-01T02:00:00Z,101250,0,0,0\n";

    /// A companion CSV under the **matching** `artifacts/<report-stem>/`
    /// directory resolves to `Ready` (the stem-match happy path). This is
    /// the populated-curve case the demo report exercises.
    #[test]
    fn load_equity_companion_matching_stem_dir_is_ready() {
        let dir = std::env::temp_dir().join(format!("eqcomp_match_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let stem = "backtest-20260101-000000-fixture-scn";
        let report_path = dir.join(format!("{stem}.md"));
        // Companion lives at <dir>/artifacts/<stem>/equity-*.csv (the
        // emitter's layout: the run-id dir name IS the report file stem).
        let comp_dir = dir.join("artifacts").join(stem);
        std::fs::create_dir_all(&comp_dir).expect("companion dir");
        std::fs::write(
            comp_dir.join("equity-20260101-000000.csv"),
            EQUITY_CSV_FIXTURE,
        )
        .expect("write companion");
        // The report .md itself need not exist for the companion resolver.

        let state = load_equity_companion(&report_path).expect("never Err for a valid companion");
        assert!(
            matches!(state, PanelState::Ready(_)),
            "companion under the matching-stem dir → Ready, got {}",
            state.variant_name()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **Regression guard for the first-match bug.** A companion exists, but
    /// only under a DIFFERENT report's stem directory (no dir matches this
    /// report's stem). The old first-match scan would have mis-paired that
    /// foreign companion to this report; stem-matching must return `Empty`.
    #[test]
    fn load_equity_companion_non_matching_stem_dir_is_empty() {
        let dir = std::env::temp_dir().join(format!("eqcomp_nomatch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // This report's stem — no artifacts/<this-stem>/ dir will exist.
        let this_stem = "backtest-20260101-000000-this-report";
        let report_path = dir.join(format!("{this_stem}.md"));
        // A companion DOES exist, but under a *different* report's stem dir.
        let other_stem = "backtest-20260202-111111-other-report";
        let other_dir = dir.join("artifacts").join(other_stem);
        std::fs::create_dir_all(&other_dir).expect("other companion dir");
        std::fs::write(
            other_dir.join("equity-20260202-111111.csv"),
            EQUITY_CSV_FIXTURE,
        )
        .expect("write foreign companion");

        let state =
            load_equity_companion(&report_path).expect("never Err when the matching dir is absent");
        assert!(
            matches!(state, PanelState::Empty),
            "a companion under a non-matching stem dir must NOT be paired (stem-match, \
             not first-match) → Empty, got {}",
            state.variant_name()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Real-demo smoke (backtest-equity-companion): the committed
    /// `btc-2024-h1-sma-cross` demo report has a stem-matched companion on
    /// disk, so it resolves to `Ready` (the populated-curve case AC3
    /// renders). The demo and its CSV are committed (not LFS), so a missing
    /// file FAILS — the old skip-if-absent guard could only ever hide a
    /// regression (bug-log #205).
    #[test]
    fn load_equity_companion_real_demo_report_is_ready() {
        let demo = workspace_root()
            .join("evidence/v1/v0-paper-sma/reports")
            .join("backtest-20260617-180015-btc-2024-h1-sma-cross.md");
        assert!(
            demo.is_file(),
            "the committed demo report must exist at {} (#205: committed, never pruned)",
            demo.display()
        );
        assert!(
            report_has_companion(&demo),
            "the committed demo report must probe has-companion (#205)"
        );
        let state =
            load_equity_companion(&demo).expect("real demo companion must read without Err");
        assert!(
            matches!(state, PanelState::Ready(_)),
            "the committed demo report's stem-matched companion → Ready, got {}",
            state.variant_name()
        );
    }

    /// The matching-stem dir exists but holds no `equity-*.csv` → `Empty`
    /// (no panic). Guards the "dir present, file absent" branch.
    #[test]
    fn load_equity_companion_matching_stem_dir_no_csv_is_empty() {
        let dir = std::env::temp_dir().join(format!("eqcomp_nocsv_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let stem = "backtest-20260101-000000-empty-dir";
        let report_path = dir.join(format!("{stem}.md"));
        let comp_dir = dir.join("artifacts").join(stem);
        std::fs::create_dir_all(&comp_dir).expect("companion dir");
        // A non-equity file in the matching dir must be ignored.
        std::fs::write(comp_dir.join("notes.txt"), "not a companion").expect("write decoy");

        let state = load_equity_companion(&report_path).expect("never Err for an empty dir");
        assert!(
            matches!(state, PanelState::Empty),
            "matching-stem dir with no equity-*.csv → Empty, got {}",
            state.variant_name()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── report_has_companion existence probe (UX follow-on) ──────────────────

    /// The existence probe mirrors `load_equity_companion`'s stem-match but
    /// stops at existence: a matching-stem dir holding an `equity-*.csv` →
    /// `true`. Same fixture layout as the loader happy-path test.
    #[test]
    fn report_has_companion_true_for_matching_stem_csv() {
        let dir = std::env::temp_dir().join(format!("hascomp_yes_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let stem = "backtest-20260101-000000-fixture-scn";
        let report_path = dir.join(format!("{stem}.md"));
        let comp_dir = dir.join("artifacts").join(stem);
        std::fs::create_dir_all(&comp_dir).expect("companion dir");
        std::fs::write(comp_dir.join("equity-20260101-000000.csv"), "ts\n").expect("write csv");

        assert!(
            report_has_companion(&report_path),
            "a matching-stem dir with an equity-*.csv must probe true"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A companion under a NON-matching stem dir must probe `false` (the same
    /// stem-match discipline as the loader — never a first-match false hit).
    #[test]
    fn report_has_companion_false_for_non_matching_stem() {
        let dir = std::env::temp_dir().join(format!("hascomp_nomatch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let this_stem = "backtest-20260101-000000-this";
        let report_path = dir.join(format!("{this_stem}.md"));
        let other_dir = dir.join("artifacts").join("backtest-20260202-111111-other");
        std::fs::create_dir_all(&other_dir).expect("other dir");
        std::fs::write(other_dir.join("equity-20260202-111111.csv"), "ts\n").expect("write csv");

        assert!(
            !report_has_companion(&report_path),
            "a companion under a non-matching stem dir must NOT count (stem-match, \
             not first-match)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A matching-stem dir present but holding no `equity-*.csv` → `false`.
    #[test]
    fn report_has_companion_false_for_dir_without_csv() {
        let dir = std::env::temp_dir().join(format!("hascomp_nocsv_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let stem = "backtest-20260101-000000-empty";
        let report_path = dir.join(format!("{stem}.md"));
        let comp_dir = dir.join("artifacts").join(stem);
        std::fs::create_dir_all(&comp_dir).expect("companion dir");
        std::fs::write(comp_dir.join("notes.txt"), "decoy").expect("write decoy");

        assert!(
            !report_has_companion(&report_path),
            "a matching-stem dir with no equity-*.csv must probe false"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A path with no `artifacts/` tree at all → `false`, never a panic (K2).
    #[test]
    fn report_has_companion_false_and_no_panic_for_absent_tree() {
        let bogus =
            workspace_root().join("definitely/not/real/reports/backtest-20260101-000000-x.md");
        assert!(
            !report_has_companion(&bogus),
            "an absent artifacts tree must probe false, never panic"
        );
    }

    /// Consistency invariant, two-sided, on the live corpus: the probe and the
    /// loader agree on the committed demo (true → `Ready`) AND on a committed
    /// no-companion report (false → `Empty`). No skip: both files are
    /// committed (bug-log #205); "no companion" is derived from the git
    /// listing (tracked or on disk), not from the probe under test.
    #[test]
    fn report_has_companion_agrees_with_loader_on_real_demo() {
        let demo = workspace_root()
            .join("evidence/v1/v0-paper-sma/reports")
            .join("backtest-20260617-180015-btc-2024-h1-sma-cross.md");
        assert!(
            demo.is_file(),
            "the committed demo report must exist at {} (#205)",
            demo.display()
        );
        assert!(
            report_has_companion(&demo),
            "the probe must say has-companion for the committed demo (#205)"
        );
        let state = load_equity_companion(&demo).expect("loader must not Err on the demo");
        assert!(
            matches!(state, PanelState::Ready(_)),
            "probe says has-companion → loader must resolve Ready, got {}",
            state.variant_name()
        );

        // Negative pair: a git-listed report with no git-listed artifacts/<stem>/.
        let files = git_listed_evidence_files();
        let dir: Vec<String> = ["evidence", "v1", "v0-paper-sma", "reports"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let mut report_rel = dir.clone();
        report_rel.push(format!("{NO_COMPANION_STEM}.md"));
        assert!(
            files.contains(&report_rel),
            "the no-companion control `{NO_COMPANION_STEM}` must be git-listed (tracked or on disk)"
        );
        let mut artifacts_rel = dir;
        artifacts_rel.push("artifacts".to_string());
        artifacts_rel.push(NO_COMPANION_STEM.to_string());
        assert!(
            !files.iter().any(|c| c.starts_with(&artifacts_rel)),
            "the git listing (tracked or on disk) must hold no artifacts/{NO_COMPANION_STEM}/ \
             path for the control"
        );
        let control = workspace_root()
            .join("evidence/v1/v0-paper-sma/reports")
            .join(format!("{NO_COMPANION_STEM}.md"));
        assert!(
            !report_has_companion(&control),
            "the probe must say NO companion for the committed control report"
        );
        let control_state =
            load_equity_companion(&control).expect("loader must not Err on the control");
        assert!(
            matches!(control_state, PanelState::Empty),
            "no companion → loader must resolve Empty, got {}",
            control_state.variant_name()
        );
    }
}
