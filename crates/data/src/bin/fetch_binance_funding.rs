//! `fetch_binance_funding` — Binance USDⓈ-M perp historical funding rates → Parquet.
//!
//! Fetches historical funding rates from the public Binance futures REST
//! endpoint (`GET /fapi/v1/fundingRate`) and writes per-symbol-month Parquet
//! files so the backtest harness can align carry data with the OHLCV parquets.
//!
//! # Endpoint
//!
//! `GET https://fapi.binance.com/fapi/v1/fundingRate`
//! Query params: `symbol`, `startTime` (ms), `endTime` (ms, inclusive),
//! `limit` (max 1000)
//!
//! Pagination is forward by `fundingTime`: the next page uses
//! `startTime = last_funding_time + 1`. Every request is bounded by `endTime`
//! (no over-fetch past the month window) and wrapped in a small bounded retry
//! ([`fetch_with_retry`]: 3 attempts, `Retry-After` honored on 429,
//! exponential backoff otherwise).
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
//! | funding_rate  | Utf8   | per-interval rate string, precision-preserved |
//!
//! `funding_rate` is stored as string (like OHLCV prices) to preserve the
//! exact Binance decimal representation without floating-point rounding.
//!
//! # Cadence
//!
//! Binance settles funding every 8 hours (00:00, 08:00, 16:00 UTC) for most
//! USDⓈ-M perps → 3 rows/day, but some symbols run a 4-hour funding interval
//! → 6 rows/day. Completeness checks accept EITHER cadence for a month (no
//! per-symbol interval registry is maintained). The shipped 2023-24 corpus:
//! 10 symbols × 2 years at the 8h cadence ≈ 21 900 rows — small data.
//!
//! # Idempotency & completeness
//!
//! `--start`/`--end` must span whole months (a 1st through a last-of-month).
//! An existing month file is skipped when its row count matches either
//! cadence, or when its bytes match the SHA pinned in the existing
//! `REVISION.toml` (back-port of the klines `8bcfa3a` hardening).
//! `--emit-revision-manifest` refuses to pin a corpus containing incomplete
//! or unverifiable month files unless `--allow-partial` is passed — which
//! still logs every admitted offender loudly.
//!
//! # Revision manifest
//!
//! Use `--emit-revision-manifest` to pin `REVISION.toml` in `--out` per
//! the ADR-0040 revision-pin precedent.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use polars::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;
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

    /// Inclusive start date (YYYY-MM-DD; must be the 1st of a month)
    #[arg(long, default_value = "2023-01-01")]
    start: String,

    /// Inclusive end date (YYYY-MM-DD; must be the last day of a month)
    #[arg(long, default_value = "2024-12-31")]
    end: String,

    /// Output root directory
    #[arg(long, default_value = "data/binance-funding")]
    out: PathBuf,

    /// Overwrite existing files (default: skip files that hold a complete
    /// month, or whose bytes match the SHA pinned in REVISION.toml)
    #[arg(long, default_value_t = false)]
    force: bool,

    /// After all downloads complete, write (or overwrite) a `REVISION.toml`
    /// manifest in `--out` with SHA-256 for every Parquet file present.
    /// Refuses when any month file is incomplete unless `--allow-partial`.
    /// Mirrors the ADR-0032 / ADR-0040 revision-pin precedent.
    #[arg(long, default_value_t = false)]
    emit_revision_manifest: bool,

    /// With `--emit-revision-manifest`: pin the manifest even when some month
    /// files are incomplete or unverifiable. Every admitted offender is
    /// logged loudly first — nothing is recorded silently.
    #[arg(long, default_value_t = false)]
    allow_partial: bool,

    /// Milliseconds to sleep between pagination requests and after each
    /// month that actually hit the API (rate-limit guard).
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
/// `end_ms_exclusive` bounds the response server-side: Binance's `endTime`
/// param is inclusive, so the half-open window `[start_ms, end_ms_exclusive)`
/// maps to `endTime = end_ms_exclusive - 1`. Without it, a month request
/// over-fetched ~10× (the server returned up to `limit` records from
/// `startTime` onward, most past the month).
///
/// Pure function — no I/O. Used by tests.
pub fn build_funding_url(symbol: &str, start_ms: i64, end_ms_exclusive: i64) -> String {
    let end_time_inclusive = end_ms_exclusive - 1;
    format!(
        "{BINANCE_FUNDING_URL}?symbol={symbol}&startTime={start_ms}&endTime={end_time_inclusive}&limit={PAGE_LIMIT}"
    )
}

// ── CLI validation ────────────────────────────────────────────────────────────

/// Trim and validate a CLI-supplied symbol; return it uppercased.
///
/// Only ASCII alphanumerics survive. Anything else would be interpolated raw
/// into both the query URL and the on-disk path: `"BTCUSDT, ETHUSDT"` yields
/// a `" ETHUSDT"` element (→ `symbol=%20ETHUSDT`, a 400 that aborts the run),
/// `..` escapes `--out`, and `&`/`=` rewrite the query string. Reject loudly
/// here instead.
fn validate_symbol(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("symbol is empty after trimming: {raw:?}"));
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(anyhow!(
            "invalid symbol {raw:?}: only ASCII letters and digits are allowed \
             (no spaces, path separators, or URL metacharacters)"
        ));
    }
    Ok(trimmed.to_ascii_uppercase())
}

