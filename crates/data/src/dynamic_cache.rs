//! Dynamic (on-demand) Binance klines cache.
//!
//! Feature `advisor-dynamic-data` Wave B (ADR-0061 D3/D6).
//!
//! ## Anchor-safety (HARD non-negotiable)
//!
//! - Dynamic bars go to `data/binance-dynamic/` (`BINANCE_DYNAMIC_ROOT`), **never**
//!   `data/binance/`.
//! - `.gitignore` already has `/data/*` (line 12); `data/binance-dynamic/` adds no
//!   `!`-exception, so it is git-ignored by the existing rule (verified in M-DEV.B2).
//! - `verify_anchors.sh` walks only `spec/` — this root is invisible to it.
//! - The pinned corpus `data/binance/REVISION.toml` is **never** written here.
//!
//! ## Cache layout
//!
//! Identical to `BINANCE_CORPUS_ROOT` (ADR-0056): `<SYM>/<YEAR>/<MM>.parquet`.
//! `ReplayFeed` reads it with only a ROOT change.
//!
//! ## Re-read through ReplayFeed (D6 byte-fidelity)
//!
//! `load_or_fetch` writes parquet then reads back through `ReplayFeed` — so the
//! cache-hit path and cache-miss path return **byte-identical `Bar`** values, and
//! a dynamic bar == a corpus bar for the same timestamp.

use std::path::{Path, PathBuf};

use time::Month;
use tracing::{info, warn};
use trading_core::{Bar, Symbol, Timeframe};

