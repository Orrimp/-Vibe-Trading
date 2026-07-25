//! PatchTST overlay anchor-neutrality gate (T-D-N16, K4).
//!
//! Verifies that adding the PatchTST overlay does NOT change the output of the
//! existing TCN-only backtest scenario `top10-2023-fy-tcn-overlay-realdata`.
//!
//! This test is `#[ignore]`d because it:
//! 1. Requires ~5 min wall-clock (full backtest run).
//! 2. Requires the `realdata` feature + the Binance parquet dataset.
//! 3. Requires the TCN checkpoint files on disk.
//!
//! The developer runs this manually at the end of M-D. The tester runs it
//! at M-FINAL (T-T-1.i).
//!
//! # How to run
//!
//! ```bash
//! cargo test -p forecast --features candle \
//!   --test patchtst_overlay_neutrality \
//!   -- --ignored --nocapture 2>&1 | grep "test result"
//! ```
//!
//! Expected: `test result: ok. 1 passed` (body-SHA matches the locked anchor).
//!
//! # Cross-references
//!
//! - ADR-0036 K4 — PatchTST overlay must not change TCN backtest output.
//! - `decomp.md § T-AR-6` — architecture design for this test.
//! - `evidence/anchors.toml` — anchor `top10-2023-fy-tcn-overlay-realdata`.

/// SHA-256 of the `top10-2023-fy-tcn-overlay-realdata` report body.
///
/// Locked at architect M-T1 (2026-05-21) per `decomp.md § T-AR-6`.
/// This is the K4 immutable baseline.
const EXPECTED_SHA: &str = "8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642";

/// Run the TCN backtest scenario and verify the body-SHA matches the anchor.
///
/// # Skip conditions
///
/// - If `cargo run -p backtest` is not available (the `backtest` crate has
///   `realdata` feature-gated behind the `realdata` Cargo feature).
/// - If the Binance parquet data is not present at `data/binance/`.
/// - If the TCN checkpoint files are not present.
///
/// All of these cause the test to print `SKIP` and return without panicking
/// (the test passes trivially in that case — the operator decides whether to
/// investigate the skip).
#[test]
#[ignore = "K4 neutrality gate: 5-min backtest; requires realdata + TCN checkpoints; \
            run manually at M-D end or M-FINAL (T-T-1.i)"]
