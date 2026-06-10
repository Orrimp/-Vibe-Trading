//! Parquet replay feed (T09).
//!
//! Reads `data/binance/<SYMBOL>/<YEAR>/*.parquet` and drives `MarketDataSource`
//! at wallclock pace, as-fast-as-possible, or at an accelerated-but-streamed pace.
//!
//! ## Replay pace modes
//!
//! | `fast` | `pace_ms`    | Behaviour                                        |
//! |--------|--------------|--------------------------------------------------|
//! | `true` | `None`       | **Fast**: emit all bars instantly (default for backtest/research headless) |
//! | `false`| `None`       | **Wallclock**: sleep the real interval between bars (1m bar = 1 real minute) |
//! | any    | `Some(n)`    | **Paced**: sleep `n` ms between every bar regardless of bar interval; `fast` is ignored when `pace_ms` is `Some` |
//!
//! The paced mode is the correct middle gear for the cockpit UI (e.g. 30 ms/bar):
//! the strategy warms up (~50 bars × 30 ms ≈ 1.5 s) and then emits fills and PnL
//! updates over a watchable timeline — fast enough for the UI to stay live, slow
//! enough that late bus subscribers (the iced subscription layer) catch every event.
//!
//! Expected Parquet schema (column names, all nullable):
//! ```text
//! open_time   Int64   — Unix millis, bar open
//! close_time  Int64   — Unix millis, bar close
//! open        Utf8    — price string
//! high        Utf8
//! low         Utf8
//! close       Utf8
//! volume      Utf8
//! trade_count Int64   — number of trades in bar
//! ```

use async_trait::async_trait;
use futures::stream::BoxStream;
use polars::prelude::*;
use std::path::PathBuf;
use time::OffsetDateTime;
use tracing::{debug, info};
use trading_core::{Bar, FeedError, Price, Quantity, Symbol, Tick, Timeframe, Timestamp, Venue};

use crate::source::{MarketDataSource, SymbolInfo};

/// Drives `MarketDataSource` from stored Parquet files.
pub struct ReplayFeed {
    /// Root of the data directory, e.g. `data/binance`.
    pub parquet_root: PathBuf,
    /// If `true` (and `pace_ms` is `None`), emit bars as fast as possible;
    /// if `false` (and `pace_ms` is `None`), emit at wallclock pace.
    /// Ignored when `pace_ms` is `Some`.
    pub fast: bool,
    /// Accelerated-but-streamed pace: sleep this many milliseconds between
    /// emitting consecutive bars.  `None` defers to `fast`.
    ///
    /// Use `Some(30)` for the cockpit live view (1.5 s warmup at SMA-50, then
    /// fills/pnl arrive over minutes at a rate the UI subscriptions can catch).
    /// Leave `None` for headless backtests and research replays (fast = true).
    pub pace_ms: Option<u64>,
}

impl ReplayFeed {
    /// Create a fast `ReplayFeed` pointing at `parquet_root`.
    ///
    /// Equivalent to `new_with_pace(parquet_root, true, None)`.
    #[must_use]
    pub fn new(parquet_root: impl Into<PathBuf>, fast: bool) -> Self {
        let parquet_root = parquet_root.into();
        info!(path = %parquet_root.display(), fast, pace_ms = "none", "ReplayFeed initialised");
        Self {
            parquet_root,
            fast,
            pace_ms: None,
        }
    }

    /// Create a paced `ReplayFeed` that sleeps `pace_ms` milliseconds between
    /// every emitted bar.  When `pace_ms` is `Some`, the `fast` flag is ignored.
    ///
    /// # Panics
    ///
    /// Does not panic — `pace_ms = Some(0)` is valid (no sleep, same as fast).
    #[must_use]
    pub fn new_with_pace(
        parquet_root: impl Into<PathBuf>,
        fast: bool,
        pace_ms: Option<u64>,
    ) -> Self {
        let parquet_root = parquet_root.into();
        info!(
            path = %parquet_root.display(),
            fast,
            ?pace_ms,
            "ReplayFeed initialised",
        );
        Self {
            parquet_root,
            fast,
            pace_ms,
        }
    }

