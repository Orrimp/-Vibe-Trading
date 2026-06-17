#![deny(clippy::unwrap_used)]
//! Backtest binary — T25, T516.
//!
//! Usage: `cargo run --release --bin backtest -- --scenario btc-2023-1m-sma-cross --seed 0xC0FFEE`
//!        `cargo run --release --bin backtest -- --scenario btc-2023-1m-macd-trend --strategy btc_macd_trend --seed 0xC0FFEE`
//!
//! Reads Parquet via `ReplayFeed` (or generates synthetic data if absent),
//! drives `StrategyRegistry` → `risk` → `PaperEngine` → `audit`,
//! writes a report to `spec/<feature>/reports/backtest-<stamp>-<scenario>.md`,
//! where `<feature>` is resolved from the scenario name via
//! [`scenario_to_feature`] (defined at the bottom of this file).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "realdata")]
use backtest::realdata::{RealDataBarSource, TimeSpan as RealDataTimeSpan};

use anyhow::{Context, Result};
use clap::Parser;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use tracing::info;
#[cfg(feature = "realdata")]
use tracing::warn;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Q-D1=(a) canonical real-data scenario list (v5 v0.5.0 / 2026-05-29) ─────
//
// Under operator decision Q-D1=(a) Linear-fallback: synthetic scenarios
// use `SlippageModel::Linear { bps: 8 }` regardless of CLI flags.
// Real-data scenarios (this list) use the CLI-specified model (SquareRoot when
// `--sim-slippage-sqrt-alpha > 0`).
//
// Q-D2=(β) per-scenario lazy-compute: for each real-data scenario below,
// `volume_usd_per_symbol` is populated via `data::universe_avg_daily_volume_usd_trailing`
// keyed on the scenario's logical end_date + 90-day lookback. The in-process
// Mutex<HashMap> cache inside the helper deduplicates parquet reads across
// scenarios sharing the same end_date.
#[cfg(feature = "realdata")]
const REAL_DATA_SCENARIO_IDS: &[&str] = &[
    // Group B — v1 momentum on real Binance data
    "top10-2023-fy-momentum-realdata",
    // Group F — TCN overlay realdata
    "top10-2023-fy-tcn-overlay-realdata",
    "top10-2024-fy-tcn-overlay-realdata",
    // Group G — TCN overlay weights realdata
    "top10-2023-fy-tcn-overlay-weights-realdata",
    "top10-2024-fy-tcn-overlay-weights-realdata",
    // Group H — PatchTST overlay realdata
    "top10-2023-fy-patchtst-overlay-realdata",
    // Group I — GARCH vol-target overlay realdata
    "top10-2023-fy-vol-target-overlay-realdata",
    // v3.0.0-regime — RegimeDispatcher realdata
    "top10-2023-fy-regime-dispatcher-realdata",
    "top10-2024-fy-regime-dispatcher-realdata",
];

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "backtest",
    about = "v0.5 backtest engine (SMA + composed strategies)"
)]
struct Args {
    /// Scenario name, e.g. btc-2023-1m-sma-cross
    #[arg(long)]
    scenario: String,

    /// Strategy id: compiled-in (e.g. sma_crossover) or composed TOML id
    /// (e.g. btc_macd_trend → loads config/strategies/btc_macd_trend.toml).
    /// When omitted the scenario's default strategy is used.
    #[arg(long)]
    strategy: Option<String>,

    /// RNG seed (hex or decimal), e.g. 0xC0FFEE
    #[arg(long, default_value = "0xC0FFEE")]
    seed: String,

    /// Path to agent.toml config
    #[arg(long, default_value = "config/agent.toml")]
    config: PathBuf,

    /// Override the report output directory (default: resolved from scenario
    /// name via `report_dir_for_scenario`).
    /// When set, the report is written to `<reports_dir>/backtest-<stamp>-<scenario>.md`
    /// instead. Useful for re-running into a tempdir without touching the
    /// anchored reports under `spec/`.
    ///
    /// Strictly additive flag — default-disabled; existing behaviour unchanged.
    #[arg(long)]
    reports_dir: Option<PathBuf>,

    /// When set, write the equity-curve as a plain text file (one f64 per line)
    /// to the given path after the report is written.
    ///
    /// Strictly additive flag — default-disabled; the report body is unchanged
    /// so anchor SHAs are byte-identical whether the flag is set or not.
    #[arg(long)]
    emit_equity_bin: Option<PathBuf>,

    // ── v5-latency-slippage-sim v0.2.0 canonical config flags (T-D-N1) ──────────
    // Default 0 = noop (anchor-safe). Set to canonical values for the
    // v0.2.0 re-emission run under ADR-0045 D1 (30..=80 ms / 8 bps).
    // Only consumed by scenario paths that have LatencySlippageSimConfig wired;
    // other paths ignore these flags and produce byte-identical output.
    /// Minimum simulated latency added to order timestamps (ms). Default: 0 (noop).
    #[arg(long, default_value = "0")]
    sim_latency_ms_min: u64,

    /// Maximum simulated latency added to order timestamps (ms). Default: 0 (noop).
    #[arg(long, default_value = "0")]
    sim_latency_ms_max: u64,

    /// Simulated slippage in basis points applied per fill (linear model). Default: 0 (noop).
    /// Mutually exclusive with --sim-slippage-sqrt-alpha (the last specified wins).
    #[arg(long, default_value = "0")]
    sim_slippage_bps: u32,

    // ── v5-latency-slippage-sim v0.5.0 square-root model flags ──────────────
    /// Square-root market-impact model: alpha coefficient (α · √(Q/V) · 10_000).
    /// When set (non-zero), overrides --sim-slippage-bps with SquareRoot model.
    /// Operator-locked default: α = 1.0 (M-OD 2026-05-29). Use 0 for linear model.
    #[arg(long, default_value = "0")]
    sim_slippage_sqrt_alpha: f64,

    /// Square-root model: volume lookback days for daily-volume-proxy V. Default: 90.
    #[arg(long, default_value = "90")]
    sim_slippage_sqrt_lookback_days: u16,

    // ── v5-latency-slippage-sim v0.3.0 Q1=(a) flag (T-D-N3a / ADR-0047 D1) ───
    /// Force synthetic-bar generation even when Parquet data exists on disk.
    ///
    /// Affects ONLY the single-symbol (SMA/Composed) dispatch arm at the auto-
    /// detect site (main.rs line ~977). Multi-symbol scenarios (momentum / pairs /
    /// tcn / patchtst / garch) are unaffected — they branch on the statically-
    /// declared `ScenarioDataSource` enum and never reach this guard.
    ///
    /// Use this flag when re-emitting canonical reports for Group A (SMA/Composed)
    /// under v5 canonical config to preserve the synthetic-data friction-free oracle
    /// (Q1 = route (a) per operator M-OD 2026-05-27, ADR-0047 D1 + D4).
    #[arg(long, default_value_t = false)]
    force_synthetic_bars: bool,
}

fn parse_seed(s: &str) -> Result<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).context("invalid hex seed")
    } else {
        s.parse::<u64>().context("invalid decimal seed")
    }
}

/// Build a `SlippageModel` from CLI args (scenario-agnostic raw builder).
///
/// If `--sim-slippage-sqrt-alpha > 0.0`, returns `SquareRoot { alpha, volume_lookback_days }`.
/// Otherwise returns `Linear { bps: sim_slippage_bps }`.
///
/// Use [`build_slippage_model_for_scenario`] for the Q-D1=(a) dispatch that
/// applies Linear{bps:8} fallback to synthetic scenarios.
///
/// Only called from the realdata branch of `build_slippage_model_for_scenario`.
#[cfg_attr(not(feature = "realdata"), allow(dead_code))]
fn build_slippage_model(args: &Args) -> cost::SlippageModel {
    if args.sim_slippage_sqrt_alpha > 0.0_f64 {
        let alpha = rust_decimal::Decimal::try_from(args.sim_slippage_sqrt_alpha)
            .unwrap_or(rust_decimal_macros::dec!(1.0));
        cost::SlippageModel::SquareRoot {
            alpha,
            volume_lookback_days: args.sim_slippage_sqrt_lookback_days,
        }
    } else {
        cost::SlippageModel::Linear {
            bps: args.sim_slippage_bps,
        }
    }
}

