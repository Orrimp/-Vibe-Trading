//! Mark-to-market source (R11.1, R4.4).
//!
//! Provides close prices for symbols at arbitrary timestamps + over
//! ranges.  Two implementations:
//!
//! - [`ParquetMarkSource`] — reads
//!   `<root>/<SYMBOL>/<YEAR>/*.parquet` via Polars `LazyFrame`.  LRU
//!   caches the last 4096 `(symbol, ts)` lookups.
//! - [`FrozenMarkSource`] — loads a checked-in CSV for deterministic
//!   tests.  Used by every `crates/reports/tests/` integration test
//!   that needs marks without parquet I/O.
//!
//! Both implementations return [`MarkError::OutOfRange`] when the
//! caller asks for a `(symbol, ts)` outside the loaded range.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use polars::prelude::*;
use rust_decimal::Decimal;
use thiserror::Error;
use trading_core::{Symbol, Timestamp};

/// Errors returned by [`MarkSource`] implementations.
#[derive(Debug, Error)]
pub enum MarkError {
    /// `(symbol, ts)` requested but not in the loaded data set.
    #[error("mark out of range: symbol={symbol} ts={ts}")]
    OutOfRange { symbol: String, ts: String },
    /// Parquet / CSV / IO failure.
    #[error("io: {0}")]
    Io(String),
    /// Decimal parse error on a price string.
    #[error("parse: {0}")]
    Parse(String),
}

/// Operator-supplied price source for marks + BTC baseline.
///
/// Production: [`ParquetMarkSource::new`].
/// Tests: [`FrozenMarkSource::from_csv`].
pub trait MarkSource: Send + Sync {
    /// Return the close price of `symbol` at the bar whose `close_ts`
    /// is the latest one `≤ ts`.
    ///
    /// # Errors
    ///
    /// Returns [`MarkError::OutOfRange`] when no bar in the loaded data
    /// has a `close_ts ≤ ts` for `symbol`.  Returns [`MarkError::Io`] /
    /// [`MarkError::Parse`] on storage failures.
    fn close_at(&self, symbol: &Symbol, ts: Timestamp) -> Result<Decimal, MarkError>;

    /// Return close prices for `symbol` over `[from, to]` at the given
    /// cadence in minutes.  `cadence_minutes == 1` returns 1m bars;
    /// `cadence_minutes == 5` returns the close of every 5th 1m bar.
    ///
    /// # Errors
    ///
    /// Returns [`MarkError::OutOfRange`] when the requested window
    /// overlaps no loaded bars for `symbol`.  Returns
    /// [`MarkError::Io`] / [`MarkError::Parse`] on storage failures.
    fn close_series(
        &self,
        symbol: &Symbol,
        from: Timestamp,
        to: Timestamp,
        cadence_minutes: u32,
    ) -> Result<Vec<(Timestamp, Decimal)>, MarkError>;
}

// ── ParquetMarkSource ─────────────────────────────────────────────────────────

/// Tiny LRU keyed on `(symbol, unix_millis)` — the `parking_lot` mutex
/// keeps the trait object `Send + Sync` without async overhead.
struct ParquetCache {
    /// Insertion-ordered map: oldest entries at the front.  We
    /// re-insert on lookup to refresh recency.  Capped at 4096 entries.
    entries: BTreeMap<(String, i64), Decimal>,
    /// Insertion stamp counter, used as the secondary key when the
    /// map exceeds capacity.
    stamps: Vec<((String, i64), u64)>,
    next_stamp: u64,
    capacity: usize,
}

impl ParquetCache {
    fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            stamps: Vec::with_capacity(capacity),
            next_stamp: 0,
            capacity,
        }
    }

    fn get(&mut self, key: &(String, i64)) -> Option<Decimal> {
        let v = self.entries.get(key).copied()?;
        // Refresh recency by appending a new stamp; eviction will skip
        // stale stamp entries that no longer match the latest stamp.
        self.next_stamp += 1;
        self.stamps.push((key.clone(), self.next_stamp));
        Some(v)
    }

    fn insert(&mut self, key: (String, i64), value: Decimal) {
        self.next_stamp += 1;
        self.entries.insert(key.clone(), value);
        self.stamps.push((key, self.next_stamp));
        // Soft eviction: when we exceed 2× capacity stamps, prune.
        if self.stamps.len() > self.capacity * 2 {
            self.prune();
        }
    }

    fn prune(&mut self) {
        // Walk stamps from newest to oldest, keeping only the first
        // (newest) reference for each key.  Drop everything else.
        let mut seen = std::collections::HashSet::with_capacity(self.capacity);
        let mut keep = Vec::with_capacity(self.capacity);
        for (key, stamp) in self.stamps.iter().rev().cloned() {
            if seen.insert(key.clone()) && keep.len() < self.capacity {
                keep.push((key, stamp));
            }
        }
        keep.reverse();
        let kept_keys: std::collections::HashSet<(String, i64)> =
            keep.iter().map(|(k, _)| k.clone()).collect();
        self.entries.retain(|k, _| kept_keys.contains(k));
        self.stamps = keep;
    }
}

