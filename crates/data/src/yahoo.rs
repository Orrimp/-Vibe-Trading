//! Yahoo Finance parquet cache reader.
//!
//! Cargo feature: `yahoo` (default off).  The `yahoo-online` feature
//! additionally compiles the async `fetch_and_cache` method (used by the
//! `fetch_yahoo_klines` CLI only).
//!
//! # Cache layout
//!
//! ```text
//! <cache_root>/
//! ├── REVISION.toml
//! └── <TICKER>/
//!     └── <INTERVAL>/          ← "1d", "1h", or "1m"
//!         └── <YEAR>/
//!             └── <MONTH>.parquet   ← e.g. 01.parquet
//! ```
//!
//! Parquet schema mirrors `replay_feed.rs`:
//! ```text
//! open_time   Int64  — Unix millis, bar open
//! close_time  Int64  — Unix millis, bar close
//! open        Utf8   — price string
//! high        Utf8
//! low         Utf8
//! close       Utf8
//! volume      Utf8
//! trade_count Int64  — number of trades in bar (may be 0 if not available)
//! ```
//!
//! Bars returned by `load_cached` carry `Venue::Yahoo`
//! (landed in Wave C-4).

#![cfg(feature = "yahoo")]

use std::path::PathBuf;
use std::sync::OnceLock;

use polars::prelude::*;
use smol_str::SmolStr;
use time::{Month, OffsetDateTime, PrimitiveDateTime, Time};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// Re-use the revision machinery that already ships with `crates/data`.
use crate::revision::{compute_aggregate_sha, file_sha256, read_manifest_raw};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Coverage tolerance for Yahoo cache loads (Q9 = (b) / R2.5).
///
/// Relaxed from ADR-0032's 99.50% (Binance) because Yahoo's free
/// crypto series occasionally has 1-2 day gaps around exchange
/// outages.  K6 mitigation; v0.2.0 equities expansion will further
/// motivate the relaxed bound (weekends + holidays).
///
/// Integer arithmetic: threshold = ceil(expected * 95 / 100).
pub const MISSING_DATA_THRESHOLD_PCT: u32 = 95;

// ── Interval enum ─────────────────────────────────────────────────────────────

/// Yahoo bar cadence (Q4 = (c) adaptive cadence / ADR-0040 § D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interval {
    /// 1-minute bars.  Yahoo free tier: ≤ 7 days lookback.
    Minutes1,
    /// 1-hour bars.  Yahoo free tier: ≤ 730 days lookback.
    Hours1,
    /// 1-day bars.  Yahoo free tier: 30+ years lookback.
    Days1,
}

impl Interval {
    /// Adaptive cadence derived from a date range (T-AR4 / ADR-0040 § D6).
    ///
    /// Decision boundaries (operator-locked, Q4 = (c)):
    /// - range < 7 days         → `Minutes1`
    /// - range ∈ [7, 60] days   → `Hours1`
    /// - range > 60 days        → `Days1`
    pub fn derive_from_range(start_ms: i64, end_ms: i64) -> Self {
        const MS_PER_DAY: i64 = 86_400_000;
        let range_days = (end_ms - start_ms).max(0) / MS_PER_DAY;
        match range_days {
            d if d < 7 => Interval::Minutes1,
            d if d <= 60 => Interval::Hours1,
            _ => Interval::Days1,
        }
    }

    /// Format as the Yahoo API string parameter.
    pub const fn as_yahoo_str(self) -> &'static str {
        match self {
            Interval::Minutes1 => "1m",
            Interval::Hours1 => "1h",
            Interval::Days1 => "1d",
        }
    }

    /// Format as the cache subdirectory name (same as Yahoo string).
    pub const fn as_cache_dir(self) -> &'static str {
        match self {
            Interval::Minutes1 => "1m",
            Interval::Hours1 => "1h",
            Interval::Days1 => "1d",
        }
    }

    /// Map interval to the corresponding `trading_core::Timeframe`.
    pub const fn as_timeframe(self) -> Timeframe {
        match self {
            Interval::Minutes1 => Timeframe::OneMinute,
            Interval::Hours1 => Timeframe::OneHour,
            Interval::Days1 => Timeframe::OneDay,
        }
    }
}

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors from Yahoo cache operations (R1.3 / ADR-0040 § D5).
#[derive(thiserror::Error, Debug)]
pub enum YahooError {
    /// `REVISION.toml` not found at the given path.
    #[error("REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    /// Could not parse `REVISION.toml`.
    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    /// On-disk SHA for a file does not match the manifest.
    #[error("revision mismatch for {file}: manifest={manifest_sha}, on-disk={actual_sha}")]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
    },

    /// The requested `(ticker, interval, year, month)` parquet is absent
    /// from the cache.  The `hint` field carries the exact CLI invocation
    /// that would populate it (Q8 UX / R3.4).
    #[error(
        "cache miss for ({ticker}, {interval_str}, {start_label} .. {end_label})\n\
         Run: cargo run -p data --features yahoo-online --bin fetch_yahoo_klines \
         -- --tickers {ticker} --interval {interval_str} \
         --start {start_label} --end {end_label}"
    )]
    CacheMiss {
        ticker: String,
        interval_str: String,
        start_label: String,
        end_label: String,
    },

    /// Loaded bar count is below the 95% coverage threshold (R2.5).
    #[error(
        "insufficient data for {ticker} ({interval:?}) \
         {start_label} .. {end_label}: \
         expected {expected} bars, got {actual} ({pct:.1}% < 95%)"
    )]
    MissingData {
        ticker: String,
        interval: Interval,
        expected: usize,
        actual: usize,
        pct: f64,
        start_label: String,
        end_label: String,
    },

    /// Ticker is not in the 10-pair crypto-mirror table.
    #[error("unmapped Binance ticker: {input} — not in Yahoo crypto-mirror universe")]
    UnmappedTicker { input: String },

    /// Network error (used by `fetch_and_cache`).
    #[error("network error: {0}")]
    Http(String),

    /// Yahoo returned HTTP 429.
    #[error("rate limited by Yahoo; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    /// Failed to read or parse a parquet file.
    #[error("parquet read error: {0}")]
    Parquet(String),

    /// Generic I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── LoadedBars ────────────────────────────────────────────────────────────────