/// Build a `SlippageModel` for a named scenario, applying the Q-D1=(a) operator
/// decision: synthetic scenarios fall back to `Linear { bps: 8 }` regardless of
/// CLI flags; real-data scenarios use the CLI-specified model.
///
/// **Q-D1=(a) — operator ratified 2026-05-29** (per decision brief
/// `spec/dev-notes/v5-v0.5.0-q-d1-q-d2-decision-brief-2026-05-29.md`):
/// Linear fallback for synthetic preserves the v0.4.0 anchor SHAs under both
/// namespaces (`v5-realdata-medium-2026-05` stays byte-identical; the new
/// `v5-sqrt-impact-2026-05` namespace has only the real-data new SHAs).
///
/// Logs a tracing line at INFO level so the fallback path is auditable in the
/// backtest run log (K3 saturation audit + anchor-provenance audit).
fn build_slippage_model_for_scenario(args: &Args, scenario_name: &str) -> cost::SlippageModel {
    // Q-D1=(a) operator decision 2026-05-29: real-data scenarios use the CLI-specified
    // model; synthetic scenarios fall back to Linear{bps:8} regardless of CLI flags.
    #[cfg(feature = "realdata")]
    if REAL_DATA_SCENARIO_IDS.contains(&scenario_name) {
        // Real-data scenario: honour CLI sqrt flags.
        let model = build_slippage_model(args);
        info!(
            scenario = %scenario_name,
            model = ?model,
            "Q-D1=(a): real-data scenario — using CLI-specified slippage model"
        );
        return model;
    }

    // Suppress unused-variable lint when realdata is not compiled in.
    #[cfg(not(feature = "realdata"))]
    let _ = args;

    // Synthetic scenario (or realdata feature disabled): Q-D1=(a) Linear{bps:8} fallback.
    // Overrides any --sim-slippage-sqrt-alpha CLI flag for synthetic paths.
    info!(
        scenario = %scenario_name,
        "Q-D1=(a): synthetic scenario — slippage_model=Linear{{bps:8}} (fallback: synthetic data has no V proxy)"
    );
    cost::SlippageModel::Linear { bps: 8 }
}

/// Build the per-symbol volume USD map for a real-data scenario (Q-D2=(β)).
///
/// **Q-D2=(β) — operator ratified 2026-05-29**: per-scenario lazy-compute via
/// `data::universe_avg_daily_volume_usd_trailing`. The in-process Mutex<HashMap>
/// cache inside the helper deduplicates parquet reads across scenarios sharing
/// the same (end_date, lookback_days) tuple.
///
/// Returns `None` for synthetic scenarios under Q-D1=(a) Linear fallback, or
/// when the `realdata` feature is not compiled in.
///
/// Returns `Some(Arc<HashMap<Symbol, Decimal>>)` mapping every universe symbol
/// to the universe-average daily volume in USD. The single universe-avg V is
/// used for all symbols — per ADR-0043 § D3 v0.5.0 amendment (Kissell 2014
/// ch. 3 § "Volume-based impact" production-grade approximation).
///
/// Logs saturation warnings if the universe-avg V is zero (K3 gate).
/// Build volume map — realdata-feature-enabled implementation.
#[cfg(feature = "realdata")]
fn build_volume_map_for_scenario(
    data_root: &Path,
    scenario_name: &str,
    scenario_end_year: i32,
) -> Option<Arc<HashMap<Symbol, Decimal>>> {
    if !REAL_DATA_SCENARIO_IDS.contains(&scenario_name) {
        return None;
    }

    // Q-D2=(β): pin end_date to scenario's own end_date (scenario_end_year-12-31).
    // Deterministic across reruns as long as the parquet revision SHA is pinned.
    let end_date =
        match time::Date::from_calendar_date(scenario_end_year, time::Month::December, 31) {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    scenario = %scenario_name,
                    error = %e,
                    "build_volume_map_for_scenario: invalid end_date — falling back to None"
                );
                return None;
            }
        };

    let universe: Vec<Symbol> = backtest::scenarios::momentum::top10_symbols_with_prices()
        .into_iter()
        .map(|(sym, _)| sym)
        .collect();

    let lookback_days: u16 = 90; // Q2=(a) operator-locked

    match data::universe_avg_daily_volume_usd_trailing(
        data_root,
        &universe,
        end_date,
        lookback_days,
    ) {
        Ok(avg_v) => {
            if avg_v.is_zero() {
                warn!(
                    scenario = %scenario_name,
                    "build_volume_map_for_scenario: universe_avg_v=ZERO — K3 saturation risk; fills will hit MAX_SLIPPAGE_BPS"
                );
            } else {
                info!(
                    scenario = %scenario_name,
                    universe_avg_v_usd = %avg_v,
                    end_date = %end_date,
                    lookback_days = lookback_days,
                    "Q-D2=(β): universe-avg daily volume USD computed"
                );
            }
            // Map all universe symbols to the same universe-avg V.
            let map: HashMap<Symbol, Decimal> = universe.into_iter().map(|s| (s, avg_v)).collect();
            Some(Arc::new(map))
        }
        Err(e) => {
            warn!(
                scenario = %scenario_name,
                error = %e,
                "build_volume_map_for_scenario: universe_avg_daily_volume_usd_trailing failed — volume_usd_per_symbol=None"
            );
            None
        }
    }
}

/// Build volume map — stub when realdata feature is disabled.
#[cfg(not(feature = "realdata"))]
fn build_volume_map_for_scenario(
    _data_root: &Path,
    _scenario_name: &str,
    _scenario_end_year: i32,
) -> Option<Arc<HashMap<Symbol, Decimal>>> {
    None
}

// ── Scenario catalogue ────────────────────────────────────────────────────────

/// Data source axis — orthogonal to `ScenarioStrategy` (ADR-0032 § 3).
///
/// All existing v0/v0.5/v1/v1.5a/v2.5 scenarios use `Synthetic`.
/// The four new `-realdata` scenarios use `RealData` (feature-gated).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "realdata"), allow(dead_code))]
enum ScenarioDataSource {
    /// Seeded `ChaCha20Rng` GBM. Default for all existing scenarios.
    /// No data-on-disk requirement.
    Synthetic,
    /// `data/binance/<SYM>/<YEAR>/<MM>.parquet` via `RealDataBarSource`.
    /// Requires `--features realdata`; refuses to run without
    /// `data/binance/REVISION.toml`.
    RealData,
}

/// Whether the scenario uses the compiled-in SMA or a composed TOML strategy.
#[derive(Debug, Clone)]
enum ScenarioStrategy {
    /// Compiled-in SMA crossover.
    // fast_len / slow_len are set from `Scenario::from_name`; the extracted
    // `sma_composed_run::run` currently hardcodes 20/50 (the single canonical
    // values used by all v0 SMA scenarios). These fields are retained for
    // potential future parameterisation.
    #[allow(dead_code)]
    SmaCrossover { fast_len: usize, slow_len: usize },
    /// Composed strategy — resolved at run-time from a config path.
    Composed { id: String },
    /// v1 cross-sectional momentum — multi-symbol, loaded from TOML config.
    Momentum { config_id: String },
    /// v1.5a mean-reversion pairs — 4-symbol universe, loaded from TOML config.
    MeanReversionPairs { config_id: String },
    /// v2.5 TCN overlay momentum — v1 momentum + TCN forecast overlay.
    TcnOverlayMomentum {
        config_id: String,
        forecaster_id: String,
    },
    /// v2.5 TCN overlay momentum with real anchor weights (M3).
    ///
    /// Requires `--features candle` at compile time — without it the scenario
    /// dispatches to a runtime error rather than a silent passthrough fallback.
    TcnOverlayMomentumWeights {
        config_id: String,
        forecaster_id: String,
    },
    /// v2.5a `PatchTST` overlay momentum with real anchor weights (Wave D T-D-N23).
    ///
    /// Requires `--features candle realdata` at compile time — without them the
    /// scenario dispatches to a runtime error rather than a silent passthrough
    /// fallback.
    #[cfg_attr(not(feature = "realdata"), allow(dead_code))]
    PatchtstOverlayMomentumWeights {
        config_id: String,
        forecaster_id: String,
    },
    /// v3.0.0-volatility GARCH vol-targeting overlay on v1 momentum (R6.a primary).
    ///
    /// Requires `--features realdata` for real-data bar loading.
    /// Does NOT require `--features candle` — GARCH params are loaded from a
    /// plain JSON checkpoint.
    #[cfg_attr(not(feature = "realdata"), allow(dead_code))]
    GarchVolTargetOverlayMomentum {
        config_id: String,
        forecaster_id: String,
    },
    /// v3.0.0-regime RegimeDispatcher wrapping v1 momentum (Wave E T-D-E1).
    ///
    /// Routes Bull/Bear → v1 MomentumStrategy; Volatile/Calm → CashHoldStrategy.
    /// Uses MarkovSwitchingClassifier with ADR-0049 § D1 priors.
    /// Requires `--features realdata` for real-data bar loading.
    /// Does NOT require `--features candle` — Markov-switching is pure Rust.
    #[cfg_attr(not(feature = "realdata"), allow(dead_code))]
    RegimeDispatcherMomentum { config_id: String },
}

