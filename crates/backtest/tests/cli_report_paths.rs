//! bug-log #210 (ADR-0104) — the `backtest` bin's report paths, pinned against
//! the REAL committed tree and the REAL binary.
//!
//! - R1/R2/R3 read the workspace's committed corpus and `.gitignore` through
//!   read-only `git` (`-C <workspace>`); they write nothing.
//! - W1/W2 run `CARGO_BIN_EXE_backtest` with the working directory set to a
//!   tempdir sandbox, so every path the bin resolves (`evidence/`,
//!   `backtest-runs/`) is inside the sandbox. Nothing is written to the repo.
//!
//! T-T4 probes (one edit each; red sets MEASURED 2026-11-24, see bug-log #210):
//!
//! | Probe | Edit | Red |
//! |---|---|---|
//! | P1 | delete the container-depth loop in `cli_paths::corpus_reports_dir` | C1, C4, R1, W1, determinism t521/t622/t717 macd/rsi/bbands, R-REAL `realdata_row_btc_2023_1m_{macd_trend,rsi_reversion,bbands_mean_revert}`, ui LY-210 |
//! | P2 | `CORPUS_MAX_CONTAINER_DEPTH = 2` | C3, ui LY-210 |
//! | P3 | `resolve_baseline`'s `None => Err(BaselineMissing{..})` → `None => Ok(None)` | B2, W2 |
//! | P4 | `CLI_OUTPUT_ROOT = "evidence"` | R3, W1, and every determinism / multi-pair helper's default-path assertion |
//! | P5 | delete the `backtest-runs/` rule from `.gitignore` | R3 |
//! | P6 | `latest_report_named` back to HEAD's `contains(scenario)` match | B1, B2 |
//! | P7 | drop the bbands row from `BASELINE_DECLARATIONS` | R1, R2 |
//! | P8 | `TwoCorpusHomes` → take the first home | C4 |
//! | P9 | move the `resolve_baseline` line below the final `create_dir_all` in main | W2 |
//! | P10 | delete the `scenario.baseline_report = …?;` line in main | W1, W2 |
//! | P14 | point main's TcnOverlay writer at `evidence/<feature>/reports` | determinism `tt1_top10_{2023,2024}_fy_tcn_overlay_*` |
//! | P15 | `child_dirs`: drop the `!= REPORTS_DIR` filter | C5, ui LY-210 |
//! | P16 | delete the `!/evidence/**/backtest-runs/` carve-out from `.gitignore` | R3 |

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use backtest::cli_paths::{
    self, BASELINE_DECLARATIONS, EVIDENCE_ROOT, report_dir_for_scenario, resolve_baseline,
};

// bug-log #227: the snapshot walker and the workspace root live in one shared copy.
#[path = "support/sandbox.rs"]
mod sandbox;
use sandbox::{snapshot, workspace_root};

/// Rows in `BASELINE_DECLARATIONS` at the fix (2026-11-24): btc-2024-h1-sma-cross,
/// macd, rsi, bbands. Declared, not measured off the table.
const DECLARED_BASELINE_ROWS: usize = 4;

/// Committed `*/reports/backtest-*.md` with a `scenario:` front-matter line at
/// the fix (197 git-listed, 2 without one: the `advisor-corpus-expansion`
/// p2-verdict-rerun pair). A floor, so the skip in R2 cannot hide a parse failure.
const COMMITTED_SCENARIO_REPORTS_FLOOR: usize = 195;

/// Committed reports whose `baseline_report:` is not `n/a` at the fix.
const COMMITTED_NON_NA_BASELINE_LINES_FLOOR: usize = 11;

/// Distinct scenarios in `evidence/anchors.toml` at the fix.
const ANCHORED_SCENARIOS_FLOOR: usize = 77;

fn git(ws: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(ws)
        .args(args)
        .output()
        .expect("spawn git")
}

/// The leading `---` … `---` block of a report, if it has one.
fn front_matter(text: &str) -> Option<&str> {
    let text = text.strip_prefix("---\n")?;
    let end = text.find("\n---\n")?;
    Some(&text[..end])
}

