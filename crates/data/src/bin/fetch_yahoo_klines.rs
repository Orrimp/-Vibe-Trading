//! `fetch_yahoo_klines` — Yahoo Finance OHLCV bars → Parquet cache downloader.
//!
//! Fetches historical OHLCV bars from the Yahoo Finance API and writes them
//! into the per-ticker, per-interval, per-month Parquet files that
//! `YahooBarSource::load_cached` can read.
//!
//! # Output layout
//!
//! ```text
//! <out>/<TICKER>/<INTERVAL>/<YEAR>/<MONTH-padded>.parquet
//! <out>/REVISION.toml
//! ```
//!
//! # Usage
//!
//! ```text
//! fetch_yahoo_klines --tickers BTC-USD,ETH-USD \
//!                   --interval 1d \
//!                   --start 2024-01-01 \
//!                   --end 2024-12-31 \
//!                   --out data/yahoo
//! ```
//!
//! # Feature gate
//!
//! This binary requires `--features yahoo-online`.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use data::yahoo::{Interval, YahooBarSource, YahooError, write_revision_manifest};
use time::{Date, Month, PrimitiveDateTime, Time};
use tracing::{error, info, warn};
// EnvFilter now used via llm::tracing_init::install_global (T-RED-D12).

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "fetch_yahoo_klines",
    version = "0.1.0",
    about = "Fetch Yahoo Finance OHLCV bars into the data/yahoo/ parquet cache (ADR-0040)"
)]
struct Args {
    /// Comma-separated Yahoo tickers (e.g. "BTC-USD,ETH-USD").
    /// v0.1.0 accepts any valid Yahoo Finance ticker symbol.
    #[arg(long, value_delimiter = ',')]
    tickers: Vec<String>,

    /// Bar cadence. One of `1m`, `1h`, `1d`.
    #[arg(long)]
    interval: String,

    /// Inclusive start date, YYYY-MM-DD (UTC midnight).
    #[arg(long)]
    start: String,

    /// Inclusive end date, YYYY-MM-DD (UTC midnight).
    #[arg(long)]
    end: String,

    /// Output cache root directory. Default: `data/yahoo`.
    #[arg(long, default_value = "data/yahoo")]
    out: PathBuf,

    /// Dry-run: print the URL + expected bar counts; do not write parquet.
    #[arg(long)]
    dry_run: bool,

    /// Emit / update REVISION.toml after all tickers are fetched. Default: `true`.
    #[arg(long, default_value_t = true)]
    emit_revision_manifest: bool,
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
    llm::tracing_init::install_global(&[], false)?;

    let args = Args::parse();

    if args.tickers.is_empty() {
        bail!("at least one --tickers value is required");
    }

    let interval = parse_interval(&args.interval)
        .with_context(|| format!("invalid interval: {:?}", args.interval))?;

    let (start_ms, end_ms) = parse_date_range(&args.start, &args.end)
        .with_context(|| format!("invalid date range: {} .. {}", args.start, args.end))?;

    if args.dry_run {
        return run_dry(&args.tickers, interval, start_ms, end_ms);
    }

    // Ensure the output directory exists.
    std::fs::create_dir_all(&args.out).with_context(|| format!("create_dir_all {:?}", args.out))?;

    let src = YahooBarSource::new(args.out.clone());

    let mut any_error = false;
    for ticker in &args.tickers {
        if let Err(e) = fetch_with_backoff(&src, ticker, interval, start_ms, end_ms).await {
            error!(ticker = %ticker, error = %e, "fetch failed");
            any_error = true;
        }
    }

    // Emit or update REVISION.toml (covers all tickers fetched so far).
    if args.emit_revision_manifest {
        write_revision_manifest(&args.out)
            .with_context(|| format!("write_revision_manifest {:?}", args.out))?;
        info!(path = ?args.out.join("REVISION.toml"), "revision manifest written");
    }

    if any_error {
        bail!("one or more tickers failed to fetch; see logs above");
    }

    Ok(())
}

// ── Fetch with exponential backoff (K1 mitigation) ───────────────────────────