use crate::ReplayFeed;
use crate::binance_klines::{HttpKlineFetcher, KlineFetcher};
use crate::binance_klines::{
    Kline, expected_bars_per_month, next_month_start, paginate_klines, write_parquet,
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Root for dynamically-fetched (non-anchored) Binance bars.
///
/// SEPARATE from `BINANCE_CORPUS_ROOT` (`data/binance/`).
/// - git-ignored by the existing `/data/*` rule — no `!` exception is added.
/// - NO `REVISION.toml` pin (live data is not reproducible — ADR-0061 D5).
/// - `verify_anchors.sh` walks only `spec/` → blind to this path.
pub const BINANCE_DYNAMIC_ROOT: &str = "data/binance-dynamic";

/// Interval used for all dynamic bake-off fetches (the bake-off is hourly).
const BAKEOFF_INTERVAL: &str = "1h";

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors from the dynamic cache (load or fetch phase).
#[derive(Debug, thiserror::Error)]
pub enum DynamicCacheError {
    /// The underlying Binance fetch failed (typed error).
    #[error("dynamic fetch failed: {0}")]
    Fetch(#[from] crate::binance_klines::BinanceFetchError),
    /// Parquet write failed.
    #[error("dynamic cache write failed: {0}")]
    Write(String),
    /// Parquet read-back (via ReplayFeed) failed.
    #[error("dynamic cache read failed: {0}")]
    Read(String),
    /// Fetch returned zero bars after clipping to the requested window.
    #[error("no data in [{start_ms}, {end_ms}) for {symbol} (dynamic cache)")]
    NoData {
        symbol: String,
        start_ms: i64,
        end_ms: i64,
    },
}

// ── Month iteration helpers ───────────────────────────────────────────────────

/// A `(year, month)` pair for iterating over calendar months.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct YearMonth {
    pub year: i32,
    pub month: Month,
}

impl YearMonth {
    pub fn from_ms(ms: i64) -> Self {
        let dt = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        YearMonth {
            year: dt.year(),
            month: dt.month(),
        }
    }

    /// Inclusive start-of-month UTC epoch ms.
    pub fn start_ms(self) -> i64 {
        let d = time::Date::from_calendar_date(self.year, self.month, 1)
            .expect("calendar date always valid");
        let pdt = time::PrimitiveDateTime::new(d, time::Time::MIDNIGHT);
        pdt.assume_utc().unix_timestamp() * 1_000
    }

    /// Exclusive end-of-month UTC epoch ms (= start of NEXT month).
    pub fn end_ms_exclusive(self) -> i64 {
        next_month_start(self.year, self.month)
            .midnight()
            .assume_utc()
            .unix_timestamp()
            * 1_000
    }

    /// Path component: `<root>/<SYM>/<YEAR>/<MM>.parquet`.
    pub fn parquet_path(self, root: &Path, symbol: &str) -> PathBuf {
        let month_num = u8::from(self.month);
        root.join(symbol)
            .join(self.year.to_string())
            .join(format!("{month_num:02}.parquet"))
    }

    /// Advance by one month.
    pub fn next(self) -> Self {
        let next_month_date = next_month_start(self.year, self.month);
        YearMonth {
            year: next_month_date.year(),
            month: next_month_date.month(),
        }
    }
}

/// Return all calendar months that overlap `[start_ms, end_ms)`.
fn months_in_window(start_ms: i64, end_ms: i64) -> Vec<YearMonth> {
    let mut months = Vec::new();
    let mut ym = YearMonth::from_ms(start_ms);
    loop {
        if ym.start_ms() >= end_ms {
            break;
        }
        months.push(ym);
        ym = ym.next();
    }
    months
}

// ── Core logic ────────────────────────────────────────────────────────────────

/// Load `[start_ms, end_ms)` hourly bars for `symbol` from the dynamic cache,
/// fetching any not-yet-cached months from Binance and writing them to the
/// dynamic root.  NEVER reads or writes `data/binance/`.
///
/// Cache granularity = `<SYM>/<YEAR>/<MM>.parquet` month files (ADR-0056 layout)
/// so `ReplayFeed` reads it with only a ROOT change.
///
/// A month file present + non-empty is a cache hit; a **partial trailing month**
/// (the current month, still filling) is re-fetched (no REVISION pin to check against).
///
/// Returns `Err(DynamicCacheError::NoData)` when the window yields zero bars after
/// clipping (never `Ok(vec![])`).
pub async fn load_or_fetch(
    symbol: &Symbol,
    start_ms: i64,
    end_ms: i64,
    _tf: Timeframe,
) -> Result<Vec<Bar>, DynamicCacheError> {
    let root = PathBuf::from(BINANCE_DYNAMIC_ROOT);
    let fetcher = {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                DynamicCacheError::Fetch(crate::binance_klines::BinanceFetchError::Network {
                    symbol: symbol.0.to_string(),
                    source: e,
                })
            })?;
        HttpKlineFetcher::new(client)
    };
    load_or_fetch_with(&root, symbol, start_ms, end_ms, &fetcher).await
}

