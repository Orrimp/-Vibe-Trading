//! Anchor-safety test for `dynamic_cache::load_or_fetch` (M-TEST.B4).
//!
//! ## Non-negotiable guarantee (ADR-0061 D4)
//!
//! The dynamic cache path MUST NOT:
//! (a) create or modify any file under a sentinel `data/binance/`-shaped fixture;
//! (b) write a `REVISION.toml` under the dynamic root;
//! (c) break `read_and_verify_revision_manifest` on the pinned fixture.
//!
//! Uses a mock fetcher — NO live network.
//!
//! ## Feature requirement
//!
//! This test requires `--features fixtures` to access `MockFetcher` and `make_batch`.
//! Run with: `cargo test -p data --features fixtures dynamic_cache_anchor_safety`

// Only compile this test when the `fixtures` feature exposes MockFetcher/make_batch.
#![cfg(feature = "fixtures")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use data::{
    binance_klines::{Kline, MockFetcher, make_batch},
    dynamic_cache::load_or_fetch_with,
    revision::{file_sha256, read_and_verify_revision_manifest, write_revision_manifest},
};
use tempfile::tempdir;
use trading_core::Symbol;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Snapshot the (path, mtime, sha256) tuple for every file in `dir` recursively.
fn snapshot_dir(dir: &Path) -> HashMap<PathBuf, (SystemTime, String)> {
    let mut map = HashMap::new();
    if !dir.exists() {
        return map;
    }
    walk(dir, &mut map);
    map
}

fn walk(dir: &Path, map: &mut HashMap<PathBuf, (SystemTime, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, map);
        } else {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let sha = file_sha256(&p).unwrap_or_default();
            map.insert(p, (mtime, sha));
        }
    }
}

/// Write a minimal REVISION-pinned corpus fixture:
///
/// ```
/// <corpus_root>/
/// ├── REVISION.toml
/// └── BTCUSDT/
///     └── 2024/
///         └── 01.parquet  (minimal valid parquet)
/// ```
fn create_pinned_corpus_fixture(corpus_root: &Path) {
    // Write a dummy parquet via write_parquet.
    let parquet_path = corpus_root.join("BTCUSDT").join("2024").join("01.parquet");

    let klines = vec![Kline {
        open_time: 1_704_067_200_000,
        close_time: 1_704_070_799_999,
        open: "42000.00".to_owned(),
        high: "42500.00".to_owned(),
        low: "41800.00".to_owned(),
        close: "42300.00".to_owned(),
        volume: "10.0".to_owned(),
        trade_count: 100,
    }];

    data::binance_klines::write_parquet(&klines, &parquet_path).expect("write corpus fixture");
    write_revision_manifest(corpus_root).expect("write REVISION.toml");
}

// ── Test ─────────────────────────────────────────────────────────────────────

/// **Anchor-safety proof (M-TEST.B4):** `load_or_fetch_with` with a dynamic
/// root pointed at `tempdir` and a mock fetcher MUST NOT modify the pinned
/// corpus fixture in any way.
#[tokio::test]
async fn load_or_fetch_does_not_touch_pinned_corpus() {
    // 1. Create the sentinel pinned corpus fixture.
    let corpus_tmp = tempdir().expect("corpus tempdir");
    let corpus_root = corpus_tmp.path();
    create_pinned_corpus_fixture(corpus_root);

    // 2. Snapshot the fixture BEFORE running load_or_fetch.
    let before = snapshot_dir(corpus_root);
    assert!(
        !before.is_empty(),
        "corpus fixture must contain at least one file"
    );
    let agg_sha_before =
        read_and_verify_revision_manifest(corpus_root).expect("corpus must verify before");

    // 3. Create a SEPARATE dynamic root (the real load_or_fetch_with target).
    let dynamic_tmp = tempdir().expect("dynamic tempdir");
    let dynamic_root = dynamic_tmp.path();

    // 4. Run load_or_fetch_with with a mock fetcher (no live network).
    //    Use a 2025-06-01 window (past month, not the current month) with 48 bars.
    let start_ms = 1_748_736_000_000_i64; // 2025-06-01 00:00 UTC
    let end_ms = start_ms + 48 * 3_600_000; // 48 hourly bars
    let step = 3_600_000_i64;
    let batch = make_batch(start_ms, step, 48);
    let fetcher = MockFetcher::new(vec![batch, vec![]]);
    let sym = Symbol::new("BTCUSDT");

    let bars = load_or_fetch_with(dynamic_root, &sym, start_ms, end_ms, &fetcher)
        .await
        .expect("load_or_fetch_with must succeed with a mock fetcher");
    assert!(!bars.is_empty(), "must return bars from the mock fetch");

    // (a) Assert the corpus fixture is byte-identical after the run.
    let after = snapshot_dir(corpus_root);
    for (path, (mtime_before, sha_before)) in &before {
        let (mtime_after, sha_after) = after
            .get(path)
            .unwrap_or_else(|| panic!("file disappeared from corpus: {}", path.display()));
        assert_eq!(
            sha_before,
            sha_after,
            "corpus file must be byte-identical after load_or_fetch: {}",
            path.display()
        );
        assert_eq!(
            mtime_before,
            mtime_after,
            "corpus file mtime must be unchanged: {}",
            path.display()
        );
    }
    assert_eq!(
        before.len(),
        after.len(),
        "no new files must appear in the corpus fixture"
    );

    // (b) Assert no REVISION.toml was written under the dynamic root.
    let revision_in_dynamic = dynamic_root.join("REVISION.toml");
    assert!(
        !revision_in_dynamic.exists(),
        "REVISION.toml must NOT be created under the dynamic root: {}",
        revision_in_dynamic.display()
    );

    // Also walk the dynamic root to confirm no REVISION.toml anywhere.
    let dynamic_snapshot = snapshot_dir(dynamic_root);
    for path in dynamic_snapshot.keys() {
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert_ne!(
            fname,
            "REVISION.toml",
            "REVISION.toml must not appear under dynamic root: {}",
            path.display()
        );
    }

    // (c) Assert read_and_verify_revision_manifest still returns Ok with the
    //     SAME aggregate SHA as before.
    let agg_sha_after =
        read_and_verify_revision_manifest(corpus_root).expect("corpus must verify after");
    assert_eq!(
        agg_sha_before, agg_sha_after,
        "corpus aggregate SHA must be unchanged after load_or_fetch"
    );
}
