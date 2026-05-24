//! Integration tests for `YahooBarSource::load_cached` (T-C1.6 / T-AR6).
//!
//! All tests run against fixture caches constructed in a `tempdir` — no
//! network required, no external parquet files required at test time.
//!
//! # Test cases
//!
//! 1. `happy_path` — load bars from a fixture cache; SHA stable across 2 calls.
//! 2. `tamper_detects_revision_mismatch` — flip a byte → `RevisionMismatch`.
//! 3. `cache_miss_returns_actionable_error` — ask for a missing month → `CacheMiss`.
//! 4. `coverage_94_pct_returns_missing_data` — < 95% coverage → `MissingData`.
//! 5. `revision_missing_returns_error` — no `REVISION.toml` → `RevisionMissing`.

#![cfg(feature = "yahoo")]

use std::path::Path;

use polars::prelude::*;
use tempfile::TempDir;

use data::revision::write_revision_manifest;
use data::yahoo::{Interval, YahooBarSource, YahooError};

// ── Fixture helpers ───────────────────────────────────────────────────────────

/// One OHLCV row for fixture purposes.
struct FakeBar {
    open_time_ms: i64,
    close_time_ms: i64,
    open: &'static str,
    high: &'static str,
    low: &'static str,
    close: &'static str,
    volume: &'static str,
}

/// Write a parquet file at `path` using the Yahoo/Replay schema.
fn write_fixture_parquet(path: &Path, bars: &[FakeBar]) {
    let n = bars.len();
    let open_times: Vec<i64> = bars.iter().map(|b| b.open_time_ms).collect();
    let close_times: Vec<i64> = bars.iter().map(|b| b.close_time_ms).collect();
    let opens: Vec<&str> = bars.iter().map(|b| b.open).collect();
    let highs: Vec<&str> = bars.iter().map(|b| b.high).collect();
    let lows: Vec<&str> = bars.iter().map(|b| b.low).collect();
    let closes: Vec<&str> = bars.iter().map(|b| b.close).collect();
    let volumes: Vec<&str> = bars.iter().map(|b| b.volume).collect();
    let trade_counts: Vec<i64> = vec![0i64; n];

    let mut df = DataFrame::new(vec![
        Column::new("open_time".into(), &open_times),
        Column::new("close_time".into(), &close_times),
        Column::new("open".into(), &opens),
        Column::new("high".into(), &highs),
        Column::new("low".into(), &lows),
        Column::new("close".into(), &closes),
        Column::new("volume".into(), &volumes),
        Column::new("trade_count".into(), &trade_counts),
    ])
    .expect("construct fixture DataFrame");

    std::fs::create_dir_all(path.parent().expect("parent dir")).expect("create parent dirs");
    let mut file = std::fs::File::create(path).expect("create parquet file");
    ParquetWriter::new(&mut file)
        .finish(&mut df)
        .expect("write parquet");
}

/// Build 31 synthetic daily bars for January 2024.
///
/// open_time: 2024-01-{01..31}T00:00:00Z  (1704067200000 ms + day*86_400_000)
/// Each bar is 86_400_000 ms wide.
fn jan_2024_daily_bars() -> Vec<FakeBar> {
    const MS_PER_DAY: i64 = 86_400_000;
    const JAN_1_2024_MS: i64 = 1_704_067_200_000;
    (0i64..31)
        .map(|day| FakeBar {
            open_time_ms: JAN_1_2024_MS + day * MS_PER_DAY,
            close_time_ms: JAN_1_2024_MS + (day + 1) * MS_PER_DAY - 1,
            open: "42000.00",
            high: "43000.00",
            low: "41000.00",
            close: "42500.00",
            volume: "100.0",
        })
        .collect()
}

/// Populate a `TempDir` with `BTC-USD/1d/2024/01.parquet` + `REVISION.toml`.
///
/// Returns the `TempDir` (must stay alive for the test) and the parquet path.
fn setup_fixture_cache() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let parquet_path = root.join("BTC-USD/1d/2024/01.parquet");
    let bars = jan_2024_daily_bars();
    write_fixture_parquet(&parquet_path, &bars);
    write_revision_manifest(root).expect("write_revision_manifest");
    (tmp, parquet_path)
}

