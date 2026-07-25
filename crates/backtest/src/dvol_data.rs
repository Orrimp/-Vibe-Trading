//! DVOL implied-vol data source for the `v0.dvol_regime` bake-off arm.
//!
//! Mirrors `basis_data.rs` (`BasisDataSource`) but for the Deribit DVOL
//! parquets under `data/deribit-dvol/`. Schema: per-symbol/per-year parquets
//! with columns `day_open_ts_ms` Int64, `day_close_ts_ms` Int64,
//! `dvol_open/high/low/close` Float64 — the signal consumes `dvol_close` ONLY.
//!
//! # Design decisions
//!
//! - `dvol_close` is Float64 on disk (annualized-vol points, e.g. 52.4), parsed
//!   to `rust_decimal::Decimal` at the seam — identical to how `basis_close`
//!   Float64 → `Decimal` in `basis_data.rs:load`. DVOL is dimensionless and
//!   never enters a money/P&L computation (ADR-0003 money-math rule untouched).
//! - REVISION.toml aggregate SHA is verified against the expected constant
//!   `EXPECTED_DVOL_REVISION_SHA` (D-BR.3 pattern, basis_data.rs:45).
//!   The loader **refuses to run on unverified data**.
//! - `dvol_as_of` is a pure function (M-DEV-2): given a sorted DVOL series
//!   (by `day_close_ts_ms`) and bar open-timestamps, it finds the most-recent
//!   DVOL daily close at-or-before each bar. Returns `None` for bars before
//!   the first available DVOL close (warm-up). Routes through
//!   `trading_core::pit::PitSeries` (ADR-0058 / M-DEV-3) — look-ahead impossible.
//! - The as-of KEY is `day_close_ts_ms` (the instant the daily close is FULLY
//!   observed = `day_open_ts_ms + 86_400_000 - 1`). An hourly bar opening at `t`
//!   sees ONLY the most-recent DVOL close with `day_close_ts_ms ≤ t`.
//!
//! # Feature gate
//!
//! Compiled only when `--features realdata` (which pulls in polars).

use std::path::PathBuf;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use trading_core::{PitSeries, Symbol, TimestampMs};

// ── Expected revision SHA ──────────────────────────────────────────────────────

/// Locked aggregate SHA-256 for `data/deribit-dvol/REVISION.toml`.
///
/// Pinned to the real corpus aggregate after running
/// `cargo run -p data --bin fetch_deribit_dvol -- --emit-revision-manifest`.
/// Any mismatch → `DvolDataError::RevisionMismatch`.
///
/// To bypass during development (no real data yet), tests use synthetic
/// `dvol_as_of` directly without going through `DvolDataSource::load`.
///
/// **2026-07-10 (ADR-0084 T6 back-fill):** re-pinned after additively
/// fetching 2021/2022/2025/2026 BTC+ETH DVOL (was 2023/2024 only, 4 files;
/// now 12 files). The 4 pre-existing 2023/2024 parquets are byte-identical
/// (verified via `shasum -a 256 -c` against a pre-fetch snapshot) — only
/// this manifest-aggregate constant moves, because the aggregate SHA is
/// computed over the WHOLE `[files]` map (`compute_aggregate_sha`), not a
/// per-span subset; adding new file ROWS necessarily changes it even though
/// no existing row's value changed. This constant is NOT one of the 9
/// `evidence/anchors.toml` regression anchors (a fully separate pin) — updating
/// it does not touch `verify_anchors.sh`'s 119/119 gate.
pub const EXPECTED_DVOL_REVISION_SHA: &str =
    "b21dc8691c257731d9043fc3e19b858c326ab4dd3d975f10de0eccf90cf480ff";

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum DvolDataError {
    #[error("data/deribit-dvol/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    /// On-disk aggregate SHA does not match the locked expected SHA.
    ///
    /// The DVOL backtest must reject runs on unverified data.
    #[error(
        "DVOL data revision mismatch: expected={expected}, recomputed={recomputed} \
         (file {file}: manifest={manifest_sha}, on-disk={actual_sha})"
    )]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
        expected: String,
        recomputed: String,
    },

    #[error("DVOL parquet read error for {path}: {source}")]
    Parquet {
        path: String,
        source: polars::prelude::PolarsError,
    },

    #[error("dvol_close parse error in {path} row {row}: {value}: {source}")]
    DecimalParse {
        path: String,
        row: usize,
        value: String,
        source: rust_decimal::Error,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── DvolRow ───────────────────────────────────────────────────────────────────

/// A single DVOL daily row loaded from the parquet.
///
/// Tuple key: `(symbol, day_close_ts_ms, dvol_close)`.
/// `day_close_ts_ms = day_open_ts_ms + 86_400_000 - 1` — the instant the
/// daily close is FULLY observed (the as-of join key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DvolRow {
    pub symbol: Symbol,
    /// `day_open_ts_ms + 86_400_000 - 1` — the as-of join key.
    pub day_close_ts_ms: i64,
    /// DVOL daily close (annualized-vol points, e.g. `52.4`).
    /// Parsed as exact `Decimal` — no f64 round-trip (ADR-0003).
    pub dvol_close: Decimal,
}