    /// Collect all `.parquet` files for `<symbol>` under `parquet_root`,
    /// sorted by path (which sorts by year then filename).
    fn parquet_files(&self, symbol: &Symbol) -> Vec<PathBuf> {
        let sym_dir = self.parquet_root.join(symbol.0.as_str());
        if !sym_dir.exists() {
            return vec![];
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(&sym_dir)
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|entry| {
                let p = entry.path();
                // Recursively descend one level (year directories)
                if p.is_dir() {
                    std::fs::read_dir(&p)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|e| e.path())
                        .filter(|ep| ep.extension().is_some_and(|x| x == "parquet"))
                        .collect::<Vec<_>>()
                } else if p.extension().is_some_and(|x| x == "parquet") {
                    vec![p]
                } else {
                    vec![]
                }
            })
            .collect();
        files.sort();
        files
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn millis_to_ts(ms: i64) -> Timestamp {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map(Timestamp::new)
        .unwrap_or_else(|_| Timestamp::now())
}

fn parse_price_str(s: &str) -> Result<Price, FeedError> {
    s.trim()
        .parse::<rust_decimal::Decimal>()
        .map_err(|e| FeedError::Parse(format!("price parse: {e}")))
        .and_then(|d| Price::new(d).map_err(|e| FeedError::Parse(e.to_string())))
}

fn parse_qty_str(s: &str) -> Result<Quantity, FeedError> {
    s.trim()
        .parse::<rust_decimal::Decimal>()
        .map_err(|e| FeedError::Parse(format!("qty parse: {e}")))
        .and_then(|d| Quantity::new(d).map_err(|e| FeedError::Parse(e.to_string())))
}

/// Read one Parquet file into an ordered list of `Bar` values.
fn read_parquet_bars(
    path: &PathBuf,
    symbol: &Symbol,
    tf: Timeframe,
) -> Result<Vec<Bar>, FeedError> {
    let df = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .sort(
            ["open_time"],
            SortMultipleOptions::default().with_order_descending(false),
        )
        .collect()
        .map_err(|e| FeedError::Parse(e.to_string()))?;

    let n = df.height();
    let mut bars = Vec::with_capacity(n);

    let open_times = df
        .column("open_time")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .i64()
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    let close_times = df
        .column("close_time")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .i64()
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    let opens = df
        .column("open")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .str()
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    let highs = df
        .column("high")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .str()
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    let lows = df
        .column("low")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .str()
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    let closes = df
        .column("close")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .str()
        .map_err(|e| FeedError::Parse(e.to_string()))?;
    let volumes = df
        .column("volume")
        .map_err(|e| FeedError::Parse(e.to_string()))?
        .str()
        .map_err(|e| FeedError::Parse(e.to_string()))?;

    // trade_count may be missing in older datasets — default to 0.
    let trade_counts: Option<&ChunkedArray<Int64Type>> =
        df.column("trade_count").ok().and_then(|s| s.i64().ok());

    for i in 0..n {
        let open_time = open_times
            .get(i)
            .ok_or_else(|| FeedError::Parse("null open_time".into()))?;
        let close_time = close_times
            .get(i)
            .ok_or_else(|| FeedError::Parse("null close_time".into()))?;
        let open_str = opens
            .get(i)
            .ok_or_else(|| FeedError::Parse("null open".into()))?;
        let high_str = highs
            .get(i)
            .ok_or_else(|| FeedError::Parse("null high".into()))?;
        let low_str = lows
            .get(i)
            .ok_or_else(|| FeedError::Parse("null low".into()))?;
        let close_str = closes
            .get(i)
            .ok_or_else(|| FeedError::Parse("null close".into()))?;
        let vol_str = volumes
            .get(i)
            .ok_or_else(|| FeedError::Parse("null volume".into()))?;
        let tc = trade_counts.and_then(|tc| tc.get(i)).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let trade_count = tc.max(0) as u32;

        bars.push(Bar {
            symbol: symbol.clone(),
            tf,
            open_ts: millis_to_ts(open_time),
            close_ts: millis_to_ts(close_time),
            open: parse_price_str(open_str)?,
            high: parse_price_str(high_str)?,
            low: parse_price_str(low_str)?,
            close: parse_price_str(close_str)?,
            volume: parse_qty_str(vol_str)?,
            trade_count,
            local_recv_ts: Timestamp::now(),
            // Replay fixtures originate from Binance archives.
            venue: Venue::Binance,
        });
    }

    debug!(file = ?path, bars = n, "parquet file loaded");
    Ok(bars)
}

// ── Multi-symbol k-way merge (v1 T611) ───────────────────────────────────────

impl ReplayFeed {
    /// Deterministic k-way merge of N per-symbol bar streams.
    ///
    /// Sort key: `(venue_ts ASC, symbol ASC)` — matches R12.2.
    ///
    /// Reads all Parquet files for every symbol up-front (same strategy as
    /// `subscribe_bars` for v0 single-symbol), then merges via a sorted
    /// structure.  Memory bound: O(total bar count × bar size).  For a year
    /// of 10 symbols at 1m granularity (~5.2M bars), each bar is ~200 bytes
    /// → ~1 GB peak; within the 64 MiB integration-test budget for the
    /// 10-symbol fixture (which uses synthetic bars, not full-year Parquet).
    ///
    /// A `debug_assert!` verifies per-symbol monotonic `venue_ts` order before
    /// the merge runs (architect risk #1 — silent failure if out-of-order).
    ///
    /// # Errors
    ///
    /// Returns `FeedError::Io` if no Parquet files are found for any symbol.
    /// Returns `FeedError::Parse` on Parquet schema errors.
    pub fn merge_symbols(
        &self,
        symbol_paths: &[(Symbol, std::path::PathBuf)],
        tf: Timeframe,
    ) -> Result<Vec<Bar>, FeedError> {
        let mut all_bars: Vec<Bar> = Vec::new();

        for (symbol, _root) in symbol_paths {
            let files = self.parquet_files(symbol);
            if files.is_empty() {
                return Err(FeedError::Io(format!(
                    "no parquet files found for {} under {:?}",
                    symbol, self.parquet_root
                )));
            }

            let mut sym_bars: Vec<Bar> = Vec::new();
            for file in &files {
                let mut parsed = read_parquet_bars(file, symbol, tf)?;
                sym_bars.append(&mut parsed);
            }
            sym_bars.sort_by_key(|b| b.open_ts);

            // Architect risk #1: assert per-symbol monotonicity before merge.
            debug_assert!(
                sym_bars.windows(2).all(|w| w[0].open_ts <= w[1].open_ts),
                "per-symbol bars must be monotonically non-decreasing in venue_ts for symbol {}",
                symbol
            );

            all_bars.append(&mut sym_bars);
        }

        // k-way merge: sort by (venue_ts ASC, symbol ASC) per R12.2.
        all_bars.sort_by(|a, b| {
            a.open_ts
                .cmp(&b.open_ts)
                .then_with(|| a.symbol.0.cmp(&b.symbol.0))
        });

        Ok(all_bars)
    }