/// Extract `Err(e)` from a `Result<_, YahooError>`, panicking on `Ok`.
///
/// `load_cached` returns `Result<LoadedBars, YahooError>`, and `LoadedBars`
/// does not implement `Debug` (it contains `Vec<Bar>` which does but the
/// outer struct is not derived), so we cannot use `.unwrap_err()`.
fn expect_err(result: Result<data::yahoo::LoadedBars, YahooError>, context: &str) -> YahooError {
    match result {
        Ok(_) => panic!("{context}: expected Err(_), got Ok(_)"),
        Err(e) => e,
    }
}

// ── Test 1: happy path ────────────────────────────────────────────────────────

/// Load bars from a fixture cache; assert SHA is stable across 2 calls.
#[test]
fn happy_path() {
    let (tmp, _parquet) = setup_fixture_cache();
    let root = tmp.path().to_owned();
    let src = YahooBarSource::new(root);

    const MS_PER_DAY: i64 = 86_400_000;
    const JAN_1_2024_MS: i64 = 1_704_067_200_000;
    let start_ms = JAN_1_2024_MS;
    let end_ms = JAN_1_2024_MS + 31 * MS_PER_DAY;

    let loaded1 = src
        .load_cached("BTC-USD", Interval::Days1, start_ms, end_ms)
        .expect("first load_cached should succeed");

    assert_eq!(
        loaded1.loaded_count, 31,
        "expected 31 bars for January 2024"
    );
    assert_eq!(loaded1.interval, Interval::Days1);
    assert!(
        !loaded1.revision_sha.is_empty(),
        "revision_sha must be non-empty"
    );

    // SHA must be stable across 2 calls to the same cache (determinism gate H4).
    let loaded2 = src
        .load_cached("BTC-USD", Interval::Days1, start_ms, end_ms)
        .expect("second load_cached should succeed");

    assert_eq!(
        loaded1.revision_sha, loaded2.revision_sha,
        "revision_sha must be identical across two calls"
    );
    assert_eq!(loaded2.loaded_count, 31);

    // All bars must have local_recv_ts == close_ts (ADR-0032 § D1 Step 7).
    for bar in &loaded1.bars {
        assert_eq!(
            bar.local_recv_ts, bar.close_ts,
            "local_recv_ts must equal close_ts"
        );
    }

    // Bars must be sorted ascending by open_ts.
    let sorted = loaded1
        .bars
        .windows(2)
        .all(|w| w[0].open_ts <= w[1].open_ts);
    assert!(sorted, "bars must be sorted ascending by open_ts");
}

// ── Test 2: tamper detection ──────────────────────────────────────────────────

/// Flip a byte in the fixture parquet → must get `RevisionMismatch`.
#[test]
fn tamper_detects_revision_mismatch() {
    let (tmp, parquet_path) = setup_fixture_cache();
    let root = tmp.path().to_owned();

    let mut bytes = std::fs::read(&parquet_path).expect("read parquet");
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    std::fs::write(&parquet_path, &bytes).expect("write tampered parquet");

    let src = YahooBarSource::new(root);
    const MS_PER_DAY: i64 = 86_400_000;
    const JAN_1_2024_MS: i64 = 1_704_067_200_000;
    let start_ms = JAN_1_2024_MS;
    let end_ms = JAN_1_2024_MS + 31 * MS_PER_DAY;

    let err = expect_err(
        src.load_cached("BTC-USD", Interval::Days1, start_ms, end_ms),
        "tamper_detects_revision_mismatch",
    );
    assert!(
        matches!(err, YahooError::RevisionMismatch { .. }),
        "expected RevisionMismatch after tamper, got: {err}"
    );
}

// ── Test 3: cache miss ────────────────────────────────────────────────────────

/// Ask for February 2024 (not in the fixture cache) → `CacheMiss` with CLI hint.
#[test]
fn cache_miss_returns_actionable_error() {
    let (tmp, _parquet) = setup_fixture_cache();
    let root = tmp.path().to_owned();
    let src = YahooBarSource::new(root);

    // 2024-02-01 00:00:00 UTC = 1706745600000 ms
    const MS_PER_DAY: i64 = 86_400_000;
    const FEB_1_2024_MS: i64 = 1_706_745_600_000;
    let start_ms = FEB_1_2024_MS;
    let end_ms = FEB_1_2024_MS + 29 * MS_PER_DAY;

    let err = expect_err(
        src.load_cached("BTC-USD", Interval::Days1, start_ms, end_ms),
        "cache_miss_returns_actionable_error",
    );
    assert!(
        matches!(err, YahooError::CacheMiss { .. }),
        "expected CacheMiss for February 2024, got: {err}"
    );

    let msg = err.to_string();
    assert!(
        msg.contains("fetch_yahoo_klines"),
        "CacheMiss error must contain CLI hint 'fetch_yahoo_klines', got: {msg}"
    );
}

