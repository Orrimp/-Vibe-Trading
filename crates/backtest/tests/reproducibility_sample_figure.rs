//! Story 6-12 — **one committed figure reproduces end to end, from a fresh clone.**
//!
//! Product review 2026-08-04, finding 8: every pinned corpus is machine-local and
//! gitignored, so no second party can reproduce a single real-data claim. "Honest" meant
//! honestly-intended, not verifiable-by-someone-else.
//!
//! This test closes that for exactly one claim. It runs the SAME command the runbook
//! tells a human to run, against the committed 96 KB sample corpus, and asserts the
//! report body hashes to the value `evidence/anchors.toml` has carried since 2026-05-28.
//!
//! ## Why it shells out to the binary
//!
//! It would be easy to call `sma_composed_run::run` directly and assert on the result.
//! That test would prove the library still computes the number — and would keep passing
//! if `run_yahoo_sma` drifted away from it, at which point the runbook's command and the
//! gate would be testing different things. Invoking `CARGO_BIN_EXE_run_yahoo_sma` means
//! the gate and the documented recipe cannot diverge.
//!
//! ## Why it hashes with the repo's own script
//!
//! `scripts/hash_report.py` is the canonical body-hash used by `verify_anchors.sh`. A
//! re-implementation here could agree with itself while disagreeing with the gate, which
//! is the failure this whole story exists to rule out.
//!
//! ## What this does NOT prove
//!
//! ONE figure, from ONE ticker-year. The full corpora stay machine-local. And note the
//! sharp edge, measured 2026-09-24 and recorded as bug-log `#93`: several OTHER anchored
//! scenarios in this repo **no longer reproduce their own committed bodies** — the
//! `btc-2023-1m-*` family now emits hourly bars over two years where the evidence records
//! minute bars over one. This gate is green precisely because it covers the scenario that
//! still holds. Do not read it as a statement about the corpus at large.

#![cfg(feature = "yahoo")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::process::Command;

/// The body-SHA `evidence/anchors.toml` pins for `btc-yahoo-2024-1d-sma-cross`
/// (version `lab-yahoo-realdata-v0.1.1`, locked 2026-05-28).
const ANCHORED_BODY_SHA: &str = "076929bb63d9bec03ec83684b85ced818ee32c0b2da41140712ec1d01de6a1e0";

/// The human-readable figure the body carries, quoted in the runbook so the claim is
/// legible to someone who will not read a hash.
const HEADLINE_FINAL_EQUITY: &str = "$104560.08 USDT";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_sample_corpus_reproduces_the_anchored_figure() {
    let root = repo_root();
    let out = tempfile::tempdir().expect("temp report dir");

    let status = Command::new(env!("CARGO_BIN_EXE_run_yahoo_sma"))
        .current_dir(&root)
        .args([
            "--cache-root",
            "data/yahoo-sample",
            "--ticker",
            "BTC-USD",
            "--reports-dir",
        ])
        .arg(out.path())
        .status()
        .expect("run_yahoo_sma must be runnable");
    assert!(status.success(), "run_yahoo_sma exited with {status}");

    let report = std::fs::read_dir(out.path())
        .expect("read the emitted report dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.extension().is_some_and(|e| e == "md")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains("btc-yahoo-2024-1d-sma-cross"))
        })
        .expect("the run must emit exactly one btc-yahoo-2024-1d-sma-cross report");

    let body = std::fs::read_to_string(&report).expect("read the emitted report");
    assert!(
        body.contains(HEADLINE_FINAL_EQUITY),
        "the emitted body must carry the documented headline figure {HEADLINE_FINAL_EQUITY}; \
         it reads:\n{body}"
    );

    // Hash with the repo's own canonical body-hash — the one verify_anchors.sh uses.
    let hashed = Command::new("python3")
        .current_dir(&root)
        .arg("scripts/hash_report.py")
        .arg(&report)
        .output()
        .expect("scripts/hash_report.py must be runnable (python3 is a CI prerequisite)");
    assert!(
        hashed.status.success(),
        "hash_report.py failed: {}",
        String::from_utf8_lossy(&hashed.stderr)
    );
    let stdout = String::from_utf8_lossy(&hashed.stdout);
    let sha = stdout
        .split_whitespace()
        .next()
        .expect("hash_report.py prints '<sha>  <path>'");

    assert_eq!(
        sha,
        ANCHORED_BODY_SHA,
        "the sample corpus no longer reproduces the anchored figure.\n\
         \n\
         This is the gate story 6-12 exists to provide, so read the failure carefully \
         before touching anything:\n\
         - if the ENGINE changed, this is real drift and the anchor is now a historical \
           record rather than a reproducible claim (see bug-log #93 for what that looks \
           like elsewhere in this corpus);\n\
         - if the SAMPLE changed, `reproducibility_sample_corpus` should have failed \
           first — check it;\n\
         - do NOT re-pin this constant to make the test pass. That converts a caught \
           drift into a silent one, which is bug-log #77's failure mode.\n\
         \n\
         emitted: {}",
        report.display()
    );
}
