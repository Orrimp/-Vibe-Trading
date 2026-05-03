#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T812 — `MarkSource` trait + `ParquetMarkSource` + `FrozenMarkSource`
//! integration tests.
//!
//! Acceptance criteria from the task list:
//!
//! - (a) `ParquetMarkSource::close_at` returns the expected close at a
//!   known ts within the parquet fixtures (we build a tiny one ourselves
//!   in a tempdir to avoid pinning the test to non-deterministic
//!   `data/binance/` data).
//! - (b) `FrozenMarkSource::close_at` round-trips against the
//!   checked-in `tests/fixtures/snapshot_marks.csv`.
//! - (c) `close_series(BTCUSDT, t0, t1, 1)` returns `(t1 - t0) / 60`
//!   rows.
//! - (d) Out-of-range requests on either implementation return
//!   `MarkError::OutOfRange`.

use polars::prelude::*;
use reports::{FrozenMarkSource, MarkError, MarkSource, ParquetMarkSource};
use rust_decimal::Decimal;
use trading_core::{Symbol, Timestamp};

fn ms_to_ts(ms: i64) -> Timestamp {
    let nanos = i128::from(ms) * 1_000_000;
    Timestamp::new(time::OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap())
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("snapshot_marks.csv")
}

#[test]
fn t812_frozen_close_at_round_trips_from_csv_fixture() {
    let src = FrozenMarkSource::from_csv(fixture_path()).unwrap();
    let btc = Symbol::new("BTCUSDT");
    // CSV row: BTCUSDT,1714521600000,68000.00
    let ts = ms_to_ts(1714521600000);
    let v = src.close_at(&btc, ts).unwrap();
    assert_eq!(v, "68000.00".parse::<Decimal>().unwrap());

    // Bar between two rows — pulls the latest one ≤ ts.
    let mid_ts = ms_to_ts(1714521630000);
    let mid = src.close_at(&btc, mid_ts).unwrap();
    assert_eq!(mid, "68000.00".parse::<Decimal>().unwrap());
}

#[test]
fn t812_frozen_close_at_returns_out_of_range_below_first_bar() {
    let src = FrozenMarkSource::from_csv(fixture_path()).unwrap();
    let btc = Symbol::new("BTCUSDT");
    let early = ms_to_ts(0);
    let err = src.close_at(&btc, early).unwrap_err();
    assert!(matches!(err, MarkError::OutOfRange { .. }));
}

#[test]
fn t812_frozen_close_series_btc_1m_cadence_row_count() {
    // 60-second-cadence series over 14 minutes (15 close rows in the
    // fixture).  At 1m cadence we expect 15 rows back.
    let src = FrozenMarkSource::from_csv(fixture_path()).unwrap();
    let btc = Symbol::new("BTCUSDT");
    let from = ms_to_ts(1714521600000);
    let to = ms_to_ts(1714521600000 + 14 * 60_000);
    let series = src.close_series(&btc, from, to, 1).unwrap();
    assert_eq!(series.len(), 15);
    // First row carries the first close; last row carries the last
    // close of the fixture window.
    assert_eq!(series[0].1, "68000.00".parse::<Decimal>().unwrap());
    assert_eq!(series[14].1, "68058.00".parse::<Decimal>().unwrap());
}

#[test]
fn t812_frozen_close_series_4_symbol_universe_round_trip() {
    let src = FrozenMarkSource::from_csv(fixture_path()).unwrap();
    let from = ms_to_ts(1714521600000);
    let to = ms_to_ts(1714521600000 + 5 * 60_000);
    for sym_str in ["BTCUSDT", "ETHUSDT", "SOLUSDT", "XRPUSDT"] {
        let sym = Symbol::new(sym_str);
        let series = src.close_series(&sym, from, to, 1).unwrap();
        assert_eq!(series.len(), 6, "symbol {sym_str}");
    }
}

