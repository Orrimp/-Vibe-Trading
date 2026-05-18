//! Real-Binance-data bar source for the backtest harness.
//!
//! This module is the entry point for loading real Binance hourly OHLCV from
//! `data/binance/` into the backtest loop. It wraps `data::ReplayFeed::merge_symbols()`
//! and enforces the REVISION.toml data-integrity pin described in ADR-0032 § 2.
//!
//! # Feature gate
//!
//! Compiled only when `--features realdata` is passed to cargo. The four
//! `-realdata` scenario rows in `main.rs::Scenario::from_name` are gated
//! behind `#[cfg(feature = "realdata")]` so neither the scenarios nor this
//! module appear in the default build.
//!
//! # Usage
//!
//! ```rust,ignore
//! let src = RealDataBarSource::new(PathBuf::from("data/binance"), universe);
//! let loaded = src.load(span, expected_total_bars, "my-scenario")?;
//! // loaded.bars: Vec<Bar>, k-way merged (open_ts ASC, symbol ASC)
//! // loaded.revision_sha: 64-char hex, recomputed aggregate SHA
//! ```

use std::path::PathBuf;

use time::OffsetDateTime;
use trading_core::{Bar, FeedError, Symbol, Timeframe};

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum RealDataError {
    #[error("data/binance/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    #[error("data revision mismatch for {file}: manifest={manifest_sha}, on-disk={actual_sha}")]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
    },

    #[error(
        "scenario {scenario} expected {expected} bars across {symbols} symbols \
         in [{span_start}..{span_end}); got {actual} ({pct:.2}% present), \
         below tolerance 99.50%"
    )]
    MissingData {
        scenario: String,
        expected: usize,
        actual: usize,
        symbols: usize,
        /// Percentage present — computed only for the human-readable error message;
        /// all decision logic uses integer arithmetic (see `load()`).
        #[allow(clippy::float_arithmetic)]
        pct: f64,
        span_start: String,
        span_end: String,
    },

    #[error("parquet read error: {0}")]
    Feed(#[from] FeedError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── TimeSpan ──────────────────────────────────────────────────────────────────

/// Half-open calendar-year time span `[start, end)`, in Unix milliseconds.
#[derive(Debug, Clone, Copy)]
pub struct TimeSpan {
    pub start_ms: i64,
    pub end_ms: i64,
    /// RFC-3339 label for report output.
    pub start_label: &'static str,
    /// RFC-3339 label for report output.
    pub end_label: &'static str,
}

impl TimeSpan {
    /// Calendar-year span: `[Y-01-01T00:00:00Z, (Y+1)-01-01T00:00:00Z)`.
    #[must_use]
    pub fn full_year(year: i32) -> Self {
        let start = year_start_ms(year);
        let end = year_start_ms(year + 1);
        // We Box::leak the labels because TimeSpan is `'static`-label-carrying
        // but labels must be heap-allocated (they depend on `year`).
        // Called once per scenario run — the allocation is harmless.
        let start_label: &'static str =
            Box::leak(format!("{year:04}-01-01T00:00:00Z").into_boxed_str());
        let end_label: &'static str =
            Box::leak(format!("{:04}-01-01T00:00:00Z", year + 1).into_boxed_str());
        Self {
            start_ms: start,
            end_ms: end,
            start_label,
            end_label,
        }
    }
}

fn year_start_ms(year: i32) -> i64 {
    // All years 2000-2100 produce valid dates; the `or` branch is unreachable
    // in practice but required by the error type.
    let date = time::Date::from_calendar_date(year, time::Month::January, 1)
        .unwrap_or(time::Date::from_ordinal_date(2023, 1).unwrap_or(time::Date::MIN));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT).unix_timestamp() * 1_000
}

fn month_start_ms(year: i32, month: time::Month) -> i64 {
    let date = time::Date::from_calendar_date(year, month, 1)
        .unwrap_or(time::Date::from_ordinal_date(2023, 1).unwrap_or(time::Date::MIN));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT).unix_timestamp() * 1_000
}

// ── LoadedBars ────────────────────────────────────────────────────────────────

/// Result of a successful bar load.
#[derive(Debug)]
pub struct LoadedBars {
    /// k-way merged bars, sorted `(open_ts ASC, symbol ASC)`.
    pub bars: Vec<Bar>,
    /// Recomputed aggregate SHA-256 (64 hex chars). Written into the report
    /// body's `## Data source` section.
    pub revision_sha: String,
    /// Total loaded bar count (for pct-present calculation).
    pub loaded_count: usize,
    /// Expected bar count passed in by the caller.
    pub expected_count: usize,
}

// ── RealDataBarSource ─────────────────────────────────────────────────────────

/// Real-data bar source backed by `data/binance/` Parquet files.
pub struct RealDataBarSource {
    /// Root of the parquet data directory, e.g. `data/binance`.
    parquet_root: PathBuf,
    /// Universe of symbols to load (10 USDT pairs).
    universe: Vec<Symbol>,
}

impl RealDataBarSource {
    /// Create a new bar source.
    ///
    /// `parquet_root` must contain both the symbol subdirectories and `REVISION.toml`.
    #[must_use]
    pub fn new(parquet_root: PathBuf, universe: Vec<Symbol>) -> Self {
        Self {
            parquet_root,
            universe,
        }
    }

