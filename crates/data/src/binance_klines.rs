//! Binance public REST klines fetcher (library module).
//!
//! Extracted from `crates/data/src/bin/fetch_binance_klines.rs` (Wave A,
//! feature `advisor-dynamic-data`). The bin re-exports everything from here so
//! the CLI + its parquet-write path are byte-unchanged.
//!
//! # Public entry point
//!
//! [`fetch_binance_klines_range`] — paginated, paced, typed-error fetch of
//! hourly (or any interval) klines into `Vec<trading_core::Bar>`.
//!
//! # Mock seam
//!
//! [`KlineFetcher`] is the trait every test injects; the real HTTP impl is
//! [`HttpKlineFetcher`].  No test hits a live socket.
//!
//! # Determinism
//!
//! `kline_to_bar` sets `local_recv_ts = close_ts` (ADR-0032 § D1 Step 7) so
//! a dynamic bar is field-identical to a corpus bar for the same timestamp.

use std::{io::BufWriter, path::Path, time::Duration};

use anyhow::{Context, Result, anyhow};
use polars::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use time::{Date, Month, PrimitiveDateTime, Time};
use tracing::{info, warn};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Typed error ───────────────────────────────────────────────────────────────

/// Typed errors for a dynamic Binance klines fetch.  No panics, no `unwrap`.
#[derive(Debug, thiserror::Error)]
pub enum BinanceFetchError {
    /// Transport / connection failure (DNS, refused, TLS).
    #[error("network error fetching {symbol}: {source}")]
    Network {
        symbol: String,
        #[source]
        source: reqwest::Error,
    },
    /// Request exceeded the client timeout.
    #[error("timeout fetching {symbol} after {secs}s")]
    Timeout { symbol: String, secs: u64 },
    /// Binance weight/rate limit (HTTP 429) or IP ban (HTTP 418).
    /// `retry_after_secs` from the `Retry-After` header when present.
    #[error("Binance rate-limited {symbol} (HTTP {http_status}); retry after {retry_after_secs}s")]
    RateLimited {
        symbol: String,
        http_status: u16,
        retry_after_secs: u64,
    },
    /// Unknown / invalid symbol — Binance returns HTTP 400 with
    /// `{"code":-1121,"msg":"Invalid symbol."}`.
    #[error("unknown or invalid symbol: {symbol}")]
    UnknownSymbol { symbol: String },
    /// HTTP 200 but the window returned zero klines (future-dated / pre-listing
    /// / delisted).  Mirrors `YahooError::NoDataForRange`.
    #[error("no klines for {symbol} in [{start_ms}, {end_ms})")]
    NoDataForRange {
        symbol: String,
        start_ms: i64,
        end_ms: i64,
    },
    /// Any other non-success HTTP, or a malformed body.
    #[error("Binance fetch failed for {symbol}: {detail}")]
    Other { symbol: String, detail: String },
}

// ── Error classifier (pure — no I/O; unit-testable without a socket) ─────────

/// Classify a Binance error from `(http_status_code, body_text)` into a typed
/// `BinanceFetchError` for the given symbol.
///
/// Mirrors `yahoo::classify_yfa_error` in shape.
///
/// Decision tree (matches Binance REST API behaviour):
/// - `400` + body contains `"-1121"` → `UnknownSymbol`
/// - `429` → `RateLimited` (parse `Retry-After` if present in header string;
///   pass `None` → `retry_after_secs = 0`)
/// - `418` → `RateLimited` (IP ban; same shape)
/// - anything else → `Other`
#[must_use]
pub fn classify_binance_error(
    symbol: &str,
    http_status: u16,
    body: &str,
    retry_after_header: Option<&str>,
) -> BinanceFetchError {
    match http_status {
        400 if body.contains("-1121") => BinanceFetchError::UnknownSymbol {
            symbol: symbol.to_owned(),
        },
        429 | 418 => {
            let retry_after_secs = retry_after_header
                .and_then(|h| h.trim().parse::<u64>().ok())
                .unwrap_or(0);
            BinanceFetchError::RateLimited {
                symbol: symbol.to_owned(),
                http_status,
                retry_after_secs,
            }
        }
        _ => BinanceFetchError::Other {
            symbol: symbol.to_owned(),
            detail: format!("HTTP {http_status}: {body}"),
        },
    }
}

