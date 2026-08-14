//! `fetch_deribit_dvol` — Deribit implied-vol index (DVOL) → Parquet.
//!
//! Fetches the historical **DVOL implied-volatility index** from the free,
//! no-auth Deribit public REST endpoint
//! (`GET /api/v2/public/get_volatility_index_data`) for BTC and ETH, writes
//! per-symbol/per-year Parquet files under `data/deribit-dvol/`.
//!
//! # Why this endpoint
//!
//! DVOL is a live, deterministic order-book function (variance-swap construction
//! over the two expiries bracketing 30 days). It is PIT-clean by construction —
//! the same PIT class as the perp basis (`fetch_binance_premium`) and OHLCV.
//! The `/public/` namespace needs no authentication. Full history reaches back
//! to 2021-04, covering the 2023-2024 robustness window.
//!
//! # Endpoint
//!
//! `GET https://www.deribit.com/api/v2/public/get_volatility_index_data`
//! Query params:
//! - `currency`:        `BTC` or `ETH`
//! - `start_timestamp`: Unix ms (inclusive)
//! - `end_timestamp`:   Unix ms (inclusive)
//! - `resolution`:      `43200` (12h candles; two per day → daily close)
//!
//! Response envelope: `{ "result": { "data": [[ts_ms, open, high, low, close], ...], "continuation": N_or_null } }`
//!
//! Pagination: advance `start_timestamp = continuation` (the first timestamp of
//! the next page); stop when `data` is empty or `continuation` is null.
//!
//! # Output layout
//!
//! ```text
//! data/deribit-dvol/<SYM>/<YEAR>.parquet
//! ```
//!
//! Where `<SYM> ∈ {BTC, ETH}` and `<YEAR> ∈ {2023, 2024}`.
//!
//! # Schema
//!
//! | column           | dtype  | notes                                               |
//! |------------------|--------|-----------------------------------------------------|
//! | `day_open_ts_ms` | Int64  | UTC midnight of the DVOL day (candle open, ms)      |
//! | `day_close_ts_ms`| Int64  | `day_open_ts_ms + 86_400_000 - 1` (the as-of key)  |
//! | `dvol_open`      | Float64| DVOL index open (annualized vol points, e.g. 52.4)  |
//! | `dvol_high`      | Float64| DVOL index high                                     |
//! | `dvol_low`       | Float64| DVOL index low                                      |
//! | `dvol_close`     | Float64| DVOL index daily close — the ONLY signal field       |
//!
//! DVOL is dimensionless (annualized-vol points); it never enters a money/P&L
//! computation (ADR-0003 money-math rule untouched). The signal consumes
//! `dvol_close` ONLY; OHL are banked for provenance.
//!
//! # Revision manifest
//!
//! `--emit-revision-manifest` writes `data/deribit-dvol/REVISION.toml` with
//! aggregate SHA-256 over all Parquet files present, pinning the corpus exactly
//! like `data/binance-basis/`.
//!
//! # Idempotency
//!
//! Re-running over the same span produces byte-identical Parquets (same rows,
//! same column order, no wall-clock in the body). Only `fetched_at` in the
//! manifest metadata varies (advisory label, not hashed).

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
use tracing::{info, warn};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "fetch_deribit_dvol",
    about = "Fetch Deribit DVOL implied-vol index candles and write Parquet files"
)]
struct Cli {
    /// Comma-separated Deribit currencies: BTC, ETH
    #[arg(short = 'c', long, value_delimiter = ',', default_value = "BTC,ETH")]
    currencies: Vec<String>,

    /// Inclusive start date (YYYY-MM-DD)
    #[arg(long, default_value = "2023-01-01")]
    start: String,

    /// Inclusive end date (YYYY-MM-DD, inclusive)
    #[arg(long, default_value = "2024-12-31")]
    end: String,

    /// Output root directory
    #[arg(long, default_value = "data/deribit-dvol")]
    out: PathBuf,

    /// Overwrite existing Parquet files (default: skip if file exists)
    #[arg(long, default_value_t = false)]
    force: bool,

    /// After all downloads complete, write (or overwrite) a `REVISION.toml`
    /// manifest in `--out` with SHA-256 for every Parquet file present.
    #[arg(long, default_value_t = false)]
    emit_revision_manifest: bool,

    /// Milliseconds to sleep between pagination requests (rate-limit guard).
    #[arg(long, default_value_t = 300)]
    sleep_ms: u64,
}