/// Result of a successful `load_cached` call.
pub struct LoadedBars {
    /// OHLCV bars, clipped to `[start_ms, end_ms)`, sorted by `open_ts` ascending.
    pub bars: Vec<Bar>,
    /// Aggregate SHA-256 of the cache at load time (determinism handle).
    pub revision_sha: String,
    /// Number of bars in `bars` after clipping.
    pub loaded_count: usize,
    /// Expected bar count for the requested range at this interval.
    pub expected_count: usize,
    /// The interval that was used to load the bars.
    pub interval: Interval,
}

// ── YahooBarSource ────────────────────────────────────────────────────────────

/// Read-only access to a Yahoo Finance parquet cache.
///
/// The `cache_root` must point to a directory containing `REVISION.toml`
/// and `<TICKER>/<INTERVAL>/<YEAR>/<MONTH>.parquet` files.
pub struct YahooBarSource {
    cache_root: PathBuf,
    /// Lazily cached aggregate SHA (set on first call to `load_cached`).
    revision_sha: OnceLock<String>,
}

impl YahooBarSource {
    /// Create a new source pointed at `cache_root`.
    ///
    /// No I/O is performed at construction time.
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache_root,
            revision_sha: OnceLock::new(),
        }
    }

    /// Load bars from the parquet cache for `ticker` over `[start_ms, end_ms)`.
    ///
    /// # Algorithm (ADR-0040 § D5)
    ///
    /// 1. Verify `REVISION.toml` exists.
    /// 2. Parse manifest; recompute aggregate SHA.
    /// 3. Enumerate `(year, month)` pairs the range spans.
    /// 4. Per-file: verify SHA matches manifest, then read parquet into `Vec<Bar>`.
    /// 5. Clip to `[start_ms, end_ms)`.
    /// 6. Enforce 95% coverage threshold (Q9 = (b)).
    /// 7. Force `local_recv_ts = close_ts` for determinism (ADR-0032 § D1 Step 7).
    ///
    /// # Errors
    ///
    /// - `RevisionMissing` — `REVISION.toml` absent.
    /// - `RevisionParse` — manifest parse failure.
    /// - `RevisionMismatch` — per-file SHA divergence.
    /// - `CacheMiss` — parquet for a required `(year, month)` not found.
    /// - `MissingData` — loaded bar count < 95% of expected.
    /// - `Parquet` — polars error reading a parquet file.
    pub fn load_cached(
        &self,
        ticker: &str,
        interval: Interval,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<LoadedBars, YahooError> {
        // Step 1: manifest exists?
        let manifest_path = self.cache_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(YahooError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Step 2: parse manifest; recompute aggregate SHA.
        let (files_map, _claimed) = read_manifest_raw(&self.cache_root)
            .map_err(|e| YahooError::RevisionParse(e.to_string()))?;
        let revision_sha = self
            .revision_sha
            .get_or_init(|| compute_aggregate_sha(&files_map))
            .clone();

        // Step 3: compute (year, month) pairs the scenario needs.
        let scenario_months = months_in_range(start_ms, end_ms);

        // Step 4: per-file SHA verification + parquet read.
        let mut bars: Vec<Bar> = Vec::new();
        for (year, month) in &scenario_months {
            let relpath = format!(
                "{ticker}/{}/{year:04}/{month:02}.parquet",
                interval.as_cache_dir()
            );
            let abs_path = self.cache_root.join(&relpath);
            if !abs_path.exists() {
                return Err(YahooError::CacheMiss {
                    ticker: ticker.to_string(),
                    interval_str: interval.as_cache_dir().to_string(),
                    start_label: format_iso8601(start_ms),
                    end_label: format_iso8601(end_ms),
                });
            }
            let manifest_sha =
                files_map
                    .get(&relpath)
                    .ok_or_else(|| YahooError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".into(),
                        actual_sha: "n/a".into(),
                    })?;
            let actual_sha = file_sha256(&abs_path)
                .map_err(|e| YahooError::RevisionParse(format!("sha256: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(YahooError::RevisionMismatch {
                    file: relpath,
                    manifest_sha: manifest_sha.clone(),
                    actual_sha,
                });
            }

            // Step 5 (partial): read parquet into Vec<Bar>.
            let mut file_bars = read_yahoo_parquet(&abs_path, ticker, interval)?;
            bars.append(&mut file_bars);
        }

        // Step 5 (clip): retain only bars in [start_ms, end_ms).
        bars.retain(|b| {
            let ts_ms = b.open_ts.0.unix_timestamp() * 1_000;
            ts_ms >= start_ms && ts_ms < end_ms
        });
        bars.sort_by_key(|b| b.open_ts);

        // Step 6: enforce Q9 = (b) 95% coverage threshold.
        let expected_count = expected_bars_for_range(interval, start_ms, end_ms);
        let threshold = (expected_count * MISSING_DATA_THRESHOLD_PCT as usize).div_ceil(100);
        if bars.len() < threshold {
            return Err(YahooError::MissingData {
                ticker: ticker.to_string(),
                interval,
                expected: expected_count,
                actual: bars.len(),
                #[allow(clippy::cast_precision_loss)]
                pct: bars.len() as f64 / expected_count.max(1) as f64 * 100.0,
                start_label: format_iso8601(start_ms),
                end_label: format_iso8601(end_ms),
            });
        }

        // Step 7: force local_recv_ts = close_ts for determinism (ADR-0032 § D1 Step 7).
        for bar in &mut bars {
            bar.local_recv_ts = bar.close_ts;
        }

        let loaded_count = bars.len();
        Ok(LoadedBars {
            bars,
            revision_sha,
            loaded_count,
            expected_count,
            interval,
        })
    }
}

