//! bug-log #210 — where the `backtest` bin (`src/main.rs`) writes its reports,
//! and where it reads the baseline a scenario declares (ADR-0104).
//!
//! - **Writes** default to [`CLI_OUTPUT_ROOT`]`/<feature>/reports/`
//!   ([`report_dir_for_scenario`]), relative to the working directory. That root
//!   is git-ignored (the unanchored `backtest-runs/` in `.gitignore`), so from any
//!   working directory outside `evidence/` it lies outside every `evidence/**`
//!   glob and a re-run can never plant a body that `scripts/verify_anchors.sh`
//!   compares. Writing INTO `evidence/` takes an explicit `--reports-dir`.
//! - **Baselines** are read from the committed corpus under [`EVIDENCE_ROOT`],
//!   at container depth ≤ [`CORPUS_MAX_CONTAINER_DEPTH`]
//!   ([`corpus_reports_dir`]), never from the output root: a committed report
//!   must not name a git-ignored file. A scenario that declares a baseline and
//!   finds none is refused ([`resolve_baseline`]), never written as
//!   `baseline_report: n/a` (the `#66` skip-as-pass shape).
//!
//! The scope is the `backtest` bin only. `run_yahoo_sma`, `monte_carlo` and
//! `param_robustness_sweep` keep their own `--reports-dir` / `--out-dir`
//! defaults (bug-log `#227`).
//!
//! The module lives in the lib, not in `main.rs`, so the integration test
//! `tests/cli_report_paths.rs` can call the same functions the bin calls.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The default root of the CLI's report output, relative to the working
/// directory (like `data/binance` and `config/`). Git-ignored (`backtest-runs/`,
/// unanchored, carved out under `evidence/` so a run started there stays visible).
pub const CLI_OUTPUT_ROOT: &str = "backtest-runs";

/// The committed evidence corpus, relative to the working directory. Only read.
pub const EVIDENCE_ROOT: &str = "evidence";

/// Container levels searched above `<feature>/reports` in the corpus:
/// 0 = `evidence/<feature>/reports`, 1 = `evidence/<container>/<feature>/reports`
/// (the committed layout since the `spec/v1` reorganisation `1395520629dc`).
///
/// It is ADR-0055 D8's bound, `ui::reports::layout::MAX_CONTAINER_DEPTH`,
/// re-implemented here because `backtest` cannot depend on `ui`. The two are
/// pinned equal by a `ui` lib test (`layout.rs`, LY-210).
pub const CORPUS_MAX_CONTAINER_DEPTH: usize = 1;

/// The directory name that holds reports; never a container.
const REPORTS_DIR: &str = "reports";

/// The scenarios that compare themselves against another scenario's report,
/// as `(scenario, baseline scenario)`. The bin writes the baseline's file name
/// into the report's `baseline_report:` front-matter line.
///
/// Pinned against the committed corpus by `tests/cli_report_paths.rs` R2:
/// the set of `(scenario:, baseline_report:)` pairs the committed reports
/// carry equals this table.
pub const BASELINE_DECLARATIONS: &[(&str, &str)] = &[
    ("btc-2024-h1-sma-cross", "btc-2023-1m-sma-cross"),
    ("btc-2023-1m-macd-trend", "btc-2023-1m-sma-baseline-refresh"),
    (
        "btc-2023-1m-rsi-reversion",
        "btc-2023-1m-sma-baseline-refresh",
    ),
    (
        "btc-2023-1m-bbands-mean-revert",
        "btc-2023-1m-sma-baseline-refresh",
    ),
];