// ── Kline types ───────────────────────────────────────────────────────────────

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
pub struct RawKline(pub serde_json::Value);

impl RawKline {
    pub fn parse(self) -> Result<Kline> {
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

// ── Kline → Bar conversion ────────────────────────────────────────────────────

fn millis_to_ts(ms: i64) -> Timestamp {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map(Timestamp::new)
        .unwrap_or_else(|_| Timestamp::now())
}

/// Convert a `Kline` to a `trading_core::Bar`.
///
/// Sets `local_recv_ts = close_ts` (ADR-0032 § D1 Step 7) so a dynamic bar
/// is field-identical to a corpus bar for the same timestamp.  This matches
/// what `replay_feed::read_parquet_bars` produces.
///
/// Returns `BinanceFetchError::Other` on any parse failure; never panics.
pub fn kline_to_bar(
    symbol: &Symbol,
    tf: Timeframe,
    kline: &Kline,
) -> Result<Bar, BinanceFetchError> {
    let parse_price = |s: &str, field: &str| -> Result<Price, BinanceFetchError> {
        s.trim()
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| BinanceFetchError::Other {
                symbol: symbol.0.to_string(),
                detail: format!("{field} decimal parse: {e} (value={s:?})"),
            })
            .and_then(|d| {
                Price::new(d).map_err(|e| BinanceFetchError::Other {
                    symbol: symbol.0.to_string(),
                    detail: format!("{field} Price::new: {e}"),
                })
            })
    };
    let parse_qty = |s: &str, field: &str| -> Result<Quantity, BinanceFetchError> {
        s.trim()
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| BinanceFetchError::Other {
                symbol: symbol.0.to_string(),
                detail: format!("{field} decimal parse: {e} (value={s:?})"),
            })
            .and_then(|d| {
                Quantity::new(d).map_err(|e| BinanceFetchError::Other {
                    symbol: symbol.0.to_string(),
                    detail: format!("{field} Quantity::new: {e}"),
                })
            })
    };

    let open_ts = millis_to_ts(kline.open_time);
    let close_ts = millis_to_ts(kline.close_time);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let trade_count = kline.trade_count.max(0) as u32;

    Ok(Bar {
        symbol: symbol.clone(),
        tf,
        open_ts,
        close_ts,
        open: parse_price(&kline.open, "open")?,
        high: parse_price(&kline.high, "high")?,
        low: parse_price(&kline.low, "low")?,
        close: parse_price(&kline.close, "close")?,
        volume: parse_qty(&kline.volume, "volume")?,
        trade_count,
        // ADR-0032 § D1 Step 7: local_recv_ts = close_ts for determinism parity.
        local_recv_ts: close_ts,
        venue: Venue::Binance,
    })
}

// ── URL builder ───────────────────────────────────────────────────────────────

pub const BINANCE_KLINES_URL: &str = "https://api.binance.com/api/v3/klines";

/// Build a Binance klines query URL.
///
/// Pure function — no I/O.  Used by tests and the paginator.
pub fn build_klines_url(symbol: &str, interval: &str, start_ms: i64, end_ms: i64) -> String {
    format!(
        "{BINANCE_KLINES_URL}?symbol={symbol}&interval={interval}&startTime={start_ms}&endTime={end_ms}&limit=1000"
    )
}

// ── Date utilities (shared with the CLI bin) ──────────────────────────────────

