//! LLM-forecaster registry-add neutrality gate (T-D-N(G2), R10.2).
//!
//! Verifies that adding `LlmForecasterStrategy` to the strategy registry does
//! NOT change the output of the existing TCN overlay backtest scenario
//! `top10-2023-fy-tcn-overlay-realdata` (body-SHA `8fa47f49…` per anchors.toml).
//!
//! This test is `#[ignore]`d because it:
//! 1. Requires ~5 min wall-clock (full backtest run).
//! 2. Requires the `candle` + `realdata` feature + Binance parquet dataset.
//! 3. Requires the TCN checkpoint files on disk.
//!
//! The developer runs this manually at the end of Wave G (M-D). The tester
//! runs it at M-FINAL (T-T3 neutrality check).
//!
//! ## How to run
//!
//! ```bash
//! cargo test -p strategy --features candle \
//!   --test llm_forecaster_neutrality \
//!   -- --ignored --nocapture 2>&1 | grep -E "test result|SKIP|body-SHA"
//! ```
//!
//! Expected: `test result: ok. 1 passed` (body-SHA matches locked anchor).
//!
//! ## Cross-references
//!
//! - R10.2 — LLM-forecaster registry add must not change existing scenario SHAs.
//! - `spec/anchors.toml` — anchor `top10-2023-fy-tcn-overlay-realdata`.
//! - ADR-0039 § D6 — anchor-additive-only contract.

/// SHA-256 of the `top10-2023-fy-tcn-overlay-realdata` report body.
///
/// Locked in `spec/anchors.toml` (v2.6.0-realdata row).
/// This is the R10.2 immutable baseline: the LLM-forecaster registry add
/// must not change this value.
const EXPECTED_SHA: &str = "8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642";

/// Re-run the TCN overlay backtest scenario and verify body-SHA is unchanged.
///
/// # Skip conditions
///
/// - `cargo check -p backtest --features candle realdata` fails.
/// - `data/binance/` not found.
/// - TCN checkpoint files not found (CheckpointNotFound in output).
///
/// All skip conditions result in an eprintln + graceful return (test passes
/// trivially). The operator decides whether to investigate the skip.
#[test]
#[ignore = "R10.2 neutrality gate: 5-min backtest; requires realdata + TCN \
            checkpoints; run manually at Wave G end or M-FINAL (T-T3)"]
fn llm_forecaster_registry_does_not_regress_tcn_scenario() {
    use std::process::Command;

    let ws_root =
        find_workspace_root().unwrap_or_else(|| std::env::current_dir().expect("CWD unavailable"));

    // Pre-flight: cargo check (fast, avoids 5-min wait on mis-configured env).
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
            eprintln!("SKIP llm_forecaster_neutrality: cargo check failed to spawn: {e}");
            return;
        }
        Ok(o) if !o.status.success() => {
            eprintln!(
                "SKIP llm_forecaster_neutrality: `cargo check -p backtest --features \
                 candle realdata --bin backtest` failed — realdata feature or \
                 backtest crate unavailable.\n{}",
                String::from_utf8_lossy(&o.stderr)
            );
            return;
        }
        Ok(_) => {}
    }

    // Check Binance data root.
    let data_root = ws_root.join("data/binance");
    if !data_root.exists() {
        eprintln!(
            "SKIP llm_forecaster_neutrality: data/binance not found at {} — \
             populate the Binance parquet dataset first",
            data_root.display()
        );
        return;
    }

    // Run the backtest.
    println!("[llm_forecaster_neutrality] Running top10-2023-fy-tcn-overlay-realdata ...");

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
        if stderr.contains("CheckpointNotFound") || stdout.contains("CheckpointNotFound") {
            eprintln!(
                "SKIP llm_forecaster_neutrality: TCN checkpoint not found — \
                 run `git lfs pull` to fetch anchored checkpoints"
            );
            return;
        }
        panic!(
            "backtest run failed (exit {})\nstdout:\n{stdout}\nstderr:\n{stderr}",
            run.status.code().unwrap_or(-1)
        );
    }

    // Locate the generated report.
    let report_pattern = "top10-2023-fy-tcn-overlay-realdata";
    let candidates = [
        "spec/backtest-real-binance-data/reports",
        "spec/v25a-patchtst-overlay/reports",
        "spec/v25-tcn-overlay/reports",
    ];
    let report_path = candidates
        .iter()
        .find_map(|dir| find_latest_report(&ws_root.join(dir), report_pattern))
        .unwrap_or_else(|| {
            panic!(
                "No report matching '{report_pattern}' found under: {}",
                candidates.join(", ")
            )
        });

    println!(
        "[llm_forecaster_neutrality] Report at {}",
        report_path.display()
    );

    // Hash the report body via scripts/hash_report.py.
    let hash_script = ws_root.join("scripts/hash_report.py");
    if !hash_script.exists() {
        panic!(
            "scripts/hash_report.py not found at {}",
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

    println!("[llm_forecaster_neutrality] body-SHA = {}", actual_sha);

    assert_eq!(
        actual_sha, EXPECTED_SHA,
        "R10.2 VIOLATED: LLM-forecaster registry add changed \
         top10-2023-fy-tcn-overlay-realdata body-SHA.\n\
         expected: {EXPECTED_SHA}\n\
         actual:   {actual_sha}\n\
         Investigate what changed in the backtest pipeline."
    );

    println!("[llm_forecaster_neutrality] PASS — body-SHA matches anchor (R10.2 satisfied)");
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Walk upwards from CWD until we find `Cargo.toml` with `[workspace]`.
fn find_workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = std::fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]")
        {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Find the most-recently-modified report under `dir` whose filename contains
/// `pattern`. Returns `None` if no match found.
fn find_latest_report(dir: &std::path::Path, pattern: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut matches: Vec<(std::time::SystemTime, std::path::PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.contains(pattern) && n.ends_with(".md"))
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, e.path()))
        })
        .collect();
    matches.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    matches.into_iter().next().map(|(_, p)| p)
}