/// Reads parquet files under `<root>/<SYMBOL>/<YEAR>/*.parquet` for
/// each requested symbol.  Files are scanned via Polars `LazyFrame` so
/// only the columns we need (`close_time`, `close`) are pulled into
/// memory.
pub struct ParquetMarkSource {
    parquet_root: PathBuf,
    cache: Mutex<ParquetCache>,
}

impl ParquetMarkSource {
    /// Create a new `ParquetMarkSource` rooted at `parquet_root`.
    ///
    /// LRU capacity is 4096 entries — bounded uptime memory at
    /// ~150 KiB (R-4 in the risk register).
    #[must_use]
    pub fn new(parquet_root: impl Into<PathBuf>) -> Self {
        Self {
            parquet_root: parquet_root.into(),
            cache: Mutex::new(ParquetCache::new(4096)),
        }
    }

    /// Walk all `.parquet` files under `<root>/<symbol>/`, sorted by
    /// path (year ASC, then filename ASC — deterministic order, as
    /// required by the determinism plan in the feature brief).
    fn parquet_files(&self, symbol: &Symbol) -> Vec<PathBuf> {
        let sym_dir = self.parquet_root.join(symbol.0.as_str());
        if !sym_dir.exists() {
            return vec![];
        }
        let mut out: Vec<PathBuf> = Vec::new();
        let walk_dir = |dir: &Path, out: &mut Vec<PathBuf>| {
            if let Ok(it) = std::fs::read_dir(dir) {
                let mut paths: Vec<PathBuf> = it.flatten().map(|e| e.path()).collect();
                paths.sort();
                for p in paths {
                    if p.is_file() && p.extension().and_then(|x| x.to_str()) == Some("parquet") {
                        out.push(p);
                    }
                }
            }
        };
        // Top level: year subdirectories OR direct parquets.
        if let Ok(it) = std::fs::read_dir(&sym_dir) {
            let mut paths: Vec<PathBuf> = it.flatten().map(|e| e.path()).collect();
            paths.sort();
            for p in paths {
                if p.is_dir() {
                    walk_dir(&p, &mut out);
                } else if p.extension().and_then(|x| x.to_str()) == Some("parquet") {
                    out.push(p);
                }
            }
        }
        out
    }

    /// Read all `(close_time_ms, close_decimal)` pairs from a single
    /// parquet file.  Time is taken from `close_time` (Int64 millis),
    /// price is taken from the `close` Utf8 column and parsed as
    /// `Decimal`.
    fn read_pairs(path: &Path) -> Result<Vec<(i64, Decimal)>, MarkError> {
        let lf = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
            .map_err(|e| MarkError::Io(format!("scan_parquet {}: {e}", path.display())))?;
        let df = lf
            .select([col("close_time"), col("close")])
            .sort(
                ["close_time"],
                SortMultipleOptions::default().with_order_descending(false),
            )
            .collect()
            .map_err(|e| MarkError::Io(format!("collect {}: {e}", path.display())))?;

        let n = df.height();
        let mut out = Vec::with_capacity(n);
        let times = df
            .column("close_time")
            .map_err(|e| MarkError::Io(e.to_string()))?
            .i64()
            .map_err(|e| MarkError::Io(e.to_string()))?;
        let closes = df
            .column("close")
            .map_err(|e| MarkError::Io(e.to_string()))?
            .str()
            .map_err(|e| MarkError::Io(e.to_string()))?;
        for i in 0..n {
            let t = times
                .get(i)
                .ok_or_else(|| MarkError::Parse("close_time null".into()))?;
            let c_str = closes
                .get(i)
                .ok_or_else(|| MarkError::Parse("close null".into()))?;
            let c: Decimal = c_str
                .trim()
                .parse()
                .map_err(|e: rust_decimal::Error| MarkError::Parse(e.to_string()))?;
            out.push((t, c));
        }
        Ok(out)
    }

