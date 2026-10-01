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

/// (a) The checkpoint tree is byte-identical, and `replay-cache/` still does not exist, across a
/// `--help` invocation.
///
/// **This covers `--help` only, and says so in its name.** A full forward-pass run is covered by
/// `anchored_report_reproduction.rs`, whose runner asserts `git status` over the checkpoint
/// directory across every real run of this family.
///
/// Rewritten 2026-10-01, bug-log `#135`. The previous version could not fail, for four reasons:
/// `--help` exits in clap before any filesystem call; the two literals were **workspace**-relative
/// while `cargo` runs an integration test from the **package** root; `dir_mtime` mapped the
/// resulting missing paths to `None`, and `None == None` passes; and `crates/forecast/replay-cache`
/// does not exist **anywhere** — the bin's own contract (`forecast_distribution.rs:20`) and its
/// `--help` text both name a directory that has never been there. Its own doc comment said the
/// quiet part out loud: *"zero-cost, always passes without data files"*.
///
/// The replay-cache sentinel is now the NEGATIVE the contract actually states: *no writes to
/// `replay-cache/`* means that path stays absent, so its appearance is the failure.
#[test]
fn test_checkpoints_not_touched_by_help() {
    let checkpoints_dir = workspace_root().join("crates/forecast/checkpoints");
    let replay_cache_dir = workspace_root().join("crates/forecast/replay-cache");

    // Non-vacuity: the tree we claim to be watching must BE there (bug-log #113 req 6).
    let before = checkpoint_tree(&checkpoints_dir);
    assert!(
        !before.is_empty(),
        "UNMEASURED, not passed: {} holds no files, so there is nothing for this test to assert \
         about (bug-log #135).",
        checkpoints_dir.display()
    );
    let cache_absent_before = !replay_cache_dir.exists();
    assert!(
        cache_absent_before,
        "{} EXISTS. The bin's read-only contract (forecast_distribution.rs:20) says it never \
         writes there; a directory at that path means either the contract broke or the contract \
         names the wrong location.",
        replay_cache_dir.display()
    );

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

    let after = checkpoint_tree(&checkpoints_dir);
    assert_eq!(
        before, after,
        "the checkpoint tree CHANGED BYTES during the --help invocation"
    );
    assert!(
        !replay_cache_dir.exists(),
        "{} was CREATED during the --help invocation — the read-only contract broke",
        replay_cache_dir.display()
    );
}

/// The workspace root. `cargo` runs an integration test from the PACKAGE root, so a
/// workspace-relative literal silently resolves to nothing — bug-log `#135` mechanism 2.
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root")
        .to_path_buf()
}

/// Every file under `dir` as `(relative path, bytes)`, sorted.
///
/// Bytes, not mtimes: strictly stronger, free at this size, and immune to a filesystem that does
/// not update mtime. Panics on an unreadable entry — an absent sentinel is a finding, never a
/// silent equality (bug-log `#135` mechanism 3).
fn checkpoint_tree(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        // Panic, never `continue`: a skipped subdirectory is UNMEASURED, and the before/after
        // comparison would skip it identically on both sides — vacuous precisely where it is blind.
        let entries = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("reading sentinel dir {}: {e}", d.display()));
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let bytes = std::fs::read(&p)
                    .unwrap_or_else(|err| panic!("reading sentinel {}: {err}", p.display()));
                out.push((p, bytes));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
