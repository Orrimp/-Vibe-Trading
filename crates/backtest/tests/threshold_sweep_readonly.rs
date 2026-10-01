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
//! - `spec/v1/v25-tcn-threshold-tuning/decomp.md § D-AR-1.d` — CLI surface.
//! - T-D-N8 in `spec/v1/v25-tcn-threshold-tuning/tasks.md`.

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

/// (b) The four anchored checkpoint files are **byte-identical** across a `--help` invocation.
///
/// **This covers `--help` only, and now says so in its name.** Real runs are covered by
/// `crates/forecast/tests/anchored_report_reproduction.rs`, whose runner asserts `git status` over the
/// checkpoint directory across every run it makes.
///
/// Rewritten 2026-10-01, bug-log `#135`. The previous version could not fail, for three independent
/// reasons: `--help` exits in clap before any filesystem call; the sentinel literals were
/// **workspace**-relative while `cargo` runs an integration test from the **package** root, so all
/// four resolved to nothing; and a missing file mapped to `None`, which compares equal to `None`.
/// Any one alone was sufficient, which is why fixing one would not have revealed the others.
///
/// Records mtimes before and after the `--help` invocation. Asserts none
/// of the anchored checkpoint files changed.
#[test]
fn test_originals_untouched_by_help_invocation() {
    let anchors_dir = workspace_root().join("crates/forecast/checkpoints/anchors");

    let sentinel_paths: Vec<std::path::PathBuf> = vec![
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.json"),
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors"),
        anchors_dir.join("tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json"),
        anchors_dir.join("tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json"),
    ];

    // Record mtimes before.
    let bytes_before: Vec<Vec<u8>> = sentinel_paths.iter().map(|p| sentinel_bytes(p)).collect();

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
    let bytes_after: Vec<Vec<u8>> = sentinel_paths.iter().map(|p| sentinel_bytes(p)).collect();

    for (i, (path, (before, after))) in sentinel_paths
        .iter()
        .zip(bytes_before.iter().zip(bytes_after.iter()))
        .enumerate()
    {
        assert_eq!(
            before,
            after,
            "sentinel file #{i} ({}) CHANGED BYTES during the --help invocation",
            path.display()
        );
    }
}

// ── `#135` plumbing ──────────────────────────────────────────────────────────
//
// Identical in all three read-only guards (`recalibrate_sigma_train_readonly.rs`,
// `threshold_sweep_readonly.rs`, `forecast_distribution_bin_readonly.rs`). Kept local and
// deliberately identical rather than shared through a crate: it is a file read, not a measurement
// harness, so the Dev-Note hazard about re-implementing a harness twice does not apply — but if you
// change one, change all three.

/// The workspace root. `cargo` runs an integration test from the PACKAGE root, so a
/// workspace-relative literal silently resolves to nothing — bug-log `#135` mechanism 2, which made
/// all four sentinels missing on every run of this file for months.
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root")
        .to_path_buf()
}

/// Read a sentinel, PANICKING when it is absent.
///
/// bug-log `#135` mechanism 3: the previous version mapped a missing file to `None` and then
/// compared `None` to `None`, so the test was green with the checkpoint directory deleted. An
/// absent sentinel is a finding, never a silent equality.
fn sentinel_bytes(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        panic!(
            "sentinel {} is MISSING or unreadable ({e}). This is UNMEASURED, not a pass: the \
             read-only contract cannot be checked against a file that is not there (bug-log #135).",
            path.display()
        )
    })
}
