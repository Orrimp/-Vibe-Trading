//! T09 acceptance — `ReplayFeed` 60-bar fixture test.
//!
//! Generates a deterministic 1-hour BTCUSDT 1m Parquet fixture (60 bars) in a
//! temporary directory, drives `ReplayFeed` in as-fast-as-possible mode, and
//! asserts:
//!
//! 1. Exactly 60 bars are emitted.
//! 2. Every `venue_ts` (bar open timestamp) is **strictly increasing**.
//!
//! The fixture is generated inline to keep the repository free of large binary
//! assets. If you wish to run against a real Parquet file instead, drop it at
//! `crates/data/tests/fixtures/BTCUSDT/btc_1h_60bars.parquet` and point
//! `ReplayFeed` at the parent directory.

use data::{source::MarketDataSource, ReplayFeed};
use futures::StreamExt;
use polars::prelude::*;
use std::{
    io::BufWriter,
    path::{Path, PathBuf},
};
use trading_core::{Symbol, Timeframe};

// ── Fixture generation ────────────────────────────────────────────────────────

/// Write a deterministic 60-bar BTCUSDT 1m fixture to `dir/BTCUSDT/fixture.parquet`.
///
/// Bars start at Unix epoch 1_700_000_000_000 ms (2023-11-14T22:13:20 UTC) and
/// advance by 60_000 ms (1 minute) per bar. OHLCV values are simple arithmetic
/// progressions so the fixture is deterministic and computable by hand.
fn write_fixture(dir: &Path) -> PathBuf {
    const N: usize = 60;
    // Unix millis for bar 0 open
    const START_MS: i64 = 1_700_000_000_000_i64;
    const STEP_MS: i64 = 60_000; // 1 minute
    const BASE_PRICE: f64 = 37_000.0;

    let open_times: Vec<i64> = (0..N).map(|i| START_MS + i as i64 * STEP_MS).collect();
    let close_times: Vec<i64> = open_times.iter().map(|t| t + STEP_MS - 1).collect();
    // Simple pattern: open increases by $1 per bar; OHLCV are formulaic.
    let opens: Vec<String> = (0..N)
        .map(|i| format!("{:.2}", BASE_PRICE + i as f64))
        .collect();
    let highs: Vec<String> = (0..N)
        .map(|i| format!("{:.2}", BASE_PRICE + i as f64 + 50.0))
        .collect();
    let lows: Vec<String> = (0..N)
        .map(|i| format!("{:.2}", BASE_PRICE + i as f64 - 30.0))
        .collect();
    let closes: Vec<String> = (0..N)
        .map(|i| format!("{:.2}", BASE_PRICE + i as f64 + 10.0))
        .collect();
    let volumes: Vec<String> = (0..N)
        .map(|i| format!("{:.4}", 1.0 + i as f64 * 0.01))
        .collect();
    let trade_counts: Vec<i64> = (0..N).map(|i| 100 + i as i64).collect();

    let df = DataFrame::new(vec![
        Column::new("open_time".into(), open_times.as_slice()),
        Column::new("close_time".into(), close_times.as_slice()),
        Column::new("open".into(), opens.as_slice()),
        Column::new("high".into(), highs.as_slice()),
        Column::new("low".into(), lows.as_slice()),
        Column::new("close".into(), closes.as_slice()),
        Column::new("volume".into(), volumes.as_slice()),
        Column::new("trade_count".into(), trade_counts.as_slice()),
    ])
    .expect("fixture DataFrame construction should succeed");

    let sym_dir = dir.join("BTCUSDT");
    std::fs::create_dir_all(&sym_dir).expect("create sym_dir");
    let parquet_path = sym_dir.join("fixture.parquet");

    let file = std::fs::File::create(&parquet_path).expect("create parquet file");
    let writer = BufWriter::new(file);
    ParquetWriter::new(writer)
        .finish(&mut df.clone())
        .expect("write parquet fixture");

    parquet_path
}

// ── Test ──────────────────────────────────────────────────────────────────────

/// T09: ReplayFeed in fast mode emits exactly 60 bars with strictly increasing
/// `venue_ts` values.
#[tokio::test]
async fn t09_replay_60_bars_fast_mode() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();

    write_fixture(&root);

    let feed = ReplayFeed::new(&root, /* fast = */ true);
    let symbol = Symbol::new("BTCUSDT");

    let mut stream = feed
        .subscribe_bars(symbol, Timeframe::OneMinute)
        .await
        .expect("subscribe_bars should succeed for fixture");

    let mut bars = Vec::new();
    while let Some(result) = stream.next().await {
        let bar = result.expect("bar should parse without error");
        bars.push(bar);
    }

    assert_eq!(
        bars.len(),
        60,
        "expected exactly 60 bars, got {}",
        bars.len()
    );

    // Assert strictly increasing venue_ts (open timestamp of each bar).
    for window in bars.windows(2) {
        let prev_ts = window[0].open_ts.inner();
        let next_ts = window[1].open_ts.inner();
        assert!(
            next_ts > prev_ts,
            "venue_ts must be strictly increasing: bar[n]={prev_ts} >= bar[n+1]={next_ts}"
        );
    }
}
