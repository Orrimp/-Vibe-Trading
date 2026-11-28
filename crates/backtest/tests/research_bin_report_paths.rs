//! bug-log #227 (ADR-0104 D7) — where the crate's three research bins write by
//! default: `monte_carlo` and `param_robustness_sweep` here; `run_yahoo_sma` needs
//! `--features yahoo`, so its bin-level pins live in `run_yahoo_sma_ticker_flag.rs`
//! (WY1-WY3).
//!
//! - G1/G2 read the workspace's `.gitignore` and committed corpus through read-only
//!   `git -C <workspace>`; they write nothing.
//! - SRC1/SRC2 read the bins' source through `include_str!`, so a production call
//!   that stops passing the parsed flag is red without running the bin (#190 wiring).
//! - WM1/WS1 run the REAL bins (`CARGO_BIN_EXE_*`) with the working directory set to a
//!   tempdir sandbox holding a copy of `config/strategies/` and one empty `evidence/`,
//!   so every path the bins resolve is inside the sandbox. Nothing is written to the
//!   repository.
//!
//! T-T4 probes (one edit each; red sets MEASURED 2026-11-28, see bug-log #227). "WY*"
//! are in `run_yahoo_sma_ticker_flag.rs`, "D1/D2" in `cli_paths.rs`, "PS1/PS2" in the
//! sweep bin, "#210 R3/W1" in `cli_report_paths.rs`:
//!
//! | Probe | Edit | Red |
//! |---|---|---|
//! | P227-1 | `CLI_OUTPUT_ROOT = "evidence/v1"` | D1, PS1, G1, WM1, WS1, WY1, WY3, #210 R3, #210 W1 |
//! | P227-2 | `effective_reports_dir` ignores `explicit` | D2, G1, PS2, WY2, `reproducibility_sample_figure`; on a box with `data/yahoo/`, the two anchor 69/70 binary pins |
//! | P227-3 | `sweep_family_feature`: `Carry` → `"time-series-momentum-robustness"` | PS1, G1, G2 |
//! | P227-4 | the sweep's resolver filters out HEAD's momentum literal (value equality) | PS2 |
//! | P227-5 | delete `backtest-runs/` from `.gitignore` (no recompile) | G1, WM1, WS1, WY1, WY3, #210 R3 |
//! | P227-6 | `monte_carlo`'s `out_dir` gets HEAD's `default_value` back | WM1 |
//! | P227-7 | the sweep's `out_dir` gets HEAD's `default_value` back | PS1, WS1 |
//! | P227-8 | `run_yahoo_sma`'s `reports_dir` gets HEAD's `default_value` back | WY1, WY3 |
//! | P227-9 | `MONTE_CARLO_FEATURE = "strategy-robustness"` | D1, G2, WM1 |
//! | P227-10 | `sandbox::snapshot` returns only the root entry | WM1, WS1, WY1, WY2, WY3, #210 W1 |
//! | P227-11 | `YAHOO_SMA_FEATURE = "lab-yahoo"` | D1, WY1, WY3 |
//! | P227-12 | `run_yahoo_sma` main: no workspace walk around the reports dir | WY3 |
//! | P227-13 | the sweep main's call passes `SweepDirection::Momentum` | SRC1 |
//! | P227-14 | the `monte_carlo` main's call passes `None` | SRC2 |
//! | P227-15 | `monte_carlo` main creates `evidence/v1/…/reports` after its write | WM1 (its evidence assertion) |
//! | P227-16 | `effective_reports_dir` treats an explicit value under `backtest-runs/` as absent | D2 |

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use backtest::cli_paths::{
    EVIDENCE_ROOT, MONTE_CARLO_FEATURE, YAHOO_SMA_FEATURE, effective_reports_dir,
};
use backtest::mc_harness::{self, GeneratorKind};
use backtest::resample::Horizon;
use backtest::sweep_harness::{
    GridKind, LEGACY_TAKER_FEE_BPS, SweepDirection, SweepScoreSource, SweepSelectionMode,
    build_scenario_name, sweep_family_feature,
};
use clap::ValueEnum;

