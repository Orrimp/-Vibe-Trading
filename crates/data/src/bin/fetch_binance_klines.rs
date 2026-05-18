//! `fetch_binance_klines` — Binance REST klines → Parquet downloader.
//!
//! Fetches historical OHLCV (klines) from Binance public REST API and writes
//! per-symbol-month Parquet files that `ReplayFeed` can read directly.
//!
//! # Output layout
//!
//! ```text
//! <out>/<SYMBOL>/<YEAR>/<MONTH-padded>.parquet
//! ```
//!
//! # Schema (matches `replay_feed.rs`)
//!
//! ```text
//! open_time   Int64  — Unix millis, bar open
//! close_time  Int64  — Unix millis, bar close
//! open        Utf8   — price string
//! high        Utf8
//! low         Utf8
//! close       Utf8
//! volume      Utf8
//! trade_count Int64  — number of trades in bar
//! ```

use std::{
    io::BufWriter,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use polars::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use time::{Date, Month, PrimitiveDateTime, Time};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "fetch_binance_klines",
    about = "Fetch Binance historical klines and write Parquet files"
)]
struct Cli {
    /// Comma-separated symbols, e.g. BTCUSDT,ETHUSDT
    #[arg(short = 's', long, value_delimiter = ',')]
    symbols: Vec<String>,

    /// Inclusive start date (YYYY-MM-DD)
    #[arg(long, default_value = "2023-01-01")]
    start: String,

    /// Inclusive end date (YYYY-MM-DD)
    #[arg(long, default_value = "2024-12-31")]
    end: String,

    /// Binance interval string: 1m, 5m, 15m, 1h, 4h, 1d
    #[arg(long, default_value = "1h")]
    interval: String,

    /// Output root directory
    #[arg(long, default_value = "data/binance")]
    out: PathBuf,

    /// Overwrite existing files (default: skip existing)
    #[arg(long, default_value_t = false)]
    force: bool,

    /// After all downloads complete, write (or overwrite) a `REVISION.toml`
    /// manifest in `--out` with a SHA-256 for every Parquet file present.
    /// See ADR-0032 and `data::revision::write_revision_manifest`.
    #[arg(long, default_value_t = false)]
    emit_revision_manifest: bool,
}

// ── Binance API types ─────────────────────────────────────────────────────────

/// One kline bar, parsed from Binance's array-of-arrays response.
///
/// Binance response indices:
/// ```text
/// 0  open_time             (i64  ms)
/// 1  open                  (String)
/// 2  high                  (String)
/// 3  low                   (String)
/// 4  close                 (String)
/// 5  volume                (String)
/// 6  close_time            (i64  ms)
/// 7  quote_volume          (String) — ignored
/// 8  trade_count           (i64)
/// 9  taker_buy_base_volume (String) — ignored
/// 10 taker_buy_quote_volume(String) — ignored
/// 11 ignore                (String) — ignored
/// ```
#[derive(Debug, Clone)]
pub struct Kline {
    pub open_time: i64,
    pub close_time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub trade_count: i64,
}

/// Intermediate JSON representation for a kline array element.
/// Binance uses a heterogeneous JSON array; we deserialize to `Value` first.
#[derive(Deserialize)]
struct RawKline(serde_json::Value);

impl RawKline {
    fn parse(self) -> Result<Kline> {
        let arr = self
            .0
            .as_array()
            .ok_or_else(|| anyhow!("kline element is not an array"))?;
        if arr.len() < 9 {
            return Err(anyhow!("kline array too short: len={}", arr.len()));
        }
        let open_time = arr[0]
            .as_i64()
            .ok_or_else(|| anyhow!("open_time not i64"))?;
        let open = arr[1]
            .as_str()
            .ok_or_else(|| anyhow!("open not str"))?
            .to_owned();
        let high = arr[2]
            .as_str()
            .ok_or_else(|| anyhow!("high not str"))?
            .to_owned();
        let low = arr[3]
            .as_str()
            .ok_or_else(|| anyhow!("low not str"))?
            .to_owned();
        let close = arr[4]
            .as_str()
            .ok_or_else(|| anyhow!("close not str"))?
            .to_owned();
        let volume = arr[5]
            .as_str()
            .ok_or_else(|| anyhow!("volume not str"))?
            .to_owned();
        let close_time = arr[6]
            .as_i64()
            .ok_or_else(|| anyhow!("close_time not i64"))?;
        let trade_count = arr[8]
            .as_i64()
            .ok_or_else(|| anyhow!("trade_count not i64"))?;
        Ok(Kline {
            open_time,
            close_time,
            open,
            high,
            low,
            close,
            volume,
            trade_count,
        })
    }
}