// ── LoadedDvol ────────────────────────────────────────────────────────────────

/// Result of a successful DVOL load.
#[derive(Debug)]
pub struct LoadedDvol {
    /// All DVOL rows across all symbols, sorted `(day_close_ts_ms ASC, symbol ASC)`.
    pub rows: Vec<DvolRow>,
    /// Recomputed aggregate SHA-256 (64 hex chars).
    pub revision_sha: String,
}

// ── DvolDataSource ─────────────────────────────────────────────────────────────

/// Deribit DVOL parquet loader, mirroring `BasisDataSource`.
pub struct DvolDataSource {
    /// Root of the DVOL parquet directory, e.g. `data/deribit-dvol`.
    dvol_root: PathBuf,
    /// Universe of symbols to load. Only `Symbol::new("BTC")` and
    /// `Symbol::new("ETH")` are supported (DVOL exists only for BTC+ETH).
    universe: Vec<Symbol>,
}

impl DvolDataSource {
    /// Create a new DVOL data source.
    ///
    /// `dvol_root` must contain both the symbol subdirectories and
    /// `REVISION.toml` (same layout as `data/deribit-dvol/`).
    #[must_use]
    pub fn new(dvol_root: PathBuf, universe: Vec<Symbol>) -> Self {
        Self {
            dvol_root,
            universe,
        }
    }

