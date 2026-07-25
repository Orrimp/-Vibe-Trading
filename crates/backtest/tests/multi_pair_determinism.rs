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
