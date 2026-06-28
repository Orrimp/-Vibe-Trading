//! Parquet → 5-feature 256-bar window iterator (v2.5 TCN feature pipeline).
//!
//! ## Overview
//!
//! This module provides the pure-function OHLCV → feature-window builder that
//! is shared verbatim between training (`bin/train_tcn.rs`) and inference
//! (`TcnForecaster::forecast()`). Strict sharing is load-bearing for
//! strict-replay determinism: any drift between training-time and
//! inference-time feature construction would cause replay-cache misses.
//!
//! ## Feature construction (per bar, D3)
//!
//! Given bars `[t-255 … t]`:
//!
//! - `logret      = ln(close_t / close_{t-1})`
//! - `logrange    = ln(1 + (high_t - low_t) / close_t)`
//! - `logvol_z    = (ln(1 + volume_t) − μ₇₂₀) / σ₇₂₀`
//!   where μ₇₂₀/σ₇₂₀ are rolling 30-day (720 h) means within-symbol,
//!   warm-up bars dropped, pinned in metadata for inference.
//! - `hour_sin    = sin(2π · hour_of_week / 168)`
//! - `hour_cos    = cos(2π · hour_of_week / 168)`
//!
//! ## Warm-up
//!
//! The first 720 bars of the span are warm-up for volume-z statistics;
//! the first 1 bar is consumed by `logret`. Window iteration starts at
//! bar index 720, each window covering `[t−255 … t]` (context = 256 bars)
//! with target `r_{t+1} = ln(close_{t+1} / close_t)`.
//!
//! ## Parquet schema (from `crates/data/src/replay_feed.rs`)
//!
//! ```text
//! open_time   Int64   — Unix millis
//! close_time  Int64   — Unix millis
//! open        Utf8
//! high        Utf8
//! low         Utf8
//! close       Utf8
//! volume      Utf8
//! trade_count Int64
//! ```
//!
//! ## Cross-references
//!
//! - `spec/v1/v25-tcn-overlay/feature.md § D3`
//! - `crates/data/src/replay_feed.rs` — parquet schema origin
//! - `ADR-0029` — provenance contract (μ/σ pinned in metadata)

use std::path::Path;
use std::str::FromStr;

use thiserror::Error;
use time::OffsetDateTime;
use tracing::{debug, warn};

#[cfg(feature = "candle")]
use candle_core::{Device, Tensor};

/// Feature-vector length per bar.
pub const FEATURE_DIM: usize = 5;

/// Context window length in bars.
pub const CONTEXT_BARS: usize = 256;

/// Volume-z warm-up length in bars.
pub const VOL_Z_WARMUP: usize = 720;

// ── TimeSpan ──────────────────────────────────────────────────────────────────

/// A half-open `[start, end)` interval used to slice the parquet data.
///
/// Both bounds are in UTC (hourly bar open timestamps).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSpan {
    /// Inclusive start (bar open_time >= start).
    pub start: OffsetDateTime,
    /// Exclusive end (bar open_time < end).
    pub end: OffsetDateTime,
}

impl TimeSpan {
    /// Construct a new `TimeSpan`.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `start >= end`.
    #[must_use]
    pub fn new(start: OffsetDateTime, end: OffsetDateTime) -> Self {
        debug_assert!(start < end, "TimeSpan: start must be before end");
        Self { start, end }
    }
}

// ── VolTargetKind ─────────────────────────────────────────────────────────────

/// Which realized-volatility estimator to use for the vol-target label.
///
/// Added additively in v3.0.0-volatility (T-D-N9 / ADR-0038 § D3 / T-AR-3).
/// Existing TCN / PatchTST callers pass `vol_target_kind: None`, so their
/// `FeatureWindow::target_parkinson_vol` is `None` and all byte outputs are
/// unchanged (R11.7 + R11.8 guards).
///
/// ## Parkinson formula (Q1=(b) operator default; locked in ADR-0038 § D3)
///
/// ```text
/// σ̂_P² = (1 / (4 · ln 2)) · mean over k of (ln(high_k / low_k))²
/// σ̂_P  = sqrt(σ̂_P²)
/// ```
///
/// where `k` ranges over the H target-horizon bars immediately following
/// the context window (`window_end+1 ..= window_end+H`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolTargetKind {
    /// (Default) Parkinson realized-vol over the next H bars.
    ///
    /// Formula: `σ̂_P = sqrt((1/(4·ln 2)) · mean over k of (ln(high_k/low_k))²)`.
    /// H = `FeatureConfig::target_horizon_bars` (default 24).
    Parkinson,
    /// (v0.1.1) Realized-vol from close-to-close returns.
    #[allow(dead_code)]
    RealizedVol,
}

// ── FeatureConfig ─────────────────────────────────────────────────────────────