// ── fetch_and_cache (yahoo-online feature only) ───────────────────────────────

#[cfg(feature = "yahoo-online")]
impl YahooBarSource {
    /// Fetch bars from the Yahoo Finance API and write them to the parquet cache.
    ///
    /// Used exclusively by the `fetch_yahoo_klines` CLI binary.
    ///
    /// After writing, delegates to `load_cached` to return verified bars with
    /// the revision SHA. K2 mitigation: the response checksum is written to
    /// `[revision.yahoo_response]` in `REVISION.toml` for forensics.
    pub async fn fetch_and_cache(
        &self,
        ticker: &str,
        interval: Interval,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<LoadedBars, YahooError> {
        use yahoo_finance_api as yfa;

        let provider = yfa::YahooConnector::new().map_err(|e| YahooError::Http(e.to_string()))?;

        let start_dt = time::OffsetDateTime::from_unix_timestamp(start_ms / 1_000)
            .map_err(|e| YahooError::Http(format!("invalid start_ms {start_ms}: {e}")))?;
        let end_dt = time::OffsetDateTime::from_unix_timestamp(end_ms / 1_000)
            .map_err(|e| YahooError::Http(format!("invalid end_ms {end_ms}: {e}")))?;

        let response = provider
            .get_quote_history_interval(ticker, start_dt, end_dt, interval.as_yahoo_str())
            .await
            .map_err(classify_yfa_error)?;

        let quotes = response
            .quotes()
            .map_err(|e| YahooError::Http(e.to_string()))?;

        // K2 mitigation: hash the serialised quotes for forensic tracking.
        let response_sha = sha256_of_quotes(&quotes);

        // Convert quotes to bars.
        let bars = quotes_to_bars(ticker, interval, &quotes);

        // Write parquet files grouped by (year, month).
        write_bars_by_month(&self.cache_root, ticker, interval, &bars)?;

        // Update [revision.yahoo_response] forensics table.
        upsert_yahoo_response_checksum(
            &self.cache_root,
            ticker,
            interval,
            start_ms,
            end_ms,
            &response_sha,
        )?;

        // Regenerate aggregate manifest.
        regenerate_revision_manifest(&self.cache_root)?;

        // Round-trip read to get verified result.
        self.load_cached(ticker, interval, start_ms, end_ms)
    }
}

/// Classify a `yahoo_finance_api::YahooError` into our local error type.
#[cfg(feature = "yahoo-online")]
fn classify_yfa_error(e: yahoo_finance_api::YahooError) -> YahooError {
    match &e {
        yahoo_finance_api::YahooError::TooManyRequests(_) => YahooError::RateLimited {
            retry_after_secs: 60,
        },
        _ => YahooError::Http(e.to_string()),
    }
}

/// Compute a SHA-256 over the quotes slice for K2 forensics.
/// The hash is over a deterministic CSV-like serialisation of the fields.
#[cfg(feature = "yahoo-online")]
fn sha256_of_quotes(quotes: &[yahoo_finance_api::Quote]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for q in quotes {
        hasher.update(q.timestamp.to_string().as_bytes());
        hasher.update(b",");
        hasher.update(q.open.to_string().as_bytes());
        hasher.update(b",");
        hasher.update(q.high.to_string().as_bytes());
        hasher.update(b",");
        hasher.update(q.low.to_string().as_bytes());
        hasher.update(b",");
        hasher.update(q.close.to_string().as_bytes());
        hasher.update(b",");
        hasher.update(q.volume.to_string().as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

/// Convert a `yahoo_finance_api::Quote` slice into a `Vec<Bar>`.
#[cfg(feature = "yahoo-online")]
fn quotes_to_bars(
    ticker: &str,
    interval: Interval,
    quotes: &[yahoo_finance_api::Quote],
) -> Vec<Bar> {
    let tf = interval.as_timeframe();
    let symbol = Symbol::new(ticker);
    let ms_per_bar = match interval {
        Interval::Minutes1 => 60_000i64,
        Interval::Hours1 => 3_600_000,
        Interval::Days1 => 86_400_000,
    };

    let mut bars = Vec::with_capacity(quotes.len());
    for q in quotes {
        // Skip bars with zero/negative prices.
        #[allow(clippy::cast_possible_truncation)]
        if q.open <= 0.0_f64 || q.high <= 0.0_f64 || q.low <= 0.0_f64 || q.close <= 0.0_f64 {
            continue;
        }
        let Ok(open_dt) = time::OffsetDateTime::from_unix_timestamp(q.timestamp) else {
            continue;
        };
        let Ok(close_dt) =
            time::OffsetDateTime::from_unix_timestamp(q.timestamp + ms_per_bar / 1_000)
        else {
            continue;
        };

        let Some(open_dec) = rust_decimal::Decimal::try_from(q.open).ok() else {
            continue;
        };
        let Some(high_dec) = rust_decimal::Decimal::try_from(q.high).ok() else {
            continue;
        };
        let Some(low_dec) = rust_decimal::Decimal::try_from(q.low).ok() else {
            continue;
        };
        let Some(close_dec) = rust_decimal::Decimal::try_from(q.close).ok() else {
            continue;
        };

        let Ok(open_price) = Price::new(open_dec) else {
            continue;
        };
        let Ok(high_price) = Price::new(high_dec) else {
            continue;
        };
        let Ok(low_price) = Price::new(low_dec) else {
            continue;
        };
        let Ok(close_price) = Price::new(close_dec) else {
            continue;
        };
        let Ok(qty) = Quantity::new(rust_decimal::Decimal::from(q.volume)) else {
            continue;
        };

        let open_ts = Timestamp::new(open_dt);
        let close_ts = Timestamp::new(close_dt);
        bars.push(Bar {
            symbol: symbol.clone(),
            tf,
            open_ts,
            close_ts,
            open: open_price,
            high: high_price,
            low: low_price,
            close: close_price,
            volume: qty,
            trade_count: 0,
            local_recv_ts: Timestamp::new(close_dt),
            venue: Venue::Yahoo,
        });
    }
    bars
}

/// Write bars grouped by (year, month) into the parquet cache.
///
/// Path: `<cache_root>/<ticker>/<interval>/<year>/<month:02>.parquet`
///
/// The schema matches the read path in `read_yahoo_parquet`:
/// columns: `open_time` (i64 ms), `close_time` (i64 ms),
/// `open`, `high`, `low`, `close`, `volume` (all Utf8),
/// `trade_count` (i64).
#[cfg(feature = "yahoo-online")]
fn write_bars_by_month(
    cache_root: &std::path::Path,
    ticker: &str,
    interval: Interval,
    bars: &[Bar],
) -> Result<(), YahooError> {
    use polars::prelude::*;
    use std::collections::HashMap;

    // Group by (year, month).
    let mut groups: HashMap<(i32, u8), Vec<&Bar>> = HashMap::new();
    for bar in bars {
        let dt = bar.open_ts.0;
        let key = (dt.year(), dt.month() as u8);
        groups.entry(key).or_default().push(bar);
    }

    for ((year, month), group) in &groups {
        let dir = cache_root
            .join(ticker)
            .join(interval.as_cache_dir())
            .join(format!("{year:04}"));
        std::fs::create_dir_all(&dir)?;

        let path = dir.join(format!("{month:02}.parquet"));

        // Sort within the group for determinism.
        let mut sorted: Vec<&Bar> = group.to_vec();
        sorted.sort_by_key(|b| b.open_ts.0.unix_timestamp());

        let mut open_times: Vec<i64> = Vec::with_capacity(sorted.len());
        let mut close_times: Vec<i64> = Vec::with_capacity(sorted.len());
        let mut opens: Vec<String> = Vec::with_capacity(sorted.len());
        let mut highs: Vec<String> = Vec::with_capacity(sorted.len());
        let mut lows: Vec<String> = Vec::with_capacity(sorted.len());
        let mut closes: Vec<String> = Vec::with_capacity(sorted.len());
        let mut volumes: Vec<String> = Vec::with_capacity(sorted.len());
        let mut trade_counts: Vec<i64> = Vec::with_capacity(sorted.len());

        for bar in &sorted {
            open_times.push(bar.open_ts.0.unix_timestamp() * 1_000);
            close_times.push(bar.close_ts.0.unix_timestamp() * 1_000);
            opens.push(bar.open.get().to_string());
            highs.push(bar.high.get().to_string());
            lows.push(bar.low.get().to_string());
            closes.push(bar.close.get().to_string());
            volumes.push(bar.volume.get().to_string());
            #[allow(clippy::cast_lossless)]
            trade_counts.push(bar.trade_count as i64);
        }

        let df = DataFrame::new(vec![
            Column::new("open_time".into(), open_times),
            Column::new("close_time".into(), close_times),
            Column::new("open".into(), opens),
            Column::new("high".into(), highs),
            Column::new("low".into(), lows),
            Column::new("close".into(), closes),
            Column::new("volume".into(), volumes),
            Column::new("trade_count".into(), trade_counts),
        ])
        .map_err(|e| YahooError::Parquet(format!("df build: {e}")))?;

        let file = std::fs::File::create(&path)?;
        let writer = polars::io::parquet::write::ParquetWriter::new(file);
        writer
            .finish(&mut df.clone())
            .map_err(|e| YahooError::Parquet(format!("parquet write: {e}")))?;
    }

    Ok(())
}

/// Update the `[revision.yahoo_response]` forensics table in `REVISION.toml`.
///
/// Key format: `"{TICKER}/{INTERVAL}/{YEAR}-{MONTH:02}"`.
/// The value is the SHA-256 of the raw quotes from that fetch.
#[cfg(feature = "yahoo-online")]
fn upsert_yahoo_response_checksum(
    cache_root: &std::path::Path,
    ticker: &str,
    interval: Interval,
    start_ms: i64,
    _end_ms: i64,
    response_sha: &str,
) -> Result<(), YahooError> {
    let manifest_path = cache_root.join("REVISION.toml");

    // Read existing or create an empty skeleton.
    let content = if manifest_path.exists() {
        std::fs::read_to_string(&manifest_path)?
    } else {
        String::new()
    };

    // Parse as a generic TOML value so we can manipulate it without
    // re-serialising the entire typed struct.
    use toml::Value;
    let mut doc: toml::Table = if content.is_empty() {
        toml::Table::new()
    } else {
        toml::from_str(&content)
            .map_err(|e| YahooError::RevisionParse(format!("toml parse: {e}")))?
    };

    // Ensure [revision] and [revision.yahoo_response] tables exist.
    let revision = doc
        .entry("revision")
        .or_insert_with(|| Value::Table(toml::Table::new()));
    let revision_table = revision
        .as_table_mut()
        .ok_or_else(|| YahooError::RevisionParse("revision is not a table".to_string()))?;
    let yahoo_response = revision_table
        .entry("yahoo_response")
        .or_insert_with(|| Value::Table(toml::Table::new()));
    let yr_table = yahoo_response
        .as_table_mut()
        .ok_or_else(|| YahooError::RevisionParse("yahoo_response is not a table".to_string()))?;

    // Key: "{ticker}/{interval}/{YYYY}-{MM}".
    let dt = time::OffsetDateTime::from_unix_timestamp(start_ms / 1_000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let key = format!(
        "{ticker}/{}/{:04}-{:02}",
        interval.as_cache_dir(),
        dt.year(),
        dt.month() as u8,
    );
    yr_table.insert(key, Value::String(response_sha.to_string()));

    let toml_str = toml::to_string_pretty(&doc)
        .map_err(|e| YahooError::RevisionParse(format!("toml serialize: {e}")))?;
    std::fs::write(&manifest_path, toml_str)?;

    Ok(())
}

/// Recompute `REVISION.toml` by scanning all parquet files under `cache_root`.
#[cfg(feature = "yahoo-online")]
fn regenerate_revision_manifest(cache_root: &std::path::Path) -> Result<(), YahooError> {
    use crate::revision::{compute_aggregate_sha, file_sha256};
    use std::collections::BTreeMap;

    fn collect_parquet_files(
        root: &std::path::Path,
    ) -> Result<BTreeMap<String, String>, YahooError> {
        fn recurse(
            root: &std::path::Path,
            dir: &std::path::Path,
            out: &mut BTreeMap<String, String>,
        ) -> Result<(), YahooError> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    recurse(root, &path, out)?;
                } else if path.extension().is_some_and(|e| e == "parquet") {
                    let rel = path.strip_prefix(root).map_err(|_| {
                        YahooError::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "strip_prefix failed",
                        ))
                    })?;
                    let rel_str = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/");
                    let sha =
                        file_sha256(&path).map_err(|e| YahooError::RevisionParse(e.to_string()))?;
                    out.insert(rel_str, sha);
                }
            }
            Ok(())
        }

        let mut files = BTreeMap::new();
        recurse(root, root, &mut files)?;
        Ok(files)
    }