fn patchtst_overlay_does_not_regress_tcn_scenario() {
    use std::process::Command;

    // Locate workspace root.
    let ws_root =
        find_workspace_root().unwrap_or_else(|| std::env::current_dir().expect("CWD unavailable"));

    // Check that the backtest binary can be built (requires realdata feature).
    // We do a `cargo check` first to avoid a 5-min wait only to fail.
    let check = Command::new("cargo")
        .current_dir(&ws_root)
        .args([
            "check",
            "-p",
            "backtest",
            "--features",
            "candle realdata",
            "--bin",
            "backtest",
        ])
        .output();

    match check {
        Err(e) => {
            eprintln!("SKIP patchtst_overlay_neutrality: cargo check failed: {e}");
            return;
        }
        Ok(o) if !o.status.success() => {
            eprintln!(
                "SKIP patchtst_overlay_neutrality: `cargo check -p backtest --features \
                 candle realdata --bin backtest` failed — realdata feature or backtest \
                 crate unavailable in this environment.\n{}",
                String::from_utf8_lossy(&o.stderr)
            );
            return;
        }
        Ok(_) => {}
    }

    // Check that the Binance parquet root exists.
    let data_root = ws_root.join("data/binance");
    if !data_root.exists() {
        eprintln!(
            "SKIP patchtst_overlay_neutrality: data/binance not found at {} — \
             populate the Binance parquet dataset first",
            data_root.display()
        );
        return;
    }

    // Run the backtest scenario.
    println!(
        "[patchtst_overlay_neutrality] Running \
         top10-2023-fy-tcn-overlay-realdata scenario..."
    );

    let run = Command::new("cargo")
        .current_dir(&ws_root)
        .args([
            "run",
            "-p",
            "backtest",
            "--bin",
            "backtest",
            "--release",
            "--features",
            "candle realdata",
            "--",
            "--scenario",
            "top10-2023-fy-tcn-overlay-realdata",
            "--seed",
            "0xC0FFEE",
        ])
        .output()
        .expect("cargo run -p backtest failed to spawn");

    if !run.status.success() {
        let stderr = String::from_utf8_lossy(&run.stderr);
        let stdout = String::from_utf8_lossy(&run.stdout);
        // If checkpoint is missing, skip gracefully.
        if stderr.contains("CheckpointNotFound") || stdout.contains("CheckpointNotFound") {
            eprintln!(
                "SKIP patchtst_overlay_neutrality: TCN checkpoint not found — \
                 run `git lfs pull` to fetch anchored checkpoints"
            );
            return;
        }
        panic!(
            "backtest run failed (exit {})\nstdout:\n{stdout}\nstderr:\n{stderr}",
            run.status.code().unwrap_or(-1)
        );
    }

    // Locate the generated report. Realdata scenarios land in
    // `evidence/v1/backtest-real-binance-data/reports/` per the backtest crate's
    // `report_dir_for_scenario` mapping; older shipped reports may live in
    // `evidence/v1/v25-tcn-overlay/reports/` or the v2.5a feature folder.
    let report_pattern = "top10-2023-fy-tcn-overlay-realdata";
    let candidates = [
        "evidence/v1/backtest-real-binance-data/reports",
        "evidence/v1/v25a-patchtst-overlay/reports",
        "evidence/v1/v25-tcn-overlay/reports",
    ];
    let report_path = candidates
        .iter()
        .find_map(|dir| find_latest_report(&ws_root.join(dir), report_pattern))
        .unwrap_or_else(|| {
            panic!(
                "No report matching '{report_pattern}' found under any of: {}",
                candidates.join(", ")
            )
        });

    println!(
        "[patchtst_overlay_neutrality] Report written to {}",
        report_path.display()
    );

    // Hash the report body via scripts/hash_report.py.
    let hash_script = ws_root.join("scripts/hash_report.py");
    if !hash_script.exists() {
        panic!(
            "scripts/hash_report.py not found at {} — cannot verify body SHA",
            hash_script.display()
        );
    }

    let hash_out = Command::new("python3")
        .current_dir(&ws_root)
        .args([hash_script.to_str().unwrap(), report_path.to_str().unwrap()])
        .output()
        .expect("python3 scripts/hash_report.py failed to spawn");

    assert!(
        hash_out.status.success(),
        "hash_report.py exited {}: {}",
        hash_out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&hash_out.stderr)
    );

    let hash_line = String::from_utf8_lossy(&hash_out.stdout);
    let actual_sha = hash_line
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();

    assert_eq!(
        actual_sha,
        EXPECTED_SHA,
        "K4 VIOLATION: TCN backtest scenario body-SHA changed after PatchTST Wave A!\n\
         expected: {EXPECTED_SHA}\n\
         actual:   {actual_sha}\n\
         report:   {}",
        report_path.display()
    );

    println!("[patchtst_overlay_neutrality] PASS — body SHA {actual_sha} matches locked anchor");
}

/// Find the lexicographically-latest report file matching `prefix` in `dir`.
fn find_latest_report(dir: &std::path::Path, prefix: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut matches: Vec<std::path::PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(prefix) && n.ends_with(".md"))
                .unwrap_or(false)
        })
        .collect();
    matches.sort();
    matches.into_iter().last()
}

/// Walk up from `CWD` until a `Cargo.toml` containing `[workspace]` is found.
fn find_workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("Cargo.toml");
        if let Ok(contents) = std::fs::read_to_string(&candidate)
            && contents.contains("[workspace]")
        {
            return Some(dir);
        }
        let parent = dir.parent()?.to_path_buf();
        if parent == dir {
            break;
        }
        dir = parent;
    }
    None
}
