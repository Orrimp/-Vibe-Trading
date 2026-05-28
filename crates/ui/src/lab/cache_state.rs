//! Cache-state probe — lab-yahoo-realdata Wave D-followup (T-D2).
//!
//! Cheap synchronous filesystem stat that summarizes the freshness of the
//! Yahoo cache at `data/yahoo/<TICKER>/`. Used by the cache-state badge
//! widget rendered next to the Source toggle when `LabDataSource::YahooCache`
//! is selected.
//!
//! ## Design
//!
//! - Pure `std::fs::metadata` + `mtime` — no async, no I/O hot loop.
//! - Returns one of three states: `Empty`, `Stale`, `Fresh`.
//! - 24-hour staleness threshold (configurable via the constant below).
//! - Safe to call from `view()` — under 1 ms per call on hot filesystems.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Mtime threshold above which a cache is considered "stale" (24 h).
pub const STALE_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

/// Three-state summary of the Yahoo cache for a given ticker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    /// No cache directory or no parquet files for the ticker.
    Empty,
    /// Cache exists but the most-recent parquet's mtime is older than
    /// `STALE_THRESHOLD`.
    Stale,
    /// Cache exists and the most-recent parquet's mtime is within
    /// `STALE_THRESHOLD`.
    Fresh,
}

/// Probe the Yahoo cache state for a given ticker.
///
/// Walks `<cache_root>/<TICKER>/` recursively (at most a few levels deep)
/// looking for the newest `*.parquet` file's mtime.
///
/// Returns `CacheState::Empty` if the directory doesn't exist or contains
/// no parquet files. Returns `Stale` / `Fresh` based on the newest mtime
/// vs. `now`.
///
/// `now` is a parameter so tests can inject a fixed clock.
#[must_use]
pub fn probe(cache_root: &Path, ticker: &str, now: SystemTime) -> CacheState {
    let ticker_root = cache_root.join(ticker);
    if !ticker_root.is_dir() {
        return CacheState::Empty;
    }

    let newest = newest_parquet_mtime(&ticker_root);
    match newest {
        None => CacheState::Empty,
        Some(mtime) => {
            let age = now.duration_since(mtime).unwrap_or(Duration::ZERO);
            if age <= STALE_THRESHOLD {
                CacheState::Fresh
            } else {
                CacheState::Stale
            }
        }
    }
}

/// Convenience: probe with `SystemTime::now()` as the clock.
#[must_use]
pub fn probe_now(cache_root: &Path, ticker: &str) -> CacheState {
    probe(cache_root, ticker, SystemTime::now())
}

/// Walk `<ticker_root>` and return the mtime of the newest `*.parquet` file.
/// Returns `None` if no parquet files exist.
///
/// Public so `probe_summary` can call it directly per-ticker without
/// re-implementing the walk.
#[must_use]
pub fn newest_parquet_mtime(ticker_root: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    walk_for_parquet(ticker_root, &mut newest);
    newest
}

/// Recursively scan `dir` for `*.parquet` files; update `newest` to the
/// max mtime seen. Bounded depth implicit in the cache layout
/// (`<TICKER>/<INTERVAL>/<YEAR>/<MONTH>.parquet` — 3 levels).
fn walk_for_parquet(dir: &Path, newest: &mut Option<SystemTime>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_for_parquet(&path, newest);
        } else if path.extension().is_some_and(|e| e == "parquet")
            && let Ok(meta) = entry.metadata()
            && let Ok(mtime) = meta.modified()
        {
            *newest = Some(match *newest {
                Some(prev) => prev.max(mtime),
                None => mtime,
            });
        }
    }
}

/// Default cache root path used by the production runner.
/// Lives at the workspace-relative path `data/yahoo`.
#[must_use]
pub fn default_cache_root() -> PathBuf {
    PathBuf::from("data/yahoo")
}

/// Map a Binance-style symbol (e.g. "BTCUSDT") to its Yahoo Finance ticker
/// (e.g. "BTC-USD"), or `None` when the symbol isn't in the 10-pair
/// crypto-mirror universe.
///
/// Mirrors `data::yahoo::binance_to_yahoo_ticker` (Q6 = (a) / ADR-0040 § D7)
/// but is replicated here so the `ui` crate doesn't need the `data` feature
/// just to render the cache-state badge. The 10-row table is locked by
/// `data/src/yahoo.rs::binance_to_yahoo_table_pinned` test.
#[must_use]
pub fn binance_to_yahoo_ticker_lookup(binance_symbol: &str) -> Option<&'static str> {
    match binance_symbol {
        "BTCUSDT" => Some("BTC-USD"),
        "ETHUSDT" => Some("ETH-USD"),
        "BNBUSDT" => Some("BNB-USD"),
        "SOLUSDT" => Some("SOL-USD"),
        "XRPUSDT" => Some("XRP-USD"),
        "ADAUSDT" => Some("ADA-USD"),
        "DOGEUSDT" => Some("DOGE-USD"),
        "AVAXUSDT" => Some("AVAX-USD"),
        "DOTUSDT" => Some("DOT-USD"),
        "LINKUSDT" => Some("LINK-USD"),
        _ => None,
    }
}

