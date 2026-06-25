//! Gate-tied hyperparameter sweep engine (ADR-0069).
//!
//! # Overview
//!
//! `run_param_sweep(cfg, cancel_rx, progress_tx, sweep_progress_tx)` is the
//! library entry point for the operator's parameter-grid editor. It is the
//! bake-off's sister: where `run_bakeoff` loops over N *strategy ids*, the sweep
//! loops over N *parameterised configs of ONE family* and scores each through the
//! **identical** frozen robustness gate (`classify_verdict` + the 5-signal
//! weakest-link composite).
//!
//! # Current scope: SMA family only (T1–T5)
//!
//! MACD / RSI / Bollinger families are stubs guarded by `// T7` markers.
//! They are NOT implemented here — see `build_swept_strategy` for the stubs.
//!
//! # Anchor safety (ADR-0069 D9)
//!
//! Every cell runs `write_report = false` → no anchored report body is written →
//! `verify_anchors.sh` stays 119/119. The gate bands + bootstrap seed rule are
//! frozen (T1 delegation proof).
//!
//! # Grid cap (ADR-0069 D4)
//!
//! `MAX_SWEEP_CONFIGS = 24` is the single source of truth. The UI reads this
//! constant for the live "N configs → …" readout. Truncation is honest
//! (`SweepRequestEcho.truncated = true` + `requested_count`).

#![allow(clippy::float_arithmetic)] // statistical metric computations in KPI derivation

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

use crate::{
    ScenarioConfig,
    cancel::RunCancelReceiver,
    engine::{DateRange, RunError, ScenarioDataSource, run_scenario},
    progress::{BakeoffProgress, BakeoffProgressSender, ProgressSender},
    stats::DistributionSummary,
};

use super::{
    CandidateKpis,
    bootstrap::{compute_robustness_distribution, derive_master_seed},
    derive_candidate_kpis, resolve_bakeoff_bars,
    robustness::ParamRobustnessVerdict,
};
use crate::stats::MetricDistribution;

// ── Grid cap (ADR-0069 D4, single source of truth) ───────────────────────────

/// Maximum number of sweep configurations per run.
///
/// The UI reads this constant for the live "N configs → ~M bootstrap runs (~T)"
/// readout. This is the single source of truth — do NOT hardcode 24 elsewhere.
///
/// Cost model: each config = 1 `run_scenario` + 1000 bootstrap paths.
/// 24 configs ≈ 2× the 13-arm bake-off — the ceiling of "still an interactive
/// click" on the determinate-progress on-demand path.
pub const MAX_SWEEP_CONFIGS: usize = 24;

/// SMA strategy id used for every sweep cell (the strategy whose fast/slow
/// window parameters are overridden by the sweep grid).
const SMA_STRATEGY_ID: &str = "v0.sma";

/// Buy-and-hold benchmark strategy id (mirrors `BUYHOLD_ID` in `bakeoff/mod.rs`).
const BUYHOLD_ID: &str = "v0.buyhold";

// ── Strategy families ─────────────────────────────────────────────────────────

/// One strategy family that can be swept. Closed enum — no string parsing.
///
/// Only `Sma` is implemented in T1–T5. The other arms are `// T7` stubs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepFamily {
    /// SMA crossover (`fast_len` / `slow_len`). EXISTS: `ScenarioConfig.sma_fast_len/slow_len`.
    Sma,
    /// MACD (fast / slow / signal). T7 — string-generation builder not yet built.
    Macd,
    /// RSI (period / oversold). T7 — string-generation builder not yet built.
    Rsi,
    /// Bollinger (period / k). T7 — string-generation builder not yet built.
    Bollinger,
}

impl SweepFamily {
    /// Human-readable label for display in the UI and progress copy.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Sma => "SMA crossover",
            Self::Macd => "MACD",
            Self::Rsi => "RSI",
            Self::Bollinger => "Bollinger bands",
        }
    }
}

// ── Axis and grid types ───────────────────────────────────────────────────────

/// One parameter axis: inclusive `{min, max, step}` integer range.
///
/// Enumerates `min, min+step, min+2*step, … ≤ max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepAxis {
    /// Minimum value (inclusive).
    pub min: u32,
    /// Maximum value (inclusive).
    pub max: u32,
    /// Step size (must be ≥ 1).
    pub step: u32,
}

impl SweepAxis {
    /// Enumerate the values this axis produces.
    ///
    /// Returns `(min, min+step, …, ≤ max)`. If `step == 0`, treated as 1.
    #[must_use]
    pub fn values(self) -> Vec<u32> {
        let step = self.step.max(1);
        let mut v = Vec::new();
        let mut x = self.min;
        while x <= self.max {
            v.push(x);
            x = x.saturating_add(step);
        }
        v
    }

    /// The shipped/default SMA fast-len axis centred on 20.
    #[must_use]
    pub fn sma_fast_default() -> Self {
        Self {
            min: 10,
            max: 30,
            step: 5,
        }
    }

    /// The shipped/default SMA slow-len axis centred on 50.
    #[must_use]
    pub fn sma_slow_default() -> Self {
        Self {
            min: 30,
            max: 70,
            step: 10,
        }
    }
}

/// SMA-family grid: `fast_len` axis × `slow_len` axis.
///
/// The two integer axes enumerate a cartesian product; cells where
/// `fast_len >= slow_len` are DROPPED pre-run (not silent — reported in
/// `SweepRequestEcho.invalid_count`).
#[derive(Debug, Clone)]
pub struct SmaGrid {
    /// Fast-window axis (min/max/step). Constraint: `1 ≤ fast < slow ≤ 400`.
    pub fast_len: SweepAxis,
    /// Slow-window axis (min/max/step). Constraint: `1 ≤ fast < slow ≤ 400`.
    pub slow_len: SweepAxis,
}

impl Default for SmaGrid {
    /// Default shipped grid (narrow, centred on the shipped config fast=20, slow=50).
    fn default() -> Self {
        Self {
            fast_len: SweepAxis::sma_fast_default(),
            slow_len: SweepAxis::sma_slow_default(),
        }
    }
}

impl SmaGrid {
    /// Enumerate the cartesian product (fast, slow) in axis-major order (fast outer,
    /// slow inner), dropping cells where `fast >= slow` or out of range.
    ///
    /// Returns `(all_cells_count, valid_cells)` where `all_cells_count` is the
    /// unconstrained cartesian product size (for truncation reporting) and
    /// `valid_cells` is the validated list before the cap is applied.
    ///
    /// Guard: `1 ≤ fast < slow ≤ 400`.
    #[must_use]
    pub fn enumerate_valid(&self) -> (usize, Vec<(u32, u32)>) {
        let fast_vals = self.fast_len.values();
        let slow_vals = self.slow_len.values();
        let total_unconstrained = fast_vals.len() * slow_vals.len();
        let valid: Vec<(u32, u32)> = fast_vals
            .iter()
            .flat_map(|&f| slow_vals.iter().map(move |&s| (f, s)))
            .filter(|&(f, s)| f >= 1 && f < s && s <= 400)
            .collect();
        (total_unconstrained, valid)
    }
}

/// MACD-family grid: `fast_period` × `slow_period` × `signal_period` axes.
///
/// Grid constraint: `fast < slow` (same as SMA family's ordering rule).
/// Signal axis is independent.
#[derive(Debug, Clone)]
pub struct MacdGrid {
    /// Fast EMA period axis. Shipped default: centred on 12.
    pub fast: SweepAxis,
    /// Slow EMA period axis. Shipped default: centred on 26.
    pub slow: SweepAxis,
    /// Signal (smoothing) period axis. Shipped default: centred on 9.
    pub signal: SweepAxis,
}