    let files = collect_parquet_files(cache_root)?;
    let aggregate = compute_aggregate_sha(&files);

    let now = time::OffsetDateTime::now_utc();
    let generated_at = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    // Preserve any existing [revision.yahoo_response] table.
    let manifest_path = cache_root.join("REVISION.toml");
    let existing_yahoo_response: Option<toml::Value> = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
        toml::from_str::<toml::Table>(&content).ok().and_then(|t| {
            t.get("revision")?
                .as_table()?
                .get("yahoo_response")
                .cloned()
        })
    } else {
        None
    };

    // Build the final TOML document.
    use toml::Value;

    let mut files_table = toml::Table::new();
    for (k, v) in &files {
        files_table.insert(k.clone(), Value::String(v.clone()));
    }

    let mut metadata_table = toml::Table::new();
    metadata_table.insert("generated_at".to_string(), Value::String(generated_at));
    metadata_table.insert(
        "yahoo_base".to_string(),
        Value::String("https://query1.finance.yahoo.com".to_string()),
    );
    metadata_table.insert(
        "fetch_tool".to_string(),
        Value::String("fetch_yahoo_klines".to_string()),
    );
    metadata_table.insert(
        "fetch_version".to_string(),
        Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );

    let mut revision_table = toml::Table::new();
    revision_table.insert("sha256".to_string(), Value::String(aggregate));
    revision_table.insert("metadata".to_string(), Value::Table(metadata_table));
    if let Some(yr) = existing_yahoo_response {
        revision_table.insert("yahoo_response".to_string(), yr);
    }

    let mut doc = toml::Table::new();
    doc.insert("revision".to_string(), Value::Table(revision_table));
    doc.insert("files".to_string(), Value::Table(files_table));

    let toml_str = toml::to_string_pretty(&doc)
        .map_err(|e| YahooError::RevisionParse(format!("toml serialize: {e}")))?;
    std::fs::write(&manifest_path, toml_str)?;

    Ok(())
}