/// Fetch a single ticker with exponential back-off on HTTP 429.
///
/// Strategy (ADR-0040 § T-AR7):
/// - Initial delay: 1 s.
/// - Multiplier: ×2 per retry.
/// - Cap: 60 s.
/// - Max retries: 5.
async fn fetch_with_backoff(
    src: &YahooBarSource,
    ticker: &str,
    interval: Interval,
    start_ms: i64,
    end_ms: i64,
) -> Result<(), YahooError> {
    let max_retries: u32 = 5;
    let mut backoff = Duration::from_secs(1);
    let cap = Duration::from_secs(60);

    for attempt in 0..=max_retries {
        match src
            .fetch_and_cache(ticker, interval, start_ms, end_ms)
            .await
        {
            Ok(loaded) => {
                info!(
                    ticker = %ticker,
                    bars = loaded.loaded_count,
                    expected = loaded.expected_count,
                    revision_sha = %&loaded.revision_sha[..8],
                    "fetched OK"
                );
                return Ok(());
            }
            Err(YahooError::RateLimited { retry_after_secs }) if attempt < max_retries => {
                let delay = backoff.max(Duration::from_secs(retry_after_secs));
                warn!(
                    ticker = %ticker,
                    attempt,
                    delay_s = delay.as_secs(),
                    "rate-limited by Yahoo, backing off"
                );
                tokio::time::sleep(delay).await;
                backoff = (backoff * 2).min(cap);
            }
            Err(e) => return Err(e),
        }
    }

    // Unreachable: the loop exits either via Ok or via the non-RateLimited arm.
    Err(YahooError::Http(format!(
        "max retries ({max_retries}) exhausted for {ticker}"
    )))
}

// ── Dry-run ───────────────────────────────────────────────────────────────────

/// Print what would be fetched without performing any I/O.
fn run_dry(tickers: &[String], interval: Interval, start_ms: i64, end_ms: i64) -> Result<()> {
    use data::yahoo::expected_bars_for_range;

    println!("DRY RUN — no files will be written");
    println!("  interval  : {}", interval.as_yahoo_str());
    println!(
        "  range     : {} .. {}",
        format_date(start_ms),
        format_date(end_ms)
    );

    for ticker in tickers {
        let expected = expected_bars_for_range(interval, start_ms, end_ms);
        let base_url = "https://query1.finance.yahoo.com/v8/finance/chart";
        let url = format!(
            "{base_url}/{ticker}?period1={}&period2={}&interval={}",
            start_ms / 1_000,
            end_ms / 1_000,
            interval.as_yahoo_str()
        );
        println!("  ticker    : {ticker}");
        println!("    URL     : {url}");
        println!("    expected: {expected} bars");
    }

    Ok(())
}

// ── Parsers ───────────────────────────────────────────────────────────────────

/// Parse the `--interval` CLI argument.
fn parse_interval(s: &str) -> Result<Interval> {
    match s {
        "1m" => Ok(Interval::Minutes1),
        "1h" => Ok(Interval::Hours1),
        "1d" => Ok(Interval::Days1),
        other => bail!("unknown interval '{other}'; use one of: 1m, 1h, 1d"),
    }
}

/// Parse `--start YYYY-MM-DD` and `--end YYYY-MM-DD` into Unix-ms boundaries.
///
/// `start` is at 00:00:00 UTC on the given date (inclusive).
/// `end` is at 00:00:00 UTC on the day **after** the given date (exclusive),
/// so the entire `end` day is included in the range.
fn parse_date_range(start_str: &str, end_str: &str) -> Result<(i64, i64)> {
    let start_ms = parse_date_to_midnight_ms(start_str)
        .with_context(|| format!("invalid start date: {start_str:?}"))?;
    // end date is inclusive: advance by one day so end_ms points to midnight of
    // the day *after* end_str.
    let end_day_ms = parse_date_to_midnight_ms(end_str)
        .with_context(|| format!("invalid end date: {end_str:?}"))?;
    let end_ms = end_day_ms + 86_400_000; // + 1 day in ms
    if end_ms <= start_ms {
        bail!("end date must be after start date");
    }
    Ok((start_ms, end_ms))
}