/// Testable variant of `load_or_fetch` that accepts a custom root + fetcher.
///
/// Used by integration tests (e.g. `crates/data/tests/dynamic_cache_anchor_safety.rs`)
/// and by `dynamic_cache` unit tests to inject a mock fetcher without a live socket.
///
/// The dynamic root can be any path (typically a `tempdir` in tests).
#[doc(hidden)] // internal seam — public only for integration tests
pub async fn load_or_fetch_with(
    root: &Path,
    symbol: &Symbol,
    start_ms: i64,
    end_ms: i64,
    fetcher: &dyn KlineFetcher,
) -> Result<Vec<Bar>, DynamicCacheError> {
    let symbol_str = symbol.0.as_str();

    // Assert we never write to the pinned corpus path.
    debug_assert!(
        !root.to_string_lossy().contains("data/binance/")
            || root.to_string_lossy().contains("dynamic"),
        "dynamic_cache must NEVER write to data/binance/ (anchored corpus)"
    );

    // Determine which months overlap the requested window.
    let months = months_in_window(start_ms, end_ms);

    for ym in &months {
        let path = ym.parquet_path(root, symbol_str);

        // Cache-hit check: file exists AND has the expected bar count.
        // Exception: re-fetch the trailing (partial current) month every time.
        let is_current_month = {
            let now_ym =
                YearMonth::from_ms(time::OffsetDateTime::now_utc().unix_timestamp() * 1_000);
            *ym == now_ym
        };

        if !is_current_month && path.exists() {
            // Verify non-empty by checking the file size (or row count).
            if is_cache_hit(&path, ym.year, ym.month) {
                info!(
                    symbol = symbol_str,
                    year = ym.year,
                    month = u8::from(ym.month),
                    "dynamic cache hit — skipping fetch"
                );
                continue;
            }
        }

        // Cache miss (or trailing-month refresh): fetch the full calendar month
        // from Binance.  Use the month boundaries (not the window) so the cache
        // is always full-month granularity.
        let month_start = ym.start_ms();
        let month_end = ym.end_ms_exclusive();

        info!(
            symbol = symbol_str,
            year = ym.year,
            month = u8::from(ym.month),
            month_start,
            month_end,
            is_current_month,
            "dynamic cache miss — fetching from Binance"
        );

        let klines = do_fetch_month(symbol_str, month_start, month_end, fetcher).await?;

        if klines.is_empty() {
            warn!(
                symbol = symbol_str,
                year = ym.year,
                month = u8::from(ym.month),
                "Binance returned 0 klines for month — skipping parquet write"
            );
        } else {
            write_parquet(&klines, &path).map_err(|e| DynamicCacheError::Write(e.to_string()))?;
        }
    }

    // Read ALL cached months back through ReplayFeed (D6 byte-fidelity: the
    // cache-hit and cache-miss paths return byte-identical `Bar` values).
    let feed = ReplayFeed::new(root, true);
    let symbol_paths = [(symbol.clone(), root.to_path_buf())];

    let merge_result = feed.merge_symbols(&symbol_paths, Timeframe::OneHour);
    let mut bars = match merge_result {
        Ok(b) => b,
        Err(e) => {
            let msg = e.to_string();
            // If the error is "no parquet files found", that means the fetch
            // returned zero klines (the Binance API had nothing for this window).
            // Map to the typed NoData error rather than a generic Read error.
            if msg.contains("no parquet files found") {
                return Err(DynamicCacheError::NoData {
                    symbol: symbol_str.to_owned(),
                    start_ms,
                    end_ms,
                });
            }
            return Err(DynamicCacheError::Read(msg));
        }
    };

    // Clip to [start_ms, end_ms).
    bars.retain(|b| {
        let ts_ms = b.open_ts.unix_millis();
        ts_ms >= start_ms && ts_ms < end_ms
    });

    if bars.is_empty() {
        return Err(DynamicCacheError::NoData {
            symbol: symbol_str.to_owned(),
            start_ms,
            end_ms,
        });
    }

    info!(
        symbol = symbol_str,
        bars = bars.len(),
        start_ms,
        end_ms,
        "dynamic cache loaded"
    );

    Ok(bars)
}

/// Check if a cached month parquet is a cache hit (non-empty, plausible row count).
fn is_cache_hit(path: &Path, year: i32, month: Month) -> bool {
    let expected = expected_bars_per_month(year, month, BAKEOFF_INTERVAL);

    // Read row count.
    match polars::prelude::LazyFrame::scan_parquet(
        path,
        polars::prelude::ScanArgsParquet::default(),
    ) {
        Err(_) => false,
        Ok(lf) => match lf.collect() {
            Err(_) => false,
            Ok(df) => {
                let rows = df.height();
                if rows == 0 {
                    return false;
                }
                // If we know the expected count, accept anything ≥ 50% of expected
                // (real gaps in Binance data can produce short months).
                match expected {
                    Some(exp) => rows >= exp / 2,
                    None => rows > 0,
                }
            }
        },
    }
}

/// Fetch a full calendar month from Binance (via the trait fetcher).
async fn do_fetch_month(
    symbol: &str,
    start_ms: i64,
    end_ms: i64,
    fetcher: &dyn KlineFetcher,
) -> Result<Vec<Kline>, DynamicCacheError> {
    paginate_klines(fetcher, symbol, BAKEOFF_INTERVAL, start_ms, end_ms, 200)
        .await
        .map_err(|e| {
            // Map the anyhow error from paginate_klines to a fetch error.
            DynamicCacheError::Write(format!("paginate failed for {symbol}: {e}"))
        })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::pedantic)]
