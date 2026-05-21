//! Read-only guard tests for the `threshold_sweep` bin (T-D-N8).
//!
//! Tests:
//! (a) `test_help_no_forbidden_flags` — `--help` output must NOT contain
//!     `retrain`, `update`, `write-checkpoint`, `write-metadata`.
//! (b) `test_originals_untouched_by_run` — mtime of the anchored checkpoint
//!     files must be unchanged after a `--help` invocation.
//!
//! Mirrors `forecast/tests/recalibrate_sigma_train_readonly.rs` shape.
//!
//! # Cross-references
//!
//! - ADR-0035 D2 / ADR-0033 § D1.c — read-only hard invariant.
//! - `spec/v25-tcn-threshold-tuning/decomp.md § D-AR-1.d` — CLI surface.
//! - T-D-N8 in `spec/v25-tcn-threshold-tuning/tasks.md`.

use std::process::Command;

/// (a) `--help` output must NOT contain forbidden flags.
///
/// Forbidden: `retrain`, `update`, `write-checkpoint`, `write-metadata`.
/// Required: `--scenario`, `--data-root`, `--metadata-path`, `--out-dir`,
///           `--expected-revision-sha`.
#[test]
fn test_help_no_forbidden_flags() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "backtest",
            "--features",
            "candle,realdata",
            "--bin",
            "threshold_sweep",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    // Forbidden flags (read-only contract).
    assert!(
        !combined.to_lowercase().contains("retrain"),
        "help must not mention 'retrain'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("write-checkpoint"),
        "help must not mention 'write-checkpoint'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("write-metadata"),
        "help must not mention 'write-metadata'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("update-sigma"),
        "help must not mention 'update-sigma'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("update-original"),
        "help must not mention 'update-original'; got: {combined}"
    );

    // Required flags (D-AR-1.d).
    assert!(
        combined.contains("--scenario"),
        "help must contain --scenario; got: {combined}"
    );
    assert!(
        combined.contains("--data-root"),
        "help must contain --data-root; got: {combined}"
    );
    assert!(
        combined.contains("--metadata-path"),
        "help must contain --metadata-path; got: {combined}"
    );
    assert!(
        combined.contains("--out-dir"),
        "help must contain --out-dir; got: {combined}"
    );
    assert!(
        combined.contains("--expected-revision-sha"),
        "help must contain --expected-revision-sha; got: {combined}"
    );

    // Smoke: binary name must appear somewhere.
    assert!(
        combined.contains("threshold_sweep"),
        "help must contain binary name 'threshold_sweep'; got: {combined}"
    );
}

/// (b) Checkpoint and metadata mtimes are unchanged by a `--help` invocation.
///
/// Records mtimes before and after the `--help` invocation. Asserts none
/// of the anchored checkpoint files changed.
#[test]
fn test_originals_untouched_by_run() {
    let anchors_dir = std::path::Path::new("crates/forecast/checkpoints/anchors");

    let sentinel_paths: Vec<std::path::PathBuf> = vec![
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.json"),
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors"),
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json"),
    ];

    // Record mtimes before.
    let mtimes_before: Vec<Option<std::time::SystemTime>> = sentinel_paths
        .iter()
        .map(|p| p.metadata().ok().and_then(|m| m.modified().ok()))
        .collect();

    // Run --help (must not touch any checkpoint file).
    let _ = Command::new("cargo")
        .args([
            "run",
            "-p",
            "backtest",
            "--features",
            "candle,realdata",
            "--bin",
            "threshold_sweep",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    // Record mtimes after.
    let mtimes_after: Vec<Option<std::time::SystemTime>> = sentinel_paths
        .iter()
        .map(|p| p.metadata().ok().and_then(|m| m.modified().ok()))
        .collect();

    for (i, (path, (before, after))) in sentinel_paths
        .iter()
        .zip(mtimes_before.iter().zip(mtimes_after.iter()))
        .enumerate()
    {
        assert_eq!(
            before,
            after,
            "sentinel file #{i} ({}) mtime changed during --help invocation",
            path.display()
        );
    }
}