    /// Load every `(close_time_ms, close)` pair for `symbol` across all
    /// parquet files, sorted ascending by `close_time`.  Used by both
    /// `close_at` (linear scan for now — LRU absorbs the cost) and
    /// `close_series`.
    fn load_all(&self, symbol: &Symbol) -> Result<Vec<(i64, Decimal)>, MarkError> {
        let mut all: Vec<(i64, Decimal)> = Vec::new();
        for f in self.parquet_files(symbol) {
            let pairs = Self::read_pairs(&f)?;
            all.extend(pairs);
        }
        all.sort_by_key(|(t, _)| *t);
        Ok(all)
    }
}

impl MarkSource for ParquetMarkSource {
    fn close_at(&self, symbol: &Symbol, ts: Timestamp) -> Result<Decimal, MarkError> {
        let want_ms = ts.unix_millis();
        let key = (symbol.0.as_str().to_string(), want_ms);
        if let Some(v) = self.cache.lock().get(&key) {
            return Ok(v);
        }
        let pairs = self.load_all(symbol)?;
        // Latest bar with close_time <= want_ms.
        let mut found: Option<Decimal> = None;
        for (t, c) in &pairs {
            if *t <= want_ms {
                found = Some(*c);
            } else {
                break;
            }
        }
        let v = found.ok_or_else(|| MarkError::OutOfRange {
            symbol: symbol.0.as_str().to_string(),
            ts: ts.to_string(),
        })?;
        self.cache.lock().insert(key, v);
        Ok(v)
    }

    fn close_series(
        &self,
        symbol: &Symbol,
        from: Timestamp,
        to: Timestamp,
        cadence_minutes: u32,
    ) -> Result<Vec<(Timestamp, Decimal)>, MarkError> {
        let from_ms = from.unix_millis();
        let to_ms = to.unix_millis();
        if to_ms < from_ms {
            return Err(MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: format!("{from} > {to}"),
            });
        }
        let pairs = self.load_all(symbol)?;
        if pairs.is_empty() {
            return Err(MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: format!("{from}..{to}"),
            });
        }

        let cadence = i64::from(cadence_minutes.max(1)) * 60_000;
        let mut out: Vec<(Timestamp, Decimal)> = Vec::new();
        let mut idx = 0usize;
        let mut cursor = from_ms;
        while cursor <= to_ms {
            // Advance idx to the latest pair with t <= cursor.
            while idx + 1 < pairs.len() && pairs[idx + 1].0 <= cursor {
                idx += 1;
            }
            if pairs[idx].0 <= cursor {
                let ts = ms_to_timestamp(cursor);
                out.push((ts, pairs[idx].1));
            }
            // Bump cursor by cadence.
            cursor = cursor.saturating_add(cadence);
            if cadence == 0 {
                break;
            }
        }
        if out.is_empty() {
            return Err(MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: format!("{from}..{to}"),
            });
        }
        Ok(out)
    }
}

// ── FrozenMarkSource (test-only deterministic CSV) ──────────────────────────

/// Test-only deterministic mark source.  Loads
/// `(symbol, close_time_ms, close)` rows from a CSV at construction
/// time and serves them from memory.
///
/// CSV columns (header required, exact order):
///
/// ```csv
/// symbol,close_time,close
/// BTCUSDT,1714521660000,68000.00
/// ```
///
/// `close_time` is Unix milliseconds.  `close` is a plain decimal
/// string.  Rows are sorted at load time by `(symbol, close_time)`.
pub struct FrozenMarkSource {
    /// `BTreeMap<symbol, Vec<(close_time_ms, close)>>`, sorted by
    /// `close_time`.
    by_symbol: BTreeMap<String, Vec<(i64, Decimal)>>,
}

impl FrozenMarkSource {
    /// Load a `FrozenMarkSource` from a CSV at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`MarkError::Io`] on file IO or schema failures, and
    /// [`MarkError::Parse`] on Decimal / integer parse failures in the
    /// CSV body.
    pub fn from_csv(path: impl AsRef<Path>) -> Result<Self, MarkError> {
        let body = std::fs::read_to_string(path.as_ref())
            .map_err(|e| MarkError::Io(format!("read csv: {e}")))?;
        Self::from_csv_str(&body)
    }

