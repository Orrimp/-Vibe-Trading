//! PatchTST byte-identity guard (T-D-N7, R11.8, K-vol-3 scope-creep guard).
//!
//! Asserts that `crates/forecast/src/patchtst.rs` and the anchored PatchTST
//! checkpoint files have not been modified relative to HEAD after the
//! v3-volatility-forecaster Wave A ship.
//!
//! Any diff on these files would mean the vol ship accidentally modified
//! PatchTST code or checkpoint data — a K-vol-3 violation.
//!
//! ## What is checked
//!
//! - `crates/forecast/src/patchtst.rs` — model code must be byte-identical to HEAD.
//! - `crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.{safetensors,
//!   metadata.json}` — 2 checkpoint files must not be touched.
//!
//! ## Mechanism
//!
//! `git diff --quiet HEAD -- <path>` exits 0 when there are no uncommitted
//! changes to `<path>`. Exit 1 means the file was modified.
//!
//! ## Cross-references
//!
//! - K-vol-3 — scope-creep guard.
//! - R11.8 — patchtst.rs byte-identity after vol ship.
//! - `decomp.md T-D-N7` — architecture design for this test.

use std::process::Command;

/// Known PatchTST checkpoint SHA (BS-1 only in v0.1.0).
const PATCHTST_CHECKPOINT_SHA: (&str, &str) = (
    "bs1",
    "62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d",
);

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

/// T-D-N7 (R11.8): assert patchtst.rs + PatchTST checkpoint files are
/// byte-identical to HEAD.
#[test]
fn patchtst_files_unchanged_from_head() {
    let ws_root =
        find_workspace_root().unwrap_or_else(|| std::env::current_dir().expect("CWD unavailable"));

    let ws_str = ws_root.to_string_lossy().to_string();

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
                 file was MODIFIED. K-vol-3 violation: vol Wave A must not touch \
                 PatchTST files. stderr: {stderr}",
                output.status.code().unwrap_or(-1)
            ))
        }
    };

    let mut errors: Vec<String> = Vec::new();

    // 1. patchtst.rs model code.
    let patchtst_src = "crates/forecast/src/patchtst.rs";
    if let Err(e) = assert_from_ws(patchtst_src) {
        errors.push(e);
    } else {
        println!("[patchtst_byte_identity] OK: {patchtst_src} unchanged from HEAD");
    }

    // 2. Anchored PatchTST checkpoint files.
    let (scenario, sha) = PATCHTST_CHECKPOINT_SHA;
    let anchors_prefix = "crates/forecast/checkpoints/anchors";
    let base = format!("patchtst-{scenario}-{sha}");

    let extensions = ["safetensors", "metadata.json"];
    let mut checked_count = 0usize;

    for ext in &extensions {
        let rel_path = format!("{anchors_prefix}/{base}.{ext}");

        let tracked = Command::new("git")
            .current_dir(&ws_root)
            .args(["ls-files", "--error-unmatch", &rel_path])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !tracked {
            println!(
                "[patchtst_byte_identity] SKIP {rel_path}: not tracked in git \
                 (git lfs not pulled or file absent)"
            );
            continue;
        }

        if let Err(e) = assert_from_ws(&rel_path) {
            errors.push(e);
        } else {
            println!("[patchtst_byte_identity] OK: {rel_path} unchanged from HEAD");
            checked_count += 1;
        }
    }

    if errors.is_empty() {
        println!(
            "[patchtst_byte_identity] PASS — patchtst.rs + {checked_count} \
             checkpoint file(s) all unchanged from HEAD (K-vol-3 guard)"
        );
    } else {
        for e in &errors {
            eprintln!("[patchtst_byte_identity] FAIL: {e}");
        }
        panic!(
            "patchtst_byte_identity: {} K-vol-3 violation(s) detected — see above",
            errors.len()
        );
    }
}
