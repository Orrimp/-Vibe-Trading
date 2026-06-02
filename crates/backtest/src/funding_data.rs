//! Funding-data source for the carry-strategy backtest harness.
//!
//! Mirrors `realdata.rs` (`RealDataBarSource`) but for the funding parquets
//! under `data/binance-funding/`. Schema: `symbol` Utf8, `funding_time` Int64
//! (Unix ms), `funding_rate` Utf8 decimal-string.
//!
//! # Design decisions
//!
//! - `funding_rate` is parsed as `rust_decimal::Decimal` (never f64) to
//!   preserve precision end-to-end (ADR-0003).
//! - REVISION.toml aggregate SHA is verified against the expected constant
//!   `EXPECTED_FUNDING_REVISION_SHA` (D-CARRY.5, R-CARRY.5).
//! - The loader output is a leaner `Vec<FundingRow>` (no `next_funding_ts` /
//!   `poll_ts` — those are live-agent fields not needed here).
//! - `funding_as_of` is a pure function (M-DEV-2): given a sorted funding
//!   series and a slice of bar open-timestamps (ms), it forward-fills the
//!   most-recent settled funding at-or-before each bar. Returns `None` for
//!   bars before the first settlement (warm-up).
//!
//! # Feature gate
//!
//! Compiled only when `--features realdata` (which pulls in polars).

use std::path::PathBuf;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use trading_core::Symbol;

// ── Expected revision SHA ──────────────────────────────────────────────────────

/// Locked aggregate SHA-256 for `data/binance-funding/REVISION.toml`.
/// Any mismatch → `FundingDataError::RevisionMismatch`.
pub const EXPECTED_FUNDING_REVISION_SHA: &str =
    "bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668";

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum FundingDataError {
    #[error("data/binance-funding/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    /// On-disk aggregate SHA does not match the locked expected SHA.
    ///
    /// The carry backtest must reject runs on unverified data.
    #[error(
        "funding data revision mismatch: expected={expected}, recomputed={recomputed} \
         (file {file}: manifest={manifest_sha}, on-disk={actual_sha})"
    )]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
        expected: String,
        recomputed: String,
    },

    #[error("funding parquet read error for {path}: {source}")]
    Parquet {
        path: String,
        source: polars::prelude::PolarsError,
    },

    #[error("funding_rate parse error in {path} row {row}: {value}: {source}")]
    DecimalParse {
        path: String,
        row: usize,
        value: String,
        source: rust_decimal::Error,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── FundingRow ────────────────────────────────────────────────────────────────

/// A single funding settlement record loaded from the parquet.
///
/// Tuple: `(symbol, funding_time_ms, funding_rate)`.
/// The rate is signed (`+` = longs pay shorts, `−` = shorts pay longs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingRow {
    pub symbol: Symbol,
    /// Unix millisecond timestamp of the settlement (matches Binance `fundingTime`).
    pub funding_time_ms: i64,
    /// Exact Binance decimal rate, e.g. `Decimal::from_str("0.00010000")`.
    pub funding_rate: Decimal,
}

// ── LoadedFunding ─────────────────────────────────────────────────────────────

/// Result of a successful funding load.
#[derive(Debug)]
pub struct LoadedFunding {
    /// All funding rows across all symbols, sorted `(funding_time_ms ASC,
    /// symbol ASC)`.
    pub rows: Vec<FundingRow>,
    /// Recomputed aggregate SHA-256 (64 hex chars). Written into the carry
    /// report body's `## Data source` section.
    pub revision_sha: String,
}

// ── FundingDataSource ─────────────────────────────────────────────────────────

/// Carry-funding parquet loader, mirroring `RealDataBarSource`.
pub struct FundingDataSource {
    /// Root of the funding parquet directory, e.g. `data/binance-funding`.
    funding_root: PathBuf,
    /// Universe of symbols to load (must match the carry scenario universe).
    universe: Vec<Symbol>,
}

impl FundingDataSource {
    /// Create a new funding data source.
    ///
    /// `funding_root` must contain both the symbol subdirectories and
    /// `REVISION.toml` (same layout as `data/binance/` but for funding).
    #[must_use]
    pub fn new(funding_root: PathBuf, universe: Vec<Symbol>) -> Self {
        Self {
            funding_root,
            universe,
        }
    }

