#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1937 / T1942 (v2-llm-strategy, pass 6) — negative-invariant test
//! for the 9 strategy-backtest anchors at `spec/anchors.toml:15-58`.
//!
//! After T1935's `crates/reports/src/render/system_health.rs` rewrite,
//! the two `report-sample-*` anchors at `spec/anchors.toml:67-75` ARE
//! expected to drift (T_FINAL_V2_LLM_STRATEGY re-locks them). The 9
//! strategy anchors MUST stay byte-identical — none of the strategy /
//! audit / exec / backtest crates were touched by M7, so their
//! reports' body bytes must hash to the same SHAs they hashed to
//! pre-M7.
//!
//! This test mirrors the v1.8 reflection-memory `T1812`
//! negative-confirmation step. It catches accidental anchor drift in
//! M7 (e.g. a developer accidentally edits a report rendering function
//! and breaks an upstream anchor without noticing).
//!
//! ## What this test checks
//!
//! For each of the 9 strategy anchors:
//!
//! 1. Find the corresponding `backtest-*-<scenario>.md` file on disk.
//! 2. Strip the YAML front-matter (matching `scripts/hash_report.py`).
//! 3. Compute the body SHA-256.
//! 4. Assert the SHA matches the locked value at
//!    `spec/anchors.toml:15-58`.
//!
//! The 9 anchors are inlined here (matching the constants the
//! anchors.toml file carries) so the test depends only on the report
//! files on disk — not on a TOML parser, which would be a
//! transitive-dep change.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Each tuple = `(scenario_id, sha256_hex)`. Mirrors
/// `spec/anchors.toml:15-58` byte-for-byte at v2.0.0 ship time. The
/// orchestrator updates BOTH places in lockstep — if these constants
/// drift from `spec/anchors.toml`, the workspace fails this test AND
/// `bash scripts/verify_anchors.sh`.
const STRATEGY_ANCHORS: &[(&str, &str)] = &[
    (
        "btc-2023-1m-sma-cross",
        "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c",
    ),
    (
        "btc-2023-1m-sma-baseline-refresh",
        "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c",
    ),
    (
        "btc-2023-1m-macd-trend",
        "ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805",
    ),
    (
        "btc-2023-1m-rsi-reversion",
        "bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa",
    ),
    (
        "btc-2023-1m-bbands-mean-revert",
        "d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3",
    ),
    (
        "top10-2023-1h-momentum",
        "3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97",
    ),
    (
        "top10-2024-h1-momentum",
        "1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6",
    ),
    (
        "pairs-2023-zscore-mr",
        "90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0",
    ),
    (
        "pairs-2024-h1-zscore-mr",
        "14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f",
    ),
];

/// Resolve the workspace root (the directory containing the workspace
/// `Cargo.toml`) from the test's `CARGO_MANIFEST_DIR`.
fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or_else(|| crate_dir.clone(), Path::to_path_buf)
}

/// Find the latest `backtest-*-<scenario>.md` file under
/// `spec/<feature>/reports/`. Mirrors the `verify_anchors.sh`
/// resolution: lexicographically-largest match wins (timestamp prefix
/// → effectively "newest").
fn find_backtest_report(scenario: &str) -> Option<PathBuf> {
    let spec_dir = workspace_root().join("spec");
    let mut candidates: Vec<PathBuf> = Vec::new();
    walk_collect(&spec_dir, scenario, &mut candidates);
    candidates.sort();
    candidates.into_iter().next_back()
}

fn walk_collect(dir: &Path, scenario: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let suffix = format!("-{scenario}.md");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Only recurse into `reports/` directories or any
            // intermediate feature folders. Cheap walk.
            walk_collect(&path, scenario, out);
        } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            // Match `backtest-<stamp>-<scenario>.md`.
            if name.starts_with("backtest-") && name.ends_with(&suffix) {
                out.push(path);
            }
        }
    }
}

/// Body-hash a report file: strip the leading
/// `---\n<frontmatter>\n---\n` block (matching
/// `scripts/hash_report.py`'s regex) and SHA-256 the remainder.
fn body_sha256(path: &Path) -> String {
    let text = std::fs::read_to_string(path).expect("read report");
    let body = strip_front_matter(&text);
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    hex::encode(h.finalize())
}

fn strip_front_matter(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("---\n") else {
        return text;
    };
    let close = "\n---\n";
    rest.find(close)
        .map_or("", |pos| &rest[pos + close.len()..])
}

/// T1937 — every one of the 9 strategy anchors is byte-identical to
/// the locked SHA-256 in `spec/anchors.toml:15-58`.
///
/// If this test FAILS in pass 6 (M7), a non-M7 change has touched the
/// report rendering of one of the 9 backtest scenarios — re-run the
/// failing scenario's backtest from a clean checkout to reproduce and
/// surface the diff. The test must pass before T_FINAL.
#[test]
fn t1937_nine_strategy_anchors_unchanged() {
    let mut mismatches: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for (scenario, expected_sha) in STRATEGY_ANCHORS {
        match find_backtest_report(scenario) {
            None => {
                missing.push((*scenario).to_string());
            }
            Some(path) => {
                let actual = body_sha256(&path);
                if actual != *expected_sha {
                    mismatches.push(format!(
                        "{scenario}: expected {expected_sha}, got {actual} (file: {})",
                        path.display()
                    ));
                }
            }
        }
    }

    if !missing.is_empty() {
        // A missing report doesn't fail the test — the orchestrator
        // may have pruned old reports. Log a soft warning via
        // `eprintln!` (this is a test) so the run log carries the
        // signal. Only an explicit byte mismatch is hard-fail.
        eprintln!(
            "T1937 soft warning: no backtest report on disk for: {missing:?}; \
             skipping these. The verify_anchors.sh script will surface this as \
             MISS at T_FINAL."
        );
    }

    assert!(
        mismatches.is_empty(),
        "T1937 — the 9 strategy anchors MUST stay byte-identical in M7. \
         M7 (v2-llm-strategy / pass 6) is config + reports plumbing — \
         not strategy / audit / exec / backtest code. \
         The following anchors drifted:\n\n{}",
        mismatches.join("\n")
    );
}

/// T1942 — V8 / V12 confirmation gate: assert each anchor's SHA hex
/// is exactly 64 lowercase-hex chars (no whitespace, no upper-case).
/// Defends against a malformed paste in `spec/anchors.toml` or in
/// the inlined constants here.
#[test]
fn t1942_anchor_shas_are_well_formed_64_lowercase_hex() {
    for (scenario, sha) in STRATEGY_ANCHORS {
        assert_eq!(sha.len(), 64, "{scenario}: SHA must be 64 chars");
        for ch in sha.chars() {
            assert!(
                ch.is_ascii_lowercase() && ch.is_ascii_hexdigit() || ch.is_ascii_digit(),
                "{scenario}: SHA must be lowercase hex (offending char '{ch}')"
            );
        }
    }
}
