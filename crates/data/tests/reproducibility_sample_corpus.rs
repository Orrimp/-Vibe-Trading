//! Story 6-12 — the committed reproducibility sample corpus.
//!
//! Every other pinned corpus in this repo commits only its `REVISION.toml`: the bulk
//! parquets are gigabytes and machine-local, which is finding 8 of the 2026-08-04
//! product review — **no second party can reproduce a single real-data claim**.
//!
//! `data/yahoo-sample/` is the exception, on purpose. One ticker, one year, twelve daily
//! parquets, 96 KB, plain git, no LFS. It exists so that a fresh clone can reproduce
//! exactly one committed figure without the operator's disks.
//!
//! This file owns the corpus's integrity. The figure it produces is asserted in
//! `crates/backtest/tests/reproducibility_sample_figure.rs`; the two are deliberately
//! separate, because "the bytes are intact" and "the engine still computes the same
//! answer from them" are different claims and a single test that mixed them could pass
//! for the wrong reason.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

/// `data/yahoo-sample`, resolved from this crate rather than the process CWD — the
/// cwd-relative corpus root is what made the `ui` real-data guards vacuous for months
/// (bug-log #66).
fn sample_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/yahoo-sample")
}

/// **The rot guard (AC4).** The committed sample verifies against its own manifest.
///
/// This is the cheap half of the reproducibility claim and it runs everywhere, with no
/// feature flag and no engine: it proves the bytes a fresh clone receives are the bytes
/// the manifest pins. If someone re-fetches the slice, touches a parquet, or commits a
/// partial checkout, this fails immediately and by name.
#[test]
fn the_committed_sample_verifies_against_its_manifest() {
    let root = sample_root();
    assert!(
        root.join("REVISION.toml").is_file(),
        "the sample corpus must ship its own REVISION.toml at {}",
        root.display()
    );

    let sha = data::revision::read_and_verify_revision_manifest(&root).unwrap_or_else(|e| {
        panic!(
            "the committed sample corpus does not verify against its manifest: {e}\n\
             This means the bytes in git are not the bytes the manifest pins. Do NOT \
             re-generate the manifest to make this pass — that would pin whatever is \
             there now. Find out what changed the parquets."
        )
    });

    assert_eq!(
        sha, SAMPLE_AGGREGATE_SHA,
        "the sample corpus's aggregate SHA changed. Same warning as above: this value is \
         pinned so that a silent re-fetch cannot pass."
    );
}

/// The sample's aggregate SHA-256, pinned here as well as in its `REVISION.toml`.
///
/// Pinned in BOTH places deliberately. `read_and_verify_revision_manifest` recomputes the
/// aggregate and compares it to the manifest's own claim, so a hand-edited manifest that
/// agrees with hand-edited parquets would verify. This constant is outside that loop.
const SAMPLE_AGGREGATE_SHA: &str =
    "8f855f30231edbcd7c168317f9bb3575817ae39d65d1122014f5bf8ebc80d3b2";

/// The slice is small ON PURPOSE. If it grows, someone has widened it without deciding
/// to, and a repo that commits market data should notice that.
#[test]
fn the_sample_stays_small() {
    let dir = sample_root().join("BTC-USD/1d/2024");
    let mut files = 0usize;
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(&dir)
        .expect("the sample's parquet directory")
        .flatten()
    {
        if entry.path().extension().is_some_and(|e| e == "parquet") {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    assert_eq!(files, 12, "one daily parquet per month of 2024");
    assert!(
        bytes < 200_000,
        "the sample is meant to be ~96 KB so it needs no LFS; it is now {bytes} bytes. \
         Widening it is a decision, not an accident — make it deliberately."
    );
}

/// Regenerate the sample from the operator's full Yahoo corpus.
///
/// ```text
/// cargo test -p data --test reproducibility_sample_corpus regenerate -- --ignored --nocapture
/// ```
///
/// Only the operator can run this — it reads `data/yahoo/`, which is machine-local. The
/// OUTPUT is what ships, and `the_committed_sample_verifies_against_its_manifest` is what
/// guards it afterwards. Follows the `yahoo_revision_verify::generate_checked_in_fixture`
/// precedent.
#[test]
#[ignore = "regenerator — needs the operator's data/yahoo/; the output is committed"]
fn regenerate_the_sample_corpus() {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/yahoo/BTC-USD/1d/2024");
    assert!(
        source.is_dir(),
        "needs the operator's full Yahoo corpus at {}",
        source.display()
    );
    let dest = sample_root().join("BTC-USD/1d/2024");
    std::fs::create_dir_all(&dest).expect("create sample dir");
    for entry in std::fs::read_dir(&source).expect("read source").flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "parquet") {
            let name = p.file_name().expect("file name");
            std::fs::copy(&p, dest.join(name)).expect("copy parquet");
        }
    }
    let sha = data::revision::write_revision_manifest_with_tool(
        &sample_root(),
        data::revision::RevisionMetadataInput {
            fetch_tool: "fetch_yahoo_klines",
            binance_base: "https://query1.finance.yahoo.com",
            interval: Some("1d"),
        },
    )
    .expect("write the sample manifest");
    println!("sample aggregate SHA = {sha}");
    println!("if this differs from SAMPLE_AGGREGATE_SHA, the slice changed — say why.");
}
