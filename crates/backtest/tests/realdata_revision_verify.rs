//! T-D-5: Integration tests for `RealDataBarSource` revision verification.
//!
//! Tests 4 paths:
//! (a) Happy load returns expected aggregate SHA.
//! (b) Tampering one byte of one parquet → `RevisionMismatch`.
//! (c) Deleting the manifest → `RevisionMissing`.
//! (d) Injecting a 0.6% gap (delete N bars from one symbol) → `MissingData`.
//!
//! The fixture uses synthetic parquet files that have the same schema as real
//! Binance parquet (produced by `fetch_binance_klines`).

#![cfg(feature = "realdata")]
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use polars::prelude::*;
use tempfile::TempDir;
use trading_core::Symbol;

// ── Fixture builder ───────────────────────────────────────────────────────────

/// One hour in milliseconds.
const ONE_HOUR_MS: i64 = 3_600_000;

/// Write a synthetic hourly parquet file matching the Binance schema
/// (`open_time`, `close_time`, `open`, `high`, `low`, `close`, `volume`, `trade_count`).
///
/// `start_ms` is the Unix-millisecond open_time of the first bar.
/// `bar_count` bars are emitted at 1h cadence.
fn write_synthetic_parquet(path: &Path, start_ms: i64, bar_count: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mut open_times = Vec::with_capacity(bar_count);
    let mut close_times = Vec::with_capacity(bar_count);
    let mut opens = Vec::with_capacity(bar_count);
    let mut highs = Vec::with_capacity(bar_count);
    let mut lows = Vec::with_capacity(bar_count);
    let mut closes = Vec::with_capacity(bar_count);
    let mut volumes = Vec::with_capacity(bar_count);
    let mut trade_counts = Vec::with_capacity(bar_count);

    let mut price = 30_000.0_f64;
    for i in 0..bar_count {
        let open_ms = start_ms + i as i64 * ONE_HOUR_MS;
        let close_ms = open_ms + ONE_HOUR_MS - 1;
        // Simple deterministic price walk.
        #[allow(clippy::float_arithmetic)]
        let next = price * (1.0 + 0.001 * ((i % 10) as f64 - 5.0) * 0.01);
        open_times.push(open_ms);
        close_times.push(close_ms);
        opens.push(format!("{price:.2}"));
        highs.push(format!("{:.2}", price.max(next) + 10.0));
        lows.push(format!("{:.2}", price.min(next) - 10.0));
        closes.push(format!("{next:.2}"));
        volumes.push("100.0".to_string());
        trade_counts.push(100_i64);
        price = next;
    }

    let opens_ref: Vec<&str> = opens.iter().map(|s| s.as_str()).collect();
    let highs_ref: Vec<&str> = highs.iter().map(|s| s.as_str()).collect();
    let lows_ref: Vec<&str> = lows.iter().map(|s| s.as_str()).collect();
    let closes_ref: Vec<&str> = closes.iter().map(|s| s.as_str()).collect();
    let volumes_ref: Vec<&str> = volumes.iter().map(|s| s.as_str()).collect();

    let mut df = DataFrame::new(vec![
        Column::new("open_time".into(), open_times.as_slice()),
        Column::new("close_time".into(), close_times.as_slice()),
        Column::new("open".into(), opens_ref.as_slice()),
        Column::new("high".into(), highs_ref.as_slice()),
        Column::new("low".into(), lows_ref.as_slice()),
        Column::new("close".into(), closes_ref.as_slice()),
        Column::new("volume".into(), volumes_ref.as_slice()),
        Column::new("trade_count".into(), trade_counts.as_slice()),
    ])
    .unwrap();

    let file = fs::File::create(path).unwrap();
    let writer = std::io::BufWriter::new(file);
    ParquetWriter::new(writer).finish(&mut df).unwrap();
}

/// Build a 2-symbol fixture under `root`.
///
/// Symbols: `ADAUSDT`, `BTCUSDT`
/// Months: 2023/01 and 2023/02
/// Bars per file: 31 * 24 = 744 (January) or 28 * 24 = 672 (February).
///
/// After creating the files, writes `REVISION.toml`.
///
/// Returns the written aggregate SHA.
pub fn build_fixture(root: &Path) -> String {
    let symbols = ["ADAUSDT", "BTCUSDT"];

    // 2023-01-01T00:00:00Z in ms
    let jan_start: i64 = 1_672_531_200_000;
    // 2023-02-01T00:00:00Z in ms
    let feb_start: i64 = 1_675_209_600_000;

    for sym in &symbols {
        let jan_path = root.join(sym).join("2023").join("01.parquet");
        let feb_path = root.join(sym).join("2023").join("02.parquet");
        write_synthetic_parquet(&jan_path, jan_start, 31 * 24); // 744 bars
        write_synthetic_parquet(&feb_path, feb_start, 28 * 24); // 672 bars (2023 is not a leap year)
    }

    data::revision::write_revision_manifest(root).unwrap()
}