    /// Load a `FrozenMarkSource` from a CSV body string (used by both
    /// [`Self::from_csv`] and the in-tree test fixtures).
    ///
    /// # Errors
    ///
    /// Returns [`MarkError::Io`] for header / schema failures and
    /// [`MarkError::Parse`] for unparseable price/time fields.
    pub fn from_csv_str(body: &str) -> Result<Self, MarkError> {
        let mut by_symbol: BTreeMap<String, Vec<(i64, Decimal)>> = BTreeMap::new();
        let mut lines = body.lines();
        let header = lines
            .next()
            .ok_or_else(|| MarkError::Io("empty csv".into()))?;
        let cols: Vec<&str> = header.split(',').map(str::trim).collect();
        if cols != ["symbol", "close_time", "close"] {
            return Err(MarkError::Io(format!(
                "csv header mismatch: expected `symbol,close_time,close`, got `{header}`"
            )));
        }
        for (lineno, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() != 3 {
                return Err(MarkError::Io(format!(
                    "csv row {lineno}: expected 3 columns, got {}",
                    parts.len()
                )));
            }
            let sym = parts[0].to_string();
            let t: i64 = parts[1]
                .parse()
                .map_err(|e: std::num::ParseIntError| MarkError::Parse(e.to_string()))?;
            let c: Decimal = parts[2]
                .parse()
                .map_err(|e: rust_decimal::Error| MarkError::Parse(e.to_string()))?;
            by_symbol.entry(sym).or_default().push((t, c));
        }
        for v in by_symbol.values_mut() {
            v.sort_by_key(|(t, _)| *t);
        }
        Ok(Self { by_symbol })
    }
}