// ── URL builder ───────────────────────────────────────────────────────────────

const BINANCE_KLINES_URL: &str = "https://api.binance.com/api/v3/klines";

/// Build a Binance klines query URL.
///
/// Pure function — no I/O. Used by tests.
pub fn build_klines_url(symbol: &str, interval: &str, start_ms: i64, end_ms: i64) -> String {
    format!(
        "{BINANCE_KLINES_URL}?symbol={symbol}&interval={interval}&startTime={start_ms}&endTime={end_ms}&limit=1000"
    )
}

// ── Date utilities ────────────────────────────────────────────────────────────

/// Parse "YYYY-MM-DD" into a `time::Date`.
fn parse_date(s: &str) -> Result<Date> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(anyhow!("date must be YYYY-MM-DD, got: {s}"));
    }
    let year: i32 = parts[0]
        .parse()
        .with_context(|| format!("bad year in date: {s}"))?;
    let month_num: u8 = parts[1]
        .parse()
        .with_context(|| format!("bad month in date: {s}"))?;
    let day: u8 = parts[2]
        .parse()
        .with_context(|| format!("bad day in date: {s}"))?;
    let month = Month::try_from(month_num)
        .map_err(|_| anyhow!("month {month_num} out of range in date: {s}"))?;
    Date::from_calendar_date(year, month, day).map_err(|e| anyhow!("invalid date {s}: {e}"))
}

/// Convert a `Date` to Unix milliseconds at midnight UTC.
fn date_to_millis(d: Date) -> i64 {
    let pdt = PrimitiveDateTime::new(d, Time::MIDNIGHT);
    let odt = pdt.assume_utc();
    odt.unix_timestamp() * 1_000
}

/// Return the first day of the month after `d`'s month (next-month boundary).
fn next_month_start(year: i32, month: Month) -> Date {
    let next_month_num = u8::from(month) % 12 + 1;
    let next_year = if next_month_num == 1 { year + 1 } else { year };
    let next_month = Month::try_from(next_month_num).expect("month arithmetic 1-12 always valid");
    Date::from_calendar_date(next_year, next_month, 1).expect("first-of-month always valid")
}

/// Compute expected bars per month for the given interval.
/// Returns `None` for intervals where bar count varies (e.g. `1d`).
fn expected_bars_per_month(year: i32, month: Month, interval: &str) -> Option<usize> {
    let minutes_per_bar: Option<u64> = match interval {
        "1m" => Some(1),
        "5m" => Some(5),
        "15m" => Some(15),
        "1h" => Some(60),
        "4h" => Some(240),
        "1d" => None, // day count varies — skip idempotency check
        _ => None,
    };
    let mins = minutes_per_bar?;

    // Days in month
    let month_start = Date::from_calendar_date(year, month, 1).ok()?;
    let next_start = next_month_start(year, month);
    let days = (next_start - month_start).whole_days() as u64;
    let total_minutes = days * 24 * 60;
    Some((total_minutes / mins) as usize)
}

// ── Paginator ─────────────────────────────────────────────────────────────────

/// Trait so tests can inject a mock fetcher.
#[async_trait::async_trait]
pub trait KlineFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<Kline>>;
}

/// Real HTTP fetcher backed by `reqwest`.
pub struct HttpKlineFetcher {
    client: Client,
}

impl HttpKlineFetcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl KlineFetcher for HttpKlineFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<Kline>> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Binance returned HTTP {status}: {body}"));
        }
        let raw: Vec<RawKline> = resp
            .json()
            .await
            .with_context(|| format!("JSON decode for {url}"))?;
        raw.into_iter()
            .map(|r| r.parse())
            .collect::<Result<Vec<_>>>()
    }
}

