//! Regression guard: `ReplayFeed` resolves the canonical on-disk layout.
//!
//! The real Binance data lives at:
//!   `data/binance/<SYMBOL>/<YEAR>/<MM>.parquet`
//!
//! `ReplayFeed` expects `parquet_root` to be the *root* of that tree
//! (`data/binance`).  It internally joins `<symbol>` onto the root.
//! Setting `parquet_root = "data/binance/BTCUSDT"` (the symbol subdir)
//! causes `ReplayFeed` to look for `data/binance/BTCUSDT/BTCUSDT/…` —
//! which does not exist — resulting in `subscribe_bars` returning
//! `FeedError::Io("no parquet files found")` and a silently dead feed.
//!
//! This test exercises the exact resolution path against a locally
//! synthesised fixture that mirrors the real two-level directory layout
//! (`<root>/<SYMBOL>/<YEAR>/<MM>.parquet`), verifying that:
//!
//! 1. `subscribe_bars` succeeds when `parquet_root` is the *root*
//!    (`data/binance`-equivalent).
//! 2. `subscribe_bars` returns `FeedError::Io` (no files found) when
//!    `parquet_root` is the *symbol subdirectory* — the exact bug that
//!    caused the Live dashboard panels to hang on "Connecting…".
//!
//! The test is hermetic (temp-dir fixture, no network, fast mode).
//! It is CI-safe and must stay green.

use data::{ReplayFeed, source::MarketDataSource};
use futures::StreamExt;
use polars::prelude::*;
use std::{io::BufWriter, path::Path};
use trading_core::{FeedError, Symbol, Timeframe};

// ── Fixture generation ────────────────────────────────────────────────────────

/// Write a minimal 5-bar BTCUSDT fixture to
/// `<root>/BTCUSDT/2023/01.parquet` — mirroring the on-disk layout.
fn write_two_level_fixture(root: &Path) {
    const N: usize = 5;
    const START_MS: i64 = 1_672_531_200_000_i64; // 2023-01-01T00:00:00 UTC
    const STEP_MS: i64 = 60_000;

    let open_times: Vec<i64> = (0..N as i64).map(|i| START_MS + i * STEP_MS).collect();
    let close_times: Vec<i64> = open_times.iter().map(|t| t + STEP_MS - 1).collect();
    let prices: Vec<String> = (0..N)
        .map(|i| format!("{:.2}", 16_500.0 + i as f64))
        .collect();
    let volumes: Vec<String> = (0..N).map(|_| "1.00".to_string()).collect();
    let tc: Vec<i64> = (0..N as i64).map(|i| 100 + i).collect();

    let df = DataFrame::new(vec![
        Column::new("open_time".into(), open_times.as_slice()),
        Column::new("close_time".into(), close_times.as_slice()),
        Column::new("open".into(), prices.as_slice()),
        Column::new("high".into(), prices.as_slice()),
        Column::new("low".into(), prices.as_slice()),
        Column::new("close".into(), prices.as_slice()),
        Column::new("volume".into(), volumes.as_slice()),
        Column::new("trade_count".into(), tc.as_slice()),
    ])
    .expect("fixture DataFrame");

    // Mirror the real layout: <root>/BTCUSDT/2023/01.parquet
    let year_dir = root.join("BTCUSDT").join("2023");
    std::fs::create_dir_all(&year_dir).expect("create year dir");
    let parquet_path = year_dir.join("01.parquet");
    let file = std::fs::File::create(&parquet_path).expect("create parquet");
    ParquetWriter::new(BufWriter::new(file))
        .finish(&mut df.clone())
        .expect("write parquet");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// T_REPLAY_LAYOUT_1 — `subscribe_bars` SUCCEEDS when `parquet_root` is
/// the **root** (`data/binance`-equivalent), i.e. the parent of the symbol
/// directory.
#[tokio::test]
async fn replay_feed_succeeds_when_parquet_root_is_tree_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_two_level_fixture(tmp.path());

    // Correct: parquet_root = tmp/<root>  →  resolved path = tmp/<root>/BTCUSDT/2023/01.parquet
    let feed = ReplayFeed::new(tmp.path(), /* fast = */ true);
    let symbol = Symbol::new("BTCUSDT");

    let result = feed.subscribe_bars(symbol, Timeframe::OneMinute).await;
    assert!(
        result.is_ok(),
        "subscribe_bars must succeed when parquet_root is the tree root; got: {:?}",
        result.err()
    );

    let mut stream = result.unwrap();
    let mut count = 0usize;
    while let Some(r) = stream.next().await {
        r.expect("bar should parse without error");
        count += 1;
    }
    assert_eq!(count, 5, "expected 5 bars from the two-level fixture");
}

/// T_REPLAY_LAYOUT_2 — `subscribe_bars` returns `FeedError::Io` when
/// `parquet_root` is the **symbol subdirectory** (the bug that caused the
/// Live dashboard to hang).
///
/// With `parquet_root = "<root>/BTCUSDT"` and `symbol = "BTCUSDT"`,
/// `ReplayFeed` looks for `<root>/BTCUSDT/BTCUSDT/…` — which does not exist.
/// The feed must return an error, NOT silently succeed with zero bars.
#[tokio::test]
async fn replay_feed_fails_when_parquet_root_is_symbol_subdir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_two_level_fixture(tmp.path());

    // BUG scenario: parquet_root = tmp/<root>/BTCUSDT  →  resolved = tmp/<root>/BTCUSDT/BTCUSDT/
    let symbol_subdir = tmp.path().join("BTCUSDT");
    let feed = ReplayFeed::new(&symbol_subdir, /* fast = */ true);
    let symbol = Symbol::new("BTCUSDT");

    let result = feed.subscribe_bars(symbol, Timeframe::OneMinute).await;
    assert!(
        result.is_err(),
        "subscribe_bars must return Err when parquet_root is the symbol subdir (not the tree root); \
         a successful return here would mean zero bars and a silently dead feed"
    );
    // Confirm it's specifically the Io/no-files error (not a schema error etc.)
    // Note: we cannot use `unwrap_err()` because the `Ok` variant (BoxStream) is not Debug.
    let err = result.err().expect("confirmed is_err above");
    match err {
        FeedError::Io(msg) => {
            assert!(
                msg.contains("no parquet files found"),
                "Io error message should state 'no parquet files found'; got: {msg}"
            );
        }
        other => panic!("expected FeedError::Io, got {other:?}"),
    }
}