impl Default for MacdGrid {
    /// Narrow grid centred on the shipped MACD config (12, 26, 9).
    fn default() -> Self {
        Self {
            fast: SweepAxis {
                min: 8,
                max: 16,
                step: 4,
            }, // [8, 12, 16]
            slow: SweepAxis {
                min: 20,
                max: 32,
                step: 6,
            }, // [20, 26, 32]
            signal: SweepAxis {
                min: 7,
                max: 11,
                step: 2,
            }, // [7, 9, 11]
        }
    }
}

impl MacdGrid {
    /// Enumerate valid `(fast, slow, signal)` triples: `fast < slow`, all ≥ 1.
    ///
    /// Returns `(unconstrained_count, valid_triples)`.
    #[must_use]
    pub fn enumerate_valid(&self) -> (usize, Vec<(u32, u32, u32)>) {
        let fast_vals = self.fast.values();
        let slow_vals = self.slow.values();
        let sig_vals = self.signal.values();
        let unconstrained = fast_vals.len() * slow_vals.len() * sig_vals.len();
        // Use a direct triple loop to avoid the borrow/move clash in nested closures.
        let mut valid: Vec<(u32, u32, u32)> = Vec::new();
        for &f in &fast_vals {
            for &s in &slow_vals {
                for &sg in &sig_vals {
                    if f >= 1 && f < s && s <= 400 && sg >= 1 {
                        valid.push((f, s, sg));
                    }
                }
            }
        }
        (unconstrained, valid)
    }
}

/// RSI-family grid: `period` × `oversold_threshold` axes.
///
/// `oversold` is swept as a discrete integer threshold (e.g. 25, 30, 35).
#[derive(Debug, Clone)]
pub struct RsiGrid {
    /// RSI lookback period axis. Shipped default: centred on 14.
    pub period: SweepAxis,
    /// Oversold threshold axis (integer, compared as `rsi < oversold`).
    /// Shipped default: centred on 30.
    pub oversold: SweepAxis,
}

impl Default for RsiGrid {
    /// Narrow grid centred on the shipped RSI config (14, 30).
    fn default() -> Self {
        Self {
            period: SweepAxis {
                min: 10,
                max: 18,
                step: 4,
            }, // [10, 14, 18]
            oversold: SweepAxis {
                min: 25,
                max: 35,
                step: 5,
            }, // [25, 30, 35]
        }
    }
}

impl RsiGrid {
    /// Enumerate all valid `(period, oversold)` pairs.
    ///
    /// Constraint: `period ≥ 2`, `1 ≤ oversold ≤ 49` (oversold never ≥ 50).
    /// Returns `(unconstrained_count, valid_pairs)`.
    #[must_use]
    pub fn enumerate_valid(&self) -> (usize, Vec<(u32, u32)>) {
        let period_vals = self.period.values();
        let os_vals = self.oversold.values();
        let unconstrained = period_vals.len() * os_vals.len();
        let valid: Vec<(u32, u32)> = period_vals
            .iter()
            .flat_map(|&p| {
                os_vals.iter().filter_map(move |&os| {
                    if p >= 2 && (1..=49).contains(&os) {
                        Some((p, os))
                    } else {
                        None
                    }
                })
            })
            .collect();
        (unconstrained, valid)
    }
}

/// Bollinger-family grid: `period` × discrete `k` presets.
///
/// `k` is NOT a `SweepAxis` because floating-point step arithmetic on Decimal
/// would introduce drift. Instead we use a preset list:
/// `{1.5, 2.0, 2.5, 3.0}` — Decimal-exact, no rounding.
#[derive(Debug, Clone)]
pub struct BollingerGrid {
    /// Lookback period axis. Shipped default: centred on 20.
    pub period: SweepAxis,
    /// Discrete band-multiplier presets (`k` in `bollinger_lower(period, k)`).
    ///
    /// Preset list avoids float-step drift. Default: `{1.5, 2.0, 2.5, 3.0}`.
    pub k_presets: Vec<rust_decimal::Decimal>,
}

impl Default for BollingerGrid {
    /// Grid centred on the shipped `BBands` config (`period=20`, `k=2.0`).
    fn default() -> Self {
        use rust_decimal_macros::dec;
        Self {
            period: SweepAxis {
                min: 14,
                max: 26,
                step: 6,
            }, // [14, 20, 26]
            k_presets: vec![dec!(1.5), dec!(2.0), dec!(2.5), dec!(3.0)],
        }
    }
}

impl BollingerGrid {
    /// Enumerate all valid `(period, k)` pairs.
    ///
    /// Constraint: `period ≥ 2`, `k > 0`.
    /// Returns `(unconstrained_count, valid_pairs)`.
    #[must_use]
    pub fn enumerate_valid(&self) -> (usize, Vec<(u32, rust_decimal::Decimal)>) {
        let period_vals = self.period.values();
        let unconstrained = period_vals.len() * self.k_presets.len();
        let valid: Vec<(u32, rust_decimal::Decimal)> = period_vals
            .iter()
            .flat_map(|&p| {
                self.k_presets.iter().filter_map(move |&k| {
                    if p >= 2 && k > rust_decimal::Decimal::ZERO {
                        Some((p, k))
                    } else {
                        None
                    }
                })
            })
            .collect();
        (unconstrained, valid)
    }
}

/// Family-specific grid specification (closed enum, one variant per family).
#[derive(Debug, Clone)]
pub enum SweepGrid {
    /// SMA crossover grid.
    Sma(SmaGrid),
    /// MACD grid.
    Macd(MacdGrid),
    /// RSI grid.
    Rsi(RsiGrid),
    /// Bollinger bands grid.
    Bollinger(BollingerGrid),
}

impl SweepGrid {
    /// The family this grid belongs to.
    #[must_use]
    pub fn family(&self) -> SweepFamily {
        match self {
            Self::Sma(_) => SweepFamily::Sma,
            Self::Macd(_) => SweepFamily::Macd,
            Self::Rsi(_) => SweepFamily::Rsi,
            Self::Bollinger(_) => SweepFamily::Bollinger,
        }
    }
}

// ── Concrete per-cell params ──────────────────────────────────────────────────

/// The concrete params for one swept cell (identity + display).
///
/// Carries enough information to reconstruct the strategy AND label the UI row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SweptParams {
    /// SMA crossover: fast and slow window lengths.
    Sma {
        /// Fast window (periods).
        fast_len: u32,
        /// Slow window (periods).
        slow_len: u32,
    },
    /// MACD: fast EMA period / slow EMA period / signal smoothing period.
    Macd {
        /// Fast EMA period.
        fast: u32,
        /// Slow EMA period.
        slow: u32,
        /// Signal line smoothing period.
        signal: u32,
    },
    /// RSI: lookback period / oversold threshold.
    Rsi {
        /// RSI lookback period.
        period: u32,
        /// Oversold threshold (entry fires when `rsi < oversold`).
        oversold: u32,
    },
    /// Bollinger bands: lookback period / band-multiplier k.
    Bollinger {
        /// Bollinger lookback period.
        period: u32,
        /// Band multiplier (k in `bollinger_lower(period, k)`).
        k: Decimal,
    },
}

impl SweptParams {
    /// A short human-readable label for the progress bar / UI row.
    #[must_use]
    pub fn label(&self) -> SmolStr {
        match self {
            Self::Sma { fast_len, slow_len } => {
                SmolStr::new(format!("fast={fast_len}, slow={slow_len}"))
            }
            Self::Macd { fast, slow, signal } => {
                SmolStr::new(format!("macd({fast},{slow},{signal})"))
            }
            Self::Rsi { period, oversold } => SmolStr::new(format!("rsi({period})<{oversold}")),
            Self::Bollinger { period, k } => SmolStr::new(format!("bb({period},{k})")),
        }
    }

    /// Whether these params match the shipped/default SMA config (fast=20, slow=50).
    #[must_use]
    pub fn is_sma_shipped_default(&self) -> bool {
        matches!(
            self,
            Self::Sma {
                fast_len: 20,
                slow_len: 50
            }
        )
    }
}