/// Write (or overwrite) `REVISION.toml` at `root` by scanning all parquet files.
///
/// Exposed for use by the `fetch_yahoo_klines` CLI binary.
#[cfg(feature = "yahoo-online")]
pub fn write_revision_manifest(root: &std::path::Path) -> Result<(), YahooError> {
    regenerate_revision_manifest(root)
}

// ── Ticker conversion ─────────────────────────────────────────────────────────

/// Convert a Binance-style UI symbol to a Yahoo-native ticker (Q6 = (a) / D7).
///
/// v0.1.0 supports the 10 crypto-mirror pairs only.  Multi-asset expansion
/// (equities, FX, commodities) at v0.2.0 will extend this table.
///
/// # Errors
///
/// Returns `YahooError::UnmappedTicker` if `sym` is not in the 10-pair table.
pub fn binance_to_yahoo_ticker(sym: &Symbol) -> Result<SmolStr, YahooError> {
    let s = sym.0.as_str();
    let mapped = match s {
        "BTCUSDT" => "BTC-USD",
        "ETHUSDT" => "ETH-USD",
        "BNBUSDT" => "BNB-USD",
        "SOLUSDT" => "SOL-USD",
        "XRPUSDT" => "XRP-USD",
        "ADAUSDT" => "ADA-USD",
        "DOGEUSDT" => "DOGE-USD",
        "AVAXUSDT" => "AVAX-USD",
        "DOTUSDT" => "DOT-USD",
        "LINKUSDT" => "LINK-USD",
        other => {
            return Err(YahooError::UnmappedTicker {
                input: other.to_string(),
            });
        }
    };
    Ok(SmolStr::new(mapped))
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Read a Yahoo parquet file into `Vec<Bar>`.
///
/// The schema is identical to the Binance replay schema (see `replay_feed.rs`)
/// so the same column names apply:
/// `open_time`, `close_time`, `open`, `high`, `low`, `close`, `volume`,
/// `trade_count`.
fn read_yahoo_parquet(
    path: &std::path::Path,
    ticker: &str,
    interval: Interval,
) -> Result<Vec<Bar>, YahooError> {
    let df = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .sort(
            ["open_time"],
            SortMultipleOptions::default().with_order_descending(false),
        )
        .collect()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;

    let n = df.height();
    let mut bars = Vec::with_capacity(n);

    let open_times = df
        .column("open_time")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .i64()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;
    let close_times = df
        .column("close_time")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .i64()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;
    let opens = df
        .column("open")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .str()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;
    let highs = df
        .column("high")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .str()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;
    let lows = df
        .column("low")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .str()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;
    let closes = df
        .column("close")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .str()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;
    let volumes = df
        .column("volume")
        .map_err(|e| YahooError::Parquet(e.to_string()))?
        .str()
        .map_err(|e| YahooError::Parquet(e.to_string()))?;

    // trade_count may be absent in Yahoo-sourced files — default to 0.
    let trade_counts: Option<&ChunkedArray<Int64Type>> =
        df.column("trade_count").ok().and_then(|s| s.i64().ok());

    let symbol = Symbol::new(ticker);
    let tf = interval.as_timeframe();

    for i in 0..n {
        let open_time = open_times
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null open_time".into()))?;
        let close_time = close_times
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null close_time".into()))?;
        let open_str = opens
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null open".into()))?;
        let high_str = highs
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null high".into()))?;
        let low_str = lows
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null low".into()))?;
        let close_str = closes
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null close".into()))?;
        let vol_str = volumes
            .get(i)
            .ok_or_else(|| YahooError::Parquet("null volume".into()))?;
        let tc = trade_counts.and_then(|tc| tc.get(i)).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let trade_count = tc.max(0) as u32;

        let open_ts = millis_to_ts(open_time);
        let close_ts = millis_to_ts(close_time);

        bars.push(Bar {
            symbol: symbol.clone(),
            tf,
            open_ts,
            close_ts,
            open: parse_price(open_str)?,
            high: parse_price(high_str)?,
            low: parse_price(low_str)?,
            close: parse_price(close_str)?,
            volume: parse_qty(vol_str)?,
            trade_count,
            // `local_recv_ts` is set to `close_ts` at the end of `load_cached`
            // (ADR-0032 § D1 Step 7 determinism rule); this initial value is
            // overwritten before the bars are returned.
            local_recv_ts: close_ts,
            venue: Venue::Yahoo,
        });
    }

    Ok(bars)
}

/// Expected bar count for an interval over `[start_ms, end_ms)`.
///
/// Uses a simple wall-clock-based approximation.  Yahoo crypto runs 24/7;
/// equities are handled at v0.2.0 with a market-calendar layer.
pub fn expected_bars_for_range(interval: Interval, start_ms: i64, end_ms: i64) -> usize {
    let range_ms = (end_ms - start_ms).max(0) as u64;
    let ms_per_bar: u64 = match interval {
        Interval::Minutes1 => 60_000,
        Interval::Hours1 => 3_600_000,
        Interval::Days1 => 86_400_000,
    };
    (range_ms / ms_per_bar) as usize
}

/// Return all `(year, month)` pairs covered by `[start_ms, end_ms)`.
fn months_in_range(start_ms: i64, end_ms: i64) -> Vec<(i32, u8)> {
    let start_dt =
        OffsetDateTime::from_unix_timestamp(start_ms / 1_000).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let end_dt = OffsetDateTime::from_unix_timestamp((end_ms - 1).max(0) / 1_000)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);

    let mut result = Vec::new();
    let mut year = start_dt.year();
    let mut month = start_dt.month();

    loop {
        result.push((year, month as u8));
        if year == end_dt.year() && month == end_dt.month() {
            break;
        }
        // Advance to next month.
        let next_month_num = month as u8 % 12 + 1;
        if next_month_num == 1 {
            year += 1;
        }
        month = Month::try_from(next_month_num).unwrap_or(Month::January);
    }
    result
}

