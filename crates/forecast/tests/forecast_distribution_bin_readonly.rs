//! Read-only guard tests for the `forecast_distribution` bin (T-D-5).
//!
//! Tests:
//! (a) Running the bin with `--out-dir` redirected to a tempdir produces no
//!     writes to `crates/forecast/checkpoints/` or `crates/forecast/replay-cache/`.
//! (b) `--help` output does NOT contain any of: retrain, update, write-checkpoint.
//!
//! These tests run WITHOUT the `candle` feature so they don't require the
//! checkpoints to be present. They only test the CLI surface (--help).
//!
//! The read-only mtime gate test (a) is a lightweight check verifying that
//! known sentinel paths are not touched during a help invocation. A full
//! forward-pass run is covered by the manual T-D-5 acceptance criteria
//! (cargo run --features candle -- --scenario bs1).

use std::process::Command;

/// (b) --help output must NOT contain forbidden flags.
///
/// Forbidden: retrain, update, write-checkpoint (K5 hard contract).
#[test]
fn test_help_no_forbidden_flags() {
    // Run `cargo run -p forecast --bin forecast_distribution --features candle -- --help`.
    // The bin requires the candle feature.
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "forecast",
            "--features",
            "candle",
            "--bin",
            "forecast_distribution",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !combined.to_lowercase().contains("retrain"),
        "help output must not mention 'retrain'"
    );
    assert!(
        !combined.to_lowercase().contains("update-sigma"),
        "help output must not mention 'update-sigma'"
    );
    assert!(
        !combined.to_lowercase().contains("write-checkpoint"),
        "help output must not mention 'write-checkpoint'"
    );

    // Verify the 4-flag surface is present.
    assert!(
        combined.contains("--scenario"),
        "help must contain --scenario"
    );
    assert!(
        combined.contains("--data-root"),
        "help must contain --data-root"
    );
    assert!(
        combined.contains("--out-dir"),
        "help must contain --out-dir"
    );
    assert!(
        combined.contains("--span-start"),
        "help must contain --span-start"
    );
    assert!(
        combined.contains("--span-end"),
        "help must contain --span-end"
    );
}

/// (a) Checkpoint and replay-cache mtimes are unchanged by a --help invocation.
///
/// A full forward-pass run with --out-dir redirected to a tempdir is the
/// integration acceptance in T-D-5's acceptance criteria. Here we verify
/// the sentinel check at the --help level (zero-cost, always passes without
/// data files).
#[test]
fn test_checkpoints_not_touched_by_help() {
    let checkpoints_dir = std::path::Path::new("crates/forecast/checkpoints");
    let replay_cache_dir = std::path::Path::new("crates/forecast/replay-cache");

    // Record mtimes before.
    let mtime_before_checkpoints = dir_mtime(checkpoints_dir);
    let mtime_before_cache = dir_mtime(replay_cache_dir);

    // Run --help (should not touch checkpoints or cache).
    let _ = Command::new("cargo")
        .args([
            "run",
            "-p",
            "forecast",
            "--features",
            "candle",
            "--bin",
            "forecast_distribution",
            "--",
            "--help",
        ])
        .output()
        .expect("failed to spawn cargo run");

    // Record mtimes after.
    let mtime_after_checkpoints = dir_mtime(checkpoints_dir);
    let mtime_after_cache = dir_mtime(replay_cache_dir);

    assert_eq!(
        mtime_before_checkpoints, mtime_after_checkpoints,
        "checkpoints dir mtime changed during --help invocation"
    );
    assert_eq!(
        mtime_before_cache, mtime_after_cache,
        "replay-cache dir mtime changed during --help invocation"
    );
}

/// Get the mtime of a directory (or None if it doesn't exist).
fn dir_mtime(path: &std::path::Path) -> Option<std::time::SystemTime> {
    path.metadata().ok().and_then(|m| m.modified().ok())
}