#[derive(Debug, Clone)]
struct Scenario {
    name: String,
    /// Canonical name written into the report body.  Usually equals `name`,
    /// but for alias scenarios (e.g. `btc-2023-1m-sma-baseline-refresh`) this
    /// is set to the v0 anchor name (`btc-2023-1m-sma-cross`) so that the
    /// body SHA-256 remains identical to the v0 ship hash.
    body_name: String,
    /// Override for the elapsed time written into the report body.
    /// `Some(0.2)` for SMA anchor scenarios so both `sma-cross` and
    /// `sma-baseline-refresh` produce a body-SHA256 == `fc2e3b4a…`.
    /// `None` means use the actual elapsed time.
    body_elapsed_override: Option<f64>,
    symbol: Symbol,
    start_year: i32,
    bar_count: usize,
    strategy: ScenarioStrategy,
    initial_capital: Decimal,
    slippage_bps: u32,
    taker_fee_bps: u32,
    baseline_report: Option<String>,
    #[allow(dead_code)]
    data_root: PathBuf,
    /// Orthogonal data-source axis (ADR-0032 § 3). Default: `Synthetic`.
    #[cfg_attr(not(feature = "realdata"), allow(dead_code))]
    data_source: ScenarioDataSource,
    /// Pinned aggregate SHA from `data/binance/REVISION.toml` at lock time.
    /// `None` for `Synthetic` scenarios and for `RealData` scenarios before
    /// the tester runs the M5 lock. When `Some`, every run asserts the
    /// on-disk aggregate SHA matches. (Tester fills at T-D-17.)
    #[cfg_attr(not(feature = "realdata"), allow(dead_code))]
    expected_revision_sha: Option<String>,
}