#[test]
fn t812_parquet_close_at_returns_expected_close_via_tempdir_fixture() {
    // Build a tiny ad-hoc parquet so the test is independent of the
    // operator's data/binance/ tree.  Schema mirrors the production
    // schema (`close_time`, `close` Utf8).
    let dir = tempfile::tempdir().unwrap();
    let sym_dir = dir.path().join("BTCUSDT").join("2024");
    std::fs::create_dir_all(&sym_dir).unwrap();
    let parquet_path = sym_dir.join("part.parquet");

    let mut df = df!(
        "open_time" => &[1_000_000_i64, 2_000_000_i64, 3_000_000_i64],
        "close_time" => &[1_059_999_i64, 2_059_999_i64, 3_059_999_i64],
        "open" => &["68000.00", "68050.00", "68100.00"],
        "high" => &["68100.00", "68150.00", "68200.00"],
        "low" => &["67900.00", "67950.00", "68000.00"],
        "close" => &["68040.00", "68080.00", "68120.00"],
        "volume" => &["10.0", "11.0", "12.0"],
        "trade_count" => &[100_i64, 110_i64, 120_i64],
    )
    .unwrap();

    let mut f = std::fs::File::create(&parquet_path).unwrap();
    ParquetWriter::new(&mut f).finish(&mut df).unwrap();

    let src = ParquetMarkSource::new(dir.path());

    // Exact bar close — close_at returns the latest bar with
    // close_time ≤ ts.  At ts=2_500_000 the latest bar is the one
    // closing at 2_059_999 with close=68080.00.
    let v = src
        .close_at(&Symbol::new("BTCUSDT"), ms_to_ts(2_500_000))
        .unwrap();
    assert_eq!(v, "68080.00".parse::<Decimal>().unwrap());
}

#[test]
fn t812_parquet_close_at_out_of_range_below_first_bar() {
    // Same tempdir setup; query a ts before the first close.
    let dir = tempfile::tempdir().unwrap();
    let sym_dir = dir.path().join("BTCUSDT").join("2024");
    std::fs::create_dir_all(&sym_dir).unwrap();
    let parquet_path = sym_dir.join("part.parquet");

    let mut df = df!(
        "open_time" => &[10_000_000_i64],
        "close_time" => &[10_059_999_i64],
        "open" => &["1.0"],
        "high" => &["1.1"],
        "low" => &["0.9"],
        "close" => &["1.05"],
        "volume" => &["1.0"],
        "trade_count" => &[1_i64],
    )
    .unwrap();

    let mut f = std::fs::File::create(&parquet_path).unwrap();
    ParquetWriter::new(&mut f).finish(&mut df).unwrap();

    let src = ParquetMarkSource::new(dir.path());
    let err = src
        .close_at(&Symbol::new("BTCUSDT"), ms_to_ts(0))
        .unwrap_err();
    assert!(matches!(err, MarkError::OutOfRange { .. }));
}

#[test]
fn t812_parquet_close_series_returns_one_row_per_cadence() {
    // 1m cadence over [from, to] returns ceil((to-from)/cadence)+1
    // rows when both endpoints are within range.
    let dir = tempfile::tempdir().unwrap();
    let sym_dir = dir.path().join("BTCUSDT").join("2024");
    std::fs::create_dir_all(&sym_dir).unwrap();
    let parquet_path = sym_dir.join("part.parquet");

    let mut df = df!(
        "open_time" => &[0_i64, 60_000_i64, 120_000_i64, 180_000_i64, 240_000_i64],
        "close_time" => &[59_999_i64, 119_999_i64, 179_999_i64, 239_999_i64, 299_999_i64],
        "open" => &["1.0", "1.0", "1.0", "1.0", "1.0"],
        "high" => &["1.1", "1.1", "1.1", "1.1", "1.1"],
        "low" => &["0.9", "0.9", "0.9", "0.9", "0.9"],
        "close" => &["1.0", "2.0", "3.0", "4.0", "5.0"],
        "volume" => &["1.0", "1.0", "1.0", "1.0", "1.0"],
        "trade_count" => &[1_i64, 1_i64, 1_i64, 1_i64, 1_i64],
    )
    .unwrap();

    let mut f = std::fs::File::create(&parquet_path).unwrap();
    ParquetWriter::new(&mut f).finish(&mut df).unwrap();

    let src = ParquetMarkSource::new(dir.path());
    let from = ms_to_ts(60_000);
    let to = ms_to_ts(240_000);
    let series = src
        .close_series(&Symbol::new("BTCUSDT"), from, to, 1)
        .unwrap();
    // Cadence 60_000ms over a 180_000ms window → 4 cells (at 60k,
    // 120k, 180k, 240k).
    assert_eq!(series.len(), 4);
}