/// Why a path could not be resolved. Each message names the path it looked at.
#[derive(Debug, thiserror::Error)]
pub enum CliPathError {
    /// No `reports/` dir for `feature` at any container depth ≤ `max` under `root`.
    #[error(
        "no corpus home for feature `{feature}` under `{root}`: neither `<root>/{feature}/reports` \
         nor `<root>/<container>/{feature}/reports` (container depth <= {max}) is a directory"
    )]
    NoCorpusHome {
        /// The feature slug looked for.
        feature: String,
        /// The corpus root searched.
        root: PathBuf,
        /// The container-depth bound searched.
        max: usize,
    },
    /// More than one `reports/` dir for `feature`: a stray twin, refused.
    #[error(
        "feature `{feature}` has {} corpus homes and exactly one is required: {homes:?}",
        homes.len()
    )]
    TwoCorpusHomes {
        /// The feature slug looked for.
        feature: String,
        /// Every home found, in search order.
        homes: Vec<PathBuf>,
    },
    /// The baseline's home exists and holds no `backtest-*-<baseline>.md`.
    #[error(
        "scenario `{scenario}` declares the baseline `{baseline}`, and `{dir}` holds no \
         `backtest-*-{baseline}.md`"
    )]
    BaselineMissing {
        /// The scenario being run.
        scenario: String,
        /// The baseline scenario it declares.
        baseline: String,
        /// The corpus dir searched.
        dir: PathBuf,
    },
    /// The baseline's home could not be found or read. Wraps the cause so a
    /// refusal always names the scenario and its declared baseline.
    #[error("scenario `{scenario}` declares the baseline `{baseline}`, which cannot be resolved")]
    BaselineUnresolved {
        /// The scenario being run.
        scenario: String,
        /// The baseline scenario it declares.
        baseline: String,
        /// Why it could not be resolved.
        #[source]
        source: Box<CliPathError>,
    },
    /// A directory could not be listed (other than an absent corpus root).
    #[error("cannot list `{dir}`: {source}")]
    Unreadable {
        /// The directory that failed.
        dir: PathBuf,
        /// The I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Map a backtest scenario name to the feature slug that owns its reports.
/// Fixed mapping (each scenario was first locked into `evidence/anchors.toml`
/// under exactly one feature). Future scenarios that don't match return
/// `"_unknown"`, which causes [`report_dir_for_scenario`] to write under
/// `backtest-runs/_unknown/reports/` so orphaned reports surface immediately.
#[must_use]
pub fn scenario_to_feature(scenario: &str) -> &'static str {
    match scenario {
        "btc-2023-1m-sma-cross"
        | "btc-2023-1m-sma-baseline-refresh"
        | "btc-2024-h1-sma-cross"
        | "eth-2024-h1-sma-cross"
        | "sol-2024-h1-sma-cross"
        | "bnb-2024-h1-sma-cross"
        | "xrp-2024-h1-sma-cross"
        | "link-2024-h1-sma-cross"
        | "ada-2024-h1-sma-cross"
        // Group A-ext: AVAX, DOT, DOGE 2024 full-year
        | "avax-2024-sma-cross"
        | "dot-2024-sma-cross"
        | "doge-2024-sma-cross"
        // Group A-broaduni: NEAR, OP, ARB, ATOM, SUI 2024 full-year
        | "near-2024-sma-cross"
        | "op-2024-sma-cross"
        | "arb-2024-sma-cross"
        | "atom-2024-sma-cross"
        | "sui-2024-sma-cross" => "v0-paper-sma",
        "btc-2023-1m-macd-trend"
        | "btc-2023-1m-rsi-reversion"
        | "btc-2023-1m-bbands-mean-revert" => "v05-composed-strategies",
        "top10-2023-1h-momentum" | "top10-2024-h1-momentum" => "v1-cross-sectional-momentum",
        "pairs-2023-zscore-mr" | "pairs-2024-h1-zscore-mr" => "v15a-mean-reversion-pairs",
        "top10-2023-fy-tcn-overlay"
        | "top10-2024-fy-tcn-overlay"
        | "top10-2023-fy-tcn-overlay-weights"
        | "top10-2024-fy-tcn-overlay-weights" => "v25-tcn-overlay",
        // v2.6.0-realdata scenarios (ADR-0032).
        "top10-2023-fy-tcn-overlay-realdata"
        | "top10-2024-fy-tcn-overlay-realdata"
        | "top10-2023-fy-tcn-overlay-weights-realdata"
        | "top10-2024-fy-tcn-overlay-weights-realdata" => "backtest-real-binance-data",
        // v2.5a PatchTST overlay realdata scenario (T-D-N23).
        "top10-2023-fy-patchtst-overlay-realdata" => "v25a-patchtst-overlay",
        // v3.0.0-volatility GARCH vol-targeting overlay realdata scenario (T-D-N24).
        "top10-2023-fy-vol-target-overlay-realdata" => "v3-volatility-forecaster",
        // v3.0.0-volatility-rebaseline: un-targeted v1 momentum on real Binance data.
        "top10-2023-fy-momentum-realdata" => "v3-volatility-forecaster-rebaseline",
        // v3.0.0-regime RegimeDispatcher scenarios (Wave E T-D-E1).
        "top10-2023-fy-regime-dispatcher-realdata" | "top10-2024-fy-regime-dispatcher-realdata" => {
            "v3-regime-classifier"
        }
        _ => "_unknown",
    }
}