#[path = "support/sandbox.rs"]
mod sandbox;
use sandbox::{
    files_under, ignored, printed_path, research_sandbox, run_serialised, snapshot, workspace_root,
};

/// Rows of `scripts/relock/surfaces.tsv` at the fix (2026-11-28): the 34 anchored
/// θ-surfaces. Declared, not measured off the manifest.
const MANIFEST_ROWS_FLOOR: usize = 34;

/// Distinct sweep families, i.e. distinct `evidence/v1/<slug>/` dirs a θ-surface can
/// live in: momentum, MR, carry, TS, horizon, basis, MN. Declared.
const SWEEP_FAMILY_SLUGS: usize = 7;

/// Every axis tuple the sweep's CLI can parse, from clap's own variant lists.
fn every_sweep_axis_tuple() -> Vec<(
    Horizon,
    SweepSelectionMode,
    SweepScoreSource,
    SweepDirection,
)> {
    let mut out = Vec::new();
    for h in Horizon::value_variants() {
        for m in SweepSelectionMode::value_variants() {
            for s in SweepScoreSource::value_variants() {
                for d in SweepDirection::value_variants() {
                    out.push((*h, *m, *s, *d));
                }
            }
        }
    }
    out
}

/// G1 — every research bin's default dir is outside `evidence/` and git-ignored.
/// In-test control (#187): the same probe says "not ignored" for the bin's
/// evidence home, and an explicit path into it still resolves verbatim.
#[test]
fn research_bin_defaults_are_git_ignored_and_outside_evidence() {
    let ws = workspace_root();
    let mut cases: Vec<(&str, String)> = vec![
        (
            YAHOO_SMA_FEATURE,
            "backtest-00000000-000000-btc-yahoo-2024-1d-sma-cross.md".to_string(),
        ),
        (
            MONTE_CARLO_FEATURE,
            format!(
                "robustness-00000000-000000-{}.md",
                mc_harness::scenario_name(GeneratorKind::BlockBootstrapReal, 2023)
            ),
        ),
    ];
    let sweep_slugs: BTreeSet<&str> = every_sweep_axis_tuple()
        .into_iter()
        .map(|(h, m, s, d)| sweep_family_feature(h, m, s, d))
        .collect();
    assert_eq!(
        sweep_slugs.len(),
        SWEEP_FAMILY_SLUGS,
        "distinct sweep family slugs: {sweep_slugs:?}"
    );
    for slug in &sweep_slugs {
        cases.push((slug, "robustness-sweep-00000000-000000-x.md".to_string()));
    }
    for (slug, file) in &cases {
        let d = effective_reports_dir(None, slug);
        assert!(
            !d.starts_with(EVIDENCE_ROOT),
            "{slug}: the default dir {} is under evidence/",
            d.display()
        );
        assert!(
            ignored(&ws, &d.join(file)),
            "{slug}: {} is not git-ignored",
            d.join(file).display()
        );
        let e = Path::new("evidence/v1").join(slug).join("reports");
        assert_eq!(
            effective_reports_dir(Some(&e), slug),
            e,
            "{slug}: an explicit path into evidence/ must resolve verbatim"
        );
        assert!(
            !ignored(&ws, &e.join(file)),
            "control: {} must read as NOT ignored",
            e.join(file).display()
        );
    }
}