impl Scenario {
    fn from_name(name: &str, data_root: PathBuf) -> Result<Self> {
        match name {
            "btc-2023-1m-sma-cross" | "btc-2023-1m-sma-baseline-refresh" => Ok(Self {
                name: name.to_string(),
                // Both SMA scenarios share the same body_name so their report
                // body is byte-identical and the body-SHA256 anchors to the v0
                // ship hash (fc2e3b4a…).
                body_name: "btc-2023-1m-sma-cross".to_string(),
                // Fixed body elapsed of 0.2s preserves the v0 anchor hash
                // regardless of actual run duration.  The authoritative timing
                // is in the YAML front-matter `wall_clock_s:` field.
                body_elapsed_override: Some(0.2),
                symbol: Symbol::new("BTCUSDT"),
                start_year: 2023,
                bar_count: 525_600, // 365 days × 1440 bars/day
                strategy: ScenarioStrategy::SmaCrossover {
                    fast_len: 20,
                    slow_len: 50,
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "btc-2024-h1-sma-cross" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: Some(0.1),
                symbol: Symbol::new("BTCUSDT"),
                start_year: 2024,
                bar_count: 262_800, // ~182.5 days × 1440 bars/day
                strategy: ScenarioStrategy::SmaCrossover {
                    fast_len: 20,
                    slow_len: 50,
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "eth-2024-h1-sma-cross" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: Some(0.1),
                symbol: Symbol::new("ETHUSDT"),
                start_year: 2024,
                bar_count: 262_800, // mirrors btc-2024-h1-sma-cross (~182.5 days × 1440 bars/day)
                strategy: ScenarioStrategy::SmaCrossover {
                    fast_len: 20,
                    slow_len: 50,
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "btc-2023-1m-macd-trend" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: Some(2.5),
                symbol: Symbol::new("BTCUSDT"),
                start_year: 2023,
                bar_count: 525_600,
                strategy: ScenarioStrategy::Composed {
                    id: "btc_macd_trend".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "btc-2023-1m-rsi-reversion" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: Some(1.8),
                symbol: Symbol::new("BTCUSDT"),
                start_year: 2023,
                bar_count: 525_600,
                strategy: ScenarioStrategy::Composed {
                    id: "btc_rsi_reversion".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "btc-2023-1m-bbands-mean-revert" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: Some(6.2),
                symbol: Symbol::new("BTCUSDT"),
                start_year: 2023,
                bar_count: 525_600,
                strategy: ScenarioStrategy::Composed {
                    id: "btc_bbands_mean_revert".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            // v1 multi-symbol momentum scenarios (T617)
            "top10-2023-1h-momentum" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"), // placeholder — multi-symbol scenario
                start_year: 2023,
                // 365 days * 24 h/day = 8760 hourly bars per symbol × 10 symbols
                bar_count: 8760,
                strategy: ScenarioStrategy::Momentum {
                    config_id: "top10_momentum_h1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "top10-2024-h1-momentum" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2024,
                // ~182.5 days * 24 h/day = 4380 hourly bars per symbol × 10 symbols
                bar_count: 4380,
                strategy: ScenarioStrategy::Momentum {
                    config_id: "top10_momentum_h1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            // v1.5a mean-reversion pairs scenarios (T715)
            "pairs-2023-zscore-mr" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"), // 4-symbol universe
                start_year: 2023,
                // 365 days × 24 h/day = 8760 hourly bars per symbol × 4 symbols
                bar_count: 8760,
                strategy: ScenarioStrategy::MeanReversionPairs {
                    config_id: "pairs_mr_h1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "pairs-2024-h1-zscore-mr" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"), // 4-symbol universe
                start_year: 2024,
                // ~182.5 days × 24 h/day = 4380 hourly bars per symbol × 4 symbols
                bar_count: 4380,
                strategy: ScenarioStrategy::MeanReversionPairs {
                    config_id: "pairs_mr_h1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            // v2.5 TCN overlay momentum scenarios (T-D-15, T-D-16)
            // Canonical names per feature.md § Backtest Scenarios and trace.toml REQ-V25-TCN-001.
            "top10-2023-fy-tcn-overlay" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                // Oct–Dec 2023: 92 days × 24 h = 2208 hourly bars per symbol × 10 symbols
                bar_count: 2208,
                strategy: ScenarioStrategy::TcnOverlayMomentum {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "top10-2024-fy-tcn-overlay" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2024,
                // Q2–Q4 2024: 275 days × 24 h = 6600 hourly bars per symbol × 10 symbols
                bar_count: 6600,
                strategy: ScenarioStrategy::TcnOverlayMomentum {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs2".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            // M3 real-weights scenarios (v2.5.0-tcn-weights).
            // These require `--features candle`; without it the binary emits an
            // explicit error (see the TcnOverlayMomentumWeights dispatch arm).
            "top10-2023-fy-tcn-overlay-weights" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                // Oct–Dec 2023: 92 days × 24 h = 2208 hourly bars per symbol × 10 symbols
                bar_count: 2208,
                strategy: ScenarioStrategy::TcnOverlayMomentumWeights {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            "top10-2024-fy-tcn-overlay-weights" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2024,
                // Q2–Q4 2024: 275 days × 24 h = 6600 hourly bars per symbol × 10 symbols
                bar_count: 6600,
                strategy: ScenarioStrategy::TcnOverlayMomentumWeights {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs2".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::Synthetic,
                expected_revision_sha: None,
            }),
            // v2.6.0-realdata scenarios (ADR-0032). Feature-gated.
            // These require `--features realdata`; without it the binary
            // emits a clear error from the dispatch prelude (T-D-7).
            #[cfg(feature = "realdata")]
            "top10-2023-fy-tcn-overlay-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                // Full 2023: 365 days × 24 h = 8760 hourly bars per symbol × 10 symbols.
                bar_count: 8760,
                strategy: ScenarioStrategy::TcnOverlayMomentum {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                // T-D-17: pinned 2026-05-18 after revision-roundtrip fix.
                // SHA = aggregate over all 240 Binance hourly parquets (10 symbols × 24 months,
                // 2023-01-01 through 2024-12-31, fetched 2026-05-18 16:33 UTC).
                // See spec/backtest-real-binance-data/reports/ for the M5 capture log.
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            #[cfg(feature = "realdata")]
            "top10-2024-fy-tcn-overlay-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2024,
                // Full 2024 (leap year): 366 days × 24 h = 8784 hourly bars × 10 symbols.
                bar_count: 8784,
                strategy: ScenarioStrategy::TcnOverlayMomentum {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs2".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            #[cfg(feature = "realdata")]
            "top10-2023-fy-tcn-overlay-weights-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                bar_count: 8760,
                strategy: ScenarioStrategy::TcnOverlayMomentumWeights {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            #[cfg(feature = "realdata")]
            "top10-2024-fy-tcn-overlay-weights-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2024,
                bar_count: 8784,
                strategy: ScenarioStrategy::TcnOverlayMomentumWeights {
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: "tcn-bs2".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            // v3.0.0-volatility-rebaseline: un-targeted v1 cross-sectional momentum on
            // real Binance hourly data. Sibling baseline for the
            // sharpe-comparison-vol-target-bs1-realbaseline report (Q1=(b) default).
            // Same dataset SHA + initial_capital + slippage + fees as the parent
            // vol-target-overlay-realdata scenario for apples-to-apples Sharpe delta.
            #[cfg(feature = "realdata")]
            "top10-2023-fy-momentum-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                // Full 2023: 365 days × 24 h = 8760 hourly bars per symbol × 10 symbols.
                bar_count: 8760,
                strategy: ScenarioStrategy::Momentum {
                    config_id: "top10_momentum_h1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                // Same dataset SHA as the parent vol-target-realdata scenario — pinned
                // 2026-05-18 per ADR-0032.
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            // v2.5a PatchTST overlay — realdata scenario (ADR-0036 § D7, T-D-N23).
            // Requires `--features realdata candle`.
            #[cfg(feature = "realdata")]
            "top10-2023-fy-patchtst-overlay-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                // Full 2023: 365 days × 24 h = 8760 hourly bars per symbol × 10 symbols.
                bar_count: 8760,
                strategy: ScenarioStrategy::PatchtstOverlayMomentumWeights {
                    config_id: "patchtst_overlay_momentum".to_string(),
                    forecaster_id: "patchtst-bs1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                // Same dataset as the TCN realdata scenarios — pinned 2026-05-18.
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            // v3.0.0-volatility: GARCH vol-targeting overlay on v1 momentum (R6.a primary).
            // Full 2023: 365 days × 24 h = 8760 hourly bars per symbol × 10 symbols.
            "top10-2023-fy-vol-target-overlay-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                bar_count: 8760,
                strategy: ScenarioStrategy::GarchVolTargetOverlayMomentum {
                    config_id: "vol_target_overlay_momentum".to_string(),
                    forecaster_id: "garch-bs1".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                // Same dataset as the TCN realdata scenarios — pinned 2026-05-18.
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            // v3.0.0-regime RegimeDispatcher scenarios (Wave E T-D-E1).
            // Requires `--features realdata`.
            // Canonical friction: latency 30..=80ms / 8 bps per ADR-0045 D1.
            // Dataset SHA 3a8b96c4… pinned 2026-05-18 (same as sibling realdata scenarios).
            #[cfg(feature = "realdata")]
            "top10-2023-fy-regime-dispatcher-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2023,
                // Full 2023: 365 days × 24 h = 8760 hourly bars per symbol × 10 symbols.
                bar_count: 8760,
                strategy: ScenarioStrategy::RegimeDispatcherMomentum {
                    config_id: "regime_dispatcher_momentum".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            #[cfg(feature = "realdata")]
            "top10-2024-fy-regime-dispatcher-realdata" => Ok(Self {
                name: name.to_string(),
                body_name: name.to_string(),
                body_elapsed_override: None,
                symbol: Symbol::new("multi"),
                start_year: 2024,
                // Full 2024 (leap year): 366 days × 24 h = 8784 hourly bars × 10 symbols.
                bar_count: 8784,
                strategy: ScenarioStrategy::RegimeDispatcherMomentum {
                    config_id: "regime_dispatcher_momentum".to_string(),
                },
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
                data_root,
                data_source: ScenarioDataSource::RealData,
                expected_revision_sha: Some(
                    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into(),
                ),
            }),
            other => anyhow::bail!("unknown scenario: {other}"),
        }
    }

    /// Compute the calendar-year half-open span for this scenario.
    ///
    /// Returns `(start_ms, end_ms)` in Unix milliseconds.
    #[cfg(feature = "realdata")]
    fn span(&self) -> RealDataTimeSpan {
        RealDataTimeSpan::full_year(self.start_year)
    }
}

// ── Synthetic data generation ─────────────────────────────────────────────────

fn synthetic_bars(
    symbol: &Symbol,
    count: usize,
    seed: u64,
    start_price: Decimal,
    start_year: i32,
) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut bars = Vec::with_capacity(count);

    let per_min_vol: f64 = 0.001_10;
    let per_min_drift: f64 = 0.000_001_9;

    let epoch_base = {
        let date = time::Date::from_calendar_date(start_year, time::Month::January, 1)
            .unwrap_or_else(|_| {
                // 2023-01-01 is always valid; unreachable branch
                time::Date::from_calendar_date(2023, time::Month::January, 1)
                    .unwrap_or_else(|e| unreachable!("2023-01-01 is always valid: {e}"))
            });
        OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    let mut close: f64 = start_price.to_string().parse::<f64>().unwrap_or(30_000.0);

    for i in 0..count {
        // Box-Muller for Gaussian noise
        let u1: f64 = rng.random::<f64>().max(1e-10_f64);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos();
        let ret = per_min_drift + per_min_vol * z;
        let next = (close * (1.0 + ret)).clamp(1_000.0_f64, 500_000.0_f64);

        let intra_vol = close * 0.000_5_f64;
        let noise1: f64 = rng.random::<f64>() * intra_vol;
        let noise2: f64 = rng.random::<f64>() * intra_vol;

        let open = close;
        let high = open.max(next) + noise1;
        let low = (open.min(next) - noise2).max(0.01_f64);
        let vol_btc: f64 = rng.random::<f64>() * 50.0_f64 + 1.0_f64;

        let open_ts = Timestamp::new(epoch_base + time::Duration::minutes(i as i64));
        let close_ts = Timestamp::new(
            epoch_base + time::Duration::minutes(i as i64 + 1) - time::Duration::seconds(1),
        );

        let to_dec =
            |v: f64| -> Decimal { Decimal::try_from(v.max(0.01_f64)).unwrap_or(dec!(0.01)) };
        let price_or_one = |v: f64| -> Price {
            Price::new(to_dec(v)).unwrap_or_else(|_| {
                // dec!(1) is always positive; this branch is unreachable
                Price::new(dec!(1)).unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            })
        };

        bars.push(Bar {
            symbol: symbol.clone(),
            tf: Timeframe::OneMinute,
            open_ts,
            close_ts,
            open: price_or_one(open),
            high: price_or_one(high.max(open).max(next)),
            low: price_or_one(low.min(open).min(next).max(0.01)),
            close: price_or_one(next),
            volume: Quantity::new(to_dec(vol_btc)).unwrap_or_else(|_| {
                // dec!(1) is always positive; this branch is unreachable
                Quantity::new(dec!(1))
                    .unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            }),
            trade_count: rng.random_range(10_u32..500_u32),
            local_recv_ts: close_ts,
            venue: Venue::Binance,
        });

        close = next;
    }

    bars
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // T-RED-D11 (v2-1-tracing-layer-redactor): migrated to install_global.
    llm::tracing_init::install_global(&["backtest=info"], false)?;

    let args = Args::parse();
    let seed = parse_seed(&args.seed)?;

    info!(scenario = %args.scenario, seed = seed, "backtest starting");

    let data_root = PathBuf::from("data/binance");
    let mut scenario = Scenario::from_name(&args.scenario, data_root.clone())?;

    // ── Determine data source ──────────────────────────────────────────────────
    // Multi-symbol (momentum + pairs) scenarios always use synthetic data.
    // T616: no Parquet fixture; seeded ChaCha20Rng provides determinism.
    // T713: same synthetic fallback for 4-symbol pairs universe.
    let is_momentum = matches!(&scenario.strategy, ScenarioStrategy::Momentum { .. });
    let is_pairs = matches!(
        &scenario.strategy,
        ScenarioStrategy::MeanReversionPairs { .. }
    );
    let is_tcn_overlay = matches!(
        &scenario.strategy,
        ScenarioStrategy::TcnOverlayMomentum { .. }
            | ScenarioStrategy::TcnOverlayMomentumWeights { .. }
            | ScenarioStrategy::PatchtstOverlayMomentumWeights { .. }
            | ScenarioStrategy::GarchVolTargetOverlayMomentum { .. }
            | ScenarioStrategy::RegimeDispatcherMomentum { .. }
    );

    // Pre-loaded real bars for TCN realdata path (ADR-0032 § 3).
    // Set in the RealData arm of the is_tcn_overlay dispatch and passed to
    // run_tcn_overlay_backtest / run_tcn_overlay_weights_backtest as bars_override.
    let mut realdata_bars_for_tcn: Option<Vec<Bar>> = None;
    // `mut` is required by the #[cfg(feature = "realdata")] arm; suppress without feature.
    #[cfg_attr(not(feature = "realdata"), allow(unused_mut))]
    let mut realdata_revision_sha_for_tcn: Option<String> = None;

    // Pre-loaded real bars for the momentum realdata path (v3.0.0-volatility-rebaseline).
    // Set in the RealData arm of the is_momentum dispatch and passed to momentum::run
    // via MomentumScenarioInput::bars_override.
    let mut realdata_bars_for_momentum: Option<Vec<Bar>> = None;
    // `mut` is required by the #[cfg(feature = "realdata")] arm; suppress without feature.
    #[cfg_attr(not(feature = "realdata"), allow(unused_mut))]
    let mut realdata_revision_sha_for_momentum: Option<String> = None;

    let (bars, data_source) = if is_momentum {
        match scenario.data_source {
            ScenarioDataSource::Synthetic => {
                info!(
                    bar_count = scenario.bar_count,
                    "multi-symbol scenario — generating synthetic bars"
                );
                // Momentum scenarios: data_source string is part of the v1 ship contract
                // (locked anchor hashes 3b60ef07… / 1f33534f…).  Must stay byte-for-byte
                // identical to what v1 emitted.  Do NOT change this string.
                (
                    Vec::<Bar>::new(),
                    "synthetic (seeded RNG, v1 multi-symbol)".to_string(),
                )
            }
            ScenarioDataSource::RealData => {
                // realdata feature gate: clear error if not compiled in.
                // Uses `return Err(...)` so the (non-realdata) branch is `!`-typed
                // and the cfg-gated realdata block is the only producing branch in
                // realdata builds; eliminates the trailing `unreachable!()` that
                // clippy flagged as dead code under `-D warnings`.
                #[cfg(not(feature = "realdata"))]
                return Err(anyhow::anyhow!(
                    "scenario '{}' requires --features realdata. \
                     Rebuild with: cargo run -p backtest --release --features realdata -- \
                     --scenario {} --seed 0xC0FFEE",
                    scenario.name,
                    scenario.name,
                ));
                #[cfg(feature = "realdata")]
                {
                    let expected_total = scenario.bar_count
                        * backtest::scenarios::momentum::top10_symbols_with_prices().len();
                    let src = RealDataBarSource::new(
                        data_root.clone(),
                        backtest::scenarios::momentum::top10_symbols_with_prices()
                            .into_iter()
                            .map(|(s, _)| s)
                            .collect(),
                    );
                    let loaded = src
                        .load(scenario.span(), expected_total, &scenario.name)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;

                    // Assert pinned revision if set (ADR-0032).
                    if let Some(pinned) = &scenario.expected_revision_sha
                        && pinned != &loaded.revision_sha
                    {
                        anyhow::bail!(
                            "data revision mismatch: scenario pinned {} \
                             but on-disk computed {}",
                            pinned,
                            loaded.revision_sha
                        );
                    }

                    info!(
                        bar_count = loaded.loaded_count,
                        revision_sha = %loaded.revision_sha,
                        "momentum realdata bars loaded"
                    );
                    realdata_bars_for_momentum = Some(loaded.bars);
                    realdata_revision_sha_for_momentum = Some(loaded.revision_sha);
                    (
                        Vec::<Bar>::new(),
                        "real (Binance Vision via data/binance/, v3.0.0-volatility-rebaseline)"
                            .to_string(),
                    )
                }
            }
        }
    } else if is_tcn_overlay {
        let is_weights = matches!(
            &scenario.strategy,
            ScenarioStrategy::TcnOverlayMomentumWeights { .. }
        );

        // Detect PatchTST to adjust the synthetic/log labels.
        let is_patchtst_overlay = matches!(
            &scenario.strategy,
            ScenarioStrategy::PatchtstOverlayMomentumWeights { .. }
        );

        // Detect GARCH vol-target overlay.
        let is_garch_vol_target = matches!(
            &scenario.strategy,
            ScenarioStrategy::GarchVolTargetOverlayMomentum { .. }
        );

        // Detect regime dispatcher (v3.0.0-regime Wave E).
        let is_regime_dispatcher = matches!(
            &scenario.strategy,
            ScenarioStrategy::RegimeDispatcherMomentum { .. }
        );

        // Branch on data source axis (ADR-0032 § 3).
        match scenario.data_source {
            ScenarioDataSource::Synthetic => {
                let src = if is_regime_dispatcher {
                    "synthetic (seeded RNG, v3.0.0-regime dispatcher)"
                } else if is_garch_vol_target {
                    "synthetic (seeded RNG, v3.0.0 garch-vol-target-overlay)"
                } else if is_patchtst_overlay {
                    "synthetic (seeded RNG, v2.5a patchtst-overlay-weights)"
                } else if is_weights {
                    "synthetic (seeded RNG, v2.5 tcn-overlay-weights)"
                } else {
                    "synthetic (seeded RNG, v2.5 tcn-overlay)"
                };
                info!(
                    bar_count = scenario.bar_count,
                    weights = is_weights,
                    patchtst = is_patchtst_overlay,
                    garch_vol_target = is_garch_vol_target,
                    regime_dispatcher = is_regime_dispatcher,
                    "tcn/patchtst/garch/regime-overlay scenario — generating synthetic bars"
                );
                (Vec::<Bar>::new(), src.to_string())
            }
            ScenarioDataSource::RealData => {
                // realdata feature gate: clear error if not compiled in.
                #[cfg(not(feature = "realdata"))]
                anyhow::bail!(
                    "scenario '{}' requires --features realdata. \
                     Rebuild with: cargo run -p backtest --release --features realdata -- \
                     --scenario {} --seed 0xC0FFEE",
                    scenario.name,
                    scenario.name,
                );
                #[cfg(feature = "realdata")]
                {
                    let expected_total = scenario.bar_count
                        * backtest::scenarios::momentum::top10_symbols_with_prices().len();
                    let src = RealDataBarSource::new(
                        data_root.clone(),
                        backtest::scenarios::momentum::top10_symbols_with_prices()
                            .into_iter()
                            .map(|(s, _)| s)
                            .collect(),
                    );
                    let loaded = src
                        .load(scenario.span(), expected_total, &scenario.name)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;

                    // Assert pinned revision if set (T-D-17).
                    if let Some(pinned) = &scenario.expected_revision_sha
                        && pinned != &loaded.revision_sha
                    {
                        anyhow::bail!(
                            "data revision mismatch: scenario pinned {} \
                             but on-disk computed {}",
                            pinned,
                            loaded.revision_sha
                        );
                    }

                    let data_src_str = if is_regime_dispatcher {
                        "real (Binance Vision via data/binance/, v3.0.0-regime)".to_string()
                    } else if is_garch_vol_target {
                        "real (Binance Vision via data/binance/, v3.0.0-volatility)".to_string()
                    } else if is_patchtst_overlay {
                        "real (Binance Vision via data/binance/, v2.5a.0-patchtst)".to_string()
                    } else {
                        "real (Binance Vision via data/binance/, v2.6.0-realdata)".to_string()
                    };
                    info!(
                        bar_count = loaded.loaded_count,
                        revision_sha = %loaded.revision_sha,
                        "tcn/patchtst/garch/regime-overlay realdata bars loaded"
                    );
                    // Stash the real bars; they are passed to the run function below.
                    // The `bars` variable is unused for TCN overlay (functions generate
                    // bars internally — except for the RealData path which uses
                    // `bars_override`).  We pass the bars via `realdata_bars_for_tcn`.
                    realdata_bars_for_tcn = Some(loaded.bars);
                    realdata_revision_sha_for_tcn = Some(loaded.revision_sha);
                    (Vec::<Bar>::new(), data_src_str)
                }
            }
        }
    } else if is_pairs {
        info!(
            bar_count = scenario.bar_count,
            "pairs scenario — generating synthetic bars"
        );
        // Pairs scenarios (v1.5a): new scenarios with no locked v1 anchor.
        // Free to use the v1.5a label.
        (
            Vec::<Bar>::new(),
            "synthetic (seeded RNG, v1.5a multi-symbol)".to_string(),
        )
    } else {
        let parquet_dir = data_root
            .join(scenario.symbol.to_string())
            .join(scenario.start_year.to_string());

        // v5-latency-slippage-sim v0.3.0 Q1=(a) guard (ADR-0047 D1):
        // when --force-synthetic-bars is set, skip the Parquet auto-detect so
        // Group A (SMA/Composed) scenarios always use the synthetic GBM baseline.
        // This preserves the friction-free oracle for 5/69 anchors (noop-baseline
        // SHAs stay valid apples-to-apples comparisons when friction is the only
        // variable). Default: false → existing auto-detect behaviour unchanged.
        let has_parquet = !args.force_synthetic_bars
            && parquet_dir.exists()
            && std::fs::read_dir(&parquet_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);

        if has_parquet {
            info!(path = ?parquet_dir, "loading Parquet data");
            use data::MarketDataSource as _;
            let feed = data::ReplayFeed::new(&data_root, true);
            let stream = feed
                .subscribe_bars(scenario.symbol.clone(), Timeframe::OneMinute)
                .await
                .context("open replay feed")?;
            use tokio_stream::StreamExt as _;
            let bars: Vec<Bar> = stream
                .filter_map(|r: Result<Bar, trading_core::FeedError>| r.ok())
                .collect()
                .await;
            info!(bars = bars.len(), "Parquet bars loaded");
            (bars, "real (Binance Vision)".to_string())
        } else {
            info!(
                count = scenario.bar_count,
                "no Parquet data — generating synthetic bars"
            );
            let start_price = match scenario.name.as_str() {
                "btc-2023-1m-sma-cross"
                | "btc-2023-1m-sma-baseline-refresh"
                | "btc-2023-1m-macd-trend"
                | "btc-2023-1m-rsi-reversion"
                | "btc-2023-1m-bbands-mean-revert" => dec!(16_500),
                "btc-2024-h1-sma-cross" => dec!(42_000),
                "eth-2024-h1-sma-cross" => dec!(2_400),
                _ => dec!(30_000),
            };
            let bars = synthetic_bars(
                &scenario.symbol,
                scenario.bar_count,
                seed,
                start_price,
                scenario.start_year,
            );
            (bars, "synthetic (seeded RNG, v0 fallback)".to_string())
        }
    };

    // ── Find baseline for comparative scenarios ────────────────────────────────
    if args.scenario == "btc-2024-h1-sma-cross" {
        let baseline_dir = report_dir_for_scenario("btc-2023-1m-sma-cross");
        if let Some(b) = find_latest_report(&baseline_dir, "btc-2023-1m-sma-cross") {
            scenario.baseline_report = Some(b);
        }
    } else if matches!(
        args.scenario.as_str(),
        "btc-2023-1m-macd-trend" | "btc-2023-1m-rsi-reversion" | "btc-2023-1m-bbands-mean-revert"
    ) {
        let baseline_dir = report_dir_for_scenario("btc-2023-1m-sma-baseline-refresh");
        if let Some(b) = find_latest_report(&baseline_dir, "btc-2023-1m-sma-baseline-refresh") {
            scenario.baseline_report = Some(b);
        }
    }

    let bar_count = bars.len();
    info!(bars = bar_count, data = %data_source, "data ready");

    // ── Strategy + risk setup ──────────────────────────────────────────────────
    // ── v1 multi-symbol momentum: separate execution path ─────────────────────
    if let ScenarioStrategy::Momentum { config_id } = &scenario.strategy.clone() {
        let config_id = config_id.clone();
        // Q-D1=(a): dispatch on synthetic vs real-data for slippage model selection.
        // Q-D2=(β): lazy-compute universe-avg V for real-data scenarios.
        let mom_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let mom_volume_map =
            build_volume_map_for_scenario(&data_root, &scenario.name, scenario.start_year);
        let input = backtest::cli_types::MomentumScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id,
            bars_override: realdata_bars_for_momentum.take(),
            data_revision_sha: realdata_revision_sha_for_momentum.clone(),
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) + Q-D2=(β) wiring.
            // Real-data: SquareRoot model + universe-avg V map.
            // Synthetic: Linear{bps:8} fallback + no V map (per operator 2026-05-29).
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: mom_slippage_model,
                volume_usd_per_symbol: mom_volume_map,
            },
        };
        // Bug #63 — CLI uses no-op cancel + progress so byte-identical to pre-fix.
        let (_h, m_cancel) = backtest::cancel::cancellation_pair();
        let m_progress = backtest::progress::ProgressSender::disabled();
        let result = backtest::scenarios::momentum::run(&input, seed, m_cancel, m_progress).await?;

        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        backtest::report::momentum::write(&input, &result, seed, &data_source, &report_path)?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            input.start_year,
        )?;

        // Emit equity bin (moved to MomentumScenarioInput; kept here for
        // backwards-compat with the --emit-equity-bin flag path used by sharpe_comparison).
        if let Some(ref eq_path) = args.emit_equity_bin {
            let eq_text: String = result
                .equity_curve
                .iter()
                .map(|d| format!("{d}\n"))
                .collect();
            std::fs::write(eq_path, eq_text)
                .with_context(|| format!("writing equity bin to {}", eq_path.display()))?;
            info!(path = %eq_path.display(), len = result.equity_curve.len(), "equity bin written");
        }

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {bar_count}");
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // ── v1.5a mean-reversion pairs: separate execution path ──────────────────
    if let ScenarioStrategy::MeanReversionPairs { config_id } = &scenario.strategy.clone() {
        let config_id = config_id.clone();
        // Q-D1=(a): Pairs scenarios are always synthetic → Linear{bps:8} fallback.
        let pairs_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let pairs_input = backtest::cli_types::PairsScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id,
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) dispatch.
            // Pairs scenarios are synthetic → Linear{bps:8} fallback (no V map needed).
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: pairs_slippage_model,
                volume_usd_per_symbol: None,
            },
        };
        // Bug #63 — CLI uses no-op cancel + progress so byte-identical to pre-fix.
        let (_h, p_cancel) = backtest::cancel::cancellation_pair();
        let p_progress = backtest::progress::ProgressSender::disabled();
        let result =
            backtest::scenarios::pairs::run(&pairs_input, seed, p_cancel, p_progress).await?;

        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        backtest::report::pairs::write(&pairs_input, &result, seed, &data_source, &report_path)?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            pairs_input.start_year,
        )?;

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {}", result.bar_count);
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // ── v2.5 TCN overlay momentum: separate execution path (T-D-15, T-D-16) ────
    if let ScenarioStrategy::TcnOverlayMomentum {
        config_id,
        forecaster_id,
    } = &scenario.strategy.clone()
    {
        let config_id = config_id.clone();
        let forecaster_id = forecaster_id.clone();
        // Q-D1=(a) + Q-D2=(β): dispatch on scenario identity.
        let tcn_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let tcn_volume_map =
            build_volume_map_for_scenario(&data_root, &scenario.name, scenario.start_year);
        let tcn_input = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id: config_id.clone(),
            forecaster_id: forecaster_id.clone(),
            bars_override: realdata_bars_for_tcn.take(),
            emit_equity_bin: args.emit_equity_bin.clone(),
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) + Q-D2=(β) wiring.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: tcn_slippage_model,
                volume_usd_per_symbol: tcn_volume_map.clone(),
            },
            funding_override: None,
            basis_override: None,
        };
        // Keep a report-only copy of the input (without the moved bars/equity_bin).
        let tcn_input_for_report = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id,
            forecaster_id,
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: tcn_slippage_model,
                volume_usd_per_symbol: tcn_volume_map,
            },
            funding_override: None,
            basis_override: None,
        };
        // Bug #63 — CLI uses no-op cancel + progress so byte-identical to pre-fix.
        let (_h, t_cancel) = backtest::cancel::cancellation_pair();
        let t_progress = backtest::progress::ProgressSender::disabled();
        let result =
            backtest::scenarios::tcn_overlay::run(tcn_input, seed, t_cancel, t_progress).await?;

        // T-D-8: --reports-dir override (default: resolved from scenario name).
        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        // T-D-10/11: pass revision SHA + loaded bar info.
        let rev_sha = realdata_revision_sha_for_tcn.as_deref().unwrap_or("n/a");
        let loaded_info = realdata_revision_sha_for_tcn.as_ref().map(|_| {
            let expected = scenario.bar_count
                * backtest::scenarios::momentum::top10_symbols_with_prices().len();
            (result.bar_count, expected)
        });

        backtest::report::tcn_overlay::write(
            &tcn_input_for_report,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
            loaded_info,
        )?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            tcn_input_for_report.start_year,
        )?;

        // T-D-8: --emit-equity-bin (strictly additive; report body unchanged).
        if let Some(ref eq_path) = args.emit_equity_bin {
            let eq_text: String = result
                .equity_curve
                .iter()
                .map(|d| format!("{d}\n"))
                .collect();
            std::fs::write(eq_path, eq_text)
                .with_context(|| format!("writing equity bin to {}", eq_path.display()))?;
            info!(path = %eq_path.display(), len = result.equity_curve.len(), "equity bin written");
        }

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {}", result.bar_count);
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!(
            "Modulation   : dampened={} passed_through={} warming_up={}",
            result.dampened_signals, result.passed_through_signals, result.warmup_signals,
        );
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // ── v2.5 TCN overlay momentum with real anchor weights (M3) ──────────────
    // Requires `--features candle`.  Without it the binary emits a clear error.
    if let ScenarioStrategy::TcnOverlayMomentumWeights {
        config_id,
        forecaster_id,
    } = &scenario.strategy.clone()
    {
        let config_id = config_id.clone();
        let forecaster_id = forecaster_id.clone();
        // Q-D1=(a) + Q-D2=(β): dispatch on scenario identity.
        let tcnw_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let tcnw_volume_map =
            build_volume_map_for_scenario(&data_root, &scenario.name, scenario.start_year);
        let tcn_w_input = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id: config_id.clone(),
            forecaster_id: forecaster_id.clone(),
            bars_override: realdata_bars_for_tcn.take(),
            emit_equity_bin: args.emit_equity_bin.clone(),
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) + Q-D2=(β) wiring.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: tcnw_slippage_model,
                volume_usd_per_symbol: tcnw_volume_map.clone(),
            },
            funding_override: None,
            basis_override: None,
        };
        let tcn_w_input_for_report = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id,
            forecaster_id,
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: tcnw_slippage_model,
                volume_usd_per_symbol: tcnw_volume_map,
            },
            funding_override: None,
            basis_override: None,
        };
        let result = backtest::scenarios::tcn_overlay_weights::run(tcn_w_input, seed).await?;

        // T-D-8: --reports-dir override (default: resolved from scenario name).
        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        // T-D-10/11: pass revision SHA + loaded bar info.
        let rev_sha = realdata_revision_sha_for_tcn.as_deref().unwrap_or("n/a");
        let loaded_info = realdata_revision_sha_for_tcn.as_ref().map(|_| {
            let expected = scenario.bar_count
                * backtest::scenarios::momentum::top10_symbols_with_prices().len();
            (result.bar_count, expected)
        });

        backtest::report::tcn_overlay::write(
            &tcn_w_input_for_report,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
            loaded_info,
        )?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            tcn_w_input_for_report.start_year,
        )?;

        // T-D-8: --emit-equity-bin (strictly additive; report body unchanged).
        if let Some(ref eq_path) = args.emit_equity_bin {
            let eq_text: String = result
                .equity_curve
                .iter()
                .map(|d| format!("{d}\n"))
                .collect();
            std::fs::write(eq_path, eq_text)
                .with_context(|| format!("writing equity bin to {}", eq_path.display()))?;
            info!(path = %eq_path.display(), len = result.equity_curve.len(), "equity bin written");
        }

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {}", result.bar_count);
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!(
            "Modulation   : dampened={} passed_through={} warming_up={}",
            result.dampened_signals, result.passed_through_signals, result.warmup_signals,
        );
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // ── v2.5a PatchTST overlay momentum with real anchor weights (Wave D T-D-N23) ──
    // Requires `--features candle`. Without it the binary emits a clear error.
    if let ScenarioStrategy::PatchtstOverlayMomentumWeights {
        config_id,
        forecaster_id,
    } = &scenario.strategy.clone()
    {
        let config_id = config_id.clone();
        let forecaster_id = forecaster_id.clone();
        // Q-D1=(a) + Q-D2=(β): dispatch on scenario identity.
        let ptst_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let ptst_volume_map =
            build_volume_map_for_scenario(&data_root, &scenario.name, scenario.start_year);
        let patchtst_input = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id: config_id.clone(),
            forecaster_id: forecaster_id.clone(),
            bars_override: realdata_bars_for_tcn.take(),
            emit_equity_bin: args.emit_equity_bin.clone(),
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) + Q-D2=(β) wiring.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: ptst_slippage_model,
                volume_usd_per_symbol: ptst_volume_map.clone(),
            },
            funding_override: None,
            basis_override: None,
        };
        let patchtst_input_for_report = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id,
            forecaster_id,
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: ptst_slippage_model,
                volume_usd_per_symbol: ptst_volume_map,
            },
            funding_override: None,
            basis_override: None,
        };
        let result =
            backtest::scenarios::patchtst_overlay_weights::run(patchtst_input, seed).await?;

        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        let rev_sha = realdata_revision_sha_for_tcn.as_deref().unwrap_or("n/a");
        let loaded_info = realdata_revision_sha_for_tcn.as_ref().map(|_| {
            let expected = scenario.bar_count
                * backtest::scenarios::momentum::top10_symbols_with_prices().len();
            (result.bar_count, expected)
        });

        backtest::report::tcn_overlay::write(
            &patchtst_input_for_report,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
            loaded_info,
        )?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            patchtst_input_for_report.start_year,
        )?;