// ── Deribit DVOL candle types ────────────────────────────────────────────────

const DERIBIT_DVOL_URL: &str = "https://www.deribit.com/api/v2/public/get_volatility_index_data";

/// One Deribit DVOL candle (12h or daily, folded to a daily row on write).
#[derive(Debug, Clone)]
pub struct DvolCandle {
    /// Candle open timestamp (Unix ms).
    pub open_ts_ms: i64,
    /// DVOL index open (annualized vol points).
    pub open: f64,
    /// DVOL index high.
    pub high: f64,
    /// DVOL index low.
    pub low: f64,
    /// DVOL index close.
    pub close: f64,
}

/// Deribit API response envelope.
#[derive(Deserialize, Debug)]
struct DeribitEnvelope {
    result: DeribitResult,
}

/// The `result` field of the Deribit response.
#[derive(Deserialize, Debug)]
struct DeribitResult {
    data: Vec<[serde_json::Value; 5]>,
    #[serde(default)]
    continuation: Option<i64>,
}

// ── URL builder ───────────────────────────────────────────────────────────────

/// Build a Deribit get_volatility_index_data query URL.
///
/// Pure function — no I/O. Used by tests and the paginator.
pub fn build_dvol_url(currency: &str, resolution: u64, start_ms: i64, end_ms: i64) -> String {
    format!(
        "{DERIBIT_DVOL_URL}?currency={currency}&resolution={resolution}&start_timestamp={start_ms}&end_timestamp={end_ms}"
    )
}

// ── DvolFetcher trait + impls ─────────────────────────────────────────────────

/// Trait so tests can inject a mock fetcher.
///
/// Every external I/O is behind this trait (CLAUDE.md coding rule).
#[async_trait::async_trait]
pub trait DvolFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<(Vec<DvolCandle>, Option<i64>)>;
}

/// Real HTTP fetcher backed by `reqwest`.
pub struct HttpDvolFetcher {
    client: Client,
}

impl HttpDvolFetcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl DvolFetcher for HttpDvolFetcher {
    async fn fetch(&self, url: &str) -> Result<(Vec<DvolCandle>, Option<i64>)> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(anyhow!("Deribit rate limited (429) for {url}"));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Deribit returned HTTP {status}: {body}"));
        }

        let envelope: DeribitEnvelope = resp
            .json()
            .await
            .with_context(|| format!("JSON decode for {url}"))?;

        let candles = envelope
            .result
            .data
            .iter()
            .map(parse_dvol_candle_row)
            .collect::<Result<Vec<_>>>()?;

        Ok((candles, envelope.result.continuation))
    }
}

/// Parse one `[ts_ms, open, high, low, close]` row from the Deribit response.
fn parse_dvol_candle_row(row: &[serde_json::Value; 5]) -> Result<DvolCandle> {
    let open_ts_ms = row[0]
        .as_i64()
        .ok_or_else(|| anyhow!("DVOL row[0] (ts) not i64: {:?}", row[0]))?;
    let open = row[1]
        .as_f64()
        .ok_or_else(|| anyhow!("DVOL row[1] (open) not f64: {:?}", row[1]))?;
    let high = row[2]
        .as_f64()
        .ok_or_else(|| anyhow!("DVOL row[2] (high) not f64: {:?}", row[2]))?;
    let low = row[3]
        .as_f64()
        .ok_or_else(|| anyhow!("DVOL row[3] (low) not f64: {:?}", row[3]))?;
    let close = row[4]
        .as_f64()
        .ok_or_else(|| anyhow!("DVOL row[4] (close) not f64: {:?}", row[4]))?;
    Ok(DvolCandle {
        open_ts_ms,
        open,
        high,
        low,
        close,
    })
}

// ── Paginator ─────────────────────────────────────────────────────────────────

/// Deribit DVOL resolution in milliseconds for `resolution=43200` (12 hours).
const RESOLUTION_MS: i64 = 43_200_000; // 12 h in ms

