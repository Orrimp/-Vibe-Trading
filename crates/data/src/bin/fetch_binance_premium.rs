//! `fetch_binance_premium` — Binance USDⓈ-M perp **premium-index klines** → Parquet.
//!
//! Fetches the historical *premium index* (the perpetual mark-vs-spot premium,
//! a.k.a. the **basis**) from the public Binance futures REST endpoint
//! (`GET /fapi/v1/premiumIndexKlines`) and writes per-symbol-month Parquet files
//! aligned to the **same hourly bar grid** as the OHLCV parquets under
//! `data/binance/`.
//!
//! # Why this endpoint (the cheapest valid basis source)
//!
//! The premium index kline's close = `(markPrice − indexPrice) / indexPrice`
//! already computed by Binance and bucketed to arbitrary intervals (here `1h`).
//! That IS the basis at each hourly bar, natively aligned to the OHLCV open-time
//! grid — no separate `markPrice`/`indexPrice` fetch and no manual division.
//! (Contrast `fetch_binance_funding`'s `GET /fapi/v1/fundingRate`, which only
//! exposes `markPrice` — not `indexPrice` — at the sparse 8-hour funding cadence;
//! it cannot reconstruct the basis on the hourly grid.) `premiumIndexKlines` is
//! free, unauthenticated, and full-history — the same `/fapi/v1/*` family as
//! funding, so it sidesteps the 30-day `futures/data/*` history cap.
//!
//! # Endpoint
//!
//! `GET https://fapi.binance.com/fapi/v1/premiumIndexKlines`
//! Query params: `symbol`, `interval`, `startTime` (ms), `endTime` (ms),
//! `limit` (max 1500).
//!
//! Response is a kline array-of-arrays. Indices used:
//! ```text
//! 0  open_time   (i64  ms)
//! 1  open        (String)   ← premium index open
//! 2  high        (String)   ← premium index high
//! 3  low         (String)   ← premium index low
//! 4  close       (String)   ← premium index close (the basis at this bar)
//! 6  close_time  (i64  ms)
//! ```
//! Indices 5 and 7-11 are present but advisory (no real volume for an index
//! series) and are not stored.
//!
//! Pagination is forward by `close_time`: the next page uses
//! `startTime = last_close_time + 1`.
//!
//! # Output layout
//!
//! ```text
//! <out>/<SYMBOL>/<YEAR>/<MONTH-padded>.parquet
//! ```
//!
//! Mirrors the OHLCV / funding layout so a future harness loader can align the
//! basis with the OHLCV parquets.
//!
//! # Schema
//!
//! | column      | dtype | notes                                            |
//! |-------------|-------|--------------------------------------------------|
//! | open_time   | Int64 | Unix ms, bar open (matches OHLCV `open_time`)    |
//! | close_time  | Int64 | Unix ms, bar close                               |
//! | basis_open  | Utf8  | premium-index open, decimal string (signed)      |
//! | basis_high  | Utf8  | premium-index high, decimal string (signed)      |
//! | basis_low   | Utf8  | premium-index low, decimal string (signed)       |
//! | basis_close | Utf8  | premium-index close = the basis at this bar      |
//!
//! Premium values are stored as strings (like OHLCV prices) to preserve the
//! exact Binance decimal representation; they are SIGNED (negative when the
//! perp trades below spot index).
//!
//! # Revision manifest
//!
//! Use `--emit-revision-manifest` to pin `REVISION.toml` in `--out` per the
//! ADR-0040 revision-pin precedent (mirrors `data/binance-funding`).

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
    name = "fetch_binance_premium",
    about = "Fetch Binance premium-index (perp basis) klines and write Parquet files"
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

    /// Binance interval string: 1m, 5m, 15m, 1h, 4h, 1d
    #[arg(long, default_value = "1h")]
    interval: String,

    /// Output root directory
    #[arg(long, default_value = "data/binance-basis")]
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

// ── Binance premium-index kline types ─────────────────────────────────────────

const BINANCE_PREMIUM_URL: &str = "https://fapi.binance.com/fapi/v1/premiumIndexKlines";
const PAGE_LIMIT: u64 = 1500;