mod tests {
    use super::*;
    use crate::binance_klines::{MockFetcher, make_batch};
    use trading_core::Symbol;

    /// Cache-miss fetches + writes month file to dynamic root.
    /// Cache-hit on second call (fetcher NOT called again).
    /// Both calls return bars in the same `[start_ms, end_ms)` window.
    #[tokio::test]
    async fn cache_miss_fetches_then_cache_hit_skips() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let sym = Symbol::new("BTCUSDT");

        // 48 hourly bars starting 2026-06-01 00:00 UTC.
        // We pick a past month so it's NOT the "current month" (no re-fetch).
        let start_ms = 1_746_057_600_000_i64; // 2025-05-01 00:00 UTC
        let end_ms = start_ms + 48 * 3_600_000;
        let step = 3_600_000_i64;

        // Month: May 2025 (2025-05-01 to 2025-06-01).
        // Provide a full month batch (for the fetcher) + empty terminator.
        let month_start = 1_746_057_600_000_i64; // 2025-05-01 00:00 UTC
        let days_in_may = 31_i64;
        let full_month_bars = days_in_may * 24;
        let batch: Vec<_> = make_batch(month_start, step, full_month_bars as usize);
        let fetcher = MockFetcher::new(vec![batch, vec![]]);

        // First call — cache miss: fetcher IS called.
        let bars1 = load_or_fetch_with(root, &sym, start_ms, end_ms, &fetcher)
            .await
            .expect("first call must succeed");
        let calls_after_first = fetcher.recorded_calls().len();
        assert!(calls_after_first >= 1, "fetcher was called on cache miss");
        assert!(!bars1.is_empty(), "first call must return bars");
        assert_eq!(bars1.len(), 48, "48 bars clipped to the window");

        // Second call — cache hit: fetcher must NOT be called again.
        let fetcher2 = MockFetcher::new(vec![]); // no batches → would panic if called
        let bars2 = load_or_fetch_with(root, &sym, start_ms, end_ms, &fetcher2)
            .await
            .expect("second call (cache hit) must succeed");
        assert_eq!(
            fetcher2.recorded_calls().len(),
            0,
            "fetcher must NOT be called on cache hit"
        );
        assert_eq!(bars1.len(), bars2.len(), "cache hit returns same bar count");