// ── Test 4: insufficient coverage ────────────────────────────────────────────

/// Write only 27 bars for a 30-day window → 90% coverage < 95% → `MissingData`.
#[test]
fn coverage_94_pct_returns_missing_data() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();

    const MS_PER_DAY: i64 = 86_400_000;
    const JAN_1_2024_MS: i64 = 1_704_067_200_000;

    // 27 bars only (days 0..26 inclusive).
    let sparse_bars: Vec<FakeBar> = (0i64..27)
        .map(|day| FakeBar {
            open_time_ms: JAN_1_2024_MS + day * MS_PER_DAY,
            close_time_ms: JAN_1_2024_MS + (day + 1) * MS_PER_DAY - 1,
            open: "42000.00",
            high: "43000.00",
            low: "41000.00",
            close: "42500.00",
            volume: "100.0",
        })
        .collect();

    let parquet_path = root.join("BTC-USD/1d/2024/01.parquet");
    write_fixture_parquet(&parquet_path, &sparse_bars);
    write_revision_manifest(root).expect("write_revision_manifest");

    let src = YahooBarSource::new(root.to_owned());

    // 30-day window: expected = 30 bars, actual = 27 → 90.0% < 95%.
    let end_ms = JAN_1_2024_MS + 30 * MS_PER_DAY;
    let err = expect_err(
        src.load_cached("BTC-USD", Interval::Days1, JAN_1_2024_MS, end_ms),
        "coverage_94_pct_returns_missing_data",
    );
    assert!(
        matches!(err, YahooError::MissingData { .. }),
        "expected MissingData for 90% coverage, got: {err}"
    );

    let msg = err.to_string();
    assert!(
        msg.contains("BTC-USD"),
        "MissingData error must contain ticker, got: {msg}"
    );
    assert!(
        msg.contains("< 95%"),
        "MissingData error must mention the 95% threshold, got: {msg}"
    );
}

// ── Test 5: revision missing ──────────────────────────────────────────────────

/// No `REVISION.toml` → `RevisionMissing`.
#[test]
fn revision_missing_returns_error() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_owned();

    // Write a parquet but NO REVISION.toml.
    let parquet_path = root.join("BTC-USD/1d/2024/01.parquet");
    let bars = jan_2024_daily_bars();
    write_fixture_parquet(&parquet_path, &bars);

    let src = YahooBarSource::new(root);
    const MS_PER_DAY: i64 = 86_400_000;
    const JAN_1_2024_MS: i64 = 1_704_067_200_000;
    let start_ms = JAN_1_2024_MS;
    let end_ms = JAN_1_2024_MS + 31 * MS_PER_DAY;

    let err = expect_err(
        src.load_cached("BTC-USD", Interval::Days1, start_ms, end_ms),
        "revision_missing_returns_error",
    );
    assert!(
        matches!(err, YahooError::RevisionMissing { .. }),
        "expected RevisionMissing when no REVISION.toml, got: {err}"
    );
}

// ── Fixture generator (run once to seed checked-in fixtures) ─────────────────

/// Generate the checked-in fixture parquet at
/// `crates/data/tests/fixtures/yahoo/BTC-USD/1d/2024/01.parquet`.
///
/// Run with:
///   cargo test -p data --features yahoo --test yahoo_revision_verify \
///     generate_checked_in_fixture -- --ignored
///
/// Only needs to run once; the output is committed to git.
#[test]
#[ignore = "fixture generator — run once to seed tests/fixtures/yahoo/"]
fn generate_checked_in_fixture() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join("tests/fixtures/yahoo");

    let parquet_path = fixture_root.join("BTC-USD/1d/2024/01.parquet");
    let bars = jan_2024_daily_bars();
    write_fixture_parquet(&parquet_path, &bars);
    write_revision_manifest(&fixture_root).expect("write_revision_manifest for checked-in fixture");

    println!("Generated fixture at {}", parquet_path.display());
    println!(
        "REVISION.toml at {}",
        fixture_root.join("REVISION.toml").display()
    );
}
