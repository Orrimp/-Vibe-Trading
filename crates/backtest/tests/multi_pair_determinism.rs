//! T716 — Multi-pair determinism integration test (v1.5a R9 / V5).
//!
//! Runs `pairs-2023-zscore-mr` twice at seed `0xC0FFEE` and asserts:
//!   (a) report body-SHA256 byte-identical across runs.
//!   (b) `pnl_by_pair` results identical across runs.
//!
//! Extends the CI determinism job from 6 v0/v0.5/v1 scenarios to 7
//! (adds `pairs-2023-zscore-mr` for byte-identical-across-two-runs check).

#![allow(clippy::unwrap_used)]

/// Helper: run the `pairs-2023-zscore-mr` scenario once and return the report body.
fn run_pairs_scenario_once(scenario: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root");

    let bin_path = workspace_root.join("target/debug/backtest");
    if !bin_path.exists() {
        let status = std::process::Command::new("cargo")
            .args(["build", "--bin", "backtest"])
            .current_dir(workspace_root)
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "cargo build --bin backtest failed");
    }

    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    // Copy strategy TOML into temp config dir.
    let config_dir = tmp.path().join("config/strategies");
    std::fs::create_dir_all(&config_dir).expect("create config/strategies");
    let src_strategies = workspace_root.join("config/strategies");
    for entry in std::fs::read_dir(&src_strategies)
        .expect("read config/strategies")
        .flatten()
    {
        let dst = config_dir.join(entry.file_name());
        std::fs::copy(entry.path(), dst).expect("copy strategy TOML");
    }

    let output = std::process::Command::new(&bin_path)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary failed for {scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("'Report written:' line in output");

    let report_path = tmp.path().join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

// ── T716: pairs-2023-zscore-mr is body-SHA256 deterministic ──────────────────

#[test]
fn t716_pairs_2023_zscore_mr_deterministic() {
    let report1 = run_pairs_scenario_once("pairs-2023-zscore-mr");
    let report2 = run_pairs_scenario_once("pairs-2023-zscore-mr");

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1 = hash1.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hex2 = hash2.iter().map(|b| format!("{b:02x}")).collect::<String>();

    assert_eq!(
        hex1, hex2,
        "T716: pairs-2023-zscore-mr body-SHA256 must be identical across two runs at seed 0xC0FFEE\n\
         hash1: {hex1}\nhash2: {hex2}"
    );
}

// ── T716: pairs-2024-h1-zscore-mr is body-SHA256 deterministic ───────────────

#[test]
fn t716_pairs_2024_h1_zscore_mr_deterministic() {
    let report1 = run_pairs_scenario_once("pairs-2024-h1-zscore-mr");
    let report2 = run_pairs_scenario_once("pairs-2024-h1-zscore-mr");

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1 = hash1.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hex2 = hash2.iter().map(|b| format!("{b:02x}")).collect::<String>();

    assert_eq!(
        hex1, hex2,
        "T716: pairs-2024-h1-zscore-mr body-SHA256 must be identical across two runs at seed 0xC0FFEE\n\
         hash1: {hex1}\nhash2: {hex2}"
    );
}

// ── R-REPRO-10/11 — anchor REPRODUCTION for the two pairs scenarios ──────────
//
// The `t716_*` tests above run a scenario twice and compare it with ITSELF. That
// proves determinism and cannot see code-vs-evidence drift (bug-log #113); the
// anchor-gate coverage audit of 2026-09-26 lists these two as the cheapest
// coverage gap in the whole corpus — CI-runnable, no corpus needed, the runner
// already here, one `assert_eq!` missing.
//
// Condition is the same one `run_pairs_scenario_once` already pins and the same
// one the measurements were taken under: the **no-feature** debug binary, CWD a
// tempdir holding only `config/strategies/`. Both matter — bug-log #112 is two
// sets of false verdicts from getting exactly this wrong.
//
// Comparison target is the current canonical namespace,
// `v1.5a + v5-realdata-medium-2026-05`. The `+ noop-baseline` rows
// (`90591a0e…` / `14f50a59…`) are the frozen pre-friction oracle and are NOT what
// current code should produce.
//
// Both were measured red on 2026-09-26 (`ac647a59…`, `5bee5e9c…` — identical from two
// independent passes, so the feature set does not move them), and the 1-27 D6.b re-lock
// re-priced their `v1.5a + v5-realdata-medium-2026-05` rows to those bodies the same day.
// So they are LIVE gates now, not `#[ignore]`d. Attribution of the original drift to the
// #67 engine fix is mechanism rather than bisect — these are 2-symbol scenarios and #67
// was "buying one symbol at another symbol's price".
//
//     cargo test -p backtest --test multi_pair_determinism -- --ignored reproduces_anchor

/// R-REPRO-10 — `pairs-2023-zscore-mr` reproduces its canonical anchor.
#[test]
fn pairs_2023_zscore_mr_reproduces_anchor() {
    const ANCHOR: &str = "ac647a593a962c5d6612ec052cf2b7d89ab40a9967e493b0554f0ebee0e30f10";
    assert_pairs_reproduces("pairs-2023-zscore-mr", ANCHOR);
}

/// R-REPRO-11 — `pairs-2024-h1-zscore-mr` reproduces its canonical anchor.
#[test]
fn pairs_2024_h1_zscore_mr_reproduces_anchor() {
    const ANCHOR: &str = "5bee5e9cf6a80a838bf9bf667a6733712563eede0b21ff5b9dfd19b933d75e6b";
    assert_pairs_reproduces("pairs-2024-h1-zscore-mr", ANCHOR);
}

/// Shared body: re-run and compare the body-SHA against an `anchors.toml` row.
///
/// # Panics
///
/// Panics on drift — that is the gate.
fn assert_pairs_reproduces(scenario: &str, anchor: &str) {
    let report = run_pairs_scenario_once(scenario);
    let hex: String = backtest::report_body_hash(&report)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    // Assert the CONDITION before the SHA (bug-log #112 requirement 2): if the
    // data path silently changes, this fails as a condition mismatch rather than
    // as a fake drift, which is the trap that produced two sets of false verdicts.
    assert!(
        report.contains("synthetic (seeded RNG, v1.5a multi-symbol)"),
        "R-REPRO condition mismatch for {scenario}: expected the synthetic \
         v1.5a multi-symbol data path, and the body does not say so. The anchor was \
         locked under that path; comparing a SHA across data paths is bug-log #112."
    );

    assert_eq!(
        hex, anchor,
        "R-REPRO: {scenario} no longer reproduces its canonical \
         `v1.5a + v5-realdata-medium-2026-05` anchor.\n\
         Expected: {anchor}\nGot:      {hex}\n\
         Code-vs-evidence drift — invisible to verify_anchors.sh (bug-log #93). Do NOT \
         re-pin to the produced value (bug-log #77); the resolution is the D6.b re-lock \
         of story 1-27."
    );
}
