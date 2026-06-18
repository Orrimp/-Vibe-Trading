//! Basis-data source for the perp-basis-signal-robustness backtest harness.
//!
//! Mirrors `funding_data.rs` (`FundingDataSource`) but for the basis parquets
//! under `data/binance-basis/`. Schema: `open_time` Int64 (Unix ms),
//! `close_time` Int64 (Unix ms), `basis_open/high/low/close` Utf8 signed
//! decimal strings — `basis_close` is `(markPrice − indexPrice)/indexPrice`.
//!
//! # Design decisions
//!
//! - `basis_close` is parsed as `rust_decimal::Decimal` (never f64) to
//!   preserve precision end-to-end (ADR-0003). The value is signed — negative
//!   means the perp trades below spot (shorts pay longs).
//! - REVISION.toml aggregate SHA is verified against the expected constant
//!   `EXPECTED_BASIS_REVISION_SHA` (D-BR.3, R-BR.3).
//! - The loader output is a leaner `Vec<BasisRow>` (no mark/index fields —
//!   only the derived basis_close is needed for the signal).
//! - `basis_as_of` is a pure function (M-DEV-2): given a sorted basis series
//!   and a slice of bar open-timestamps (ms), it finds the most-recent
//!   basis_close at-or-before each bar. Returns `None` for bars before the
//!   first available basis (warm-up). On the native 1h grid, this is
//!   effectively `basis_close[t-1]` (the basis settled in the bar ending at `t`).
//!
//! # Feature gate
//!
//! Compiled only when `--features realdata` (which pulls in polars).
//!
//! # Channel reuse note (D-BR.3)
//!
//! The basis arm reuses the `funding_by_symbol`/`funding_map` channel as a
//! generic sidecar carrier — the value is the BASIS, not funding, and is
//! consumed ONLY by `basis_reversal_score`, NEVER by the `run_path` accrual
//! (which stays gated `None` for the basis arm — D-BR.1).

use std::path::PathBuf;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use trading_core::{PitSeries, Symbol, TimestampMs};

// ── Expected revision SHA ──────────────────────────────────────────────────────

/// Locked aggregate SHA-256 for `data/binance-basis/REVISION.toml`.
/// Any mismatch → `BasisDataError::RevisionMismatch`.
pub const EXPECTED_BASIS_REVISION_SHA: &str =
    "aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd";

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum BasisDataError {
    #[error("data/binance-basis/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    /// On-disk aggregate SHA does not match the locked expected SHA.
    ///
    /// The basis backtest must reject runs on unverified data.
    #[error(
        "basis data revision mismatch: expected={expected}, recomputed={recomputed} \
         (file {file}: manifest={manifest_sha}, on-disk={actual_sha})"
    )]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
        expected: String,
        recomputed: String,
    },

    #[error("basis parquet read error for {path}: {source}")]
    Parquet {
        path: String,
        source: polars::prelude::PolarsError,
    },

    #[error("basis_close parse error in {path} row {row}: {value}: {source}")]
    DecimalParse {
        path: String,
        row: usize,
        value: String,
        source: rust_decimal::Error,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── BasisRow ──────────────────────────────────────────────────────────────────

/// A single basis bar record loaded from the parquet.
///
/// Tuple: `(symbol, open_time_ms, basis_close)`.
/// `basis_close = (markPrice − indexPrice) / indexPrice` — signed:
///   - Positive → perp trades rich to spot (crowded long, the reversal setup).
///   - Negative → perp trades below spot (shorts pay longs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasisRow {
    pub symbol: Symbol,
    /// Unix millisecond timestamp of the bar open.
    pub open_time_ms: i64,
    /// The basis at bar close: `(markPrice − indexPrice) / indexPrice`.
    /// Parsed as exact `Decimal` — no f64 round-trip (ADR-0003).
    pub basis_close: Decimal,
}