        if let Some(ref eq_path) = args.emit_equity_bin {
            let eq_text: String = result
                .equity_curve
                .iter()
                .map(|d| format!("{d}\n"))
                .collect();
            std::fs::write(eq_path, eq_text)
                .with_context(|| format!("writing equity bin to {}", eq_path.display()))?;
            info!(path = %eq_path.display(), len = result.equity_curve.len(), "equity bin written");
        }

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {}", result.bar_count);
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!(
            "Modulation   : dampened={} passed_through={} warming_up={}",
            result.dampened_signals, result.passed_through_signals, result.warmup_signals,
        );
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // ── GARCH vol-targeting overlay dispatch ───────────────────────────────────
    // v3.0.0-volatility R6.a primary — GARCH(1,1) vol-targeting overlay on v1 momentum.
    if let ScenarioStrategy::GarchVolTargetOverlayMomentum {
        config_id,
        forecaster_id,
    } = &scenario.strategy.clone()
    {
        let config_id = config_id.clone();
        let forecaster_id = forecaster_id.clone();
        // Q-D1=(a) + Q-D2=(β): dispatch on scenario identity.
        let vt_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let vt_volume_map =
            build_volume_map_for_scenario(&data_root, &scenario.name, scenario.start_year);
        let vol_target_input = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id: config_id.clone(),
            forecaster_id: forecaster_id.clone(),
            bars_override: realdata_bars_for_tcn.take(),
            emit_equity_bin: args.emit_equity_bin.clone(),
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) + Q-D2=(β) wiring.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: vt_slippage_model,
                volume_usd_per_symbol: vt_volume_map.clone(),
            },
            funding_override: None,
            basis_override: None,
        };
        let vol_target_input_for_report = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id,
            forecaster_id,
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: vt_slippage_model,
                volume_usd_per_symbol: vt_volume_map,
            },
            funding_override: None,
            basis_override: None,
        };
        let result =
            backtest::scenarios::garch_vol_target_overlay::run(vol_target_input, seed).await?;

        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        let rev_sha = realdata_revision_sha_for_tcn.as_deref().unwrap_or("n/a");
        let loaded_info = realdata_revision_sha_for_tcn.as_ref().map(|_| {
            let expected = scenario.bar_count
                * backtest::scenarios::momentum::top10_symbols_with_prices().len();
            (result.bar_count, expected)
        });

        backtest::report::tcn_overlay::write(
            &vol_target_input_for_report,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
            loaded_info,
        )?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            vol_target_input_for_report.start_year,
        )?;

        if let Some(ref eq_path) = args.emit_equity_bin {
            let eq_text: String = result
                .equity_curve
                .iter()
                .map(|d| format!("{d}\n"))
                .collect();
            std::fs::write(eq_path, eq_text)
                .with_context(|| format!("writing equity bin to {}", eq_path.display()))?;
            info!(path = %eq_path.display(), len = result.equity_curve.len(), "equity bin written");
        }

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {}", result.bar_count);
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!(
            "Vol-target   : signals_scaled={} passthrough={} no_model={}",
            result.dampened_signals, result.passed_through_signals, result.warmup_signals,
        );
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // ── v3.0.0-regime RegimeDispatcher momentum dispatch ─────────────────────
    // Wave E T-D-E1: Markov-switching 4-state dispatcher on v1 momentum.
    if let ScenarioStrategy::RegimeDispatcherMomentum { config_id } = &scenario.strategy.clone() {
        let config_id = config_id.clone();
        // Q-D1=(a) + Q-D2=(β): dispatch on scenario identity.
        let regime_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
        let regime_volume_map =
            build_volume_map_for_scenario(&data_root, &scenario.name, scenario.start_year);
        let regime_input = backtest::cli_types::TcnScenarioInput {
            scenario_name: scenario.name.clone(),
            start_year: scenario.start_year,
            bar_count: scenario.bar_count,
            initial_capital: scenario.initial_capital,
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            config_id: config_id.clone(),
            forecaster_id: "regime-dispatcher-markov-4state".to_string(),
            bars_override: realdata_bars_for_tcn.take(),
            emit_equity_bin: args.emit_equity_bin.clone(),
            // v5-latency-slippage-sim v0.5.0: Q-D1=(a) + Q-D2=(β) wiring.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
                latency_ms_min: args.sim_latency_ms_min,
                latency_ms_max: args.sim_latency_ms_max,
                slippage_model: regime_slippage_model,
                volume_usd_per_symbol: regime_volume_map,
            },
            funding_override: None,
            basis_override: None,
        };
        let result =
            backtest::scenarios::regime_dispatcher::run(regime_input.clone(), seed).await?;

        let report_dir = args
            .reports_dir
            .clone()
            .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
        std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
        let now = OffsetDateTime::now_utc();
        let stamp = format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

        let rev_sha = realdata_revision_sha_for_tcn.as_deref().unwrap_or("n/a");

        backtest::report::regime_dispatcher::write(
            &regime_input,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
        )?;
        backtest::report::write_equity_companion(
            &report_path,
            &result.equity_curve,
            regime_input.start_year,
        )?;

        // Emit equity bin for sharpe_comparison subprocess use.
        if let Some(ref eq_path) = args.emit_equity_bin {
            let eq_text: String = result
                .equity_curve
                .iter()
                .map(|d| format!("{d}\n"))
                .collect();
            std::fs::write(eq_path, eq_text)
                .with_context(|| format!("writing equity bin to {}", eq_path.display()))?;
            info!(path = %eq_path.display(), len = result.equity_curve.len(), "equity bin written");
        }

        println!("Report written: {}", report_path.display());
        println!("Scenario     : {}", args.scenario);
        println!("Bars (total) : {}", result.bar_count);
        println!("Trades       : {}", result.trades);
        println!("Final equity : ${:.2} USDT", result.final_equity);
        println!("Elapsed      : {:.1}s", result.elapsed_secs);
        println!(
            "Dispatcher   : suppressed={} momentum={} warmup={}",
            result.suppressed_bars, result.momentum_bars, result.warmup_bars,
        );
        println!("Data source  : {data_source}");
        return Ok(());
    }

    // Wave D-2 / T-AR-4: Resolve strategy_id for the extracted bar loop.
    // Priority: CLI `--strategy` flag → scenario default.
    // `None` for SmaCrossover → normalised to "sma_crossover" below.
    let effective_strategy_id: String =
        args.strategy
            .clone()
            .unwrap_or_else(|| match &scenario.strategy {
                ScenarioStrategy::Composed { id } => id.clone(),
                ScenarioStrategy::SmaCrossover { .. } => "sma_crossover".to_string(),
                ScenarioStrategy::Momentum { .. } => unreachable!("handled above"),
                ScenarioStrategy::MeanReversionPairs { .. } => unreachable!("handled above"),
                ScenarioStrategy::TcnOverlayMomentum { .. } => unreachable!("handled above"),
                ScenarioStrategy::TcnOverlayMomentumWeights { .. } => unreachable!("handled above"),
                ScenarioStrategy::PatchtstOverlayMomentumWeights { .. } => {
                    unreachable!("handled above")
                }
                ScenarioStrategy::GarchVolTargetOverlayMomentum { .. } => {
                    unreachable!("handled above")
                }
                ScenarioStrategy::RegimeDispatcherMomentum { .. } => {
                    unreachable!("handled above")
                }
            });

    // Wave D-2 / T-AR-4: delegate to the extracted sma_composed_run module.
    // Passing `bars_override = Some(bars)` ensures the CLI uses the same
    // pre-generated bar stream as before (anchor-preserving behaviour move).
    // Q-D1=(a): SMA/Composed scenarios are always synthetic → Linear{bps:8} fallback.
    let sma_slippage_model = build_slippage_model_for_scenario(&args, &scenario.name);
    let sma_run_input = backtest::cli_types::SmaComposedRunInput {
        strategy_id: effective_strategy_id,
        symbol: scenario.symbol.clone(),
        start_year: scenario.start_year,
        bar_count: scenario.bar_count,
        initial_capital: scenario.initial_capital,
        slippage_bps: scenario.slippage_bps,
        taker_fee_bps: scenario.taker_fee_bps,
        // lab-polish-round-2 R2 — CLI scenarios always pass None to preserve
        // anchored byte-identity (defaults map back to the hardcoded 20/50
        // pair inside `sma_composed_run::run`).
        sma_fast_len: None,
        sma_slow_len: None,
        // v5-latency-slippage-sim v0.5.0: Q-D1=(a) dispatch.
        // SMA/Composed scenarios are always synthetic → Linear{bps:8} fallback.
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig {
            latency_ms_min: args.sim_latency_ms_min,
            latency_ms_max: args.sim_latency_ms_max,
            slippage_model: sma_slippage_model,
            volume_usd_per_symbol: None,
        },
    };
    // CLI path: use no-op cancel + progress so the anchor bytes are unchanged.
    let (_cancel_handle, cancel_rx) = backtest::cancel::cancellation_pair();
    let progress_tx = backtest::progress::ProgressSender::disabled();
    let result = backtest::scenarios::sma_composed_run::run(
        &sma_run_input,
        Some(bars),
        seed,
        cancel_rx,
        progress_tx,
    )
    .await?;

    let final_equity = result.final_equity;
    let elapsed = result.elapsed_secs;
    let strategy_meta = result.strategy_meta.clone();
    let state = &result.state;

    // ── Write report ───────────────────────────────────────────────────────────
    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    // T-D-8: --reports-dir override (additive; default unchanged).
    let report_dir = args
        .reports_dir
        .clone()
        .unwrap_or_else(|| report_dir_for_scenario(&args.scenario));
    std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
    let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

    let sma_input = backtest::cli_types::SmaScenarioInput {
        scenario_name: scenario.name.clone(),
        body_name: scenario.body_name.clone(),
        body_elapsed_override: scenario.body_elapsed_override,
        symbol: scenario.symbol.clone(),
        start_year: scenario.start_year,
        initial_capital: scenario.initial_capital,
        slippage_bps: scenario.slippage_bps,
        taker_fee_bps: scenario.taker_fee_bps,
        baseline_report: scenario.baseline_report.clone(),
    };
    backtest::report::sma::write(
        &sma_input,
        state,
        scenario.initial_capital,
        final_equity,
        seed,
        &data_source,
        elapsed,
        &report_path,
        &strategy_meta,
        None, // Binance/synthetic path — None preserves 33 existing SMA anchors byte-identically
    )?;
    backtest::report::write_equity_companion(
        &report_path,
        &result.equity_curve,
        scenario.start_year,
    )?;

    println!("Report written: {}", report_path.display());
    println!("Scenario     : {}", args.scenario);
    println!("Bars         : {bar_count}");
    println!("Trades       : {}", state.trades);
    println!("Final equity : ${final_equity:.2} USDT");
    println!("Elapsed      : {elapsed:.1}s");
    println!("Data source  : {data_source}");
    println!("Ledger imbal : {}", state.ledger_imbalance_events);

    Ok(())
}