/// Paginate over Deribit DVOL candles for a currency + time window.
///
/// Returns all candles whose `open_ts_ms` falls within `[start_ms, end_ms]`.
/// Advances cursor via the `continuation` field (or window by timestamp)
/// until the requested span is covered.
///
/// # Arguments
///
/// - `fetcher`:    I/O source (real or mock).
/// - `currency`:   `"BTC"` or `"ETH"`.
/// - `resolution`: Deribit resolution parameter (default `43200` for 12h).
/// - `start_ms`:   Inclusive start timestamp (Unix ms).
/// - `end_ms`:     Inclusive end timestamp (Unix ms).
/// - `sleep_ms`:   Milliseconds to sleep between requests (rate-limit guard).
pub async fn paginate_dvol(
    fetcher: &dyn DvolFetcher,
    currency: &str,
    resolution: u64,
    start_ms: i64,
    end_ms: i64,
    sleep_ms: u64,
) -> Result<Vec<DvolCandle>> {
    let mut all: Vec<DvolCandle> = Vec::new();
    let mut cursor = start_ms;
    let mut request_count: u32 = 0;

    loop {
        let url = build_dvol_url(currency, resolution, cursor, end_ms);
        let (batch, continuation) = fetcher.fetch(&url).await?;
        request_count += 1;

        if batch.is_empty() {
            break;
        }

        // Keep only candles in [start_ms, end_ms] (defensive against boundary bars).
        let in_window: Vec<DvolCandle> = batch
            .into_iter()
            .filter(|c| c.open_ts_ms >= start_ms && c.open_ts_ms <= end_ms)
            .collect();
        all.extend(in_window);

        // Advance cursor: prefer the Deribit `continuation` field (the next page's
        // start timestamp). Fall back to the MAXIMUM `open_ts_ms` seen so far.
        //
        // Review 3-15 HIGH: this fallback used to read `all.last()` — the last
        // candle in INSERTION order. Nothing in the fetcher asserted that Deribit
        // returns pages ascending, and if a page ever arrived newest-first the
        // cursor would jump to the OLDEST candle's timestamp + one resolution,
        // fail the `next_cursor <= cursor` guard, and break after page 1 —
        // silently truncating the year to a single page while the run reported
        // success. `max()` is order-independent by construction.
        let next_cursor = match continuation {
            Some(cont) if cont > cursor => cont,
            _ => {
                // Derive from the newest candle's open_ts, whatever order the
                // pages arrived in.
                match all.iter().map(|c| c.open_ts_ms).max() {
                    Some(max_open) => max_open + RESOLUTION_MS,
                    None => break, // no candles at all
                }
            }
        };

        if next_cursor > end_ms || next_cursor <= cursor {
            break;
        }
        cursor = next_cursor;

        if sleep_ms > 0 {
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }
    }

    info!(
        currency,
        requests = request_count,
        candles = all.len(),
        "paginated Deribit DVOL candles"
    );
    Ok(all)
}

// ── Daily aggregation ─────────────────────────────────────────────────────────

/// A daily DVOL row (open-high-low-close from the day's candles).
#[derive(Debug, Clone)]
pub struct DailyDvolRow {
    /// UTC midnight of this DVOL day (ms since epoch).
    pub day_open_ts_ms: i64,
    /// The as-of key: `day_open_ts_ms + 86_400_000 - 1`.
    ///
    /// This is the instant the daily close is FULLY observed — used by the
    /// as-of join (`dvol_as_of`) so a bar opening at `t` sees only the
    /// DVOL close with `day_close_ts_ms ≤ t` (strict no-look-ahead).
    pub day_close_ts_ms: i64,
    pub dvol_open: f64,
    pub dvol_high: f64,
    pub dvol_low: f64,
    /// The ONLY field the `v0.dvol_regime` signal consumes.
    pub dvol_close: f64,
}

const ONE_DAY_MS: i64 = 86_400_000;