// ── Sweep configuration ───────────────────────────────────────────────────────

/// Configuration for one sweep run — mirrors `BakeoffConfig`.
#[derive(Debug, Clone)]
pub struct SweepConfig {
    /// Strategy family to sweep.
    pub family: SweepFamily,
    /// Parameter grid for the family.
    pub grid: SweepGrid,
    /// The single coin to sweep on.
    pub symbol: Symbol,
    /// Backtest date range (same range for every cell).
    pub range: DateRange,
    /// Mandatory `ChaCha20` seed (32-byte, same seed for every cell → apples-to-apples).
    pub seed: [u8; 32],
    /// Data source (default `BinanceCache` in production; `Synthetic` in tests).
    pub data_source: ScenarioDataSource,
    /// Number of bootstrap paths per cell (default 1000 per ADR-0063 § D4).
    pub paths: usize,
}

// ── Request echo (in the report) ─────────────────────────────────────────────

/// Echo of the sweep request — in `SweepReport` for reproducibility.
#[derive(Debug, Clone)]
pub struct SweepRequestEcho {
    /// Family name (display).
    pub family_label: SmolStr,
    /// Coin, e.g. `"BTCUSDT"`.
    pub coin: SmolStr,
    /// Human-readable range label, e.g. `"2024 H1"`.
    pub range_label: SmolStr,
    /// Number of valid cells actually run (post-cap, post-invalid-drop).
    pub grid_size: usize,
    /// Whether the cartesian product was truncated to `MAX_SWEEP_CONFIGS`.
    pub truncated: bool,
    /// The full requested count (before cap), including invalid cells.
    pub requested_count: usize,
    /// Number of cells dropped because `fast >= slow` (or other validity failures).
    pub invalid_count: usize,
}

// ── Per-cell result ───────────────────────────────────────────────────────────

/// One swept config's outcome — the sweep analogue of `CandidateResult`,
/// but carrying the FULL bootstrap distribution (R3), not just the flag.
#[derive(Debug, Clone)]
pub struct SweepCellResult {
    /// The concrete params for this cell (display + identity).
    pub params: SweptParams,
    /// In-sample KPIs from the single realized run (the point estimate).
    pub kpis: CandidateKpis,
    /// The bootstrap verdict (Robust/Marginal/Fragile) — the SAME gate.
    pub verdict: ParamRobustnessVerdict,
    /// The bootstrap distribution summary (p5/p50/p95 Sharpe, `prob_loss`,
    /// P(Sharpe>1), p95 `MaxDD`) — the five gate signals, surfaced (R3).
    pub distribution: DistributionSummary,
    /// Ordered oldest-first equity curve from `RunReport.equity_series`.
    pub equity_curve: Vec<(Timestamp, Money<Usdt>)>,
}

// ── Sweep report ─────────────────────────────────────────────────────────────

/// The sweep result — returned by `run_param_sweep`.
#[derive(Debug, Clone)]
pub struct SweepReport {
    /// Echo of the request (family + coin + range + grid size + truncated?).
    pub config_echo: SweepRequestEcho,
    /// Per-cell results in insertion order (= grid order, post-cap).
    pub cells: Vec<SweepCellResult>,
    /// The SHIPPED config as a labelled baseline row (the divergence anchor, D8).
    pub baseline: SweepCellResult,
    /// Buy-and-hold KPIs (always shown — "vs just holding").
    pub benchmark: CandidateKpis,
}

// ── per-family strategy construction ─────────────────────────────────────────

/// An error from `build_swept_config` — parameterisation failure.
#[derive(Debug, thiserror::Error)]
pub enum SweepBuildError {
    /// The SMA params are invalid (fast >= slow or out of range).
    #[error("invalid SMA params: fast={fast} must be < slow={slow} and both in [1,400]")]
    InvalidSmaParams { fast: u32, slow: u32 },
    /// The MACD params are invalid (fast >= slow or period < 1).
    #[error("invalid MACD params: fast={fast} must be < slow={slow} and signal={signal} >= 1")]
    InvalidMacdParams { fast: u32, slow: u32, signal: u32 },
    /// The RSI params are invalid.
    #[error("invalid RSI params: period={period} >= 2 and oversold={oversold} in [1,49]")]
    InvalidRsiParams { period: u32, oversold: u32 },
    /// The Bollinger params are invalid.
    #[error("invalid Bollinger params: period={period} >= 2 and k={k} > 0")]
    InvalidBollingerParams { period: u32, k: Decimal },
    /// TOML generation produced a string that failed round-trip parse.
    #[error("generated TOML failed to parse: {0}")]
    TomlParseFailure(String),
}

// ── TOML generation helpers (ADR-0069 D3) ────────────────────────────────────

/// Generate the TOML string for a swept MACD cell.
///
/// The signal DSL is parameterized: `"macd_hist(fast,slow,signal) > 0 AND close > ema(200)"`.
/// The `id` field matches the `strategy_id` used in engine dispatch (e.g. `"btc_macd_trend"`).
fn macd_toml(fast: u32, slow: u32, signal: u32) -> String {
    format!(
        "id     = \"btc_macd_trend\"\n\
         kind   = \"composed\"\n\
         symbol = \"BTCUSDT\"\n\
         stage  = \"research\"\n\
         signal = \"macd_hist({fast},{slow},{signal}) > 0 AND close > ema(200)\"\n\
         size   = \"fixed_fraction(0.1)\"\n"
    )
}

/// Generate the TOML string for a swept RSI cell.
///
/// Signal DSL: `"rsi(period) < oversold AND close > min(low, 20)"`.
/// The `oversold` integer is the threshold, mirroring the shipped DSL.
fn rsi_toml(period: u32, oversold: u32) -> String {
    format!(
        "id     = \"btc_rsi_reversion\"\n\
         kind   = \"composed\"\n\
         symbol = \"BTCUSDT\"\n\
         stage  = \"research\"\n\
         signal = \"rsi({period}) < {oversold} AND close > min(low, 20)\"\n\
         size   = \"fixed_fraction(0.1)\"\n"
    )
}

/// Generate the TOML string for a swept Bollinger cell.
///
/// Signal DSL: `"close < bollinger_lower(period,k) AND volume > 1.5 * avg(volume, 20)"`.
/// `k` is `Decimal` — formatted without trailing zeros for DSL cleanliness.
fn bbands_toml(period: u32, k: Decimal) -> String {
    // Normalize k: strip trailing zeros (e.g. "2.0" not "2.00") for DSL compat.
    let k_str = k.normalize().to_string();
    format!(
        "id     = \"btc_bbands_mean_revert\"\n\
         kind   = \"composed\"\n\
         symbol = \"BTCUSDT\"\n\
         stage  = \"research\"\n\
         signal = \"close < bollinger_lower({period},{k_str}) AND volume > 1.5 * avg(volume, 20)\"\n\
         size   = \"fixed_fraction(0.1)\"\n"
    )
}

