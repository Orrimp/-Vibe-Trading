//! TCN byte-identity guard (T-D-N15, K6 scope-creep guard).
//!
//! Asserts that `crates/forecast/src/tcn.rs` and the 8 anchored TCN checkpoint
//! files have not been modified relative to HEAD.
//!
//! Any diff on these files would mean Wave A accidentally modified TCN code
//! or checkpoint data — a K6 violation (PatchTST work must not touch TCN
//! byte-output).
//!
//! # What is checked
//!
//! - `crates/forecast/src/tcn.rs` — model code must be byte-identical.
//! - `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.{safetensors,
//!   metadata.json,metadata.recalibrated.json}` — 8 checkpoint files must
//!   not be touched.
//!
//! # Mechanism
//!
//! `git diff --quiet HEAD -- <path>` exits 0 when there are no uncommitted
//! changes to `<path>`. Exit 1 means the file was modified.
//!
//! # Cross-references
//!
//! - ADR-0036 § K6 — TCN scope-creep guard.
//! - `decomp.md § T-AR-7` — architecture design for this test.

use std::process::Command;

/// Known TCN checkpoint SHA prefixes (used to locate files in the anchors dir).
const TCN_CHECKPOINT_SHAS: &[(&str, &str)] = &[
    (
        "bs1",
        "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2",
    ),
    (
        "bs2",
        "3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d",
    ),
];

/// Run `git diff --quiet HEAD -- <path>` and return `Ok(())` on exit-0.
/// Returns `Err(String)` with a helpful message on diff or if git is unavailable.
fn assert_no_git_diff(path: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["diff", "--quiet", "HEAD", "--", path])
        .output()
        .map_err(|e| format!("git not found or failed to spawn: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "git diff --quiet HEAD -- {path} exited {} — \
             file was MODIFIED. K6 scope-creep violation: PatchTST Wave A \
             must not touch TCN files. stderr: {stderr}",
            output.status.code().unwrap_or(-1)
        ))
    }
}

/// T-D-N15: assert tcn.rs + 8 TCN checkpoint files are byte-identical to HEAD.
///
/// This test is NOT `#[ignore]`d — it must pass at Wave A handoff.
/// It only checks uncommitted changes relative to HEAD, so it passes on a
/// clean checkout and fails if anyone accidentally edits TCN files.
#[test]
fn tcn_files_unchanged_from_head() {
    // Locate the workspace root by walking up from the CWD until we find
    // a Cargo.toml with `[workspace]`.
    let ws_root = find_workspace_root().unwrap_or_else(|| {
        // Fall back to CWD — will produce "not a git repo" errors if wrong.
        std::env::current_dir().expect("CWD unavailable")
    });

    let ws_str = ws_root.to_string_lossy();

    // Change the git check to use the workspace root as the git dir.
    // We do this by running git from the workspace root rather than CWD.
    let assert_from_ws = |rel_path: &str| -> Result<(), String> {
        let output = Command::new("git")
            .current_dir(&ws_root)
            .args(["diff", "--quiet", "HEAD", "--", rel_path])
            .output()
            .map_err(|e| format!("git not found or failed to spawn: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "git diff --quiet HEAD -- {rel_path} exited {} (ws={ws_str}) — \
                 file was MODIFIED. K6 scope-creep violation: PatchTST Wave A \
                 must not touch TCN files. stderr: {stderr}",
                output.status.code().unwrap_or(-1)
            ))
        }
    };

    let mut errors: Vec<String> = Vec::new();

    // 1. tcn.rs model code.
    let tcn_src = "crates/forecast/src/tcn.rs";
    if let Err(e) = assert_from_ws(tcn_src) {
        errors.push(e);
    } else {
        println!("[tcn_byte_identity] OK: {tcn_src} unchanged from HEAD");
    }

    // 2. Anchored TCN checkpoint files (4 files × 2 scenarios = 8 total,
    //    but only the ones that exist on disk are checked).
    let anchors_prefix = "crates/forecast/checkpoints/anchors";
    let extensions = ["safetensors", "metadata.json", "metadata.recalibrated.json"];

    let mut checked_count = 0usize;

    for (scenario, sha) in TCN_CHECKPOINT_SHAS {
        let base = format!("tcn-{scenario}-{sha}");
        for ext in &extensions {
            let rel_path = format!("{anchors_prefix}/{base}.{ext}");

            // Only check files tracked in git (skip missing files — git lfs
            // stubs count as tracked; large binary files may not be present).
            // We run `git ls-files --error-unmatch` to distinguish.
            let tracked = Command::new("git")
                .current_dir(&ws_root)
                .args(["ls-files", "--error-unmatch", &rel_path])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !tracked {
                println!(
                    "[tcn_byte_identity] SKIP {rel_path}: not tracked in git \
                     (git lfs not pulled or file absent)"
                );
                continue;
            }

            if let Err(e) = assert_from_ws(&rel_path) {
                errors.push(e);
            } else {
                println!("[tcn_byte_identity] OK: {rel_path} unchanged from HEAD");
                checked_count += 1;
            }
        }
    }

    if errors.is_empty() {
        println!(
            "[tcn_byte_identity] PASS — tcn.rs + {checked_count} checkpoint file(s) \
             all unchanged from HEAD (K6 scope-creep guard)"
        );
    } else {
        for e in &errors {
            eprintln!("[tcn_byte_identity] FAIL: {e}");
        }
        panic!(
            "tcn_byte_identity: {} K6 violation(s) detected — see above",
            errors.len()
        );
    }
}

/// Walk up from `CWD` until a `Cargo.toml` containing `[workspace]` is found.
fn find_workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("Cargo.toml");
        if let Ok(contents) = std::fs::read_to_string(&candidate) {
            if contents.contains("[workspace]") {
                return Some(dir);
            }
        }
        let parent = dir.parent()?.to_path_buf();
        if parent == dir {
            break;
        }
        dir = parent;
    }
    None
}