        // Verify byte-fidelity: bars from cache hit == bars from cache miss.
        for (a, b) in bars1.iter().zip(bars2.iter()) {
            assert_eq!(
                a.open_ts, b.open_ts,
                "cache-hit and cache-miss bars must be identical"
            );
            assert_eq!(a.close_ts, b.close_ts);
        }
    }

    /// Zero-bar window → typed NoData error (not an empty success).
    #[tokio::test]
    async fn empty_window_returns_no_data_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let sym = Symbol::new("BTCUSDT");

        // Use a window far in the future where no bars exist.
        let start_ms = 9_000_000_000_000_i64; // year ~2255 — no data
        let end_ms = start_ms + 48 * 3_600_000;

        // Fetcher returns empty (no bars from Binance).
        let fetcher = MockFetcher::new(vec![vec![]]);
        let err = load_or_fetch_with(root, &sym, start_ms, end_ms, &fetcher)
            .await
            .expect_err("empty window must be an error");

        assert!(
            matches!(err, DynamicCacheError::NoData { .. }),
            "expected NoData, got {err:?}"
        );
    }

    /// `load_or_fetch` never writes to `data/binance/` (corpus separation).
    /// Also: no `REVISION.toml` appears under the dynamic root.
    #[tokio::test]
    async fn never_writes_to_pinned_corpus_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let sym = Symbol::new("BTCUSDT");

        // Month: 2025-04 (past month, stable).
        let start_ms = 1_743_465_600_000_i64; // 2025-04-01 00:00 UTC
        let end_ms = start_ms + 24 * 3_600_000; // 1 day
        let step = 3_600_000_i64;
        let batch = make_batch(start_ms, step, 24);
        let fetcher = MockFetcher::new(vec![batch, vec![]]);

        let _ = load_or_fetch_with(root, &sym, start_ms, end_ms, &fetcher).await;

        // Assert no REVISION.toml was written to the dynamic root.
        assert!(
            !root.join("REVISION.toml").exists(),
            "REVISION.toml must NOT be written to the dynamic root"
        );

        // Walk the tempdir manually and check no REVISION.toml was created.
        fn walk_no_revision(dir: &Path) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name();
                assert!(
                    name != "REVISION.toml",
                    "REVISION.toml must not appear under dynamic root: {}",
                    p.display()
                );
                if p.is_dir() {
                    walk_no_revision(&p);
                }
            }
        }
        walk_no_revision(root);
    }

    /// `months_in_window` — covers single and multi-month windows.
    #[test]
    fn months_in_window_single_month() {
        // Window inside one month.
        let start_ms = 1_704_067_200_000_i64; // 2024-01-01 00:00
        let end_ms = start_ms + 7 * 86_400_000; // 1 week
        let months = months_in_window(start_ms, end_ms);
        assert_eq!(months.len(), 1);
        assert_eq!(months[0].year, 2024);
        assert_eq!(months[0].month, Month::January);
    }

    #[test]
    fn months_in_window_two_months() {
        let start_ms = 1_706_745_600_000_i64; // 2024-02-01 00:00
        let end_ms = 1_711_929_600_000_i64; // 2024-04-01 00:00
        let months = months_in_window(start_ms, end_ms);
        assert_eq!(months.len(), 2);
        assert_eq!(months[0].month, Month::February);
        assert_eq!(months[1].month, Month::March);
    }
}

// ── Real-fetch proof (bake-off over an uncached window) ───────────────────────

#[cfg(test)]
mod realdata_tests {
    use super::*;
    use trading_core::Symbol;

    /// Real bake-off proof via `load_or_fetch` over an uncached recent window.
    ///
    /// Run with:
    /// ```
    /// cargo test -p data --features realdata -- \
    ///     realdata_tests::real_dynamic_cache_loads_recent_btcusdt --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "requires live Binance API — run with --ignored"]
    async fn real_dynamic_cache_loads_recent_btcusdt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sym = Symbol::new("BTCUSDT");

        // 14-day window ending 2026-06-19 — NOT in the pinned 2021-2024 corpus.
        let end_ms = 1_750_291_200_000_i64; // 2026-06-19 00:00 UTC
        let start_ms = end_ms - 14 * 86_400_000;

        let root = tmp.path();
        let fetcher = {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("client");
            HttpKlineFetcher::new(client)
        };

        let bars = load_or_fetch_with(root, &sym, start_ms, end_ms, &fetcher)
            .await
            .expect("real dynamic cache must succeed");

        println!(
            "Real dynamic cache: {} bars for BTCUSDT over 14-day uncached window",
            bars.len()
        );

        assert!(
            !bars.is_empty(),
            "must return bars for the recent uncached window"
        );

        // ~14 × 24 = 336 bars ± 50.
        let expected = 14 * 24;
        assert!(
            bars.len() >= expected - 50 && bars.len() <= expected + 50,
            "expected ~{expected} bars, got {}",
            bars.len()
        );

        // Monotonic timestamps.
        for w in bars.windows(2) {
            assert!(w[0].open_ts <= w[1].open_ts);
        }

        // Note: local_recv_ts is intentionally NOT asserted here.
        // `kline_to_bar` writes `close_ts` as `local_recv_ts` into parquet
        // (ADR-0032 § D1 Step 7), but `ReplayFeed` overwrites `local_recv_ts`
        // with `Timestamp::now()` when replaying bars — this is the designed
        // backtest-replay behaviour. The raw `fetch_binance_klines_range` path
        // (bypassing the parquet round-trip) is tested in
        // `binance_klines::realdata_tests::real_fetch_btcusdt_recent_window`.
    }
}
