//! `fetch_binance_funding` — Binance USDⓈ-M perp historical funding rates → Parquet.
//!
//! Fetches 8-hour historical funding rates from the public Binance futures REST
//! endpoint (`GET /fapi/v1/fundingRate`) and writes per-symbol-month Parquet
//! files so the backtest harness can align carry data with the OHLCV parquets.
//!
//! # Endpoint
//!
//! `GET https://fapi.binance.com/fapi/v1/fundingRate`
//! Query params: `symbol`, `startTime` (ms), `limit` (max 1000)
//!
//! Pagination is forward by `fundingTime`: the next page uses
//! `startTime = last_funding_time + 1`.
//!
//! # Output layout
//!
//! ```text
//! <out>/<SYMBOL>/<YEAR>/<MONTH-padded>.parquet
//! ```
//!
//! Mirrors the OHLCV layout under `data/binance/` so a future harness loader
//! can extend `realdata.rs` to read funding parquets from the sibling root.
//!
//! # Schema
//!
//! | column        | dtype  | notes                                         |
//! |---------------|--------|-----------------------------------------------|
//! | symbol        | Utf8   | e.g. `"BTCUSDT"`                              |
//! | funding_time  | Int64  | Unix milliseconds of the funding settlement   |
//! | funding_rate  | Utf8   | 8-hour rate string, precision-preserved       |
//!
//! `funding_rate` is stored as string (like OHLCV prices) to preserve the
//! exact Binance decimal representation without floating-point rounding.
//!
//! # Cadence
//!
//! Binance settles funding every 8 hours (00:00, 08:00, 16:00 UTC) →
//! 3 rows/day × 365 days/year ≈ 1095 rows/year per symbol.  Total for
//! 10 symbols × 2 years ≈ 21 900 rows — small data.
//!
//! # Revision manifest
//!
//! Use `--emit-revision-manifest` to pin `REVISION.toml` in `--out` per
//! the ADR-0040 revision-pin precedent.

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

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "fetch_binance_funding",
    about = "Fetch Binance historical funding rates and write Parquet files"
)]
struct Cli {
    /// Comma-separated symbols (USDⓈ-M perp, e.g. BTCUSDT,ETHUSDT)
    #[arg(short = 's', long, value_delimiter = ',')]
    symbols: Vec<String>,

    /// Inclusive start date (YYYY-MM-DD)
    #[arg(long, default_value = "2023-01-01")]
    start: String,

    /// Inclusive end date (YYYY-MM-DD)
    #[arg(long, default_value = "2024-12-31")]
    end: String,

    /// Output root directory
    #[arg(long, default_value = "data/binance-funding")]
    out: PathBuf,

    /// Overwrite existing files (default: skip if file exists with correct row count)
    #[arg(long, default_value_t = false)]
    force: bool,

    /// After all downloads complete, write (or overwrite) a `REVISION.toml`
    /// manifest in `--out` with SHA-256 for every Parquet file present.
    /// Mirrors the ADR-0032 / ADR-0040 revision-pin precedent.
    #[arg(long, default_value_t = false)]
    emit_revision_manifest: bool,

    /// Milliseconds to sleep between pagination requests (rate-limit guard).
    #[arg(long, default_value_t = 200)]
    sleep_ms: u64,
}

// ── Binance funding API types ─────────────────────────────────────────────────

const BINANCE_FUNDING_URL: &str = "https://fapi.binance.com/fapi/v1/fundingRate";
const PAGE_LIMIT: u64 = 1000;

/// One funding rate record from Binance.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFundingRecord {
    symbol: String,
    funding_time: i64,
    funding_rate: String,
    // markPrice is present but we do not store it (advisory, not settlement price)
}

/// Parsed funding record for Parquet output.
#[derive(Debug, Clone)]
pub struct FundingRecord {
    pub symbol: String,
    pub funding_time: i64,
    pub funding_rate: String,
}

// ── URL builder ───────────────────────────────────────────────────────────────