fn fm_value<'a>(fm: &'a str, key: &str) -> Option<&'a str> {
    fm.lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(':'))
        .map(str::trim)
}

/// `backtest-<8 digits>-<6 digits>-<scenario>.md` → `<scenario>`.
fn scenario_of_report_name(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("backtest-")?.strip_suffix(".md")?;
    let (date, rest) = rest.split_at_checked(8)?;
    let rest = rest.strip_prefix('-')?;
    let (time, rest) = rest.split_at_checked(6)?;
    let scenario = rest.strip_prefix('-')?;
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    (digits(date) && digits(time) && !scenario.is_empty()).then_some(scenario)
}

/// R1 — every declared baseline resolves, in the committed tree, to a report
/// git tracks.
#[test]
fn every_declared_baseline_resolves_to_a_committed_report() {
    let ws = workspace_root();
    let root = ws.join(EVIDENCE_ROOT);
    assert!(
        BASELINE_DECLARATIONS.len() >= DECLARED_BASELINE_ROWS,
        "BASELINE_DECLARATIONS has {} rows; {DECLARED_BASELINE_ROWS} declared",
        BASELINE_DECLARATIONS.len()
    );
    for (scenario, baseline) in BASELINE_DECLARATIONS {
        let name = resolve_baseline(&root, scenario)
            .unwrap_or_else(|e| panic!("{scenario}: {e} ({e:?})"))
            .unwrap_or_else(|| panic!("{scenario} declares {baseline} and resolved to None"));
        assert_eq!(
            scenario_of_report_name(&name),
            Some(*baseline),
            "{scenario}: resolved {name}, which is not a report of {baseline}"
        );
        let dir = cli_paths::corpus_reports_dir(&root, cli_paths::scenario_to_feature(baseline))
            .expect("the resolver found this dir");
        let path = dir.join(&name);
        let out = git(
            &ws,
            &[
                "ls-files",
                "--error-unmatch",
                path.to_str().expect("utf-8 path"),
            ],
        );
        assert!(
            out.status.success(),
            "{scenario}: {} is not tracked by git: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// R2 — an independent oracle for `BASELINE_DECLARATIONS`: the pairs
/// (`scenario:`, the scenario named by a non-`n/a` `baseline_report:`) carried
/// by every committed `*/reports/backtest-*.md` equal the table.
#[test]
fn baseline_declarations_match_the_committed_corpus() {
    let ws = workspace_root();
    let out = git(&ws, &["ls-files", "-z", "--", EVIDENCE_ROOT]);
    assert!(out.status.success(), "git ls-files failed");
    let listed = String::from_utf8(out.stdout).expect("utf-8 listing");
    let mut parsed = 0usize;
    let mut non_na = 0usize;
    let mut committed: BTreeSet<(String, String)> = BTreeSet::new();
    for rel in listed.split('\0').filter(|p| !p.is_empty()) {
        let path = Path::new(rel);
        let in_reports = path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|d| d == "reports");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !(in_reports
            && name.starts_with("backtest-")
            && Path::new(name).extension().is_some_and(|e| e == "md"))
        {
            continue;
        }
        let text = std::fs::read_to_string(ws.join(rel)).expect("read committed report");
        // Out of scope: a report with no `scenario:` front-matter line is not a
        // CLI report (2 at the fix); the floor below keeps this skip honest.
        let Some(scenario) = front_matter(&text).and_then(|fm| fm_value(fm, "scenario")) else {
            continue;
        };
        parsed += 1;
        let fm = front_matter(&text).expect("checked above");
        let Some(baseline) = fm_value(fm, "baseline_report") else {
            continue;
        };
        if baseline == "n/a" {
            continue;
        }
        non_na += 1;
        let base_scenario = scenario_of_report_name(baseline)
            .unwrap_or_else(|| panic!("{rel}: unparseable baseline_report `{baseline}`"));
        committed.insert((scenario.to_string(), base_scenario.to_string()));
    }
    assert!(
        parsed >= COMMITTED_SCENARIO_REPORTS_FLOOR,
        "parsed {parsed} committed reports, floor {COMMITTED_SCENARIO_REPORTS_FLOOR}"
    );
    assert!(
        non_na >= COMMITTED_NON_NA_BASELINE_LINES_FLOOR,
        "{non_na} non-n/a baseline_report lines, floor {COMMITTED_NON_NA_BASELINE_LINES_FLOOR}"
    );
    let declared: BTreeSet<(String, String)> = BASELINE_DECLARATIONS
        .iter()
        .map(|(s, b)| ((*s).to_string(), (*b).to_string()))
        .collect();
    assert_eq!(
        declared, committed,
        "BASELINE_DECLARATIONS (left) differs from the committed corpus (right)"
    );
}

/// R3 — the default output dir of every anchored scenario is git-ignored and
/// outside `evidence/`. Control: the same probe on a committed-corpus path says
/// "not ignored", so the probe can fail (#187).
#[test]
fn default_output_dir_is_git_ignored_for_every_anchored_scenario() {
    let ws = workspace_root();
    let anchors =
        std::fs::read_to_string(ws.join(EVIDENCE_ROOT).join("anchors.toml")).expect("anchors");
    let scenarios: BTreeSet<&str> = anchors
        .lines()
        .filter_map(|l| {
            let v = l.trim_start().strip_prefix("scenario")?.trim_start();
            let v = v.strip_prefix('=')?.trim();
            v.strip_prefix('"')?.split('"').next()
        })
        .collect();
    assert!(
        scenarios.len() >= ANCHORED_SCENARIOS_FLOOR,
        "{} anchored scenarios, floor {ANCHORED_SCENARIOS_FLOOR}",
        scenarios.len()
    );

    let ignored = |rel: &Path| -> bool {
        let rel = rel.to_str().expect("utf-8 path").replace('\\', "/");
        let out = git(&ws, &["check-ignore", "-q", "--no-index", &rel]);
        match out.status.code() {
            Some(0) => true,
            Some(1) => false,
            other => panic!(
                "git check-ignore {rel}: exit {other:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
        }
    };

    let control = Path::new("evidence/v1/v0-paper-sma/reports")
        .join("backtest-00000000-000000-btc-2023-1m-sma-cross.md");
    assert!(
        !ignored(&control),
        "control: a committed-corpus path must read as NOT ignored"
    );
    // The root is cwd-relative, so the rule is unanchored: a run from a crate dir is
    // ignored too. Under evidence/ it is carved out, so a run started inside the corpus
    // (where `verify_anchors.sh` looks) stays visible in git status.
    let from_a_crate_dir = Path::new("crates/backtest/backtest-runs/v0-paper-sma/reports")
        .join("backtest-00000000-000000-btc-2023-1m-sma-cross.md");
    assert!(
        ignored(&from_a_crate_dir),
        "a default run from crates/backtest/ must be git-ignored"
    );
    let from_inside_evidence = Path::new("evidence/v3/backtest-runs/v0-paper-sma/reports")
        .join("backtest-00000000-000000-btc-2023-1m-sma-cross.md");
    assert!(
        !ignored(&from_inside_evidence),
        "a default run from inside evidence/ must stay visible (not ignored)"
    );

    for s in &scenarios {
        let dir = report_dir_for_scenario(s);
        assert!(
            !dir.starts_with(EVIDENCE_ROOT),
            "{s}: default output dir {} is under evidence/",
            dir.display()
        );
        let report = dir.join(format!("backtest-00000000-000000-{s}.md"));
        assert!(
            ignored(&report),
            "{s}: {} is not git-ignored",
            report.display()
        );
    }
}

const H1_SCENARIO: &str = "btc-2024-h1-sma-cross";
const H1_BASELINE: &str = "btc-2023-1m-sma-cross";
const H1_BASELINE_FILE: &str = "backtest-20260420-202621-btc-2023-1m-sma-cross.md";

/// A sandbox with `evidence/v1/v0-paper-sma/reports/` and, optionally, the
/// committed baseline's file name as an empty stub.
fn sandbox(with_stub: bool) -> tempfile::TempDir {
    let t = tempfile::tempdir().expect("tempdir");
    let reports = t.path().join("evidence/v1/v0-paper-sma/reports");
    std::fs::create_dir_all(&reports).expect("mkdir sandbox reports");
    if with_stub {
        std::fs::write(reports.join(H1_BASELINE_FILE), b"").expect("write stub");
    }
    t
}

fn run_bin(cwd: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_backtest"))
        .args(["--scenario", H1_SCENARIO, "--seed", "0xC0FFEE"])
        .current_dir(cwd)
        .output()
        .expect("spawn backtest")
}

/// W1 — with no `--reports-dir`, the bin writes under `backtest-runs/`, names
/// the committed baseline, and adds nothing under `evidence/`.
#[test]
fn cli_names_the_committed_baseline_and_writes_outside_evidence() {
    let t = sandbox(true);
    let evidence_before = snapshot(&t.path().join("evidence"));
    let out = run_bin(t.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "exit {:?}\nstdout: {stdout}\nstderr: {stderr}",
        out.status
    );
    let rel = stdout
        .lines()
        .find_map(|l| l.strip_prefix("Report written: "))
        .map(str::trim)
        .unwrap_or_else(|| panic!("no `Report written:` line\nstdout: {stdout}"));
    let rel = Path::new(rel);
    // Independent oracle: the literal layout, not `report_dir_for_scenario`.
    let expected_dir = Path::new("backtest-runs")
        .join("v0-paper-sma")
        .join("reports");
    assert!(
        !rel.is_absolute() && rel.starts_with(&expected_dir),
        "report {} is not under {}",
        rel.display(),
        expected_dir.display()
    );
    let text = std::fs::read_to_string(t.path().join(rel)).expect("read report");
    let fm = front_matter(&text).expect("report has front matter");
    assert_eq!(
        fm_value(fm, "baseline_report"),
        Some(H1_BASELINE_FILE),
        "front matter:\n{fm}"
    );
    // Positive control for the walker (#187): it sees the report this run wrote, so
    // an unchanged evidence/ snapshot below is a measurement, not a blind spot.
    assert!(
        snapshot(t.path()).contains_key(rel),
        "snapshot() must see the report the run wrote ({})",
        rel.display()
    );
    assert_eq!(
        snapshot(&t.path().join("evidence")),
        evidence_before,
        "the run changed the sandbox's evidence/ tree (a twin)"
    );
}

/// W2 — a declared baseline with no committed report refuses the run, names
/// the scenario and its baseline, and writes NOTHING (#187).
#[test]
fn cli_refuses_a_missing_declared_baseline_and_writes_nothing() {
    let t = sandbox(false);
    // Control (#187): the snapshot sees a write, so an equal snapshot below means something.
    let ctl = tempfile::tempdir().expect("tempdir");
    let ctl_before = snapshot(ctl.path());
    std::fs::write(ctl.path().join("x"), b"x").expect("write control file");
    assert_ne!(
        snapshot(ctl.path()),
        ctl_before,
        "control: snapshot() missed a write"
    );
    let before = snapshot(t.path());
    let out = run_bin(t.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "a missing baseline must refuse the run\nstdout: {stdout}\nstderr: {stderr}"
    );
    // The refusal's own sentence (both `BaselineMissing` and `BaselineUnresolved` render
    // it): the scenario name alone is in every run's `backtest starting` log line.
    let refusal = format!("scenario `{H1_SCENARIO}` declares the baseline `{H1_BASELINE}`");
    assert!(
        stderr.contains(&refusal),
        "stderr must carry the refusal naming {H1_SCENARIO} and {H1_BASELINE}:\n{stderr}"
    );
    assert!(
        !stdout.contains("Report written:"),
        "a refused run printed a report path:\n{stdout}"
    );
    assert!(
        !t.path().join("backtest-runs").exists(),
        "a refused run created the output root"
    );
    assert_eq!(
        snapshot(t.path()),
        before,
        "a refused run changed the sandbox (names, sizes or mtimes)"
    );
}