/// Parse `YYYY-MM-DD` to Unix milliseconds at midnight UTC.
fn parse_date_to_midnight_ms(s: &str) -> Result<i64> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        bail!("expected YYYY-MM-DD, got: {s:?}");
    }
    let year: i32 = parts[0].parse().context("year")?;
    let month_u8: u8 = parts[1].parse().context("month")?;
    let day: u8 = parts[2].parse().context("day")?;
    let month = Month::try_from(month_u8).context("month out of range 1-12")?;
    let date = Date::from_calendar_date(year, month, day).context("invalid calendar date")?;
    let pdt = PrimitiveDateTime::new(date, Time::MIDNIGHT);
    Ok(pdt.assume_utc().unix_timestamp() * 1_000)
}

/// Format Unix-ms as `YYYY-MM-DD` for display.
fn format_date(ms: i64) -> String {
    match time::OffsetDateTime::from_unix_timestamp(ms / 1_000) {
        Ok(dt) => format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day()),
        Err(_) => format!("{ms}ms"),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── T-C2.1 / T-C2.5 — arg parsing tests (no network) ────────────────────

    #[test]
    fn parse_interval_all_variants() {
        assert_eq!(parse_interval("1m").unwrap(), Interval::Minutes1);
        assert_eq!(parse_interval("1h").unwrap(), Interval::Hours1);
        assert_eq!(parse_interval("1d").unwrap(), Interval::Days1);
    }

    #[test]
    fn parse_interval_unknown_errors() {
        assert!(parse_interval("5m").is_err());
        assert!(parse_interval("").is_err());
        assert!(parse_interval("daily").is_err());
    }

    #[test]
    fn parse_date_range_jan_2024() {
        // 2024-01-01 00:00 UTC → 1704067200000 ms.
        // 2024-01-31 end_ms → midnight of 2024-02-01.
        let (start_ms, end_ms) = parse_date_range("2024-01-01", "2024-01-31").unwrap();
        assert_eq!(start_ms, 1_704_067_200_000, "start must be 2024-01-01 UTC");
        // end_ms = midnight 2024-02-01 = 1706745600000
        assert_eq!(end_ms, 1_706_745_600_000, "end must be 2024-02-01 UTC");
    }

    #[test]
    fn parse_date_range_end_before_start_errors() {
        assert!(parse_date_range("2024-01-31", "2024-01-01").is_err());
    }

    #[test]
    fn parse_date_range_same_day_errors() {
        // start = end midnight; end_ms = start_ms + 1d > start_ms → OK for one day.
        let result = parse_date_range("2024-01-01", "2024-01-01");
        assert!(result.is_ok(), "single-day range should be accepted");
        let (start_ms, end_ms) = result.unwrap();
        assert_eq!(end_ms - start_ms, 86_400_000, "single day = 86400000 ms");
    }

    #[test]
    fn parse_date_to_midnight_ms_known() {
        // 2024-01-01 00:00:00 UTC = 1704067200 s = 1704067200000 ms.
        let ms = parse_date_to_midnight_ms("2024-01-01").unwrap();
        assert_eq!(ms, 1_704_067_200_000);
    }

    #[test]
    fn parse_date_to_midnight_ms_invalid() {
        assert!(parse_date_to_midnight_ms("2024-13-01").is_err()); // month 13
        assert!(parse_date_to_midnight_ms("2024-00-01").is_err()); // month 0
        assert!(parse_date_to_midnight_ms("notadate").is_err());
        assert!(parse_date_to_midnight_ms("2024-01").is_err()); // missing day
    }

    #[test]
    fn format_date_known_ts() {
        assert_eq!(format_date(1_704_067_200_000), "2024-01-01");
    }

    // ── T-C2.4 — dry-run path (no network, no FS writes) ────────────────────

    #[test]
    fn dry_run_executes_without_panic() {
        let tickers = vec!["BTC-USD".to_string(), "ETH-USD".to_string()];
        let (start_ms, end_ms) = parse_date_range("2024-01-01", "2024-01-31").unwrap();
        // Should not panic — just prints to stdout.
        run_dry(&tickers, Interval::Days1, start_ms, end_ms).unwrap();
    }
}