/// Build a `ScenarioConfig` for the given `(params, base_cfg)`, parameterised
/// for the swept cell.
///
/// For SMA: sets `sma_fast_len` / `sma_slow_len` in the `ScenarioConfig` (the
/// existing typed override — `runtime.rs:347`). The strategy id is always
/// `"v0.sma"`.
///
/// For MACD / RSI / Bollinger: generates an in-memory TOML string from the
/// swept params, validated via `ComposedStrategyConfig::from_str`, and
/// injects it via `composed_toml_override`. The dispatch strategy id is
/// the family's canonical composed id (e.g. `"btc_macd_trend"` for MACD).
///
/// # Errors
///
/// Returns `SweepBuildError` if the params are invalid or if the generated
/// TOML fails to round-trip through `from_str` (the identity guard).
#[allow(clippy::too_many_lines)] // 4 match arms × required validation + ScenarioConfig fields
pub fn build_swept_config(
    params: &SweptParams,
    symbol: &Symbol,
    range: &DateRange,
    seed: &[u8; 32],
    data_source: ScenarioDataSource,
    bars_override: Option<Vec<trading_core::Bar>>,
    initial_capital: Option<Decimal>,
) -> Result<ScenarioConfig, SweepBuildError> {
    match params {
        SweptParams::Sma { fast_len, slow_len } => {
            let fast = *fast_len as usize;
            let slow = *slow_len as usize;
            // Validate (guard: 1 ≤ fast < slow ≤ 400).
            if *fast_len < 1 || *fast_len >= *slow_len || *slow_len > 400 {
                return Err(SweepBuildError::InvalidSmaParams {
                    fast: *fast_len,
                    slow: *slow_len,
                });
            }
            Ok(ScenarioConfig {
                strategy: StrategyId(SmolStr::new_static(SMA_STRATEGY_ID)),
                pair: (trading_core::Venue::Binance, symbol.clone()),
                range: range.clone(),
                seed: *seed,
                write_report: false, // anchor-safe (ADR-0069 D9)
                data_source,
                bars_override,
                sma_fast_len: Some(fast),
                sma_slow_len: Some(slow),
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                reports_dir: None,
                params: None,
                short_enabled: false,
                initial_capital,
                // SMA uses typed override path — no in-memory TOML string needed.
                composed_toml_override: None,
            })
        }

        SweptParams::Macd { fast, slow, signal } => {
            // Validate: fast < slow, both ≥ 1, signal ≥ 1.
            if *fast < 1 || *fast >= *slow || *slow > 400 || *signal < 1 {
                return Err(SweepBuildError::InvalidMacdParams {
                    fast: *fast,
                    slow: *slow,
                    signal: *signal,
                });
            }
            let toml_str = macd_toml(*fast, *slow, *signal);
            // Round-trip validate (identity guard).
            strategy::ComposedStrategyConfig::from_str(&toml_str, "btc_macd_trend")
                .map_err(|e| SweepBuildError::TomlParseFailure(e.to_string()))?;
            Ok(ScenarioConfig {
                strategy: StrategyId(SmolStr::new_static("v0.5.macd")),
                pair: (trading_core::Venue::Binance, symbol.clone()),
                range: range.clone(),
                seed: *seed,
                write_report: false, // anchor-safe (ADR-0069 D9)
                data_source,
                bars_override,
                sma_fast_len: None,
                sma_slow_len: None,
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                reports_dir: None,
                params: None,
                short_enabled: false,
                initial_capital,
                composed_toml_override: Some(toml_str),
            })
        }

        SweptParams::Rsi { period, oversold } => {
            // Validate: period ≥ 2, oversold in [1,49].
            if *period < 2 || *oversold < 1 || *oversold > 49 {
                return Err(SweepBuildError::InvalidRsiParams {
                    period: *period,
                    oversold: *oversold,
                });
            }
            let toml_str = rsi_toml(*period, *oversold);
            // Round-trip validate (identity guard).
            strategy::ComposedStrategyConfig::from_str(&toml_str, "btc_rsi_reversion")
                .map_err(|e| SweepBuildError::TomlParseFailure(e.to_string()))?;
            Ok(ScenarioConfig {
                strategy: StrategyId(SmolStr::new_static("v0.5.rsi")),
                pair: (trading_core::Venue::Binance, symbol.clone()),
                range: range.clone(),
                seed: *seed,
                write_report: false, // anchor-safe (ADR-0069 D9)
                data_source,
                bars_override,
                sma_fast_len: None,
                sma_slow_len: None,
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                reports_dir: None,
                params: None,
                short_enabled: false,
                initial_capital,
                composed_toml_override: Some(toml_str),
            })
        }

        SweptParams::Bollinger { period, k } => {
            // Validate: period ≥ 2, k > 0.
            if *period < 2 || *k <= Decimal::ZERO {
                return Err(SweepBuildError::InvalidBollingerParams {
                    period: *period,
                    k: *k,
                });
            }
            let toml_str = bbands_toml(*period, *k);
            // Round-trip validate (identity guard).
            strategy::ComposedStrategyConfig::from_str(&toml_str, "btc_bbands_mean_revert")
                .map_err(|e| SweepBuildError::TomlParseFailure(e.to_string()))?;
            Ok(ScenarioConfig {
                strategy: StrategyId(SmolStr::new_static("v0.5.bbands")),
                pair: (trading_core::Venue::Binance, symbol.clone()),
                range: range.clone(),
                seed: *seed,
                write_report: false, // anchor-safe (ADR-0069 D9)
                data_source,
                bars_override,
                sma_fast_len: None,
                sma_slow_len: None,
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                reports_dir: None,
                params: None,
                short_enabled: false,
                initial_capital,
                composed_toml_override: Some(toml_str),
            })
        }
    }
}

// ── SweepProgress (the progress event) ───────────────────────────────────────

/// A sweep-progress sender wrapper — mirrors `BakeoffProgressSender`.
///
/// Reuses `BakeoffProgress` as the wire type (same `{done, total, current_id}`
/// shape). The UI uses the `BakeoffProgress` from the sweep progress channel
/// to drive the same determinate bar widget as the bake-off.
#[derive(Debug, Clone)]
pub struct SweepProgressSender(pub BakeoffProgressSender);

impl SweepProgressSender {
    /// Build a no-op sender. No channel allocation.
    #[must_use]
    pub fn disabled() -> Self {
        Self(BakeoffProgressSender::disabled())
    }

    /// Build a sender backed by a `tokio::sync::mpsc::Sender<BakeoffProgress>`.
    #[must_use]
    pub fn new(tx: tokio::sync::mpsc::Sender<BakeoffProgress>) -> Self {
        Self(BakeoffProgressSender::new(tx))
    }

    /// Send a progress event, dropping it if the channel is full or closed.
    pub fn try_send(&self, progress: BakeoffProgress) {
        self.0.try_send(progress);
    }
}

// ── Helper: range label ───────────────────────────────────────────────────────

fn range_label_for(range: &DateRange) -> &'static str {
    match range {
        DateRange::Last30d => "last 30 days",
        DateRange::Last90d => "last 90 days",
        DateRange::H1_2024 => "2024 H1",
        DateRange::H2_2024 => "2024 H2",
        DateRange::Custom { .. } => "custom window",
    }
}

// ── The shipped-config baseline params ───────────────────────────────────────

/// The shipped (default) SMA params: fast=20, slow=50.
const SHIPPED_SMA_FAST: u32 = 20;
const SHIPPED_SMA_SLOW: u32 = 50;

/// The shipped (default) MACD params: fast=12, slow=26, signal=9.
const SHIPPED_MACD_FAST: u32 = 12;
const SHIPPED_MACD_SLOW: u32 = 26;
const SHIPPED_MACD_SIGNAL: u32 = 9;

/// The shipped (default) RSI params: period=14, oversold=30.
const SHIPPED_RSI_PERIOD: u32 = 14;
const SHIPPED_RSI_OVERSOLD: u32 = 30;

/// The shipped (default) Bollinger params: period=20, k=2.0.
const SHIPPED_BBANDS_PERIOD: u32 = 20;

fn shipped_sma_params() -> SweptParams {
    SweptParams::Sma {
        fast_len: SHIPPED_SMA_FAST,
        slow_len: SHIPPED_SMA_SLOW,
    }
}

fn shipped_macd_params() -> SweptParams {
    SweptParams::Macd {
        fast: SHIPPED_MACD_FAST,
        slow: SHIPPED_MACD_SLOW,
        signal: SHIPPED_MACD_SIGNAL,
    }
}

fn shipped_rsi_params() -> SweptParams {
    SweptParams::Rsi {
        period: SHIPPED_RSI_PERIOD,
        oversold: SHIPPED_RSI_OVERSOLD,
    }
}

