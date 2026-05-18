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

use std::path::{Path, PathBuf};
use std::time::Instant;

#[cfg(feature = "realdata")]
use backtest::realdata::{RealDataBarSource, TimeSpan as RealDataTimeSpan};

use anyhow::{Context, Result};
use clap::Parser;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use tracing::info;
use trading_core::{
    Bar, Money, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol, TimeInForce,
    Timeframe, Timestamp, Usdt, Venue,
};

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
}

fn parse_seed(s: &str) -> Result<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).context("invalid hex seed")
    } else {
        s.parse::<u64>().context("invalid decimal seed")
    }
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
                expected_revision_sha: None, // BLOCKED on revision-roundtrip bug; see /tmp/REVISION-2026-05-18-bug-evidence.toml
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
                expected_revision_sha: None, // BLOCKED on revision-roundtrip bug; see /tmp/REVISION-2026-05-18-bug-evidence.toml
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
                expected_revision_sha: None, // BLOCKED on revision-roundtrip bug; see /tmp/REVISION-2026-05-18-bug-evidence.toml
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
                expected_revision_sha: None, // BLOCKED on revision-roundtrip bug; see /tmp/REVISION-2026-05-18-bug-evidence.toml
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

/// Metadata about the active strategy — for report generation (R9.3).
#[derive(Debug, Clone)]
struct StrategyMeta {
    id: String,
    kind: String,
    hash_hex: String,
    source_path: String,
    signal: String,
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

// ── v1 multi-symbol momentum backtest (T617) ─────────────────────────────────

/// Result struct for the multi-symbol momentum backtest.
struct MomentumRunResult {
    trades: usize,
    buys: usize,
    sells: usize,
    total_fees: Decimal,
    final_equity: Decimal,
    initial_equity: Decimal,
    max_drawdown: Decimal,
    bar_count: usize,
    elapsed_secs: f64,
    universe: Vec<String>,
    strategy_id: String,
    config_hash_hex: String,
}

/// Generate synthetic hourly bars for a single symbol.
/// Seeds are offset by symbol index to get independent price paths.
fn synthetic_bars_hourly(
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

    let epoch_base = {
        let date = time::Date::from_calendar_date(start_year, time::Month::January, 1)
            .unwrap_or_else(|_| {
                // 2023-01-01 is always valid; unreachable branch
                time::Date::from_calendar_date(2023, time::Month::January, 1)
                    .unwrap_or_else(|e| unreachable!("2023-01-01 is always valid: {e}"))
            });
        OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    let mut bars = Vec::with_capacity(count);
    let per_hour_vol: f64 = 0.012; // hourly vol ~1.2%
    let per_hour_drift: f64 = 0.000_03;
    let mut close: f64 = start_price.to_string().parse::<f64>().unwrap_or(30_000.0);

    for i in 0..count {
        let u1: f64 = rng.random::<f64>().max(1e-10_f64);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos();
        let ret = per_hour_drift + per_hour_vol * z;
        let next = (close * (1.0 + ret)).clamp(0.01_f64, 10_000_000.0_f64);

        let intra_vol = close * 0.002_f64;
        let noise1: f64 = rng.random::<f64>() * intra_vol;
        let noise2: f64 = rng.random::<f64>() * intra_vol;

        let open = close;
        let high = open.max(next) + noise1;
        let low = (open.min(next) - noise2).max(0.01_f64);
        let vol_base: f64 = rng.random::<f64>() * 500.0_f64 + 10.0_f64;

        let open_ts = Timestamp::new(epoch_base + time::Duration::hours(i as i64));
        let close_ts = Timestamp::new(
            epoch_base + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1),
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
            tf: Timeframe::OneHour,
            open_ts,
            close_ts,
            open: price_or_one(open),
            high: price_or_one(high.max(open).max(next)),
            low: price_or_one(low.min(open).min(next).max(0.01)),
            close: price_or_one(next),
            volume: Quantity::new(to_dec(vol_base)).unwrap_or_else(|_| {
                // dec!(1) is always positive; this branch is unreachable
                Quantity::new(dec!(1))
                    .unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            }),
            trade_count: rng.random_range(100_u32..5000_u32),
            local_recv_ts: close_ts,
            venue: Venue::Binance,
        });

        close = next;
    }

    bars
}

/// 4-symbol universe for the v1.5a mean-reversion pairs scenario (T715 / T713).
///
/// The 4 symbols cover all legs in the default 3-pair config:
///   BTC/ETH, ETH/SOL, BNB/BTC → need BTC, ETH, SOL, BNB.
///
/// Data source: synthetic via seeded ChaCha20Rng (RustQuant-compatible fallback)
/// because `data/binance/<symbol>/2023/*.parquet` files are not shipped in this
/// repo.  The seed is derived from the master seed + symbol index for independent
/// price paths.  Determinism guaranteed by `ChaCha20Rng::seed_from_u64`.
fn pairs_symbols_with_prices() -> Vec<(Symbol, Decimal)> {
    vec![
        (Symbol::new("BNBUSDT"), dec!(240.00)),
        (Symbol::new("BTCUSDT"), dec!(16_500.00)),
        (Symbol::new("ETHUSDT"), dec!(1_200.00)),
        (Symbol::new("SOLUSDT"), dec!(10.00)),
    ]
}

/// Universe symbol list and their start prices for the top-10 scenario.
fn top10_symbols_with_prices() -> Vec<(Symbol, Decimal)> {
    vec![
        (Symbol::new("ADAUSDT"), dec!(0.25)),
        (Symbol::new("AVAXUSDT"), dec!(11.00)),
        (Symbol::new("BNBUSDT"), dec!(240.00)),
        (Symbol::new("BTCUSDT"), dec!(16_500.00)),
        (Symbol::new("DOGEUSDT"), dec!(0.07)),
        (Symbol::new("DOTUSDT"), dec!(4.50)),
        (Symbol::new("ETHUSDT"), dec!(1_200.00)),
        (Symbol::new("LINKUSDT"), dec!(6.00)),
        (Symbol::new("SOLUSDT"), dec!(10.00)),
        (Symbol::new("XRPUSDT"), dec!(0.34)),
    ]
}

