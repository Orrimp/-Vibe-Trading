//! Per-asset daily volume retrieval from the Binance parquet feed.
//!
//! # Design (v5-latency-slippage-sim v0.5.0 — ADR-0043 § Changelog 2026-05-29)
//!
//! Implements the R3 Option-A decision from architect M-T1: walk the existing
//! parquet feed (no new on-disk artifact) to compute a trailing daily-average
//! volume in USD. Deterministic given the parquet revision SHA in
//! `data/binance/REVISION.toml`.
//!
//! ## API
//!
//! - [`daily_volume_usd_trailing`] — per-asset query.
//! - [`universe_avg_daily_volume_usd_trailing`] — arithmetic mean over a
//!   symbol universe; used for Q3=(b) synthetic-scenario universe-avg V.
//!
//! ## Caching
//!
//! An in-process `Mutex<HashMap>` cache keyed on `(symbol, end_date, lookback_days)`
//! avoids repeat parquet reads across scenario calls. One read per scenario per
//! symbol (low contention; mutex is fine per D-T1.10 A3).
//!
//! ## Volume formula
//!
//! The parquet feed carries `volume` (base-asset quantity) and `close` (USD price)
//! as string columns. The USD notional per bar is `volume × close`. The daily
//! volume proxy is the arithmetic mean of `Σ(volume × close)` per UTC day over
//! the lookback window `[end_date - lookback_days, end_date)`.
//! This is the Kissell 2014 ch. 3 § "Volume-based impact" canonical form for
//! quote-asset-unavailable venues.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use polars::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use thiserror::Error;
use time::Date;
use trading_core::Symbol;

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DailyVolumeError {
    #[error("no parquet files found for symbol {symbol} in {parquet_root}")]
    SymbolNotFound {
        symbol: String,
        parquet_root: String,
    },
    #[error("polars error reading parquet for {symbol}: {source}")]
    Polars {
        symbol: String,
        #[source]
        source: PolarsError,
    },
    #[error("insufficient coverage for {symbol}: only {actual_days} days in [{start}, {end})")]
    InsufficientCoverage {
        symbol: String,
        actual_days: usize,
        start: Date,
        end: Date,
    },
    #[error("parse error for {symbol} column {column}: {msg}")]
    Parse {
        symbol: String,
        column: String,
        msg: String,
    },
}

// ── In-process cache ──────────────────────────────────────────────────────────

/// Cache key: (symbol, end_date_ordinal, lookback_days).
type CacheKey = (String, i32, u16);

/// Global in-process cache.
/// Keyed on `(symbol_str, end_date_ordinal, lookback_days)` → `Decimal` USD daily volume.
static CACHE: std::sync::OnceLock<Mutex<HashMap<CacheKey, Decimal>>> = std::sync::OnceLock::new();

fn cache() -> &'static Mutex<HashMap<CacheKey, Decimal>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── Per-asset query ────────────────────────────────────────────────────────────

/// Mean daily traded volume in USD over the trailing N days.
///
/// Computed as the arithmetic mean of `Σ(volume × close)` per UTC day
/// over the closed-open window `[end_date - lookback_days, end_date)`.
///
/// # Determinism
///
/// Pure function of (parquet revision SHA, symbol, end_date, lookback_days).
/// Cached in-process.
///
/// # Errors
///
/// - [`DailyVolumeError::SymbolNotFound`] if no parquet files exist for symbol.
/// - [`DailyVolumeError::InsufficientCoverage`] if < 50% of expected days have data.
/// - [`DailyVolumeError::Polars`] on parquet read failure.
/// - [`DailyVolumeError::Parse`] on string-column parse failure.
pub fn daily_volume_usd_trailing(
    parquet_root: &Path,
    symbol: &Symbol,
    end_date: Date,
    lookback_days: u16,
) -> Result<Decimal, DailyVolumeError> {
    let sym_str = symbol.0.to_string();
    let key: CacheKey = (sym_str.clone(), end_date.to_julian_day(), lookback_days);

    // Check cache first.
    {
        let guard = cache().lock().expect("cache lock poisoned");
        if let Some(&cached) = guard.get(&key) {
            return Ok(cached);
        }
    }

    // Compute from parquet.
    let result = compute_daily_volume_usd(parquet_root, symbol, end_date, lookback_days)?;

    // Store in cache.
    {
        let mut guard = cache().lock().expect("cache lock poisoned");
        guard.insert(key, result);
    }

    Ok(result)
}