/// Aggregate 12h DVOL candles into daily rows (one per UTC day).
///
/// For each UTC day, takes the EARLIEST candle's open as `dvol_open`, the max
/// high, the min low, and the LATEST candle's close as `dvol_close`. Candles
/// are grouped by their UTC day (floor to midnight).
///
/// Returns rows sorted by `day_open_ts_ms ASC`.
///
/// # Candle order is asserted, not assumed (review 3-15 HIGH)
///
/// This function used to take `day_candles.first()` / `.last()` in **insertion**
/// order and nothing anywhere checked that the API returned candles ascending.
/// `dvol_close = last.close` is the single field the `v0.dvol_regime` signal
/// consumes, so if Deribit ever returned a page newest-first, `dvol_close` would
/// become the 00:00 candle's close instead of the 12:00 one — a systematic 12h
/// look-back **baked into the parquet and then SHA-pinned forever**, invisible to
/// every downstream check because the value is perfectly plausible.
///
/// Candles are now sorted by `open_ts_ms` within each day and de-duplicated on
/// that key, so the output is a pure function of the SET of candles and is
/// independent of the order (and of page-seam repeats) the paginator saw them in.
/// The existing `test_aggregate_two_candles_one_day` feeds already-ascending
/// candles and therefore could never have caught this; see
/// `test_aggregate_is_order_independent` and
/// `test_aggregate_deduplicates_page_seam_repeats`.
#[must_use]
pub fn aggregate_to_daily(candles: &[DvolCandle]) -> Vec<DailyDvolRow> {
    use std::collections::BTreeMap;

    // Group candles by UTC-midnight day key, keyed WITHIN the day by
    // `open_ts_ms`. A BTreeMap gives ascending order by construction and makes a
    // page-seam duplicate (the same candle returned at the end of one page and
    // the start of the next) collapse to one entry instead of skewing the day.
    let mut days: BTreeMap<i64, BTreeMap<i64, &DvolCandle>> = BTreeMap::new();
    for c in candles {
        let day_key = (c.open_ts_ms / ONE_DAY_MS) * ONE_DAY_MS;
        days.entry(day_key).or_default().insert(c.open_ts_ms, c);
    }

    let mut rows = Vec::with_capacity(days.len());
    for (day_start, day_candles) in &days {
        // `values()` is ascending by `open_ts_ms` — the earliest candle of the
        // day opens it, the latest closes it, whatever order they arrived in.
        let Some(first) = day_candles.values().next() else {
            continue; // unreachable: a day key only exists once a candle inserted
        };
        let Some(last) = day_candles.values().next_back() else {
            continue;
        };
        let high = day_candles
            .values()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = day_candles
            .values()
            .map(|c| c.low)
            .fold(f64::INFINITY, f64::min);

        rows.push(DailyDvolRow {
            day_open_ts_ms: *day_start,
            day_close_ts_ms: day_start + ONE_DAY_MS - 1,
            dvol_open: first.open,
            dvol_high: high,
            dvol_low: low,
            dvol_close: last.close,
        });
    }
    rows
}

// ── Parquet writer ────────────────────────────────────────────────────────────

/// Write a `Vec<DailyDvolRow>` to a Parquet file at `path`.
///
/// Creates parent directories as needed. Schema:
/// `day_open_ts_ms` Int64, `day_close_ts_ms` Int64,
/// `dvol_open/high/low/close` Float64.
pub fn write_parquet(rows: &[DailyDvolRow], path: &Path) -> Result<()> {
    if rows.is_empty() {
        warn!(?path, "no DVOL rows to write — skipping parquet creation");
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create directories for {}", path.display()))?;
    }

    let day_opens: Vec<i64> = rows.iter().map(|r| r.day_open_ts_ms).collect();
    let day_closes: Vec<i64> = rows.iter().map(|r| r.day_close_ts_ms).collect();
    let dvol_opens: Vec<f64> = rows.iter().map(|r| r.dvol_open).collect();
    let dvol_highs: Vec<f64> = rows.iter().map(|r| r.dvol_high).collect();
    let dvol_lows: Vec<f64> = rows.iter().map(|r| r.dvol_low).collect();
    let dvol_closes: Vec<f64> = rows.iter().map(|r| r.dvol_close).collect();

    let mut df = DataFrame::new(vec![
        Column::new("day_open_ts_ms".into(), day_opens.as_slice()),
        Column::new("day_close_ts_ms".into(), day_closes.as_slice()),
        Column::new("dvol_open".into(), dvol_opens.as_slice()),
        Column::new("dvol_high".into(), dvol_highs.as_slice()),
        Column::new("dvol_low".into(), dvol_lows.as_slice()),
        Column::new("dvol_close".into(), dvol_closes.as_slice()),
    ])
    .with_context(|| format!("build DataFrame for {}", path.display()))?;

    let file = std::fs::File::create(path)
        .with_context(|| format!("create parquet file: {}", path.display()))?;
    let writer = BufWriter::new(file);
    ParquetWriter::new(writer)
        .finish(&mut df)
        .with_context(|| format!("write parquet: {}", path.display()))?;

    info!(path = %path.display(), rows = rows.len(), "wrote DVOL parquet");
    Ok(())
}

// ── Date utilities ────────────────────────────────────────────────────────────