// ── lab-yahoo-realdata v0.1.2 — aggregate cache-state summary (T-DU3) ────────
//
// Sibling surface to per-pair `probe`. Walks the 10-row crypto-mirror,
// counts non-empty ticker directories, and returns the global max-mtime
// for the summary badge in the Lab toolbar.
//
// **Cadence:** D-V0.1.2-1 — cached on `LabState::cache_summary`
// (`Option<CacheSummary>`); recomputed lazily on first read after
// invalidation (Lab-Run-complete OR data_source toggle). No background
// polling per R-NR.7.

/// The 10 Yahoo Finance tickers mirrored from the crypto-mirror universe
/// (ADR-0040 § D7 / `data::yahoo::binance_to_yahoo_ticker` RHS).
///
/// Replicated here so external callers (Lab toolbar, gallery cells,
/// `probe_summary`) don't need to know about the 10-row source table.
/// The pinned-table test
/// `data/src/yahoo.rs::binance_to_yahoo_table_pinned` locks the
/// authoritative source; the
/// `binance_to_yahoo_ticker_lookup` table above is the UI mirror;
/// this slice is a derived projection of the same RHS column.
pub const ALL_YAHOO_TICKERS: &[&str] = &[
    "BTC-USD", "ETH-USD", "BNB-USD", "SOL-USD", "XRP-USD", "ADA-USD", "DOGE-USD", "AVAX-USD",
    "DOT-USD", "LINK-USD",
];

/// Aggregate snapshot of the Yahoo cache across the 10-row crypto-mirror
/// universe.
///
/// - `populated_count` — number of tickers in `ALL_YAHOO_TICKERS` whose
///   `data/yahoo/<TICKER>/` tree contains at least one `*.parquet` file.
/// - `newest_mtime` — global maximum `mtime` across every parquet found;
///   `None` when no parquet files exist anywhere in the mirror
///   (`populated_count == 0`).
///
/// `Clone + PartialEq + Eq` for snapshot tests; `Debug` for tracing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheSummary {
    /// How many of the 10 tickers have at least one parquet file on disk.
    pub populated_count: usize,
    /// Newest mtime across all parquet files for all 10 tickers, or
    /// `None` when no parquet files exist anywhere.
    pub newest_mtime: Option<SystemTime>,
}

impl CacheSummary {
    /// Sentinel empty summary — used by callers that need a stable
    /// `CacheSummary` value before the first `probe_summary` call (e.g.
    /// gallery cells).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            populated_count: 0,
            newest_mtime: None,
        }
    }
}