/// Build a Binance funding rate query URL.
///
/// Pure function — no I/O. Used by tests.
pub fn build_funding_url(symbol: &str, start_ms: i64) -> String {
    format!(
        "{BINANCE_FUNDING_URL}?symbol={symbol}&startTime={start_ms}&limit={PAGE_LIMIT}"
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

/// Expected number of funding settlements per month.
///
/// Binance settles 3×/day (every 8 hours). Returns `None` when the precise
/// count cannot be determined without live data (shouldn't happen in practice,
/// but we are conservative).
///
/// Note: We use this for idempotency skip logic only — a ±1 mismatch (e.g.
/// a late settlement timestamp rolling into the next month at Binance) will
/// cause a re-fetch rather than a silent skip. That is the safe direction.
fn expected_settlements_per_month(year: i32, month: Month) -> usize {
    let month_start = Date::from_calendar_date(year, month, 1)
        .expect("first-of-month always valid");
    let next_start = next_month_start(year, month);
    let days = (next_start - month_start).whole_days() as usize;
    days * 3 // 3 settlements per day at 8-hour cadence
}

// ── Paginator ─────────────────────────────────────────────────────────────────

/// Trait so tests can inject a mock fetcher.
#[async_trait::async_trait]
pub trait FundingFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<FundingRecord>>;
}

/// Real HTTP fetcher backed by `reqwest`.
pub struct HttpFundingFetcher {
    client: Client,
}

impl HttpFundingFetcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl FundingFetcher for HttpFundingFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<FundingRecord>> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("Binance rate limited (429) for {url}"));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Binance returned HTTP {status}: {body}"));
        }

        let raw: Vec<RawFundingRecord> = resp
            .json()
            .await
            .with_context(|| format!("JSON decode for {url}"))?;

        Ok(raw
            .into_iter()
            .map(|r| FundingRecord {
                symbol: r.symbol,
                funding_time: r.funding_time,
                funding_rate: r.funding_rate,
            })
            .collect())
    }
}

/// Paginate over Binance funding records for a symbol within `[start_ms, end_ms)`.
///
/// Advances cursor to `last_funding_time + 1` after each page. Stops when
/// the API returns fewer than `PAGE_LIMIT` records (end of data), or when
/// the last record's `funding_time >= end_ms`.
pub async fn paginate_funding(
    fetcher: &dyn FundingFetcher,
    symbol: &str,
    start_ms: i64,
    end_ms: i64,
    sleep_ms: u64,
) -> Result<Vec<FundingRecord>> {
    let mut all: Vec<FundingRecord> = Vec::new();
    let mut cursor = start_ms;
    let mut request_count: u32 = 0;

    loop {
        let url = build_funding_url(symbol, cursor);
        let batch = fetcher.fetch(&url).await?;
        request_count += 1;

        if batch.is_empty() {
            break;
        }

        let last_time = batch.last().expect("non-empty batch").funding_time;

        // Filter to window — Binance may return records slightly before start
        // on the first page if there's a settlement exactly at cursor boundary.
        let in_window: Vec<FundingRecord> = batch
            .into_iter()
            .filter(|r| r.funding_time >= start_ms && r.funding_time < end_ms)
            .collect();

        all.extend(in_window);

        // Advance cursor past the last record. Even if all records were
        // filtered out we still advance to avoid infinite loops.
        let next_cursor = last_time + 1;
        if next_cursor >= end_ms || last_time < cursor {
            // last_time < cursor guard: the API returned stale data (shouldn't
            // happen with well-behaved API but defensive).
            break;
        }
        cursor = next_cursor;

        if sleep_ms > 0 {
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    }

    info!(
        symbol,
        requests = request_count,
        records = all.len(),
        "paginated funding rates"
    );
    Ok(all)
}

// ── Parquet writer ────────────────────────────────────────────────────────────

/// Write a `Vec<FundingRecord>` to a Parquet file at `path`.
///
/// Creates parent directories as needed.
pub fn write_parquet(records: &[FundingRecord], path: &Path) -> Result<()> {
    if records.is_empty() {
        warn!(?path, "no funding records to write — skipping parquet creation");
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create directories for {}", path.display()))?;
    }

    let symbols: Vec<&str> = records.iter().map(|r| r.symbol.as_str()).collect();
    let funding_times: Vec<i64> = records.iter().map(|r| r.funding_time).collect();
    let funding_rates: Vec<&str> = records.iter().map(|r| r.funding_rate.as_str()).collect();

    let mut df = DataFrame::new(vec![
        Column::new("symbol".into(), symbols.as_slice()),
        Column::new("funding_time".into(), funding_times.as_slice()),
        Column::new("funding_rate".into(), funding_rates.as_slice()),
    ])
    .with_context(|| format!("build DataFrame for {}", path.display()))?;

    let file = std::fs::File::create(path)
        .with_context(|| format!("create parquet file: {}", path.display()))?;
    let writer = BufWriter::new(file);
    ParquetWriter::new(writer)
        .finish(&mut df)
        .with_context(|| format!("write parquet: {}", path.display()))?;

    info!(path = %path.display(), rows = records.len(), "wrote funding parquet");
    Ok(())
}

/// Check idempotency: if file exists and row-count matches expected, skip it.
///
/// Returns `true` if we should skip this month.
fn should_skip(path: &Path, expected_rows: usize) -> bool {
    if !path.exists() {
        return false;
    }
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
                if rows == expected_rows {
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
                        expected_rows,
                        "row count mismatch — will re-fetch"
                    );
                    false
                }
            }
        },
    }
}