use time::{Date, Month, PrimitiveDateTime, Time};

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

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    llm::tracing_init::install_global(&[], false)?;

    let cli = Cli::parse();

    if cli.currencies.is_empty() {
        return Err(anyhow!("--currencies must not be empty"));
    }

    let start_date =
        parse_date(&cli.start).with_context(|| format!("parse --start date: {}", cli.start))?;
    let end_date =
        parse_date(&cli.end).with_context(|| format!("parse --end date: {}", cli.end))?;
    if end_date < start_date {
        return Err(anyhow!("--end must be >= --start"));
    }

    let start_ms = date_to_millis(start_date);
    // End is inclusive: use end-of-day (start of next day - 1 ms)
    let end_date_next = end_date
        .next_day()
        .ok_or_else(|| anyhow!("end_date next_day overflow"))?;
    let end_ms = date_to_millis(end_date_next) - 1;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")?;
    let fetcher = HttpDvolFetcher::new(client);

    for currency_raw in &cli.currencies {
        let currency = currency_raw.to_uppercase();
        info!(currency = %currency, "starting Deribit DVOL download");

        // Determine the output path per year:
        // data/deribit-dvol/<CURRENCY>/<YEAR>.parquet
        // Since the resolution is daily, we write one parquet per year.
        let mut year = start_date.year();
        let end_year = end_date.year();

        while year <= end_year {
            // Year window: Jan 1 to Dec 31 of `year`, clipped to [start_ms, end_ms].
            let year_start_date =
                Date::from_calendar_date(year, Month::January, 1).expect("valid year start");
            let year_end_date =
                Date::from_calendar_date(year, Month::December, 31).expect("valid year end");
            let year_start_ms = date_to_millis(year_start_date).max(start_ms);
            let year_end_ms = (date_to_millis(year_end_date) + ONE_DAY_MS - 1).min(end_ms);

            if year_start_ms > year_end_ms {
                year += 1;
                continue;
            }

            let parquet_path = cli.out.join(&currency).join(format!("{year}.parquet"));

            if !cli.force && parquet_path.exists() {
                info!(
                    path = %parquet_path.display(),
                    "file exists — skipping (use --force to overwrite)"
                );
                year += 1;
                continue;
            }

            info!(
                currency = %currency, year, start_ms = year_start_ms, end_ms = year_end_ms,
                "fetching Deribit DVOL"
            );

            let candles = paginate_dvol(
                &fetcher,
                &currency,
                43200, // 12h resolution → daily close
                year_start_ms,
                year_end_ms,
                cli.sleep_ms,
            )
            .await
            .with_context(|| format!("fetch DVOL for {currency} {year}"))?;

            if candles.is_empty() {
                warn!(
                    currency = %currency, year,
                    "API returned 0 candles for this year — skipping parquet write"
                );
            } else {
                let daily_rows = aggregate_to_daily(&candles);
                write_parquet(&daily_rows, &parquet_path)
                    .with_context(|| format!("write parquet for {currency}/{year}"))?;
                println!(
                    "[OK] {currency}/{year}.parquet  ({} daily rows from {} 12h candles)",
                    daily_rows.len(),
                    candles.len()
                );
            }

            year += 1;
        }

        info!(currency = %currency, "finished Deribit DVOL download");
    }

    if cli.emit_revision_manifest {
        let agg_sha = data::revision::write_revision_manifest_with_tool(
            &cli.out,
            data::revision::RevisionMetadataInput {
                fetch_tool: "fetch_deribit_dvol",
                binance_base: "https://www.deribit.com/api/v2",
                interval: Some("43200"),
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
    clippy::float_arithmetic,
    clippy::pedantic
)]
mod tests {
    use super::*;

    // ── URL builder ──────────────────────────────────────────────────────────

    #[test]
    fn test_url_builder_basic() {
        let url = build_dvol_url("BTC", 43200, 1_672_531_200_000, 1_675_209_599_999);
        assert!(url.contains("currency=BTC"), "currency in url: {url}");
        assert!(url.contains("resolution=43200"), "resolution in url: {url}");
        assert!(
            url.contains("start_timestamp=1672531200000"),
            "start_timestamp in url: {url}"
        );
        assert!(
            url.contains("end_timestamp=1675209599999"),
            "end_timestamp in url: {url}"
        );
        assert!(
            url.starts_with("https://www.deribit.com/api/v2/public/get_volatility_index_data"),
            "correct base url: {url}"
        );
    }

    // ── MockDvolFetcher + paginator tests ───────────────────────────────────

    struct MockDvolFetcher {
        batches: std::sync::Mutex<Vec<(Vec<DvolCandle>, Option<i64>)>>,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl MockDvolFetcher {
        fn new(batches: Vec<(Vec<DvolCandle>, Option<i64>)>) -> Self {
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
    impl DvolFetcher for MockDvolFetcher {
        async fn fetch(&self, url: &str) -> Result<(Vec<DvolCandle>, Option<i64>)> {
            self.calls.lock().unwrap().push(url.to_owned());
            let mut batches = self.batches.lock().unwrap();
            if batches.is_empty() {
                Ok((vec![], None))
            } else {
                Ok(batches.remove(0))
            }
        }
    }

    const ONE_DAY_MS_T: i64 = 86_400_000;
    const HALF_DAY_MS: i64 = 43_200_000;

    fn make_candle(open_ts_ms: i64, close: f64) -> DvolCandle {
        DvolCandle {
            open_ts_ms,
            open: close * 0.99,
            high: close * 1.01,
            low: close * 0.98,
            close,
        }
    }

    /// Paginator stops on empty batch.
    #[tokio::test]
    async fn test_paginator_stops_on_empty() {
        let fetcher = MockDvolFetcher::new(vec![(vec![], None)]);
        let result = paginate_dvol(&fetcher, "BTC", 43200, 0, 1_000_000_000, 0)
            .await
            .expect("should not error on empty");
        assert!(result.is_empty(), "empty response → no candles");
        assert_eq!(fetcher.recorded_calls().len(), 1, "exactly one call");
    }

    /// Paginator advances cursor using continuation.
    #[tokio::test]
    async fn test_paginator_advances_cursor_via_continuation() {
        let start_ms: i64 = 1_672_531_200_000; // 2023-01-01
        let page1_candle = make_candle(start_ms, 50.0);
        let next_start_ms = start_ms + HALF_DAY_MS;
        let page2_candle = make_candle(next_start_ms, 55.0);
        let end_ms = start_ms + 2 * ONE_DAY_MS_T;

        let fetcher = MockDvolFetcher::new(vec![
            // page 1: returns continuation = next_start_ms
            (vec![page1_candle], Some(next_start_ms)),
            // page 2: no continuation
            (vec![page2_candle], None),
            // page 3: empty (stop)
            (vec![], None),
        ]);

        let result = paginate_dvol(&fetcher, "BTC", 43200, start_ms, end_ms, 0)
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 2, "2 candles across 2 pages");
        let calls = fetcher.recorded_calls();
        assert!(calls.len() >= 2, "at least 2 requests");
        // Second call must start at next_start_ms.
        assert!(
            calls[1].contains(&format!("start_timestamp={next_start_ms}")),
            "second request must use cursor={next_start_ms}, got: {}",
            calls[1]
        );
    }

    /// Paginator filters out-of-window candles.
    #[tokio::test]
    async fn test_paginator_filters_out_of_window() {
        let window_start: i64 = 1_672_531_200_000;
        let window_end: i64 = window_start + ONE_DAY_MS_T;

        // Candle before window, one at start, one at end, one after.
        let before = make_candle(window_start - ONE_DAY_MS_T, 40.0);
        let at_start = make_candle(window_start, 50.0);
        let at_end = make_candle(window_end, 55.0);
        let after = make_candle(window_end + ONE_DAY_MS_T, 60.0);

        let fetcher = MockDvolFetcher::new(vec![(vec![before, at_start, at_end, after], None)]);

        let result = paginate_dvol(&fetcher, "BTC", 43200, window_start, window_end, 0)
            .await
            .expect("should succeed");

        assert_eq!(result.len(), 2, "2 candles in [start, end]");
        assert_eq!(result[0].open_ts_ms, window_start);
        assert_eq!(result[1].open_ts_ms, window_end);
    }

    // ── Daily aggregation ────────────────────────────────────────────────────

    /// **Review 3-15 HIGH — the RED-before test for candle ordering.**
    ///
    /// `dvol_close = last.close` used to take the LAST candle in *insertion*
    /// order. Feeding the same day's two candles newest-first therefore produced
    /// `dvol_close = 52.0` (the 00:00 candle) instead of `57.0` (the 12:00 one) —
    /// a systematic 12h look-back written into the parquet and SHA-pinned forever.
    /// Note `test_aggregate_two_candles_one_day` below feeds them already
    /// ascending, so it cannot catch this.
    #[test]
    fn test_aggregate_is_order_independent() {
        let day_start: i64 = 1_672_531_200_000; // 2023-01-01T00:00Z
        let c_00 = DvolCandle {
            open_ts_ms: day_start,
            open: 50.0,
            high: 55.0,
            low: 48.0,
            close: 52.0,
        };
        let c_12 = DvolCandle {
            open_ts_ms: day_start + HALF_DAY_MS,
            open: 52.0,
            high: 58.0,
            low: 50.0,
            close: 57.0,
        };

        let ascending = aggregate_to_daily(&[c_00.clone(), c_12.clone()]);
        // A page returned NEWEST-FIRST.
        let descending = aggregate_to_daily(&[c_12, c_00]);

        assert_eq!(ascending.len(), 1);
        assert_eq!(descending.len(), 1);
        assert!(
            (descending[0].dvol_close - 57.0).abs() < 1e-9,
            "dvol_close must be the 12:00 candle's close (57.0) regardless of page \
             order — got {}. Taking `last()` in insertion order yields 52.0, the \
             00:00 close: a silent 12h look-back baked into the corpus.",
            descending[0].dvol_close
        );
        assert!(
            (descending[0].dvol_open - 50.0).abs() < 1e-9,
            "dvol_open must be the 00:00 candle's open (50.0), got {}",
            descending[0].dvol_open
        );
        assert!(
            (ascending[0].dvol_close - descending[0].dvol_close).abs() < 1e-9
                && (ascending[0].dvol_open - descending[0].dvol_open).abs() < 1e-9
                && (ascending[0].dvol_high - descending[0].dvol_high).abs() < 1e-9
                && (ascending[0].dvol_low - descending[0].dvol_low).abs() < 1e-9,
            "the daily row must be a pure function of the SET of candles"
        );
    }

    /// A candle repeated at a page seam must not change the day (review 3-15 HIGH).
    ///
    /// Deribit's `continuation` is the NEXT page's start timestamp, but a
    /// boundary candle showing up twice is exactly the kind of thing a paginator
    /// hits. De-duplicating on `open_ts_ms` makes the fold idempotent.
    #[test]
    fn test_aggregate_deduplicates_page_seam_repeats() {
        let day_start: i64 = 1_672_531_200_000;
        let c_00 = make_candle(day_start, 50.0);
        let c_12 = make_candle(day_start + HALF_DAY_MS, 57.0);

        let clean = aggregate_to_daily(&[c_00.clone(), c_12.clone()]);
        // The 12:00 candle arrives at the end of page 1 AND the start of page 2,
        // and the pages are stitched newest-first for good measure.
        let seamed = aggregate_to_daily(&[c_12.clone(), c_00, c_12]);

        assert_eq!(
            seamed.len(),
            1,
            "a seam repeat must not create a second day"
        );
        assert!(
            (clean[0].dvol_close - seamed[0].dvol_close).abs() < 1e-9
                && (clean[0].dvol_open - seamed[0].dvol_open).abs() < 1e-9,
            "a duplicated seam candle must leave the daily row byte-identical: \
             clean={clean:?} seamed={seamed:?}"
        );
    }

    /// The paginator's cursor fallback must not assume ascending pages
    /// (review 3-15 HIGH).
    ///
    /// With `continuation: None` the cursor used to advance from
    /// `all.last().open_ts_ms` — the last candle in insertion order. On a
    /// newest-first page that is the OLDEST candle, so `next_cursor <= cursor`
    /// and the loop breaks after page 1: the rest of the year is silently
    /// dropped while the run reports success. The cursor now advances from the
    /// MAXIMUM `open_ts_ms` seen.
    #[tokio::test]
    async fn test_paginator_cursor_survives_a_descending_page() {
        let start_ms: i64 = 1_672_531_200_000; // 2023-01-01
        let end_ms = start_ms + 10 * ONE_DAY_MS_T;

        // Page 1 returns 4 half-day candles NEWEST-FIRST and no continuation.
        let page1: Vec<DvolCandle> = (0..4)
            .rev()
            .map(|i| make_candle(start_ms + i * HALF_DAY_MS, 50.0 + i as f64))
            .collect();
        // Page 2 continues from where page 1 really ended.
        let page2: Vec<DvolCandle> = (4..6)
            .map(|i| make_candle(start_ms + i * HALF_DAY_MS, 50.0 + i as f64))
            .collect();

        let fetcher = MockDvolFetcher::new(vec![(page1, None), (page2, None), (vec![], None)]);
        let result = paginate_dvol(&fetcher, "BTC", 43200, start_ms, end_ms, 0)
            .await
            .expect("pagination must succeed");

        assert_eq!(
            result.len(),
            6,
            "the paginator must keep advancing past a newest-first page; it \
             collected {} candle(s) — truncating after page 1 is the defect",
            result.len()
        );
        let calls = fetcher.recorded_calls();
        assert!(calls.len() >= 2, "at least 2 requests, got {}", calls.len());
        let expected_cursor = start_ms + 3 * HALF_DAY_MS + RESOLUTION_MS;
        assert!(
            calls[1].contains(&format!("start_timestamp={expected_cursor}")),
            "the second request must advance from the NEWEST candle of page 1 \
             ({expected_cursor}); got: {}",
            calls[1]
        );
    }

    /// Two 12h candles on the same day → one daily row.
    ///
    /// **Scope note (review 3-15):** this test feeds candles ALREADY ascending,
    /// so it pins the OHLC fold but says nothing about ordering. The ordering
    /// guarantee is `test_aggregate_is_order_independent`.
    #[test]
    fn test_aggregate_two_candles_one_day() {
        let day_start: i64 = 1_672_531_200_000; // 2023-01-01T00:00Z
        let c1 = DvolCandle {
            open_ts_ms: day_start,
            open: 50.0,
            high: 55.0,
            low: 48.0,
            close: 52.0,
        };
        let c2 = DvolCandle {
            open_ts_ms: day_start + HALF_DAY_MS,
            open: 52.0,
            high: 58.0,
            low: 50.0,
            close: 57.0,
        };

        let rows = aggregate_to_daily(&[c1, c2]);
        assert_eq!(rows.len(), 1, "two 12h candles → one daily row");
        let r = &rows[0];
        assert_eq!(r.day_open_ts_ms, day_start);
        assert_eq!(r.day_close_ts_ms, day_start + ONE_DAY_MS_T - 1);
        // open = first candle's open, close = last candle's close.
        assert!((r.dvol_open - 50.0).abs() < 1e-9);
        assert!((r.dvol_close - 57.0).abs() < 1e-9);
        // high = max(55, 58) = 58.
        assert!((r.dvol_high - 58.0).abs() < 1e-9);
        // low = min(48, 50) = 48.
        assert!((r.dvol_low - 48.0).abs() < 1e-9);
    }

    /// `day_close_ts_ms = day_open_ts_ms + 86_400_000 - 1` (the as-of key).
    #[test]
    fn test_day_close_ts_is_end_of_day() {
        let day_start: i64 = 1_672_531_200_000;
        let c = make_candle(day_start, 50.0);
        let rows = aggregate_to_daily(&[c]);
        assert_eq!(rows[0].day_close_ts_ms, day_start + ONE_DAY_MS_T - 1);
    }

    /// Empty input → empty output.
    #[test]
    fn test_aggregate_empty() {
        let rows = aggregate_to_daily(&[]);
        assert!(rows.is_empty());
    }

    // ── Parquet schema round-trip ────────────────────────────────────────────

    #[test]
    fn test_parquet_schema_roundtrip() {
        let rows = vec![DailyDvolRow {
            day_open_ts_ms: 1_672_531_200_000,
            day_close_ts_ms: 1_672_531_200_000 + ONE_DAY_MS_T - 1,
            dvol_open: 50.0,
            dvol_high: 60.0,
            dvol_low: 48.0,
            dvol_close: 55.5,
        }];

        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test_dvol.parquet");
        write_parquet(&rows, &path).expect("write_parquet");

        let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
            .expect("scan")
            .collect()
            .expect("collect");

        assert_eq!(df.height(), 1, "1 row");
        assert_eq!(df.width(), 6, "6 columns");

        let schema = df.schema();
        assert_eq!(schema.get("day_open_ts_ms").cloned(), Some(DataType::Int64));
        assert_eq!(
            schema.get("day_close_ts_ms").cloned(),
            Some(DataType::Int64)
        );
        assert_eq!(schema.get("dvol_open").cloned(), Some(DataType::Float64));
        assert_eq!(schema.get("dvol_close").cloned(), Some(DataType::Float64));

        let closes = df
            .column("dvol_close")
            .unwrap()
            .f64()
            .unwrap()
            .get(0)
            .unwrap();
        assert!((closes - 55.5).abs() < 1e-9, "close value preserved");
    }
}