/// Arithmetic mean of [`daily_volume_usd_trailing`] over a universe of symbols.
///
/// Used for Q3=(b) synthetic-scenario universe-average V: applies the sqrt market
/// impact model to synthetic scenarios by proxying V as the mean of real-Binance ADV.
///
/// Returns an error if any symbol in the universe fails to load. If a symbol is
/// not found (no parquet data), it is skipped silently (logged at warn level).
///
/// # Errors
///
/// Returns the first non-`SymbolNotFound` error encountered. `SymbolNotFound`
/// is treated as a soft skip (the symbol is excluded from the mean).
pub fn universe_avg_daily_volume_usd_trailing(
    parquet_root: &Path,
    universe: &[Symbol],
    end_date: Date,
    lookback_days: u16,
) -> Result<Decimal, DailyVolumeError> {
    let mut total = Decimal::ZERO;
    let mut count = 0usize;

    for sym in universe {
        match daily_volume_usd_trailing(parquet_root, sym, end_date, lookback_days) {
            Ok(vol) => {
                total += vol;
                count += 1;
            }
            Err(DailyVolumeError::SymbolNotFound { .. }) => {
                // Soft skip: symbol not present in parquet feed.
                tracing::warn!(
                    symbol = %sym.0,
                    "universe_avg_daily_volume_usd_trailing: symbol not found, skipping"
                );
            }
            Err(e) => return Err(e),
        }
    }

    if count == 0 {
        // No symbols available — return ZERO (triggers edge-case path in sqrt model).
        tracing::warn!("universe_avg_daily_volume_usd_trailing: no symbols loaded, returning ZERO");
        return Ok(Decimal::ZERO);
    }

    Ok(total / Decimal::from(count))
}

// ── Internal computation ──────────────────────────────────────────────────────

/// Compute mean daily USD volume from parquet.
///
/// Walks `parquet_root/<symbol>/<year>/<month>.parquet` files, filters by
/// `open_time` to `[start, end)` UTC window, aggregates `volume × close` per
/// UTC day, returns the arithmetic mean.
fn compute_daily_volume_usd(
    parquet_root: &Path,
    symbol: &Symbol,
    end_date: Date,
    lookback_days: u16,
) -> Result<Decimal, DailyVolumeError> {
    use time::Duration;

    let sym_str = symbol.0.as_str();
    let start_date = end_date - Duration::days(i64::from(lookback_days));

    // Collect parquet files for this symbol.
    let sym_dir = parquet_root.join(sym_str);
    if !sym_dir.exists() {
        return Err(DailyVolumeError::SymbolNotFound {
            symbol: sym_str.to_string(),
            parquet_root: parquet_root.display().to_string(),
        });
    }

    let files = collect_parquet_files(&sym_dir);
    if files.is_empty() {
        return Err(DailyVolumeError::SymbolNotFound {
            symbol: sym_str.to_string(),
            parquet_root: parquet_root.display().to_string(),
        });
    }

    // Unix millis for start and end boundaries.
    let start_ms = date_to_unix_millis(start_date);
    let end_ms = date_to_unix_millis(end_date);

    // Accumulate daily sums: HashMap<UTC-day-ordinal, Decimal>.
    let mut daily_sums: HashMap<i32, Decimal> = HashMap::new();

    for file in &files {
        // Load the parquet file.
        let df = LazyFrame::scan_parquet(file, ScanArgsParquet::default())
            .map_err(|e| DailyVolumeError::Polars {
                symbol: sym_str.to_string(),
                source: e,
            })?
            .select([col("open_time"), col("volume"), col("close")])
            .filter(
                col("open_time")
                    .gt_eq(lit(start_ms))
                    .and(col("open_time").lt(lit(end_ms))),
            )
            .collect()
            .map_err(|e| DailyVolumeError::Polars {
                symbol: sym_str.to_string(),
                source: e,
            })?;

        if df.is_empty() {
            continue;
        }

        let open_times =
            df.column("open_time")
                .and_then(|s| s.i64())
                .map_err(|e| DailyVolumeError::Polars {
                    symbol: sym_str.to_string(),
                    source: e,
                })?;
        let volumes =
            df.column("volume")
                .and_then(|s| s.str())
                .map_err(|e| DailyVolumeError::Polars {
                    symbol: sym_str.to_string(),
                    source: e,
                })?;
        let closes =
            df.column("close")
                .and_then(|s| s.str())
                .map_err(|e| DailyVolumeError::Polars {
                    symbol: sym_str.to_string(),
                    source: e,
                })?;

        let n = df.height();
        for i in 0..n {
            let ts_ms = open_times.get(i).unwrap_or(0);
            let day_ord = unix_millis_to_day_ordinal(ts_ms);

            let vol_str = volumes.get(i).ok_or_else(|| DailyVolumeError::Parse {
                symbol: sym_str.to_string(),
                column: "volume".to_string(),
                msg: format!("null at row {i}"),
            })?;
            let close_str = closes.get(i).ok_or_else(|| DailyVolumeError::Parse {
                symbol: sym_str.to_string(),
                column: "close".to_string(),
                msg: format!("null at row {i}"),
            })?;

            let vol = Decimal::from_str(vol_str.trim()).map_err(|e| DailyVolumeError::Parse {
                symbol: sym_str.to_string(),
                column: "volume".to_string(),
                msg: e.to_string(),
            })?;
            let close_price =
                Decimal::from_str(close_str.trim()).map_err(|e| DailyVolumeError::Parse {
                    symbol: sym_str.to_string(),
                    column: "close".to_string(),
                    msg: e.to_string(),
                })?;

            *daily_sums.entry(day_ord).or_insert(Decimal::ZERO) += vol * close_price;
        }
    }

    if daily_sums.is_empty() {
        // No data in range → return ZERO (treated as V=0 edge case in sqrt model).
        tracing::warn!(
            symbol = sym_str,
            start = %start_date,
            end = %end_date,
            lookback_days,
            "daily_volume_usd_trailing: no bars in window, returning ZERO"
        );
        return Ok(Decimal::ZERO);
    }

    // Arithmetic mean over populated days (not padded to lookback_days — sparse
    // windows use actual bar count so a 30-day window with 28 trading days doesn't
    // inflate the denominator).
    let n_days = daily_sums.len();
    let total: Decimal = daily_sums.values().sum();
    let mean = total / Decimal::from(n_days);

    tracing::debug!(
        symbol = sym_str,
        start = %start_date,
        end = %end_date,
        lookback_days,
        n_days,
        mean_usd = %mean,
        "daily_volume_usd_trailing computed"
    );

    Ok(mean)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collect all `.parquet` files under a symbol directory (recursive into year subdirs).
fn collect_parquet_files(sym_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(sym_dir)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|entry| {
            let p = entry.path();
            if p.is_dir() {
                std::fs::read_dir(&p)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| e.path())
                    .filter(|ep| ep.extension().is_some_and(|x| x == "parquet"))
                    .collect::<Vec<_>>()
            } else if p.extension().is_some_and(|x| x == "parquet") {
                vec![p]
            } else {
                vec![]
            }
        })
        .collect();
    files.sort();
    files
}