/// Build a 10-symbol, 12-month fixture (full year 2023 or 2024).
///
/// Used by determinism tests in `determinism.rs` that need 10 symbols × N months.
/// Returns the aggregate SHA written to `REVISION.toml`.
pub fn build_ten_symbol_fixture(root: &Path, year: i32) -> String {
    let symbols = [
        "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
        "SOLUSDT", "XRPUSDT",
    ];

    // January 1 of `year` at midnight UTC.
    let year_start_ms = year_start_unix_ms(year);

    for sym in &symbols {
        let mut month_start = year_start_ms;
        for m in 1u8..=12 {
            let bars = hours_in_month(year, m) as usize;
            let path = root
                .join(sym)
                .join(year.to_string())
                .join(format!("{m:02}.parquet"));
            write_synthetic_parquet(&path, month_start, bars);
            month_start += bars as i64 * ONE_HOUR_MS;
        }
    }

    data::revision::write_revision_manifest(root).unwrap()
}

fn year_start_unix_ms(year: i32) -> i64 {
    // Count days from UNIX epoch (1970-01-01) to year-01-01.
    let days_from_epoch: i64 = (1970..year)
        .map(|y| if is_leap(y) { 366 } else { 365 })
        .sum();
    days_from_epoch * 24 * 3_600 * 1_000
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn hours_in_month(year: i32, month: u8) -> u32 {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    };
    days * 24
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// (a) Happy path: load succeeds and returns the correct aggregate SHA.
#[test]
fn test_happy_load_returns_expected_sha() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    let expected_sha = build_fixture(&root);

    let universe = vec![Symbol::new("ADAUSDT"), Symbol::new("BTCUSDT")];

    let src = backtest::realdata::RealDataBarSource::new(root.clone(), universe);

    // Span: 2023-01-01 to 2023-03-01 (covers both months in fixture).
    // 2023-01-01T00:00:00Z = 1672531200000 ms
    // 2023-03-01T00:00:00Z = 1677628800000 ms
    let span = backtest::realdata::TimeSpan {
        start_ms: 1_672_531_200_000,
        end_ms: 1_677_628_800_000,
        start_label: "2023-01-01T00:00:00Z",
        end_label: "2023-03-01T00:00:00Z",
    };

    // Expected: 2 syms × (744 + 672) bars = 2832 bars total.
    let expected_total = 2 * (744 + 672); // 2832
    // Tolerance threshold = ceil(2832 * 995 / 1000) = 2818.
    // We have 2832 bars so this passes.

    let loaded = src.load(span, expected_total, "test-scenario").unwrap();

    assert_eq!(
        loaded.revision_sha, expected_sha,
        "loaded revision SHA must match written SHA"
    );
    assert_eq!(
        loaded.revision_sha.len(),
        64,
        "SHA-256 must be 64 hex chars"
    );
    assert_eq!(
        loaded.loaded_count, expected_total,
        "loaded_count must equal the total bars in fixture"
    );
}

/// (b) Tampering one byte of one parquet → `RevisionMismatch`.
#[test]
fn test_tamper_one_byte_causes_revision_mismatch() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    build_fixture(&root);

    // Tamper the first parquet file.
    let parquet = root.join("ADAUSDT/2023/01.parquet");
    let mut bytes = fs::read(&parquet).unwrap();
    // Flip the last byte.
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    fs::write(&parquet, bytes).unwrap();

    let universe = vec![Symbol::new("ADAUSDT"), Symbol::new("BTCUSDT")];
    let src = backtest::realdata::RealDataBarSource::new(root.clone(), universe);

    let span = backtest::realdata::TimeSpan {
        start_ms: 1_672_531_200_000,
        end_ms: 1_677_628_800_000,
        start_label: "2023-01-01T00:00:00Z",
        end_label: "2023-03-01T00:00:00Z",
    };

    let err = src.load(span, 2832, "test-scenario").unwrap_err();
    assert!(
        matches!(
            err,
            backtest::realdata::RealDataError::RevisionMismatch { .. }
        ),
        "expected RevisionMismatch, got: {err}"
    );
}