/// Enforce whole-month `--start`/`--end` bounds; return the exclusive end date.
///
/// Why reject instead of making expected counts window-aware: funding
/// parquets are whole-month files (`<SYMBOL>/<YEAR>/<MM>.parquet`) and every
/// completeness oracle in this tool (`should_skip`,
/// `verify_corpus_completeness`) is keyed to FULL months. A mid-month bound
/// would write a truncated month file indistinguishable on disk from a
/// complete one (the exact hazard the emit gate closes), and such a window is
/// never idempotent (expected = full month vs clamped fetch → perpetual
/// re-fetch). Rejecting up front is the simplest honest contract and keeps a
/// re-run over the complete frozen corpus a pure no-op — the default bounds
/// are month-aligned.
///
/// Also fails loudly when `end` has no next day (`Date::MAX`) instead of
/// silently dropping the final requested day.
fn validate_month_aligned_bounds(start: Date, end: Date) -> Result<Date> {
    if end < start {
        return Err(anyhow!("--end must be >= --start"));
    }
    let end_exclusive = end.next_day().ok_or_else(|| {
        anyhow!(
            "--end {end} is the maximum representable date — cannot compute the \
             exclusive upper bound; choose an earlier month-end"
        )
    })?;
    if start.day() != 1 {
        return Err(anyhow!(
            "--start {start} is not month-aligned: funding data is fetched in whole \
             months (a mid-month start would write a truncated <MM>.parquet that is \
             indistinguishable from a complete month). Use the 1st of the month."
        ));
    }
    if end_exclusive.day() != 1 {
        return Err(anyhow!(
            "--end {end} is not month-aligned: funding data is fetched in whole \
             months (a mid-month end would write a truncated <MM>.parquet that is \
             indistinguishable from a complete month). Use the last day of the month."
        ));
    }
    Ok(end_exclusive)
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

/// Expected settlement counts for a FULL month, one per known funding cadence.
///
/// Binance settles most USDⓈ-M perps every 8 hours (00:00/08:00/16:00 UTC →
/// 3/day), but some symbols run a 4-hour funding interval (6/day). There is
/// deliberately no per-symbol interval registry; a month counts as complete
/// when its row count matches EITHER cadence — the return value is
/// `[days × 3, days × 6]`.
///
/// This function is total: it always returns both candidates. (An earlier doc
/// claimed a conservative `Returns None` path that was never implemented —
/// that lie is gone.) A count matching neither cadence causes a loud re-fetch
/// rather than a silent skip; that is the safe direction for idempotency
/// logic, and the emit-time completeness gate applies the same acceptance.
fn expected_settlements_per_month(year: i32, month: Month) -> [usize; 2] {
    let month_start =
        Date::from_calendar_date(year, month, 1).expect("first-of-month always valid");
    let next_start = next_month_start(year, month);
    let days = (next_start - month_start).whole_days() as usize;
    [days * 3, days * 6] // 8-hour cadence, 4-hour cadence
}

// ── Fetch layer (trait + HTTP impl + retry) ───────────────────────────────────

/// Trait so tests can inject a mock fetcher.
#[async_trait::async_trait]
pub trait FundingFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<Vec<FundingRecord>>;
}

/// Structured fetch error so [`fetch_with_retry`] can classify retryability.
#[derive(Debug, Error)]
pub enum FetchError {
    /// HTTP 429 — retryable; honor the server's `Retry-After` when present.
    #[error("Binance rate limited (429) for {url} (Retry-After: {retry_after:?})")]
    RateLimited {
        url: String,
        retry_after: Option<Duration>,
    },
    /// HTTP 5xx, transport failure, or decode failure — retryable with backoff.
    #[error("transient fetch failure for {url}: {reason}")]
    Transient { url: String, reason: String },
    /// Non-retryable client error (4xx other than 429) — abort immediately.
    #[error("fatal fetch failure for {url}: {reason}")]
    Fatal { url: String, reason: String },
}

/// Total attempts per request (1 initial try + up to 2 retries).
const RETRY_ATTEMPTS: u32 = 3;
/// Base backoff for the exponential retry ladder (base, 2×base, …).
const RETRY_BASE_BACKOFF: Duration = Duration::from_millis(500);
/// Upper bound honored for a server-provided `Retry-After`.
const RETRY_AFTER_CAP: Duration = Duration::from_secs(30);

/// Exponential backoff delay for the given FAILED attempt number (1-based).
fn backoff_delay(base: Duration, attempt: u32) -> Duration {
    base.saturating_mul(1u32 << (attempt - 1).min(16))
}

/// Call `fetcher.fetch(url)` with a small bounded retry.
///
/// Policy: up to `attempts` total tries. [`FetchError::Fatal`] aborts
/// immediately. A 429 honors the server's `Retry-After` (capped at
/// [`RETRY_AFTER_CAP`]); every other retryable failure — including errors
/// that are not a [`FetchError`] at all (e.g. from test seams) — backs off
/// exponentially from `base_backoff`. One 429/5xx/timeout no longer aborts a
/// whole multi-symbol backfill.
pub async fn fetch_with_retry(
    fetcher: &dyn FundingFetcher,
    url: &str,
    attempts: u32,
    base_backoff: Duration,
) -> Result<Vec<FundingRecord>> {
    let mut attempt: u32 = 1;
    loop {
        match fetcher.fetch(url).await {
            Ok(batch) => return Ok(batch),
            Err(err) => {
                let (retryable, delay) = match err.downcast_ref::<FetchError>() {
                    Some(FetchError::Fatal { .. }) => (false, Duration::ZERO),
                    Some(FetchError::RateLimited { retry_after, .. }) => (
                        true,
                        match *retry_after {
                            Some(ra) => ra.min(RETRY_AFTER_CAP),
                            None => backoff_delay(base_backoff, attempt),
                        },
                    ),
                    // Transient — and any non-FetchError error is treated as
                    // transient too (conservative for network blips).
                    _ => (true, backoff_delay(base_backoff, attempt)),
                };
                if !retryable {
                    return Err(err.context(format!("non-retryable fetch failure: {url}")));
                }
                if attempt >= attempts {
                    return Err(
                        err.context(format!("fetch failed after {attempt} attempt(s): {url}"))
                    );
                }
                warn!(
                    url,
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "fetch attempt failed — retrying"
                );
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                attempt += 1;
            }
        }
    }
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
        let resp = match self.client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(FetchError::Transient {
                    url: url.to_owned(),
                    reason: format!("transport error: {e}"),
                }
                .into());
            }
        };

        let status = resp.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(FetchError::RateLimited {
                url: url.to_owned(),
                retry_after,
            }
            .into());
        }
        if status.is_server_error() {
            let body = resp.text().await.unwrap_or_default();
            return Err(FetchError::Transient {
                url: url.to_owned(),
                reason: format!("HTTP {status}: {body}"),
            }
            .into());
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(FetchError::Fatal {
                url: url.to_owned(),
                reason: format!("HTTP {status}: {body}"),
            }
            .into());
        }

        let raw: Vec<RawFundingRecord> = match resp.json().await {
            Ok(raw) => raw,
            Err(e) => {
                return Err(FetchError::Transient {
                    url: url.to_owned(),
                    reason: format!("JSON decode: {e}"),
                }
                .into());
            }
        };

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