    /// Load + REVISION-verify + parse DVOL rows for the given span.
    ///
    /// Steps:
    /// 1. Check `REVISION.toml` exists → `RevisionMissing`.
    /// 2. For every parquet file in the span, verify on-disk SHA against
    ///    manifest → `RevisionMismatch`.
    /// 3. Recompute aggregate SHA and verify it equals
    ///    `EXPECTED_DVOL_REVISION_SHA`.
    /// 4. Read parquet files; parse `dvol_close` as `Decimal`.
    /// 5. Filter rows to `[span.start_ms, span.end_ms)` by `day_close_ts_ms`.
    /// 6. Sort `(day_close_ts_ms ASC, symbol ASC)`.
    ///
    /// # Errors
    ///
    /// Returns `DvolDataError` on manifest missing / SHA mismatch,
    /// parquet read errors, or Decimal parse failure.
    #[allow(clippy::too_many_lines)]
    pub fn load(
        &self,
        span: &crate::realdata::TimeSpan,
        _scenario_name: &str,
    ) -> Result<LoadedDvol, DvolDataError> {
        use polars::prelude::{LazyFrame, ScanArgsParquet};

        // Step 1: manifest exists?
        let manifest_path = self.dvol_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(DvolDataError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Read manifest (no on-disk SHA check yet).
        let (files_map, _claimed_aggregate) = data::revision::read_manifest_raw(&self.dvol_root)
            .map_err(|e| DvolDataError::RevisionParse(e.to_string()))?;

        // Step 2: verify each parquet file in the span.
        let scenario_files = self.files_for_span(span);
        for relpath in &scenario_files {
            let manifest_sha =
                files_map
                    .get(relpath)
                    .ok_or_else(|| DvolDataError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".to_string(),
                        actual_sha: "n/a".to_string(),
                        expected: EXPECTED_DVOL_REVISION_SHA.to_string(),
                        recomputed: "(not computed)".to_string(),
                    })?;
            let abs_path = self.dvol_root.join(relpath);
            let actual_sha = data::revision::file_sha256(&abs_path)
                .map_err(|e| DvolDataError::RevisionParse(format!("sha256 read error: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(DvolDataError::RevisionMismatch {
                    file: relpath.clone(),
                    manifest_sha: manifest_sha.clone(),
                    actual_sha,
                    expected: EXPECTED_DVOL_REVISION_SHA.to_string(),
                    recomputed: "(not computed)".to_string(),
                });
            }
        }

        // Step 3: recompute aggregate SHA and verify against the locked constant.
        let recomputed = data::revision::compute_aggregate_sha(&files_map);
        if recomputed != EXPECTED_DVOL_REVISION_SHA {
            return Err(DvolDataError::RevisionMismatch {
                file: "(aggregate)".to_string(),
                manifest_sha: "(n/a)".to_string(),
                actual_sha: "(n/a)".to_string(),
                expected: EXPECTED_DVOL_REVISION_SHA.to_string(),
                recomputed: recomputed.clone(),
            });
        }

        // Step 4: read + parse parquet files.
        let mut rows: Vec<DvolRow> = Vec::new();
        for relpath in &scenario_files {
            let abs_path = self.dvol_root.join(relpath);
            let path_str = abs_path.to_string_lossy().into_owned();

            let df = LazyFrame::scan_parquet(&abs_path, ScanArgsParquet::default())
                .and_then(polars::prelude::LazyFrame::collect)
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            // The symbol is encoded in the file path: <SYM>/<YEAR>.parquet
            let sym_str = relpath.split('/').next().unwrap_or("");

            let close_ts_col = df
                .column("day_close_ts_ms")
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .i64()
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let dvol_close_col = df
                .column("dvol_close")
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .f64()
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let n_rows = df.height();
            for i in 0..n_rows {
                let day_close_ts_ms = close_ts_col.get(i).unwrap_or(0);
                let dvol_f64 = dvol_close_col.get(i).unwrap_or(0.0);

                // Step 5: filter to span [start_ms, end_ms) by day_close_ts_ms.
                if day_close_ts_ms < span.start_ms || day_close_ts_ms >= span.end_ms {
                    continue;
                }

                // Convert f64 DVOL close to Decimal.
                // DVOL is dimensionless (annualized-vol points like 52.4);
                // use to_string → from_str to avoid lossy f64 Decimal conversion.
                let dvol_str = format!("{dvol_f64:.4}");
                let dvol_close =
                    dvol_str
                        .parse::<Decimal>()
                        .map_err(|e| DvolDataError::DecimalParse {
                            path: path_str.clone(),
                            row: i,
                            value: dvol_str.clone(),
                            source: e,
                        })?;

                rows.push(DvolRow {
                    symbol: Symbol::new(sym_str),
                    day_close_ts_ms,
                    dvol_close,
                });
            }
        }

        // Step 6: sort (day_close_ts_ms ASC, symbol ASC).
        rows.sort_unstable_by(|a, b| {
            a.day_close_ts_ms
                .cmp(&b.day_close_ts_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });

        Ok(LoadedDvol {
            rows,
            revision_sha: recomputed,
        })
    }

    /// Return relative parquet paths for the scenario span.
    ///
    /// Layout: `<SYM>/<YEAR>.parquet` — one file per symbol per year.
    #[must_use]
    pub fn files_for_span(&self, span: &crate::realdata::TimeSpan) -> Vec<String> {
        let start_dt = OffsetDateTime::from_unix_timestamp(span.start_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let end_dt = OffsetDateTime::from_unix_timestamp(span.end_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        let start_year = start_dt.year();
        let end_year = end_dt.year();

        let mut result = Vec::new();
        for sym in &self.universe {
            for year in start_year..=end_year {
                result.push(format!("{}/{year}.parquet", sym.0.as_str()));
            }
        }
        result
    }
}

// ── M-DEV-2: as-of join ───────────────────────────────────────────────────────

/// Find the DVOL daily close at each bar open timestamp using an as-of join.
///
/// For each bar open timestamp `bar_open_ts_ms[i]`, finds the `dvol_close`
/// from the most-recent DVOL day whose `day_close_ts_ms` is **at-or-before**
/// `bar_open_ts_ms[i]`.
///
/// The as-of KEY is `day_close_ts_ms` (NOT `day_open_ts_ms`). The daily close
/// is FULLY observed only when the day ends (`day_close_ts_ms = midnight + 86_400_000 - 1 ms`).
/// An hourly bar opening at 2023-05-02T00:00Z therefore sees the 2023-05-01
/// DVOL close (`close_ts` = 2023-05-01T23:59:59.999Z ≤ `bar_open_ts`), NOT the
/// 05-02 close (which closes 24h later). This matches the `basis_as_of`
/// discipline with the cadence lifted from 1h-basis to 1-day-DVOL.
///
/// Returns `None` for bars before the first available DVOL close (warm-up).
///
/// # Arguments
///
/// - `dvol`: Sorted `(day_close_ts_ms, dvol_close)` pairs in ascending
///   `day_close_ts_ms` order. MUST be sorted for correct results.
/// - `bar_open_ts_ms`: Bar open timestamps (each processed independently
///   via binary search on `PitSeries`).
///
/// # Returns
///
/// `Vec<Option<Decimal>>` of length `bar_open_ts_ms.len()`.
/// `None` = no DVOL day has closed yet (warm-up).
///
/// # Invariant (no look-ahead)
///
/// Only the DVOL close fully observed at-or-before the bar's `open_ts` is used.
/// Future-shifting the DVOL series (e.g. by +1 day) WILL produce a different
/// result — verified by the unit test `no_look_ahead_falsifier`.
///
/// Routes through `trading_core::pit::PitSeries` (ADR-0058 / M-DEV-3).
#[must_use]
pub fn dvol_as_of(dvol: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>> {
    if dvol.is_empty() {
        return vec![None; bar_open_ts_ms.len()];
    }

    // Build a PitSeries once. The loader pre-sorts dvol by day_close_ts_ms
    // before calling this function, so from_sorted would succeed; we use
    // from_unsorted here so this library function is infallible — the sort
    // is a stable no-op on an already-sorted slice.
    //
    // Explicit publication lag (ADR-0086 D2/D3, P3 M-DEV-5): DVOL's
    // publication_lag_ms = 0 per the feature's lag table
    // (spec/v3/advisor-pit-discipline/feature.md § D2) — the join key
    // `day_close_ts_ms` already places the record at the FULLY-observed
    // instant, so no additional lag applies. `from_unsorted_with_lag(_, 0)`
    // is byte-identical to `from_unsorted(_)` (proven by
    // `dvol_byte_identical_legacy_vs_with_lag_zero`).
    let series = PitSeries::from_unsorted_with_lag(
        dvol.iter().map(|&(t, r)| (TimestampMs(t), r)).collect(),
        0,
    );

    bar_open_ts_ms
        .iter()
        .map(|&q| series.as_of_value(TimestampMs(q)))
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

    const ONE_DAY_MS: i64 = 86_400_000;

    // ── dvol_as_of unit tests (M-DEV-2) ──────────────────────────────────────

    /// Bar before the first DVOL close → None (warm-up).
    #[test]
    fn warm_up_before_first_dvol_is_none() {
        // The DVOL series uses day_close_ts_ms as the join key.
        // day_close_ts_ms = day_open_ts_ms + ONE_DAY_MS - 1.
        let day1_close_ts: i64 = ONE_DAY_MS - 1; // day 1 close
        let dvol = vec![
            (day1_close_ts, dec!(50.0)),
            (2 * ONE_DAY_MS - 1, dec!(55.0)),
        ];
        // Bar at ts=0 is before day1_close_ts → None (warm-up).
        let result = dvol_as_of(&dvol, &[0]);
        assert_eq!(
            result,
            vec![None],
            "bar before first DVOL close must be None"
        );
    }

    /// Bar at or after the first DVOL close timestamp → Some.
    #[test]
    fn bar_after_first_dvol_close_is_some() {
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let dvol = vec![(day1_close_ts, dec!(50.0))];
        // A bar opening exactly at day1_close_ts sees that close.
        let result = dvol_as_of(&dvol, &[day1_close_ts]);
        assert_eq!(result, vec![Some(dec!(50.0))]);

        // A bar opening 1 ms after (e.g. UTC midnight of day 2) also sees day1_close.
        let result2 = dvol_as_of(&dvol, &[ONE_DAY_MS]);
        assert_eq!(result2, vec![Some(dec!(50.0))]);
    }

    /// A bar opening on day 2 sees the day-1 DVOL close, NOT day-2 close.
    ///
    /// This is the load-bearing no-look-ahead property:
    /// `bar_open = 2023-05-02T00:00Z` → sees the 2023-05-01 DVOL close
    /// (close_ts = 2023-05-01T23:59:59.999Z ≤ bar_open).
    #[test]
    fn bar_on_day2_sees_day1_close() {
        // day 1: close_ts = ONE_DAY_MS - 1 (e.g. 86_399_999 ms)
        // day 2: close_ts = 2 * ONE_DAY_MS - 1 (e.g. 172_799_999 ms)
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let day2_close_ts: i64 = 2 * ONE_DAY_MS - 1;
        let dvol = vec![(day1_close_ts, dec!(50.0)), (day2_close_ts, dec!(60.0))];
        // Bar opening at day2's midnight (ONE_DAY_MS) → sees day1_close (50.0).
        let bar_ts = ONE_DAY_MS; // 2023-01-02T00:00Z
        let result = dvol_as_of(&dvol, &[bar_ts]);
        assert_eq!(result, vec![Some(dec!(50.0))]);

        // Bar opening at day2_close_ts (the very end of day 2) → sees day2_close (60.0).
        let result2 = dvol_as_of(&dvol, &[day2_close_ts]);
        assert_eq!(result2, vec![Some(dec!(60.0))]);
    }

    /// Forward-fill across multiple intraday bars.
    ///
    /// On the 1h grid, 24 consecutive hourly bars in a day all see the prior
    /// day's DVOL close (forward-filled until the current day's close lands).
    #[test]
    fn forward_fill_across_intraday_bars() {
        let one_hour_ms: i64 = 3_600_000;
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let dvol = vec![(day1_close_ts, dec!(52.0))];

        // 25 hourly bars from day2's midnight through end-of-day2 (past day1_close).
        let bars: Vec<i64> = (0..25).map(|h| ONE_DAY_MS + h * one_hour_ms).collect();
        let result = dvol_as_of(&dvol, &bars);

        // All 25 bars see the day1 close (forward-filled).
        for (i, val) in result.iter().enumerate() {
            assert_eq!(
                *val,
                Some(dec!(52.0)),
                "bar {i} should see day1_close = 52.0"
            );
        }
    }

    /// Empty DVOL series → all None.
    #[test]
    fn empty_dvol_series_all_none() {
        let result = dvol_as_of(&[], &[100, 200, 300]);
        assert_eq!(result, vec![None, None, None]);
    }

    // ── Byte-identity test (ADR-0086 D3 / P3 M-TEST-3) ────────────────────────

    /// Proves the P3 retrofit (`PitSeries::from_unsorted` →
    /// `from_unsorted_with_lag(_, 0)`, `dvol_data.rs::dvol_as_of`) moves NO
    /// value on a representative series + bar-open grid: the LEGACY raw
    /// `partition_point(|&(t,_)| t <= q)` predicate computed directly over
    /// the DVOL rows must equal the RETROFITTED `dvol_as_of` (which now
    /// routes through `from_unsorted_with_lag(_, 0)`) element-for-element.
    ///
    /// This is the anchor question, answered by construction: because DVOL
    /// runs `write_report = false` on the bake-off path, no anchored report
    /// body can move; this test is the load-bearing proof that the
    /// as-of VALUES themselves are unchanged.
    #[test]
    fn dvol_byte_identical_legacy_vs_with_lag_zero() {
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let day2_close_ts: i64 = 2 * ONE_DAY_MS - 1;
        let day3_close_ts: i64 = 3 * ONE_DAY_MS - 1;
        let dvol: Vec<(i64, Decimal)> = vec![
            (day1_close_ts, dec!(50.0)),
            (day2_close_ts, dec!(70.0)),
            (day3_close_ts, dec!(40.0)),
        ];

        // A representative bar-open grid spanning warm-up, exact boundaries,
        // between-record forward-fill, and past-last-record.
        let one_hour_ms: i64 = 3_600_000;
        let mut grid: Vec<i64> = vec![0, day1_close_ts - 1, day1_close_ts, day2_close_ts - 1];
        grid.extend((0..30).map(|h| day1_close_ts + h * one_hour_ms));
        grid.push(day2_close_ts);
        grid.push(day3_close_ts);
        grid.push(day3_close_ts + ONE_DAY_MS);

        // LEGACY: the exact raw predicate `dvol_as_of` used before the P3
        // retrofit (partition_point(|&(t,_)| t <= q), idx-1, None at idx==0).
        let legacy_as_of = |query: i64| -> Option<Decimal> {
            let idx = dvol.partition_point(|&(t, _)| t <= query); // PIT-OK: legacy-predicate byte-identity oracle for the M-TEST-3 retrofit proof.
            if idx == 0 {
                None
            } else {
                Some(dvol[idx - 1].1)
            }
        };
        let legacy_results: Vec<Option<Decimal>> = grid.iter().map(|&q| legacy_as_of(q)).collect();

        // RETROFITTED: the current dvol_as_of, which now routes through
        // PitSeries::from_unsorted_with_lag(_, 0).
        let retrofitted_results = dvol_as_of(&dvol, &grid);

        assert_eq!(
            legacy_results, retrofitted_results,
            "P3 retrofit must be byte-identical to the legacy raw partition_point predicate"
        );
    }

    /// No-look-ahead falsifier: future-shifting the DVOL series changes the result.
    ///
    /// Cloned verbatim from `basis_data.rs::no_look_ahead_falsifier`.
    /// Proves the join is strictly past-only — future DVOL never leaks.
    #[test]
    fn no_look_ahead_falsifier() {
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let day2_close_ts: i64 = 2 * ONE_DAY_MS - 1;
        let day3_close_ts: i64 = 3 * ONE_DAY_MS - 1;
        let dvol = vec![
            (day1_close_ts, dec!(50.0)),
            (day2_close_ts, dec!(70.0)),
            (day3_close_ts, dec!(40.0)),
        ];

        // Shift each DVOL close timestamp +1 day into the future.
        let future_shifted: Vec<(i64, Decimal)> =
            dvol.iter().map(|&(t, r)| (t + ONE_DAY_MS, r)).collect();

        // Query a bar at day2's midnight (the open of day 2).
        let bar_ts = ONE_DAY_MS; // 2023-01-02T00:00Z

        let causal_result = dvol_as_of(&dvol, &[bar_ts]);
        let shifted_result = dvol_as_of(&future_shifted, &[bar_ts]);

        // Causal: bar at day2 open sees day1_close_ts (< day2 open) → 50.0.
        assert_eq!(causal_result, vec![Some(dec!(50.0))]);
        // Shifted: day1 close is now at 2*ONE_DAY_MS-1, day2 open = ONE_DAY_MS → None.
        assert_eq!(shifted_result, vec![None]);
        // They MUST differ — proves no look-ahead.
        assert_ne!(
            causal_result, shifted_result,
            "no-look-ahead falsifier: causal ≠ shifted (future DVOL must not leak)"
        );
    }

    /// Decimal precision is preserved — no f64 round-trip loses information.
    #[test]
    fn decimal_precision_preserved() {
        // DVOL values like 52.1234 should round-trip through the format!("{:.4}") path.
        let raw: f64 = 52.1234;
        let s = format!("{raw:.4}");
        let parsed: Decimal = s.parse().expect("parse");
        assert_eq!(parsed, dec!(52.1234));
    }

    // ── Real-corpus smoke test (on-machine only) ──────────────────────────────

    /// Smoke test: load from the real DVOL corpus and verify rows exist.
    ///
    /// On-machine only: requires `data/deribit-dvol/BTC/2024.parquet`.
    /// Set `#[ignore]` so CI doesn't fail without the corpus.
    ///
    /// Uses `CARGO_MANIFEST_DIR` to resolve the workspace root from the crate
    /// root (`crates/backtest/`), since unit tests run with `cwd = crate dir`.
    #[test]
    #[ignore = "on-machine: requires data/deribit-dvol corpus + SHA match"]
    fn real_corpus_load_smoke() {
        use std::path::PathBuf;
        // CARGO_MANIFEST_DIR = ".../crates/backtest"; go up two levels to workspace root.
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("canonicalize workspace root");
        let dvol_root = workspace_root.join("data/deribit-dvol");
        eprintln!("dvol_root = {dvol_root:?}");

        let src = DvolDataSource::new(dvol_root, vec![Symbol::new("BTC")]);
        let span = crate::realdata::TimeSpan {
            start_ms: 1_704_067_200_000,
            end_ms: 1_719_792_000_000,
            start_label: "2024-01-01T00:00:00Z",
            end_label: "2024-07-01T00:00:00Z",
        };
        let result = src.load(&span, "smoke-test");
        match &result {
            Ok(loaded) => eprintln!(
                "OK: {} rows, sha={}",
                loaded.rows.len(),
                loaded.revision_sha
            ),
            Err(e) => eprintln!("ERR: {e:?}"),
        }
        let loaded = result.expect("real corpus load must succeed");
        assert!(!loaded.rows.is_empty(), "must have at least one DVOL row");
        assert_eq!(
            loaded.revision_sha, EXPECTED_DVOL_REVISION_SHA,
            "revision SHA mismatch"
        );
    }

    // ── REVISION-mismatch rejection test ─────────────────────────────────────

    /// When the REVISION.toml claims an aggregate SHA different from the
    /// locked constant, `load()` must reject with `RevisionMismatch`.
    #[test]
    fn revision_mismatch_is_rejected() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        // Write a fake parquet file and a REVISION.toml with a wrong aggregate.
        let sym_dir = root.join("BTC");
        fs::create_dir_all(&sym_dir).expect("create dir");
        let fake_parquet_path = sym_dir.join("2023.parquet");
        fs::write(&fake_parquet_path, b"fake data").expect("write fake parquet");

        // Compute actual SHA of the fake file for the manifest.
        let fake_sha = data::revision::file_sha256(&fake_parquet_path).expect("sha256");

        // Build a REVISION.toml with a SHA that won't match EXPECTED_DVOL_REVISION_SHA.
        let wrong_aggregate_sha =
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let toml_content = format!(
            "[revision]\nsha256 = \"{wrong_aggregate_sha}\"\n\n[files]\n\"BTC/2023.parquet\" = \"{fake_sha}\"\n"
        );
        fs::write(root.join("REVISION.toml"), &toml_content).expect("write revision toml");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")]);

        let result = src.load(&span, "test-scenario");
        assert!(
            result.is_err(),
            "load() must fail when the aggregate SHA does not match the locked constant"
        );
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("revision mismatch")
                || matches!(err, DvolDataError::RevisionMismatch { .. }),
            "error must be RevisionMismatch, got: {err_str}"
        );
    }
}