fn shipped_bbands_params() -> SweptParams {
    SweptParams::Bollinger {
        period: SHIPPED_BBANDS_PERIOD,
        k: dec!(2.0),
    }
}

/// Select the shipped baseline params for the given family (the divergence anchor).
fn shipped_params_for_family(family: SweepFamily) -> SweptParams {
    match family {
        SweepFamily::Sma => shipped_sma_params(),
        SweepFamily::Macd => shipped_macd_params(),
        SweepFamily::Rsi => shipped_rsi_params(),
        SweepFamily::Bollinger => shipped_bbands_params(),
    }
}

// ── run_param_sweep ───────────────────────────────────────────────────────────

/// Run the gate-tied hyperparameter sweep.
///
/// # Algorithm
///
/// 1. Enumerate the grid (cartesian product), drop invalid cells
///    (SMA: fast >= slow), cap at `MAX_SWEEP_CONFIGS` (deterministic
///    axis-major order).
/// 2. Preload real bars ONCE via `resolve_bakeoff_bars` (apples-to-apples,
///    mirroring `run_bakeoff`). `Synthetic` / `YahooCache` → `None`.
/// 3. Run the shipped-config `baseline` cell FIRST (used as the divergence anchor).
/// 4. Loop the capped grid: `run_scenario` per cell with `write_report = false`,
///    score via `compute_robustness_distribution`, collect `SweepCellResult`.
/// 5. Run the buy-and-hold `benchmark` arm.
/// 6. Check `cancel_rx` before each cell. Emit `SweepProgress` per cell.
/// 7. Return `SweepReport`.
///
/// # Anchor safety
///
/// Every cell uses `write_report = false` — no anchored report body is written
/// (ADR-0069 D9). `verify_anchors.sh` stays 119/119.
///
/// # Errors
///
/// Returns `RunError::Cancelled` if the operator cancels. Returns
/// `RunError::Internal` on data-load or scenario failure.
#[allow(clippy::too_many_lines)] // orchestrator function — scatter=readability loss (same as run_bakeoff)
pub async fn run_param_sweep(
    cfg: SweepConfig,
    cancel_rx: RunCancelReceiver,
    progress_tx: ProgressSender,
    sweep_progress_tx: SweepProgressSender,
) -> Result<SweepReport, RunError> {
    // ── 1. Enumerate + validate + cap the grid ────────────────────────────────
    let (all_params, invalid_count, requested_count) = enumerate_and_validate(&cfg.grid);

    let truncated = all_params.len() > MAX_SWEEP_CONFIGS;
    let capped_params: Vec<SweptParams> = all_params.into_iter().take(MAX_SWEEP_CONFIGS).collect();
    let grid_size = capped_params.len();

    tracing::info!(
        target: "sweep",
        family = cfg.family.label(),
        symbol = %cfg.symbol,
        grid_size,
        truncated,
        requested_count,
        invalid_count,
        "run_param_sweep: grid enumerated"
    );

    // ── 2. Preload bars once (apples-to-apples) ───────────────────────────────
    let preloaded_bars: Option<Vec<trading_core::Bar>> =
        resolve_bakeoff_bars(&cfg.symbol, &cfg.range, cfg.data_source).await?;

    // ── 3. Seed → u64 for bootstrap (mirrors advisor_robustness() LE-byte pattern) ─
    // This is the same derivation used in `advisor_robustness()` in leaderboard/runner.rs:
    // `u64::from_le_bytes([s[0..7]])`. The low 8 bytes of the [u8;32] ChaCha20 seed.
    let s = &cfg.seed;
    let seed_u64 = u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]);

    // ── 4. Run the shipped-config baseline cell ───────────────────────────────
    if cancel_rx.is_cancelled() {
        return Err(RunError::Cancelled);
    }
    let baseline_params = shipped_params_for_family(cfg.family);
    let baseline = run_one_cell(
        &baseline_params,
        &cfg.symbol,
        &cfg.range,
        &cfg.seed,
        seed_u64,
        cfg.data_source,
        preloaded_bars.clone(),
        cfg.paths,
        0, // candidate_index=0 for the baseline (salt slot 0)
        cancel_rx.sibling(),
        progress_tx.clone(),
    )
    .await?;

    // ── 5. Loop the capped grid ───────────────────────────────────────────────
    // total = grid_size + 1 (buy-and-hold) — but we report sweep progress for
    // the grid cells only (matching the bakeoff shape: done = cells completed).
    let total_for_progress = u16::try_from(grid_size).unwrap_or(u16::MAX);
    let mut cells: Vec<SweepCellResult> = Vec::with_capacity(grid_size);

    for (cell_index, params) in capped_params.iter().enumerate() {
        if cancel_rx.is_cancelled() {
            return Err(RunError::Cancelled);
        }

        // Emit progress BEFORE starting this cell.
        let done = u16::try_from(cell_index).unwrap_or(u16::MAX);
        sweep_progress_tx.try_send(BakeoffProgress {
            done,
            total: total_for_progress,
            current_id: params.label(),
        });

        // candidate_index offset: +1 so baseline (slot 0) and grid cells
        // (slots 1+) draw different resample streams.
        let candidate_index = cell_index + 1;
        let cell = run_one_cell(
            params,
            &cfg.symbol,
            &cfg.range,
            &cfg.seed,
            seed_u64,
            cfg.data_source,
            preloaded_bars.clone(),
            cfg.paths,
            candidate_index,
            cancel_rx.sibling(),
            progress_tx.clone(),
        )
        .await?;

        cells.push(cell);
    }

    // ── 6. Buy-and-hold benchmark ─────────────────────────────────────────────
    if cancel_rx.is_cancelled() {
        return Err(RunError::Cancelled);
    }

    let buyhold_cfg = ScenarioConfig {
        strategy: StrategyId(SmolStr::new_static(BUYHOLD_ID)),
        pair: (trading_core::Venue::Binance, cfg.symbol.clone()),
        range: cfg.range.clone(),
        seed: cfg.seed,
        write_report: false,
        data_source: cfg.data_source,
        bars_override: preloaded_bars,
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        params: None,
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
    };

    let buyhold_report = run_scenario(buyhold_cfg, cancel_rx.sibling(), progress_tx).await?;
    let benchmark = derive_candidate_kpis(&buyhold_report);

    // ── 7. Assemble the report ────────────────────────────────────────────────
    let config_echo = SweepRequestEcho {
        family_label: SmolStr::new(cfg.family.label()),
        coin: SmolStr::new(cfg.symbol.0.as_str()),
        range_label: SmolStr::new_static(range_label_for(&cfg.range)),
        grid_size,
        truncated,
        requested_count,
        invalid_count,
    };

    Ok(SweepReport {
        config_echo,
        cells,
        baseline,
        benchmark,
    })
}

// ── Internal: enumerate and validate the grid ─────────────────────────────────

/// Enumerate + validate the grid cells.
///
/// Returns `(valid_params, invalid_count, requested_count_including_invalid)`.
/// `valid_params` is in axis-major order (deterministic); invalid cells are
/// dropped and counted. `requested_count` is the full unconstrained cartesian
/// product size (before validity + cap filtering).
fn enumerate_and_validate(grid: &SweepGrid) -> (Vec<SweptParams>, usize, usize) {
    match grid {
        SweepGrid::Sma(sma) => {
            let (unconstrained_count, valid_pairs) = sma.enumerate_valid();
            let invalid_count = unconstrained_count - valid_pairs.len();
            let params: Vec<SweptParams> = valid_pairs
                .into_iter()
                .map(|(f, s)| SweptParams::Sma {
                    fast_len: f,
                    slow_len: s,
                })
                .collect();
            (params, invalid_count, unconstrained_count)
        }
        SweepGrid::Macd(macd) => {
            let (unconstrained_count, valid_triples) = macd.enumerate_valid();
            let invalid_count = unconstrained_count - valid_triples.len();
            let params: Vec<SweptParams> = valid_triples
                .into_iter()
                .map(|(f, s, sg)| SweptParams::Macd {
                    fast: f,
                    slow: s,
                    signal: sg,
                })
                .collect();
            (params, invalid_count, unconstrained_count)
        }
        SweepGrid::Rsi(rsi) => {
            let (unconstrained_count, valid_pairs) = rsi.enumerate_valid();
            let invalid_count = unconstrained_count - valid_pairs.len();
            let params: Vec<SweptParams> = valid_pairs
                .into_iter()
                .map(|(p, os)| SweptParams::Rsi {
                    period: p,
                    oversold: os,
                })
                .collect();
            (params, invalid_count, unconstrained_count)
        }
        SweepGrid::Bollinger(bb) => {
            let (unconstrained_count, valid_pairs) = bb.enumerate_valid();
            let invalid_count = unconstrained_count - valid_pairs.len();
            let params: Vec<SweptParams> = valid_pairs
                .into_iter()
                .map(|(p, k)| SweptParams::Bollinger { period: p, k })
                .collect();
            (params, invalid_count, unconstrained_count)
        }
    }
}

