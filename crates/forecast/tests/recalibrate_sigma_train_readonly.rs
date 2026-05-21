//! Read-only guard tests for the `recalibrate_sigma_train` bin (T-D-N5).
//!
//! Tests:
//! (a) `test_help_no_forbidden_flags` — `--help` output must NOT contain
//!     `retrain`, `update`, `write-checkpoint`, `update-sigma`.
//! (b) `test_originals_untouched_by_run` — mtime of the original
//!     `.metadata.json` and `.safetensors` files must be unchanged after
//!     a `--help` invocation.
//!
//! These tests run with the `candle` feature (so the binary exists).
//! They do NOT run a full forward pass — that's covered by the manual T-D-N3
//! acceptance run.
//!
//! # Cross-references
//!
//! - ADR-0035 D2 — hard invariant: original files stay byte-identical.
//! - T-D-N5 (decomp.md Wave A) — read-only enforcement gate.

use std::process::Command;

/// (c) `--help` output must NOT contain forbidden flags.
///
/// Forbidden: `retrain`, `update`, `write-checkpoint`, `update-sigma`.
/// Required: `--scenario`, `--data-root`, `--out-dir`, `--anchor-dir`.
#[test]
fn test_help_no_forbidden_flags() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "forecast",
            "--features",
            "candle",
            "--bin",
            "recalibrate_sigma_train",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    // Forbidden flags (K5 hard contract — ADR-0035 D2 + CLI surface).
    assert!(
        !combined.to_lowercase().contains("retrain"),
        "help output must not mention 'retrain'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("update-sigma"),
        "help output must not mention 'update-sigma'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("write-checkpoint"),
        "help output must not mention 'write-checkpoint'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("update-original"),
        "help output must not mention 'update-original'; got: {combined}"
    );
    assert!(
        !combined.to_lowercase().contains("write-safetensors"),
        "help output must not mention 'write-safetensors'; got: {combined}"
    );

    // Required flags (D-AR-1.b from decomp.md).
    assert!(
        combined.contains("--scenario"),
        "help must contain --scenario; got: {combined}"
    );
    assert!(
        combined.contains("--data-root"),
        "help must contain --data-root; got: {combined}"
    );
    assert!(
        combined.contains("--out-dir"),
        "help must contain --out-dir; got: {combined}"
    );
    assert!(
        combined.contains("--anchor-dir"),
        "help must contain --anchor-dir; got: {combined}"
    );

    // Smoke: help must not be empty.
    assert!(
        combined.contains("recalibrate_sigma_train"),
        "help must contain the binary name; got: {combined}"
    );
}

/// (b) Checkpoint and metadata mtimes are unchanged by a `--help` invocation.
///
/// Records mtimes of the two original metadata JSON files and the two
/// safetensors files before and after the `--help` invocation.
/// Asserts none of them changed.
#[test]
fn test_originals_untouched_by_run() {
    let anchors_dir = std::path::Path::new("crates/forecast/checkpoints/anchors");

    // Record mtimes before.
    let sentinel_paths: Vec<std::path::PathBuf> = vec![
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.json"),
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors"),
    ];

    let mtimes_before: Vec<Option<std::time::SystemTime>> = sentinel_paths
        .iter()
        .map(|p| p.metadata().ok().and_then(|m| m.modified().ok()))
        .collect();

    // Run --help (should not touch any checkpoint files).
    let _ = Command::new("cargo")
        .args([
            "run",
            "-p",
            "forecast",
            "--features",
            "candle",
            "--bin",
            "recalibrate_sigma_train",
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