impl MarkSource for FrozenMarkSource {
    fn close_at(&self, symbol: &Symbol, ts: Timestamp) -> Result<Decimal, MarkError> {
        let want_ms = ts.unix_millis();
        let pairs = self
            .by_symbol
            .get(symbol.0.as_str())
            .ok_or_else(|| MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: ts.to_string(),
            })?;
        let mut found: Option<Decimal> = None;
        for (t, c) in pairs {
            if *t <= want_ms {
                found = Some(*c);
            } else {
                break;
            }
        }
        found.ok_or_else(|| MarkError::OutOfRange {
            symbol: symbol.0.as_str().to_string(),
            ts: ts.to_string(),
        })
    }

    fn close_series(
        &self,
        symbol: &Symbol,
        from: Timestamp,
        to: Timestamp,
        cadence_minutes: u32,
    ) -> Result<Vec<(Timestamp, Decimal)>, MarkError> {
        let pairs = self
            .by_symbol
            .get(symbol.0.as_str())
            .ok_or_else(|| MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: format!("{from}..{to}"),
            })?;
        let from_ms = from.unix_millis();
        let to_ms = to.unix_millis();
        if to_ms < from_ms {
            return Err(MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: format!("{from} > {to}"),
            });
        }
        let cadence = i64::from(cadence_minutes.max(1)) * 60_000;
        let mut out: Vec<(Timestamp, Decimal)> = Vec::new();
        let mut idx = 0usize;
        let mut cursor = from_ms;
        while cursor <= to_ms {
            while idx + 1 < pairs.len() && pairs[idx + 1].0 <= cursor {
                idx += 1;
            }
            if pairs[idx].0 <= cursor {
                let ts = ms_to_timestamp(cursor);
                out.push((ts, pairs[idx].1));
            }
            cursor = cursor.saturating_add(cadence);
            if cadence == 0 {
                break;
            }
        }
        if out.is_empty() {
            return Err(MarkError::OutOfRange {
                symbol: symbol.0.as_str().to_string(),
                ts: format!("{from}..{to}"),
            });
        }
        Ok(out)
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn ms_to_timestamp(ms: i64) -> Timestamp {
    let nanos = i128::from(ms) * 1_000_000;
    let dt = time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    Timestamp::new(dt)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const TINY_CSV: &str = "symbol,close_time,close\n\
        BTCUSDT,1000,1.00\n\
        BTCUSDT,2000,2.00\n\
        BTCUSDT,3000,3.00\n";

    #[test]
    fn t812_frozen_close_at_returns_latest_le() {
        let src = FrozenMarkSource::from_csv_str(TINY_CSV).unwrap();
        let ts = ms_to_timestamp(2500);
        let v = src.close_at(&Symbol::new("BTCUSDT"), ts).unwrap();
        assert_eq!(v, "2.00".parse::<Decimal>().unwrap());
    }

    #[test]
    fn t812_frozen_close_at_out_of_range_below_first_bar() {
        let src = FrozenMarkSource::from_csv_str(TINY_CSV).unwrap();
        let ts = ms_to_timestamp(0);
        let err = src.close_at(&Symbol::new("BTCUSDT"), ts).unwrap_err();
        assert!(matches!(err, MarkError::OutOfRange { .. }));
    }

    #[test]
    fn t812_frozen_close_at_unknown_symbol_out_of_range() {
        let src = FrozenMarkSource::from_csv_str(TINY_CSV).unwrap();
        let ts = ms_to_timestamp(1500);
        let err = src.close_at(&Symbol::new("ETHUSDT"), ts).unwrap_err();
        assert!(matches!(err, MarkError::OutOfRange { .. }));
    }

    #[test]
    fn t812_frozen_close_series_matches_cadence() {
        let src = FrozenMarkSource::from_csv_str(TINY_CSV).unwrap();
        let from = ms_to_timestamp(1000);
        let to = ms_to_timestamp(3000);
        let series = src
            .close_series(&Symbol::new("BTCUSDT"), from, to, 1)
            .unwrap();
        // Cadence = 60_000 ms (1 minute).  Window is 2 seconds, so we
        // get exactly one row at the start.
        assert_eq!(series.len(), 1);
    }

    #[test]
    fn t812_frozen_csv_header_mismatch_errors() {
        let bad = "wrong,header,here\n";
        assert!(FrozenMarkSource::from_csv_str(bad).is_err());
    }

    #[test]
    fn t812_parquet_files_returns_sorted_paths() {
        // Build a tempdir with a faux symbol layout and assert we walk
        // it deterministically.  No actual parquets needed for this
        // structure-only test; load_all on missing dir just returns [].
        let dir = tempfile::tempdir().unwrap();
        let sym_dir = dir.path().join("BTCUSDT");
        std::fs::create_dir_all(sym_dir.join("2023")).unwrap();
        std::fs::create_dir_all(sym_dir.join("2024")).unwrap();
        let f1 = sym_dir.join("2023").join("01.parquet");
        let f2 = sym_dir.join("2024").join("01.parquet");
        std::fs::write(&f1, b"placeholder").unwrap();
        std::fs::write(&f2, b"placeholder").unwrap();

        let src = ParquetMarkSource::new(dir.path());
        let files = src.parquet_files(&Symbol::new("BTCUSDT"));
        assert_eq!(files, vec![f1, f2]);
    }

    #[test]
    fn t812_parquet_close_at_round_trips_via_tempdir() {
        // Write a tiny ad-hoc parquet via Polars and round-trip
        // close_at against it.  No fixture dependency.
        use polars::prelude::*;
        let dir = tempfile::tempdir().unwrap();
        let sym_dir = dir.path().join("BTCUSDT").join("2024");
        std::fs::create_dir_all(&sym_dir).unwrap();
        let parquet_path = sym_dir.join("part.parquet");

        let mut df = df!(
            "open_time" => &[1_000_i64, 2_000_i64, 3_000_i64],
            "close_time" => &[1_059_i64, 2_059_i64, 3_059_i64],
            "open" => &["1.0", "2.0", "3.0"],
            "high" => &["1.1", "2.1", "3.1"],
            "low" => &["0.9", "1.9", "2.9"],
            "close" => &["1.00", "2.00", "3.00"],
            "volume" => &["1.0", "1.0", "1.0"],
            "trade_count" => &[1_i64, 1_i64, 1_i64],
        )
        .unwrap();
        let mut f = std::fs::File::create(&parquet_path).unwrap();
        ParquetWriter::new(&mut f).finish(&mut df).unwrap();

        let src = ParquetMarkSource::new(dir.path());
        let ts = ms_to_timestamp(2_500);
        let v = src.close_at(&Symbol::new("BTCUSDT"), ts).unwrap();
        assert_eq!(v, "2.00".parse::<Decimal>().unwrap());

        // Out-of-range: ts before first bar.
        let early = ms_to_timestamp(500);
        let err = src.close_at(&Symbol::new("BTCUSDT"), early).unwrap_err();
        assert!(matches!(err, MarkError::OutOfRange { .. }));
    }
}