/// Configuration for feature construction.
///
/// All fields can be overridden via `train_tcn.toml`; these are the
/// architect-locked defaults per feature.md R4 / D3.
#[derive(Debug, Clone)]
pub struct FeatureConfig {
    /// Number of bars per context window (default 256).
    pub context_bars: usize,
    /// Warm-up length for volume-z statistics (default 720).
    pub vol_z_lookback: usize,
    /// Direction epsilon for `ForecastOverlay` translation (default 0.0005).
    pub direction_epsilon: f32,
    /// Number of bars ahead for target log-return derivation.
    ///
    /// Default 1 for TCN byte-compatibility: `target = r_{t+1}`.
    /// Set to 24 for PatchTST 24h horizon per Q4=(b) / ADR-0036 § D1.
    ///
    /// The target computation becomes:
    /// `target_logret = ln(close_{t + target_horizon_bars} / close_t)`
    pub target_horizon_bars: usize,
    /// Optional realized-vol target kind (v3.0.0-volatility additive field).
    ///
    /// `None` (default) → no `target_parkinson_vol` emitted — TCN / PatchTST
    /// callers are unaffected (anchor-additive-zero contract, R11.7 + R11.8).
    ///
    /// `Some(VolTargetKind::Parkinson)` → emit Parkinson σ in
    /// `FeatureWindow::target_parkinson_vol`.
    pub vol_target_kind: Option<VolTargetKind>,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            context_bars: CONTEXT_BARS,
            vol_z_lookback: VOL_Z_WARMUP,
            direction_epsilon: 0.000_5_f32,
            // Default 1 preserves TCN byte-compatibility (R7 / ADR-0036 § D1).
            target_horizon_bars: 1,
            // Default None: no vol target — TCN/PatchTST callers unaffected.
            vol_target_kind: None,
        }
    }
}

// ── VolStats ──────────────────────────────────────────────────────────────────

/// Rolling volume-z statistics computed over the warm-up period.
///
/// Pinned in checkpoint metadata for inference-time determinism (ADR-0029).
#[derive(Debug, Clone, PartialEq)]
pub struct VolStats {
    /// Mean of `ln(1 + volume)` over the warm-up window.
    pub mu: f32,
    /// Standard deviation of `ln(1 + volume)` over the warm-up window.
    /// Clamped to a minimum of `1e-8` to prevent division by zero.
    pub sigma: f32,
}

// ── FeatureWindow ─────────────────────────────────────────────────────────────

/// A single training / inference sample.
///
/// Contains a `[context_bars, FEATURE_DIM]` feature matrix and the
/// next-bar log-return target.
///
/// ## Feature layout (column order in tensor)
///
/// | col | feature    |
/// |-----|------------|
/// | 0   | logret     |
/// | 1   | logrange   |
/// | 2   | logvol_z   |
/// | 3   | hour_sin   |
/// | 4   | hour_cos   |
#[derive(Debug, Clone)]
pub struct FeatureWindow {
    /// Feature matrix, shape `[context_bars, FEATURE_DIM]`, dtype `f32`.
    ///
    /// When the `candle` feature is enabled this is a `candle_core::Tensor`.
    /// When it is disabled (e.g. in property tests), it is a `Vec<f32>` with
    /// implicit row-major layout `[bar_0_feat_0, bar_0_feat_1, …, bar_N_feat_4]`.
    pub features: FeatureTensor,
    /// Next-bar log-return target: `r_{t+1} = ln(close_{t+1} / close_t)`.
    pub target_logret: f32,
    /// Parkinson realized-vol target over the next H bars (v3.0.0-volatility additive).
    ///
    /// `None` when `FeatureConfig::vol_target_kind` is `None` (TCN / PatchTST callers).
    /// `Some(sigma)` when `vol_target_kind == Some(VolTargetKind::Parkinson)`.
    ///
    /// Formula: `σ̂_P = sqrt((1/(4·ln 2)) · mean over k of (ln(high_k/low_k))²)`.
    pub target_parkinson_vol: Option<f32>,
    /// Symbol these bars belong to.
    pub symbol: String,
    /// Bar-close timestamp of the last bar in the context window (bar `t`).
    pub bar_close_ts: OffsetDateTime,
    /// Volume-z statistics used for feature construction (for metadata pinning).
    pub vol_stats: VolStats,
}

/// The feature tensor representation.
///
/// `candle_core::Tensor` when the `candle` feature is enabled;
/// `Vec<f32>` in plain builds (for property tests + CI without Metal).
#[cfg(feature = "candle")]
pub type FeatureTensor = Tensor;

/// Plain `Vec<f32>` fallback when `candle` feature is disabled.
/// Layout: row-major `[bar_0_feat_0, …, bar_0_feat_4, bar_1_feat_0, …]`.
#[cfg(not(feature = "candle"))]
pub type FeatureTensor = Vec<f32>;

// ── FeatureError ──────────────────────────────────────────────────────────────

/// Errors from the feature pipeline.
#[derive(Debug, Error)]
pub enum FeatureError {
    /// A required parquet file was not found.
    #[error("parquet file not found: {path}")]
    FileNotFound { path: String },

    /// Parquet schema error or column missing.
    #[error("parquet schema error: {0}")]
    Schema(String),

    /// The loaded data has fewer bars than required (warm-up + context + 1).
    #[error("insufficient bars: need {need}, got {got}")]
    InsufficientBars { need: usize, got: usize },

    /// A numeric value could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),

    /// Tensor construction error (candle).
    #[error("tensor error: {0}")]
    Tensor(String),
}

// ── Internal raw-bar type ─────────────────────────────────────────────────────