// ── Advance month helper ──────────────────────────────────────────────────────

/// Advance year+month by 1. Returns `true` when we have passed `end_date`'s month.
fn advance_month(year: &mut i32, month: &mut Month, end_date: Date) -> bool {
    let next_num = u8::from(*month) % 12 + 1;
    let next_year = if next_num == 1 { *year + 1 } else { *year };
    *month = Month::try_from(next_num).expect("1-12 always valid");
    *year = next_year;
    let cur_month_start =
        Date::from_calendar_date(*year, *month, 1).expect("month-start always valid");
    cur_month_start > end_date
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Reuse llm::tracing_init::install_global (same as fetch_binance_klines).
    llm::tracing_init::install_global(&[], false)?;

    let cli = Cli::parse();

    if cli.symbols.is_empty() {
        return Err(anyhow!("--symbols must not be empty"));
    }

    let start_date = parse_date(&cli.start)
        .with_context(|| format!("parse --start date: {}", cli.start))?;
    let end_date = parse_date(&cli.end)
        .with_context(|| format!("parse --end date: {}", cli.end))?;
    if end_date < start_date {
        return Err(anyhow!("--end must be >= --start"));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")?;
    let fetcher = HttpFundingFetcher::new(client);

    for symbol in &cli.symbols {
        let symbol_upper = symbol.to_uppercase();
        info!(symbol = %symbol_upper, "starting funding download");

        let mut year = start_date.year();
        let mut month = start_date.month();

        loop {
            let month_start =
                Date::from_calendar_date(year, month, 1).expect("month iteration always valid");
            let month_end_exclusive = next_month_start(year, month);

            // Skip months entirely before start_date or after end_date.
            if month_end_exclusive <= start_date || month_start > end_date {
                if advance_month(&mut year, &mut month, end_date) {
                    break;
                }
                continue;
            }

            // Clamp window to [start_date, end_date+1).
            let window_start = month_start.max(start_date);
            let window_end = month_end_exclusive
                .min(end_date.next_day().unwrap_or(end_date));

            let start_ms = date_to_millis(window_start);
            let end_ms = date_to_millis(window_end);

            // Parquet path: <out>/<SYMBOL>/<YEAR>/<MM>.parquet
            let month_num = u8::from(month);
            let parquet_path = cli
                .out
                .join(&symbol_upper)
                .join(year.to_string())
                .join(format!("{month_num:02}.parquet"));

            let expected = expected_settlements_per_month(year, month);

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
                expected_rows = expected,
                "fetching month funding rates"
            );

            let records = paginate_funding(
                &fetcher,
                &symbol_upper,
                start_ms,
                end_ms,
                cli.sleep_ms,
            )
            .await
            .with_context(|| {
                format!("fetch funding for {symbol_upper} {year}/{month_num:02}")
            })?;

            if records.is_empty() {
                warn!(
                    symbol = %symbol_upper,
                    year,
                    month = month_num,
                    "API returned 0 records for this month — skipping parquet write"
                );
            } else {
                write_parquet(&records, &parquet_path).with_context(|| {
                    format!("write parquet for {symbol_upper} {year}/{month_num:02}")
                })?;
                println!(
                    "[OK] {symbol_upper}/{year}/{month_num:02}.parquet  ({} records)",
                    records.len()
                );
            }

            if advance_month(&mut year, &mut month, end_date) {
                break;
            }
        }

        info!(symbol = %symbol_upper, "finished funding download");
    }

    // Emit REVISION.toml after all fetches complete.
    if cli.emit_revision_manifest {
        let agg_sha = data::revision::write_revision_manifest_with_tool(
            &cli.out,
            data::revision::RevisionMetadataInput {
                fetch_tool: "fetch_binance_funding",
                binance_base: "https://fapi.binance.com",
                interval: None, // funding is event-driven (8h), not a bar interval
            },
        )
        .with_context(|| format!("write REVISION.toml in {}", cli.out.display()))?;
        println!(
            "[REVISION] {} written — aggregate SHA: {}",
            cli.out.join("REVISION.toml").display(),
            agg_sha
        );
    }

    Ok(())
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

    // ── URL builder ───────────────────────────────────────────────────────────

    #[test]
    fn test_url_builder_basic() {
        let url = build_funding_url("BTCUSDT", 1_672_531_200_000);
        assert!(url.contains("symbol=BTCUSDT"), "symbol in url: {url}");
        assert!(
            url.contains("startTime=1672531200000"),
            "startTime in url: {url}"
        );
        assert!(url.contains("limit=1000"), "limit in url: {url}");
        assert!(
            url.starts_with("https://fapi.binance.com/fapi/v1/fundingRate"),
            "correct base url: {url}"
        );
    }

    #[test]
    fn test_url_builder_ethusdt() {
        let url = build_funding_url("ETHUSDT", 0);
        assert!(url.contains("symbol=ETHUSDT"));
        assert!(url.contains("startTime=0"));
    }

    // ── Expected settlements per month ────────────────────────────────────────

    #[test]
    fn test_expected_settlements_jan() {
        // January 2023: 31 days × 3 = 93
        assert_eq!(expected_settlements_per_month(2023, Month::January), 93);
    }

    #[test]
    fn test_expected_settlements_feb_leap() {
        // February 2024 (leap): 29 × 3 = 87
        assert_eq!(expected_settlements_per_month(2024, Month::February), 87);
    }

    #[test]
    fn test_expected_settlements_feb_non_leap() {
        // February 2023: 28 × 3 = 84
        assert_eq!(expected_settlements_per_month(2023, Month::February), 84);
    }

    #[test]
    fn test_expected_settlements_dec() {
        // December: 31 × 3 = 93
        assert_eq!(expected_settlements_per_month(2023, Month::December), 93);
    }

    // ── Mock fetcher + paginator ──────────────────────────────────────────────

    struct MockFetcher {
        batches: std::sync::Mutex<Vec<Vec<FundingRecord>>>,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockFetcher {
        fn new(batches: Vec<Vec<FundingRecord>>) -> Self {
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
    impl FundingFetcher for MockFetcher {
        async fn fetch(&self, url: &str) -> Result<Vec<FundingRecord>> {
            self.calls.lock().unwrap().push(url.to_owned());
            let mut batches = self.batches.lock().unwrap();
            if batches.is_empty() {
                Ok(vec![])
            } else {
                Ok(batches.remove(0))
            }
        }
    }

    fn make_record(symbol: &str, funding_time: i64, rate: &str) -> FundingRecord {
        FundingRecord {
            symbol: symbol.to_string(),
            funding_time,
            funding_rate: rate.to_string(),
        }
    }

    /// 8h in milliseconds (one funding interval).
    const EIGHT_HOURS_MS: i64 = 8 * 3_600_000;

    /// Build a batch of `n` sequential funding records starting at `start_ms`.
    fn make_batch(symbol: &str, start_ms: i64, n: usize) -> Vec<FundingRecord> {
        (0..n)
            .map(|i| make_record(symbol, start_ms + i as i64 * EIGHT_HOURS_MS, "0.00010000"))
            .collect()
    }

    /// Paginator stops on empty response.
    #[tokio::test]
    async fn test_paginator_stops_on_empty() {
        let fetcher = MockFetcher::new(vec![vec![]]);
        let result = paginate_funding(&fetcher, "BTCUSDT", 0, 1_000_000, 0)
            .await
            .expect("should not error on empty");
        assert!(result.is_empty(), "empty response → no records");
        assert_eq!(fetcher.recorded_calls().len(), 1, "exactly one call");
    }

    /// Paginator collects from two pages and advances cursor correctly.
    #[tokio::test]
    async fn test_paginator_two_pages() {
        let start_ms = 1_672_531_200_000_i64; // 2023-01-01 00:00 UTC
        // Page 1: 1000 records (full page → there might be more)
        let batch1 = make_batch("BTCUSDT", start_ms, 1000);
        let last_of_page1 = batch1.last().unwrap().funding_time;
        let next_cursor = last_of_page1 + 1;

        // Page 2: 93 records (partial → end of data for month)
        let batch2 = make_batch("BTCUSDT", next_cursor, 93);

        let end_ms = next_cursor + 93 * EIGHT_HOURS_MS + 1;

        let fetcher = MockFetcher::new(vec![batch1, batch2, vec![]]);
        let result = paginate_funding(&fetcher, "BTCUSDT", start_ms, end_ms, 0)
            .await
            .expect("paginator should succeed");

        assert_eq!(result.len(), 1093, "1000 + 93 records");

        let calls = fetcher.recorded_calls();
        // Third call should return empty (we broke out after page 2 which ends before end_ms,
        // but since batch2 has < 1000 we still call once more to confirm empty).
        // Actually with our logic: last record of batch2 < end_ms, so we continue and get empty.
        assert!(calls.len() >= 2, "at least 2 requests");

        // Second call must start at next_cursor.
        assert!(
            calls[1].contains(&format!("startTime={next_cursor}")),
            "second request must use cursor={next_cursor}, got: {}",
            calls[1]
        );
    }

    /// Paginator filters records outside the window.
    #[tokio::test]
    async fn test_paginator_filters_out_of_window() {
        // Start halfway through a day; first batch includes one record before window.
        let window_start = 1_672_560_000_008_i64; // 2023-01-01 08:00:00.008
        let window_end = window_start + 3 * EIGHT_HOURS_MS;

        // Batch has a record before window start (at window_start - 1)
        let out_of_window = make_record("BTCUSDT", window_start - 1, "0.00010000");
        let in_window1 = make_record("BTCUSDT", window_start, "0.00010000");
        let in_window2 = make_record("BTCUSDT", window_start + EIGHT_HOURS_MS, "0.00020000");
        let at_boundary = make_record("BTCUSDT", window_end - 1, "0.00030000");
        let after_window = make_record("BTCUSDT", window_end, "0.00040000");

        let batch = vec![
            out_of_window,
            in_window1,
            in_window2,
            at_boundary,
            after_window,
        ];

        let fetcher = MockFetcher::new(vec![batch]);
        let result = paginate_funding(&fetcher, "BTCUSDT", window_start, window_end, 0)
            .await
            .expect("should succeed");

        // Should keep records in [window_start, window_end), filter the rest.
        assert_eq!(result.len(), 3, "3 records in window");
        assert_eq!(result[0].funding_time, window_start);
        assert_eq!(result[1].funding_time, window_start + EIGHT_HOURS_MS);
        assert_eq!(result[2].funding_time, window_end - 1);
    }

    // ── Parquet schema round-trip ─────────────────────────────────────────────

    #[test]
    fn test_parquet_schema_roundtrip() {
        let records = vec![
            FundingRecord {
                symbol: "BTCUSDT".to_string(),
                funding_time: 1_672_531_200_000,
                funding_rate: "0.00010000".to_string(),
            },
            FundingRecord {
                symbol: "BTCUSDT".to_string(),
                funding_time: 1_672_560_000_000,
                funding_rate: "-0.00005000".to_string(),
            },
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test_funding.parquet");

        write_parquet(&records, &path).expect("write_parquet");

        let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
            .expect("scan parquet")
            .collect()
            .expect("collect");

        assert_eq!(df.height(), 2, "2 rows");
        assert_eq!(df.width(), 3, "3 columns");

        let schema = df.schema();
        assert_eq!(schema.get("symbol").cloned(), Some(DataType::String));
        assert_eq!(schema.get("funding_time").cloned(), Some(DataType::Int64));
        assert_eq!(schema.get("funding_rate").cloned(), Some(DataType::String));

        let times = df.column("funding_time").unwrap().i64().unwrap();
        assert_eq!(times.get(0), Some(1_672_531_200_000_i64));
        assert_eq!(times.get(1), Some(1_672_560_000_000_i64));

        let rates = df.column("funding_rate").unwrap().str().unwrap();
        assert_eq!(rates.get(0), Some("0.00010000"));
        assert_eq!(rates.get(1), Some("-0.00005000"));

        let syms = df.column("symbol").unwrap().str().unwrap();
        assert_eq!(syms.get(0), Some("BTCUSDT"));
    }

    // ── Date helpers ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_date_valid() {
        let d = parse_date("2023-01-01").expect("valid date");
        assert_eq!(d.year(), 2023);
        assert_eq!(d.month(), Month::January);
        assert_eq!(d.day(), 1);
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
    }

    #[test]
    fn test_date_to_millis_epoch() {
        // 2023-01-01 00:00:00 UTC = 1672531200000 ms
        let d = Date::from_calendar_date(2023, Month::January, 1).unwrap();
        assert_eq!(date_to_millis(d), 1_672_531_200_000);
    }
}
