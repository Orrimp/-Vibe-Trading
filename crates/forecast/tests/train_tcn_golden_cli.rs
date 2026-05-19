//! K5 mitigation: golden-CLI snapshot test for `train_tcn --help`.
//!
//! ADR-0034 § D9 — ensures CLI flags don't drift silently. The test runs
//! `cargo run -p forecast --bin train_tcn --features candle -- --help` and
//! verifies:
//!
//! 1. Required flags are present: `--config`, `--output-dir`, `--dry-run`,
//!    `--epochs`, `--symbols`, `--scenario`.
//! 2. No forbidden write-bypassing flags (e.g. `--skip-checkpoint`,
//!    `--no-write`).
//! 3. The `--audit-db` flag is present (T-D-N10 gate).
//!
//! The test uses keyword assertions (not a byte-exact snapshot) so that
//! prose changes to descriptions don't break CI while still catching flag
//! additions or removals.
//!
//! Required feature: `candle` (the binary only builds with it).
//! Required-features guard lives in `Cargo.toml [[test]]`.

use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo");
    let crate_dir = std::path::PathBuf::from(manifest_dir);
    crate_dir
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Invoke `cargo run -p forecast --bin train_tcn --features candle -- --help`
/// and return the combined stdout+stderr as a String.
///
/// Uses the CARGO env var so this works in cross-platform CI (cargo is always
/// on PATH inside `cargo test`). Runs from workspace root.
fn run_train_tcn_help() -> String {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let root = workspace_root();
    let out = Command::new(cargo)
        .current_dir(&root)
        .args([
            "run",
            "-p",
            "forecast",
            "--bin",
            "train_tcn",
            "--features",
            "candle",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run for train_tcn --help");

    // clap writes --help to stdout; combine both for robustness.
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    combined
}

/// R1 — Required flags must all be present in --help output.
#[test]
fn train_tcn_help_contains_required_flags() {
    let help = run_train_tcn_help();

    let required = [
        "--config",
        "--output-dir",
        "--dry-run",
        "--epochs",
        "--symbols",
        "--scenario",
        "--parquet-root",
    ];
    for flag in &required {
        assert!(
            help.contains(flag),
            "train_tcn --help missing required flag '{flag}'.\nFull output:\n{help}"
        );
    }
}

/// R2 — Forbidden flags (checkpoint-bypass) must NOT appear.
#[test]
fn train_tcn_help_no_forbidden_flags() {
    let help = run_train_tcn_help();

    let forbidden = ["--skip-checkpoint", "--no-write", "--no-checkpoint"];
    for flag in &forbidden {
        assert!(
            !help.contains(flag),
            "train_tcn --help contains forbidden flag '{flag}' — K5 contract broken.\nFull output:\n{help}"
        );
    }
}

/// R3 — `--audit-db` flag must be present after T-D-N10 lands.
///
/// This assertion intentionally gates T-D-N10: it will FAIL until the flag
/// is added to `train_tcn.rs`. That's the design — the test is the gate.
#[test]
fn train_tcn_help_has_audit_db_flag() {
    let help = run_train_tcn_help();
    assert!(
        help.contains("--audit-db"),
        "train_tcn --help missing '--audit-db' flag (T-D-N10 not yet landed).\n\
         Full output:\n{help}"
    );
}