/// A raw OHLCV bar loaded from parquet (all numeric fields as `f64` for
/// intermediate computation; converted to `f32` at tensor-boundary).
#[derive(Debug, Clone)]
pub(crate) struct RawBar {
    pub open_time_ms: i64,
    pub close_time_ms: i64,
    /// Bar open price (stored for completeness; not used in current feature set).
    #[allow(dead_code)]
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl RawBar {
    /// Bar close as `OffsetDateTime`.
    pub fn close_ts(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(self.close_time_ms) * 1_000_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
    }

    /// Hour-of-week (0..167) derived from the bar open time.
    pub fn hour_of_week(&self) -> u32 {
        let ts =
            OffsetDateTime::from_unix_timestamp_nanos(i128::from(self.open_time_ms) * 1_000_000)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let day_of_week = ts.weekday().number_days_from_monday() as u32; // 0=Mon
        let hour = ts.hour() as u32;
        day_of_week * 24 + hour
    }
}

// ── Parquet loader ────────────────────────────────────────────────────────────

/// Load all bars for a symbol within `span` from `parquet_root`, sorted by
/// `open_time` ascending.
///
/// This function reads the parquet directory layout:
/// `<parquet_root>/<SYMBOL>/<YEAR>/<MM>.parquet`
///
/// # Errors
///
/// Returns `FeatureError::FileNotFound` if no parquet files exist for the
/// symbol, or `FeatureError::Schema` / `FeatureError::Parse` on data errors.
pub(crate) fn load_bars(
    parquet_root: &Path,
    symbol: &str,
    span: &TimeSpan,
) -> Result<Vec<RawBar>, FeatureError> {
    use polars::prelude::*;

    let sym_dir = parquet_root.join(symbol);
    if !sym_dir.exists() {
        return Err(FeatureError::FileNotFound {
            path: sym_dir.display().to_string(),
        });
    }

    // Collect all .parquet files under the symbol directory.
    let mut files: Vec<std::path::PathBuf> = collect_parquet_files(&sym_dir);
    if files.is_empty() {
        return Err(FeatureError::FileNotFound {
            path: sym_dir.display().to_string(),
        });
    }
    files.sort();

    // Compute span bounds in milliseconds for filtering.
    let span_start_ms = span.start.unix_timestamp() * 1000;
    let span_end_ms = span.end.unix_timestamp() * 1000;

    let mut bars: Vec<RawBar> = Vec::new();

    for file in &files {
        let df = LazyFrame::scan_parquet(file, ScanArgsParquet::default())
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .filter(
                col("open_time")
                    .gt_eq(lit(span_start_ms))
                    .and(col("open_time").lt(lit(span_end_ms))),
            )
            .sort(
                ["open_time"],
                SortMultipleOptions::default().with_order_descending(false),
            )
            .collect()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;

        if df.height() == 0 {
            continue;
        }

        let open_times = df
            .column("open_time")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .i64()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;
        let close_times = df
            .column("close_time")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .i64()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;
        let opens = df
            .column("open")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .str()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;
        let highs = df
            .column("high")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .str()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;
        let lows = df
            .column("low")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .str()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;
        let closes = df
            .column("close")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .str()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;
        let volumes = df
            .column("volume")
            .map_err(|e| FeatureError::Schema(e.to_string()))?
            .str()
            .map_err(|e| FeatureError::Schema(e.to_string()))?;

        for i in 0..df.height() {
            let open_time_ms = open_times
                .get(i)
                .ok_or_else(|| FeatureError::Schema("null open_time".into()))?;
            let close_time_ms = close_times
                .get(i)
                .ok_or_else(|| FeatureError::Schema("null close_time".into()))?;
            let open = parse_f64(opens.get(i).unwrap_or("0"), "open")?;
            let high = parse_f64(highs.get(i).unwrap_or("0"), "high")?;
            let low = parse_f64(lows.get(i).unwrap_or("0"), "low")?;
            let close = parse_f64(closes.get(i).unwrap_or("0"), "close")?;
            let volume = parse_f64(volumes.get(i).unwrap_or("0"), "volume")?;

            bars.push(RawBar {
                open_time_ms,
                close_time_ms,
                open,
                high,
                low,
                close,
                volume,
            });
        }
    }

    bars.sort_by_key(|b| b.open_time_ms);

    debug!(symbol, bar_count = bars.len(), "loaded bars from parquet");

    Ok(bars)
}

fn collect_parquet_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // Descend one level (year sub-directories).
                if let Ok(sub) = std::fs::read_dir(&p) {
                    for sub_entry in sub.flatten() {
                        let sp = sub_entry.path();
                        if sp.extension().is_some_and(|x| x == "parquet") {
                            files.push(sp);
                        }
                    }
                }
            } else if p.extension().is_some_and(|x| x == "parquet") {
                files.push(p);
            }
        }
    }
    files
}

fn parse_f64(s: &str, field: &str) -> Result<f64, FeatureError> {
    f64::from_str(s.trim()).map_err(|e| FeatureError::Parse(format!("{field}: {e} (input={s:?})")))
}

// ── Volume-z statistics ───────────────────────────────────────────────────────

/// Compute volume-z statistics from the first `vol_z_lookback` bars.
///
/// `mu` and `sigma` are over `ln(1 + volume)`.
pub(crate) fn compute_vol_stats(bars: &[RawBar], lookback: usize) -> VolStats {
    let n = bars.len().min(lookback);
    if n == 0 {
        return VolStats {
            mu: 0.0,
            sigma: 1.0,
        };
    }
    let log_vols: Vec<f64> = bars[..n].iter().map(|b| (1.0 + b.volume).ln()).collect();
    let mu = log_vols.iter().copied().sum::<f64>() / n as f64;
    let variance = log_vols.iter().map(|v| (v - mu).powi(2)).sum::<f64>() / n as f64;
    let sigma = variance.sqrt().max(1e-8);
    VolStats {
        mu: mu as f32,
        sigma: sigma as f32,
    }
}

// ── Feature construction ──────────────────────────────────────────────────────