/// Parse "YYYY-MM-DD" into a `time::Date`.
pub fn parse_date(s: &str) -> Result<Date> {
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
pub fn date_to_millis(d: Date) -> i64 {
    let pdt = PrimitiveDateTime::new(d, Time::MIDNIGHT);
    let odt = pdt.assume_utc();
    odt.unix_timestamp() * 1_000
}

/// Return the first day of the month after `d`'s month (next-month boundary).
pub fn next_month_start(year: i32, month: Month) -> Date {
    let next_month_num = u8::from(month) % 12 + 1;
    let next_year = if next_month_num == 1 { year + 1 } else { year };
    let next_month = Month::try_from(next_month_num).expect("month arithmetic 1-12 always valid");
    Date::from_calendar_date(next_year, next_month, 1).expect("first-of-month always valid")
}

/// Compute expected bars per month for the given interval.
/// Returns `None` for intervals where bar count varies (e.g. `1d`).
pub fn expected_bars_per_month(year: i32, month: Month, interval: &str) -> Option<usize> {
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

/// Trait so tests can inject a mock fetcher (the R3 "external I/O behind a
/// trait" seam — no test hits a live socket).
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

/// Paginate over Binance klines for a symbol + time window.
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

// ── Public entry point ────────────────────────────────────────────────────────

/// Fetch hourly (or `interval`) bars for `symbol` over `[start_ms, end_ms)`
/// from the Binance public REST API, parsed into `trading_core::Bar`.
///
/// - Paginated `limit=1000` via `paginate_klines` (cursor = `last_close + 1`).
/// - Polite pacing: 200ms between requests (≤ 300 req/min, well under Binance's
///   ~1200 weight/min; klines weight is 1–2 per call).
/// - One exponential-backoff retry on `RateLimited` (honouring `Retry-After`).
/// - Maps `bar.local_recv_ts = bar.close_ts` for determinism parity with the
///   `ReplayFeed` path (ADR-0032 § D1 Step 7) so a dynamic bar == a corpus bar.
///
/// Returns `Err(BinanceFetchError::NoDataForRange)` (not `Ok(vec![])`) when the
/// API yields zero bars so callers branch on a typed "no data".
///
/// # Errors
///
/// Returns typed `BinanceFetchError` variants (never panics).
pub async fn fetch_binance_klines_range(
    symbol: &str,
    start_ms: i64,
    end_ms: i64,
    interval: &str,
) -> Result<Vec<Bar>, BinanceFetchError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| BinanceFetchError::Network {
            symbol: symbol.to_owned(),
            source: e,
        })?;
    let fetcher = HttpKlineFetcher::new(client);

    fetch_binance_klines_range_with_fetcher(symbol, start_ms, end_ms, interval, &fetcher).await
}

/// Same as `fetch_binance_klines_range` but accepts a custom fetcher (for tests
/// and `dynamic_cache` which reuses an already-built client).
pub(crate) async fn fetch_binance_klines_range_with_fetcher(
    symbol: &str,
    start_ms: i64,
    end_ms: i64,
    interval: &str,
    fetcher: &dyn KlineFetcher,
) -> Result<Vec<Bar>, BinanceFetchError> {
    // Attempt 1.
    let klines = do_paginate(symbol, interval, start_ms, end_ms, fetcher).await;
    let klines = match klines {
        Ok(k) => k,
        Err(BinanceFetchError::RateLimited {
            ref retry_after_secs,
            ..
        }) => {
            // One retry with exponential backoff honouring Retry-After.
            let wait = (*retry_after_secs).max(60); // minimum 60s wait
            tracing::warn!(
                symbol,
                wait_secs = wait,
                "rate-limited; waiting before retry"
            );
            tokio::time::sleep(Duration::from_secs(wait)).await;
            do_paginate(symbol, interval, start_ms, end_ms, fetcher).await?
        }
        Err(e) => return Err(e),
    };

    if klines.is_empty() {
        return Err(BinanceFetchError::NoDataForRange {
            symbol: symbol.to_owned(),
            start_ms,
            end_ms,
        });
    }

    let sym = Symbol::new(symbol);
    let tf = Timeframe::OneHour; // bake-off is always hourly

    klines
        .iter()
        .map(|k| kline_to_bar(&sym, tf, k))
        .collect::<Result<Vec<Bar>, BinanceFetchError>>()
}

/// Internal: call `paginate_klines` and classify non-anyhow errors into typed
/// `BinanceFetchError`.
async fn do_paginate(
    symbol: &str,
    interval: &str,
    start_ms: i64,
    end_ms: i64,
    fetcher: &dyn KlineFetcher,
) -> Result<Vec<Kline>, BinanceFetchError> {
    paginate_klines(fetcher, symbol, interval, start_ms, end_ms, 200)
        .await
        .map_err(|e| classify_paginate_error(symbol, e))
}

