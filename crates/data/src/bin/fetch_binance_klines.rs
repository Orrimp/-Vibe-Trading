//! `fetch_binance_klines` — Binance REST klines → Parquet downloader.
//!
//! Fetches historical OHLCV (klines) from Binance public REST API and writes
//! per-symbol-month Parquet files that `ReplayFeed` can read directly.
//!
//! Core fetch + parse + parquet-write logic lives in
//! `crates/data/src/binance_klines.rs` (extracted in Wave A of
//! `advisor-dynamic-data`). This bin re-exports those pieces and adds only the
//! CLI glue (`Cli`, month-iteration, skip-idempotency, `main`).
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

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use polars::prelude::*;
use reqwest::Client;
use time::{Date, Month};
use tracing::{info, warn};
// EnvFilter now used via llm::tracing_init::install_global (T-RED-D12).

// Re-export the shared types from the library module (Wave A extraction).
use data::binance_klines::{
    HttpKlineFetcher, date_to_millis, expected_bars_per_month, next_month_start, paginate_klines,
    parse_date, write_parquet,
};

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

/// Check idempotency: if file exists and is already complete, skip it.
///
/// Returns `true` if we should skip this month (no re-fetch needed).
///
/// # Skip decision tree
///
/// 1. File absent → `false` (must fetch).
/// 2. Interval bars unverifiable (e.g. `1d`) → `true` conservatively (file exists).
/// 3. Row count == calendar-expected → `true` (fast path: full month with no gaps).
/// 4. Row count < calendar-expected AND `pinned_sha` matches the on-disk file's
///    content SHA → `true` (short-but-complete: a legitimately-gapped month whose
///    bytes are byte-identical to the previously-pinned fetch; no re-fetch needed).
/// 5. Otherwise → `false` (genuinely partial or corrupt file; re-fetch).
///
/// The `pinned_sha` comes from the existing `REVISION.toml` `[files]` map for
/// this parquet's relative path.  When `REVISION.toml` is absent or the path is
/// not listed, pass `None` — step 4 is skipped and the old behaviour is preserved.
fn should_skip(
    path: &std::path::Path,
    expected_bars: Option<usize>,
    pinned_sha: Option<&str>,
) -> bool {
    if !path.exists() {
        return false;
    }
    let Some(expected) = expected_bars else {
        // Cannot check bar count for this interval — skip conservatively.
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
                    return true;
                }

                // Rescue path: short month — check if bytes are identical to
                // the previously-pinned fetch via content SHA comparison.
                if let Some(pin) = pinned_sha {
                    match data::revision::file_sha256(path) {
                        Ok(on_disk_sha) if on_disk_sha == pin => {
                            info!(
                                path = %path.display(),
                                rows,
                                expected,
                                "row count short but content SHA matches pinned manifest — \
                                 legitimately gapped month, skipping"
                            );
                            return true;
                        }
                        Ok(on_disk_sha) => {
                            warn!(
                                path = %path.display(),
                                rows,
                                expected,
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
                        expected,
                        "row count mismatch (no pinned manifest to verify) — will re-fetch"
                    );
                }
                false
            }
        },
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
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
    let fetcher = HttpKlineFetcher::new(client);

    // Load the existing REVISION.toml manifest once (if present).
    // The `[files]` map records per-file content SHAs from the previous fetch.
    // We pass the pinned SHA to `should_skip` so legitimately-short months
    // (real Binance exchange gaps) are recognised as byte-identical and skipped
    // without hitting the network.  When no manifest exists yet (first run),
    // `pinned_manifest` is empty and we fall back to calendar-count-only logic.
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
                    "no existing REVISION.toml — first-run mode, calendar-count check only"
                );
                BTreeMap::new()
            }
        };

    for symbol in &cli.symbols {
        let symbol_upper = symbol.to_uppercase();
        info!(symbol = %symbol_upper, "starting download");

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
    use super::should_skip;
    use super::*;
    use anyhow::Result;
    use data::binance_klines::{
        Kline, KlineFetcher, build_klines_url, expected_bars_per_month, next_month_start,
        paginate_klines, parse_date, write_parquet,
    };

    // Inline mock fetcher for the bin's tests (the library's MockFetcher is
    // gated by #[cfg(test)] which is a different compilation unit here).
    struct BinMockFetcher {
        batches: std::sync::Mutex<Vec<Vec<Kline>>>,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl BinMockFetcher {
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
    impl KlineFetcher for BinMockFetcher {
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

    fn bin_make_kline(open_time: i64, close_time: i64) -> Kline {
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

    fn bin_make_batch(start_ms: i64, step_ms: i64, n: usize) -> Vec<Kline> {
        (0..n)
            .map(|i| {
                let open = start_ms + i as i64 * step_ms;
                bin_make_kline(open, open + step_ms - 1)
            })
            .collect()
    }

    // ── Test 1: URL builder (re-exported from lib) ────────────────────────────

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
        assert!(url.contains("symbol=ETHUSDT"));
        assert!(url.contains("interval=1m"));
        assert!(url.contains("startTime=0"));
        assert!(url.contains("endTime=60000"));
        assert!(url.contains("limit=1000"));
    }

    // ── Test 2: Paginator boundary logic ─────────────────────────────────────

    #[tokio::test]
    async fn test_paginator_cursor_advances_after_full_batch() {
        let step = 3_600_000_i64;
        let batch1 = bin_make_batch(0, step, 1000);
        let last_close = batch1.last().unwrap().close_time;
        let expected_next_cursor = last_close + 1;
        let batch2 = bin_make_batch(expected_next_cursor, step, 5);

        let fetcher = BinMockFetcher::new(vec![batch1, batch2, vec![]]);
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
        let fetcher = BinMockFetcher::new(vec![vec![]]);
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

        let open_times = df.column("open_time").unwrap().i64().unwrap();
        assert_eq!(open_times.get(0), Some(1_704_067_200_000_i64));
        assert_eq!(open_times.get(1), Some(1_704_070_800_000_i64));

        let opens = df.column("open").unwrap().str().unwrap();
        assert_eq!(opens.get(0), Some("42000.00"));

        let trade_counts = df.column("trade_count").unwrap().i64().unwrap();
        assert_eq!(trade_counts.get(0), Some(8_000_i64));
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
        let bars = expected_bars_per_month(2024, Month::January, "1h");
        assert_eq!(bars, Some(744));
    }

    #[test]
    fn test_expected_bars_per_month_1h_feb_leap() {
        let bars = expected_bars_per_month(2024, Month::February, "1h");
        assert_eq!(bars, Some(696));
    }

    #[test]
    fn test_expected_bars_per_month_1d_none() {
        let bars = expected_bars_per_month(2024, Month::January, "1d");
        assert_eq!(bars, None);
    }

    // ── Test 4: should_skip idempotency with pinned manifest SHA ─────────────

    fn write_n_row_parquet(path: &std::path::Path, n: usize) {
        let klines: Vec<Kline> = (0..n)
            .map(|i| Kline {
                open_time: i as i64 * 3_600_000,
                close_time: i as i64 * 3_600_000 + 3_599_999,
                open: "100.00".to_owned(),
                high: "101.00".to_owned(),
                low: "99.00".to_owned(),
                close: "100.50".to_owned(),
                volume: "1.0".to_owned(),
                trade_count: 1,
            })
            .collect();
        write_parquet(&klines, path).expect("write_n_row_parquet");
    }

    #[test]
    fn test_should_skip_short_month_with_matching_pinned_sha_returns_true() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("02.parquet");
        let calendar_expected = 672_usize;
        write_n_row_parquet(&parquet, 671);
        let on_disk_sha = data::revision::file_sha256(&parquet).expect("sha256");
        assert!(should_skip(
            &parquet,
            Some(calendar_expected),
            Some(&on_disk_sha)
        ));
    }

    #[test]
    fn test_should_skip_short_month_with_mismatched_pinned_sha_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("02.parquet");
        let calendar_expected = 672_usize;
        write_n_row_parquet(&parquet, 671);
        let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(!should_skip(
            &parquet,
            Some(calendar_expected),
            Some(wrong_sha)
        ));
    }

    #[test]
    fn test_should_skip_short_month_no_manifest_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("02.parquet");
        let calendar_expected = 672_usize;
        write_n_row_parquet(&parquet, 671);
        assert!(!should_skip(&parquet, Some(calendar_expected), None));
    }

    #[test]
    fn test_should_skip_full_month_skipped_regardless_of_manifest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("01.parquet");
        let calendar_expected = 744_usize;
        write_n_row_parquet(&parquet, calendar_expected);
        assert!(should_skip(&parquet, Some(calendar_expected), None));
        let wrong_sha = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(should_skip(
            &parquet,
            Some(calendar_expected),
            Some(wrong_sha)
        ));
    }

    #[test]
    fn test_should_skip_absent_file_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("missing.parquet");
        assert!(!should_skip(&parquet, Some(744), None));
        assert!(!should_skip(&parquet, Some(744), Some("anysha")));
    }
}