/// Construct the 5-feature vector for bar at index `t` (1-indexed into the
/// bars slice, so bars[t-1] is the previous bar).
///
/// Returns `[logret, logrange, logvol_z, hour_sin, hour_cos]`.
///
/// # Errors
///
/// Returns `FeatureError::Parse` if close prices are zero or negative.
pub(crate) fn bar_features(
    bars: &[RawBar],
    t: usize,
    vol_stats: &VolStats,
) -> Result<[f32; FEATURE_DIM], FeatureError> {
    debug_assert!(t > 0, "t must be >= 1 (logret needs bars[t-1])");
    let cur = &bars[t];
    let prev = &bars[t - 1];

    if prev.close <= 0.0 || cur.close <= 0.0 {
        return Err(FeatureError::Parse(format!(
            "non-positive close at t={t}: prev={}, cur={}",
            prev.close, cur.close
        )));
    }

    // logret = ln(close_t / close_{t-1})
    let logret = (cur.close / prev.close).ln() as f32;

    // logrange = ln(1 + (high - low) / close)
    let logrange = (1.0 + (cur.high - cur.low) / cur.close).ln() as f32;

    // logvol_z = (ln(1 + volume) - mu) / sigma
    let log_vol = (1.0 + cur.volume).ln() as f32;
    let logvol_z = (log_vol - vol_stats.mu) / vol_stats.sigma;

    // hour_sin / hour_cos on hour-of-week (0..167)
    let how = cur.hour_of_week() as f32;
    let phase = 2.0 * std::f32::consts::PI * how / 168.0;
    let hour_sin = phase.sin();
    let hour_cos = phase.cos();

    Ok([logret, logrange, logvol_z, hour_sin, hour_cos])
}

// ── load_bars_pub ─────────────────────────────────────────────────────────────

/// Public OHLCV bar type for callers outside the `forecast` crate (e.g. `train_garch`).
#[derive(Debug, Clone)]
pub struct OhlcvBarRaw {
    /// Bar open timestamp (Unix ms).
    pub open_time_ms: i64,
    /// Bar close timestamp (Unix ms).
    pub close_time_ms: i64,
    /// Open price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
    /// Volume.
    pub volume: f64,
}

/// Load raw OHLCV bars for a symbol, sorted by `open_time` ascending.
///
/// Public wrapper over the internal `load_bars` function.  Used by the
/// `train_garch` binary (and `vol_verdict` bin) to access bars directly
/// without going through the full `FeatureWindow` pipeline.
///
/// # Errors
///
/// See [`FeatureError`] variants.
pub fn load_bars_pub(
    parquet_root: &Path,
    symbol: &str,
    span: &TimeSpan,
) -> Result<Vec<OhlcvBarRaw>, FeatureError> {
    let raw = load_bars(parquet_root, symbol, span)?;
    Ok(raw
        .into_iter()
        .map(|b| OhlcvBarRaw {
            open_time_ms: b.open_time_ms,
            close_time_ms: b.close_time_ms,
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        })
        .collect())
}

// ── windows_for_symbol ────────────────────────────────────────────────────────

/// Return an iterator over `FeatureWindow` values for a single symbol.
///
/// Each window covers `cfg.context_bars` bars ending at bar `t`, with
/// target `r_{t+1} = ln(close_{t+1} / close_t)`. The first
/// `cfg.vol_z_lookback` bars are consumed as warm-up for volume-z
/// statistics and never appear in a window.
///
/// ## Parquet root layout
///
/// `<parquet_root>/<symbol>/<year>/<mm>.parquet`
///
/// ## Determinism
///
/// Given the same parquet files the iterator produces windows in identical
/// order and with identical float bytes across two runs (pure function, no
/// `SystemTime`, no RNG).
///
/// # Errors
///
/// Yields `Err(FeatureError)` if:
/// - The parquet root / symbol directory does not exist.
/// - The loaded bars are fewer than `vol_z_lookback + context_bars + 1`.
/// - A numeric value cannot be parsed.
pub fn windows_for_symbol(
    parquet_root: &Path,
    symbol: &str,
    span: TimeSpan,
    cfg: &FeatureConfig,
) -> impl Iterator<Item = Result<FeatureWindow, FeatureError>> {
    let parquet_root = parquet_root.to_owned();
    let symbol = symbol.to_owned();
    let context = cfg.context_bars;
    let warmup = cfg.vol_z_lookback;
    let horizon = cfg.target_horizon_bars;
    let vol_target_kind = cfg.vol_target_kind;

    // We need warmup + context + horizon bars (horizon extra for the target logret).
    // Default horizon=1 preserves the original TCN requirement (warmup + context + 1).
    let min_bars = warmup + context + horizon;

    // Load eagerly — bars are immutable once loaded; avoids lifetime issues.
    let bars_result = load_bars(&parquet_root, &symbol, &span);

    // Produce a single-shot iterator backed by a Vec.
    WindowIterator::new(
        bars_result,
        symbol,
        warmup,
        context,
        horizon,
        min_bars,
        vol_target_kind,
    )
}