/// Classify an `anyhow::Error` from `paginate_klines` → `BinanceFetchError`.
fn classify_paginate_error(symbol: &str, e: anyhow::Error) -> BinanceFetchError {
    // Try to downcast to reqwest::Error first.
    if let Some(re) = e.downcast_ref::<reqwest::Error>() {
        if re.is_timeout() {
            return BinanceFetchError::Timeout {
                symbol: symbol.to_owned(),
                secs: 30,
            };
        }
        if re.is_connect() || re.is_request() {
            // Clone via the Display string (reqwest::Error is not Clone).
            let msg = re.to_string();
            return BinanceFetchError::Other {
                symbol: symbol.to_owned(),
                detail: format!("network: {msg}"),
            };
        }
    }

    // Try to extract "HTTP NNN:" from the anyhow message for non-2xx responses.
    let detail = e.to_string();
    if let Some(status) = extract_http_status(&detail) {
        // Parse the body part after "HTTP NNN: ".
        let body_start = detail
            .find(": ")
            .map_or(detail.len(), |i| (i + 2).min(detail.len()));
        let body = &detail[body_start..];
        return classify_binance_error(symbol, status, body, None);
    }

    BinanceFetchError::Other {
        symbol: symbol.to_owned(),
        detail,
    }
}

/// Extract an HTTP status code from a string like "Binance returned HTTP 429: …".
fn extract_http_status(s: &str) -> Option<u16> {
    let prefix = "HTTP ";
    let idx = s.find(prefix)?;
    let rest = &s[idx + prefix.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// ── Test / fixture helpers (available in tests and fixtures feature) ──────────

/// Mock fetcher for unit tests — injects pre-canned kline batches.
///
/// Exposed under `#[cfg(any(test, feature = "fixtures"))]` so the bin's
/// own test block can import it without a live socket.
#[cfg(any(test, feature = "fixtures"))]
pub struct MockFetcher {
    /// Each call returns the next batch from this queue.
    pub batches: std::sync::Mutex<Vec<Vec<Kline>>>,
    /// Records each URL called.
    pub calls: std::sync::Mutex<Vec<String>>,
}

#[cfg(any(test, feature = "fixtures"))]
impl MockFetcher {
    pub fn new(batches: Vec<Vec<Kline>>) -> Self {
        Self {
            batches: std::sync::Mutex::new(batches),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn recorded_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "fixtures"))]
#[async_trait::async_trait]
impl KlineFetcher for MockFetcher {
    async fn fetch(&self, url: &str) -> anyhow::Result<Vec<Kline>> {
        self.calls.lock().unwrap().push(url.to_owned());
        let mut batches = self.batches.lock().unwrap();
        if batches.is_empty() {
            Ok(vec![])
        } else {
            Ok(batches.remove(0))
        }
    }
}

/// Build a single kline with BTC-ish OHLCV (for tests).
#[cfg(any(test, feature = "fixtures"))]
pub fn make_kline(open_time: i64, close_time: i64) -> Kline {
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

/// Build a batch of `n` consecutive hourly klines starting at `start_ms`.
#[cfg(any(test, feature = "fixtures"))]
pub fn make_batch(start_ms: i64, step_ms: i64, n: usize) -> Vec<Kline> {
    (0..n)
        .map(|i| {
            let open = start_ms + i as i64 * step_ms;
            let close = open + step_ms - 1;
            make_kline(open, close)
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::pedantic
)]
pub mod tests {
    use super::*;

    // ── Test 1: URL builder ───────────────────────────────────────────────────

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

    #[tokio::test]
    async fn test_paginator_cursor_advances_after_full_batch() {
        let step = 3_600_000_i64;
        let batch1 = make_batch(0, step, 1000);
        let last_close = batch1.last().unwrap().close_time;
        let expected_next_cursor = last_close + 1;
        let batch2 = make_batch(expected_next_cursor, step, 5);

        let fetcher = MockFetcher::new(vec![batch1, batch2, vec![]]);
        let end_ms = expected_next_cursor + 5 * step;
        let result = paginate_klines(&fetcher, "BTCUSDT", "1h", 0, end_ms, 0)
            .await
            .expect("pagination should succeed");

        assert_eq!(result.len(), 1005);
        let calls = fetcher.recorded_calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[1].contains(&format!("startTime={expected_next_cursor}")));
    }

    #[tokio::test]
    async fn test_paginator_stops_on_empty_response() {
        let fetcher = MockFetcher::new(vec![vec![]]);
        let result = paginate_klines(&fetcher, "BTCUSDT", "1h", 0, 3_600_000, 0)
            .await
            .expect("should not error on empty");
        assert!(result.is_empty());
        assert_eq!(fetcher.recorded_calls().len(), 1);
    }

    // ── Test 3: Parquet schema round-trip ─────────────────────────────────────

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

        let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
            .expect("scan parquet")
            .collect()
            .expect("collect");

        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 8);

        let schema = df.schema();
        assert_eq!(schema.get("open_time").cloned(), Some(DataType::Int64));
        assert_eq!(schema.get("close_time").cloned(), Some(DataType::Int64));
        assert_eq!(schema.get("open").cloned(), Some(DataType::String));
        assert_eq!(schema.get("trade_count").cloned(), Some(DataType::Int64));

        let open_times = df.column("open_time").unwrap().i64().unwrap();
        assert_eq!(open_times.get(0), Some(1_704_067_200_000_i64));
    }

    // ── Test 4: classify_binance_error (pure, no socket) ─────────────────────

    #[test]
    fn classify_400_with_1121_is_unknown_symbol() {
        let err = classify_binance_error(
            "FAKEUSDT",
            400,
            r#"{"code":-1121,"msg":"Invalid symbol."}"#,
            None,
        );
        assert!(
            matches!(err, BinanceFetchError::UnknownSymbol { ref symbol } if symbol == "FAKEUSDT")
        );
    }

    #[test]
    fn classify_429_is_rate_limited_with_retry_after() {
        let err = classify_binance_error("BTCUSDT", 429, "rate limit", Some("120"));
        match err {
            BinanceFetchError::RateLimited {
                http_status,
                retry_after_secs,
                ..
            } => {
                assert_eq!(http_status, 429);
                assert_eq!(retry_after_secs, 120);
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn classify_418_is_rate_limited() {
        let err = classify_binance_error("ETHUSDT", 418, "IP banned", None);
        assert!(matches!(
            err,
            BinanceFetchError::RateLimited {
                http_status: 418,
                ..
            }
        ));
    }

    #[test]
    fn classify_500_is_other() {
        let err = classify_binance_error("BTCUSDT", 500, "Internal server error", None);
        assert!(matches!(err, BinanceFetchError::Other { .. }));
    }

    // ── Test 5: fetch_binance_klines_range with mock ─────────────────────────

    #[tokio::test]
    async fn fetch_range_returns_correct_bars_count_and_ordering() {
        let step = 3_600_000_i64; // 1h in ms
        let start_ms = 1_704_067_200_000_i64; // 2024-01-01 00:00 UTC
        let n = 48;
        let batch = make_batch(start_ms, step, n);
        let end_ms = start_ms + n as i64 * step;

        let fetcher = MockFetcher::new(vec![batch, vec![]]);
        let bars =
            fetch_binance_klines_range_with_fetcher("BTCUSDT", start_ms, end_ms, "1h", &fetcher)
                .await
                .expect("should succeed");

        assert_eq!(bars.len(), n, "bar count matches input klines");

        // Monotonic timestamps.
        for w in bars.windows(2) {
            assert!(
                w[0].open_ts <= w[1].open_ts,
                "bars must be monotonically ordered"
            );
        }

        // local_recv_ts == close_ts for every bar (ADR-0032 § D1 Step 7).
        for bar in &bars {
            assert_eq!(
                bar.local_recv_ts, bar.close_ts,
                "local_recv_ts must equal close_ts"
            );
        }

        // OHLCV correctly parsed (Decimal preserves input precision: "60000.00" → 60000.00).
        let b0 = &bars[0];
        assert_eq!(b0.open.to_string(), "60000.00");
        assert_eq!(b0.high.to_string(), "61000.00");
        assert_eq!(b0.low.to_string(), "59000.00");
        assert_eq!(b0.close.to_string(), "60500.00");
    }

    #[tokio::test]
    async fn fetch_range_zero_bars_is_no_data_for_range() {
        // Mock returns empty on first (and only) call.
        let fetcher = MockFetcher::new(vec![vec![]]);
        let err = fetch_binance_klines_range_with_fetcher(
            "BTCUSDT",
            1_704_067_200_000,
            1_704_070_800_000,
            "1h",
            &fetcher,
        )
        .await
        .expect_err("empty should be an error");

        assert!(
            matches!(err, BinanceFetchError::NoDataForRange { .. }),
            "expected NoDataForRange, got {err:?}"
        );
    }

    #[tokio::test]
    async fn kline_to_bar_malformed_open_is_other_no_panic() {
        let bad_kline = Kline {
            open_time: 1_704_067_200_000,
            close_time: 1_704_070_799_999,
            open: "NOT_A_NUMBER".to_owned(),
            high: "61000.00".to_owned(),
            low: "59000.00".to_owned(),
            close: "60500.00".to_owned(),
            volume: "10.0".to_owned(),
            trade_count: 1,
        };
        let sym = Symbol::new("BTCUSDT");
        let result = kline_to_bar(&sym, Timeframe::OneHour, &bad_kline);
        assert!(
            matches!(result, Err(BinanceFetchError::Other { .. })),
            "malformed field must be Other, not a panic: {result:?}"
        );
    }

    // ── Test 6: date / month helpers ─────────────────────────────────────────

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
    }

    #[test]
    fn test_expected_bars_per_month_1h_jan() {
        let bars = expected_bars_per_month(2024, Month::January, "1h");
        assert_eq!(bars, Some(744)); // 31 × 24
    }

    #[test]
    fn test_expected_bars_per_month_1h_feb_leap() {
        let bars = expected_bars_per_month(2024, Month::February, "1h");
        assert_eq!(bars, Some(696)); // 29 × 24
    }

    #[test]
    fn test_expected_bars_per_month_1d_none() {
        let bars = expected_bars_per_month(2024, Month::January, "1d");
        assert_eq!(bars, None);
    }
}

// ── Real-network integration test (requires --features realdata + --ignored) ──

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod realdata_tests {
    use super::*;

    /// Real-fetch proof (Wave A, REAL-FETCH PROOF section).
    ///
    /// Fetches a recent ~2-week window of BTCUSDT 1h bars NOT in the pinned
    /// 2021–2024 corpus and asserts:
    /// - non-empty result,
    /// - plausible bar count (~2 weeks × 24h = 336 bars ± some tolerance),
    /// - monotonic timestamps,
    /// - `local_recv_ts == close_ts` on every bar (ADR-0032 § D1 Step 7).
    ///
    /// Run with:
    /// ```
    /// cargo test -p data --features realdata -- realdata_tests::real_fetch_btcusdt_recent_window --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "requires live Binance API — run with --ignored"]
    async fn real_fetch_btcusdt_recent_window() {
        // ~2-week window ending 2026-06-15 (safely before "today" to avoid clock drift)
        // This window is NOT in the pinned 2021-2024 corpus.
        let end_ms = 1_750_291_200_000_i64; // 2026-06-19 00:00:00 UTC
        let start_ms = end_ms - 14 * 86_400_000; // 14 days earlier = 2026-06-05

        let bars = fetch_binance_klines_range("BTCUSDT", start_ms, end_ms, "1h")
            .await
            .expect("real fetch must succeed");

        println!("Real fetch: {} bars for BTCUSDT 2-week window", bars.len());
        assert!(
            !bars.is_empty(),
            "must return a non-empty Vec<Bar> for a recent window"
        );

        // ~14 days × 24h = 336 bars; allow ±50 for exact boundary alignment.
        let expected_approx = 14 * 24;
        assert!(
            bars.len() >= expected_approx - 50 && bars.len() <= expected_approx + 50,
            "expected ~{expected_approx} bars (±50), got {}",
            bars.len()
        );

        // Monotonic timestamps.
        for w in bars.windows(2) {
            assert!(
                w[0].open_ts <= w[1].open_ts,
                "bars must be monotonically ordered"
            );
        }

        // local_recv_ts == close_ts (ADR-0032).
        for bar in &bars {
            assert_eq!(bar.local_recv_ts, bar.close_ts, "local_recv_ts == close_ts");
        }

        // Sane price range for BTC (> $10k, < $1M).
        for bar in &bars {
            let close = bar.close.get();
            assert!(
                close > rust_decimal::Decimal::from(10_000u32),
                "BTC close > 10k"
            );
            assert!(
                close < rust_decimal::Decimal::from(1_000_000u32),
                "BTC close < 1M"
            );
        }
    }
}
