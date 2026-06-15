//! Tests for the `data/binance-2122/` corpus (2021-22 hourly, 10 symbols).
//!
//! # Test T6: always-on manifest-internal-consistency check
//!
//! Re-derives the aggregate SHA from the `[files]` map in the committed
//! `data/binance-2122/REVISION.toml` and asserts it equals the claimed
//! `[revision].sha256`. Runs with **no parquet on disk** — the TOML alone is
//! sufficient (same pattern as the `compute_aggregate_sha` unit tests in
//! `revision.rs`). CI-safe.
//!
//! # Test T7: `#[ignore]` smoke consumer
//!
//! Proves `ReplayFeed` reads `data/binance-2122` via the `OneHour` timeframe
//! for BTCUSDT/2022 and that prices parse to `rust_decimal::Decimal` (AC5 + AC6).
//! SKIP-guards when the corpus parquet files are absent so CI is unaffected.
//!
//! Run individually:
//! ```text
//! # T6 (always-on):
//! cargo test -p data --test binance_2122_revision_consistency manifest_internal_consistency
//!
//! # T7 (requires data/binance-2122/ on disk):
//! cargo test -p data --test binance_2122_revision_consistency smoke_consumer_btcusdt_2022 -- --ignored
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

fn binance_2122_root() -> PathBuf {
    workspace_root().join("data/binance-2122")
}

// ── T6: always-on manifest-internal-consistency check ────────────────────────

/// Parse `data/binance-2122/REVISION.toml`, recompute the aggregate SHA from
/// the `[files]` map, and assert it equals the manifest's claimed
/// `[revision].sha256`.
///
/// This test runs on the committed manifest alone — no parquet files required.
/// It mirrors the determinism guarantee in ADR-0032 § 2: the same file-set
/// always hashes to the same aggregate (content-only; `[revision.metadata]` is
/// excluded from the hash).
#[test]
fn manifest_internal_consistency() {
    let root = binance_2122_root();
    let manifest_path = root.join("REVISION.toml");

    if !manifest_path.exists() {
        // REVISION.toml is committed together with this test.  If it is missing
        // we are in a checkout that predates the pin — fail with a clear message
        // rather than silently passing.
        panic!(
            "data/binance-2122/REVISION.toml not found at {}.  \
             This file must be committed to git (only the bulk *.parquet files \
             are gitignored).  Run the corpus fetch first:\n\
             cargo run -p data --bin fetch_binance_klines -- \\\n  \
               --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \\\n  \
               --start 2021-01-01 --end 2022-12-31 --interval 1h \\\n  \
               --out data/binance-2122 --emit-revision-manifest",
            manifest_path.display()
        );
    }

    let (files_map, claimed_sha) =
        read_manifest_raw(&root).expect("read_manifest_raw should parse REVISION.toml");

    // The manifest must cover exactly 240 files (10 symbols × 24 months).
    // A count mismatch indicates a partial fetch or an accidental extension.
    assert_eq!(
        files_map.len(),
        240,
        "expected 240 parquet entries in REVISION.toml (10 symbols × 24 months), got {}",
        files_map.len()
    );

    let recomputed_sha = compute_aggregate_sha(&files_map);
    assert_eq!(
        recomputed_sha, claimed_sha,
        "aggregate SHA mismatch: recomputed ({recomputed_sha}) != claimed ({claimed_sha}).  \
         The REVISION.toml may have been hand-edited or the [files] map is inconsistent."
    );
}

// ── T7: #[ignore] smoke consumer ─────────────────────────────────────────────

/// Smoke-test that `ReplayFeed` can read `data/binance-2122` and that OHLC
/// prices parse to `rust_decimal::Decimal` (AC5 / AC6).
///
/// SKIP guard: if `data/binance-2122/BTCUSDT/2022/01.parquet` is absent the
/// test prints a SKIP message and returns — mirrors the guard in
/// `realdata_simple_strategy_survey.rs:112`.
///
/// When the corpus IS present: loads all bars for BTCUSDT from the 2122 root,
/// asserts at least one bar is returned, and asserts that close price parses
/// to a non-zero `Decimal` (the read path is `Utf8 → parse::<Decimal>()`,
/// never f64).
///
/// Run with:
///   cargo test -p data --test binance_2122_revision_consistency \
///     smoke_consumer_btcusdt_2022 -- --ignored
#[tokio::test]
#[ignore = "requires data/binance-2122/ on disk — run with --ignored after corpus fetch"]
async fn smoke_consumer_btcusdt_2022() {
    use data::source::MarketDataSource as _;
    use tokio_stream::StreamExt as _;
    use trading_core::{Symbol, Timeframe};

    let root = binance_2122_root();
    let sentinel = root.join("BTCUSDT/2022/01.parquet");
    if !sentinel.is_file() {
        eprintln!(
            "SKIP binance-2122 smoke: corpus absent ({})",
            sentinel.display()
        );
        return;
    }

    let feed = data::ReplayFeed::new(root.clone(), true);
    let sym = Symbol::new("BTCUSDT");

    let mut stream = feed
        .subscribe_bars(sym.clone(), Timeframe::OneHour)
        .await
        .expect("subscribe_bars should succeed when corpus is present");

    let mut bar_count = 0u64;
    while let Some(result) = stream.next().await {
        let bar = result.expect("bar should parse without error");

        // AC6: prices are Decimal (the read path parses Utf8 → Decimal, never f64).
        // We verify close is non-zero — all real Binance bars have a positive price.
        let close_dec = bar.close.get();
        assert!(
            !close_dec.is_zero(),
            "close price must be non-zero for a real BTCUSDT bar"
        );

        bar_count += 1;
    }

    // 2021-22 BTCUSDT: 24 months × ~720 bars/month (30 d × 24 h) = ~17 280 bars.
    // Accept ≥ 100 as the SKIP-vs-real guard; the corpus should deliver ~17 000+.
    assert!(
        bar_count >= 100,
        "expected ≥ 100 BTCUSDT bars from data/binance-2122, got {bar_count}"
    );

    eprintln!("OK binance-2122 smoke: BTCUSDT read {bar_count} bars from {root:?}");
}