/// One premium-index kline (the basis bar) parsed from Binance's array form.
#[derive(Debug, Clone)]
pub struct PremiumKline {
    pub open_time: i64,
    pub close_time: i64,
    pub basis_open: String,
    pub basis_high: String,
    pub basis_low: String,
    pub basis_close: String,
}

/// Intermediate JSON representation for a premium-index kline array element.
/// Binance uses a heterogeneous JSON array; we deserialize to `Value` first.
#[derive(Deserialize)]
struct RawPremiumKline(serde_json::Value);

impl RawPremiumKline {
    fn parse(self) -> Result<PremiumKline> {
        let arr = self
            .0
            .as_array()
            .ok_or_else(|| anyhow!("premium kline element is not an array"))?;
        if arr.len() < 7 {
            return Err(anyhow!("premium kline array too short: len={}", arr.len()));
        }
        let open_time = arr[0]
            .as_i64()
            .ok_or_else(|| anyhow!("open_time not i64"))?;
        let basis_open = arr[1]
            .as_str()
            .ok_or_else(|| anyhow!("open not str"))?
            .to_owned();
        let basis_high = arr[2]
            .as_str()
            .ok_or_else(|| anyhow!("high not str"))?
            .to_owned();
        let basis_low = arr[3]
            .as_str()
            .ok_or_else(|| anyhow!("low not str"))?
            .to_owned();
        let basis_close = arr[4]
            .as_str()
            .ok_or_else(|| anyhow!("close not str"))?
            .to_owned();
        let close_time = arr[6]
            .as_i64()
            .ok_or_else(|| anyhow!("close_time not i64"))?;
        Ok(PremiumKline {
            open_time,
            close_time,
            basis_open,
            basis_high,
            basis_low,
            basis_close,
        })
    }
}

// ── URL builder ───────────────────────────────────────────────────────────────

/// Build a Binance premium-index kline query URL.
///
/// Pure function — no I/O. Used by tests.
pub fn build_premium_url(symbol: &str, interval: &str, start_ms: i64, end_ms: i64) -> String {
    format!(
        "{BINANCE_PREMIUM_URL}?symbol={symbol}&interval={interval}&startTime={start_ms}&endTime={end_ms}&limit={PAGE_LIMIT}"
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
        "1d" => None,
        _ => None,
    };
    let mins = minutes_per_bar?;
    let month_start = Date::from_calendar_date(year, month, 1).ok()?;
    let next_start = next_month_start(year, month);
    let days = (next_start - month_start).whole_days() as u64;
    let total_minutes = days * 24 * 60;
    Some((total_minutes / mins) as usize)
}

// ── Paginator ─────────────────────────────────────────────────────────────────

/// Trait so tests can inject a mock fetcher.
#[async_trait::async_trait]
pub trait PremiumFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<PremiumKline>>;
}

/// Real HTTP fetcher backed by `reqwest`.
pub struct HttpPremiumFetcher {
    client: Client,
}

impl HttpPremiumFetcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl PremiumFetcher for HttpPremiumFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<PremiumKline>> {
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

        let raw: Vec<RawPremiumKline> = resp
            .json()
            .await
            .with_context(|| format!("JSON decode for {url}"))?;
        raw.into_iter()
            .map(RawPremiumKline::parse)
            .collect::<Result<Vec<_>>>()
    }
}