/// Internal iterator state for `windows_for_symbol`.
struct WindowIterator {
    bars: Vec<RawBar>,
    symbol: String,
    vol_stats: VolStats,
    context: usize,
    /// Number of bars ahead for target log-return (default 1 for TCN; 24 for PatchTST).
    target_horizon_bars: usize,
    /// Optional realized-vol target kind (v3.0.0-volatility additive field).
    ///
    /// `None` → no Parkinson computation, `target_parkinson_vol` stays `None`.
    /// `Some(VolTargetKind::Parkinson)` → compute Parkinson σ over H bars.
    vol_target_kind: Option<VolTargetKind>,
    /// Current position of the window's *last bar* (index into `bars`).
    cursor: usize,
    /// Maximum valid cursor (so bars[cursor + target_horizon_bars] is the target).
    max_cursor: usize,
    /// Permanent error (e.g. file not found, insufficient bars).
    error: Option<FeatureError>,
}

impl WindowIterator {
    fn new(
        bars_result: Result<Vec<RawBar>, FeatureError>,
        symbol: String,
        warmup: usize,
        context: usize,
        target_horizon_bars: usize,
        min_bars: usize,
        vol_target_kind: Option<VolTargetKind>,
    ) -> Self {
        match bars_result {
            Err(e) => Self {
                bars: vec![],
                symbol,
                vol_stats: VolStats {
                    mu: 0.0,
                    sigma: 1.0,
                },
                context,
                target_horizon_bars,
                vol_target_kind,
                // cursor > max_cursor ensures we stop after yielding the error.
                cursor: 1,
                max_cursor: 0,
                error: Some(e),
            },
            Ok(bars) => {
                if bars.len() < min_bars {
                    let got = bars.len();
                    return Self {
                        bars: vec![],
                        symbol,
                        vol_stats: VolStats {
                            mu: 0.0,
                            sigma: 1.0,
                        },
                        context,
                        target_horizon_bars,
                        vol_target_kind,
                        // cursor > max_cursor ensures we stop after yielding the error.
                        cursor: 1,
                        max_cursor: 0,
                        error: Some(FeatureError::InsufficientBars {
                            need: min_bars,
                            got,
                        }),
                    };
                }

                let vol_stats = compute_vol_stats(&bars, warmup);

                // First window ends at index `warmup + context - 1`;
                // target is at `warmup + context + (horizon - 1)`.
                let cursor_start = warmup + context - 1;
                // Last window's last bar: bars[cursor + target_horizon_bars] must be valid.
                // So max_cursor = bars.len() - 1 - target_horizon_bars.
                let max_cursor = bars.len() - 1 - target_horizon_bars;

                Self {
                    bars,
                    symbol,
                    vol_stats,
                    context,
                    target_horizon_bars,
                    vol_target_kind,
                    cursor: cursor_start,
                    max_cursor,
                    error: None,
                }
            }
        }
    }
}

impl Iterator for WindowIterator {
    type Item = Result<FeatureWindow, FeatureError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Yield the permanent error once.
        if let Some(e) = self.error.take() {
            return Some(Err(e));
        }

        if self.cursor > self.max_cursor {
            return None;
        }

        // Window covers bars[cursor - context + 1 ..= cursor].
        let window_start = self.cursor + 1 - self.context; // inclusive
        let window_end = self.cursor; // inclusive last bar of context

        // Build feature matrix: shape [context, FEATURE_DIM].
        // `bar_features(bars, t, stats)` computes features for bars[t] using bars[t-1].
        // We need t = window_start (needs bars[window_start - 1]) through t = window_end.
        // window_start >= warmup + context - (context - 1) = warmup, so window_start >= warmup.
        // warmup >= 1 so window_start - 1 is valid.
        let mut feat_matrix: Vec<f32> = Vec::with_capacity(self.context * FEATURE_DIM);

        for t in window_start..=window_end {
            match bar_features(&self.bars, t, &self.vol_stats) {
                Ok(f) => feat_matrix.extend_from_slice(&f),
                Err(e) => {
                    self.cursor = self.max_cursor + 1; // stop iteration
                    return Some(Err(e));
                }
            }
        }

        // Target: r_{t+h} = ln(close_{t+h} / close_t), where h = target_horizon_bars.
        // Default h=1 preserves TCN byte-compatibility.
        let t_next = window_end + self.target_horizon_bars;
        let close_t = self.bars[window_end].close;
        let close_t1 = self.bars[t_next].close;
        let target_logret = if close_t > 0.0 && close_t1 > 0.0 {
            (close_t1 / close_t).ln() as f32
        } else {
            warn!(
                symbol = self.symbol,
                cursor = self.cursor,
                "non-positive close for target; using 0.0"
            );
            0.0_f32
        };

        // Parkinson realized-vol target over bars[window_end+1 ..= window_end+H].
        // Locked formula (ADR-0038 § D3 / T-AR-3):
        //   σ̂_P = sqrt( (1 / (4·ln 2)) · mean_k(ln(high_k/low_k)²) )
        // Only computed when vol_target_kind == Some(Parkinson).
        // None for TCN/PatchTST callers — anchor-additive-zero contract (R11.7/R11.8).
        let target_parkinson_vol: Option<f32> = match self.vol_target_kind {
            Some(VolTargetKind::Parkinson) => {
                let h = self.target_horizon_bars;
                let mut sum_sq = 0.0_f64;
                for k in 1..=h {
                    let bar = &self.bars[window_end + k];
                    if bar.high > 0.0 && bar.low > 0.0 && bar.high >= bar.low {
                        let ln_hl = (bar.high / bar.low).ln();
                        sum_sq += ln_hl * ln_hl;
                    } else {
                        warn!(
                            symbol = self.symbol,
                            cursor = self.cursor,
                            k,
                            "invalid high/low for Parkinson target; treating as 0 contribution"
                        );
                    }
                }
                // σ̂_P = sqrt( (1/(4·ln2)) · (sum_sq / H) )
                let parkinson_var = (1.0 / (4.0 * f64::ln(2.0))) * (sum_sq / h as f64);
                Some(parkinson_var.sqrt() as f32)
            }
            Some(VolTargetKind::RealizedVol) => {
                unimplemented!("VolTargetKind::RealizedVol not implemented until v0.1.1")
            }
            None => None,
        };