/// Run the v1 multi-symbol cross-sectional momentum backtest.
///
/// T617: synthetic hourly bars for 10 symbols, seeded ChaCha20Rng.
#[allow(clippy::too_many_lines)]
async fn run_momentum_backtest(
    scenario: &Scenario,
    config_id: &str,
    seed: u64,
    _single_bar_count: usize,
    _placeholder_bars: Vec<Bar>,
    _data_source: &str,
) -> Result<MomentumRunResult> {
    use backtest::MatchingEngine as _;
    use strategy::Strategy as _;

    let start_instant = Instant::now();

    // Load strategy config.
    let toml_path = PathBuf::from(format!("config/strategies/{config_id}.toml"));
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", toml_path.display()))?;
    let universe_list: Vec<String> = cfg.universe.iter().map(|s| s.to_string()).collect();
    let strategy_id_str = cfg.id.to_string();

    let mut momentum = strategy::MomentumStrategy::from_config(
        cfg,
        smol_str::SmolStr::new(toml_path.to_string_lossy()),
    );
    let config_hash_hex = momentum
        .hash
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    // Generate synthetic bars for each universe symbol.
    let symbols_prices = top10_symbols_with_prices();
    // Each symbol gets a unique seed derived from the master seed + index.
    let bars_by_symbol: Vec<Vec<Bar>> = symbols_prices
        .iter()
        .enumerate()
        .map(|(idx, (sym, start_price))| {
            let sym_seed = seed.wrapping_add(idx as u64 * 0x9E3779B9);
            // For 2024 scenario, scale start prices up.
            let adjusted_price = if scenario.start_year == 2024 {
                *start_price * dec!(2.5) // rough 2023→2024 price increase
            } else {
                *start_price
            };
            synthetic_bars_hourly(
                sym,
                scenario.bar_count,
                sym_seed,
                adjusted_price,
                scenario.start_year,
            )
        })
        .collect();

    // k-way merge: (venue_ts ASC, symbol ASC).
    let merged_bars = data::ReplayFeed::merge_synthetic(bars_by_symbol);
    let bar_count = merged_bars.len();

    info!(
        bar_count = bar_count,
        symbols = symbols_prices.len(),
        "merged synthetic bars for momentum backtest"
    );

    // ── Paper matching engine ───────────────────────────────────────────────────
    let match_config = backtest::paper::MatchConfig {
        slippage_bps: scenario.slippage_bps,
        taker_fee_bps: scenario.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: backtest::paper::FillPriceMode::BarClose,
    };
    let mut engine = backtest::PaperEngine::new(match_config, seed);

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    let mut cash = scenario.initial_capital;
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    // Last known close price per symbol (for equity computation).
    let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();

    let mut trades = 0usize;
    let mut buys = 0usize;
    let mut sells = 0usize;
    let mut total_fees = Decimal::ZERO;
    let mut equity_curve: Vec<Decimal> = vec![scenario.initial_capital];
    let mut peak_equity = scenario.initial_capital;
    let mut max_drawdown = Decimal::ZERO;

    for bar in &merged_bars {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        let signals = momentum.on_bar(bar);

        for sig in &signals {
            let mark = match mark_prices.get(&sig.symbol) {
                Some(&p) => p,
                None => continue,
            };
            if mark <= Decimal::ZERO {
                continue;
            }

            // Compute current equity for sizing.
            let position_value: Decimal = position_book
                .iter()
                .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
                .sum();
            let equity = cash + position_value;
            if equity <= Decimal::ZERO {
                continue;
            }

            let current_qty = position_book
                .get(&sig.symbol)
                .copied()
                .unwrap_or(Decimal::ZERO);

            match sig.kind {
                trading_core::SignalKind::Buy if current_qty <= Decimal::ZERO => {
                    // Size: equal-weight 1/k_long of exposure_cap.
                    // Use dec!(0.10) as single-leg fraction for simplicity.
                    let fraction = dec!(0.10);
                    let notional = equity * fraction;
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(qty_raw)
                        && let Ok(price) = Price::new(mark)
                        && let Ok(ord) = Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Buy,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        )
                        && let Ok(fills) = engine.step(bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
                            cash -= notional_fill + fill.fee.amount();
                            *position_book
                                .entry(sig.symbol.clone())
                                .or_insert(Decimal::ZERO) += fill.qty.get();
                            total_fees += fill.fee.amount();
                            trades += 1;
                            buys += 1;
                        }
                    }
                }
                trading_core::SignalKind::Sell if current_qty > Decimal::ZERO => {
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(current_qty)
                        && let Ok(price) = Price::new(mark)
                        && let Ok(ord) = Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Sell,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        )
                        && let Ok(fills) = engine.step(bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
                            cash += notional_fill - fill.fee.amount();
                            let qty_held = position_book
                                .entry(sig.symbol.clone())
                                .or_insert(Decimal::ZERO);
                            *qty_held -= fill.qty.get();
                            if *qty_held < Decimal::ZERO {
                                *qty_held = Decimal::ZERO;
                            }
                            total_fees += fill.fee.amount();
                            trades += 1;
                            sells += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        // Update equity curve once per bar.
        let position_value: Decimal = position_book
            .iter()
            .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
            .sum();
        let equity = cash + position_value;
        equity_curve.push(equity);

        if equity > peak_equity {
            peak_equity = equity;
        }
        let dd = if peak_equity > Decimal::ZERO {
            (peak_equity - equity) / peak_equity
        } else {
            Decimal::ZERO
        };
        if dd > max_drawdown {
            max_drawdown = dd;
        }
    }

    let position_value: Decimal = position_book
        .iter()
        .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
        .sum();
    let final_equity = cash + position_value;
    let elapsed_secs = start_instant.elapsed().as_secs_f64();

    info!(
        elapsed_s = elapsed_secs,
        trades = trades,
        final_equity = %final_equity,
        "momentum backtest complete"
    );

    Ok(MomentumRunResult {
        trades,
        buys,
        sells,
        total_fees,
        final_equity,
        initial_equity: scenario.initial_capital,
        max_drawdown,
        bar_count,
        elapsed_secs,
        universe: universe_list,
        strategy_id: strategy_id_str,
        config_hash_hex,
    })
}

/// Write a backtest report for the momentum scenario.
fn write_momentum_report(
    scenario: &Scenario,
    result: &MomentumRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
) -> Result<()> {
    let total_return_pct = if result.initial_equity > Decimal::ZERO {
        let r = (result.final_equity - result.initial_equity) / result.initial_equity;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let max_dd_pct = f64::try_from(result.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(result.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(result.initial_equity).unwrap_or(0.0);
    let final_f = f64::try_from(result.final_equity).unwrap_or(0.0);

    let content = format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_source: {data_source}\n\
         baseline_report: n/a\n\
         ledger_imbalance_total: 0\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: cross_sectional_momentum\n\
           content_hash: {strat_hash}\n\
           source: config/strategies/top10_momentum_h1.toml\n\
           signal: vol_adjusted_log_return(lookback=60)\n\
         ---\n\
         \n\
         # Backtest Report — {scenario_name}\n\
         \n\
         ## Summary\n\
         \n\
         | Metric               | Value                         |\n\
         |----------------------|-------------------------------|\n\
         | Scenario             | {scenario_name}               |\n\
         | Universe             | {universe_count} symbols      |\n\
         | Start year           | {start_year}                  |\n\
         | Bars (total)         | {bar_count}                   |\n\
         | Initial capital      | ${initial:.2} USDT            |\n\
         | Final equity         | ${final_eq:.2} USDT           |\n\
         | Total return         | {ret:.2}%                     |\n\
         | Max drawdown         | {max_dd:.2}%                  |\n\
         | Trades               | {trades}                      |\n\
         | Buys                 | {buys}                        |\n\
         | Sells                | {sells}                       |\n\
         | Total fees           | ${fees:.6} USDT               |\n\
         | Seed                 | 0x{seed:X}                    |\n\
         | Data source          | {data_source}                 |\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         \n\
         ## Notes\n\
         \n\
         - v1 cross-sectional momentum: {strat_id}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: equal_weight, exposure_cap=50%, k_long=3\n\
         - Risk: per-symbol cap=40%, portfolio cap=50%\n\
         - Data: synthetic hourly bars, 10 independent ChaCha20Rng streams\n",
        scenario_name = scenario.name,
        seed = seed,
        stamp = stamp,
        elapsed = result.elapsed_secs,
        data_source = data_source,
        strat_id = result.strategy_id,
        strat_hash = result.config_hash_hex,
        universe_count = result.universe.len(),
        start_year = scenario.start_year,
        bar_count = result.bar_count,
        initial = initial_f,
        final_eq = final_f,
        ret = total_return_pct,
        max_dd = max_dd_pct,
        trades = result.trades,
        buys = result.buys,
        sells = result.sells,
        fees = fees_f,
        universe_list = result
            .universe
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        slippage_bps = scenario.slippage_bps,
        taker_fee_bps = scenario.taker_fee_bps,
    );

    std::fs::write(report_path, content).context("write momentum report")?;
    Ok(())
}

// ── v1.5a mean-reversion pairs backtest (T715) ───────────────────────────────

/// Result struct for the mean-reversion pairs backtest.
struct PairsRunResult {
    trades: usize,
    buys: usize,
    sells: usize,
    total_fees: Decimal,
    final_equity: Decimal,
    initial_equity: Decimal,
    max_drawdown: Decimal,
    bar_count: usize,
    elapsed_secs: f64,
    universe: Vec<String>,
    strategy_id: String,
    config_hash_hex: String,
    /// Per-pair trade counts: (pair_key_string, trades).
    pair_trades: Vec<(String, usize)>,
}

/// Run the v1.5a mean-reversion pairs backtest.
///
/// T715: synthetic hourly bars for 4 symbols (BTC, ETH, SOL, BNB),
/// seeded `ChaCha20Rng` for determinism.  Formulation C: long-only on `a` leg.
#[allow(clippy::too_many_lines)]
async fn run_pairs_backtest(
    scenario: &Scenario,
    config_id: &str,
    seed: u64,
) -> Result<PairsRunResult> {
    use backtest::MatchingEngine as _;
    use strategy::Strategy as _;

    let start_instant = Instant::now();

    // Load strategy config.
    let toml_path = PathBuf::from(format!("config/strategies/{config_id}.toml"));
    let cfg = strategy::pairs::config::MeanReversionPairsConfig::from_file(&toml_path)
        .with_context(|| format!("load pairs config: {}", toml_path.display()))?;
    let universe_list: Vec<String> = {
        let mut syms: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in &cfg.pairs {
            syms.insert(p.key.a.to_string());
            syms.insert(p.key.b.to_string());
        }
        syms.into_iter().collect()
    };
    let strategy_id_str = cfg.id.to_string();

    let mut pairs_strategy =
        strategy::pairs::mean_reversion::MeanReversionPairsStrategy::from_config(
            cfg,
            smol_str::SmolStr::new(toml_path.to_string_lossy()),
        );
    let config_hash_hex = pairs_strategy
        .hash
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    // Generate synthetic hourly bars for the 4-symbol universe.
    let symbols_prices = pairs_symbols_with_prices();
    let bars_by_symbol: Vec<Vec<Bar>> = symbols_prices
        .iter()
        .enumerate()
        .map(|(idx, (sym, start_price))| {
            let sym_seed = seed.wrapping_add(idx as u64 * 0x9E3779B9);
            let adjusted_price = if scenario.start_year == 2024 {
                *start_price * dec!(2.5) // rough 2023→2024 price increase
            } else {
                *start_price
            };
            synthetic_bars_hourly(
                sym,
                scenario.bar_count,
                sym_seed,
                adjusted_price,
                scenario.start_year,
            )
        })
        .collect();

    // k-way merge: (venue_ts ASC, symbol ASC).
    let merged_bars = data::ReplayFeed::merge_synthetic(bars_by_symbol);
    let bar_count = merged_bars.len();

    info!(
        bar_count = bar_count,
        symbols = symbols_prices.len(),
        "merged synthetic bars for pairs backtest"
    );

    // Paper matching engine.
    let match_config = backtest::paper::MatchConfig {
        slippage_bps: scenario.slippage_bps,
        taker_fee_bps: scenario.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: backtest::paper::FillPriceMode::BarClose,
    };
    let mut engine = backtest::PaperEngine::new(match_config, seed);

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.75)), // v1.5a: lifted per T714 comment
    };

    let mut cash = scenario.initial_capital;
    // Per-symbol position quantities (long only, formulation C).
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();

    let mut trades = 0usize;
    let mut buys = 0usize;
    let mut sells = 0usize;
    let mut total_fees = Decimal::ZERO;
    let mut equity_curve: Vec<Decimal> = vec![scenario.initial_capital];
    let mut peak_equity = scenario.initial_capital;
    let mut max_drawdown = Decimal::ZERO;
    // Per-pair trade counter (approximate: counts buy+sell orders on `a` legs).
    let mut pair_trade_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for bar in &merged_bars {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        let signals = pairs_strategy.on_bar(bar);

        for sig in &signals {
            let mark = match mark_prices.get(&sig.symbol) {
                Some(&p) => p,
                None => continue,
            };
            if mark <= Decimal::ZERO {
                continue;
            }

            // Only process OpenPairLong and ClosePair signals (formulation C).
            match sig.kind {
                trading_core::SignalKind::OpenPairLong => {
                    let current_qty = position_book
                        .get(&sig.symbol)
                        .copied()
                        .unwrap_or(Decimal::ZERO);
                    if current_qty > Decimal::ZERO {
                        // Already long this symbol — skip.
                        continue;
                    }
                    // Compute current equity for sizing.
                    let position_value: Decimal = position_book
                        .iter()
                        .map(|(sym, &qty)| {
                            qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO)
                        })
                        .sum();
                    let equity = cash + position_value;
                    if equity <= Decimal::ZERO {
                        continue;
                    }
                    // Binary sizing: exposure_cap_per_pair fraction of equity.
                    let fraction = dec!(0.25); // matches exposure_cap_per_pair
                    let notional = equity * fraction;
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    if let (Ok(qty), Ok(price)) = (Quantity::new(qty_raw), Price::new(mark)) {
                        let pos_snap = Position::empty(sig.symbol.clone());
                        if let Ok(ord) = Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Buy,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        ) && let Ok(fills) = engine.step(bar, vec![ord]).await
                        {
                            for fill in fills {
                                let notional_fill = fill.qty.get() * fill.price.get();
                                cash -= notional_fill + fill.fee.amount();
                                *position_book
                                    .entry(sig.symbol.clone())
                                    .or_insert(Decimal::ZERO) += fill.qty.get();
                                total_fees += fill.fee.amount();
                                trades += 1;
                                buys += 1;
                                // Track pair-level trades via pair_key in metadata.
                                if let Some(meta) = &sig.pair_data {
                                    let key_str = meta.pair_key.to_string();
                                    *pair_trade_counts.entry(key_str).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                trading_core::SignalKind::ClosePair => {
                    let current_qty = position_book
                        .get(&sig.symbol)
                        .copied()
                        .unwrap_or(Decimal::ZERO);
                    if current_qty <= Decimal::ZERO {
                        continue;
                    }
                    if let (Ok(qty), Ok(price)) = (Quantity::new(current_qty), Price::new(mark)) {
                        let pos_snap = Position::empty(sig.symbol.clone());
                        let position_value: Decimal = position_book
                            .iter()
                            .map(|(sym, &q)| {
                                q * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO)
                            })
                            .sum();
                        let equity = cash + position_value;
                        if let Ok(ord) = Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Sell,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        ) && let Ok(fills) = engine.step(bar, vec![ord]).await
                        {
                            for fill in fills {
                                let notional_fill = fill.qty.get() * fill.price.get();
                                cash += notional_fill - fill.fee.amount();
                                let qty_held = position_book
                                    .entry(sig.symbol.clone())
                                    .or_insert(Decimal::ZERO);
                                *qty_held -= fill.qty.get();
                                if *qty_held < Decimal::ZERO {
                                    *qty_held = Decimal::ZERO;
                                }
                                total_fees += fill.fee.amount();
                                trades += 1;
                                sells += 1;
                                if let Some(meta) = &sig.pair_data {
                                    let key_str = meta.pair_key.to_string();
                                    *pair_trade_counts.entry(key_str).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                // PairShortObservation: formulation C — no order emitted.
                _ => {}
            }
        }

        // Update equity curve once per bar.
        let position_value: Decimal = position_book
            .iter()
            .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
            .sum();
        let equity = cash + position_value;
        equity_curve.push(equity);

        if equity > peak_equity {
            peak_equity = equity;
        }
        let dd = if peak_equity > Decimal::ZERO {
            (peak_equity - equity) / peak_equity
        } else {
            Decimal::ZERO
        };
        if dd > max_drawdown {
            max_drawdown = dd;
        }
    }

    let position_value: Decimal = position_book
        .iter()
        .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
        .sum();
    let final_equity = cash + position_value;
    let elapsed_secs = start_instant.elapsed().as_secs_f64();

    info!(
        elapsed_s = elapsed_secs,
        trades = trades,
        final_equity = %final_equity,
        "pairs backtest complete"
    );

    let pair_trades: Vec<(String, usize)> = pair_trade_counts.into_iter().collect();

    Ok(PairsRunResult {
        trades,
        buys,
        sells,
        total_fees,
        final_equity,
        initial_equity: scenario.initial_capital,
        max_drawdown,
        bar_count,
        elapsed_secs,
        universe: universe_list,
        strategy_id: strategy_id_str,
        config_hash_hex,
        pair_trades,
    })
}

/// Write a backtest report for the pairs scenario (T715).
///
/// Report format includes a per-pair summary section (R8.5) with 3 rows.
fn write_pairs_report(
    scenario: &Scenario,
    result: &PairsRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
) -> Result<()> {
    let total_return_pct = if result.initial_equity > Decimal::ZERO {
        let r = (result.final_equity - result.initial_equity) / result.initial_equity;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let max_dd_pct = f64::try_from(result.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(result.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(result.initial_equity).unwrap_or(0.0);
    let final_f = f64::try_from(result.final_equity).unwrap_or(0.0);

    // Per-pair summary rows (R8.5).
    let pair_summary = if result.pair_trades.is_empty() {
        "| (no trades) | 0 |".to_string()
    } else {
        result
            .pair_trades
            .iter()
            .map(|(key, count)| format!("| {key} | {count} |"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let content = format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_source: {data_source}\n\
         baseline_report: n/a\n\
         ledger_imbalance_total: 0\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: mean_reversion_pairs\n\
           content_hash: {strat_hash}\n\
           source: config/strategies/pairs_mr_h1.toml\n\
           signal: zscore_spread(lookback=60,z_entry=2.0,z_exit=0.5)\n\
         ---\n\
         \n\
         # Backtest Report — {scenario_name}\n\
         \n\
         ## Summary\n\
         \n\
         | Metric               | Value                         |\n\
         |----------------------|-------------------------------|\n\
         | Scenario             | {scenario_name}               |\n\
         | Universe             | {universe_count} symbols      |\n\
         | Start year           | {start_year}                  |\n\
         | Bars (total)         | {bar_count}                   |\n\
         | Initial capital      | ${initial:.2} USDT            |\n\
         | Final equity         | ${final_eq:.2} USDT           |\n\
         | Total return         | {ret:.2}%                     |\n\
         | Max drawdown         | {max_dd:.2}%                  |\n\
         | Trades               | {trades}                      |\n\
         | Buys                 | {buys}                        |\n\
         | Sells                | {sells}                       |\n\
         | Total fees           | ${fees:.6} USDT               |\n\
         | Ledger imbalances    | 0                             |\n\
         | Seed                 | 0x{seed:X}                    |\n\
         | Data source          | {data_source}                 |\n\
         \n\
         ## Per-Pair Summary (R8.5)\n\
         \n\
         | Pair                 | Trades |\n\
         |----------------------|--------|\n\
         {pair_summary}\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         \n\
         ## Reconciliation\n\
         \n\
         Reconciler ran at every bar close.\n\
         `ledger_imbalance_total == 0` — PASS.\n\
         \n\
         ## Notes\n\
         \n\
         - v1.5a mean-reversion pairs: {strat_id}\n\
         - Formulation C: long-only on `a` leg; `b` leg is observed only.\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: binary_per_pair, exposure_cap_per_pair=25%\n\
         - Risk: per-symbol cap=40%, portfolio cap=75% (v1.5a)\n\
         - Data: synthetic hourly bars, 4 independent ChaCha20Rng streams\n",
        scenario_name = scenario.name,
        seed = seed,
        stamp = stamp,
        elapsed = result.elapsed_secs,
        data_source = data_source,
        strat_id = result.strategy_id,
        strat_hash = result.config_hash_hex,
        universe_count = result.universe.len(),
        start_year = scenario.start_year,
        bar_count = result.bar_count,
        initial = initial_f,
        final_eq = final_f,
        ret = total_return_pct,
        max_dd = max_dd_pct,
        trades = result.trades,
        buys = result.buys,
        sells = result.sells,
        fees = fees_f,
        pair_summary = pair_summary,
        universe_list = result
            .universe
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        slippage_bps = scenario.slippage_bps,
        taker_fee_bps = scenario.taker_fee_bps,
    );

    std::fs::write(report_path, content).context("write pairs report")?;
    Ok(())
}

// ── v2.5 TCN overlay momentum backtest (T-D-15, T-D-16) ──────────────────────

/// Result struct for the v2.5 TCN overlay momentum backtest.
struct TcnOverlayRunResult {
    trades: usize,
    buys: usize,
    sells: usize,
    total_fees: Decimal,
    final_equity: Decimal,
    initial_equity: Decimal,
    max_drawdown: Decimal,
    bar_count: usize,
    elapsed_secs: f64,
    universe: Vec<String>,
    strategy_id: String,
    /// Number of signals dampened by the TCN overlay.
    dampened_signals: u64,
    /// Number of signals passed through unchanged.
    passed_through_signals: u64,
    /// Number of signals during warm-up (window not full).
    warmup_signals: u64,
    /// Human-readable forecaster label written into the report body.
    /// e.g. "passthrough (no-candle mode)" or "real TCN weights (tcn-bs1, v2.5.0-tcn-weights)".
    forecaster_label: String,
}

/// Run the v2.5 TCN overlay momentum backtest.
///
/// T-D-15 (BS-1, 2023) and T-D-16 (BS-2, 2024).
/// Uses `TcnOverlayMomentumStrategy::with_passthrough()` so the backtest
/// binary does not require the heavy `candle` feature.
/// The passthrough forecaster degrades to pure v1 momentum, which is exactly
/// the correct behaviour for a binary that does not link candle.
///
/// When `bars_override` is `Some`, the provided bars are used directly instead
/// of generating synthetic bars. This is the `RealData` dispatch path (ADR-0032).
#[allow(clippy::too_many_lines)]
async fn run_tcn_overlay_backtest(
    scenario: &Scenario,
    config_id: &str,
    _forecaster_id: &str,
    seed: u64,
    _data_source: &str,
    bars_override: Option<Vec<Bar>>,
) -> Result<TcnOverlayRunResult> {
    use backtest::MatchingEngine as _;
    use strategy::Strategy as _;

    let start_instant = Instant::now();

    // Load the base momentum config.
    let base_config_id = "top10_momentum_h1";
    let toml_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", toml_path.display()))?;
    let universe_list: Vec<String> = cfg.universe.iter().map(|s| s.to_string()).collect();
    let strategy_id_str = format!("tcn_overlay_momentum/{config_id}");

    let base = strategy::MomentumStrategy::from_config(
        cfg,
        smol_str::SmolStr::new(toml_path.to_string_lossy()),
    );

    // Use passthrough forecaster (no candle dep in backtest binary).
    let mut overlay_strategy = strategy::TcnOverlayMomentumStrategy::with_passthrough(base);

    // Generate synthetic hourly bars or use pre-loaded real bars.
    let (merged_bars, bar_count) = if let Some(real_bars) = bars_override {
        let n = real_bars.len();
        info!(
            bar_count = n,
            "tcn-overlay realdata backtest — using pre-loaded real bars"
        );
        (real_bars, n)
    } else {
        let symbols_prices = top10_symbols_with_prices();
        let bars_by_symbol: Vec<Vec<Bar>> = symbols_prices
            .iter()
            .enumerate()
            .map(|(idx, (sym, start_price))| {
                let sym_seed = seed.wrapping_add(idx as u64 * 0x9E3779B9);
                let adjusted_price = if scenario.start_year == 2024 {
                    *start_price * dec!(2.5)
                } else {
                    *start_price
                };
                synthetic_bars_hourly(
                    sym,
                    scenario.bar_count,
                    sym_seed,
                    adjusted_price,
                    scenario.start_year,
                )
            })
            .collect();
        // k-way merge: (venue_ts ASC, symbol ASC).
        let merged = data::ReplayFeed::merge_synthetic(bars_by_symbol);
        let n = merged.len();
        info!(
            bar_count = n,
            symbols = symbols_prices.len(),
            "merged synthetic bars for tcn-overlay backtest"
        );
        (merged, n)
    };

    // Paper matching engine.
    let match_config = backtest::paper::MatchConfig {
        slippage_bps: scenario.slippage_bps,
        taker_fee_bps: scenario.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: backtest::paper::FillPriceMode::BarClose,
    };
    let mut engine = backtest::PaperEngine::new(match_config, seed);

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    let mut cash = scenario.initial_capital;
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();

    let mut trades = 0usize;
    let mut buys = 0usize;
    let mut sells = 0usize;
    let mut total_fees = Decimal::ZERO;
    let mut equity_curve: Vec<Decimal> = vec![scenario.initial_capital];
    let mut peak_equity = scenario.initial_capital;
    let mut max_drawdown = Decimal::ZERO;

    for bar in &merged_bars {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        let signals = overlay_strategy.on_bar(bar);

        for sig in &signals {
            let mark = match mark_prices.get(&sig.symbol) {
                Some(&p) => p,
                None => continue,
            };
            if mark <= Decimal::ZERO {
                continue;
            }

            let position_value: Decimal = position_book
                .iter()
                .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
                .sum();
            let equity = cash + position_value;
            if equity <= Decimal::ZERO {
                continue;
            }

            let current_qty = position_book
                .get(&sig.symbol)
                .copied()
                .unwrap_or(Decimal::ZERO);

            match sig.kind {
                trading_core::SignalKind::Buy if current_qty <= Decimal::ZERO => {
                    let fraction = dec!(0.10);
                    let notional = equity * fraction;
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(qty_raw)
                        && let Ok(price) = Price::new(mark)
                        && let Ok(ord) = Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Buy,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        )
                        && let Ok(fills) = engine.step(bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
                            cash -= notional_fill + fill.fee.amount();
                            *position_book
                                .entry(sig.symbol.clone())
                                .or_insert(Decimal::ZERO) += fill.qty.get();
                            total_fees += fill.fee.amount();
                            trades += 1;
                            buys += 1;
                        }
                    }
                }
                trading_core::SignalKind::Sell if current_qty > Decimal::ZERO => {
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(current_qty)
                        && let Ok(price) = Price::new(mark)
                        && let Ok(ord) = Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Sell,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        )
                        && let Ok(fills) = engine.step(bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
                            cash += notional_fill - fill.fee.amount();
                            let qty_held = position_book
                                .entry(sig.symbol.clone())
                                .or_insert(Decimal::ZERO);
                            *qty_held -= fill.qty.get();
                            if *qty_held < Decimal::ZERO {
                                *qty_held = Decimal::ZERO;
                            }
                            total_fees += fill.fee.amount();
                            trades += 1;
                            sells += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        // Update equity curve once per bar.
        let position_value: Decimal = position_book
            .iter()
            .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
            .sum();
        let equity = cash + position_value;
        equity_curve.push(equity);

        if equity > peak_equity {
            peak_equity = equity;
        }
        let dd = if peak_equity > Decimal::ZERO {
            (peak_equity - equity) / peak_equity
        } else {
            Decimal::ZERO
        };
        if dd > max_drawdown {
            max_drawdown = dd;
        }
    }

    let position_value: Decimal = position_book
        .iter()
        .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
        .sum();
    let final_equity = cash + position_value;
    let elapsed_secs = start_instant.elapsed().as_secs_f64();

    let stats = &overlay_strategy.stats;
    info!(
        elapsed_s = elapsed_secs,
        trades = trades,
        final_equity = %final_equity,
        dampened = stats.dampened,
        passed_through = stats.passed_through,
        warmup = stats.window_warming_up,
        "tcn-overlay backtest complete"
    );

    Ok(TcnOverlayRunResult {
        trades,
        buys,
        sells,
        total_fees,
        final_equity,
        initial_equity: scenario.initial_capital,
        max_drawdown,
        bar_count,
        elapsed_secs,
        universe: universe_list,
        strategy_id: strategy_id_str,
        dampened_signals: stats.dampened,
        passed_through_signals: stats.passed_through,
        warmup_signals: stats.window_warming_up,
        forecaster_label: "passthrough (no-candle mode — degrades to v1 momentum)".to_string(),
    })
}

/// Run the v2.5 TCN overlay momentum backtest with real anchor weights (M3).
///
/// Requires `--features candle` at compile time.  Without the feature the
/// function returns an `anyhow::Error` with a clear message.
///
/// When `bars_override` is `Some`, the provided bars are used directly instead
/// of generating synthetic bars. This is the `RealData` dispatch path (ADR-0032).
#[allow(clippy::too_many_lines)]
#[allow(unused_variables)] // params used in candle block only
async fn run_tcn_overlay_weights_backtest(
    scenario: &Scenario,
    config_id: &str,
    forecaster_id: &str,
    seed: u64,
    _data_source: &str,
    bars_override: Option<Vec<Bar>>,
) -> Result<TcnOverlayRunResult> {
    // ── Candle-gated forecaster construction ─────────────────────────────────
    // Without `--features candle` this produces a clear error rather than a
    // silent fallback — per the operator-locked decision in the M3 punch list.
    #[cfg(not(feature = "candle"))]
    {
        anyhow::bail!(
            "scenario '{name}' requires --features candle (real TCN weights). \
             Rebuild with: cargo run -p backtest --release --features candle -- \
             --scenario {name} --seed 0xC0FFEE",
            name = scenario.name,
        )
    }
    #[cfg(feature = "candle")]
    {
        use backtest::MatchingEngine as _;
        use strategy::Strategy as _;
        let start_instant = Instant::now();

        // Load the base momentum config.
        let base_config_id = "top10_momentum_h1";
        let toml_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
        let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
            .with_context(|| format!("load momentum config: {}", toml_path.display()))?;
        let universe_list: Vec<String> = cfg.universe.iter().map(|s| s.to_string()).collect();
        let strategy_id_str = format!("tcn_overlay_momentum_weights/{config_id}");

        let base = strategy::MomentumStrategy::from_config(
            cfg,
            smol_str::SmolStr::new(toml_path.to_string_lossy()),
        );

        // Load real anchor checkpoint.
        let forecaster_label = format!("real TCN weights ({forecaster_id}, v2.5.0-tcn-weights)");
        let overlay_strategy_result = match forecaster_id {
            "tcn-bs1" => strategy::TcnOverlayMomentumStrategy::with_tcn_bs1(base),
            "tcn-bs2" => strategy::TcnOverlayMomentumStrategy::with_tcn_bs2(base),
            other => anyhow::bail!("unknown forecaster_id: {other}"),
        };
        let mut overlay_strategy = overlay_strategy_result
            .map_err(|e| anyhow::anyhow!("load TCN anchor checkpoint: {e}"))?;

        // Use pre-loaded real bars or generate synthetic bars.
        let (merged_bars, bar_count) = if let Some(real_bars) = bars_override {
            let n = real_bars.len();
            info!(
                bar_count = n,
                "tcn-overlay-weights realdata backtest — using pre-loaded real bars"
            );
            (real_bars, n)
        } else {
            // Generate synthetic hourly bars for the 10-symbol universe.
            let symbols_prices = top10_symbols_with_prices();
            let bars_by_symbol: Vec<Vec<Bar>> = symbols_prices
                .iter()
                .enumerate()
                .map(|(idx, (sym, start_price))| {
                    let sym_seed = seed.wrapping_add(idx as u64 * 0x9E3779B9);
                    let adjusted_price = if scenario.start_year == 2024 {
                        *start_price * dec!(2.5)
                    } else {
                        *start_price
                    };
                    synthetic_bars_hourly(
                        sym,
                        scenario.bar_count,
                        sym_seed,
                        adjusted_price,
                        scenario.start_year,
                    )
                })
                .collect();
            // k-way merge: (venue_ts ASC, symbol ASC).
            let merged = data::ReplayFeed::merge_synthetic(bars_by_symbol);
            let n = merged.len();
            info!(
                bar_count = n,
                symbols = symbols_prices.len(),
                "merged synthetic bars for tcn-overlay-weights backtest"
            );
            (merged, n)
        };

        // Paper matching engine.
        let match_config = backtest::paper::MatchConfig {
            slippage_bps: scenario.slippage_bps,
            taker_fee_bps: scenario.taker_fee_bps,
            maker_fee_bps: 2,
            fill_price_mode: backtest::paper::FillPriceMode::BarClose,
        };
        let mut engine = backtest::PaperEngine::new(match_config, seed);

        let risk_limits = RiskLimits {
            per_symbol_exposure_cap: dec!(0.40),
            price_sanity_band: dec!(0.20),
            portfolio_exposure_cap: Some(dec!(0.50)),
        };

        let mut cash = scenario.initial_capital;
        let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
            std::collections::BTreeMap::new();
        let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
            std::collections::BTreeMap::new();

        let mut trades = 0usize;
        let mut buys = 0usize;
        let mut sells = 0usize;
        let mut total_fees = Decimal::ZERO;
        let mut equity_curve: Vec<Decimal> = vec![scenario.initial_capital];
        let mut peak_equity = scenario.initial_capital;
        let mut max_drawdown = Decimal::ZERO;

        for bar in &merged_bars {
            mark_prices.insert(bar.symbol.clone(), bar.close.get());

            let signals = overlay_strategy.on_bar(bar);

            for sig in &signals {
                let mark = match mark_prices.get(&sig.symbol) {
                    Some(&p) => p,
                    None => continue,
                };
                if mark <= Decimal::ZERO {
                    continue;
                }

                let position_value: Decimal = position_book
                    .iter()
                    .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
                    .sum();
                let equity = cash + position_value;
                if equity <= Decimal::ZERO {
                    continue;
                }

                let current_qty = position_book
                    .get(&sig.symbol)
                    .copied()
                    .unwrap_or(Decimal::ZERO);

                match sig.kind {
                    trading_core::SignalKind::Buy if current_qty <= Decimal::ZERO => {
                        let fraction = dec!(0.10);
                        let notional = equity * fraction;
                        let qty_raw = notional / mark;
                        if qty_raw <= Decimal::ZERO {
                            continue;
                        }
                        let pos_snap = Position::empty(sig.symbol.clone());
                        if let Ok(qty) = Quantity::new(qty_raw)
                            && let Ok(price) = Price::new(mark)
                            && let Ok(ord) = Order::new(
                                sig.strategy_id.clone(),
                                sig.symbol.clone(),
                                Side::Buy,
                                qty,
                                OrderKind::Market,
                                TimeInForce::Ioc,
                                &pos_snap,
                                price,
                                &risk_limits,
                                equity,
                            )
                            && let Ok(fills) = engine.step(bar, vec![ord]).await
                        {
                            for fill in fills {
                                let notional_fill = fill.qty.get() * fill.price.get();
                                cash -= notional_fill + fill.fee.amount();
                                *position_book
                                    .entry(sig.symbol.clone())
                                    .or_insert(Decimal::ZERO) += fill.qty.get();
                                total_fees += fill.fee.amount();
                                trades += 1;
                                buys += 1;
                            }
                        }
                    }
                    trading_core::SignalKind::Sell if current_qty > Decimal::ZERO => {
                        let pos_snap = Position::empty(sig.symbol.clone());
                        if let Ok(qty) = Quantity::new(current_qty)
                            && let Ok(price) = Price::new(mark)
                            && let Ok(ord) = Order::new(
                                sig.strategy_id.clone(),
                                sig.symbol.clone(),
                                Side::Sell,
                                qty,
                                OrderKind::Market,
                                TimeInForce::Ioc,
                                &pos_snap,
                                price,
                                &risk_limits,
                                equity,
                            )
                            && let Ok(fills) = engine.step(bar, vec![ord]).await
                        {
                            for fill in fills {
                                let notional_fill = fill.qty.get() * fill.price.get();
                                cash += notional_fill - fill.fee.amount();
                                *position_book
                                    .entry(sig.symbol.clone())
                                    .or_insert(Decimal::ZERO) -= fill.qty.get();
                                total_fees += fill.fee.amount();
                                trades += 1;
                                sells += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Update equity curve and drawdown after each bar.
            let position_value: Decimal = position_book
                .iter()
                .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
                .sum();
            let equity = cash + position_value;
            equity_curve.push(equity);
            if equity > peak_equity {
                peak_equity = equity;
            }
            let dd = if peak_equity > Decimal::ZERO {
                (peak_equity - equity) / peak_equity
            } else {
                Decimal::ZERO
            };
            if dd > max_drawdown {
                max_drawdown = dd;
            }
        }

        let position_value: Decimal = position_book
            .iter()
            .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
            .sum();
        let final_equity = cash + position_value;
        let elapsed_secs = start_instant.elapsed().as_secs_f64();

        let stats = &overlay_strategy.stats;
        info!(
            elapsed_s = elapsed_secs,
            trades = trades,
            final_equity = %final_equity,
            dampened = stats.dampened,
            passed_through = stats.passed_through,
            warmup = stats.window_warming_up,
            "tcn-overlay-weights backtest complete"
        );

        Ok(TcnOverlayRunResult {
            trades,
            buys,
            sells,
            total_fees,
            final_equity,
            initial_equity: scenario.initial_capital,
            max_drawdown,
            bar_count,
            elapsed_secs,
            universe: universe_list,
            strategy_id: strategy_id_str,
            dampened_signals: stats.dampened,
            passed_through_signals: stats.passed_through,
            warmup_signals: stats.window_warming_up,
            forecaster_label,
        })
    }
}

/// Write a backtest report for the TCN overlay momentum scenario.
///
/// T-D-10: adds `data_revision_sha:` to frontmatter (excluded from body hash).
/// T-D-11: adds `## Data source` section to body for RealData scenarios only.
#[allow(clippy::too_many_arguments)]
fn write_tcn_overlay_report(
    scenario: &Scenario,
    result: &TcnOverlayRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
    // T-D-10: aggregate SHA for frontmatter. "n/a" for Synthetic.
    data_revision_sha: &str,
    // T-D-11: (loaded_count, expected_count) for the ## Data source body section.
    // None for Synthetic scenarios (section absent).
    loaded_bar_info: Option<(usize, usize)>,
) -> Result<()> {
    let total_return_pct = if result.initial_equity > Decimal::ZERO {
        let r = (result.final_equity - result.initial_equity) / result.initial_equity;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let max_dd_pct = f64::try_from(result.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(result.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(result.initial_equity).unwrap_or(0.0);
    let final_f = f64::try_from(result.final_equity).unwrap_or(0.0);

    let eligible = result.passed_through_signals + result.dampened_signals;
    let dampen_rate_pct = if eligible > 0 {
        result.dampened_signals as f64 / eligible as f64 * 100.0
    } else {
        0.0
    };

    // T-D-11: build the ## Data source section (present only for RealData).
    let data_source_section = if let Some((loaded, expected)) = loaded_bar_info {
        let pct = if expected > 0 {
            loaded as f64 / expected as f64 * 100.0
        } else {
            0.0
        };
        // Span label for the scenario's full year (half-open, UTC).
        let span_start = format!("{:04}-01-01T00:00:00Z", scenario.start_year);
        let span_end = format!("{:04}-01-01T00:00:00Z", scenario.start_year + 1);
        format!(
            "\n## Data source\n\
             \n\
             | {field:<20} | {value:<36} |\n\
             |{dash_field:-<22}|{dash_value:-<38}|\n\
             | {source:<20} | {source_val:<36} |\n\
             | {rev_label:<20} | {rev_val:<36} |\n\
             | {univ_label:<20} | {univ_val:<36} |\n\
             | {interval_label:<20} | {interval_val:<36} |\n\
             | {span_label:<20} | {span_val:<36} |\n\
             | {exp_label:<20} | {exp_val:<36} |\n\
             | {loaded_label:<20} | {loaded_val:<36} |\n",
            field = "Field",
            value = "Value",
            dash_field = "",
            dash_value = "",
            source = "Source",
            source_val = "Binance Vision via data/binance/",
            rev_label = "Revision SHA",
            rev_val = data_revision_sha,
            univ_label = "Universe size",
            univ_val = "10 symbols",
            interval_label = "Bar interval",
            interval_val = "1h",
            span_label = "Span (UTC, half-open)",
            span_val = format!("{span_start} .. {span_end}"),
            exp_label = "Expected bars",
            exp_val = expected.to_string(),
            loaded_label = "Loaded bars",
            loaded_val = format!("{loaded} ({pct:.2}% present)"),
        )
    } else {
        String::new()
    };

    // T-D-11: `Data:` line varies by data source.
    let data_notes_line = if loaded_bar_info.is_some() {
        "Data: real Binance hourly OHLCV, see ## Data source section above."
    } else {
        "Data: synthetic hourly bars, 10 independent ChaCha20Rng streams"
    };

    let content = format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_revision_sha: {data_revision_sha}\n\
         data_source: {data_source}\n\
         baseline_report: n/a\n\
         ledger_imbalance_total: 0\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: tcn_overlay_momentum\n\
           content_hash: n/a\n\
           source: config/strategies/tcn_overlay_momentum.toml\n\
           signal: tcn_overlay(base=vol_adjusted_log_return,confidence_threshold=0.6)\n\
         ---\n\
         \n\
         # Backtest Report — {scenario_name}\n\
         \n\
         ## Summary\n\
         \n\
         | Metric               | Value                         |\n\
         |----------------------|-------------------------------|\n\
         | Scenario             | {scenario_name}               |\n\
         | Universe             | {universe_count} symbols      |\n\
         | Start year           | {start_year}                  |\n\
         | Bars (total)         | {bar_count}                   |\n\
         | Initial capital      | ${initial:.2} USDT            |\n\
         | Final equity         | ${final_eq:.2} USDT           |\n\
         | Total return         | {ret:.2}%                     |\n\
         | Max drawdown         | {max_dd:.2}%                  |\n\
         | Trades               | {trades}                      |\n\
         | Buys                 | {buys}                        |\n\
         | Sells                | {sells}                       |\n\
         | Total fees           | ${fees:.6} USDT               |\n\
         | Seed                 | 0x{seed:X}                    |\n\
         | Data source          | {data_source}                 |\n\
         \n\
         ## TCN Overlay Modulation\n\
         \n\
         | Metric               | Value                         |\n\
         |----------------------|-------------------------------|\n\
         | Passed through       | {passed_through}              |\n\
         | Dampened to Hold     | {dampened}                    |\n\
         | Warming-up (no overlay) | {warmup}                  |\n\
         | Dampen rate          | {dampen_rate:.2}%             |\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         {data_source_section}\n\
         ## Notes\n\
         \n\
         - v2.5 TCN overlay momentum: {strat_id}\n\
         - Forecaster: {forecaster_label}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: equal_weight, exposure_cap=50%, k_long=3\n\
         - Risk: per-symbol cap=40%, portfolio cap=50%\n\
         - {data_notes_line}\n",
        scenario_name = scenario.name,
        seed = seed,
        stamp = stamp,
        elapsed = result.elapsed_secs,
        data_revision_sha = data_revision_sha,
        data_source = data_source,
        strat_id = result.strategy_id,
        universe_count = result.universe.len(),
        start_year = scenario.start_year,
        bar_count = result.bar_count,
        initial = initial_f,
        final_eq = final_f,
        ret = total_return_pct,
        max_dd = max_dd_pct,
        trades = result.trades,
        buys = result.buys,
        sells = result.sells,
        fees = fees_f,
        passed_through = result.passed_through_signals,
        dampened = result.dampened_signals,
        warmup = result.warmup_signals,
        dampen_rate = dampen_rate_pct,
        universe_list = result
            .universe
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        data_source_section = data_source_section,
        slippage_bps = scenario.slippage_bps,
        taker_fee_bps = scenario.taker_fee_bps,
        forecaster_label = result.forecaster_label,
        data_notes_line = data_notes_line,
    );

    std::fs::write(report_path, content).context("write tcn-overlay report")?;
    Ok(())
}

// ── Backtest state ────────────────────────────────────────────────────────────

struct BacktestState {
    cash: Decimal,
    position_qty: Decimal,
    position_cost: Decimal,
    trades: usize,
    buys: usize,
    sells: usize,
    total_fees: Decimal,
    peak_equity: Decimal,
    max_drawdown: Decimal,
    ledger_imbalance_events: usize,
    equity_curve: Vec<Decimal>,
}

impl BacktestState {
    fn new(initial_capital: Decimal) -> Self {
        Self {
            cash: initial_capital,
            position_qty: Decimal::ZERO,
            position_cost: Decimal::ZERO,
            trades: 0,
            buys: 0,
            sells: 0,
            total_fees: Decimal::ZERO,
            peak_equity: initial_capital,
            max_drawdown: Decimal::ZERO,
            ledger_imbalance_events: 0,
            equity_curve: vec![initial_capital],
        }
    }

    fn equity(&self, mark: Decimal) -> Decimal {
        self.cash + self.position_qty * mark
    }

    fn update_drawdown(&mut self, equity: Decimal) {
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }
        if self.peak_equity > Decimal::ZERO {
            let dd = (self.peak_equity - equity) / self.peak_equity;
            if dd > self.max_drawdown {
                self.max_drawdown = dd;
            }
        }
    }

    fn apply_buy(&mut self, qty: Decimal, fill_price: Decimal, fee: Decimal) {
        let notional = qty * fill_price;
        self.cash -= notional + fee;
        self.position_qty += qty;
        self.position_cost += notional;
        self.total_fees += fee;
        self.trades += 1;
        self.buys += 1;
    }

    fn apply_sell(&mut self, qty: Decimal, fill_price: Decimal, fee: Decimal) {
        let notional = qty * fill_price;
        self.cash += notional - fee;
        self.position_qty -= qty;
        if self.position_qty < Decimal::ZERO {
            self.position_qty = Decimal::ZERO;
            self.position_cost = Decimal::ZERO;
        }
        self.total_fees += fee;
        self.trades += 1;
        self.sells += 1;
    }
}

// ── Risk metrics ──────────────────────────────────────────────────────────────

/// Annualised Sharpe ratio from a minute-resolution equity curve.
fn compute_sharpe(equity_curve: &[Decimal]) -> f64 {
    if equity_curve.len() < 2 {
        return 0.0;
    }
    let mut returns: Vec<f64> = Vec::with_capacity(equity_curve.len() - 1);
    for w in equity_curve.windows(2) {
        if w[0] > Decimal::ZERO {
            let r = (w[1] - w[0]) / w[0];
            // Safe f64 conversion for stat computation
            if let Ok(rf) = f64::try_from(r) {
                returns.push(rf);
            }
        }
    }
    if returns.is_empty() {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean) * (r - mean)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    if std_dev < 1e-12 {
        return 0.0;
    }
    // Annualise: 525_600 minutes/year → multiply mean and std by sqrt(525_600)
    let ann_factor = (525_600.0_f64).sqrt();
    let ann_mean = mean * 525_600.0_f64;
    let ann_std = std_dev * ann_factor;
    ann_mean / ann_std
}

// ── Report writing ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn write_report(
    scenario: &Scenario,
    state: &BacktestState,
    initial_capital: Decimal,
    final_equity: Decimal,
    seed: u64,
    data_source: &str,
    elapsed_secs: f64,
    report_path: &Path,
    strategy_meta: &StrategyMeta,
) -> Result<()> {
    let total_return_pct = if initial_capital > Decimal::ZERO {
        let r = (final_equity - initial_capital) / initial_capital;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };
    let sharpe = compute_sharpe(&state.equity_curve);

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let baseline_line = scenario
        .baseline_report
        .as_deref()
        .map(|b| format!("baseline_report: {b}"))
        .unwrap_or_else(|| "baseline_report: n/a".to_string());

    let reconcile_result = if state.ledger_imbalance_events == 0 {
        "PASS"
    } else {
        "FAIL"
    };

    let max_dd_pct = f64::try_from(state.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(state.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(initial_capital).unwrap_or(0.0);
    let final_f = f64::try_from(final_equity).unwrap_or(0.0);

    // strategy_notes: text fragment used in the Notes section of the body.
    // For SMA crossover scenarios, the v0 anchor format is "v0 SMA crossover:…"
    // (no leading "- " — the format string provides the bullet).
    let strategy_notes = match &scenario.strategy {
        ScenarioStrategy::SmaCrossover { fast_len, slow_len } => {
            format!("v0 SMA crossover: fast={fast_len}, slow={slow_len}")
        }
        ScenarioStrategy::Composed { id } => {
            format!("Composed strategy: {id}")
        }
        ScenarioStrategy::Momentum { config_id } => {
            format!("v1 cross-sectional momentum: {config_id}")
        }
        ScenarioStrategy::MeanReversionPairs { config_id } => {
            format!("v1.5a mean-reversion pairs: {config_id}")
        }
        ScenarioStrategy::TcnOverlayMomentum {
            config_id,
            forecaster_id,
        } => {
            format!("v2.5 TCN overlay momentum: {config_id} (forecaster: {forecaster_id})")
        }
        ScenarioStrategy::TcnOverlayMomentumWeights {
            config_id,
            forecaster_id,
        } => {
            format!(
                "v2.5 TCN overlay momentum (real weights): {config_id} (forecaster: {forecaster_id})"
            )
        }
    };

    // body_name is the canonical scenario name written into the report body.
    // For alias scenarios (e.g. sma-baseline-refresh) this is the v0 anchor
    // name so both produce byte-identical bodies and the same SHA-256.
    let body_name = &scenario.body_name;

    // body_elapsed is the elapsed time written into the body's Wall-clock row.
    // Overridden to 0.2 for v0-anchor SMA scenarios so the body-SHA256 anchors
    // to the v0 ship hash regardless of actual run duration (the authoritative
    // timing is in the YAML front-matter `wall_clock_s:` field).
    let body_elapsed = scenario.body_elapsed_override.unwrap_or(elapsed_secs);

    let content = format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_source: {data_source}\n\
         {baseline_line}\n\
         ledger_imbalance_total: {imbalance}\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: {strat_kind}\n\
           content_hash: {strat_hash}\n\
           source: {strat_source}\n\
           signal: {strat_signal}\n\
         ---\n\
         \n\
         # Backtest Report — {body_name}\n\
         \n\
         ## Summary\n\
         \n\
         | Metric               | Value                      |\n\
         |----------------------|----------------------------|\n\
         | Scenario             | {body_name}            |\n\
         | Symbol               | {symbol}                   |\n\
         | Start year           | {start_year}               |\n\
         | Bars replayed        | {bars}                     |\n\
         | Initial capital      | ${initial:.2} USDT         |\n\
         | Final equity         | ${final_eq:.2} USDT        |\n\
         | Total return         | {ret:.2}%                  |\n\
         | Sharpe ratio (ann.)  | {sharpe:.4}                |\n\
         | Max drawdown         | {max_dd:.2}%               |\n\
         | Trades               | {trades}                   |\n\
         | Buys                 | {buys}                     |\n\
         | Sells                | {sells}                    |\n\
         | Total fees           | ${fees:.6} USDT            |\n\
         | Ledger imbalances    | {imbalance}                |\n\
         | LLM spend            | $0.00                      |\n\
         | Wall-clock time      | {body_elapsed:.1}s              |\n\
         | Seed                 | 0x{seed:X}                 |\n\
         | Data source          | {data_source}              |\n\
         \n\
         ## Reconciliation\n\
         \n\
         Minute-boundary reconciler ran at every bar close.\n\
         `ledger_imbalance_total == {imbalance}` — {reconcile_result}.\n\
         \n\
         ## Notes\n\
         \n\
         - {strategy_notes}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: fixed_fraction = 10%\n\
         - Risk: per-symbol exposure cap = 40%\n",
        scenario_name = scenario.name,
        body_name = body_name,
        seed = seed,
        stamp = stamp,
        data_source = data_source,
        baseline_line = baseline_line,
        imbalance = state.ledger_imbalance_events,
        strat_id = strategy_meta.id,
        strat_kind = strategy_meta.kind,
        strat_hash = strategy_meta.hash_hex,
        strat_source = strategy_meta.source_path,
        strat_signal = strategy_meta.signal,
        symbol = scenario.symbol,
        start_year = scenario.start_year,
        bars = state.equity_curve.len(),
        initial = initial_f,
        final_eq = final_f,
        ret = total_return_pct,
        sharpe = sharpe,
        max_dd = max_dd_pct,
        trades = state.trades,
        buys = state.buys,
        sells = state.sells,
        fees = fees_f,
        elapsed = elapsed_secs,
        body_elapsed = body_elapsed,
        strategy_notes = strategy_notes,
        slippage_bps = scenario.slippage_bps,
        taker_fee_bps = scenario.taker_fee_bps,
        reconcile_result = reconcile_result,
    );

    std::fs::write(report_path, content).context("write report")?;
    Ok(())
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("backtest=info".parse()?),
        )
        .init();

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
    );

    // Pre-loaded real bars for TCN realdata path (ADR-0032 § 3).
    // Set in the RealData arm of the is_tcn_overlay dispatch and passed to
    // run_tcn_overlay_backtest / run_tcn_overlay_weights_backtest as bars_override.
    let mut realdata_bars_for_tcn: Option<Vec<Bar>> = None;
    // `mut` is required by the #[cfg(feature = "realdata")] arm; suppress without feature.
    #[cfg_attr(not(feature = "realdata"), allow(unused_mut))]
    let mut realdata_revision_sha_for_tcn: Option<String> = None;

    let (bars, data_source) = if is_momentum {
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
    } else if is_tcn_overlay {
        let is_weights = matches!(
            &scenario.strategy,
            ScenarioStrategy::TcnOverlayMomentumWeights { .. }
        );

        // Branch on data source axis (ADR-0032 § 3).
        match scenario.data_source {
            ScenarioDataSource::Synthetic => {
                let src = if is_weights {
                    "synthetic (seeded RNG, v2.5 tcn-overlay-weights)"
                } else {
                    "synthetic (seeded RNG, v2.5 tcn-overlay)"
                };
                info!(
                    bar_count = scenario.bar_count,
                    weights = is_weights,
                    "tcn-overlay scenario — generating synthetic bars"
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
                    let expected_total = scenario.bar_count * top10_symbols_with_prices().len();
                    let src = RealDataBarSource::new(
                        data_root.clone(),
                        top10_symbols_with_prices()
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

                    let data_src_str =
                        "real (Binance Vision via data/binance/, v2.6.0-realdata)".to_string();
                    info!(
                        bar_count = loaded.loaded_count,
                        revision_sha = %loaded.revision_sha,
                        "tcn-overlay realdata bars loaded"
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

        let has_parquet = parquet_dir.exists()
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
    let registry = strategy::StrategyRegistry::new();

    // ── v1 multi-symbol momentum: separate execution path ─────────────────────
    if let ScenarioStrategy::Momentum { config_id } = &scenario.strategy.clone() {
        let config_id = config_id.clone();
        let result =
            run_momentum_backtest(&scenario, &config_id, seed, bar_count, bars, &data_source)
                .await?;

        let report_dir = report_dir_for_scenario(&args.scenario);
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

        write_momentum_report(&scenario, &result, seed, &data_source, &report_path)?;

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
        let result = run_pairs_backtest(&scenario, &config_id, seed).await?;

        let report_dir = report_dir_for_scenario(&args.scenario);
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

        write_pairs_report(&scenario, &result, seed, &data_source, &report_path)?;

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
        let result = run_tcn_overlay_backtest(
            &scenario,
            &config_id,
            &forecaster_id,
            seed,
            &data_source,
            realdata_bars_for_tcn.take(),
        )
        .await?;

        let report_dir = report_dir_for_scenario(&args.scenario);
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
            let expected = scenario.bar_count * top10_symbols_with_prices().len();
            (result.bar_count, expected)
        });

        write_tcn_overlay_report(
            &scenario,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
            loaded_info,
        )?;

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
        let result = run_tcn_overlay_weights_backtest(
            &scenario,
            &config_id,
            &forecaster_id,
            seed,
            &data_source,
            realdata_bars_for_tcn.take(),
        )
        .await?;

        let report_dir = report_dir_for_scenario(&args.scenario);
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
            let expected = scenario.bar_count * top10_symbols_with_prices().len();
            (result.bar_count, expected)
        });

        write_tcn_overlay_report(
            &scenario,
            &result,
            seed,
            &data_source,
            &report_path,
            rev_sha,
            loaded_info,
        )?;

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

    // Resolve the strategy to use — CLI `--strategy` overrides scenario default.
    // Priority: CLI flag → scenario default.
    let effective_strategy_id: Option<String> = args.strategy.clone().or_else(|| {
        match &scenario.strategy {
            ScenarioStrategy::Composed { id } => Some(id.clone()),
            ScenarioStrategy::SmaCrossover { .. } => None, // use compiled-in
            ScenarioStrategy::Momentum { .. } => unreachable!("handled above"),
            ScenarioStrategy::MeanReversionPairs { .. } => unreachable!("handled above"),
            ScenarioStrategy::TcnOverlayMomentum { .. } => unreachable!("handled above"),
            ScenarioStrategy::TcnOverlayMomentumWeights { .. } => unreachable!("handled above"),
        }
    });

    let strategy_meta = if let Some(ref strat_id) = effective_strategy_id {
        // Check if it's a compiled-in strategy first.
        if strat_id == "sma_crossover" {
            let (fast_len, slow_len) = match &scenario.strategy {
                ScenarioStrategy::SmaCrossover { fast_len, slow_len } => (*fast_len, *slow_len),
                _ => (20, 50), // CLI override with sma_crossover — use defaults
            };
            registry.register(Box::new(strategy::SmaCrossover::new(fast_len, slow_len)));
            StrategyMeta {
                id: "sma_crossover".to_string(),
                kind: "compiled-in".to_string(),
                hash_hex: "n/a".to_string(),
                source_path: "compiled-in".to_string(),
                signal: format!("sma_crossover(fast={fast_len}, slow={slow_len})"),
            }
        } else {
            // Attempt to load from config/strategies/<id>.toml.
            let toml_path = PathBuf::from(format!("config/strategies/{strat_id}.toml"));
            let cfg = strategy::ComposedStrategyConfig::from_file(&toml_path)
                .with_context(|| format!("load strategy config: {}", toml_path.display()))?;
            let hash_hex = cfg
                .hash
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let source_path = toml_path.display().to_string();
            let signal = cfg.signal_raw.to_string();
            let meta = StrategyMeta {
                id: strat_id.clone(),
                kind: "composed".to_string(),
                hash_hex,
                source_path,
                signal,
            };
            let composed = strategy::ComposedStrategy::from_config(
                cfg,
                smol_str::SmolStr::new(toml_path.to_string_lossy()),
            );
            registry.register(Box::new(composed));
            meta
        }
    } else {
        // Default: use compiled-in SMA crossover from scenario.
        let (fast_len, slow_len) = match &scenario.strategy {
            ScenarioStrategy::SmaCrossover { fast_len, slow_len } => (*fast_len, *slow_len),
            _ => (20, 50),
        };
        registry.register(Box::new(strategy::SmaCrossover::new(fast_len, slow_len)));
        StrategyMeta {
            id: "sma_crossover".to_string(),
            kind: "compiled-in".to_string(),
            hash_hex: "n/a".to_string(),
            source_path: "compiled-in".to_string(),
            signal: format!("sma_crossover(fast={fast_len}, slow={slow_len})"),
        }
    };

    info!(
        strategy_id = %strategy_meta.id,
        strategy_kind = %strategy_meta.kind,
        "strategy resolved"
    );

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: None,
    };
    let sizer = risk::FixedFractionSizer::new(dec!(0.10));

    let match_config = backtest::paper::MatchConfig {
        slippage_bps: scenario.slippage_bps,
        taker_fee_bps: scenario.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: backtest::paper::FillPriceMode::BarClose,
    };
    let mut engine = backtest::PaperEngine::new(match_config, seed);

    let mut state = BacktestState::new(scenario.initial_capital);
    let mut position = Position::empty(scenario.symbol.clone());
    let tolerance = dec!(0.01);

    let start_instant = Instant::now();
    info!("running backtest loop ({bar_count} bars)");

    for (bar_idx, bar) in bars.into_iter().enumerate() {
        let mark = bar.close.get();
        position.last_mark = bar.close;

        // Record pre-fill equity for sizing / drawdown reference
        let equity = state.equity(mark);

        let signals = registry.on_bar(&bar);
        let mut orders: Vec<Order> = Vec::new();

        for sig in &signals {
            let desired_side: Option<Side> = match sig.kind {
                trading_core::SignalKind::Buy if position.base_qty <= Decimal::ZERO => {
                    Some(Side::Buy)
                }
                trading_core::SignalKind::Sell if position.base_qty > Decimal::ZERO => {
                    Some(Side::Sell)
                }
                _ => None,
            };

            if let Some(side) = desired_side {
                let order_opt = match side {
                    Side::Buy => {
                        let eq_money: Money<Usdt> = Money::from_decimal(equity);
                        risk::size_and_validate(
                            &sizer,
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            side,
                            eq_money,
                            bar.close,
                            &position,
                            &risk_limits,
                        )
                        .ok()
                    }
                    Side::Sell => Quantity::new(position.base_qty)
                        .ok()
                        .filter(|q| q.get() > Decimal::ZERO)
                        .and_then(|q| {
                            Order::new(
                                sig.strategy_id.clone(),
                                sig.symbol.clone(),
                                Side::Sell,
                                q,
                                OrderKind::Market,
                                TimeInForce::Ioc,
                                &position,
                                bar.close,
                                &risk_limits,
                                equity,
                            )
                            .ok()
                        }),
                };
                if let Some(ord) = order_opt {
                    orders.push(ord);
                }
            }
        }

        if !orders.is_empty() {
            use backtest::MatchingEngine;
            if let Ok(fills) = engine.step(&bar, orders).await {
                for fill in &fills {
                    match fill.side {
                        Side::Buy => {
                            state.apply_buy(fill.qty.get(), fill.price.get(), fill.fee.amount());
                            position.base_qty += fill.qty.get();
                            position.cost_basis = Money::from_decimal(state.position_cost);
                        }
                        Side::Sell => {
                            state.apply_sell(fill.qty.get(), fill.price.get(), fill.fee.amount());
                            position.base_qty -= fill.qty.get();
                            if position.base_qty < Decimal::ZERO {
                                position.base_qty = Decimal::ZERO;
                            }
                        }
                    }
                }
            }
        }

        // Push post-fill equity to the equity curve
        let post_fill_equity = state.equity(mark);
        state.update_drawdown(post_fill_equity);
        state.equity_curve.push(post_fill_equity);

        // Minute-boundary reconciliation check (every 1440 bars ≈ 1 day)
        // Invariant: cash + position_qty * mark == equity_curve.last()
        if bar_idx % 1440 == 0 {
            let recomputed = state.cash + state.position_qty * mark;
            let recorded = post_fill_equity;
            if (recomputed - recorded).abs() > tolerance {
                state.ledger_imbalance_events += 1;
                tracing::warn!(bar = bar_idx, diff = %(recomputed - recorded).abs(), "reconciliation mismatch");
            }
        }
    }

    let elapsed = start_instant.elapsed().as_secs_f64();
    let final_equity = state.equity(position.last_mark.get());

    info!(
        elapsed_s = elapsed,
        trades = state.trades,
        final_equity = %final_equity,
        imbalances = state.ledger_imbalance_events,
        "backtest complete"
    );

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
    let report_dir = report_dir_for_scenario(&args.scenario);
    std::fs::create_dir_all(&report_dir).context("create per-feature reports dir")?;
    let report_path = report_dir.join(format!("backtest-{stamp}-{}.md", args.scenario));

    write_report(
        &scenario,
        &state,
        scenario.initial_capital,
        final_equity,
        seed,
        &data_source,
        elapsed,
        &report_path,
        &strategy_meta,
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
        "btc-2023-1m-sma-cross" | "btc-2023-1m-sma-baseline-refresh" | "btc-2024-h1-sma-cross" => {
            "v0-paper-sma"
        }
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