    /// Load + verify + merge bars in one call.
    ///
    /// Steps (ADR-0032 § 2 verifier):
    /// 1. Check `REVISION.toml` exists → `RevisionMissing`.
    /// 2. For every parquet file in the scenario's span, verify on-disk SHA
    ///    against manifest → `RevisionMismatch`.
    /// 3. Recompute aggregate SHA from manifest `[files]` entries.
    /// 4. Read bars via `data::ReplayFeed::merge_symbols`.
    /// 5. Filter to `[span.start_ms, span.end_ms)`.
    /// 6. Enforce R3 missing-bar tolerance (≥ 99.5%) → `MissingData`.
    /// 7. Force-set `local_recv_ts = close_ts` (determinism normalization).
    ///
    /// # Errors
    ///
    /// Returns `RealDataError` on manifest missing / mismatch, feed error, or
    /// insufficient bar coverage (< 99.5% of `expected_total_bars`).
    #[allow(clippy::too_many_lines)]
    pub fn load(
        &self,
        span: TimeSpan,
        expected_total_bars: usize,
        scenario_name: &str,
    ) -> Result<LoadedBars, RealDataError> {
        // Step 1: manifest exists?
        let manifest_path = self.parquet_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(RealDataError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Read manifest raw (without full on-disk verification yet).
        let (files_map, _claimed_aggregate) = data::revision::read_manifest_raw(&self.parquet_root)
            .map_err(|e| RealDataError::RevisionParse(e.to_string()))?;

        // Step 2: verify each parquet file the scenario will read.
        let scenario_files = self.files_for_span(&span);
        for relpath in &scenario_files {
            let manifest_sha =
                files_map
                    .get(relpath)
                    .ok_or_else(|| RealDataError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".to_string(),
                        actual_sha: "n/a".to_string(),
                    })?;
            let abs_path = self.parquet_root.join(relpath);
            let actual_sha = data::revision::file_sha256(&abs_path)
                .map_err(|e| RealDataError::RevisionParse(format!("sha256 read error: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(RealDataError::RevisionMismatch {
                    file: relpath.clone(),
                    manifest_sha: manifest_sha.clone(),
                    actual_sha,
                });
            }
        }

        // Step 3: recompute aggregate SHA from manifest's [files] entries.
        let revision_sha = data::revision::compute_aggregate_sha(&files_map);

        // Step 4: read bars via ReplayFeed::merge_symbols.
        let feed = data::ReplayFeed::new(&self.parquet_root, true);
        let symbol_paths: Vec<(Symbol, PathBuf)> = self
            .universe
            .iter()
            .map(|s| (s.clone(), self.parquet_root.clone()))
            .collect();

        let mut bars = feed
            .merge_symbols(&symbol_paths, Timeframe::OneHour)
            .map_err(RealDataError::Feed)?;

        // Step 5: filter to the scenario span.
        bars.retain(|b| {
            let ts_ms = b.open_ts.0.unix_timestamp() * 1_000;
            ts_ms >= span.start_ms && ts_ms < span.end_ms
        });

        // Step 6: enforce R3 missing-bar tolerance (≥ 99.5%).
        // Integer arithmetic only — no floats in the comparison.
        let loaded_count = bars.len();
        // threshold = ceil(expected * 995 / 1000) using integer div_ceil.
        let threshold = (expected_total_bars * 995).div_ceil(1000);
        if loaded_count < threshold {
            // pct is only for the human-readable error string; no decision logic
            // uses it. The #[allow] on the field suppresses the lint there.
            #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
            let pct = if expected_total_bars > 0 {
                loaded_count as f64 / expected_total_bars as f64 * 100.0
            } else {
                0.0
            };
            return Err(RealDataError::MissingData {
                scenario: scenario_name.to_string(),
                expected: expected_total_bars,
                actual: loaded_count,
                symbols: self.universe.len(),
                pct,
                span_start: span.start_label.to_string(),
                span_end: span.end_label.to_string(),
            });
        }

        // Step 7: force-set local_recv_ts = close_ts for determinism.
        // ReplayFeed::read_parquet_bars sets `local_recv_ts = Timestamp::now()`;
        // normalising here makes the two code paths byte-symmetric.
        for bar in &mut bars {
            bar.local_recv_ts = bar.close_ts;
        }

        Ok(LoadedBars {
            loaded_count,
            expected_count: expected_total_bars,
            revision_sha,
            bars,
        })
    }

    /// Return the relative parquet paths for the scenario span.
    ///
    /// For a full-year span, yields `<SYM>/<YEAR>/<MM>.parquet` for
    /// all months × universe size.
    fn files_for_span(&self, span: &TimeSpan) -> Vec<String> {
        // Parse the boundary timestamps. On failure (should never happen for
        // valid Unix milliseconds) fall back to Unix epoch / far-future.
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

            // Advance to next month.
            let next_num = month_num % 12 + 1;
            let next_year = if next_num == 1 { year + 1 } else { year };
            // next_num is always 1-12 (month_num is 1-12, % 12 gives 0-11, +1 gives 1-12)
            let next_month = time::Month::try_from(next_num).unwrap_or(time::Month::January);

            // If next month start is at or past the end, we're done.
            let next_ms = month_start_ms(next_year, next_month);
            if next_ms >= span.end_ms {
                break;
            }
            // Guard: don't go past end_dt's month.
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
