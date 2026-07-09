//! `fetch_coinbase_klines` — Coinbase Exchange `get-product-candles` →
//! Parquet downloader (ADR-0084 D2.a).
//!
//! Fetches historical OHLCV (candles) from the Coinbase Exchange public REST
//! API and writes per-symbol-month Parquet files that `ReplayFeed` can read
//! directly — a direct mirror of `crates/data/src/bin/fetch_binance_klines.rs`.
//!
//! Core fetch + parse + parquet-write logic lives in
//! `crates/data/src/coinbase_klines.rs`. This bin re-exports those pieces and
//! adds only the CLI glue (`Cli`, month-iteration, skip-idempotency, `main`).
//!
//! # Symbol mapping (ADR-0084 D2.a, the one real seam)
//!
//! `--symbols` takes the canonical on-disk symbol (e.g. `BTCUSDT` — the same
//! `Symbol::new("BTCUSDT")` the rest of the engine uses), mapped via
//! `data::coinbase_product_id_for_symbol` to the Coinbase product-id
//! (`BTC-USD`, a FIXED `-USD` quote) for the REST call. **NOT**
//! `data::coinbase_symbol_map` — that helper (designed for the live-feed's
//! own USDC/USDT/USD symbol space) would return `BTC-USDT` for a `BTCUSDT`
//! input, a thinner, non-blessed Coinbase product (discovered + fixed
//! during T1/T2 unit testing). The parquet is written under
//! `<out>/BTCUSDT/<YEAR>/<MM>.parquet` — NOT `<out>/BTC-USD/...` — so the
//! corpus is consumable by `resolve_bakeoff_bars` / `ReplayFeed` with zero
//! engine change.
//!
//! # Output layout (identical to Binance)
//!
//! ```text
//! <out>/<SYMBOL>/<YEAR>/<MONTH-padded>.parquet
//! ```
//!
//! # Schema (matches `replay_feed.rs`, identical to Binance)
//!
//! ```text
//! open_time   Int64  — Unix millis, bar open
//! close_time  Int64  — Unix millis, bar close
//! open        Utf8   — price string
//! high        Utf8
//! low         Utf8
//! close       Utf8
//! volume      Utf8
//! trade_count Int64  — always 0 (Coinbase candles do not report it)
//! ```

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use polars::prelude::*;
use reqwest::Client;
use time::{Date, Month};
use tracing::{info, warn};

// Re-export the shared types from the library modules.
use data::binance_klines::{date_to_millis, expected_bars_per_month, next_month_start, parse_date};
use data::coinbase_klines::{
    HttpCoinbaseKlineFetcher, coinbase_product_id_for_symbol, paginate_coinbase_candles,
    write_parquet,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "fetch_coinbase_klines",
    about = "Fetch Coinbase Exchange historical candles and write Parquet files"
)]
struct Cli {
    /// Comma-separated symbols in CANONICAL on-disk form, e.g. BTCUSDT
    /// (mapped to the Coinbase product-id BTC-USD — a FIXED `-USD` quote,
    /// via `coinbase_product_id_for_symbol` — for the REST call; the
    /// parquet dir stays `BTCUSDT`).
    #[arg(short = 's', long, value_delimiter = ',')]
    symbols: Vec<String>,

    /// Inclusive start date (YYYY-MM-DD)
    #[arg(long, default_value = "2020-01-01")]
    start: String,

    /// Inclusive end date (YYYY-MM-DD)
    #[arg(long, default_value = "2020-12-31")]
    end: String,

    /// Interval string: only `1h` is supported today (granularity=3600).
    /// Kept as a CLI arg (mirrors the Binance bin's `--interval`) for a
    /// future finer-grained fetch without a breaking CLI change.
    #[arg(long, default_value = "1h")]
    interval: String,