// ── Internal: run one cell ────────────────────────────────────────────────────

/// Run one sweep cell: scenario + bootstrap → `SweepCellResult`.
///
/// Uses `compute_robustness_distribution` (the T1 additive sibling) to surface
/// the full distribution. Falls back to `Fragile` if the distribution can't
/// be computed (too-short equity).
#[allow(clippy::too_many_arguments)]
async fn run_one_cell(
    params: &SweptParams,
    symbol: &Symbol,
    range: &DateRange,
    seed: &[u8; 32],
    seed_u64: u64,
    data_source: ScenarioDataSource,
    bars_override: Option<Vec<trading_core::Bar>>,
    paths: usize,
    candidate_index: usize,
    cancel_rx: RunCancelReceiver,
    progress_tx: ProgressSender,
) -> Result<SweepCellResult, RunError> {
    let scenario_cfg = build_swept_config(
        params,
        symbol,
        range,
        seed,
        data_source,
        bars_override,
        None, // use default 100_000 capital (sweep cells don't expose capital tuning yet)
    )
    .map_err(|e| RunError::Internal(e.to_string()))?;

    let report = run_scenario(scenario_cfg, cancel_rx, progress_tx).await?;
    let kpis = derive_candidate_kpis(&report);
    let equity_curve = report.equity_series.clone();

    // Extract equity for the bootstrap.
    let equity_decimals: Vec<Decimal> = report
        .equity_series
        .iter()
        .map(|(_, m)| m.amount())
        .collect();

    // Bootstrap: use `compute_robustness_distribution` (surfaces the full distribution).
    let master_seed = derive_master_seed(seed_u64, candidate_index);
    let (distribution, verdict) =
        compute_robustness_distribution(&equity_decimals, paths, master_seed).unwrap_or_else(
            || {
                // Curve too short — treat as Fragile (consistent with Skipped→Fragile
                // in the leaderboard context; a curve too short to score is untrustworthy).
                tracing::warn!(
                    target: "sweep",
                    params = %params.label(),
                    "sweep cell equity too short for bootstrap — marking Fragile"
                );
                (
                    fallback_distribution_summary(),
                    ParamRobustnessVerdict::Fragile,
                )
            },
        );

    tracing::debug!(
        target: "sweep",
        params = %params.label(),
        ?verdict,
        paths,
        "sweep cell complete"
    );

    Ok(SweepCellResult {
        params: params.clone(),
        kpis,
        verdict,
        distribution,
        equity_curve,
    })
}

/// A minimal `DistributionSummary` used as a fallback when the curve is too short
/// to run bootstrap. All values indicate "bad" so the verdict (Fragile) is honest.
fn fallback_distribution_summary() -> DistributionSummary {
    let bad = MetricDistribution {
        mean: 0.0,
        std: 0.0,
        p5: -1.0,
        p25: -1.0,
        p50: -1.0,
        p75: -1.0,
        p95: -1.0,
        min: -1.0,
        max: -1.0,
    };
    DistributionSummary {
        sharpe: bad.clone(),
        sortino: bad.clone(),
        calmar: bad.clone(),
        max_drawdown: bad.clone(),
        total_return: bad,
        prob_loss: 1.0,
        prob_sharpe_gt_0: 0.0,
        prob_sharpe_gt_1: 0.0,
        max_dd_tail_p50: 1.0,
        max_dd_tail_p95: 1.0,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_arithmetic, clippy::expect_used)]
mod tests {
    use super::*;

    // ── SweepAxis ────────────────────────────────────────────────────────────

    #[test]
    fn sweep_axis_values_basic() {
        let axis = SweepAxis {
            min: 10,
            max: 30,
            step: 5,
        };
        assert_eq!(axis.values(), vec![10, 15, 20, 25, 30]);
    }