// ── Paginator ─────────────────────────────────────────────────────────────────

/// Paginate over Binance funding records for a symbol within `[start_ms, end_ms)`.
///
/// Each request is bounded server-side via `endTime` and wrapped in
/// [`fetch_with_retry`]. The cursor advances to `last_funding_time + 1` after
/// each page; the loop stops when the API returns an empty batch, when the
/// advanced cursor reaches `end_ms`, or when the API returns stale data
/// (last record behind the cursor — defensive). There is deliberately NO
/// "fewer than `PAGE_LIMIT` records" early stop: a short page still advances
/// the cursor and the next (cheap, bounded) request confirms end-of-window.
///
/// The result is sorted by `funding_time` and deduplicated (first occurrence
/// wins) — overlapping server pages cannot persist duplicate settlements.
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
        let url = build_funding_url(symbol, cursor, end_ms);
        let batch = fetch_with_retry(fetcher, &url, RETRY_ATTEMPTS, RETRY_BASE_BACKOFF).await?;
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

    // Dedup by `funding_time`: an overlapping server page (or a boundary
    // record served twice) must not persist duplicate settlements. Sort is
    // stable, so the first-fetched occurrence wins.
    all.sort_by_key(|r| r.funding_time);
    all.dedup_by_key(|r| r.funding_time);

    info!(
        symbol,
        requests = request_count,
        records = all.len(),
        "paginated funding rates"
    );
    Ok(all)
}

// ── Parquet writer ────────────────────────────────────────────────────────────

/// `<dir>/<file_name>.tmp` — same directory, so `rename` stays atomic.
fn tmp_sibling_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map_or_else(std::ffi::OsString::new, |n| n.to_os_string());
    name.push(".tmp");
    path.with_file_name(name)
}

/// Write a `Vec<FundingRecord>` to a Parquet file at `path`.
///
/// Creates parent directories as needed. The write is atomic: bytes go to a
/// same-directory `<name>.tmp` sibling first and are renamed over `path` only
/// after a fully-successful write, so a crash mid-write can never leave a
/// truncated/corrupt parquet at the final path. (The `.tmp` extension is
/// invisible to both the manifest scanner and the backtest loaders.)
pub fn write_parquet(records: &[FundingRecord], path: &Path) -> Result<()> {
    if records.is_empty() {
        warn!(
            ?path,
            "no funding records to write — skipping parquet creation"
        );
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

    let tmp_path = tmp_sibling_path(path);
    if let Err(e) = write_df_parquet(&mut df, &tmp_path) {
        // Best-effort cleanup; a stale .tmp would be inert either way.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }
    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), path.display()))?;

    info!(path = %path.display(), rows = records.len(), "wrote funding parquet");
    Ok(())
}

/// Write `df` directly onto a `File` handle (no userspace buffer, so a
/// successful return means every byte reached the OS before the caller's
/// atomic rename).
fn write_df_parquet(df: &mut DataFrame, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("create parquet file: {}", path.display()))?;
    ParquetWriter::new(file)
        .finish(df)
        .with_context(|| format!("write parquet: {}", path.display()))?;
    Ok(())
}

// ── Idempotency skip ──────────────────────────────────────────────────────────

/// Check idempotency: if the file exists and is already complete, skip it.
///
/// Returns `true` if we should skip this month (no re-fetch needed).
///
/// # Skip decision tree (pinned-SHA hardening back-ported from
/// `fetch_binance_klines`, commit `8bcfa3a`)
///
/// 1. File absent → `false` (must fetch).
/// 2. Row count matches EITHER expected cadence count (8h → days×3, 4h →
///    days×6) → `true` (fast path: full month at a known funding cadence).
/// 3. Row count matches neither AND `pinned_sha` matches the on-disk file's
///    content SHA → `true` (byte-identical to the previously-pinned fetch —
///    the content REVISION.toml already vouches for; no re-fetch needed).
/// 4. Otherwise → `false` (genuinely partial, drifted, or corrupt; re-fetch).
///
/// The `pinned_sha` comes from the existing `REVISION.toml` `[files]` map for
/// this parquet's relative path. When `REVISION.toml` is absent or the path
/// is not listed, pass `None` — step 3 is skipped and the count-only
/// behaviour is preserved. (Unlike the klines sibling there is no
/// unverifiable-interval `Option` arm: funding expected counts are always
/// computable.)
fn should_skip(path: &Path, expected_rows: [usize; 2], pinned_sha: Option<&str>) -> bool {
    if !path.exists() {
        return false;
    }
    let [expected_8h, expected_4h] = expected_rows;
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
                if rows == expected_8h || rows == expected_4h {
                    // Fast path: full month at a known funding cadence.
                    info!(
                        path = %path.display(),
                        rows,
                        "file exists with a full month at a known cadence — skipping"
                    );
                    return true;
                }

                // Rescue path: off-cadence count — check if bytes are
                // identical to the previously-pinned fetch via content SHA.
                if let Some(pin) = pinned_sha {
                    match data::revision::file_sha256(path) {
                        Ok(on_disk_sha) if on_disk_sha == pin => {
                            info!(
                                path = %path.display(),
                                rows,
                                expected_8h,
                                expected_4h,
                                "row count off-cadence but content SHA matches pinned manifest — \
                                 skipping"
                            );
                            return true;
                        }
                        Ok(on_disk_sha) => {
                            warn!(
                                path = %path.display(),
                                rows,
                                expected_8h,
                                expected_4h,
                                on_disk_sha,
                                "row count mismatch and content SHA differs from manifest — will re-fetch"
                            );
                        }
                        Err(e) => {
                            warn!(
                                path = %path.display(),
                                error = %e,
                                "could not hash existing parquet — will re-fetch"
                            );
                        }
                    }
                } else {
                    warn!(
                        path = %path.display(),
                        rows,
                        expected_8h,
                        expected_4h,
                        "row count mismatch (no pinned manifest to verify) — will re-fetch"
                    );
                }
                false
            }
        },
    }
}

// ── Completeness gate (manifest emit) ─────────────────────────────────────────

/// One completeness violation under `--out`, found at manifest-emit time.
#[derive(Debug)]
struct CompletenessOffender {
    /// Path relative to the corpus root, `/`-separated.
    rel_path: String,
    /// Human-readable reason this file cannot be pinned as complete.
    detail: String,
}

