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
fn newest_parquet_mtime(ticker_root: &Path) -> Option<SystemTime> {
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
}