/// Format a Unix-ms timestamp as `YYYY-MM-DD` for display in error messages.
fn format_iso8601(ms: i64) -> String {
    match OffsetDateTime::from_unix_timestamp(ms / 1_000) {
        Ok(dt) => format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day()),
        Err(_) => format!("{ms}ms"),
    }
}

fn millis_to_ts(ms: i64) -> Timestamp {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map(|odt| Timestamp(PrimitiveDateTime::new(odt.date(), odt.time()).assume_utc()))
        .unwrap_or_else(|_| {
            Timestamp(PrimitiveDateTime::new(time::Date::MIN, Time::MIDNIGHT).assume_utc())
        })
}

fn parse_price(s: &str) -> Result<Price, YahooError> {
    s.trim()
        .parse::<rust_decimal::Decimal>()
        .map_err(|e| YahooError::Parquet(format!("price parse '{s}': {e}")))
        .and_then(|d| Price::new(d).map_err(|e| YahooError::Parquet(format!("price invalid: {e}"))))
}

fn parse_qty(s: &str) -> Result<Quantity, YahooError> {
    s.trim()
        .parse::<rust_decimal::Decimal>()
        .map_err(|e| YahooError::Parquet(format!("qty parse '{s}': {e}")))
        .and_then(|d| {
            Quantity::new(d).map_err(|e| YahooError::Parquet(format!("qty invalid: {e}")))
        })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;

    // ── T-AR4: Interval::derive_from_range 10-row truth table ─────────────────

    /// Asserts the 10-row boundary truth table from decomp.md § T-AR4.
    #[test]
    fn interval_derive_boundaries() {
        const MS_PER_DAY: i64 = 86_400_000;
        let base: i64 = 0; // arbitrary epoch start

        // (range_days, expected_interval)
        let cases: &[(i64, Interval)] = &[
            (0, Interval::Minutes1),
            (1, Interval::Minutes1),
            (6, Interval::Minutes1),
            (7, Interval::Hours1),
            (30, Interval::Hours1),
            (60, Interval::Hours1),
            (61, Interval::Days1),
            (90, Interval::Days1),
            (365, Interval::Days1),
            (3650, Interval::Days1),
        ];

        for (days, expected) in cases {
            let end_ms = base + days * MS_PER_DAY;
            let got = Interval::derive_from_range(base, end_ms);
            assert_eq!(
                got, *expected,
                "derive_from_range for {days}d: expected {expected:?}, got {got:?}"
            );
        }
    }

    /// Verifies `as_yahoo_str` and `as_cache_dir` are byte-stable.
    #[test]
    fn interval_string_stability() {
        assert_eq!(Interval::Minutes1.as_yahoo_str(), "1m");
        assert_eq!(Interval::Hours1.as_yahoo_str(), "1h");
        assert_eq!(Interval::Days1.as_yahoo_str(), "1d");
        assert_eq!(Interval::Minutes1.as_cache_dir(), "1m");
        assert_eq!(Interval::Hours1.as_cache_dir(), "1h");
        assert_eq!(Interval::Days1.as_cache_dir(), "1d");
    }

    // ── T-AR2: binance_to_yahoo_ticker 10-row table ───────────────────────────

    /// Asserts all 10 round-trips + that an unknown ticker returns `UnmappedTicker`.
    #[test]
    fn binance_to_yahoo_table_pinned() {
        let table: &[(&str, &str)] = &[
            ("BTCUSDT", "BTC-USD"),
            ("ETHUSDT", "ETH-USD"),
            ("BNBUSDT", "BNB-USD"),
            ("SOLUSDT", "SOL-USD"),
            ("XRPUSDT", "XRP-USD"),
            ("ADAUSDT", "ADA-USD"),
            ("DOGEUSDT", "DOGE-USD"),
            ("AVAXUSDT", "AVAX-USD"),
            ("DOTUSDT", "DOT-USD"),
            ("LINKUSDT", "LINK-USD"),
        ];

        for (binance, yahoo) in table {
            let sym = Symbol::new(*binance);
            let result = binance_to_yahoo_ticker(&sym)
                .unwrap_or_else(|_| panic!("binance_to_yahoo_ticker({binance}) should succeed"));
            assert_eq!(result.as_str(), *yahoo, "mapping for {binance}");
        }

        // Unknown ticker must error.
        let unknown = Symbol::new("FOOUSDT");
        let err = binance_to_yahoo_ticker(&unknown).unwrap_err();
        assert!(
            matches!(err, YahooError::UnmappedTicker { .. }),
            "expected UnmappedTicker for FOOUSDT, got: {err}"
        );
    }

    // ── T-AR5: MISSING_DATA_THRESHOLD_PCT ────────────────────────────────────

    /// Verifies the const value is exactly 95.
    #[test]
    fn missing_data_threshold_const() {
        assert_eq!(MISSING_DATA_THRESHOLD_PCT, 95u32);
    }

    /// Q9 = (b) 94.99% case must error; 95.00% case must pass.
    ///
    /// Uses `expected_bars_for_range` for a 100-day window at 1d cadence
    /// (expected = 100 bars).  94 bars < threshold(95/100 × 100 = 95) → error;
    /// 95 bars == threshold → pass.
    #[test]
    fn coverage_threshold_95_pct() {
        // 100 days × 1d → expected = 100 bars.
        const MS_PER_DAY: i64 = 86_400_000;
        let start_ms: i64 = 0;
        let end_ms: i64 = 100 * MS_PER_DAY;
        let expected = expected_bars_for_range(Interval::Days1, start_ms, end_ms);
        assert_eq!(
            expected, 100,
            "sanity: expected 100 bars for 100-day 1d window"
        );

        let threshold = (expected * MISSING_DATA_THRESHOLD_PCT as usize).div_ceil(100);
        assert_eq!(
            threshold, 95,
            "threshold should be 95 for expected=100, pct=95"
        );

        // 94 bars → below threshold.
        assert!(
            94 < threshold,
            "94 bars should be below threshold ({threshold})"
        );
        // 95 bars → meets threshold.
        assert!(
            95 >= threshold,
            "95 bars should meet threshold ({threshold})"
        );
    }

    // ── expected_bars_for_range arithmetic ───────────────────────────────────

    #[test]
    fn expected_bars_for_range_arithmetic() {
        const MS_PER_DAY: i64 = 86_400_000;
        const MS_PER_HOUR: i64 = 3_600_000;
        const MS_PER_MIN: i64 = 60_000;

        // 1d × 30 days = 30
        assert_eq!(
            expected_bars_for_range(Interval::Days1, 0, 30 * MS_PER_DAY),
            30
        );
        // 1h × 24 hours = 24
        assert_eq!(
            expected_bars_for_range(Interval::Hours1, 0, 24 * MS_PER_HOUR),
            24
        );
        // 1m × 60 minutes = 60
        assert_eq!(
            expected_bars_for_range(Interval::Minutes1, 0, 60 * MS_PER_MIN),
            60
        );
        // Zero range → 0
        assert_eq!(expected_bars_for_range(Interval::Days1, 100, 100), 0);
        // Negative range → 0 (clamped)
        assert_eq!(expected_bars_for_range(Interval::Days1, 200, 100), 0);
    }

    // ── months_in_range helper ────────────────────────────────────────────────

    #[test]
    fn months_in_range_single_month() {
        // 2024-01-01 .. 2024-01-31 → [(2024, 1)]
        let start_ms = date_to_millis(2024, 1, 1);
        let end_ms = date_to_millis(2024, 1, 31);
        let months = months_in_range(start_ms, end_ms);
        assert_eq!(months, vec![(2024, 1)]);
    }

    #[test]
    fn months_in_range_cross_year() {
        // 2024-12-01 .. 2025-01-31 → [(2024, 12), (2025, 1)]
        let start_ms = date_to_millis(2024, 12, 1);
        let end_ms = date_to_millis(2025, 1, 31);
        let months = months_in_range(start_ms, end_ms);
        assert_eq!(months, vec![(2024, 12), (2025, 1)]);
    }

    #[test]
    fn months_in_range_three_months() {
        let start_ms = date_to_millis(2024, 1, 15);
        let end_ms = date_to_millis(2024, 3, 10);
        let months = months_in_range(start_ms, end_ms);
        assert_eq!(months, vec![(2024, 1), (2024, 2), (2024, 3)]);
    }

    /// Helper: date to Unix millis at midnight UTC.
    fn date_to_millis(year: i32, month: u8, day: u8) -> i64 {
        use time::{Date, Month};
        let m = Month::try_from(month).expect("month 1-12");
        let d = Date::from_calendar_date(year, m, day).expect("valid date");
        let pdt = PrimitiveDateTime::new(d, Time::MIDNIGHT);
        pdt.assume_utc().unix_timestamp() * 1_000
    }
}