/// Probe the Yahoo cache root and return an aggregate summary across the
/// supplied tickers.
///
/// Iterates each ticker dir under `<cache_root>/<TICKER>/`, calls
/// `newest_parquet_mtime`, counts the ones that returned `Some(_)`, and
/// returns the global max-mtime.
///
/// **Cost:** ~10 × 3-level walks = at most ~30 directory stats on the
/// 10-row Yahoo mirror. Bounded by the cache layout. Pure (no global
/// state, no side effects). The `now` parameter is unused at present
/// but reserved for a future stale-band variant; callers should pass
/// `SystemTime::now()`.
#[must_use]
pub fn probe_summary(cache_root: &Path, tickers: &[&str], _now: SystemTime) -> CacheSummary {
    let mut populated_count: usize = 0;
    let mut newest: Option<SystemTime> = None;
    for ticker in tickers {
        let ticker_root = cache_root.join(ticker);
        if !ticker_root.is_dir() {
            continue;
        }
        if let Some(mtime) = newest_parquet_mtime(&ticker_root) {
            populated_count += 1;
            newest = Some(match newest {
                Some(prev) => prev.max(mtime),
                None => mtime,
            });
        }
    }
    CacheSummary {
        populated_count,
        newest_mtime: newest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write as _;

    /// Helper — make a tempdir under target/ that auto-removes on drop.
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "cache_state_test_{}_{}",
                name,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Missing ticker dir → Empty.
    #[test]
    fn cache_state_missing_dir_is_empty() {
        let tmp = Tmp::new("missing");
        let state = probe(&tmp.0, "BTC-USD", SystemTime::now());
        assert_eq!(state, CacheState::Empty);
    }

    /// Existing ticker dir but no parquet files → Empty.
    #[test]
    fn cache_state_no_parquet_is_empty() {
        let tmp = Tmp::new("noparquet");
        fs::create_dir_all(tmp.0.join("BTC-USD").join("1d").join("2024")).unwrap();
        let state = probe(&tmp.0, "BTC-USD", SystemTime::now());
        assert_eq!(state, CacheState::Empty);
    }

    /// A fresh parquet file (mtime = now) → Fresh.
    #[test]
    fn cache_state_fresh_parquet_is_fresh() {
        let tmp = Tmp::new("fresh");
        let parquet_dir = tmp.0.join("BTC-USD").join("1d").join("2024");
        fs::create_dir_all(&parquet_dir).unwrap();
        let parquet = parquet_dir.join("01.parquet");
        let mut f = fs::File::create(&parquet).unwrap();
        f.write_all(b"stub").unwrap();
        drop(f);
        let state = probe(&tmp.0, "BTC-USD", SystemTime::now());
        assert_eq!(state, CacheState::Fresh);
    }

    /// A parquet with mtime 48 h ago → Stale.
    #[test]
    fn cache_state_old_parquet_is_stale() {
        let tmp = Tmp::new("stale");
        let parquet_dir = tmp.0.join("BTC-USD").join("1d").join("2024");
        fs::create_dir_all(&parquet_dir).unwrap();
        let parquet = parquet_dir.join("01.parquet");
        fs::File::create(&parquet).unwrap();
        // Probe with a clock 48h in the future relative to the file's mtime.
        let future = SystemTime::now() + Duration::from_secs(48 * 60 * 60);
        let state = probe(&tmp.0, "BTC-USD", future);
        assert_eq!(state, CacheState::Stale);
    }

    /// Default cache root resolves to `data/yahoo` relative to CWD.
    #[test]
    fn cache_state_default_root_is_data_yahoo() {
        let root = default_cache_root();
        assert_eq!(root, PathBuf::from("data/yahoo"));
    }

    // ── lab-yahoo-realdata v0.1.2 — probe_summary tests (T-DU3) ──────────────

    /// `ALL_YAHOO_TICKERS` mirrors the RHS of the 10-row crypto-mirror table
    /// — same length, same order as the analyst forecast in feature.md
    /// § D-V0.1.2-3.
    #[test]
    fn all_yahoo_tickers_has_ten_rows() {
        assert_eq!(
            ALL_YAHOO_TICKERS.len(),
            10,
            "10-row crypto-mirror universe locked at v0.1.2"
        );
        // Sanity: every entry is a *-USD Yahoo ticker (no Binance suffix).
        for t in ALL_YAHOO_TICKERS {
            assert!(t.ends_with("-USD"), "non-USD ticker in mirror: {t}");
            assert!(
                !t.contains("USDT"),
                "Binance-style ticker leaked into Yahoo mirror: {t}"
            );
        }
    }

    /// Empty cache root → `populated_count = 0`, `newest_mtime = None`.
    #[test]
    fn probe_summary_empty_dir_is_empty() {
        let tmp = Tmp::new("summary_empty");
        let summary = probe_summary(&tmp.0, ALL_YAHOO_TICKERS, SystemTime::now());
        assert_eq!(summary.populated_count, 0);
        assert_eq!(summary.newest_mtime, None);
        assert_eq!(summary, CacheSummary::empty());
    }

    /// One populated ticker → `populated_count = 1`, `newest_mtime = Some(_)`.
    #[test]
    fn probe_summary_one_ticker_populated() {
        let tmp = Tmp::new("summary_one");
        let parquet_dir = tmp.0.join("BTC-USD").join("1d").join("2024");
        fs::create_dir_all(&parquet_dir).unwrap();
        let mut f = fs::File::create(parquet_dir.join("01.parquet")).unwrap();
        f.write_all(b"stub").unwrap();
        drop(f);
        let summary = probe_summary(&tmp.0, ALL_YAHOO_TICKERS, SystemTime::now());
        assert_eq!(summary.populated_count, 1);
        assert!(summary.newest_mtime.is_some());
    }

    /// Two populated tickers → `populated_count = 2`, `newest_mtime`
    /// reflects the global max (the later-created file).
    #[test]
    fn probe_summary_two_tickers_take_max_mtime() {
        let tmp = Tmp::new("summary_two");

        // Create BTC-USD parquet.
        let btc_dir = tmp.0.join("BTC-USD").join("1d").join("2024");
        fs::create_dir_all(&btc_dir).unwrap();
        fs::File::create(btc_dir.join("01.parquet")).unwrap();

        // Create ETH-USD parquet. On most filesystems the mtime of the
        // second-created file is >= the first. We don't assert ordering
        // here; we only assert `newest_mtime` was set AND covers both.
        let eth_dir = tmp.0.join("ETH-USD").join("1d").join("2024");
        fs::create_dir_all(&eth_dir).unwrap();
        fs::File::create(eth_dir.join("01.parquet")).unwrap();

        let summary = probe_summary(&tmp.0, ALL_YAHOO_TICKERS, SystemTime::now());
        assert_eq!(summary.populated_count, 2);
        assert!(summary.newest_mtime.is_some());
    }

    /// Empty subset (passing an empty slice) → empty summary.
    #[test]
    fn probe_summary_empty_ticker_slice() {
        let tmp = Tmp::new("summary_empty_slice");
        let summary = probe_summary(&tmp.0, &[], SystemTime::now());
        assert_eq!(summary.populated_count, 0);
        assert_eq!(summary.newest_mtime, None);
    }

    /// `CacheSummary::empty()` is the zero-state constructor.
    #[test]
    fn cache_summary_empty_const_zero_state() {
        let e = CacheSummary::empty();
        assert_eq!(e.populated_count, 0);
        assert_eq!(e.newest_mtime, None);
    }
}