/// G2 — an independent oracle for the default slugs: the committed corpus says
/// where each family's anchored bodies live.
///
/// `evidence/v1` is the container the gate's namespace branches hard-code
/// (`verify_anchors.sh`'s `mc-robustness-2026-06`, `horizon-retest-robustness`,
/// `perp-basis-signal-robustness` and `perp-basis-mn-spread` branches).
/// `evidence/v2/harness-relock/` (the story 1-26 regeneration copies) is
/// deliberately not read: the gate never reads it. `run_yahoo_sma` has no committed
/// body in `lab-yahoo-realdata/`, so its slug is pinned by D1 and WY1, not here.
#[test]
fn research_bin_default_slugs_name_the_committed_corpus_dirs() {
    let ws = workspace_root();
    let out = Command::new("git")
        .arg("-C")
        .arg(&ws)
        .args(["ls-files", "-z", "--", "evidence/v1"])
        .output()
        .expect("spawn git");
    assert!(out.status.success(), "git ls-files failed");
    let listed = String::from_utf8(out.stdout).expect("utf-8 listing");
    let tracked: Vec<&str> = listed.split('\0').filter(|p| !p.is_empty()).collect();
    // `<slug>` of every tracked `evidence/v1/<slug>/reports/<prefix>*-<scenario>.md`.
    let slugs_of = |prefix: &str, scenario: &str| -> BTreeSet<String> {
        let suffix = format!("-{scenario}.md");
        tracked
            .iter()
            .filter_map(|p| {
                let (slug, rest) = p.strip_prefix("evidence/v1/")?.split_once('/')?;
                let name = rest.strip_prefix("reports/")?;
                (!name.contains('/')
                    && name.len() > prefix.len() + suffix.len()
                    && name.starts_with(prefix)
                    && name.ends_with(&suffix))
                .then(|| slug.to_string())
            })
            .collect()
    };

    let mc = mc_harness::scenario_name(GeneratorKind::BlockBootstrapReal, 2023);
    assert_eq!(
        slugs_of("robustness-", &mc),
        BTreeSet::from([MONTE_CARLO_FEATURE.to_string()]),
        "the committed {mc} bodies must live in exactly the dir MONTE_CARLO_FEATURE names"
    );

    let manifest_path = ws.join("scripts/relock/surfaces.tsv");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    let mut rows = 0usize;
    let mut seen: BTreeSet<&'static str> = BTreeSet::new();
    for line in manifest.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            cols.len(),
            3,
            "manifest row is not scenario\\tbinary\\targs: {line}"
        );
        let (scenario, binary, args) = (cols[0], cols[1], cols[2]);
        assert_eq!(binary, "param_robustness_sweep", "row {scenario}");
        let words: Vec<&str> = args.split_whitespace().collect();
        let flag = |name: &str| -> &str {
            words
                .iter()
                .position(|w| *w == name)
                .and_then(|i| words.get(i + 1))
                .copied()
                .unwrap_or_else(|| panic!("row {scenario} has no {name} value: {args}"))
        };
        let h = Horizon::from_str(flag("--horizon"), false)
            .unwrap_or_else(|e| panic!("row {scenario}: --horizon: {e}"));
        let m = SweepSelectionMode::from_str(flag("--selection-mode"), false)
            .unwrap_or_else(|e| panic!("row {scenario}: --selection-mode: {e}"));
        let s = SweepScoreSource::from_str(flag("--score-source"), false)
            .unwrap_or_else(|e| panic!("row {scenario}: --score-source: {e}"));
        let d = SweepDirection::from_str(flag("--direction"), false)
            .unwrap_or_else(|e| panic!("row {scenario}: --direction: {e}"));
        let slug = sweep_family_feature(h, m, s, d);
        assert_eq!(
            slugs_of("robustness-sweep-", scenario),
            BTreeSet::from([slug.to_string()]),
            "row {scenario}: its committed body must live in exactly evidence/v1/{slug}/reports"
        );
        rows += 1;
        seen.insert(slug);
    }
    assert!(
        rows >= MANIFEST_ROWS_FLOOR,
        "{rows} manifest rows read, floor {MANIFEST_ROWS_FLOOR}"
    );
    assert_eq!(
        seen.len(),
        SWEEP_FAMILY_SLUGS,
        "distinct family slugs over the manifest: {seen:?}"
    );
}