/// Paginate over Binance klines for a symbol + month window.
///
/// Returns all klines whose `open_time` falls within `[start_ms, end_ms)`.
/// Sleeps `sleep_ms` between requests to stay under rate-limit budget.
pub async fn paginate_klines(
    fetcher: &dyn KlineFetcher,
    symbol: &str,
    interval: &str,
    start_ms: i64,
    end_ms: i64,
    sleep_ms: u64,
) -> Result<Vec<Kline>> {
    let mut all: Vec<Kline> = Vec::new();
    let mut cursor = start_ms;
    let mut request_count: u32 = 0;

    loop {
        let url = build_klines_url(symbol, interval, cursor, end_ms - 1);
        let batch = fetcher.fetch(&url).await?;
        request_count += 1;

        if batch.is_empty() {
            break;
        }

        let last_close = batch.last().expect("non-empty batch").close_time;
        all.extend(batch);

        // Advance cursor past the last bar's close_time.
        // Binance pagination: next startTime = last_close_time + 1 ms.
        let next_cursor = last_close + 1;
        if next_cursor >= end_ms {
            break;
        }
        cursor = next_cursor;

        if sleep_ms > 0 {
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    }

    info!(
        symbol,
        interval,
        requests = request_count,
        bars = all.len(),
        "paginated klines"
    );
    Ok(all)
}

// ── Parquet writer ────────────────────────────────────────────────────────────

/// Write a `Vec<Kline>` to a Parquet file at `path`.
///
/// Creates parent directories as needed.
pub fn write_parquet(klines: &[Kline], path: &Path) -> Result<()> {
    if klines.is_empty() {
        warn!(?path, "no klines to write — skipping parquet creation");
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create directories for {}", path.display()))?;
    }

    let open_times: Vec<i64> = klines.iter().map(|k| k.open_time).collect();
    let close_times: Vec<i64> = klines.iter().map(|k| k.close_time).collect();
    let opens: Vec<&str> = klines.iter().map(|k| k.open.as_str()).collect();
    let highs: Vec<&str> = klines.iter().map(|k| k.high.as_str()).collect();
    let lows: Vec<&str> = klines.iter().map(|k| k.low.as_str()).collect();
    let closes: Vec<&str> = klines.iter().map(|k| k.close.as_str()).collect();
    let volumes: Vec<&str> = klines.iter().map(|k| k.volume.as_str()).collect();
    let trade_counts: Vec<i64> = klines.iter().map(|k| k.trade_count).collect();

    let mut df = DataFrame::new(vec![
        Column::new("open_time".into(), open_times.as_slice()),
        Column::new("close_time".into(), close_times.as_slice()),
        Column::new("open".into(), opens.as_slice()),
        Column::new("high".into(), highs.as_slice()),
        Column::new("low".into(), lows.as_slice()),
        Column::new("close".into(), closes.as_slice()),
        Column::new("volume".into(), volumes.as_slice()),
        Column::new("trade_count".into(), trade_counts.as_slice()),
    ])
    .with_context(|| format!("build DataFrame for {}", path.display()))?;

    let file = std::fs::File::create(path)
        .with_context(|| format!("create parquet file: {}", path.display()))?;
    let writer = BufWriter::new(file);
    ParquetWriter::new(writer)
        .finish(&mut df)
        .with_context(|| format!("write parquet: {}", path.display()))?;

    info!(path = %path.display(), rows = klines.len(), "wrote parquet");
    Ok(())
}

/// Check idempotency: if file exists and row-count matches expected, skip it.
///
/// Returns `true` if we should skip this month.
fn should_skip(path: &Path, expected_bars: Option<usize>) -> bool {
    if !path.exists() {
        return false;
    }
    let Some(expected) = expected_bars else {
        // Cannot check — skip only if file exists (conservative).
        info!(path = %path.display(), "file exists (bar count unverifiable for this interval) — skipping");
        return true;
    };
    // Try reading row count from the existing parquet.
    match LazyFrame::scan_parquet(path, ScanArgsParquet::default()) {
        Err(e) => {
            warn!(path = %path.display(), error = %e, "could not scan existing parquet — will re-fetch");
            false
        }
        Ok(lf) => match lf.collect() {
            Err(e) => {
                warn!(path = %path.display(), error = %e, "could not collect existing parquet — will re-fetch");
                false
            }
            Ok(df) => {
                let rows = df.height();
                if rows == expected {
                    info!(
                        path = %path.display(),
                        rows,
                        "file exists with expected row count — skipping"
                    );
                    true
                } else {
                    warn!(
                        path = %path.display(),
                        rows,
                        expected,
                        "row count mismatch — will re-fetch"
                    );
                    false
                }
            }
        },
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise tracing; fall back to INFO if RUST_LOG is not set.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    if cli.symbols.is_empty() {
        return Err(anyhow!("--symbols must not be empty"));
    }

    let start_date =
        parse_date(&cli.start).with_context(|| format!("parse --start date: {}", cli.start))?;
    let end_date =
        parse_date(&cli.end).with_context(|| format!("parse --end date: {}", cli.end))?;
    if end_date < start_date {
        return Err(anyhow!("--end must be >= --start"));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")?;
    let fetcher = HttpKlineFetcher::new(client);

    for symbol in &cli.symbols {
        let symbol_upper = symbol.to_uppercase();
        info!(symbol = %symbol_upper, "starting download");

        // Walk over calendar months from start_date to end_date.
        let mut year = start_date.year();
        let mut month = start_date.month();

        loop {
            let month_start =
                Date::from_calendar_date(year, month, 1).expect("month iteration always valid");
            let month_end_exclusive = next_month_start(year, month);

            // Skip months entirely before start_date or after end_date.
            if month_end_exclusive <= start_date || month_start > end_date {
                // Advance month and check loop termination.
                if advance_month(&mut year, &mut month, end_date) {
                    break;
                }
                continue;
            }

            // Clamp window to [start_date, end_date+1).
            let window_start = month_start.max(start_date);
            let window_end = month_end_exclusive.min(
                // end_date is inclusive; add 1 day to make it exclusive.
                end_date.next_day().unwrap_or(end_date),
            );

            let start_ms = date_to_millis(window_start);
            let end_ms = date_to_millis(window_end);

            // Parquet path: <out>/<SYMBOL>/<YEAR>/<MM>.parquet
            let month_num = u8::from(month);
            let parquet_path = cli
                .out
                .join(&symbol_upper)
                .join(year.to_string())
                .join(format!("{month_num:02}.parquet"));

            let expected = expected_bars_per_month(year, month, &cli.interval);

            if !cli.force && should_skip(&parquet_path, expected) {
                if advance_month(&mut year, &mut month, end_date) {
                    break;
                }
                continue;
            }

            info!(
                symbol = %symbol_upper,
                year,
                month = month_num,
                start_ms,
                end_ms,
                "fetching month"
            );

            let klines = paginate_klines(
                &fetcher,
                &symbol_upper,
                &cli.interval,
                start_ms,
                end_ms,
                200, // 200ms between requests → ≤300 req/min, well under limit
            )
            .await
            .with_context(|| format!("fetch klines for {symbol_upper} {year}/{month_num:02}"))?;

            if klines.is_empty() {
                warn!(
                    symbol = %symbol_upper,
                    year,
                    month = month_num,
                    "API returned 0 klines for this month — skipping parquet write"
                );
            } else {
                write_parquet(&klines, &parquet_path).with_context(|| {
                    format!("write parquet for {symbol_upper} {year}/{month_num:02}")
                })?;
                println!(
                    "[OK] {symbol_upper}/{year}/{month_num:02}.parquet  ({} bars)",
                    klines.len()
                );
            }

            if advance_month(&mut year, &mut month, end_date) {
                break;
            }
        }

        info!(symbol = %symbol_upper, "finished download");
    }

    // T-D-3: emit REVISION.toml after all fetches complete.
    if cli.emit_revision_manifest {
        let agg_sha = data::revision::write_revision_manifest(&cli.out)
            .with_context(|| format!("write REVISION.toml in {}", cli.out.display()))?;
        println!(
            "[REVISION] {} written — aggregate SHA: {}",
            cli.out.join("REVISION.toml").display(),
            agg_sha
        );
    }

    Ok(())
}

/// Advance year+month by 1. Returns `true` when we have passed `end_date`'s month.
fn advance_month(year: &mut i32, month: &mut Month, end_date: Date) -> bool {
    let next_num = u8::from(*month) % 12 + 1;
    let next_year = if next_num == 1 { *year + 1 } else { *year };
    *month = Month::try_from(next_num).expect("1-12 always valid");
    *year = next_year;
    // Done when we advance past end_date's month.
    let cur_month_start =
        Date::from_calendar_date(*year, *month, 1).expect("month-start always valid");
    cur_month_start > end_date
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::pedantic
)]
mod tests {
    use super::*;

    // ── Test 1: URL builder ───────────────────────────────────────────────────

    /// Verify `build_klines_url` produces the expected query string.
    #[test]
    fn test_url_builder() {
        let url = build_klines_url("BTCUSDT", "1h", 1_704_067_200_000, 1_706_745_599_999);
        assert_eq!(
            url,
            "https://api.binance.com/api/v3/klines\
?symbol=BTCUSDT&interval=1h&startTime=1704067200000&endTime=1706745599999&limit=1000"
        );
    }

    #[test]
    fn test_url_builder_ethusdt_1m() {
        let url = build_klines_url("ETHUSDT", "1m", 0, 60_000);
        assert!(url.contains("symbol=ETHUSDT"), "symbol in url");
        assert!(url.contains("interval=1m"), "interval in url");
        assert!(url.contains("startTime=0"), "startTime in url");
        assert!(url.contains("endTime=60000"), "endTime in url");
        assert!(url.contains("limit=1000"), "limit in url");
    }

    // ── Test 2: Paginator boundary logic ─────────────────────────────────────

    /// Mock fetcher that simulates paginated responses.
    struct MockFetcher {
        /// Each call returns the next batch from this queue.
        /// When exhausted, returns empty (signals end of data).
        batches: std::sync::Mutex<Vec<Vec<Kline>>>,
        /// Records each URL that was called.
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockFetcher {
        fn new(batches: Vec<Vec<Kline>>) -> Self {
            Self {
                batches: std::sync::Mutex::new(batches),
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn recorded_calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl KlineFetcher for MockFetcher {
        async fn fetch(&self, url: &str) -> Result<Vec<Kline>> {
            self.calls.lock().unwrap().push(url.to_owned());
            let mut batches = self.batches.lock().unwrap();
            if batches.is_empty() {
                Ok(vec![])
            } else {
                Ok(batches.remove(0))
            }
        }
    }

    fn make_kline(open_time: i64, close_time: i64) -> Kline {
        Kline {
            open_time,
            close_time,
            open: "60000.00".to_owned(),
            high: "61000.00".to_owned(),
            low: "59000.00".to_owned(),
            close: "60500.00".to_owned(),
            volume: "10.0".to_owned(),
            trade_count: 42,
        }
    }

    /// Make a batch of `n` klines starting at `start_ms` with `step_ms` spacing.
    fn make_batch(start_ms: i64, step_ms: i64, n: usize) -> Vec<Kline> {
        (0..n)
            .map(|i| {
                let open = start_ms + i as i64 * step_ms;
                let close = open + step_ms - 1;
                make_kline(open, close)
            })
            .collect()
    }

    /// Test: paginator advances cursor to `last_close_time + 1` after a 1000-bar batch.
    #[tokio::test]
    async fn test_paginator_cursor_advances_after_full_batch() {
        // Batch 1: 1000 bars from 0 to 999*3600000 ms (1h interval)
        let step = 3_600_000_i64; // 1 hour in ms
        let batch1 = make_batch(0, step, 1000);
        let last_close = batch1.last().unwrap().close_time; // = 999 * step + step - 1

        // The expected next startTime = last_close + 1.
        let expected_next_cursor = last_close + 1;

        // Batch 2: fewer bars (signals end of data for this window).
        let batch2 = make_batch(expected_next_cursor, step, 5);

        let fetcher = MockFetcher::new(vec![batch1, batch2, vec![]]);

        let end_ms = expected_next_cursor + 5 * step;
        let result = paginate_klines(&fetcher, "BTCUSDT", "1h", 0, end_ms, 0)
            .await
            .expect("pagination should succeed");

        assert_eq!(
            result.len(),
            1005,
            "should collect all bars from both batches"
        );

        let calls = fetcher.recorded_calls();
        assert_eq!(
            calls.len(),
            2,
            "should have made exactly 2 requests (second batch < 1000 means done)"
        );

        // Second call must use expected_next_cursor as startTime.
        assert!(
            calls[1].contains(&format!("startTime={expected_next_cursor}")),
            "second request startTime should be last_close + 1 = {expected_next_cursor}, got: {}",
            calls[1]
        );
    }

    /// Test: paginator stops when API returns empty.
    #[tokio::test]
    async fn test_paginator_stops_on_empty_response() {
        let fetcher = MockFetcher::new(vec![vec![]]);
        let result = paginate_klines(&fetcher, "BTCUSDT", "1h", 0, 3_600_000, 0)
            .await
            .expect("should not error on empty");
        assert!(result.is_empty(), "empty response → no klines");
        assert_eq!(fetcher.recorded_calls().len(), 1, "exactly one call");
    }

    // ── Test 3: Parquet schema round-trip ─────────────────────────────────────

    /// Write a fixture kline Vec to parquet, read it back, assert schema + values.
    #[test]
    fn test_parquet_schema_roundtrip() {
        let klines = vec![
            Kline {
                open_time: 1_704_067_200_000,
                close_time: 1_704_070_799_999,
                open: "42000.00".to_owned(),
                high: "42500.00".to_owned(),
                low: "41800.00".to_owned(),
                close: "42300.00".to_owned(),
                volume: "123.456".to_owned(),
                trade_count: 8_000,
            },
            Kline {
                open_time: 1_704_070_800_000,
                close_time: 1_704_074_399_999,
                open: "42300.00".to_owned(),
                high: "42800.00".to_owned(),
                low: "42100.00".to_owned(),
                close: "42700.00".to_owned(),
                volume: "98.765".to_owned(),
                trade_count: 7_200,
            },
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test_klines.parquet");

        write_parquet(&klines, &path).expect("write_parquet");

        // Read back with polars.
        let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
            .expect("scan parquet")
            .collect()
            .expect("collect");

        // Row count.
        assert_eq!(df.height(), 2, "expected 2 rows");

        // Column count.
        assert_eq!(df.width(), 8, "expected 8 columns");

        // Column types.
        let schema = df.schema();
        assert_eq!(
            schema.get("open_time").cloned(),
            Some(DataType::Int64),
            "open_time must be Int64"
        );
        assert_eq!(
            schema.get("close_time").cloned(),
            Some(DataType::Int64),
            "close_time must be Int64"
        );
        assert_eq!(
            schema.get("open").cloned(),
            Some(DataType::String),
            "open must be Utf8/String"
        );
        assert_eq!(
            schema.get("high").cloned(),
            Some(DataType::String),
            "high must be Utf8/String"
        );
        assert_eq!(
            schema.get("low").cloned(),
            Some(DataType::String),
            "low must be Utf8/String"
        );
        assert_eq!(
            schema.get("close").cloned(),
            Some(DataType::String),
            "close must be Utf8/String"
        );
        assert_eq!(
            schema.get("volume").cloned(),
            Some(DataType::String),
            "volume must be Utf8/String"
        );
        assert_eq!(
            schema.get("trade_count").cloned(),
            Some(DataType::Int64),
            "trade_count must be Int64"
        );

        // Sample row 0 values.
        let open_times = df.column("open_time").unwrap().i64().unwrap();
        assert_eq!(open_times.get(0), Some(1_704_067_200_000_i64));
        assert_eq!(open_times.get(1), Some(1_704_070_800_000_i64));

        let opens = df.column("open").unwrap().str().unwrap();
        assert_eq!(opens.get(0), Some("42000.00"));
        assert_eq!(opens.get(1), Some("42300.00"));

        let trade_counts = df.column("trade_count").unwrap().i64().unwrap();
        assert_eq!(trade_counts.get(0), Some(8_000_i64));
        assert_eq!(trade_counts.get(1), Some(7_200_i64));

        let volumes = df.column("volume").unwrap().str().unwrap();
        assert_eq!(volumes.get(0), Some("123.456"));
    }

    // ── Auxiliary: date / month helpers ───────────────────────────────────────

    #[test]
    fn test_parse_date_valid() {
        let d = parse_date("2024-01-15").expect("valid date");
        assert_eq!(d.year(), 2024);
        assert_eq!(d.month(), Month::January);
        assert_eq!(d.day(), 15);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("not-a-date").is_err());
        assert!(parse_date("2024-13-01").is_err());
    }

    #[test]
    fn test_next_month_start_december() {
        let next = next_month_start(2023, Month::December);
        assert_eq!(next.year(), 2024);
        assert_eq!(next.month(), Month::January);
        assert_eq!(next.day(), 1);
    }

    #[test]
    fn test_next_month_start_january() {
        let next = next_month_start(2024, Month::January);
        assert_eq!(next.year(), 2024);
        assert_eq!(next.month(), Month::February);
    }

    #[test]
    fn test_expected_bars_per_month_1h_jan() {
        // January has 31 days × 24 bars/day = 744 bars at 1h.
        let bars = expected_bars_per_month(2024, Month::January, "1h");
        assert_eq!(bars, Some(744));
    }

    #[test]
    fn test_expected_bars_per_month_1h_feb_leap() {
        // February 2024 is a leap year: 29 days × 24 = 696 bars.
        let bars = expected_bars_per_month(2024, Month::February, "1h");
        assert_eq!(bars, Some(696));
    }

    #[test]
    fn test_expected_bars_per_month_1d_none() {
        // 1d returns None (variable day count per month is fine but we also
        // skip bar-count verification for "1d").
        let bars = expected_bars_per_month(2024, Month::January, "1d");
        assert_eq!(bars, None);
    }
}