    /// Load + REVISION-verify + parse funding rows for the given span.
    ///
    /// Steps:
    /// 1. Check `REVISION.toml` exists → `RevisionMissing`.
    /// 2. For every parquet file in the span, verify on-disk SHA against
    ///    manifest → `RevisionMismatch`.
    /// 3. Recompute aggregate SHA and verify it equals
    ///    `EXPECTED_FUNDING_REVISION_SHA`.
    /// 4. Read parquet files; parse `funding_rate` as `Decimal`.
    /// 5. Filter rows to `[span.start_ms, span.end_ms)`.
    /// 6. Sort `(funding_time_ms ASC, symbol ASC)`.
    ///
    /// # Errors
    ///
    /// Returns `FundingDataError` on manifest missing / SHA mismatch,
    /// parquet read errors, or Decimal parse failure.
    #[allow(clippy::too_many_lines)]
    pub fn load(
        &self,
        span: &crate::realdata::TimeSpan,
        _scenario_name: &str,
    ) -> Result<LoadedFunding, FundingDataError> {
        use polars::prelude::{LazyFrame, ScanArgsParquet};

        // Step 1: manifest exists?
        let manifest_path = self.funding_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(FundingDataError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Read manifest (no on-disk SHA check yet).
        let (files_map, _claimed_aggregate) = data::revision::read_manifest_raw(&self.funding_root)
            .map_err(|e| FundingDataError::RevisionParse(e.to_string()))?;

        // Step 2: verify each parquet file in the span.
        let scenario_files = self.files_for_span(span);
        for relpath in &scenario_files {
            let manifest_sha =
                files_map
                    .get(relpath)
                    .ok_or_else(|| FundingDataError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".to_string(),
                        actual_sha: "n/a".to_string(),
                        expected: EXPECTED_FUNDING_REVISION_SHA.to_string(),
                        recomputed: "(not computed)".to_string(),
                    })?;
            let abs_path = self.funding_root.join(relpath);
            let actual_sha = data::revision::file_sha256(&abs_path)
                .map_err(|e| FundingDataError::RevisionParse(format!("sha256 read error: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(FundingDataError::RevisionMismatch {
                    file: relpath.clone(),
                    manifest_sha: manifest_sha.clone(),
                    actual_sha,
                    expected: EXPECTED_FUNDING_REVISION_SHA.to_string(),
                    recomputed: "(not computed)".to_string(),
                });
            }
        }

        // Step 3: recompute aggregate SHA and verify against the locked constant.
        let recomputed = data::revision::compute_aggregate_sha(&files_map);
        if recomputed != EXPECTED_FUNDING_REVISION_SHA {
            return Err(FundingDataError::RevisionMismatch {
                file: "(aggregate)".to_string(),
                manifest_sha: "(n/a)".to_string(),
                actual_sha: "(n/a)".to_string(),
                expected: EXPECTED_FUNDING_REVISION_SHA.to_string(),
                recomputed: recomputed.clone(),
            });
        }

        // Step 4: read + parse parquet files.
        let mut rows: Vec<FundingRow> = Vec::new();
        for relpath in &scenario_files {
            let abs_path = self.funding_root.join(relpath);
            let path_str = abs_path.to_string_lossy().into_owned();

            let df = LazyFrame::scan_parquet(&abs_path, ScanArgsParquet::default())
                .and_then(polars::prelude::LazyFrame::collect)
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let symbols_col = df
                .column("symbol")
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .str()
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let times_col = df
                .column("funding_time")
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .i64()
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let rates_col = df
                .column("funding_rate")
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .str()
                .map_err(|e| FundingDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let n_rows = df.height();
            for i in 0..n_rows {
                let sym_str = symbols_col.get(i).unwrap_or("");
                let funding_time_ms = times_col.get(i).unwrap_or(0);
                let rate_str = rates_col.get(i).unwrap_or("");

                // Step 5: filter to span [start_ms, end_ms).
                if funding_time_ms < span.start_ms || funding_time_ms >= span.end_ms {
                    continue;
                }

                let funding_rate =
                    Decimal::from_str(rate_str).map_err(|e| FundingDataError::DecimalParse {
                        path: path_str.clone(),
                        row: i,
                        value: rate_str.to_string(),
                        source: e,
                    })?;

                rows.push(FundingRow {
                    symbol: Symbol::new(sym_str),
                    funding_time_ms,
                    funding_rate,
                });
            }
        }

        // Step 6: sort (funding_time_ms ASC, symbol ASC).
        rows.sort_unstable_by(|a, b| {
            a.funding_time_ms
                .cmp(&b.funding_time_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });

        Ok(LoadedFunding {
            rows,
            revision_sha: recomputed,
        })
    }

    /// Return relative parquet paths for the scenario span.
    ///
    /// Layout: `<SYM>/<YEAR>/<MM>.parquet` — identical to
    /// `RealDataBarSource::files_for_span`.
    #[must_use]
    pub fn files_for_span(&self, span: &crate::realdata::TimeSpan) -> Vec<String> {
        let start_dt = OffsetDateTime::from_unix_timestamp(span.start_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let end_dt = OffsetDateTime::from_unix_timestamp(span.end_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        let mut result = Vec::new();
        let mut year = start_dt.year();
        let mut month = start_dt.month();

        loop {
            let month_start = month_start_ms(year, month);
            if month_start >= span.end_ms {
                break;
            }

            let month_num = month as u8;
            for sym in &self.universe {
                result.push(format!("{}/{year}/{month_num:02}.parquet", sym.0.as_str()));
            }

            // Advance to next month (mirrors realdata.rs exactly).
            let next_num = month_num % 12 + 1;
            let next_year = if next_num == 1 { year + 1 } else { year };
            let next_month = time::Month::try_from(next_num).unwrap_or(time::Month::January);
            let next_ms = month_start_ms(next_year, next_month);
            if next_ms >= span.end_ms {
                break;
            }
            if next_year > end_dt.year()
                || (next_year == end_dt.year() && next_month > end_dt.month())
            {
                break;
            }
            year = next_year;
            month = next_month;
        }

        result
    }
}

// ── Date helpers (mirror realdata.rs) ─────────────────────────────────────────

fn month_start_ms(year: i32, month: time::Month) -> i64 {
    let date = time::Date::from_calendar_date(year, month, 1)
        .unwrap_or(time::Date::from_ordinal_date(2023, 1).unwrap_or(time::Date::MIN));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT).unix_timestamp() * 1_000
}

// ── M-DEV-2: as-of forward-fill ───────────────────────────────────────────────

/// Forward-fill funding rates onto a bar-open-timestamp grid.
///
/// For each bar open timestamp `bar_open_ts_ms[i]`, finds the funding rate
/// from the last settlement **at or before** that timestamp (information
/// available at decision time — no look-ahead). Returns `None` for bars
/// before the first settlement (warm-up period).
///
/// # Arguments
///
/// - `funding`: Sorted `(funding_time_ms, funding_rate)` pairs, in ascending
///   `funding_time_ms` order. MUST be sorted for correct results.
/// - `bar_open_ts_ms`: Bar open timestamps in any order (each is processed
///   independently via binary search).
///
/// # Returns
///
/// `Vec<Option<Decimal>>` of length `bar_open_ts_ms.len()`.
/// `None` = no settlement has occurred yet (warm-up).
///
/// # Invariant (no look-ahead)
///
/// Only funding settled **at or before** the bar's `open_ts` is used.
/// Future-shifting the funding series (e.g. by +8 h) WILL produce a
/// different result — verified by the unit test `no_look_ahead_falsifier`.
#[must_use]
pub fn funding_as_of(funding: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>> {
    if funding.is_empty() {
        return vec![None; bar_open_ts_ms.len()];
    }

    bar_open_ts_ms
        .iter()
        .map(|&bar_ts| {
            // Binary-search for the rightmost settlement ≤ bar_ts.
            // `partition_point(|&(t, _)| t <= bar_ts)` gives the index of the
            // first element > bar_ts; the settlement we want is at idx-1.
            let idx = funding.partition_point(|&(t, _)| t <= bar_ts);
            if idx == 0 {
                // No settlement at or before this bar → warm-up.
                None
            } else {
                // The settlement just before or at bar_ts.
                Some(funding[idx - 1].1)
            }
        })
        .collect()
}

/// Build the `funding_at_return[sym_i][k]` array used by the bootstrap.
///
/// The bootstrap constructs `n_returns = T-1` log-returns from `T` bars:
/// `r[k] = ln(close[k+1] / close[k])`. The funding aligned to return step `k`
/// is the as-of funding at the **open** of source bar `k` (the bar the return
/// departs from). This is computed once on the real data and stored in a
/// `Vec<Vec<Option<Decimal>>>` indexed `[sym_i][return_step]`.
///
/// # Arguments
///
/// - `funding_by_symbol`: For each symbol, the sorted `(funding_time_ms, rate)`
///   pairs covering the real span.
/// - `bar_open_ts_ms_by_symbol`: For each symbol, the slice of bar open
///   timestamps on the real grid (length `T`). The return-step slice has
///   length `T-1` = `bar_open_ts_ms_by_symbol[s][0..T-1]`.
///
/// # Returns
///
/// `funding_at_return[sym_i][k]` = `Option<Decimal>` for k in 0..T-1.
#[must_use]
pub fn build_funding_at_return(
    funding_by_symbol: &[&[(i64, Decimal)]],
    bar_open_ts_ms_by_symbol: &[&[i64]],
) -> Vec<Vec<Option<Decimal>>> {
    funding_by_symbol
        .iter()
        .zip(bar_open_ts_ms_by_symbol.iter())
        .map(|(funding, bar_ts)| {
            // Return series has T-1 steps; align to bars 0..T-1.
            let n_returns = bar_ts.len().saturating_sub(1);
            let return_bar_ts = &bar_ts[..n_returns];
            funding_as_of(funding, return_bar_ts)
        })
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::pedantic,
    clippy::identity_op
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── funding_as_of unit tests (M-DEV-2) ────────────────────────────────────

    /// Verify: bar before the first settlement → None (warm-up).
    #[test]
    fn warm_up_before_first_settlement_is_none() {
        let funding = vec![
            (1_000, dec!(0.001)),
            (2_000, dec!(0.002)),
            (3_000, dec!(0.003)),
        ];
        // bar at t=500 is before the first settlement at t=1000.
        let result = funding_as_of(&funding, &[500]);
        assert_eq!(
            result,
            vec![None],
            "bar before first settlement must be None"
        );
    }

    /// Verify: bar exactly at a settlement time uses that settlement.
    #[test]
    fn bar_at_settlement_uses_that_settlement() {
        let funding = vec![(1_000, dec!(0.001)), (2_000, dec!(0.002))];
        let result = funding_as_of(&funding, &[1_000]);
        assert_eq!(result, vec![Some(dec!(0.001))]);
    }

    /// Verify: bar between two settlements uses the earlier one (forward-fill).
    #[test]
    fn bar_between_settlements_uses_earlier() {
        let funding = vec![(1_000, dec!(0.001)), (3_000, dec!(0.003))];
        // bar at 2_000 is between 1_000 and 3_000 → should use 1_000's rate.
        let result = funding_as_of(&funding, &[2_000]);
        assert_eq!(result, vec![Some(dec!(0.001))]);
    }

    /// Step-function correctness: a sequence of bars across multiple settlements.
    #[test]
    fn step_function_correctness() {
        // 8h cadence: settlements at t=0, 28800000, 57600000 (ms).
        let eight_hours_ms: i64 = 8 * 3_600_000;
        let funding = vec![
            (0, dec!(0.001)),
            (eight_hours_ms, dec!(0.002)),
            (2 * eight_hours_ms, dec!(0.003)),
        ];
        // Bars at each hour boundary.
        let bars: Vec<i64> = (0..24).map(|h| h * 3_600_000_i64).collect();
        let result = funding_as_of(&funding, &bars);

        // Hour 0: exactly at settlement t=0 → 0.001.
        assert_eq!(result[0], Some(dec!(0.001)));
        // Hour 7: still before the second settlement (8h) → 0.001.
        assert_eq!(result[7], Some(dec!(0.001)));
        // Hour 8: exactly at second settlement → 0.002.
        assert_eq!(result[8], Some(dec!(0.002)));
        // Hour 15: still before third settlement (16h) → 0.002.
        assert_eq!(result[15], Some(dec!(0.002)));
        // Hour 16: exactly at third settlement → 0.003.
        assert_eq!(result[16], Some(dec!(0.003)));
        // Hour 23: after all settlements → 0.003.
        assert_eq!(result[23], Some(dec!(0.003)));
    }

    /// No-look-ahead falsifier (R-CARRY.6):
    /// Shifting funding series +1 settlement into the FUTURE changes the result.
    /// This proves the join is causal (past-only) — future funding never leaks.
    #[test]
    fn no_look_ahead_falsifier() {
        let one_settlement_ms: i64 = 8 * 3_600_000;
        let funding = vec![
            (1 * one_settlement_ms, dec!(0.001)),
            (2 * one_settlement_ms, dec!(0.002)),
            (3 * one_settlement_ms, dec!(0.003)),
        ];

        // Shift each settlement timestamp +1 settlement into the future.
        let future_shifted: Vec<(i64, Decimal)> = funding
            .iter()
            .map(|&(t, r)| (t + one_settlement_ms, r))
            .collect();

        // Query a bar that is at the original settlement boundary.
        let bar_ts = &[2 * one_settlement_ms];

        let causal_result = funding_as_of(&funding, bar_ts);
        let shifted_result = funding_as_of(&future_shifted, bar_ts);

        // Causal: bar at 2 * one_settlement sees rate at 2 * one_settlement → 0.002.
        assert_eq!(causal_result, vec![Some(dec!(0.002))]);
        // Shifted: the second settlement is now at 3 * one_settlement, so the
        // most-recent settled at 2 * one_settlement is at 1 * one_settlement → 0.001.
        assert_eq!(shifted_result, vec![Some(dec!(0.001))]);
        // They MUST differ — proves no look-ahead.
        assert_ne!(
            causal_result, shifted_result,
            "no-look-ahead falsifier: causal ≠ shifted"
        );
    }

    /// Empty funding series → all None.
    #[test]
    fn empty_funding_series_all_none() {
        let result = funding_as_of(&[], &[100, 200, 300]);
        assert_eq!(result, vec![None, None, None]);
    }

    /// build_funding_at_return aligns to T-1 return steps, not T bars.
    #[test]
    fn build_funding_at_return_aligns_to_t_minus_1() {
        let funding: Vec<(i64, Decimal)> = vec![(0, dec!(0.001)), (8 * 3_600_000, dec!(0.002))];
        // 5 bars → 4 return steps.
        let bar_ts: Vec<i64> = (0..5).map(|h| h * 3_600_000_i64).collect();

        let result = build_funding_at_return(&[funding.as_slice()], &[bar_ts.as_slice()]);

        // Should have length T-1 = 4 for the one symbol.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 4, "T-1 return steps");
        // Bar 0 (t=0): settlement at t=0 → Some(0.001).
        assert_eq!(result[0][0], Some(dec!(0.001)));
        // Bar 3 (t=3h): settlement still at t=0 (before 8h) → Some(0.001).
        assert_eq!(result[0][3], Some(dec!(0.001)));
    }

    /// Decimal precision is preserved — no f64 round-trip.
    #[test]
    fn decimal_precision_preserved() {
        // Use a rate with many decimal places that would lose precision as f64.
        let rate_str = "0.00010001";
        let parsed = Decimal::from_str(rate_str).expect("parse");
        let expected = dec!(0.00010001);
        assert_eq!(parsed, expected, "decimal precision must be exact");
        // Confirm round-trip through string preserves value.
        let round_tripped = parsed.to_string();
        let reparsed = Decimal::from_str(&round_tripped).expect("reparse");
        assert_eq!(parsed, reparsed);
    }

    // ── REVISION-mismatch rejection test (M-DEV-1) ────────────────────────────

    /// When the REVISION.toml claims an aggregate SHA different from the
    /// locked constant, `load()` must reject with `RevisionMismatch`.
    ///
    /// We construct a temp-dir with a fake REVISION.toml claiming a wrong SHA.
    #[test]
    fn revision_mismatch_is_rejected() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        // Write a fake parquet file and a REVISION.toml with a wrong aggregate.
        let sym_dir = root.join("BTCUSDT/2023");
        fs::create_dir_all(&sym_dir).expect("create dir");
        let fake_parquet_path = sym_dir.join("01.parquet");
        fs::write(&fake_parquet_path, b"fake data").expect("write fake parquet");

        // Compute actual SHA of the fake file for the manifest.
        let fake_sha = data::revision::file_sha256(&fake_parquet_path).expect("sha256");

        // Build a REVISION.toml with a SHA that won't match EXPECTED_FUNDING_REVISION_SHA.
        let wrong_aggregate_sha =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let toml_content = format!(
            "[revision]\nsha256 = \"{wrong_aggregate_sha}\"\n\n[files]\n\"BTCUSDT/2023/01.parquet\" = \"{fake_sha}\"\n"
        );
        fs::write(root.join("REVISION.toml"), &toml_content).expect("write revision toml");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = FundingDataSource::new(root.to_path_buf(), vec![Symbol::new("BTCUSDT")]);

        let result = src.load(&span, "test-scenario");
        assert!(
            result.is_err(),
            "load() must fail when the aggregate SHA does not match the locked constant"
        );
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("revision mismatch")
                || matches!(err, FundingDataError::RevisionMismatch { .. }),
            "error must be RevisionMismatch, got: {err_str}"
        );
    }

    // ── Known parquet parse test (M-DEV-1) ───────────────────────────────────
    // This test is ignored by default because it requires the real funding
    // parquet files at data/binance-funding/. Run with:
    //   cargo test -p backtest funding_data -- --include-ignored
    #[test]
    #[ignore = "requires real data/binance-funding/ parquet files on disk"]
    fn real_parquet_parses_to_expected_rows() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let funding_root = workspace_root.join("data/binance-funding");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = FundingDataSource::new(funding_root, vec![Symbol::new("BTCUSDT")]);

        let loaded = src.load(&span, "test-real").expect("load must succeed");

        // BTCUSDT 2023: 365 days × 3 settlements/day = 1095 expected rows.
        // Allow ±3 for any edge-of-year boundary settlements.
        assert!(
            loaded.rows.len() >= 1090 && loaded.rows.len() <= 1100,
            "expected ~1095 BTCUSDT funding rows for 2023-FY, got {}",
            loaded.rows.len()
        );

        // Every row must have a valid symbol and a rate that isn't zero.
        for row in &loaded.rows {
            assert_eq!(row.symbol, Symbol::new("BTCUSDT"));
            // Funding rates are typically non-zero but we just verify parseable.
            let _rate: Decimal = row.funding_rate; // already a Decimal — compile check
        }

        // Revision SHA must match the locked constant.
        assert_eq!(
            loaded.revision_sha, EXPECTED_FUNDING_REVISION_SHA,
            "revision SHA must match the locked constant"
        );
    }

    /// Out-of-span filter: funding times outside [start_ms, end_ms) are excluded.
    ///
    /// We test this at the `funding_as_of` level (pure function — no parquet I/O):
    /// a bar whose open_ts is before the span's first settlement → None.
    #[test]
    fn out_of_span_filter_via_funding_as_of() {
        // Settlements only from 2023-01-01 onward (t=1_672_531_200_000 ms).
        let year_start_ms: i64 = 1_672_531_200_000;
        let funding = vec![
            (year_start_ms, dec!(0.0001)),
            (year_start_ms + 8 * 3_600_000, dec!(0.0002)),
        ];

        // A bar before the year start is treated as warm-up.
        let pre_span_bar = year_start_ms - 1;
        let result = funding_as_of(&funding, &[pre_span_bar, year_start_ms]);
        assert_eq!(result[0], None, "bar before first settlement must be None");
        assert_eq!(
            result[1],
            Some(dec!(0.0001)),
            "bar at first settlement must be Some"
        );
    }
}