/// The production part of a bin's source: everything before its test module.
fn production_part<'a>(src: &'a str, file: &str) -> &'a str {
    let marker = "\n#[cfg(test)]\nmod tests {";
    let at = src
        .find(marker)
        .unwrap_or_else(|| panic!("{file}: no test-module marker {marker:?}"));
    &src[..at]
}

/// The whitespace-free argument text of the ONE call that starts with `call`, up to
/// the next `);`, with a trailing `,` trimmed.
fn squashed_args_of_the_one_call(prod: &str, call: &str, file: &str) -> String {
    assert_eq!(
        prod.matches(call).count(),
        1,
        "{file}: `{call}` must occur exactly once in production code"
    );
    let start = prod.find(call).expect("counted above") + call.len();
    let end = prod[start..]
        .find(");")
        .unwrap_or_else(|| panic!("{file}: `{call}` is not closed by `);`"));
    let squashed: String = prod[start..start + end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    squashed.trim_end_matches(',').to_string()
}

fn assert_once(prod: &str, needle: &str, file: &str) {
    assert_eq!(
        prod.matches(needle).count(),
        1,
        "{file}: `{needle}` must occur exactly once in production code"
    );
}

/// SRC1 (#190 wiring) — the sweep's `main` resolves its write dir from the PARSED
/// axes and flag, and writes there. WS1 exercises `main` for the momentum family
/// only; this is what reaches it for the other nine (the bin's own source-level
/// precedent cannot fail: its needles match themselves, bug-log #238).
#[test]
fn sweep_main_routes_its_write_through_the_parsed_axes() {
    let file = "param_robustness_sweep.rs";
    let src = include_str!("../src/bin/param_robustness_sweep.rs");
    let prod = production_part(src, file);
    assert_eq!(
        squashed_args_of_the_one_call(
            prod,
            "let effective_out_dir = resolve_effective_out_dir(",
            file
        ),
        "args.out_dir.as_deref(),args.horizon,args.selection_mode,args.score_source,args.direction"
    );
    assert_once(
        prod,
        "let report_path = effective_out_dir.join(&report_filename);",
        file,
    );
    assert_once(prod, "std::fs::create_dir_all(&effective_out_dir)", file);
}

/// SRC2 (#190 wiring) — `monte_carlo`'s `main` resolves its write dir from the
/// PARSED flag, and writes there.
#[test]
fn monte_carlo_main_routes_its_write_through_the_parsed_out_dir() {
    let file = "monte_carlo.rs";
    let prod = include_str!("../src/bin/monte_carlo.rs");
    assert!(
        !prod.contains("#[cfg(test)]"),
        "{file} gained a test module: take only its production part"
    );
    assert_eq!(
        squashed_args_of_the_one_call(
            prod,
            "let out_dir = backtest::cli_paths::effective_reports_dir(",
            file
        ),
        "args.out_dir.as_deref(),backtest::cli_paths::MONTE_CARLO_FEATURE"
    );
    assert_once(prod, "std::fs::create_dir_all(&out_dir)", file);
    assert_once(
        prod,
        "let report_path = out_dir.join(&report_filename);",
        file,
    );
}

/// What a default research-bin run in `tmp` must have done: printed one relative
/// report path under `expected_dir`, named `<prefix>*<suffix>`, that git ignores;
/// written exactly that one file; and changed nothing under `evidence/`.
struct DefaultRun<'a> {
    tmp: &'a Path,
    before_all: &'a BTreeSet<PathBuf>,
    before_ev: &'a std::collections::BTreeMap<PathBuf, sandbox::Meta>,
    expected_dir: PathBuf,
    prefix: &'a str,
    suffix: String,
}