/// The default report directory for `scenario`:
/// `backtest-runs/<feature>/reports/`, relative to the working directory.
/// `--reports-dir` overrides it. Caller is responsible for `create_dir_all`.
#[must_use]
pub fn report_dir_for_scenario(scenario: &str) -> PathBuf {
    PathBuf::from(CLI_OUTPUT_ROOT)
        .join(scenario_to_feature(scenario))
        .join(REPORTS_DIR)
}

/// The baseline scenario `scenario` declares in [`BASELINE_DECLARATIONS`], if any.
#[must_use]
pub fn baseline_scenario_for(scenario: &str) -> Option<&'static str> {
    BASELINE_DECLARATIONS
        .iter()
        .find(|(s, _)| *s == scenario)
        .map(|(_, b)| *b)
}

/// Sorted child directories of `dir`, never one named `reports`. An absent
/// `dir` has none.
fn child_dirs(dir: &Path) -> Result<Vec<PathBuf>, CliPathError> {
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(CliPathError::Unreadable {
                dir: dir.to_path_buf(),
                source,
            });
        }
    };
    let mut out = Vec::new();
    for entry in rd {
        let entry = entry.map_err(|source| CliPathError::Unreadable {
            dir: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() && path.file_name() != Some(OsStr::new(REPORTS_DIR)) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// The ONE `reports/` dir of `feature` in the corpus under `evidence_root`:
/// `<root>/<feature>/reports` (container depth 0) or
/// `<root>/<container>/<feature>/reports` (container depth 1), where no
/// container is named `reports`. Nothing deeper than
/// [`CORPUS_MAX_CONTAINER_DEPTH`] is searched.
///
/// # Errors
///
/// [`CliPathError::NoCorpusHome`] when there is none,
/// [`CliPathError::TwoCorpusHomes`] when there are several (a stray twin is
/// refused, never silently picked), and [`CliPathError::Unreadable`] when a
/// directory exists but cannot be listed.
pub fn corpus_reports_dir(evidence_root: &Path, feature: &str) -> Result<PathBuf, CliPathError> {
    let mut containers = vec![evidence_root.to_path_buf()];
    let mut level = vec![evidence_root.to_path_buf()];
    for _ in 1..=CORPUS_MAX_CONTAINER_DEPTH {
        let mut next = Vec::new();
        for dir in &level {
            next.extend(child_dirs(dir)?);
        }
        containers.extend(next.iter().cloned());
        level = next;
    }
    let mut homes: Vec<PathBuf> = containers
        .iter()
        .map(|c| c.join(feature).join(REPORTS_DIR))
        .filter(|d| d.is_dir())
        .collect();
    match homes.len() {
        0 => Err(CliPathError::NoCorpusHome {
            feature: feature.to_string(),
            root: evidence_root.to_path_buf(),
            max: CORPUS_MAX_CONTAINER_DEPTH,
        }),
        1 => Ok(homes.remove(0)),
        _ => Err(CliPathError::TwoCorpusHomes {
            feature: feature.to_string(),
            homes,
        }),
    }
}

/// The newest `backtest-*-<scenario>.md` FILE directly in `dir`, by file name:
/// the anchor gate's own `backtest-*-$scenario.md` match, so
/// `backtest-<stamp>-<scenario>-v2.md` and `backtest-<stamp>-x<scenario>.md`
/// are not `scenario`'s reports.
///
/// # Errors
///
/// [`CliPathError::Unreadable`] when `dir` cannot be listed (an absent `dir`
/// included): a failed listing is never "no report".
pub fn latest_report_named(dir: &Path, scenario: &str) -> Result<Option<String>, CliPathError> {
    let unreadable = |source| CliPathError::Unreadable {
        dir: dir.to_path_buf(),
        source,
    };
    let suffix = format!("-{scenario}.md");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let matches = name
            .strip_prefix("backtest-")
            .is_some_and(|rest| rest.ends_with(&suffix));
        if matches && entry.path().is_file() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names.pop())
}

/// The `baseline_report:` value for `scenario`: the file name of the newest
/// committed report of the baseline it declares, or `None` when it declares
/// none (then the disk is not touched).
///
/// # Errors
///
/// [`CliPathError::BaselineMissing`] when the baseline's corpus dir holds no
/// report of it, and [`CliPathError::BaselineUnresolved`] (wrapping the cause)
/// when that dir cannot be found or read. Every error names `scenario` and its
/// baseline. The bin calls this before any data load or write, so a refusal
/// writes nothing.
pub fn resolve_baseline(
    evidence_root: &Path,
    scenario: &str,
) -> Result<Option<String>, CliPathError> {
    let Some(baseline) = baseline_scenario_for(scenario) else {
        return Ok(None);
    };
    let unresolved = |source: CliPathError| CliPathError::BaselineUnresolved {
        scenario: scenario.to_string(),
        baseline: baseline.to_string(),
        source: Box::new(source),
    };
    let dir =
        corpus_reports_dir(evidence_root, scenario_to_feature(baseline)).map_err(unresolved)?;
    match latest_report_named(&dir, baseline).map_err(unresolved)? {
        Some(name) => Ok(Some(name)),
        None => Err(CliPathError::BaselineMissing {
            scenario: scenario.to_string(),
            baseline: baseline.to_string(),
            dir,
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const SMA_FEATURE: &str = "v0-paper-sma";
    const MACD: &str = "btc-2023-1m-macd-trend";
    const REFRESH: &str = "btc-2023-1m-sma-baseline-refresh";

    fn mkdir(p: &Path) -> PathBuf {
        std::fs::create_dir_all(p).unwrap();
        p.to_path_buf()
    }

    fn touch(p: &Path) {
        std::fs::write(p, b"").unwrap();
    }

    /// C1 — the committed layout: `evidence/v1/<feature>/reports` is the home.
    #[test]
    fn corpus_dir_found_at_container_depth_1() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("evidence");
        let home = mkdir(&root.join("v1").join(SMA_FEATURE).join("reports"));
        assert_eq!(corpus_reports_dir(&root, SMA_FEATURE).unwrap(), home);
    }

    /// C2 — the pre-reorganisation layout: `evidence/<feature>/reports`.
    #[test]
    fn corpus_dir_found_at_container_depth_0() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("evidence");
        let home = mkdir(&root.join(SMA_FEATURE).join("reports"));
        assert_eq!(corpus_reports_dir(&root, SMA_FEATURE).unwrap(), home);
    }

    /// C3 — C1's two-sided partner for the bound: container depth 2 is not read.
    #[test]
    fn corpus_dir_not_found_at_container_depth_2() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("evidence");
        mkdir(&root.join("a").join("b").join(SMA_FEATURE).join("reports"));
        let err = corpus_reports_dir(&root, SMA_FEATURE).unwrap_err();
        assert!(
            matches!(&err, CliPathError::NoCorpusHome { feature, root: r, max: 1 }
                if feature == SMA_FEATURE && r == &root),
            "got {err:?}"
        );
    }

    /// C4 — a twin (C1's and C2's homes together) is refused, listing both.
    #[test]
    fn corpus_dir_with_two_homes_is_refused() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("evidence");
        let d0 = mkdir(&root.join(SMA_FEATURE).join("reports"));
        let d1 = mkdir(&root.join("v1").join(SMA_FEATURE).join("reports"));
        let err = corpus_reports_dir(&root, SMA_FEATURE).unwrap_err();
        match err {
            CliPathError::TwoCorpusHomes { feature, homes } => {
                assert_eq!(feature, SMA_FEATURE);
                assert_eq!(homes, vec![d0, d1]);
            }
            other => panic!("expected TwoCorpusHomes, got {other:?}"),
        }
    }

    /// C5 — a dir named `reports` is never a container.
    #[test]
    fn reports_is_never_a_container() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("evidence");
        mkdir(&root.join("reports").join(SMA_FEATURE).join("reports"));
        let err = corpus_reports_dir(&root, SMA_FEATURE).unwrap_err();
        assert!(
            matches!(err, CliPathError::NoCorpusHome { .. }),
            "got {err:?}"
        );
    }

    /// The B1/B2 fixture, at container depth 0 so B tests only the file match
    /// (C1-C5 test the depth). Returns (root, the baseline's reports dir).
    fn b_fixture(with_real_reports: bool) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("evidence");
        let dir = mkdir(&root.join(scenario_to_feature(REFRESH)).join("reports"));
        if with_real_reports {
            touch(&dir.join(format!("backtest-20260101-000000-{REFRESH}.md")));
            touch(&dir.join(format!("backtest-20260420-151944-{REFRESH}.md")));
        }
        // Decoys, each newer than the real reports: HEAD's `contains` match took
        // the first; none is a `backtest-*-<REFRESH>.md` file.
        touch(&dir.join(format!("backtest-20991231-235959-{REFRESH}-v2.md")));
        touch(&dir.join(format!("backtest-20991231-235959-{REFRESH}-equity.csv")));
        touch(&dir.join(format!("backtest-20991231-235959-x{REFRESH}.md")));
        (t, root, dir)
    }

    /// B1 — the baseline is the newest EXACT `backtest-*-<baseline>.md`.
    #[test]
    fn baseline_is_the_newest_exact_match() {
        let (_t, root, _dir) = b_fixture(true);
        assert_eq!(
            resolve_baseline(&root, MACD).unwrap(),
            Some(format!("backtest-20260420-151944-{REFRESH}.md"))
        );
    }

    /// B2 — B1's fixture minus the two real reports (decoys kept) is refused,
    /// naming the scenario, its baseline and the dir. B1/B2 differ only in
    /// whether the baseline exists.
    #[test]
    fn declared_baseline_missing_is_refused() {
        let (_t, root, dir) = b_fixture(false);
        let err = resolve_baseline(&root, MACD).unwrap_err();
        match &err {
            CliPathError::BaselineMissing {
                scenario,
                baseline,
                dir: d,
            } => {
                assert_eq!(scenario, MACD);
                assert_eq!(baseline, REFRESH);
                assert_eq!(d, &dir);
            }
            other => panic!("expected BaselineMissing, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains(MACD) && msg.contains(REFRESH), "{msg}");
    }

    /// B3 — a scenario that declares no baseline never reads the corpus.
    /// In-test control: the same absent root fails a declaring scenario, so the
    /// root would have failed had the function read it.
    #[test]
    fn undeclared_scenario_never_reads_the_corpus() {
        let t = tempfile::tempdir().unwrap();
        let absent = t.path().join("no-such-evidence");
        assert_eq!(
            resolve_baseline(&absent, "eth-2024-h1-sma-cross").unwrap(),
            None
        );
        let err = resolve_baseline(&absent, "btc-2024-h1-sma-cross").unwrap_err();
        match &err {
            CliPathError::BaselineUnresolved {
                scenario,
                baseline,
                source,
            } => {
                assert_eq!(scenario, "btc-2024-h1-sma-cross");
                assert_eq!(baseline, "btc-2023-1m-sma-cross");
                assert!(
                    matches!(**source, CliPathError::NoCorpusHome { .. }),
                    "{source:?}"
                );
            }
            other => panic!("expected BaselineUnresolved, got {other:?}"),
        }
    }
}