    #[test]
    fn sweep_axis_values_step_one() {
        let axis = SweepAxis {
            min: 1,
            max: 4,
            step: 1,
        };
        assert_eq!(axis.values(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn sweep_axis_values_zero_step_treated_as_one() {
        let axis = SweepAxis {
            min: 5,
            max: 8,
            step: 0,
        };
        assert_eq!(axis.values(), vec![5, 6, 7, 8]);
    }

    #[test]
    fn sweep_axis_values_single_point() {
        let axis = SweepAxis {
            min: 20,
            max: 20,
            step: 1,
        };
        assert_eq!(axis.values(), vec![20]);
    }

    // ── SmaGrid ──────────────────────────────────────────────────────────────

    #[test]
    fn sma_grid_drops_invalid_fast_ge_slow() {
        // fast=[10,20], slow=[10,20] → invalid where fast >= slow (10,10), (20,10), (20,20)
        let grid = SmaGrid {
            fast_len: SweepAxis {
                min: 10,
                max: 20,
                step: 10,
            },
            slow_len: SweepAxis {
                min: 10,
                max: 20,
                step: 10,
            },
        };
        let (unconstrained, valid) = grid.enumerate_valid();
        assert_eq!(unconstrained, 4, "unconstrained = 2×2 = 4");
        // Only (10,20) is valid: fast=10 < slow=20.
        assert_eq!(valid, vec![(10, 20)]);
    }

    #[test]
    fn sma_grid_respects_400_upper_bound() {
        // slow > 400 should be filtered out.
        let grid = SmaGrid {
            fast_len: SweepAxis {
                min: 10,
                max: 10,
                step: 1,
            },
            slow_len: SweepAxis {
                min: 395,
                max: 410,
                step: 5,
            },
        };
        let (_, valid) = grid.enumerate_valid();
        // slow=395, 400 are valid; slow=405, 410 exceed 400 and are filtered.
        for (_, slow) in &valid {
            assert!(*slow <= 400, "slow must be ≤ 400");
        }
        // 395 and 400 are the only valid values.
        assert_eq!(valid.len(), 2);
    }

    // ── enumerate_and_validate ────────────────────────────────────────────────

    #[test]
    fn sweep_grid_truncates_at_cap() {
        // Build a grid large enough to exceed MAX_SWEEP_CONFIGS=24.
        // fast=[10..50 step 5] = [10,15,20,25,30,35,40,45,50] = 9 values
        // slow=[20..80 step 5] = [20,25,30,35,40,45,50,55,60,65,70,75,80] = 13 values
        // Cartesian: 9×13 = 117, many valid (fast < slow) — well above 24.
        let grid = SweepGrid::Sma(SmaGrid {
            fast_len: SweepAxis {
                min: 10,
                max: 50,
                step: 5,
            },
            slow_len: SweepAxis {
                min: 20,
                max: 80,
                step: 5,
            },
        });
        let (valid_params, _invalid_count, requested_count) = enumerate_and_validate(&grid);
        // valid_params has ALL valid entries; the cap is applied in run_param_sweep.
        // Here we just check the helper returns > 24 valid entries.
        assert!(
            valid_params.len() > MAX_SWEEP_CONFIGS,
            "grid should produce more than {MAX_SWEEP_CONFIGS} valid entries, got {}",
            valid_params.len()
        );
        // And requested_count (unconstrained) > valid (because some fast >= slow).
        assert!(
            requested_count >= valid_params.len(),
            "requested_count >= valid"
        );

        // Simulate the cap as done in run_param_sweep.
        let capped: Vec<_> = valid_params.into_iter().take(MAX_SWEEP_CONFIGS).collect();
        assert_eq!(capped.len(), MAX_SWEEP_CONFIGS);
        // And truncated would be true.
    }

    #[test]
    fn sweep_drops_invalid_sma_cells() {
        // A grid with some invalid cells (fast >= slow).
        let grid = SweepGrid::Sma(SmaGrid {
            fast_len: SweepAxis {
                min: 20,
                max: 40,
                step: 10,
            }, // [20, 30, 40]
            slow_len: SweepAxis {
                min: 20,
                max: 40,
                step: 10,
            }, // [20, 30, 40]
        });
        let (valid_params, invalid_count, requested_count) = enumerate_and_validate(&grid);
        // Unconstrained: 3×3 = 9. Valid: (20,30),(20,40),(30,40) = 3. Invalid=6.
        assert_eq!(requested_count, 9);
        assert_eq!(valid_params.len(), 3);
        assert_eq!(invalid_count, 6);
        // Verify all valid params have fast < slow.
        for p in &valid_params {
            match p {
                SweptParams::Sma { fast_len, slow_len } => {
                    assert!(fast_len < slow_len, "fast must be < slow");
                }
                _ => panic!("unexpected non-SMA variant in SMA grid test"),
            }
        }
    }

    // ── SweptParams ──────────────────────────────────────────────────────────

    #[test]
    fn swept_params_sma_label() {
        let p = SweptParams::Sma {
            fast_len: 15,
            slow_len: 40,
        };
        assert_eq!(p.label(), "fast=15, slow=40");
    }

    #[test]
    fn swept_params_is_sma_shipped_default() {
        let shipped = SweptParams::Sma {
            fast_len: 20,
            slow_len: 50,
        };
        let other = SweptParams::Sma {
            fast_len: 10,
            slow_len: 30,
        };
        assert!(shipped.is_sma_shipped_default());
        assert!(!other.is_sma_shipped_default());
    }

    // ── build_swept_config ───────────────────────────────────────────────────

    #[test]
    fn build_swept_config_sma_threads_params() {
        use trading_core::Symbol;
        let params = SweptParams::Sma {
            fast_len: 10,
            slow_len: 30,
        };
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let cfg = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        )
        .expect("should succeed");
        assert_eq!(cfg.sma_fast_len, Some(10));
        assert_eq!(cfg.sma_slow_len, Some(30));
        assert_eq!(cfg.strategy.0.as_str(), SMA_STRATEGY_ID);
        assert!(
            !cfg.write_report,
            "write_report must be false (anchor-safe)"
        );
    }

    #[test]
    fn build_swept_config_sma_rejects_invalid_params() {
        use trading_core::Symbol;
        let params = SweptParams::Sma {
            fast_len: 50,
            slow_len: 20,
        }; // fast >= slow
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let result = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        );
        assert!(result.is_err(), "fast >= slow must return Err");
    }

    // ── TOML generation identity guard tests (ADR-0069 D3) ──────────────────
    //
    // Each test asserts: macd_toml/rsi_toml/bbands_toml with the SHIPPED params
    // generates a TOML string that round-trips through ComposedStrategyConfig::from_str
    // and produces the SAME hash as loading the committed TOML file from disk.
    //
    // This is the architect's required identity guard: generated-with-shipped-params
    // == committed-TOML-file strategy (same AST → same hash).
    //
    // The tests are `#[ignore]` because they require `config/strategies/*.toml`
    // on disk (relative CWD = workspace root). Run with:
    // ```text
    // cargo test -p backtest --lib bakeoff::sweep::tests::macd_toml_shipped_params_round_trip -- --ignored
    // ```

    /// Identity guard: `macd_toml(12,26,9)` round-trips AND matches the committed file.
    #[test]
    #[ignore = "requires config/strategies/btc_macd_trend.toml at CWD (workspace root) — run with --ignored"]
    fn macd_toml_shipped_params_round_trip() {
        let generated = macd_toml(SHIPPED_MACD_FAST, SHIPPED_MACD_SLOW, SHIPPED_MACD_SIGNAL);
        // Must parse without error.
        let cfg_gen = strategy::ComposedStrategyConfig::from_str(&generated, "btc_macd_trend")
            .expect("generated MACD TOML must parse cleanly with shipped params");

        // Load the committed TOML file from disk.
        let disk_path = std::path::PathBuf::from("config/strategies/btc_macd_trend.toml");
        let disk_path_abs = crate::paths::resolve_workspace_path(&disk_path);
        let cfg_disk = strategy::ComposedStrategyConfig::from_file(&disk_path_abs)
            .expect("committed btc_macd_trend.toml must load");

        // The hash (AST fingerprint) must be identical — same strategy.
        assert_eq!(
            cfg_gen.hash, cfg_disk.hash,
            "generated MACD TOML with shipped params must produce the same AST hash as the committed TOML file. \
             If this fails, the signal DSL template does not match the committed file."
        );
        assert_eq!(
            cfg_gen.signal_raw.as_str(),
            cfg_disk.signal_raw.as_str(),
            "generated signal string must match committed signal string"
        );
    }

    /// Identity guard: `rsi_toml(14,30)` round-trips AND matches the committed file.
    #[test]
    #[ignore = "requires config/strategies/btc_rsi_reversion.toml at CWD (workspace root) — run with --ignored"]
    fn rsi_toml_shipped_params_round_trip() {
        let generated = rsi_toml(SHIPPED_RSI_PERIOD, SHIPPED_RSI_OVERSOLD);
        let cfg_gen = strategy::ComposedStrategyConfig::from_str(&generated, "btc_rsi_reversion")
            .expect("generated RSI TOML must parse cleanly with shipped params");

        let disk_path = std::path::PathBuf::from("config/strategies/btc_rsi_reversion.toml");
        let disk_path_abs = crate::paths::resolve_workspace_path(&disk_path);
        let cfg_disk = strategy::ComposedStrategyConfig::from_file(&disk_path_abs)
            .expect("committed btc_rsi_reversion.toml must load");

        assert_eq!(
            cfg_gen.hash, cfg_disk.hash,
            "generated RSI TOML with shipped params must produce the same AST hash as the committed TOML file."
        );
        assert_eq!(
            cfg_gen.signal_raw.as_str(),
            cfg_disk.signal_raw.as_str(),
            "generated signal string must match committed signal string"
        );
    }

    /// Identity guard: `bbands_toml(20, 2.0)` round-trips AND matches the committed file.
    #[test]
    #[ignore = "requires config/strategies/btc_bbands_mean_revert.toml at CWD (workspace root) — run with --ignored"]
    fn bbands_toml_shipped_params_round_trip() {
        let generated = bbands_toml(SHIPPED_BBANDS_PERIOD, dec!(2.0));
        let cfg_gen =
            strategy::ComposedStrategyConfig::from_str(&generated, "btc_bbands_mean_revert")
                .expect("generated BBands TOML must parse cleanly with shipped params");

        let disk_path = std::path::PathBuf::from("config/strategies/btc_bbands_mean_revert.toml");
        let disk_path_abs = crate::paths::resolve_workspace_path(&disk_path);
        let cfg_disk = strategy::ComposedStrategyConfig::from_file(&disk_path_abs)
            .expect("committed btc_bbands_mean_revert.toml must load");

        assert_eq!(
            cfg_gen.hash, cfg_disk.hash,
            "generated BBands TOML with shipped params must produce the same AST hash as the committed TOML file."
        );
        assert_eq!(
            cfg_gen.signal_raw.as_str(),
            cfg_disk.signal_raw.as_str(),
            "generated signal string must match committed signal string"
        );
    }

    // ── MacdGrid / RsiGrid / BollingerGrid unit tests ────────────────────────

    #[test]
    fn macd_grid_default_enumerate_valid_all_pairs_have_fast_lt_slow() {
        let grid = MacdGrid::default();
        let (_, valid) = grid.enumerate_valid();
        assert!(
            !valid.is_empty(),
            "default MACD grid must have valid triples"
        );
        for (f, s, sg) in &valid {
            assert!(f < s, "fast({f}) must be < slow({s})");
            assert!(*sg >= 1, "signal({sg}) must be >= 1");
        }
    }

    #[test]
    fn macd_grid_drops_fast_ge_slow() {
        // fast=[20,30], slow=[20] — (30,20) and (20,20) are invalid.
        let grid = MacdGrid {
            fast: SweepAxis {
                min: 20,
                max: 30,
                step: 10,
            }, // [20, 30]
            slow: SweepAxis {
                min: 20,
                max: 20,
                step: 1,
            }, // [20]
            signal: SweepAxis {
                min: 9,
                max: 9,
                step: 1,
            }, // [9]
        };
        let (unconstrained, valid) = grid.enumerate_valid();
        // Unconstrained = 2×1×1 = 2. Only (20, 20) would be invalid AND (30, 20) invalid.
        // Wait — both are invalid (20<20 false, 30<20 false) so valid = [].
        assert_eq!(unconstrained, 2);
        assert_eq!(
            valid.len(),
            0,
            "no valid triples when fast >= slow for all pairs"
        );
    }

    #[test]
    fn rsi_grid_default_enumerate_valid() {
        let grid = RsiGrid::default();
        let (unconstrained, valid) = grid.enumerate_valid();
        assert!(unconstrained > 0);
        assert!(!valid.is_empty());
        for (p, os) in &valid {
            assert!(*p >= 2, "period({p}) must be >= 2");
            assert!(*os <= 49, "oversold({os}) must be <= 49");
        }
    }

    #[test]
    fn bollinger_grid_default_enumerate_valid() {
        let grid = BollingerGrid::default();
        let (unconstrained, valid) = grid.enumerate_valid();
        assert!(unconstrained > 0);
        assert!(!valid.is_empty());
        for (p, k) in &valid {
            assert!(*p >= 2, "period({p}) must be >= 2");
            assert!(*k > Decimal::ZERO, "k({k}) must be > 0");
        }
    }

    #[test]
    fn macd_toml_generates_parseable_string() {
        let s = macd_toml(10, 24, 8);
        // Must contain the key DSL fragment.
        assert!(
            s.contains("macd_hist(10,24,8) > 0"),
            "MACD DSL must appear in TOML"
        );
        assert!(
            s.contains("btc_macd_trend"),
            "MACD TOML must have correct id"
        );
        // Must parse via from_str without error (the identity guard).
        let result = strategy::ComposedStrategyConfig::from_str(&s, "btc_macd_trend");
        assert!(
            result.is_ok(),
            "macd_toml output must parse cleanly: {result:?}"
        );
    }

    #[test]
    fn rsi_toml_generates_parseable_string() {
        let s = rsi_toml(10, 25);
        assert!(s.contains("rsi(10) < 25"), "RSI DSL must appear in TOML");
        assert!(
            s.contains("btc_rsi_reversion"),
            "RSI TOML must have correct id"
        );
        let result = strategy::ComposedStrategyConfig::from_str(&s, "btc_rsi_reversion");
        assert!(
            result.is_ok(),
            "rsi_toml output must parse cleanly: {result:?}"
        );
    }

    #[test]
    fn bbands_toml_generates_parseable_string() {
        let s = bbands_toml(18, dec!(1.5));
        assert!(
            s.contains("bollinger_lower(18,1.5)"),
            "BBands DSL must appear in TOML"
        );
        assert!(
            s.contains("btc_bbands_mean_revert"),
            "BBands TOML must have correct id"
        );
        let result = strategy::ComposedStrategyConfig::from_str(&s, "btc_bbands_mean_revert");
        assert!(
            result.is_ok(),
            "bbands_toml output must parse cleanly: {result:?}"
        );
    }

    #[test]
    fn bbands_toml_k_decimal_2_normalizes_to_2() {
        // k=2.0 must appear as "2" (normalized, no trailing zero) in the DSL.
        let s = bbands_toml(20, dec!(2.0));
        assert!(
            s.contains("bollinger_lower(20,2)"),
            "k=2.0 must normalize to '2' in DSL, got: {s}"
        );
    }

    #[test]
    fn build_swept_config_macd_sets_composed_toml_override() {
        use trading_core::Symbol;
        let params = SweptParams::Macd {
            fast: 8,
            slow: 20,
            signal: 7,
        };
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let cfg = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        )
        .expect("MACD build_swept_config must succeed for valid params");
        assert!(
            cfg.composed_toml_override.is_some(),
            "MACD sweep config must carry a composed_toml_override"
        );
        assert!(
            !cfg.write_report,
            "write_report must be false (anchor-safe)"
        );
        // Verify the override string contains the swept params.
        let toml_str = cfg.composed_toml_override.as_deref().unwrap();
        assert!(
            toml_str.contains("macd_hist(8,20,7)"),
            "TOML must contain swept MACD params"
        );
    }