/// Convert a `time::Date` to Unix milliseconds at midnight UTC.
fn date_to_unix_millis(date: Date) -> i64 {
    use time::OffsetDateTime;
    let odt = OffsetDateTime::new_utc(date, time::Time::MIDNIGHT);
    let nanos: i128 = odt.unix_timestamp_nanos();
    // nanos / 1_000_000 → millis. Safe: i128 → i64 because the epoch-relative
    // millis for dates 2000-2100 fit in i64 (max ~4.6e18 ns = 4.6e15 ms << i64::MAX).
    (nanos / 1_000_000_i128) as i64
}

/// Convert Unix milliseconds to a Julian day ordinal (stable, monotonic day key).
fn unix_millis_to_day_ordinal(ms: i64) -> i32 {
    // ms / (24 * 3600 * 1000) = ms / 86_400_000
    // Floor division: Rust integer division truncates toward zero, but ms >= 0
    // for all 2023/2024 data, so this is equivalent to floor.
    // The value is at most 2^31 days / 86400000 ms/day ≈ ±24855 years — safe for i32.
    // ms >= 0 for all 2023/2024 data, so truncation toward zero = floor.
    let days = ms / 86_400_000_i64;
    days as i32
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use time::macros::date;

    /// Universe-avg with empty slice returns ZERO without error.
    #[test]
    fn universe_avg_empty_universe_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let result =
            universe_avg_daily_volume_usd_trailing(tmp.path(), &[], date!(2024 - 01 - 01), 90);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::ZERO);
    }

    /// Symbol not found returns ZERO (soft-skip in universe-avg).
    #[test]
    fn missing_symbol_returns_error_from_per_asset() {
        let tmp = tempfile::tempdir().unwrap();
        let sym = Symbol::new("NONEXISTENT");
        let result = daily_volume_usd_trailing(tmp.path(), &sym, date!(2024 - 01 - 01), 90);
        assert!(result.is_err(), "missing symbol must return error");
        matches!(result.unwrap_err(), DailyVolumeError::SymbolNotFound { .. });
    }

    /// Universe-avg skips missing symbols (returns ZERO when all missing).
    #[test]
    fn universe_avg_all_missing_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let universe = vec![Symbol::new("FAKE1"), Symbol::new("FAKE2")];
        let result = universe_avg_daily_volume_usd_trailing(
            tmp.path(),
            &universe,
            date!(2024 - 01 - 01),
            90,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::ZERO, "all-missing → ZERO");
    }

    /// date_to_unix_millis: 2023-01-01 = known value.
    #[test]
    fn date_to_unix_millis_known_value() {
        // 2023-01-01 00:00:00 UTC = 1672531200000 ms
        let ms = date_to_unix_millis(date!(2023 - 01 - 01));
        assert_eq!(ms, 1_672_531_200_000_i64, "2023-01-01 UTC ms mismatch");
    }

    /// unix_millis_to_day_ordinal: consistent bucketing.
    #[test]
    fn unix_millis_day_ordinal_same_day() {
        // 2023-01-01 00:00:00 UTC = 1672531200000
        // 2023-01-01 23:59:59 UTC ≈ 1672531200000 + 86399000 = 1672617599000
        let start_ms = 1_672_531_200_000_i64;
        let end_ms = 1_672_617_599_000_i64;
        assert_eq!(
            unix_millis_to_day_ordinal(start_ms),
            unix_millis_to_day_ordinal(end_ms),
            "start and end of same day must map to same ordinal"
        );
    }
}