/// Verify every parquet under `root` holds a FULL month of settlements.
///
/// This is the emit-time completeness gate: the manifest writer pins every
/// `.parquet` under `root`, so each must hold a full month at a known funding
/// cadence (8h → days×3 rows, 4h → days×6) or be reported as an offender.
/// Files not matching the `<SYMBOL>/<YEAR>/<MM>.parquet` layout are offenders
/// too — the manifest would pin them, so "unverifiable" must not pass
/// silently. Without this gate, a truncated month would be pinned and every
/// downstream SHA check would then prove the integrity of INCOMPLETE data.
///
/// Returns the (deterministically sorted) offender list; empty = emit-clean.
fn verify_corpus_completeness(root: &Path) -> Result<Vec<CompletenessOffender>> {
    let mut paths = Vec::new();
    collect_parquet_paths(root, &mut paths)
        .with_context(|| format!("scan parquet files under {}", root.display()))?;
    paths.sort();

    let mut offenders = Vec::new();
    for path in paths {
        let rel_path = relative_slash_path(root, &path);
        match month_of_layout_path(root, &path) {
            None => offenders.push(CompletenessOffender {
                rel_path,
                detail: "unrecognized path layout (expected <SYMBOL>/<YYYY>/<MM>.parquet) — \
                         cannot verify completeness"
                    .to_owned(),
            }),
            Some((year, month)) => {
                let [expected_8h, expected_4h] = expected_settlements_per_month(year, month);
                match parquet_row_count(&path) {
                    Err(e) => offenders.push(CompletenessOffender {
                        rel_path,
                        detail: format!("unreadable parquet: {e}"),
                    }),
                    Ok(rows) if rows == expected_8h || rows == expected_4h => {}
                    Ok(rows) => offenders.push(CompletenessOffender {
                        rel_path,
                        detail: format!(
                            "{rows} rows — a full {year}-{month:02} needs {expected_8h} \
                             (8h cadence) or {expected_4h} (4h cadence)",
                            month = u8::from(month),
                        ),
                    }),
                }
            }
        }
    }
    Ok(offenders)
}

/// Recursively collect `*.parquet` files under `dir`.
fn collect_parquet_paths(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let path = entry
            .with_context(|| format!("read_dir entry under {}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_parquet_paths(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "parquet") {
            out.push(path);
        }
    }
    Ok(())
}

/// `/`-separated form of `path` relative to `root` (full path on failure).
fn relative_slash_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.display().to_string(),
        |rel| {
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        },
    )
}

/// Parse `(year, month)` from the `<SYMBOL>/<YYYY>/<MM>.parquet` layout.
fn month_of_layout_path(root: &Path, path: &Path) -> Option<(i32, Month)> {
    let rel = path.strip_prefix(root).ok()?;
    let comps: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if comps.len() != 3 {
        return None;
    }
    let year: i32 = comps[1].parse().ok()?;
    let stem = Path::new(&comps[2])
        .file_stem()?
        .to_string_lossy()
        .into_owned();
    let month_num: u8 = stem.parse().ok()?;
    let month = Month::try_from(month_num).ok()?;
    Some((year, month))
}