    /// Output root directory
    #[arg(long, default_value = "data/coinbase")]
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

/// Map the CLI `--interval` string to a Coinbase `granularity` in seconds.
///
/// Only `1h` is supported today (ADR-0084 D2.b scope: BTC-USD hourly
/// cross-check). Coinbase's valid granularities are {60,300,900,3600,21600,
/// 86400}; this fetcher intentionally only wires the one P2 needs.
fn interval_to_granularity_secs(interval: &str) -> Result<u64> {
    match interval {
        "1h" => Ok(3600),
        other => Err(anyhow!(
            "unsupported --interval '{other}' — only '1h' is wired for the Coinbase fetcher (ADR-0084 D2.b scope)"
        )),
    }
}

/// Check idempotency: if file exists and is already complete, skip it.
///
/// Mirrors `fetch_binance_klines.rs::should_skip` byte-for-byte (same
/// decision tree — reused as a distinct fn here rather than importing the
/// bin-private helper, since bins cannot import each other's private items;
/// the logic is deliberately identical so the two fetchers' idempotency
/// contracts never silently diverge).
fn should_skip(
    path: &std::path::Path,
    expected_bars: Option<usize>,
    pinned_sha: Option<&str>,
) -> bool {
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
                    info!(
                        path = %path.display(),
                        rows,
                        "file exists with expected row count — skipping"
                    );
                    return true;
                }

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

    let granularity_secs = interval_to_granularity_secs(&cli.interval)?;

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
    let fetcher = HttpCoinbaseKlineFetcher::new(client, granularity_secs);

    // Load the existing REVISION.toml manifest once (if present) — identical
    // idempotency contract to the Binance fetcher.
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
        let symbol_typed = trading_core::Symbol::new(&symbol_upper);
        let product_id = coinbase_product_id_for_symbol(&symbol_typed).with_context(|| {
            format!("map on-disk symbol '{symbol_upper}' to a Coinbase product-id")
        })?;
        info!(symbol = %symbol_upper, product_id = %product_id, "starting Coinbase download");