        let bar_close_ts = self.bars[window_end].close_ts();
        let vol_stats = self.vol_stats.clone();
        let symbol = self.symbol.clone();

        // Build the feature tensor.
        #[cfg(feature = "candle")]
        let features = {
            // Shape [context, FEATURE_DIM] on CPU.
            match Tensor::from_vec(feat_matrix, (self.context, FEATURE_DIM), &Device::Cpu) {
                Ok(t) => t,
                Err(e) => {
                    self.cursor = self.max_cursor + 1;
                    return Some(Err(FeatureError::Tensor(e.to_string())));
                }
            }
        };

        #[cfg(not(feature = "candle"))]
        let features = feat_matrix;

        self.cursor += 1;

        Some(Ok(FeatureWindow {
            features,
            target_logret,
            target_parkinson_vol,
            symbol,
            bar_close_ts,
            vol_stats,
        }))
    }
}

// ── aligned_batches ───────────────────────────────────────────────────────────

/// Batch multiple per-symbol iterators, grouping windows by `bar_close_ts`.
///
/// Uses `itertools::kmerge_by` to merge `n` per-symbol iterators sorted by
/// `bar_close_ts` ascending. Each call to `next()` returns a `Vec<FeatureWindow>`
/// where all elements share the same `bar_close_ts` (a "macro-step").
///
/// ## Properties
///
/// - Round-robin-by-timestamp: windows from different symbols at the same
///   timestamp form one batch before any symbol advances.
/// - Timestamp-sort invariant: the same set of windows in any input order
///   produces the same batch sequence (because merge is by timestamp key).
///
/// ## Errors
///
/// If any iterator yields `Err(FeatureError)`, the error is propagated as the
/// batch `Err` and iteration stops.
pub fn aligned_batches<I>(
    iters: Vec<I>,
) -> impl Iterator<Item = Result<Vec<FeatureWindow>, FeatureError>>
where
    I: Iterator<Item = Result<FeatureWindow, FeatureError>>,
{
    AlignedBatchIterator::new(iters)
}

/// Internal state for `aligned_batches`.
struct AlignedBatchIterator<I>
where
    I: Iterator<Item = Result<FeatureWindow, FeatureError>>,
{
    /// Peekable per-symbol iterators.
    iters: Vec<std::iter::Peekable<I>>,
    done: bool,
}

impl<I> AlignedBatchIterator<I>
where
    I: Iterator<Item = Result<FeatureWindow, FeatureError>>,
{
    fn new(iters: Vec<I>) -> Self {
        Self {
            iters: iters.into_iter().map(|it| it.peekable()).collect(),
            done: false,
        }
    }
}