/// Paginate over Binance premium-index klines for a symbol + month window.
///
/// Returns all klines whose `open_time` falls within `[start_ms, end_ms)`.
/// Advances cursor to `last_close_time + 1` after each page.
pub async fn paginate_premium(
    fetcher: &dyn PremiumFetcher,
    symbol: &str,
    interval: &str,
    start_ms: i64,
    end_ms: i64,
    sleep_ms: u64,
) -> Result<Vec<PremiumKline>> {
    let mut all: Vec<PremiumKline> = Vec::new();
    let mut cursor = start_ms;
    let mut request_count: u32 = 0;

    loop {
        let url = build_premium_url(symbol, interval, cursor, end_ms - 1);
        let batch = fetcher.fetch(&url).await?;
        request_count += 1;

        if batch.is_empty() {
            break;
        }

        let last_close = batch.last().expect("non-empty batch").close_time;
        // Keep only bars whose open_time is in-window (defensive; Binance may
        // return a boundary bar slightly outside on the first page).
        let in_window: Vec<PremiumKline> = batch
            .into_iter()
            .filter(|k| k.open_time >= start_ms && k.open_time < end_ms)
            .collect();
        all.extend(in_window);

        let next_cursor = last_close + 1;
        if next_cursor >= end_ms || last_close < cursor {
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
        "paginated premium-index klines"
    );
    Ok(all)
}

// ── Parquet writer ────────────────────────────────────────────────────────────

/// Write a `Vec<PremiumKline>` to a Parquet file at `path`.
///
/// Creates parent directories as needed.
pub fn write_parquet(klines: &[PremiumKline], path: &Path) -> Result<()> {
    if klines.is_empty() {
        warn!(
            ?path,
            "no premium klines to write — skipping parquet creation"
        );
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create directories for {}", path.display()))?;
    }

    let open_times: Vec<i64> = klines.iter().map(|k| k.open_time).collect();
    let close_times: Vec<i64> = klines.iter().map(|k| k.close_time).collect();
    let opens: Vec<&str> = klines.iter().map(|k| k.basis_open.as_str()).collect();
    let highs: Vec<&str> = klines.iter().map(|k| k.basis_high.as_str()).collect();
    let lows: Vec<&str> = klines.iter().map(|k| k.basis_low.as_str()).collect();
    let closes: Vec<&str> = klines.iter().map(|k| k.basis_close.as_str()).collect();

    let mut df = DataFrame::new(vec![
        Column::new("open_time".into(), open_times.as_slice()),
        Column::new("close_time".into(), close_times.as_slice()),
        Column::new("basis_open".into(), opens.as_slice()),
        Column::new("basis_high".into(), highs.as_slice()),
        Column::new("basis_low".into(), lows.as_slice()),
        Column::new("basis_close".into(), closes.as_slice()),
    ])
    .with_context(|| format!("build DataFrame for {}", path.display()))?;

    let file = std::fs::File::create(path)
        .with_context(|| format!("create parquet file: {}", path.display()))?;
    let writer = BufWriter::new(file);
    ParquetWriter::new(writer)
        .finish(&mut df)
        .with_context(|| format!("write parquet: {}", path.display()))?;

    info!(path = %path.display(), rows = klines.len(), "wrote premium parquet");
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
        info!(path = %path.display(), "file exists (bar count unverifiable for this interval) — skipping");
        return true;
    };
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
                    info!(path = %path.display(), rows, "file exists with expected row count — skipping");
                    true
                } else {
                    warn!(path = %path.display(), rows, expected, "row count mismatch — will re-fetch");
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
    llm::tracing_init::install_global(&[], false)?;

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
    let fetcher = HttpPremiumFetcher::new(client);

    for symbol in &cli.symbols {
        let symbol_upper = symbol.to_uppercase();
        info!(symbol = %symbol_upper, "starting premium-index download");

        let mut year = start_date.year();
        let mut month = start_date.month();

        loop {
            let month_start =
                Date::from_calendar_date(year, month, 1).expect("month iteration always valid");
            let month_end_exclusive = next_month_start(year, month);

            if month_end_exclusive <= start_date || month_start > end_date {
                if advance_month(&mut year, &mut month, end_date) {
                    break;
                }
                continue;
            }

            let window_start = month_start.max(start_date);
            let window_end = month_end_exclusive.min(end_date.next_day().unwrap_or(end_date));

            let start_ms = date_to_millis(window_start);
            let end_ms = date_to_millis(window_end);

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

            info!(symbol = %symbol_upper, year, month = month_num, start_ms, end_ms, "fetching month premium index");

            let klines = paginate_premium(
                &fetcher,
                &symbol_upper,
                &cli.interval,
                start_ms,
                end_ms,
                cli.sleep_ms,
            )
            .await
            .with_context(|| format!("fetch premium for {symbol_upper} {year}/{month_num:02}"))?;

            if klines.is_empty() {
                warn!(symbol = %symbol_upper, year, month = month_num, "API returned 0 klines for this month — skipping parquet write");
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

        info!(symbol = %symbol_upper, "finished premium-index download");
    }

    if cli.emit_revision_manifest {
        let agg_sha = data::revision::write_revision_manifest_with_tool(
            &cli.out,
            data::revision::RevisionMetadataInput {
                fetch_tool: "fetch_binance_premium",
                binance_base: "https://fapi.binance.com",
                interval: Some(cli.interval.as_str()),
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

    #[test]
    fn test_url_builder_basic() {
        let url = build_premium_url("BTCUSDT", "1h", 1_672_531_200_000, 1_675_209_599_999);
        assert!(url.contains("symbol=BTCUSDT"), "symbol in url: {url}");
        assert!(url.contains("interval=1h"), "interval in url: {url}");
        assert!(
            url.contains("startTime=1672531200000"),
            "startTime in url: {url}"
        );
        assert!(url.contains("limit=1500"), "limit in url: {url}");
        assert!(
            url.starts_with("https://fapi.binance.com/fapi/v1/premiumIndexKlines"),
            "correct base url: {url}"
        );
    }

    #[test]
    fn test_expected_bars_1h_jan() {
        // January: 31 days × 24 = 744 hourly bars.
        assert_eq!(
            expected_bars_per_month(2023, Month::January, "1h"),
            Some(744)
        );
    }

    #[test]
    fn test_expected_bars_1h_feb_leap() {
        // February 2024 (leap): 29 × 24 = 696.
        assert_eq!(
            expected_bars_per_month(2024, Month::February, "1h"),
            Some(696)
        );
    }

    // ── Raw parse: a signed (negative) premium must round-trip ────────────────

    #[test]
    fn test_raw_parse_negative_premium() {
        // Binance returns the premium index as a heterogeneous JSON array.
        let json = serde_json::json!([
            1_672_531_200_000_i64, // open_time
            "-0.00012300",         // open  (negative: perp below spot)
            "0.00005000",          // high
            "-0.00030000",         // low
            "-0.00008800",         // close (the basis at this bar)
            "0",                   // ignored
            1_672_534_799_999_i64, // close_time
            "0",                   // ignored
            0_i64,                 // ignored
            "0",                   // ignored
            "0",                   // ignored
            "0"                    // ignored
        ]);
        let raw = RawPremiumKline(json);
        let k = raw.parse().expect("parse");
        assert_eq!(k.open_time, 1_672_531_200_000);
        assert_eq!(k.close_time, 1_672_534_799_999);
        assert_eq!(k.basis_open, "-0.00012300");
        assert_eq!(k.basis_high, "0.00005000");
        assert_eq!(k.basis_low, "-0.00030000");
        assert_eq!(k.basis_close, "-0.00008800");
    }

    #[test]
    fn test_raw_parse_too_short_errors() {
        let json = serde_json::json!([1_672_531_200_000_i64, "0.0", "0.0"]);
        let raw = RawPremiumKline(json);
        assert!(raw.parse().is_err(), "array too short must error");
    }

    // ── Mock fetcher + paginator ──────────────────────────────────────────────

    struct MockFetcher {
        batches: std::sync::Mutex<Vec<Vec<PremiumKline>>>,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockFetcher {
        fn new(batches: Vec<Vec<PremiumKline>>) -> Self {
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
    impl PremiumFetcher for MockFetcher {
        async fn fetch(&self, url: &str) -> Result<Vec<PremiumKline>> {
            self.calls.lock().unwrap().push(url.to_owned());
            let mut batches = self.batches.lock().unwrap();
            if batches.is_empty() {
                Ok(vec![])
            } else {
                Ok(batches.remove(0))
            }
        }
    }

    const ONE_HOUR_MS: i64 = 3_600_000;

    fn make_bar(open_time: i64) -> PremiumKline {
        PremiumKline {
            open_time,
            close_time: open_time + ONE_HOUR_MS - 1,
            basis_open: "0.00010000".to_string(),
            basis_high: "0.00020000".to_string(),
            basis_low: "0.00005000".to_string(),
            basis_close: "0.00015000".to_string(),
        }
    }

    fn make_batch(start_ms: i64, n: usize) -> Vec<PremiumKline> {
        (0..n)
            .map(|i| make_bar(start_ms + i as i64 * ONE_HOUR_MS))
            .collect()
    }

    #[tokio::test]
    async fn test_paginator_stops_on_empty() {
        let fetcher = MockFetcher::new(vec![vec![]]);
        let result = paginate_premium(&fetcher, "BTCUSDT", "1h", 0, 1_000_000, 0)
            .await
            .expect("should not error on empty");
        assert!(result.is_empty(), "empty response → no bars");
        assert_eq!(fetcher.recorded_calls().len(), 1, "exactly one call");
    }

    #[tokio::test]
    async fn test_paginator_two_pages_advances_cursor() {
        let start_ms = 1_672_531_200_000_i64; // 2023-01-01 00:00 UTC
        let batch1 = make_batch(start_ms, 1500);
        let last_close1 = batch1.last().unwrap().close_time;
        let next_cursor = last_close1 + 1;
        let batch2 = make_batch(next_cursor, 100);
        let end_ms = next_cursor + 100 * ONE_HOUR_MS + 1;

        let fetcher = MockFetcher::new(vec![batch1, batch2, vec![]]);
        let result = paginate_premium(&fetcher, "BTCUSDT", "1h", start_ms, end_ms, 0)
            .await
            .expect("paginator should succeed");

        assert_eq!(result.len(), 1600, "1500 + 100 bars in-window");
        let calls = fetcher.recorded_calls();
        assert!(calls.len() >= 2, "at least 2 requests");
        assert!(
            calls[1].contains(&format!("startTime={next_cursor}")),
            "second request must use cursor={next_cursor}, got: {}",
            calls[1]
        );
    }

    #[tokio::test]
    async fn test_paginator_filters_out_of_window() {
        let window_start = 1_672_531_200_000_i64;
        let window_end = window_start + 3 * ONE_HOUR_MS;
        let before = make_bar(window_start - ONE_HOUR_MS);
        let in1 = make_bar(window_start);
        let in2 = make_bar(window_start + ONE_HOUR_MS);
        let in3 = make_bar(window_start + 2 * ONE_HOUR_MS);
        let after = make_bar(window_end);

        let fetcher = MockFetcher::new(vec![vec![before, in1, in2, in3, after]]);
        let result = paginate_premium(&fetcher, "BTCUSDT", "1h", window_start, window_end, 0)
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 3, "3 bars in [start, end)");
        assert_eq!(result[0].open_time, window_start);
        assert_eq!(result[2].open_time, window_start + 2 * ONE_HOUR_MS);
    }

    // ── Parquet schema round-trip ─────────────────────────────────────────────

    #[test]
    fn test_parquet_schema_roundtrip() {
        let klines = vec![
            make_bar(1_672_531_200_000),
            PremiumKline {
                open_time: 1_672_534_800_000,
                close_time: 1_672_538_399_999,
                basis_open: "-0.00005000".to_string(),
                basis_high: "0.00001000".to_string(),
                basis_low: "-0.00009000".to_string(),
                basis_close: "-0.00007000".to_string(),
            },
        ];

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test_premium.parquet");
        write_parquet(&klines, &path).expect("write_parquet");

        let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
            .expect("scan parquet")
            .collect()
            .expect("collect");

        assert_eq!(df.height(), 2, "2 rows");
        assert_eq!(df.width(), 6, "6 columns");

        let schema = df.schema();
        assert_eq!(schema.get("open_time").cloned(), Some(DataType::Int64));
        assert_eq!(schema.get("close_time").cloned(), Some(DataType::Int64));
        assert_eq!(schema.get("basis_open").cloned(), Some(DataType::String));
        assert_eq!(schema.get("basis_close").cloned(), Some(DataType::String));

        let closes = df.column("basis_close").unwrap().str().unwrap();
        assert_eq!(closes.get(0), Some("0.00015000"));
        assert_eq!(closes.get(1), Some("-0.00007000"));
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
    fn test_date_to_millis_epoch() {
        let d = Date::from_calendar_date(2023, Month::January, 1).unwrap();
        assert_eq!(date_to_millis(d), 1_672_531_200_000);
    }
}