/// Find the filename of the most recent backtest report for a given scenario.
fn find_latest_report(dir: &Path, scenario: &str) -> Option<String> {
    if !dir.exists() {
        return None;
    }
    let mut candidates: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("backtest-") && name.contains(scenario) && name.ends_with(".md") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    candidates.sort();
    candidates.into_iter().last()
}

/// Map a backtest scenario name to the feature slug that owns its
/// reports. Fixed mapping (each scenario was first locked into
/// `spec/anchors.toml` under exactly one feature). Future scenarios
/// that don't match return `"_unknown"`, which causes
/// [`report_dir_for_scenario`] to write under `spec/_unknown/reports/`
/// so orphaned reports surface immediately.
fn scenario_to_feature(scenario: &str) -> &'static str {
    match scenario {
        "btc-2023-1m-sma-cross"
        | "btc-2023-1m-sma-baseline-refresh"
        | "btc-2024-h1-sma-cross"
        | "eth-2024-h1-sma-cross" => "v0-paper-sma",
        "btc-2023-1m-macd-trend"
        | "btc-2023-1m-rsi-reversion"
        | "btc-2023-1m-bbands-mean-revert" => "v05-composed-strategies",
        "top10-2023-1h-momentum" | "top10-2024-h1-momentum" => "v1-cross-sectional-momentum",
        "pairs-2023-zscore-mr" | "pairs-2024-h1-zscore-mr" => "v15a-mean-reversion-pairs",
        "top10-2023-fy-tcn-overlay"
        | "top10-2024-fy-tcn-overlay"
        | "top10-2023-fy-tcn-overlay-weights"
        | "top10-2024-fy-tcn-overlay-weights" => "v25-tcn-overlay",
        // v2.6.0-realdata scenarios (ADR-0032).
        "top10-2023-fy-tcn-overlay-realdata"
        | "top10-2024-fy-tcn-overlay-realdata"
        | "top10-2023-fy-tcn-overlay-weights-realdata"
        | "top10-2024-fy-tcn-overlay-weights-realdata" => "backtest-real-binance-data",
        // v2.5a PatchTST overlay realdata scenario (T-D-N23).
        "top10-2023-fy-patchtst-overlay-realdata" => "v25a-patchtst-overlay",
        // v3.0.0-volatility GARCH vol-targeting overlay realdata scenario (T-D-N24).
        "top10-2023-fy-vol-target-overlay-realdata" => "v3-volatility-forecaster",
        // v3.0.0-volatility-rebaseline: un-targeted v1 momentum on real Binance data.
        "top10-2023-fy-momentum-realdata" => "v3-volatility-forecaster-rebaseline",
        // v3.0.0-regime RegimeDispatcher scenarios (Wave E T-D-E1).
        "top10-2023-fy-regime-dispatcher-realdata" | "top10-2024-fy-regime-dispatcher-realdata" => {
            "v3-regime-classifier"
        }
        _ => "_unknown",
    }
}

/// Resolve the per-feature `spec/<feature>/reports/` directory for a
/// given scenario. Caller is responsible for `create_dir_all`.
fn report_dir_for_scenario(scenario: &str) -> PathBuf {
    PathBuf::from("spec")
        .join(scenario_to_feature(scenario))
        .join("reports")
}