    #[test]
    fn build_swept_config_rsi_sets_composed_toml_override() {
        use trading_core::Symbol;
        let params = SweptParams::Rsi {
            period: 10,
            oversold: 25,
        };
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let cfg = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        )
        .expect("RSI build_swept_config must succeed for valid params");
        assert!(cfg.composed_toml_override.is_some());
        let toml_str = cfg.composed_toml_override.as_deref().unwrap();
        assert!(
            toml_str.contains("rsi(10) < 25"),
            "TOML must contain swept RSI params"
        );
    }

    #[test]
    fn build_swept_config_bbands_sets_composed_toml_override() {
        use trading_core::Symbol;
        let params = SweptParams::Bollinger {
            period: 14,
            k: dec!(1.5),
        };
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let cfg = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        )
        .expect("BBands build_swept_config must succeed for valid params");
        assert!(cfg.composed_toml_override.is_some());
        let toml_str = cfg.composed_toml_override.as_deref().unwrap();
        assert!(
            toml_str.contains("bollinger_lower(14,1.5)"),
            "TOML must contain swept BBands params"
        );
    }

    #[test]
    fn build_swept_config_macd_rejects_fast_ge_slow() {
        use trading_core::Symbol;
        let params = SweptParams::Macd {
            fast: 26,
            slow: 12,
            signal: 9,
        }; // fast > slow
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let result = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        );
        assert!(result.is_err(), "fast >= slow must return Err for MACD");
    }

    #[test]
    fn build_swept_config_rsi_rejects_oversold_above_49() {
        use trading_core::Symbol;
        let params = SweptParams::Rsi {
            period: 14,
            oversold: 55,
        }; // oversold > 49
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let result = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        );
        assert!(result.is_err(), "oversold > 49 must return Err for RSI");
    }

    #[test]
    fn build_swept_config_bbands_rejects_zero_k() {
        use trading_core::Symbol;
        let params = SweptParams::Bollinger {
            period: 20,
            k: Decimal::ZERO,
        };
        let symbol = Symbol(SmolStr::new_static("BTCUSDT"));
        let range = crate::engine::DateRange::H1_2024;
        let seed = [1u8; 32];
        let result = build_swept_config(
            &params,
            &symbol,
            &range,
            &seed,
            ScenarioDataSource::Synthetic,
            None,
            None,
        );
        assert!(result.is_err(), "k=0 must return Err for BBands");
    }
}