/// Row count of a parquet file.
fn parquet_row_count(path: &Path) -> Result<usize> {
    let df = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .with_context(|| format!("scan parquet {}", path.display()))?
        .collect()
        .with_context(|| format!("collect parquet {}", path.display()))?;
    Ok(df.height())
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
    // Trim + validate before anything is interpolated into a URL or path.
    let symbols = cli
        .symbols
        .iter()
        .map(|s| validate_symbol(s))
        .collect::<Result<Vec<String>>>()?;

    let start_date =
        parse_date(&cli.start).with_context(|| format!("parse --start date: {}", cli.start))?;
    let end_date =
        parse_date(&cli.end).with_context(|| format!("parse --end date: {}", cli.end))?;
    let end_exclusive = validate_month_aligned_bounds(start_date, end_date)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")?;
    let fetcher = HttpFundingFetcher::new(client);

    // Load the existing REVISION.toml manifest once (if present) — the
    // pinned-SHA idempotency back-port of the klines 8bcfa3a hardening. The
    // `[files]` map records per-file content SHAs from the previous fetch;
    // `should_skip` uses them to recognise byte-identical months without
    // hitting the network. First run (no manifest) → empty map → count-only.
    let pinned_manifest: BTreeMap<String, String> =
        match data::revision::read_manifest_raw(&cli.out) {
            Ok((files, _agg)) => {
                info!(
                    out = %cli.out.display(),
                    files = files.len(),
                    "loaded existing REVISION.toml for idempotency check"
                );
                files
            }
            Err(_) => {
                info!(
                    out = %cli.out.display(),
                    "no existing REVISION.toml — first-run mode, cadence-count check only"
                );
                BTreeMap::new()
            }
        };

    for symbol_upper in &symbols {
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

            // Month-aligned bounds are enforced up front, so for every
            // in-range month the fetch window is exactly
            // [month_start, month_end_exclusive) — these clamps are pure
            // defense in depth (identities).
            let window_start = month_start.max(start_date);
            let window_end = month_end_exclusive.min(end_exclusive);

            let start_ms = date_to_millis(window_start);
            let end_ms = date_to_millis(window_end);

            // Parquet path: <out>/<SYMBOL>/<YEAR>/<MM>.parquet
            let month_num = u8::from(month);
            let parquet_path = cli
                .out
                .join(symbol_upper)
                .join(year.to_string())
                .join(format!("{month_num:02}.parquet"));

            let expected = expected_settlements_per_month(year, month);

            // Relative manifest key matches the layout written by the
            // manifest writer: "<SYMBOL>/<YEAR>/<MM>.parquet".
            let manifest_key = format!("{symbol_upper}/{year}/{month_num:02}.parquet");
            let pinned_sha = pinned_manifest.get(&manifest_key).map(String::as_str);

            if !cli.force && should_skip(&parquet_path, expected, pinned_sha) {
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
                expected_rows_8h = expected[0],
                expected_rows_4h = expected[1],
                "fetching month funding rates"
            );

            let records = paginate_funding(&fetcher, symbol_upper, start_ms, end_ms, cli.sleep_ms)
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

            // Inter-month throttle (rate-limit hygiene): sleep only after a
            // month that actually hit the API — a fully-skipped idempotent
            // re-run over a complete corpus stays network-free AND sleep-free.
            if cli.sleep_ms > 0 {
                tokio::time::sleep(Duration::from_millis(cli.sleep_ms)).await;
            }

            if advance_month(&mut year, &mut month, end_date) {
                break;
            }
        }

        info!(symbol = %symbol_upper, "finished funding download");
    }

    // Emit REVISION.toml after all fetches complete.
    if cli.emit_revision_manifest {
        // Completeness gate: the manifest pins whatever bytes are on disk, so
        // refuse to pin a corpus containing truncated or unverifiable month
        // files — otherwise downstream SHA gates would forever prove the
        // integrity of INCOMPLETE data.
        let offenders = verify_corpus_completeness(&cli.out)?;
        if !offenders.is_empty() {
            if cli.allow_partial {
                // Escape hatch — but never a silent one: every admitted
                // offender is logged before the manifest pins it.
                for o in &offenders {
                    warn!(
                        file = %o.rel_path,
                        detail = %o.detail,
                        "--allow-partial: pinning an incomplete/unverifiable month into REVISION.toml"
                    );
                }
                println!(
                    "[REVISION][ALLOW-PARTIAL] pinning {} incomplete/unverifiable month file(s) — \
                     see warnings above",
                    offenders.len()
                );
            } else {
                let listing = offenders
                    .iter()
                    .map(|o| format!("  {} — {}", o.rel_path, o.detail))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(anyhow!(
                    "refusing --emit-revision-manifest: {count} file(s) under {out} do not hold \
                     a complete month:\n{listing}\n\
                     Re-fetch the offending months, or pass --allow-partial to pin them anyway \
                     (loudly).",
                    count = offenders.len(),
                    out = cli.out.display(),
                ));
            }
        }

        let agg_sha = data::revision::write_revision_manifest_with_tool(
            &cli.out,
            data::revision::RevisionMetadataInput {
                fetch_tool: "fetch_binance_funding",
                binance_base: "https://fapi.binance.com",
                interval: None, // funding is event-driven (8h/4h), not a bar interval
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
        let url = build_funding_url("BTCUSDT", 1_672_531_200_000, 1_675_209_600_000);
        assert!(url.contains("symbol=BTCUSDT"), "symbol in url: {url}");
        assert!(
            url.contains("startTime=1672531200000"),
            "startTime in url: {url}"
        );
        assert!(
            url.contains("endTime=1675209599999"),
            "inclusive endTime bounds the response to the window: {url}"
        );
        assert!(url.contains("limit=1000"), "limit in url: {url}");
        assert!(
            url.starts_with("https://fapi.binance.com/fapi/v1/fundingRate"),
            "correct base url: {url}"
        );
    }

    #[test]
    fn test_url_builder_ethusdt() {
        let url = build_funding_url("ETHUSDT", 0, 1_000);
        assert!(url.contains("symbol=ETHUSDT"));
        assert!(url.contains("startTime=0"));
        assert!(
            url.contains("endTime=999"),
            "endTime = end_exclusive - 1: {url}"
        );
    }

    // ── Wire-format serde (the only code touching the real JSON shape) ────────

    #[test]
    fn test_raw_funding_record_camelcase_wire_decode() {
        // Verbatim shape of the Binance /fapi/v1/fundingRate response —
        // camelCase keys plus a field we deliberately do not store (markPrice).
        let wire = r#"[
            {"symbol":"BTCUSDT","fundingTime":1672531200000,"fundingRate":"0.00010000","markPrice":"16512.35000000"},
            {"symbol":"BTCUSDT","fundingTime":1672560000000,"fundingRate":"-0.00005000","markPrice":"16600.00000000"}
        ]"#;
        let raw: Vec<RawFundingRecord> = serde_json::from_str(wire).expect("wire decode");
        assert_eq!(raw.len(), 2);
        assert_eq!(raw[0].symbol, "BTCUSDT");
        assert_eq!(raw[0].funding_time, 1_672_531_200_000);
        assert_eq!(raw[0].funding_rate, "0.00010000");
        assert_eq!(raw[1].funding_time, 1_672_560_000_000);
        assert_eq!(
            raw[1].funding_rate, "-0.00005000",
            "sign + precision preserved as string"
        );
    }

    #[test]
    fn test_raw_funding_record_wire_decode_rejects_snake_case() {
        // snake_case keys are NOT the wire format — rename_all = "camelCase"
        // must reject them (fundingTime/fundingRate missing), never default.
        let bad = r#"[{"symbol":"BTCUSDT","funding_time":1672531200000,"funding_rate":"0.0001"}]"#;
        assert!(
            serde_json::from_str::<Vec<RawFundingRecord>>(bad).is_err(),
            "snake_case keys must fail the camelCase wire mapping"
        );
    }

    // ── Expected settlements per month (both cadences) ────────────────────────

    #[test]
    fn test_expected_settlements_jan() {
        // January 2023: 31 days → 93 at the 8h cadence, 186 at the 4h cadence.
        assert_eq!(
            expected_settlements_per_month(2023, Month::January),
            [93, 186]
        );
    }

    #[test]
    fn test_expected_settlements_feb_leap() {
        // February 2024 (leap): 29 days → 87 / 174.
        assert_eq!(
            expected_settlements_per_month(2024, Month::February),
            [87, 174]
        );
    }

    #[test]
    fn test_expected_settlements_feb_non_leap() {
        // February 2023: 28 days → 84 / 168.
        assert_eq!(
            expected_settlements_per_month(2023, Month::February),
            [84, 168]
        );
    }

    #[test]
    fn test_expected_settlements_dec() {
        // December: 31 days → 93 / 186.
        assert_eq!(
            expected_settlements_per_month(2023, Month::December),
            [93, 186]
        );
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
        // Page 1 (1000 records) advances the cursor; page 2 (93 records, last
        // still < end_ms) advances again; page 3 returns empty and stops the
        // loop → exactly 3 requests. An exact count guards against request-
        // amplification regressions.
        assert_eq!(
            calls.len(),
            3,
            "exactly 3 requests: full page, short page, empty confirm"
        );

        // Second call must start at next_cursor.
        assert!(
            calls[1].contains(&format!("startTime={next_cursor}")),
            "second request must use cursor={next_cursor}, got: {}",
            calls[1]
        );
        // Every request is bounded server-side by the window's endTime.
        let expected_end_time = end_ms - 1;
        assert!(
            calls[0].contains(&format!("endTime={expected_end_time}")),
            "first request must carry endTime={expected_end_time}, got: {}",
            calls[0]
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

    /// Overlapping server pages must not persist duplicate settlements.
    #[tokio::test]
    async fn test_paginator_dedups_overlapping_pages() {
        let t0 = 1_672_531_200_000_i64;
        let end_ms = t0 + 4 * EIGHT_HOURS_MS;

        // Page 2 re-serves page 1's last record (server-side overlap).
        let page1 = make_batch("BTCUSDT", t0, 3); // t0, +8h, +16h
        let page2 = vec![
            make_record("BTCUSDT", t0 + 2 * EIGHT_HOURS_MS, "0.00010000"), // duplicate
            make_record("BTCUSDT", t0 + 3 * EIGHT_HOURS_MS, "0.00020000"),
        ];

        let fetcher = MockFetcher::new(vec![page1, page2, vec![]]);
        let result = paginate_funding(&fetcher, "BTCUSDT", t0, end_ms, 0)
            .await
            .expect("paginate");

        assert_eq!(result.len(), 4, "duplicate settlement must be dropped");
        let times: Vec<i64> = result.iter().map(|r| r.funding_time).collect();
        for w in times.windows(2) {
            assert!(
                w[0] < w[1],
                "funding_time must be strictly increasing, got {times:?}"
            );
        }
    }

    // ── fetch_with_retry (bounded retry / backoff classification) ─────────────

    /// Mock that replays a fixed sequence of results (errors allowed).
    struct SequenceFetcher {
        responses: std::sync::Mutex<Vec<Result<Vec<FundingRecord>>>>,
        calls: std::sync::Mutex<u32>,
    }

    impl SequenceFetcher {
        fn new(responses: Vec<Result<Vec<FundingRecord>>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
                calls: std::sync::Mutex::new(0),
            }
        }

        fn call_count(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }

    #[async_trait::async_trait]
    impl FundingFetcher for SequenceFetcher {
        async fn fetch(&self, _url: &str) -> Result<Vec<FundingRecord>> {
            *self.calls.lock().unwrap() += 1;
            let mut rs = self.responses.lock().unwrap();
            if rs.is_empty() {
                Ok(vec![])
            } else {
                rs.remove(0)
            }
        }
    }

    fn transient_err() -> anyhow::Error {
        FetchError::Transient {
            url: "test://u".into(),
            reason: "boom".into(),
        }
        .into()
    }

    /// Two transient failures then success — exponential backoff from base.
    #[tokio::test(start_paused = true)]
    async fn test_fetch_with_retry_transient_then_success_backs_off() {
        let fetcher = SequenceFetcher::new(vec![
            Err(transient_err()),
            Err(transient_err()),
            Ok(make_batch("BTCUSDT", 0, 2)),
        ]);
        let t0 = tokio::time::Instant::now();
        let out = fetch_with_retry(&fetcher, "test://u", 3, Duration::from_millis(500))
            .await
            .expect("third attempt succeeds");
        let elapsed = t0.elapsed();
        assert_eq!(out.len(), 2);
        assert_eq!(fetcher.call_count(), 3);
        // 500ms after attempt 1, 1000ms after attempt 2 (paused clock is exact).
        assert!(
            elapsed >= Duration::from_millis(1500) && elapsed < Duration::from_millis(1700),
            "exponential backoff 500+1000ms expected, got {elapsed:?}"
        );
    }

    /// All attempts fail → bounded stop at exactly `attempts` tries.
    #[tokio::test]
    async fn test_fetch_with_retry_exhausts_attempts() {
        let fetcher = SequenceFetcher::new(vec![
            Err(transient_err()),
            Err(transient_err()),
            Err(transient_err()),
        ]);
        let err = fetch_with_retry(&fetcher, "test://u", 3, Duration::ZERO)
            .await
            .expect_err("all attempts fail");
        assert_eq!(fetcher.call_count(), 3, "bounded retry stops at 3 attempts");
        assert!(
            format!("{err:#}").contains("after 3 attempt"),
            "error names the attempt budget: {err:#}"
        );
    }

    /// A non-429 4xx is fatal: no retries, immediate abort.
    #[tokio::test]
    async fn test_fetch_with_retry_fatal_aborts_immediately() {
        let fetcher = SequenceFetcher::new(vec![
            Err(FetchError::Fatal {
                url: "test://u".into(),
                reason: "HTTP 400 Bad Request".into(),
            }
            .into()),
            Ok(make_batch("BTCUSDT", 0, 1)), // must never be reached
        ]);
        let err = fetch_with_retry(&fetcher, "test://u", 3, Duration::ZERO)
            .await
            .expect_err("fatal aborts");
        assert_eq!(fetcher.call_count(), 1, "no retry on a non-retryable error");
        assert!(format!("{err:#}").contains("non-retryable"), "got: {err:#}");
    }

    /// A 429 honors the server-provided Retry-After over the exponential base.
    #[tokio::test(start_paused = true)]
    async fn test_fetch_with_retry_rate_limited_honors_retry_after() {
        let fetcher = SequenceFetcher::new(vec![
            Err(FetchError::RateLimited {
                url: "test://u".into(),
                retry_after: Some(Duration::from_secs(7)),
            }
            .into()),
            Ok(make_batch("BTCUSDT", 0, 1)),
        ]);
        let t0 = tokio::time::Instant::now();
        let out = fetch_with_retry(&fetcher, "test://u", 3, Duration::from_millis(500))
            .await
            .expect("succeeds after 429");
        let elapsed = t0.elapsed();
        assert_eq!(out.len(), 1);
        assert_eq!(fetcher.call_count(), 2);
        assert!(
            elapsed >= Duration::from_secs(7) && elapsed < Duration::from_secs(8),
            "Retry-After (7s) honored over the 500ms base, got {elapsed:?}"
        );
    }

    /// An absurd Retry-After is capped so one header cannot stall the backfill.
    #[tokio::test(start_paused = true)]
    async fn test_fetch_with_retry_retry_after_capped() {
        let fetcher = SequenceFetcher::new(vec![
            Err(FetchError::RateLimited {
                url: "test://u".into(),
                retry_after: Some(Duration::from_secs(600)),
            }
            .into()),
            Ok(make_batch("BTCUSDT", 0, 1)),
        ]);
        let t0 = tokio::time::Instant::now();
        fetch_with_retry(&fetcher, "test://u", 3, Duration::ZERO)
            .await
            .expect("succeeds after capped wait");
        let elapsed = t0.elapsed();
        assert!(
            elapsed >= Duration::from_secs(30) && elapsed < Duration::from_secs(31),
            "server Retry-After capped at 30s, got {elapsed:?}"
        );
    }

    /// Errors that are not FetchError (e.g. test seams) retry as transient.
    #[tokio::test]
    async fn test_fetch_with_retry_unknown_error_is_transient() {
        let fetcher = SequenceFetcher::new(vec![
            Err(anyhow!("some opaque failure")),
            Ok(make_batch("BTCUSDT", 0, 1)),
        ]);
        let out = fetch_with_retry(&fetcher, "test://u", 3, Duration::ZERO)
            .await
            .expect("opaque error retried");
        assert_eq!(out.len(), 1);
        assert_eq!(fetcher.call_count(), 2);
    }

    // ── Parquet schema round-trip + atomic write ──────────────────────────────

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

    #[test]
    fn test_tmp_sibling_path_shape() {
        let p = Path::new("/x/y/01.parquet");
        assert_eq!(tmp_sibling_path(p), PathBuf::from("/x/y/01.parquet.tmp"));
    }

    #[test]
    fn test_write_parquet_atomic_no_tmp_leftover() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("01.parquet");
        write_parquet(&make_batch("BTCUSDT", 0, 3), &path).expect("write");
        assert!(path.exists(), "final parquet present");
        assert!(
            !tmp.path().join("01.parquet.tmp").exists(),
            "tmp sibling must be renamed away"
        );
    }

    // ── should_skip: the sole overwrite-vs-skip guard (klines 8bcfa3a mirror) ─

    /// Helper: write a real-schema funding parquet with `n` rows at `path`.
    fn write_n_row_funding_parquet(path: &Path, n: usize) {
        write_parquet(&make_batch("BTCUSDT", 1_672_531_200_000, n), path)
            .expect("write_n_row_funding_parquet");
    }

    /// January expected counts: [93 (8h), 186 (4h)].
    const JAN_EXPECTED: [usize; 2] = [93, 186];

    /// A full 8h-cadence month is skipped without any manifest — this is the
    /// frozen-corpus fast path (the shipped 2023-24 corpus is entirely 8h).
    #[test]
    fn test_should_skip_full_8h_month_skips_without_manifest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("01.parquet");
        write_n_row_funding_parquet(&p, 93);
        assert!(
            should_skip(&p, JAN_EXPECTED, None),
            "full 8h-cadence month must be skipped without a manifest"
        );
        // A (wrong) pinned SHA must not matter — the fast path returns first.
        let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(
            should_skip(&p, JAN_EXPECTED, Some(wrong_sha)),
            "fast path must not consult the SHA"
        );
    }

    /// A full 4h-cadence month (6 settlements/day) is equally complete.
    #[test]
    fn test_should_skip_full_4h_month_skips_without_manifest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("01.parquet");
        write_n_row_funding_parquet(&p, 186);
        assert!(
            should_skip(&p, JAN_EXPECTED, None),
            "full 4h-cadence month must be skipped (either-cadence acceptance)"
        );
    }

    /// Off-cadence count with a matching pinned content SHA must be SKIPPED —
    /// byte-identical to the fetch REVISION.toml already vouches for.
    #[test]
    fn test_should_skip_short_month_with_matching_pinned_sha_returns_true() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("01.parquet");
        write_n_row_funding_parquet(&p, 90); // neither 93 nor 186

        let on_disk_sha = data::revision::file_sha256(&p).expect("sha256");
        assert!(
            should_skip(&p, JAN_EXPECTED, Some(&on_disk_sha)),
            "off-cadence month with matching pinned SHA must be skipped"
        );
    }

    /// Off-cadence count and a mismatched pinned SHA (drifted content) → re-fetch.
    #[test]
    fn test_should_skip_short_month_with_mismatched_pinned_sha_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("01.parquet");
        write_n_row_funding_parquet(&p, 90);

        let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(
            !should_skip(&p, JAN_EXPECTED, Some(wrong_sha)),
            "mismatched SHA must trigger re-fetch"
        );
    }

    /// Off-cadence count with NO pinned manifest → re-fetch (first-run path).
    #[test]
    fn test_should_skip_short_month_no_manifest_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("01.parquet");
        write_n_row_funding_parquet(&p, 90);

        assert!(
            !should_skip(&p, JAN_EXPECTED, None),
            "no manifest → off-cadence month must trigger re-fetch"
        );
    }

    /// A missing file must never be skipped.
    #[test]
    fn test_should_skip_absent_file_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("missing.parquet");

        assert!(!should_skip(&p, JAN_EXPECTED, None), "absent file → fetch");
        assert!(
            !should_skip(&p, JAN_EXPECTED, Some("anysha")),
            "absent file → fetch even with a manifest SHA"
        );
    }

    /// Frozen-corpus no-op proof: a complete corpus (full months at the 8h
    /// cadence, like the shipped 2023-24 corpus) with its pinned manifest is
    /// (a) skipped month-for-month by `should_skip` — zero network calls —
    /// and (b) emit-clean under `verify_corpus_completeness`.
    #[test]
    fn test_frozen_corpus_shape_is_pure_noop() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let months: [(&str, i32, Month, usize); 4] = [
            ("BTCUSDT", 2023, Month::January, 93),
            ("BTCUSDT", 2024, Month::February, 87), // leap
            ("ETHUSDT", 2023, Month::February, 84),
            ("ETHUSDT", 2023, Month::April, 90),
        ];
        for (sym, year, month, rows) in months {
            let path = root
                .join(sym)
                .join(year.to_string())
                .join(format!("{:02}.parquet", u8::from(month)));
            write_n_row_funding_parquet(&path, rows);
        }
        let agg = data::revision::write_revision_manifest_with_tool(
            root,
            data::revision::RevisionMetadataInput {
                fetch_tool: "fetch_binance_funding",
                binance_base: "https://fapi.binance.com",
                interval: None,
            },
        )
        .expect("write manifest over complete corpus");
        assert_eq!(agg.len(), 64);

        let (pinned, _claimed) = data::revision::read_manifest_raw(root).expect("read manifest");
        for (sym, year, month, _rows) in months {
            let rel = format!("{sym}/{year}/{:02}.parquet", u8::from(month));
            let path = root.join(&rel);
            let expected = expected_settlements_per_month(year, month);
            let pin = pinned.get(&rel).map(String::as_str);
            assert!(
                should_skip(&path, expected, pin),
                "complete corpus month {rel} must be skipped (no re-fetch)"
            );
        }

        let offenders = verify_corpus_completeness(root).expect("completeness scan");
        assert!(
            offenders.is_empty(),
            "complete corpus must be emit-clean, got {offenders:?}"
        );
    }

    // ── Completeness gate ─────────────────────────────────────────────────────

    #[test]
    fn test_verify_corpus_completeness_clean_both_cadences() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        // 8h-cadence January (93 rows) + 4h-cadence January (186 rows).
        write_n_row_funding_parquet(&root.join("BTCUSDT/2023/01.parquet"), 93);
        write_n_row_funding_parquet(&root.join("ALTUSDT/2023/01.parquet"), 186);
        let offenders = verify_corpus_completeness(root).expect("scan");
        assert!(
            offenders.is_empty(),
            "complete months at either cadence are emit-clean, got {offenders:?}"
        );
    }

    #[test]
    fn test_verify_corpus_completeness_flags_truncated_month() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write_n_row_funding_parquet(&root.join("BTCUSDT/2023/01.parquet"), 93);
        // Truncated February 2023 (needs 84 or 168).
        write_n_row_funding_parquet(&root.join("BTCUSDT/2023/02.parquet"), 50);
        let offenders = verify_corpus_completeness(root).expect("scan");
        assert_eq!(offenders.len(), 1, "exactly the truncated month flagged");
        assert_eq!(offenders[0].rel_path, "BTCUSDT/2023/02.parquet");
        assert!(
            offenders[0].detail.contains("50 rows"),
            "detail names the actual count: {}",
            offenders[0].detail
        );
        assert!(
            offenders[0].detail.contains("84") && offenders[0].detail.contains("168"),
            "detail lists both cadence expectations: {}",
            offenders[0].detail
        );
    }

    #[test]
    fn test_verify_corpus_completeness_flags_unrecognized_layout() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write_n_row_funding_parquet(&root.join("stray.parquet"), 93);
        let offenders = verify_corpus_completeness(root).expect("scan");
        assert_eq!(offenders.len(), 1);
        assert_eq!(offenders[0].rel_path, "stray.parquet");
        assert!(
            offenders[0].detail.contains("layout"),
            "unverifiable layout must be an offender, not a silent pass: {}",
            offenders[0].detail
        );
    }

    // ── CLI validation ────────────────────────────────────────────────────────

    #[test]
    fn test_validate_symbol_trims_and_uppercases() {
        assert_eq!(validate_symbol(" ETHUSDT ").expect("valid"), "ETHUSDT");
        assert_eq!(validate_symbol("btcusdt").expect("valid"), "BTCUSDT");
        assert_eq!(
            validate_symbol("1000SHIBUSDT").expect("valid"),
            "1000SHIBUSDT"
        );
    }

    #[test]
    fn test_validate_symbol_rejects_junk() {
        // ("BTCUSDT, ETHUSDT" splits to " ETHUSDT" — the trim above handles
        // that; everything else must be rejected loudly.)
        assert!(validate_symbol("BTC USDT").is_err(), "embedded space");
        assert!(validate_symbol("../evil").is_err(), "path escape");
        assert!(validate_symbol("BTC&limit=1").is_err(), "URL metacharacter");
        assert!(validate_symbol("").is_err(), "empty");
        assert!(validate_symbol("   ").is_err(), "whitespace only");
    }

    // ── Month-aligned bounds ──────────────────────────────────────────────────

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
    }

    #[test]
    fn test_month_aligned_bounds_accepts_whole_months() {
        // The default CLI bounds (the frozen-corpus invocation) are aligned.
        let end_ex =
            validate_month_aligned_bounds(d(2023, 1, 1), d(2024, 12, 31)).expect("defaults align");
        assert_eq!(end_ex, d(2025, 1, 1));
        // Leap-year February.
        assert_eq!(
            validate_month_aligned_bounds(d(2024, 2, 1), d(2024, 2, 29)).expect("leap feb"),
            d(2024, 3, 1)
        );
        // Single whole month.
        assert_eq!(
            validate_month_aligned_bounds(d(2023, 2, 1), d(2023, 2, 28)).expect("non-leap feb"),
            d(2023, 3, 1)
        );
    }

    #[test]
    fn test_month_aligned_bounds_rejects_midmonth() {
        let err = validate_month_aligned_bounds(d(2023, 1, 2), d(2023, 12, 31))
            .expect_err("mid-month start");
        assert!(err.to_string().contains("month-aligned"), "got: {err}");

        let err = validate_month_aligned_bounds(d(2023, 1, 1), d(2023, 12, 30))
            .expect_err("mid-month end");
        assert!(err.to_string().contains("month-aligned"), "got: {err}");

        // Feb 28 in a LEAP year is mid-month.
        assert!(validate_month_aligned_bounds(d(2024, 2, 1), d(2024, 2, 28)).is_err());

        // Reversed bounds are rejected too.
        assert!(validate_month_aligned_bounds(d(2024, 1, 1), d(2023, 12, 31)).is_err());
    }

    #[test]
    fn test_month_aligned_bounds_date_max_fails_loud() {
        // Date::MAX has no next day — must fail loudly, never silently drop
        // the final requested day (the old `unwrap_or(end_date)` behaviour).
        let start = Date::from_calendar_date(Date::MAX.year(), Month::December, 1).unwrap();
        let err =
            validate_month_aligned_bounds(start, Date::MAX).expect_err("Date::MAX end must error");
        assert!(
            err.to_string().contains("maximum representable"),
            "got: {err}"
        );
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
