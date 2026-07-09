//! T7 — REVISION.toml internal-consistency tests for the 4 P2 corpora
//! (ADR-0084 D1 + D2.b; `spec/v3/advisor-corpus-expansion/tasks.md` T7).
//!
//! Mirrors `binance_2122_revision_consistency.rs::manifest_internal_consistency`
//! for each of the 4 new pinned corpora: `data/binance-1718`, `data/binance-2020`,
//! `data/binance-2526`, `data/coinbase`. Each check:
//!
//! 1. Parses the committed `REVISION.toml` (manifest-only — no parquet on disk
//!    required, CI-safe).
//! 2. Asserts the `[files]` map has the expected entry count (symbols × months
//!    per the honest per-corpus subset in feature.md § R1 / ADR-0084 D1).
//! 3. Re-derives the aggregate SHA-256 from the `[files]` map and asserts it
//!    equals the manifest's claimed `[revision].sha256` — the same ADR-0032 § 2
//!    algorithm `binance_2122_revision_consistency.rs` exercises.
//!
//! Un-ignored (fast, TOML-parse-only) — these run in every `cargo test -p data`
//! invocation, including on machines without the (gitignored, multi-GB) parquet
//! corpora on disk.
//!
//! Run individually:
//! ```text
//! cargo test -p data --test p2_corpora_revision_consistency
//! ```

use std::path::PathBuf;

use data::revision::{compute_aggregate_sha, read_manifest_raw};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Absolute path to the workspace root.
///
/// `CARGO_MANIFEST_DIR` for this crate is `<workspace>/crates/data`; two
/// `parent()` calls reach the workspace root.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Assert `<workspace>/<corpus_dir>/REVISION.toml` is internally consistent:
/// the claimed aggregate SHA matches the SHA re-derived from the `[files]`
/// map, and the entry count matches `expected_file_count`.
///
/// Fails loudly (not silently) if the manifest is missing — a missing
/// `REVISION.toml` for a corpus this test targets means the corpus fetch
/// (T4) has not landed on this checkout, which is a setup error for this
/// always-on test, distinct from the T8 smoke tests' SKIP-on-absent-parquet
/// contract (REVISION.toml itself is committed to git, unlike the bulk
/// `*.parquet` files).
fn assert_manifest_internally_consistent(corpus_dir: &str, expected_file_count: usize) {
    let root = workspace_root().join(corpus_dir);
    let manifest_path = root.join("REVISION.toml");

    if !manifest_path.exists() {
        panic!(
            "{corpus_dir}/REVISION.toml not found at {}. This file must be committed to git \
             (only the bulk *.parquet files are gitignored). Run the P2 corpus fetch first — \
             see spec/v3/advisor-corpus-expansion/tasks.md T4 for the exact command.",
            manifest_path.display()
        );
    }

    let (files_map, claimed_sha) = read_manifest_raw(&root).unwrap_or_else(|e| {
        panic!("read_manifest_raw({corpus_dir}) should parse REVISION.toml: {e}")
    });

    assert_eq!(
        files_map.len(),
        expected_file_count,
        "{corpus_dir}: expected {expected_file_count} parquet entries in REVISION.toml, got {}",
        files_map.len()
    );

    let recomputed_sha = compute_aggregate_sha(&files_map);
    assert_eq!(
        recomputed_sha, claimed_sha,
        "{corpus_dir}: aggregate SHA mismatch: recomputed ({recomputed_sha}) != claimed \
         ({claimed_sha}). The REVISION.toml may have been hand-edited or the [files] map is \
         inconsistent."
    );
}

// ── T7.1 — data/binance-1718 (2017-08 → 2018-12, BTC/ETH/BNB) ─────────────────

/// 3 symbols, uneven month coverage per the honest listing-date subset
/// (ADR-0084 D1): BTCUSDT/ETHUSDT 2017-08→2018-12 (17 months each), BNBUSDT
/// 2017-11→2018-12 (14 months, listed 2017-11) — total 48, matching the
/// on-disk `find … -name '*.parquet' | wc -l` count (verified 2026-07-10).
#[test]
fn binance_1718_manifest_internal_consistency() {
    assert_manifest_internally_consistent("data/binance-1718", 48);
}

// ── T7.2 — data/binance-2020 (2020-01 → 2020-12, the 7 pre-2020 listers) ──────

/// 7 symbols × 12 months (full-year 2020 coverage for BTC/ETH/BNB/XRP/ADA/
/// LINK/DOGE — the 7 pre-2020 listers; DOT/SOL/AVAX excluded per ADR-0084 D1
/// as they listed mid/late-2020 and would be ragged) = 84.
#[test]
fn binance_2020_manifest_internal_consistency() {
    assert_manifest_internally_consistent("data/binance-2020", 84);
}

// ── T7.3 — data/binance-2526 (2025-01 → last-closed-UTC-month, all 10) ───────

/// 10 symbols × 18 months (2025-01 through 2026-06, the D5 last-fully-closed-
/// UTC-month clamp applied at fetch time 2026-07-09/10) = 180.
#[test]
fn binance_2526_manifest_internal_consistency() {
    assert_manifest_internally_consistent("data/binance-2526", 180);
}

// ── T7.4 — data/coinbase (2020-01 → last-closed-UTC-month, BTC only) ─────────

/// 1 symbol (BTCUSDT, on-disk canonical per ADR-0084 D2.a normalization) × 78
/// months (2020-01 through 2026-06 inclusive = 6 full years × 12 + 6 = 78).
#[test]
fn coinbase_manifest_internal_consistency() {
    assert_manifest_internally_consistent("data/coinbase", 78);
}