        let mut year = start_date.year();
        let mut month = start_date.month();
        // A2 (feature.md assumption) — record the earliest month for which
        // the API actually returned data, so the fetch report can state the
        // real earliest-served-candle finding (vs. the assumed ~2015-16).
        let mut earliest_nonempty_month: Option<(i32, u8)> = None;

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
                product_id = %product_id,
                year,
                month = month_num,
                start_ms,
                end_ms,
                "fetching month"
            );

            let klines = paginate_coinbase_candles(
                &fetcher,
                &product_id,
                granularity_secs,
                start_ms,
                end_ms,
                200, // 200ms between requests → ≤5 req/s, well under Coinbase's ~10 req/s
            )
            .await
            .with_context(|| format!("fetch candles for {symbol_upper} {year}/{month_num:02}"))?;

            if klines.is_empty() {
                warn!(
                    symbol = %symbol_upper,
                    year,
                    month = month_num,
                    "API returned 0 candles for this month — skipping parquet write \
                     (pre-listing / no ticks / beyond earliest served candle)"
                );
            } else {
                if earliest_nonempty_month.is_none() {
                    earliest_nonempty_month = Some((year, month_num));
                }
                write_parquet(&klines, &parquet_path).with_context(|| {
                    format!("write parquet for {symbol_upper} {year}/{month_num:02}")
                })?;
                println!(
                    "[OK] {symbol_upper}/{year}/{month_num:02}.parquet  ({} candles)",
                    klines.len()
                );
            }

            if advance_month(&mut year, &mut month, end_date) {
                break;
            }
        }

        if let Some((y, m)) = earliest_nonempty_month {
            println!(
                "[EARLIEST-SERVED] {symbol_upper} ({product_id}): first non-empty month = {y}-{m:02}"
            );
        } else {
            println!(
                "[EARLIEST-SERVED] {symbol_upper} ({product_id}): NO non-empty months in requested window \
                 [{}, {}]",
                cli.start, cli.end
            );
        }

        info!(symbol = %symbol_upper, "finished Coinbase download");
    }

    // Emit REVISION.toml after all fetches complete (identical convention to
    // the Binance fetcher, distinct `fetch_tool` label for provenance).
    if cli.emit_revision_manifest {
        let agg_sha = data::revision::write_revision_manifest_with_tool(
            &cli.out,
            data::revision::RevisionMetadataInput {
                fetch_tool: "fetch_coinbase_klines",
                binance_base: "https://api.exchange.coinbase.com",
                interval: Some(&cli.interval),
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
    use anyhow::Result;
    use data::coinbase_klines::{CoinbaseKlineFetcher, Kline, build_coinbase_candles_url};
    use std::sync::Mutex;

    // Inline mock fetcher for the bin's tests (mirrors
    // `fetch_binance_klines.rs::tests::BinMockFetcher`).
    struct BinMockCoinbaseFetcher {
        batches: Mutex<Vec<Vec<Kline>>>,
        calls: Mutex<Vec<String>>,
    }

    impl BinMockCoinbaseFetcher {
        fn new(batches: Vec<Vec<Kline>>) -> Self {
            Self {
                batches: Mutex::new(batches),
                calls: Mutex::new(Vec::new()),
            }
        }
        fn recorded_calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl CoinbaseKlineFetcher for BinMockCoinbaseFetcher {
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

    // ── Test: --interval mapping ──────────────────────────────────────────────

    #[test]
    fn test_interval_to_granularity_1h() {
        assert_eq!(interval_to_granularity_secs("1h").unwrap(), 3600);
    }

    #[test]
    fn test_interval_to_granularity_unsupported_is_error() {
        assert!(interval_to_granularity_secs("4h").is_err());
        assert!(interval_to_granularity_secs("1d").is_err());
    }

    // ── Test: symbol → product_id mapping (the on-disk-BTCUSDT seam) ────────
    //
    // Uses `coinbase_product_id_for_symbol`, NOT `data::coinbase_symbol_map`
    // — the latter would return "BTC-USDT" for a "BTCUSDT" input (discovered
    // + fixed during T1/T2 unit testing; see coinbase_klines.rs module doc).

    #[test]
    fn test_symbol_to_product_id_mapping() {
        let sym = trading_core::Symbol::new("BTCUSDT");
        assert_eq!(
            coinbase_product_id_for_symbol(&sym).expect("valid symbol"),
            "BTC-USD"
        );
        let sym = trading_core::Symbol::new("ETHUSDT");
        assert_eq!(
            coinbase_product_id_for_symbol(&sym).expect("valid symbol"),
            "ETH-USD"
        );
    }

    // ── Test: should_skip idempotency (mirrors the Binance bin's tests) ──────

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
                trade_count: 0,
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
    fn test_should_skip_absent_file_returns_false() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("missing.parquet");
        assert!(!should_skip(&parquet, Some(744), None));
    }

    #[test]
    fn test_should_skip_full_month_skipped_regardless_of_manifest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let parquet = tmp.path().join("01.parquet");
        let calendar_expected = 744_usize;
        write_n_row_parquet(&parquet, calendar_expected);
        assert!(should_skip(&parquet, Some(calendar_expected), None));
    }

    // ── Test: month-window pagination integration (mock, no socket) ─────────

    #[tokio::test]
    async fn test_paginate_and_url_shape_for_one_month() {
        // Confirm the URL the bin's fetch loop would produce contains the
        // correct product_id + granularity for a BTCUSDT symbol.
        let sym = trading_core::Symbol::new("BTCUSDT");
        let product_id = coinbase_product_id_for_symbol(&sym).expect("valid symbol");
        let url = build_coinbase_candles_url(&product_id, 3600, "2024-01-01T00:00:00Z", "x");
        assert!(url.contains("BTC-USD"));
        assert!(
            !url.contains("BTC-USDT"),
            "must not hit the thin BTC-USDT product"
        );
        assert!(url.contains("granularity=3600"));

        let fetcher = BinMockCoinbaseFetcher::new(vec![vec![]]);
        let result = data::coinbase_klines::paginate_coinbase_candles(
            &fetcher,
            &product_id,
            3600,
            0,
            3_600_000,
            0,
        )
        .await
        .expect("paginate ok");
        assert!(result.is_empty());
        assert_eq!(fetcher.recorded_calls().len(), 1);
    }
}