impl<I> Iterator for AlignedBatchIterator<I>
where
    I: Iterator<Item = Result<FeatureWindow, FeatureError>>,
{
    type Item = Result<Vec<FeatureWindow>, FeatureError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Find the minimum bar_close_ts across all iterators that still have items.
        let mut min_ts: Option<OffsetDateTime> = None;

        for iter in &mut self.iters {
            match iter.peek() {
                None => {} // exhausted
                Some(Err(_)) => {
                    // Propagate the error.
                    let err = iter.next().unwrap().unwrap_err();
                    self.done = true;
                    return Some(Err(err));
                }
                Some(Ok(w)) => {
                    let ts = w.bar_close_ts;
                    match min_ts {
                        None => min_ts = Some(ts),
                        Some(m) if ts < m => min_ts = Some(ts),
                        _ => {}
                    }
                }
            }
        }

        let Some(target_ts) = min_ts else {
            return None; // all exhausted
        };

        // Collect all windows at target_ts across all iterators.
        let mut batch: Vec<FeatureWindow> = Vec::new();
        for iter in &mut self.iters {
            // Drain windows at target_ts from this iterator.
            loop {
                match iter.peek() {
                    None => break,
                    // Only propagate Err if it is at target_ts boundary.
                    // Errors are yielded on the NEXT macro-step to ensure
                    // the current batch completes cleanly.
                    Some(Err(_)) => break,
                    Some(Ok(w)) if w.bar_close_ts == target_ts => {
                        let window = iter.next().unwrap().unwrap();
                        batch.push(window);
                    }
                    _ => break,
                }
            }
        }

        // Sort batch by symbol for deterministic ordering.
        batch.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        if batch.is_empty() {
            None
        } else {
            Some(Ok(batch))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    // ── Unit tests ────────────────────────────────────────────────────────────

    /// VolStats computation on a trivial sequence.
    #[test]
    fn vol_stats_simple() {
        let bars: Vec<RawBar> = (0..5)
            .map(|i| RawBar {
                open_time_ms: i * 3_600_000,
                close_time_ms: (i + 1) * 3_600_000 - 1,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: (i as f64 + 1.0) * 1000.0,
            })
            .collect();
        let stats = compute_vol_stats(&bars, 5);
        assert!(stats.mu > 0.0, "mu should be positive");
        assert!(stats.sigma > 0.0, "sigma should be positive");
    }

    /// `bar_features` on a trivial pair of bars.
    #[test]
    fn bar_features_trivial() {
        let bars = vec![
            RawBar {
                open_time_ms: 0,
                close_time_ms: 3_599_999,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1000.0,
            },
            RawBar {
                open_time_ms: 3_600_000,
                close_time_ms: 7_199_999,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 2000.0,
            },
        ];
        let stats = VolStats {
            mu: 7.0,
            sigma: 1.0,
        };
        let f = bar_features(&bars, 1, &stats).unwrap();
        assert_eq!(f.len(), FEATURE_DIM);
        // logret = ln(101/100) ≈ 0.00995
        let expected_logret = (101.0_f64 / 100.0).ln() as f32;
        assert!((f[0] - expected_logret).abs() < 1e-5, "logret mismatch");
    }

    /// TimeSpan rejects degenerate bounds in debug builds.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn timespan_panics_on_reversed_bounds() {
        let _ = TimeSpan::new(
            datetime!(2023-12-31 00:00 UTC),
            datetime!(2023-01-01 00:00 UTC),
        );
    }

    /// `windows_for_symbol` on a missing directory yields one `Err`.
    #[test]
    fn windows_for_symbol_missing_dir_yields_err() {
        let tmpdir = tempfile::tempdir().unwrap();
        let cfg = FeatureConfig::default();
        let span = TimeSpan::new(
            datetime!(2023-01-01 00:00 UTC),
            datetime!(2023-12-31 00:00 UTC),
        );
        let results: Vec<_> =
            windows_for_symbol(tmpdir.path(), "NONEXISTENT", span, &cfg).collect();
        assert_eq!(
            results.len(),
            1,
            "should yield exactly one item (the error)"
        );
        assert!(results[0].is_err(), "should be an error");
    }

    /// FeatureConfig defaults match spec values.
    #[test]
    fn feature_config_defaults() {
        let cfg = FeatureConfig::default();
        assert_eq!(cfg.context_bars, 256);
        assert_eq!(cfg.vol_z_lookback, 720);
        assert!((cfg.direction_epsilon - 0.0005).abs() < 1e-10);
    }

    // ── aligned_batches tests ─────────────────────────────────────────────────

    fn make_window(symbol: &str, ts_secs: i64) -> FeatureWindow {
        let ts = OffsetDateTime::from_unix_timestamp(ts_secs).unwrap();
        FeatureWindow {
            #[cfg(feature = "candle")]
            features: Tensor::zeros((256, 5), candle_core::DType::F32, &Device::Cpu).unwrap(),
            #[cfg(not(feature = "candle"))]
            features: vec![0.0_f32; 256 * 5],
            target_logret: 0.0,
            target_parkinson_vol: None,
            symbol: symbol.to_string(),
            bar_close_ts: ts,
            vol_stats: VolStats {
                mu: 0.0,
                sigma: 1.0,
            },
        }
    }

    /// Single iterator yields batches of size 1.
    #[test]
    fn aligned_batches_single_iter() {
        let base_ts = 1_700_000_000_i64;
        let windows = vec![
            Ok(make_window("BTC", base_ts)),
            Ok(make_window("BTC", base_ts + 3600)),
            Ok(make_window("BTC", base_ts + 7200)),
        ];
        let iter = windows.into_iter();
        let batches: Vec<_> = aligned_batches(vec![iter]).collect();
        assert_eq!(batches.len(), 3);
        for b in &batches {
            assert!(b.is_ok());
            assert_eq!(b.as_ref().unwrap().len(), 1);
        }
    }

    /// Two iterators at the same timestamps batch together.
    #[test]
    fn aligned_batches_two_symbols_same_ts() {
        let base_ts = 1_700_000_000_i64;
        let btc = vec![
            Ok(make_window("BTC", base_ts)),
            Ok(make_window("BTC", base_ts + 3600)),
        ]
        .into_iter();
        let eth = vec![
            Ok(make_window("ETH", base_ts)),
            Ok(make_window("ETH", base_ts + 3600)),
        ]
        .into_iter();

        let batches: Vec<_> = aligned_batches(vec![btc, eth]).collect();
        assert_eq!(batches.len(), 2, "two macro-steps");
        for b in &batches {
            let batch = b.as_ref().unwrap();
            assert_eq!(batch.len(), 2, "both symbols in each macro-step");
            // All windows in a batch share the same bar_close_ts.
            let ts0 = batch[0].bar_close_ts;
            assert!(batch.iter().all(|w| w.bar_close_ts == ts0));
        }
    }

    /// Shuffled-symbol order produces the same batch sequence (timestamp-sort invariant).
    #[test]
    fn aligned_batches_timestamp_sort_invariant() {
        let base_ts = 1_700_000_000_i64;

        // Order 1: BTC first
        let btc_a = vec![
            Ok(make_window("BTC", base_ts)),
            Ok(make_window("BTC", base_ts + 3600)),
        ]
        .into_iter();
        let eth_a = vec![
            Ok(make_window("ETH", base_ts)),
            Ok(make_window("ETH", base_ts + 3600)),
        ]
        .into_iter();
        let batches_a: Vec<_> = aligned_batches(vec![btc_a, eth_a]).collect();

        // Order 2: ETH first (shuffled)
        let eth_b = vec![
            Ok(make_window("ETH", base_ts)),
            Ok(make_window("ETH", base_ts + 3600)),
        ]
        .into_iter();
        let btc_b = vec![
            Ok(make_window("BTC", base_ts)),
            Ok(make_window("BTC", base_ts + 3600)),
        ]
        .into_iter();
        let batches_b: Vec<_> = aligned_batches(vec![eth_b, btc_b]).collect();

        assert_eq!(batches_a.len(), batches_b.len());
        for (ba, bb) in batches_a.iter().zip(batches_b.iter()) {
            let wa = ba.as_ref().unwrap();
            let wb = bb.as_ref().unwrap();
            assert_eq!(wa.len(), wb.len());
            for (a, b) in wa.iter().zip(wb.iter()) {
                assert_eq!(a.symbol, b.symbol, "symbol order must be identical");
                assert_eq!(a.bar_close_ts, b.bar_close_ts, "timestamps must match");
            }
        }
    }

    /// Error propagation from inner iterator.
    #[test]
    fn aligned_batches_propagates_error() {
        let base_ts = 1_700_000_000_i64;
        let err_iter = vec![
            Ok(make_window("ERR", base_ts)),
            Err(FeatureError::Parse("boom".to_string())),
        ]
        .into_iter();
        let btc = vec![
            Ok(make_window("BTC", base_ts)),
            Ok(make_window("BTC", base_ts + 3600)),
        ]
        .into_iter();

        let batches: Vec<_> = aligned_batches(vec![err_iter, btc]).collect();
        // First batch succeeds (ts == base_ts from both).
        assert!(batches[0].is_ok());
        // Second batch propagates the error.
        assert!(batches[1].is_err());
    }

    // ── Determinism property test (no proptest dependency needed for this) ────

    /// T-D-N9: `target_horizon_bars` defaults to 1 (TCN byte-compatibility).
    ///
    /// Verifies that:
    /// 1. `FeatureConfig::default().target_horizon_bars == 1`
    /// 2. An explicit `target_horizon_bars: 1` config produces the same window
    ///    structure as the default (same `min_bars` requirement).
    #[test]
    fn target_horizon_bars_default_1_unchanged_tcn() {
        // Default config must have target_horizon_bars = 1 (TCN compat).
        let default_cfg = FeatureConfig::default();
        assert_eq!(
            default_cfg.target_horizon_bars, 1,
            "FeatureConfig::default().target_horizon_bars must be 1 for TCN byte-compatibility"
        );

        // Explicit horizon=1 config equals the default structurally.
        let explicit_cfg = FeatureConfig {
            target_horizon_bars: 1,
            ..FeatureConfig::default()
        };
        assert_eq!(explicit_cfg.target_horizon_bars, 1);
        assert_eq!(explicit_cfg.context_bars, default_cfg.context_bars);
        assert_eq!(explicit_cfg.vol_z_lookback, default_cfg.vol_z_lookback);

        // PatchTST horizon=24 config differs only in target_horizon_bars.
        let patchtst_cfg = FeatureConfig {
            context_bars: 336,
            vol_z_lookback: 720,
            direction_epsilon: 0.0005,
            target_horizon_bars: 24,
            vol_target_kind: None,
        };
        assert_eq!(patchtst_cfg.target_horizon_bars, 24);

        // min_bars for default (TCN): warmup + context + 1 = 720 + 256 + 1 = 977.
        let tcn_min_bars =
            default_cfg.vol_z_lookback + default_cfg.context_bars + default_cfg.target_horizon_bars;
        assert_eq!(tcn_min_bars, 977, "TCN min_bars must be 977");

        // min_bars for PatchTST: 720 + 336 + 24 = 1080.
        let patchtst_min_bars = patchtst_cfg.vol_z_lookback
            + patchtst_cfg.context_bars
            + patchtst_cfg.target_horizon_bars;
        assert_eq!(patchtst_min_bars, 1080, "PatchTST min_bars must be 1080");
    }

    /// Same parquet path → same window sequence on two calls (determinism smoke).
    ///
    /// This test reads the real BTCUSDT data if available; skips gracefully if not.
    #[test]
    fn windows_determinism_on_real_data() {
        use std::env;
        let root = env::var("CARGO_MANIFEST_DIR")
            .map(|d| std::path::PathBuf::from(d).join("../../data/binance"))
            .unwrap_or_else(|_| std::path::PathBuf::from("data/binance"));

        if !root.exists() {
            eprintln!("SKIP: data/binance not found");
            return;
        }

        let span = TimeSpan::new(
            datetime!(2023-01-01 00:00 UTC),
            datetime!(2023-03-01 00:00 UTC),
        );
        let cfg = FeatureConfig::default();

        let run1: Vec<_> = windows_for_symbol(&root, "BTCUSDT", span.clone(), &cfg)
            .take(10)
            .collect();
        let run2: Vec<_> = windows_for_symbol(&root, "BTCUSDT", span, &cfg)
            .take(10)
            .collect();

        assert_eq!(run1.len(), run2.len(), "window counts must match");
        for (w1, w2) in run1.iter().zip(run2.iter()) {
            match (w1, w2) {
                (Ok(a), Ok(b)) => {
                    assert_eq!(a.target_logret, b.target_logret, "target_logret must match");
                    assert_eq!(a.bar_close_ts, b.bar_close_ts, "bar_close_ts must match");
                    assert_eq!(a.symbol, b.symbol, "symbol must match");
                    // Feature bytes must be identical.
                    #[cfg(not(feature = "candle"))]
                    assert_eq!(a.features, b.features, "feature bytes must be identical");
                }
                _ => panic!("expected Ok windows"),
            }
        }
    }
}