    /// Merge symbols from synthetic bar vectors (for testing / synthetic fallback).
    ///
    /// Same sort key as `merge_symbols`: `(venue_ts ASC, symbol ASC)`.
    pub fn merge_synthetic(bars_by_symbol: Vec<Vec<Bar>>) -> Vec<Bar> {
        let mut all_bars: Vec<Bar> = bars_by_symbol.into_iter().flatten().collect();
        all_bars.sort_by(|a, b| {
            a.open_ts
                .cmp(&b.open_ts)
                .then_with(|| a.symbol.0.cmp(&b.symbol.0))
        });
        all_bars
    }
}

// ── MarketDataSource impl ─────────────────────────────────────────────────────

#[async_trait]
impl MarketDataSource for ReplayFeed {
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError> {
        Ok(SymbolInfo {
            symbol: symbol.clone(),
            base_asset: "BTC".into(),
            quote_asset: "USDT".into(),
            min_qty: rust_decimal::Decimal::new(1, 5),
            lot_size: rust_decimal::Decimal::new(1, 5),
            min_notional: rust_decimal::Decimal::new(10, 0),
        })
    }

    /// Stream all bars from all Parquet files matching `symbol`, in time order.
    ///
    /// If `fast == false`, sleeps for the real interval between bar close
    /// times, allowing realistic time-compressed replay.
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Io`] if no Parquet files are found for the symbol,
    /// or [`FeedError::Parse`] on schema errors.
    async fn subscribe_bars(
        &self,
        symbol: Symbol,
        tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        let files = self.parquet_files(&symbol);
        if files.is_empty() {
            return Err(FeedError::Io(format!(
                "no parquet files found for {} under {:?}",
                symbol, self.parquet_root
            )));
        }

        // Pre-load all bars from all files into memory.
        // For very large datasets this should be lazy, but for v0 (1-year
        // BTCUSDT @1m ≈ 525 k rows × ~200 bytes = ~100 MB) it's fine.
        let mut all_bars: Vec<Bar> = Vec::new();
        for file in &files {
            match read_parquet_bars(file, &symbol, tf) {
                Ok(mut bars) => all_bars.append(&mut bars),
                Err(e) => return Err(e),
            }
        }
        all_bars.sort_by_key(|b| b.open_ts);

        let fast = self.fast;
        let pace_ms = self.pace_ms;
        let stream = async_stream::stream! {
            let mut prev_close: Option<Timestamp> = None;
            for bar in all_bars {
                if let Some(ms) = pace_ms {
                    // Paced mode: fixed inter-bar delay regardless of bar interval.
                    // `pace_ms = Some(0)` is valid — no sleep but still yields.
                    if ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
                    } else {
                        // Yield so other tasks get scheduled, even with pace = 0.
                        tokio::task::yield_now().await;
                    }
                } else if !fast
                    && let Some(prev) = prev_close
                {
                    // Wallclock mode: sleep the real bar interval.
                    let now = Timestamp::now();
                    let bar_interval_ms = bar.close_ts.unix_millis() - prev.unix_millis();
                    let elapsed_ms = now.unix_millis() - prev.unix_millis();
                    if bar_interval_ms > elapsed_ms {
                        let sleep_ms = u64::try_from(bar_interval_ms - elapsed_ms)
                            .unwrap_or(0);
                        tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                    }
                }
                // Fast mode: no sleep — fall through and yield immediately.
                prev_close = Some(bar.close_ts);
                yield Ok(bar);
            }
        };

        Ok(Box::pin(stream))
    }

    /// Replay feed does not support a trade stream — use `subscribe_bars`.
    ///
    /// # Errors
    ///
    /// Always returns [`FeedError::StreamClosed`].
    async fn subscribe_trades(
        &self,
        _symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        Err(FeedError::StreamClosed)
    }
}
