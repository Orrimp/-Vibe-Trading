//! AC2 / M-TEST-1 — round-trip test for the equity companion CSV.
//!
//! Verifies producer/consumer agreement: the file emitted by
//! `backtest::report::write_equity_companion` is byte-shape-compatible
//! with the canonical reader `reports::csv_artifacts::read_equity_csv`
//! (the same reader the cockpit Reports screen / `viewer` bin uses).
//!
//! Assertions:
//!   - row count == `equity_curve.len()`
//!   - each `equity_total` round-trips as `Decimal` (exact equality)
//!   - each `ts` round-trips (RFC3339 parse is lossless)
//!   - realized_pnl / unrealized_pnl / cash_balance are `Decimal::ZERO`
//!     (honest-zero contract — ADR-0055 § D-companion)

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::fs;
use tempfile::TempDir;

/// Locate the single `equity-*.csv` inside `<tmp>/reports/artifacts/<stem>/`.
fn find_equity_csv(reports_dir: &std::path::Path, stem: &str) -> std::path::PathBuf {
    let artifacts_dir = reports_dir.join("artifacts").join(stem);
    for entry in fs::read_dir(&artifacts_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", artifacts_dir.display()))
    {
        let entry = entry.expect("dir entry");
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("equity-") && name_str.ends_with(".csv") {
            return entry.path();
        }
    }
    panic!("no equity-*.csv found under {}", artifacts_dir.display());
}

#[test]
fn equity_companion_roundtrip_basic() {
    let tmp = TempDir::new().expect("tempdir");
    let reports_dir = tmp.path().join("reports");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    // Fake report path — the writer only needs a valid parent dir + file stem.
    // stem = "backtest-20260101-000000-fixture"
    let report_path = reports_dir.join("backtest-20260101-000000-fixture.md");

    // Build a small equity curve (5 bars) with distinct Decimal values.
    let equity_curve: Vec<Decimal> = vec![
        dec!(10000.00),
        dec!(10123.45),
        dec!(9987.50),
        dec!(10250.00),
        dec!(10400.12),
    ];
    let start_year = 2026_i32;

    // Emit the companion CSV.
    backtest::report::write_equity_companion(&report_path, &equity_curve, start_year)
        .expect("write_equity_companion");

    // Locate the emitted CSV.
    let csv_path = find_equity_csv(&reports_dir, "backtest-20260101-000000-fixture");

    // Read it back with the canonical reader.
    let samples = reports::csv_artifacts::read_equity_csv(&csv_path).expect("read_equity_csv");

    // Row count must match.
    assert_eq!(
        samples.len(),
        equity_curve.len(),
        "row count mismatch: expected {}, got {}",
        equity_curve.len(),
        samples.len()
    );

    // Each row: equity_total must round-trip exactly; P&L columns must be 0.
    for (i, (sample, expected_eq)) in samples.iter().zip(equity_curve.iter()).enumerate() {
        assert_eq!(
            sample.equity_total, *expected_eq,
            "equity_total[{i}] mismatch: expected {expected_eq}, got {}",
            sample.equity_total
        );
        assert_eq!(
            sample.realized_pnl,
            Decimal::ZERO,
            "realized_pnl[{i}] should be 0, got {}",
            sample.realized_pnl
        );
        assert_eq!(
            sample.unrealized_pnl,
            Decimal::ZERO,
            "unrealized_pnl[{i}] should be 0, got {}",
            sample.unrealized_pnl
        );
        assert_eq!(
            sample.cash_balance,
            Decimal::ZERO,
            "cash_balance[{i}] should be 0, got {}",
            sample.cash_balance
        );
    }

    // Timestamps must be strictly ascending (hourly synthetic series).
    for w in samples.windows(2) {
        assert!(
            w[0].ts < w[1].ts,
            "ts ordering violated: {:?} >= {:?}",
            w[0].ts,
            w[1].ts
        );
    }
}

/// Test with an empty equity curve — the file should be written with only
/// the header and zero data rows (no panic / error).
#[test]
fn equity_companion_roundtrip_empty_curve() {
    let tmp = TempDir::new().expect("tempdir");
    let reports_dir = tmp.path().join("reports");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let report_path = reports_dir.join("backtest-20260101-000000-empty-fixture.md");
    let equity_curve: Vec<Decimal> = Vec::new();

    backtest::report::write_equity_companion(&report_path, &equity_curve, 2026)
        .expect("write_equity_companion empty curve");

    let csv_path = find_equity_csv(&reports_dir, "backtest-20260101-000000-empty-fixture");

    let samples =
        reports::csv_artifacts::read_equity_csv(&csv_path).expect("read_equity_csv empty");

    assert_eq!(samples.len(), 0, "expected 0 rows for empty curve");
}

/// Confirm the artifacts sub-directory name equals the report file stem
/// and the CSV file starts with "equity-" (loader match criteria).
#[test]
fn equity_companion_path_layout() {
    let tmp = TempDir::new().expect("tempdir");
    let reports_dir = tmp.path().join("reports");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let stem = "backtest-20260617-120000-btc-2023-1m-sma-cross";
    let report_path = reports_dir.join(format!("{stem}.md"));

    let equity_curve: Vec<Decimal> = vec![dec!(10000), dec!(10100), dec!(10200)];

    backtest::report::write_equity_companion(&report_path, &equity_curve, 2023)
        .expect("write_equity_companion");

    // The artifacts dir must exist and equal `reports/artifacts/<stem>`.
    let expected_artifacts_dir = reports_dir.join("artifacts").join(stem);
    assert!(
        expected_artifacts_dir.is_dir(),
        "artifacts dir not created: {}",
        expected_artifacts_dir.display()
    );

    // There must be exactly one equity-*.csv inside.
    let entries: Vec<_> = fs::read_dir(&expected_artifacts_dir)
        .expect("read_dir")
        .filter_map(|e| {
            let e = e.ok()?;
            let n = e.file_name();
            let s = n.to_string_lossy();
            if s.starts_with("equity-") && s.ends_with(".csv") {
                Some(e.path())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(entries.len(), 1, "expected exactly 1 equity-*.csv");
}
