#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The Rust body-hash convention agrees with `scripts/hash_report.py` (bug-log `#162`).
//!
//! ## Why this test exists
//!
//! `scripts/hash_report.py` is the canonical body-hash function: it strips
//! `^---\n.*?\n---\n` and hashes the rest, and `scripts/verify_anchors.sh` execs
//! it for all 119 anchors. `tests/report_scenarios.rs::body_after_fence` is the
//! Rust mirror of that convention, and its own doc comment says why it matters:
//!
//! > The earlier draft of this helper trimmed an extra `\n` boundary […] the
//! > 9-anchor regression gate uses the python convention, so this test must too
//! > — **diverging would silently miss real anchor drift**.
//!
//! That sentence was the entire gate. Nothing in `crates/reports` executed the
//! python hasher, so the agreement was asserted in a comment and would have held
//! for any mirror that drifted. Six tests elsewhere do exec it
//! (`forecast/tests/{patchtst_overlay_neutrality,anchored_report_reproduction,
//! recalibrate_sigma_train_readonly}.rs`, `trader/tests/llm_forecaster_neutrality.rs`,
//! `backtest/tests/theta_surface_reproduction.rs`) — but not this crate, which is
//! the one that **renders** two of the 119 anchored bodies
//! (`report-sample-7d`, `report-sample-90d`, four anchor rows across two
//! namespaces).
//!
//! That the gap was real and not theoretical: `ReportArtifacts` carried a
//! `body_sha256` field computed on the WRONG side of the boundary — it hashed the
//! in-memory body, one leading newline short of the canonical slice — and the
//! divergence survived since v1 with 119/119 green, because the field had zero
//! readers. Measured on a real anchored body: `e1b286d5…` canonical vs
//! `532b09de…`. The field is gone; this test is what replaces it.

use std::path::{Path, PathBuf};
use std::process::Command;

use reports::{FrozenMarkSource, ReportWindow};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[path = "fixtures/build_ledger_7d.rs"]
mod build_ledger_7d;

use crate::build_ledger_7d::{
    FIXTURE_SEED, build_ledger_7d, fixture_period_end, fixture_period_start,
};

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or(crate_dir.clone(), Path::to_path_buf)
}

/// The Rust mirror of `hash_report.py`'s body slice, kept character-for-character
/// in step with `report_scenarios.rs::body_after_fence`. The body starts at the
/// byte AFTER the closing fence's newline, which leaves the blank line that
/// follows it INSIDE the hashed region — that blank line is the whole boundary
/// this test exists to pin.
fn body_after_fence(full: &str) -> &str {
    let Some(rest) = full.strip_prefix("---\n") else {
        return full;
    };
    let close_marker = "\n---\n";
    rest.find(close_marker)
        .map_or("", |pos| &rest[pos + close_marker.len()..])
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// Run the canonical hasher on `path` and return its digest.
fn canonical_sha(ws_root: &Path, path: &Path) -> String {
    let script = ws_root.join("scripts/hash_report.py");
    assert!(
        script.exists(),
        "scripts/hash_report.py not found at {} — this test cannot verify the \
         convention without the canonical implementation, and a skip here would \
         be an unmeasured gate, not a pass",
        script.display()
    );
    let out = Command::new("python3")
        .current_dir(ws_root)
        .args([script.to_str().unwrap(), path.to_str().unwrap()])
        .output()
        .expect("spawn python3 scripts/hash_report.py");
    assert!(
        out.status.success(),
        "hash_report.py exited {}: {}",
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let digest = stdout.split_whitespace().next().unwrap_or("").to_string();
    assert_eq!(
        digest.len(),
        64,
        "hash_report.py printed {stdout:?}, which does not start with a 64-char \
         digest — the output format changed and this test is reading the wrong field"
    );
    assert!(
        digest.chars().all(|c| c.is_ascii_hexdigit()),
        "hash_report.py digest is not hex: {digest:?}"
    );
    digest
}

#[tokio::test]
async fn rust_body_slice_agrees_with_canonical_hash_report_py() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit-7d.db");
    let _ = build_ledger_7d(&db_path).await.expect("build 7d fixture");

    let out_path = dir.path().join("report.md");
    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();
    reports::generate_at(
        fixture_period_end(),
        ReportWindow::Since(fixture_period_start()),
        &db_path,
        &frozen,
        &out_path,
        Some(FIXTURE_SEED),
    )
    .await
    .expect("render should succeed on a balanced fixture");

    let full = std::fs::read_to_string(&out_path).expect("read rendered report");
    let body = body_after_fence(&full);

    // Non-vacuity, declared and checked: the slice must be a real body, and it
    // must begin with the blank line that is the entire point of the boundary.
    // Without these two the test would pass on an empty slice — the "affirms a
    // property of a set it never proves non-empty" shape.
    assert!(
        body.len() > 200,
        "body slice is {} bytes — too short to be a rendered report, so the \
         comparison below would be between two hashes of nothing",
        body.len()
    );
    assert!(
        body.starts_with("\n## "),
        "the body slice must START with the blank line after the closing fence \
         followed by the first section heading; got {:?}. If this ever starts \
         with `## ` directly, the mirror has drifted to the producer's old \
         (wrong) convention and the digests below will disagree.",
        &body[..body.len().min(16)]
    );

    let rust_digest = sha256_hex(body);
    let canonical = canonical_sha(&workspace_root(), &out_path);

    assert_eq!(
        rust_digest, canonical,
        "the Rust body slice and scripts/hash_report.py disagree.\n\
         rust      : {rust_digest}\n\
         canonical : {canonical}\n\n\
         These must agree byte-for-byte: verify_anchors.sh hashes the committed \
         bodies with the python implementation, while report_scenarios.rs locks \
         EXPECTED_SHA_7D / EXPECTED_SHA_90D with the Rust one. A divergence \
         means the two gates are measuring different things, and the anchored \
         report-sample-* bodies would drift without either noticing. The most \
         likely cause is a mirror that trims one newline too many at the fence \
         boundary — which is exactly how the deleted ReportArtifacts.body_sha256 \
         field was wrong (bug-log #162)."
    );
}