/// (c) Deleting the manifest → `RevisionMissing`.
#[test]
fn test_missing_manifest_causes_revision_missing() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    build_fixture(&root);

    // Delete the manifest.
    fs::remove_file(root.join("REVISION.toml")).unwrap();

    let universe = vec![Symbol::new("ADAUSDT"), Symbol::new("BTCUSDT")];
    let src = backtest::realdata::RealDataBarSource::new(root.clone(), universe);

    let span = backtest::realdata::TimeSpan {
        start_ms: 1_672_531_200_000,
        end_ms: 1_677_628_800_000,
        start_label: "2023-01-01T00:00:00Z",
        end_label: "2023-03-01T00:00:00Z",
    };

    let err = src.load(span, 2832, "test-scenario").unwrap_err();
    assert!(
        matches!(
            err,
            backtest::realdata::RealDataError::RevisionMissing { .. }
        ),
        "expected RevisionMissing, got: {err}"
    );
}

/// (d) Gap > 0.6% → `MissingData` with `pct < 99.5`.
///
/// We delete the February file for ADAUSDT, which removes 672 bars out of 2832.
/// Missing: 672 / 2832 ≈ 23.7% → well below 99.5% threshold.
#[test]
fn test_gap_causes_missing_data_error() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    build_fixture(&root);

    // Delete February for ADAUSDT — this will cause `no parquet files found`
    // for that month which means fewer bars are loaded.
    // Actually ReplayFeed::merge_symbols loads ALL files for the symbol
    // regardless of time filtering. So deleting the Feb file causes 672
    // fewer bars from ADAUSDT after filtering.
    //
    // However, since we tamper the file system AFTER writing REVISION.toml,
    // we need the SHA check to pass first. The SHA check only validates files
    // that are in `files_for_span`. So we need to delete the file AND update
    // the manifest to not include it (simulating the case where the fetcher
    // wrote fewer bars than expected). Actually, per the test design, the
    // easiest approach is to write a parquet with fewer bars for one symbol.
    //
    // Let's overwrite the February ADAUSDT file with 0 bars and regenerate the manifest.
    let feb_path = root.join("ADAUSDT/2023/02.parquet");

    // Write an empty-ish replacement (just 4 bars = 0.6% gap is enough at this scale,
    // but we need to go below the threshold). For 2832 expected, threshold = 2818.
    // We need loaded < 2818. Removing 672 gives 2832-672 = 2160 << 2818. ✓
    // But we must also update the REVISION.toml to reflect the new SHA.
    // Let's write 0 bars to February ADAUSDT.
    write_synthetic_parquet(&feb_path, 1_675_209_600_000, 0);
    // Wait — write_synthetic_parquet with 0 bars won't write anything meaningful.
    // Instead delete the file and remove it from the manifest.
    fs::remove_file(&feb_path).unwrap();

    // Update the manifest so the deleted file is gone.
    // We need to rebuild the manifest after deletion.
    data::revision::write_revision_manifest(&root).unwrap();

    let universe = vec![Symbol::new("ADAUSDT"), Symbol::new("BTCUSDT")];
    let src = backtest::realdata::RealDataBarSource::new(root.clone(), universe);

    // Span covering Jan + Feb 2023.
    let span = backtest::realdata::TimeSpan {
        start_ms: 1_672_531_200_000,
        end_ms: 1_677_628_800_000,
        start_label: "2023-01-01T00:00:00Z",
        end_label: "2023-03-01T00:00:00Z",
    };

    // Expected = 2832, but we'll only get 2832-672 = 2160 bars (Feb ADAUSDT gone).
    // The load will fail because scenario_files includes ADAUSDT/2023/02.parquet
    // but that file isn't in the manifest anymore — RevisionMismatch (not in manifest).
    // So we need a different approach: write a file with fewer bars but still in the
    // manifest. Let's write 4 bars and regenerate.
    write_synthetic_parquet(&feb_path, 1_675_209_600_000, 4);
    data::revision::write_revision_manifest(&root).unwrap();

    // Now we have 2 (ADAUSDT Jan=744 + 4) + (BTCUSDT Jan=744 + Feb=672) = 748 + 1416 = 2164 bars.
    // Actually ADAUSDT total = 744 + 4 = 748. BTCUSDT total = 744 + 672 = 1416.
    // Total = 748 + 1416 = 2164 bars loaded in span.
    // threshold = ceil(2832 * 995 / 1000) = ceil(2817.84) = 2818.
    // 2164 < 2818 → MissingData.

    let err = src.load(span, 2832, "test-gap-scenario").unwrap_err();
    assert!(
        matches!(err, backtest::realdata::RealDataError::MissingData { pct, .. } if pct < 99.5),
        "expected MissingData with pct < 99.5, got: {err}"
    );
}