fn assert_default_run(run: &DefaultRun<'_>, out: &std::process::Output) {
    let ws = workspace_root();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "exit {:?}\nstdout: {stdout}\nstderr: {stderr}",
        out.status
    );
    let rel = printed_path(&stdout, "report:");
    assert!(
        !rel.is_absolute() && rel.starts_with(&run.expected_dir),
        "report {} is not under {}",
        rel.display(),
        run.expected_dir.display()
    );
    let name = rel
        .file_name()
        .and_then(|n| n.to_str())
        .expect("utf-8 file name");
    assert!(
        name.starts_with(run.prefix) && name.ends_with(&run.suffix),
        "report file name {name} is not {}*{}",
        run.prefix,
        run.suffix
    );
    assert!(
        ignored(&ws, &rel),
        "the path the bin wrote, {}, is not git-ignored",
        rel.display()
    );
    // Positive control (#187): the walker sees the report, and the run wrote one file.
    let new: BTreeSet<PathBuf> = files_under(run.tmp)
        .difference(run.before_all)
        .cloned()
        .collect();
    assert_eq!(
        new,
        BTreeSet::from([rel.clone()]),
        "the run must have written exactly its report"
    );
    assert_eq!(
        &snapshot(&run.tmp.join("evidence")),
        run.before_ev,
        "a default run changed the sandbox's evidence/ tree (names, sizes or mtimes)"
    );
}

/// WM1 — a default `monte_carlo` run (no `--out-dir`) writes under
/// `backtest-runs/strategy-robustness-harness/reports/` and NOTHING under
/// `evidence/`, not even a dir mtime. The one end-to-end research-bin witness that
/// always stays (bug-log #227's cost rule).
#[test]
fn monte_carlo_default_run_writes_under_backtest_runs_not_evidence() {
    let ws = workspace_root();
    let tmp = research_sandbox(&ws);
    let before_ev = snapshot(&tmp.path().join("evidence"));
    let before_all = files_under(tmp.path());
    let out = run_serialised(
        Command::new(env!("CARGO_BIN_EXE_monte_carlo"))
            .args(["--generator", "gbm-smoke", "--paths", "2"])
            .current_dir(tmp.path()),
    );
    assert_default_run(
        &DefaultRun {
            tmp: tmp.path(),
            before_all: &before_all,
            before_ev: &before_ev,
            expected_dir: Path::new("backtest-runs")
                .join("strategy-robustness-harness")
                .join("reports"),
            prefix: "robustness-",
            suffix: format!(
                "-{}.md",
                mc_harness::scenario_name(GeneratorKind::GbmSmoke, 2023)
            ),
        },
        &out,
    );
}

/// WS1 — a default `param_robustness_sweep` run writes under
/// `backtest-runs/momentum-parameter-robustness-sweep/reports/` and nothing under
/// `evidence/`. Momentum only: the axis guard admits a small N on the two-cell grid
/// alone; SRC1 + PS1/PS2 cover the other nine families.
#[test]
fn param_sweep_default_run_writes_under_backtest_runs_not_evidence() {
    let ws = workspace_root();
    let tmp = research_sandbox(&ws);
    let before_ev = snapshot(&tmp.path().join("evidence"));
    let before_all = files_under(tmp.path());
    let out = run_serialised(
        Command::new(env!("CARGO_BIN_EXE_param_robustness_sweep"))
            .args([
                "--grid",
                "two-cell",
                "--generator",
                "gbm-smoke",
                "--paths",
                "2",
            ])
            .current_dir(tmp.path()),
    );
    let scenario = build_scenario_name(
        GridKind::TwoCell,
        SweepDirection::Momentum,
        SweepScoreSource::VolAdjustedReturn,
        SweepSelectionMode::CrossSectionalTopK,
        Horizon::OneHour,
        2023,
        "gbm-smoke",
        LEGACY_TAKER_FEE_BPS,
    );
    assert_default_run(
        &DefaultRun {
            tmp: tmp.path(),
            before_all: &before_all,
            before_ev: &before_ev,
            expected_dir: Path::new("backtest-runs")
                .join("momentum-parameter-robustness-sweep")
                .join("reports"),
            prefix: "robustness-sweep-",
            suffix: format!("-{scenario}.md"),
        },
        &out,
    );
}