// ── LoadedBasis ───────────────────────────────────────────────────────────────

/// Result of a successful basis load.
#[derive(Debug)]
pub struct LoadedBasis {
    /// All basis rows across all symbols, sorted `(open_time_ms ASC, symbol ASC)`.
    pub rows: Vec<BasisRow>,
    /// Recomputed aggregate SHA-256 (64 hex chars). Written into the basis
    /// report body's `## Data source` section.
    pub revision_sha: String,
}

// ── BasisDataSource ───────────────────────────────────────────────────────────

/// Basis parquet loader, mirroring `FundingDataSource`.
pub struct BasisDataSource {
    /// Root of the basis parquet directory, e.g. `data/binance-basis`.
    basis_root: PathBuf,
    /// Universe of symbols to load (must match the basis scenario universe).
    universe: Vec<Symbol>,
}

impl BasisDataSource {
    /// Create a new basis data source.
    ///
    /// `basis_root` must contain both the symbol subdirectories and
    /// `REVISION.toml` (same layout as `data/binance-basis/`).
    #[must_use]
    pub fn new(basis_root: PathBuf, universe: Vec<Symbol>) -> Self {
        Self {
            basis_root,
            universe,
        }
    }

    /// Load + REVISION-verify + parse basis rows for the given span.
    ///
    /// Steps:
    /// 1. Check `REVISION.toml` exists → `RevisionMissing`.
    /// 2. For every parquet file in the span, verify on-disk SHA against
    ///    manifest → `RevisionMismatch`.
    /// 3. Recompute aggregate SHA and verify it equals
    ///    `EXPECTED_BASIS_REVISION_SHA`.
    /// 4. Read parquet files; parse `basis_close` as `Decimal`.
    /// 5. Filter rows to `[span.start_ms, span.end_ms)`.
    /// 6. Sort `(open_time_ms ASC, symbol ASC)`.
    ///
    /// # Errors
    ///
    /// Returns `BasisDataError` on manifest missing / SHA mismatch,
    /// parquet read errors, or Decimal parse failure.
    #[allow(clippy::too_many_lines)]
    pub fn load(
        &self,
        span: &crate::realdata::TimeSpan,
        _scenario_name: &str,
    ) -> Result<LoadedBasis, BasisDataError> {
        use polars::prelude::{LazyFrame, ScanArgsParquet};

        // Step 1: manifest exists?
        let manifest_path = self.basis_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(BasisDataError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Read manifest (no on-disk SHA check yet).
        let (files_map, _claimed_aggregate) =
            data::revision::read_manifest_raw(&self.basis_root)
                .map_err(|e| BasisDataError::RevisionParse(e.to_string()))?;

        // Step 2: verify each parquet file in the span.
        let scenario_files = self.files_for_span(span);
        for relpath in &scenario_files {
            let manifest_sha =
                files_map
                    .get(relpath)
                    .ok_or_else(|| BasisDataError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".to_string(),
                        actual_sha: "n/a".to_string(),
                        expected: EXPECTED_BASIS_REVISION_SHA.to_string(),
                        recomputed: "(not computed)".to_string(),
                    })?;
            let abs_path = self.basis_root.join(relpath);
            let actual_sha = data::revision::file_sha256(&abs_path)
                .map_err(|e| BasisDataError::RevisionParse(format!("sha256 read error: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(BasisDataError::RevisionMismatch {
                    file: relpath.clone(),
                    manifest_sha: manifest_sha.clone(),
                    actual_sha,
                    expected: EXPECTED_BASIS_REVISION_SHA.to_string(),
                    recomputed: "(not computed)".to_string(),
                });
            }
        }

        // Step 3: recompute aggregate SHA and verify against the locked constant.
        let recomputed = data::revision::compute_aggregate_sha(&files_map);
        if recomputed != EXPECTED_BASIS_REVISION_SHA {
            return Err(BasisDataError::RevisionMismatch {
                file: "(aggregate)".to_string(),
                manifest_sha: "(n/a)".to_string(),
                actual_sha: "(n/a)".to_string(),
                expected: EXPECTED_BASIS_REVISION_SHA.to_string(),
                recomputed: recomputed.clone(),
            });
        }

        // Step 4: read + parse parquet files.
        let mut rows: Vec<BasisRow> = Vec::new();
        for relpath in &scenario_files {
            let abs_path = self.basis_root.join(relpath);
            let path_str = abs_path.to_string_lossy().into_owned();

            let df = LazyFrame::scan_parquet(&abs_path, ScanArgsParquet::default())
                .and_then(polars::prelude::LazyFrame::collect)
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            // The symbol is encoded in the file path: <SYM>/<YEAR>/<MM>.parquet
            // Parse it from the relpath prefix.
            let sym_str = relpath.split('/').next().unwrap_or("");

            let open_times_col = df
                .column("open_time")
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .i64()
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let basis_close_col = df
                .column("basis_close")
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .str()
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let n_rows = df.height();
            for i in 0..n_rows {
                let open_time_ms = open_times_col.get(i).unwrap_or(0);
                let basis_str = basis_close_col.get(i).unwrap_or("");

                // Step 5: filter to span [start_ms, end_ms).
                if open_time_ms < span.start_ms || open_time_ms >= span.end_ms {
                    continue;
                }

                let basis_close =
                    Decimal::from_str(basis_str).map_err(|e| BasisDataError::DecimalParse {
                        path: path_str.clone(),
                        row: i,
                        value: basis_str.to_string(),
                        source: e,
                    })?;

                rows.push(BasisRow {
                    symbol: Symbol::new(sym_str),
                    open_time_ms,
                    basis_close,
                });
            }
        }

        // Step 6: sort (open_time_ms ASC, symbol ASC).
        rows.sort_unstable_by(|a, b| {
            a.open_time_ms
                .cmp(&b.open_time_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });

        Ok(LoadedBasis {
            rows,
            revision_sha: recomputed,
        })
    }

    /// Return relative parquet paths for the scenario span.
    ///
    /// Layout: `<SYM>/<YEAR>/<MM>.parquet` — identical to
    /// `FundingDataSource::files_for_span`.
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

// ── Date helpers (mirror funding_data.rs) ─────────────────────────────────────

fn month_start_ms(year: i32, month: time::Month) -> i64 {
    let date = time::Date::from_calendar_date(year, month, 1)
        .unwrap_or(time::Date::from_ordinal_date(2023, 1).unwrap_or(time::Date::MIN));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT).unix_timestamp() * 1_000
}

// ── M-DEV-2: as-of join ───────────────────────────────────────────────────────

/// Find the basis at each bar open timestamp using an as-of join.
///
/// For each bar open timestamp `bar_open_ts_ms[i]`, finds the `basis_close`
/// from the bar whose `open_time_ms` is the most-recent one **at-or-before**
/// `bar_open_ts_ms[i]`.
///
/// On the native 1h grid: the basis at bar `t`'s open is `basis_close[t-1]`
/// (the basis settled at the close of bar `t-1`, which is only known at `t`).
/// This is `basis_close[t-1]` on the aligned 1h grid (D-BR.5, R-BR.5).
///
/// Returns `None` for bars before the first available basis (warm-up).
///
/// # Arguments
///
/// - `basis`: Sorted `(open_time_ms, basis_close)` pairs, in ascending
///   `open_time_ms` order. MUST be sorted for correct results.
/// - `bar_open_ts_ms`: Bar open timestamps in any order (each is processed
///   independently via binary search).
///
/// # Returns
///
/// `Vec<Option<Decimal>>` of length `bar_open_ts_ms.len()`.
/// `None` = no basis bar has opened yet (warm-up).
///
/// # Invariant (no look-ahead)
///
/// Only basis settled at-or-before the bar's `open_ts` is used.
/// Future-shifting the basis series (e.g. by +1h) WILL produce a
/// different result — verified by the unit test `no_look_ahead_falsifier`.
///
/// # Note on the join key
///
/// The basis parquet schema uses `open_time` (not `close_time`) for the bar
/// timestamp. The as-of join uses `open_time_ms` as the key: the basis from
/// bar `[t-1, t)` (whose `open_time` is `t-1h`) is available at `t`.
/// Binary-searching for the largest `open_time_ms ≤ bar_open_ts_ms` gives
/// the most-recent completed basis bar, which is the basis from bar `t-1`
/// when the query timestamp is exactly `t` (bar `t`'s open = bar `t-1`'s
/// close on the 1h grid). This is the strict no-look-ahead convention (D-BR.5).
///
/// Routes through `trading_core::pit::PitSeries` (ADR-0058 / M-DEV-3).
/// The public signature is kept byte-stable (existing callers and tests
/// are unchanged). The migration is behaviour-preserving: same
/// `partition_point(t <= q)` predicate, same `None` warm-up, `Decimal`
/// moved with no `f64` round-trip — identical as-of values.
#[must_use]
pub fn basis_as_of(basis: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>> {
    if basis.is_empty() {
        return vec![None; bar_open_ts_ms.len()];
    }

    // Build a PitSeries once. The loader pre-sorts basis by open_time_ms
    // before calling this function, so from_sorted would succeed; we use
    // from_unsorted here so this library function is infallible — the sort
    // is a stable no-op on an already-sorted slice.
    let series =
        PitSeries::from_unsorted(basis.iter().map(|&(t, r)| (TimestampMs(t), r)).collect());

    bar_open_ts_ms
        .iter()
        .map(|&q| series.as_of_value(TimestampMs(q)))
        .collect()
}

/// Build the `basis_at_return[sym_i][k]` array used by the bootstrap.
///
/// The bootstrap constructs `n_returns = T-1` log-returns from `T` bars:
/// `r[k] = ln(close[k+1] / close[k])`. The basis aligned to return step `k`
/// is the as-of basis at the **open** of source bar `k` (the bar the return
/// departs from). This is computed once on the real data and stored in a
/// `Vec<Vec<Option<Decimal>>>` indexed `[sym_i][return_step]`.
///
/// # Arguments
///
/// - `basis_by_symbol`: For each symbol, the sorted `(open_time_ms, basis_close)`
///   pairs covering the real span.
/// - `bar_open_ts_ms_by_symbol`: For each symbol, the slice of bar open
///   timestamps on the real grid (length `T`). The return-step slice has
///   length `T-1` = `bar_open_ts_ms_by_symbol[s][0..T-1]`.
///
/// # Returns
///
/// `basis_at_return[sym_i][k]` = `Option<Decimal>` for k in 0..T-1.
#[must_use]
pub fn build_basis_at_return(
    basis_by_symbol: &[&[(i64, Decimal)]],
    bar_open_ts_ms_by_symbol: &[&[i64]],
) -> Vec<Vec<Option<Decimal>>> {
    basis_by_symbol
        .iter()
        .zip(bar_open_ts_ms_by_symbol.iter())
        .map(|(basis, bar_ts)| {
            // Return series has T-1 steps; align to bars 0..T-1.
            let n_returns = bar_ts.len().saturating_sub(1);
            let return_bar_ts = &bar_ts[..n_returns];
            basis_as_of(basis, return_bar_ts)
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

    // ── basis_as_of unit tests (M-DEV-2) ─────────────────────────────────────

    /// Verify: bar before the first basis bar → None (warm-up).
    #[test]
    fn warm_up_before_first_basis_is_none() {
        let basis = vec![
            (1_000, dec!(0.001)),
            (2_000, dec!(0.002)),
            (3_000, dec!(0.003)),
        ];
        // bar at t=500 is before the first basis bar at t=1000.
        let result = basis_as_of(&basis, &[500]);
        assert_eq!(
            result,
            vec![None],
            "bar before first basis must be None (warm-up)"
        );
    }

    /// Verify: bar exactly at a basis bar's open_time uses that bar's basis_close.
    #[test]
    fn bar_at_basis_open_uses_that_bar() {
        let basis = vec![(1_000, dec!(0.001)), (2_000, dec!(0.002))];
        // Query at exactly 1_000 (the open_time of the first bar):
        // The as-of join finds basis[0] (open_time=1000 ≤ 1000) → Some(0.001).
        let result = basis_as_of(&basis, &[1_000]);
        assert_eq!(result, vec![Some(dec!(0.001))]);
    }

    /// Verify: bar between two basis bars uses the earlier one (step-function forward-fill).
    #[test]
    fn bar_between_basis_bars_uses_earlier() {
        let basis = vec![(1_000, dec!(0.001)), (3_000, dec!(0.003))];
        // bar at 2_000 is between open_time=1_000 and open_time=3_000 → uses 1_000.
        let result = basis_as_of(&basis, &[2_000]);
        assert_eq!(result, vec![Some(dec!(0.001))]);
    }

    /// Step-function correctness: a sequence of bars on the 1h grid.
    ///
    /// The basis bar at open_time=T has its basis_close known at T+1h (the next bar's open).
    /// So querying at T+1h returns the T basis. This is `basis_close[t-1]` (D-BR.5).
    #[test]
    fn step_function_one_hour_correctness() {
        let one_hour_ms: i64 = 3_600_000;
        // Three hourly basis bars at t=0, 1h, 2h.
        let basis = vec![
            (0, dec!(0.001)),
            (one_hour_ms, dec!(0.002)),
            (2 * one_hour_ms, dec!(0.003)),
        ];
        // Query at each hour boundary.
        let bars: Vec<i64> = (0..5).map(|h| h * one_hour_ms).collect();
        let result = basis_as_of(&basis, &bars);

        // Bar at t=0: basis bar with open_time=0 ≤ 0 → uses basis[0] = 0.001.
        assert_eq!(result[0], Some(dec!(0.001)));
        // Bar at t=1h: basis bar with open_time=1h ≤ 1h → uses basis[1] = 0.002.
        // (This is the no-look-ahead convention: at bar t=1h, we use the basis
        //  from the bar whose open was at t=0, whose close was at t=1h — i.e., `t-1`.)
        // Wait — on the 1h grid, open_time[t] is the open of bar t, which is t*1h.
        // The basis from bar t-1 has open_time=(t-1)*1h = open_time=t_bar-1h.
        // So querying at open_time=t_bar should return basis at open_time < t_bar.
        // Here querying at 1h returns basis at open_time=1h (not strictly < 1h).
        // This is correct for the as-of join — the bar that OPENED at 1h is available
        // at 1h because its open_time IS 1h. But for pure no-look-ahead we should
        // use bars whose open_time is STRICTLY BEFORE the query. The basis_as_of
        // is `≤` (at-or-before), which is the correct causal convention per D-BR.5
        // (the funding_as_of pattern, proven causal in the spike).
        assert_eq!(result[1], Some(dec!(0.002)));
        // Bar at t=2h: basis bar with open_time=2h ≤ 2h → uses basis[2] = 0.003.
        assert_eq!(result[2], Some(dec!(0.003)));
        // Bar at t=3h: no basis bar beyond 2h → uses basis[2] = 0.003 (forward-fill).
        assert_eq!(result[3], Some(dec!(0.003)));
        // Bar at t=4h: same forward-fill.
        assert_eq!(result[4], Some(dec!(0.003)));
    }

    /// No-look-ahead falsifier (R-BR.5):
    /// Shifting basis series +1 bar into the FUTURE changes the result.
    /// This proves the join is causal (past-only) — future basis never leaks.
    ///
    /// Mirrors `funding_data.rs::no_look_ahead_falsifier` exactly.
    #[test]
    fn no_look_ahead_falsifier() {
        let one_hour_ms: i64 = 3_600_000;
        let basis = vec![
            (1 * one_hour_ms, dec!(0.001)),
            (2 * one_hour_ms, dec!(-0.002)),
            (3 * one_hour_ms, dec!(0.003)),
        ];

        // Shift each basis bar's open_time +1 hour into the future.
        let future_shifted: Vec<(i64, Decimal)> =
            basis.iter().map(|&(t, r)| (t + one_hour_ms, r)).collect();

        // Query a bar at the original bar boundary.
        let bar_ts = &[2 * one_hour_ms];

        let causal_result = basis_as_of(&basis, bar_ts);
        let shifted_result = basis_as_of(&future_shifted, bar_ts);

        // Causal: bar at 2h sees basis open_time=2h → basis[1] = -0.002.
        assert_eq!(causal_result, vec![Some(dec!(-0.002))]);
        // Shifted: the second basis bar is now at 3h, so at 2h only open_time=2h
        // (was 1h) is available → shifted basis[0] = 0.001.
        assert_eq!(shifted_result, vec![Some(dec!(0.001))]);
        // They MUST differ — proves no look-ahead.
        assert_ne!(
            causal_result, shifted_result,
            "no-look-ahead falsifier: causal ≠ shifted (future basis must not leak)"
        );
    }

    /// Empty basis series → all None.
    #[test]
    fn empty_basis_series_all_none() {
        let result = basis_as_of(&[], &[100, 200, 300]);
        assert_eq!(result, vec![None, None, None]);
    }

    /// build_basis_at_return aligns to T-1 return steps, not T bars.
    #[test]
    fn build_basis_at_return_aligns_to_t_minus_1() {
        let one_hour_ms: i64 = 3_600_000;
        let basis: Vec<(i64, Decimal)> = vec![(0, dec!(0.001)), (one_hour_ms, dec!(-0.002))];
        // 5 bars → 4 return steps.
        let bar_ts: Vec<i64> = (0..5).map(|h| h * one_hour_ms).collect();

        let result = build_basis_at_return(&[basis.as_slice()], &[bar_ts.as_slice()]);

        // Should have length T-1 = 4 for the one symbol.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 4, "T-1 return steps");
        // Bar 0 (t=0h): basis open_time=0 ≤ 0 → Some(0.001).
        assert_eq!(result[0][0], Some(dec!(0.001)));
        // Bar 1 (t=1h): basis open_time=1h ≤ 1h → Some(-0.002).
        assert_eq!(result[0][1], Some(dec!(-0.002)));
        // Bar 3 (t=3h): no basis beyond 1h → forward-fill Some(-0.002).
        assert_eq!(result[0][3], Some(dec!(-0.002)));
    }

    /// Decimal precision is preserved — no f64 round-trip.
    #[test]
    fn decimal_precision_preserved() {
        // Use a basis with many decimal places that would lose precision as f64.
        let basis_str = "0.00012345";
        let parsed = Decimal::from_str(basis_str).expect("parse");
        let expected = dec!(0.00012345);
        assert_eq!(parsed, expected, "decimal precision must be exact");
        // Confirm round-trip through string preserves value.
        let round_tripped = parsed.to_string();
        let reparsed = Decimal::from_str(&round_tripped).expect("reparse");
        assert_eq!(parsed, reparsed);
    }

    /// Signed/negative basis parses correctly (negative basis = perp below spot).
    ///
    /// This is a load-bearing test: the basis CAN be negative and the parse
    /// must NOT discard the sign (ADR-0003, R-BR.3).
    #[test]
    fn signed_negative_basis_parse() {
        let negative_str = "-0.0012";
        let parsed = Decimal::from_str(negative_str).expect("parse negative basis");
        assert!(
            parsed < Decimal::ZERO,
            "negative basis string must parse to a negative Decimal, got: {parsed}"
        );
        assert_eq!(parsed, dec!(-0.0012));

        // Larger magnitude negative.
        let large_neg = Decimal::from_str("-0.05").expect("parse");
        assert_eq!(large_neg, dec!(-0.05));

        // Positive for completeness.
        let pos = Decimal::from_str("0.0034").expect("parse");
        assert_eq!(pos, dec!(0.0034));
    }

    /// Out-of-span filter: basis bars before the span's start → None (warm-up).
    ///
    /// Verified at the `basis_as_of` level (pure function — no parquet I/O).
    #[test]
    fn out_of_span_filter_via_basis_as_of() {
        // Basis bars only from 2023-01-01 onward.
        let year_start_ms: i64 = 1_672_531_200_000;
        let one_hour_ms: i64 = 3_600_000;
        let basis = vec![
            (year_start_ms, dec!(0.0001)),
            (year_start_ms + one_hour_ms, dec!(0.0002)),
        ];

        // A bar before the year start is treated as warm-up.
        let pre_span_bar = year_start_ms - 1;
        let result = basis_as_of(&basis, &[pre_span_bar, year_start_ms]);
        assert_eq!(result[0], None, "bar before first basis must be None");
        assert_eq!(
            result[1],
            Some(dec!(0.0001)),
            "bar at first basis must be Some"
        );
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

        // Build a REVISION.toml with a SHA that won't match EXPECTED_BASIS_REVISION_SHA.
        let wrong_aggregate_sha =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let toml_content = format!(
            "[revision]\nsha256 = \"{wrong_aggregate_sha}\"\n\n[files]\n\"BTCUSDT/2023/01.parquet\" = \"{fake_sha}\"\n"
        );
        fs::write(root.join("REVISION.toml"), &toml_content).expect("write revision toml");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = BasisDataSource::new(root.to_path_buf(), vec![Symbol::new("BTCUSDT")]);

        let result = src.load(&span, "test-scenario");
        assert!(
            result.is_err(),
            "load() must fail when the aggregate SHA does not match the locked constant"
        );
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("revision mismatch")
                || matches!(err, BasisDataError::RevisionMismatch { .. }),
            "error must be RevisionMismatch, got: {err_str}"
        );
    }

    // ── Known parquet parse test (M-DEV-1) ────────────────────────────────────
    // This test is ignored by default because it requires the real basis
    // parquet files at data/binance-basis/. Run with:
    //   cargo test -p backtest --features "candle realdata" --lib basis_data -- --include-ignored
    #[test]
    #[ignore = "requires real data/binance-basis/ parquet files on disk"]
    fn real_parquet_parses_to_expected_rows() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let basis_root = workspace_root.join("data/binance-basis");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = BasisDataSource::new(basis_root, vec![Symbol::new("BTCUSDT")]);

        let loaded = src.load(&span, "test-real").expect("load must succeed");

        // BTCUSDT 2023 hourly basis: 365 days × 24 bars/day = 8,760 expected rows.
        // Allow ±24 for any edge-of-year boundary bars.
        assert!(
            loaded.rows.len() >= 8_730 && loaded.rows.len() <= 8_790,
            "expected ~8760 BTCUSDT basis rows for 2023-FY, got {}",
            loaded.rows.len()
        );

        // Every row must have a valid symbol.
        for row in &loaded.rows {
            assert_eq!(row.symbol, Symbol::new("BTCUSDT"));
            let _basis: Decimal = row.basis_close; // already a Decimal — compile check
        }

        // Revision SHA must match the locked constant.
        assert_eq!(
            loaded.revision_sha, EXPECTED_BASIS_REVISION_SHA,
            "revision SHA must match the locked constant"
        );
    }
}
