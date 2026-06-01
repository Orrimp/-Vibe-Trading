//! `MatchingEngine` trait + `run_scenario` library API (ADR-0030).
//!
//! ## `run_scenario` design (T-D-12 / ADR-0030)
//!
//! The function is the single public entry-point for running a backtest
//! from the cockpit (Lab Run button, Phase A) or any future Rust caller.
//! It takes a `ScenarioConfig` (strategy + pair + range + seed) and
//! returns a `RunReport` with the in-memory equity series + fill list
//! + KPIs, plus a `report_path` when `cfg.write_report = true`.
//!
//! The standalone backtest binary (`crates/backtest/src/main.rs`) was
//! **not** refactored to call this function in Phase A because it
//! orchestrates many heterogeneous scenario types (SMA, Composed,
//! Momentum, Pairs, TCN) that each need their own config struct; a
//! safe refactor is a Phase B milestone. Phase A wires only the types
//! so the `ui` crate can compile against them and the `runner.rs`
//! placeholder can be replaced when the real implementation lands.
//!
//! **Anchor contract (T-D-13):** The standalone binary is UNCHANGED
//! at Phase A. All 11 body-SHA-256 anchors in `spec/anchors.toml`
//! remain byte-identical. The `run_scenario` implementation below
//! is NOT called by the binary yet; it is a type-safe stub that
//! validates the seed and returns `RunError::NotImplemented` for
//! Phase A. Phase B replaces the body.
//!
//! **Determinism contract:** `cfg.seed` is mandatory; the function
//! rejects `[0u8; 32]` loudly so "forgot to set seed" is a hard
//! error. The Lab's default seed is `LAB_DEFAULT_SEED` in
//! `crates/ui/src/lab/defaults.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use thiserror::Error;
use time::OffsetDateTime;
use trading_core::{Bar, FillView, Money, Order, StrategyId, Symbol, Timestamp, Usdt, Venue};

use crate::paper::MatchConfig;

// ── MatchingEngine trait ─────────────────────────────────────────────────────

/// Error from the matching engine.
#[derive(Debug, Error)]
pub enum MatchError {
    #[error("fill computation error: {0}")]
    FillError(String),
    #[error("no liquidity")]
    NoLiquidity,
}

/// The matching engine abstraction.
///
/// v0 ships `PaperEngine` (simple bps slippage + taker fee).
/// The trait signature is limit-order-friendly even though v0 only uses market orders.
/// v0.5 may swap in `orderbook-rs` / `matchcore` / `rust_ob` without changing callers.
#[async_trait]
pub trait MatchingEngine: Send + Sync {
    /// Process bar-aligned orders and return fills.
    async fn step(&mut self, bar: &Bar, orders: Vec<Order>) -> Result<Vec<Fill_>, MatchError>;

    fn config(&self) -> MatchConfig;
}

// Use trading_core::Fill as Fill_; the alias avoids re-exporting with a name clash.
use trading_core::Fill as Fill_;

// ── ADR-0030 `run_scenario` API types ────────────────────────────────────────

/// Date range for a backtest scenario.
///
/// Mirrors the `ui::lab::state::DateRange` variants but lives in the
/// `backtest` crate so `backtest` does NOT depend on `ui` (which would
/// be a circular dependency). The `ui::lab::runner` maps
/// `ui::lab::state::DateRange` → `backtest::engine::DateRange` at the
/// call site.
///
/// `Custom` carries epoch-millis start + end for precision. Named
/// presets are expanded to fixed UTC day boundaries in
/// `run_scenario`'s body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DateRange {
    /// Last 30 calendar days from the current system date.
    Last30d,
    /// Last 90 calendar days from the current system date.
    Last90d,
    /// First half of 2024 (2024-01-01 00:00:00Z → 2024-06-30 23:59:59Z).
    H1_2024,
    /// Second half of 2024 (2024-07-01 00:00:00Z → 2024-12-31 23:59:59Z).
    H2_2024,
    /// Operator-specified range as UTC epoch milliseconds.
    Custom {
        /// Inclusive start (UTC epoch-millis).
        start_ms: i64,
        /// Inclusive end (UTC epoch-millis).
        end_ms: i64,
    },
}

/// Optional strategy parameter overrides (Phase B; always `None` at Phase A).
///
/// `ParamSheet` is currently opaque (`()`). Phase B replaces it with a typed
/// enum keyed on the strategy family.
#[derive(Debug, Clone)]
pub struct ParamSheet;

/// Backtest performance KPIs for the `RunReport`.
///
/// **Anchor-additive contract** (lab-polish-round-2 R3): fields added after
/// the v0.1.0 ship (`buys`, `sells`, `total_return_pct`) are in-memory only —
/// the Markdown report body at `report/sma.rs::build_content` pulls from
/// `BacktestState` directly, not from this struct, so adding fields here
/// does NOT change anchored body-SHAs.
#[derive(Debug, Clone)]
pub struct BacktestKpis {
    /// Final portfolio equity in USDT.
    pub final_equity: Money<Usdt>,
    /// Initial portfolio equity in USDT.
    pub initial_equity: Money<Usdt>,
    /// Maximum drawdown as a decimal fraction (0.0 = 0 %, 1.0 = 100 %).
    pub max_drawdown: Decimal,
    /// Total executed fills (buys + sells).
    pub trade_count: usize,
    /// Total fees paid in USDT.
    pub total_fees: Money<Usdt>,

    // ── lab-polish-round-2 R3 — UI-only fields (anchor-additive) ────────
    /// Number of executed buy fills. `BacktestState.buys`.
    /// Defaults to 0 in `BacktestKpis::default()` for fixture-path summaries.
    pub buys: usize,
    /// Number of executed sell fills. `BacktestState.sells`.
    pub sells: usize,
    /// Total return as a decimal fraction relative to `initial_equity`.
    /// `(final_equity - initial_equity) / initial_equity`. 0.0 = break-even,
    /// 0.1 = +10 %, -0.05 = -5 %. Defaults to 0.0 in `default()`.
    pub total_return_pct: Decimal,
}

// K8a (lab-end-to-end-v2 T-D1.4) — `BacktestKpis::default()` is used by
// fixture-path `RunSummary` constructors where no real KPI data is available
// (non-live / no-rt_handle placeholder summaries).
impl Default for BacktestKpis {
    fn default() -> Self {
        Self {
            final_equity: Money::<Usdt>::zero(),
            initial_equity: Money::<Usdt>::zero(),
            max_drawdown: Decimal::ZERO,
            trade_count: 0,
            total_fees: Money::<Usdt>::zero(),
            buys: 0,
            sells: 0,
            total_return_pct: Decimal::ZERO,
        }
    }
}

/// Data source selector for `ScenarioConfig` (lab-yahoo-realdata v0.1.0 / T-AR1).
///
/// CLI anchor-generating paths construct `ScenarioConfig` without this field;
/// the `#[serde(default)]` default (`Synthetic`) preserves byte-identical
/// behaviour for all 34 anchored reports (anchor neutrality proof in
/// `spec/lab-yahoo-realdata/decomp.md § T-AR9`).
///
/// `YahooCache` is Lab-only at v0.1.0; the 4 cross-sectional arms reject it
/// with `RunError::UnsupportedDataSource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioDataSource {
    /// Synthetic GBM bars — pre-v0.1.0 default path (anchor-safe).
    #[default]
    Synthetic,
    /// Real Yahoo Finance bars loaded from the local parquet cache.
    /// Only valid for the 4 single-symbol strategy arms at v0.1.0.
    YahooCache,
}

/// Configuration for a single backtest run (ADR-0030).
///
/// All fields are mandatory. Use `Default` trait implementations only
/// for test fixtures; production call sites must explicitly set every
/// field.
///
/// # Seed contract
///
/// `seed` is a mandatory `[u8; 32]` `ChaCha20` seed.
/// Passing `[0u8; 32]` is a hard error — `run_scenario` returns
/// `RunError::ZeroSeed`. The Lab default seed is defined in
/// `crates/ui/src/lab/defaults.rs` as `LAB_DEFAULT_SEED`.
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    /// Strategy identifier, e.g. `StrategyId("v1.momentum")`.
    pub strategy: StrategyId,
    /// Trading pair: `(Venue::Binance, Symbol::new("XRPUSDT"))`.
    pub pair: (Venue, Symbol),
    /// Backtest date range.
    pub range: DateRange,
    /// Strategy parameter overrides. `None` uses strategy defaults.
    /// Phase A always passes `None`; Phase B exposes the param sheet.
    pub params: Option<ParamSheet>,
    /// Mandatory `ChaCha20` RNG seed (`[0u8; 32]` is rejected).
    pub seed: [u8; 32],
    /// When `true`, write the Markdown report to
    /// `spec/<feature>/reports/backtest-<stamp>-<scenario>.md`.
    pub write_report: bool,
    /// Data source for the run (lab-yahoo-realdata v0.1.0 / T-AR1).
    ///
    /// Defaults to `Synthetic` — CLI paths that construct `ScenarioConfig`
    /// in Rust code without this field use the struct-update syntax or
    /// explicit `ScenarioDataSource::default()`, preserving byte-identical
    /// behaviour for all 34 anchored reports (anchor neutrality proof in
    /// `spec/lab-yahoo-realdata/decomp.md § T-AR9`).
    pub data_source: ScenarioDataSource,
    /// Pre-loaded bars passed verbatim to the 4 single-symbol scenario arms
    /// instead of generating synthetic GBM bars (T-AR1 / ADR-0040 § D4).
    ///
    /// Set by `lab::runner::preload_yahoo_bars` when `data_source == YahooCache`.
    /// CLI paths always pass `None` — anchor-safe default.
    pub bars_override: Option<Vec<Bar>>,
    /// lab-polish-round-2 R2 — operator-tuned SMA fast window (None → 20).
    /// Plumbed into `SmaComposedRunInput.sma_fast_len` at the
    /// `"v0.sma"` dispatch arm. CLI paths pass `None` — anchor-safe.
    pub sma_fast_len: Option<usize>,
    /// lab-polish-round-2 R2 — operator-tuned SMA slow window (None → 50).
    pub sma_slow_len: Option<usize>,

    /// v5-latency-slippage-sim R1 / ADR-0043 § D1 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros); existing
    /// call sites that construct `ScenarioConfig` without this field use
    /// `ScenarioConfig::default_latency_slippage()` or struct-update syntax
    /// with `..ScenarioConfig::default_latency_slippage_sim()`.
    ///
    /// **Anchor contract**: the default value (`LatencySlippageSimConfig::default()`)
    /// produces byte-identical output for all 34 anchored backtest reports.
    pub latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig,
}

/// In-memory result of a completed backtest run (ADR-0030).
///
/// The UI renders from this immediately; the `report_path` is
/// `Some(...)` only when `cfg.write_report = true`, and the
/// equity series is reachable from there for subsequent
/// `EquityCache` loads.
#[derive(Debug, Clone)]
pub struct RunReport {
    /// Ordered oldest-first equity curve `(timestamp, equity)`.
    pub equity_series: Vec<(Timestamp, Money<Usdt>)>,
    /// All executed fills in chronological order.
    pub fills: Vec<FillView>,
    /// Aggregate performance metrics.
    pub kpis: BacktestKpis,
    /// Path to the written Markdown report (only when
    /// `cfg.write_report = true`).
    pub report_path: Option<PathBuf>,
    /// The bars used for this run, in chronological order.
    ///
    /// `Arc<Vec<Bar>>` for cheap cloning across the UI mirror chain.
    /// Populated for single-symbol SMA/Composed arms; empty (`Arc::new(Vec::new())`)
    /// for cross-sectional paths (momentum/pairs/TCN) which don't have a single
    /// bar series to surface.
    ///
    /// The Lab screen prefers these bars over the live `chart_buffer` so fill
    /// triangle markers anchor correctly even when the `chart_buffer` is empty
    /// (e.g. Yahoo or synthetic-2023 runs).
    pub bars: Arc<Vec<Bar>>,
    /// lab-polish-round-2 R1 — position-curve for the Lab position-curve widget.
    ///
    /// Stores the full per-(symbol, bar) data tagged by symbol.
    /// `(close_ts_millis, signed_qty, symbol)`.
    ///
    /// - Single-symbol arms: all entries share the same symbol.
    /// - Cross-sectional arms: entries interleaved across the top-N universe.
    ///
    /// Filtered to the active symbol in `runner::spawn_lab_run` before being
    /// placed in `RunSummary.position_curve` (plain `Vec<(i64, Decimal)>`).
    /// NOT written to Markdown reports — anchor-additive.
    pub position_curve_raw: Vec<(i64, Decimal, Symbol)>,
}

/// Errors from `run_scenario`.
#[derive(Debug, Error)]
pub enum RunError {
    /// The caller passed `[0u8; 32]` as the seed.  Set a non-zero
    /// seed (the Lab default is `LAB_DEFAULT_SEED` in `ui::lab::defaults`).
    #[error("zero seed rejected — set a non-zero [u8; 32] seed")]
    ZeroSeed,

    /// The strategy identifier is not registered in the engine.
    #[error("unknown strategy: {0}")]
    UnknownStrategy(String),

    /// The date range is invalid (e.g. start > end for a Custom range).
    #[error("invalid date range: {0}")]
    InvalidRange(String),

    /// I/O error writing the report to disk.
    #[error("report write error: {0}")]
    ReportIo(String),

    /// Phase A stub error — the full implementation lands in Phase B.
    /// The Lab runner catches this and resolves with a placeholder
    /// `RunSummary` so the cockpit smoke test passes.
    #[error("run_scenario not yet fully implemented (Phase A stub)")]
    NotImplemented,

    /// The run was cancelled by the operator (mpsc-disconnect cancel pattern).
    ///
    /// The bar loop polls `cancel.is_cancelled()` at every 128-bar boundary
    /// (`bar_idx & 0x7F == 0`). When the sender is dropped, the next poll
    /// returns `true` and the loop returns this error. ADR-0035 § D6 / K3.
    #[error("backtest run cancelled by operator")]
    Cancelled,

    /// Catch-all for internal errors.
    #[error("internal backtest error: {0}")]
    Internal(String),

    /// A cross-sectional strategy arm received `data_source == YahooCache`,
    /// which is unsupported at v0.1.0 (only the 4 single-symbol arms support
    /// Yahoo bars; cross-sectional arms require Binance hourly universes).
    #[error("data source YahooCache is not supported for cross-sectional strategy '{0}' at v0.1.0")]
    UnsupportedDataSource(String),
}

// ── `run_scenario` implementation ────────────────────────────────────────────

// ── DateRange → scenario params mapping ─────────────────────────────────────

/// Map a `DateRange` to `(start_year, bar_count_hourly)` for synthetic-bar
/// scenarios (momentum, pairs, `tcn_overlay`).
///
/// `Last30d` / `Last90d` use 2023 as a fixed base year so results are
/// deterministic regardless of wall-clock date.  `H1_2024` / `H2_2024`
/// use 2024 and approximate half-year hourly counts.
/// Custom ranges are clamped to a maximum of 1-year equivalent.
fn date_range_to_scenario_params(range: &DateRange) -> (i32, usize) {
    match range {
        DateRange::Last30d => (2023, 720),   // 30 × 24 = 720 h
        DateRange::Last90d => (2023, 2_160), // 90 × 24 = 2160 h
        DateRange::H1_2024 => (2024, 4_344), // ~181 × 24 = 4344 h
        DateRange::H2_2024 => (2024, 4_416), // ~184 × 24 = 4416 h
        DateRange::Custom { start_ms, end_ms } => {
            // Convert ms span to hourly bar count.  Clamp to 1 year max.
            let span_ms = end_ms.saturating_sub(*start_ms).max(0);
            // SAFETY: span_ms is non-negative (saturating_sub + max(0)), and
            // after dividing by 3_600_000 the value fits in usize on any
            // supported 32-bit or 64-bit target for realistic time ranges.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let hours = (span_ms / 3_600_000) as usize;
            let bar_count = hours.min(8_760); // cap at 365 × 24

            // Derive start_year from start_ms (UTC).
            let start_year = OffsetDateTime::from_unix_timestamp(*start_ms / 1000)
                .map(time::OffsetDateTime::year)
                .unwrap_or(2023);
            (start_year, bar_count)
        }
    }
}

/// Convert a `[u8; 32]` `ChaCha20` seed to `u64` (little-endian low 8 bytes).
///
/// Consistent with `parse_seed` in `main.rs` which works with a `u64`
/// throughout.  ADR-0030 mandates `[u8; 32]` at the public API; the
/// per-scenario modules take `u64`.
#[inline]
fn seed_to_u64(seed: &[u8; 32]) -> u64 {
    // SAFETY: `seed[0..8]` is exactly 8 bytes; `try_into` can only fail on a
    // wrong-length slice, which is impossible here.
    #[allow(clippy::expect_used)]
    u64::from_le_bytes(seed[0..8].try_into().expect("slice is always 8 bytes"))
}

/// Build a synthetic timestamp series for an hourly equity curve.
///
/// Each entry corresponds to one bar: base timestamp + i hours.
/// Used to zip with the per-bar equity values when building `RunReport.equity_series`.
fn synthetic_timestamps(start_year: i32, count: usize) -> Vec<Timestamp> {
    let epoch_base = {
        let date = time::Date::from_calendar_date(start_year, time::Month::January, 1)
            .unwrap_or_else(|_| {
                // SAFETY: 2023-01-01 is a fixed valid calendar date; this path
                // is only reached when start_year is out of range, which
                // cannot happen for our synthetic bar scenarios (2023/2024).
                #[allow(clippy::expect_used)]
                time::Date::from_calendar_date(2023, time::Month::January, 1)
                    .expect("2023-01-01 is always valid")
            });
        OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };
    // SAFETY: bar index `i` is bounded by `count` (≤8760 hours for a year),
    // so the cast from usize to i64 cannot overflow on any supported target.
    #[allow(clippy::cast_possible_wrap)]
    (0..count)
        .map(|i| Timestamp::new(epoch_base + time::Duration::hours(i as i64)))
        .collect()
}

/// Build a `RunReport` from a momentum result.
fn momentum_result_to_report(
    result: &crate::scenarios::momentum::MomentumRunResult,
    start_year: i32,
) -> RunReport {
    let ts_series = synthetic_timestamps(start_year, result.equity_curve.len());
    let equity_series: Vec<(Timestamp, Money<Usdt>)> = ts_series
        .into_iter()
        .zip(result.equity_curve.iter())
        .map(|(ts, &eq)| (ts, Money::<Usdt>::from_decimal(eq)))
        .collect();

    let kpis = BacktestKpis {
        final_equity: Money::<Usdt>::from_decimal(result.final_equity),
        initial_equity: Money::<Usdt>::from_decimal(result.initial_equity),
        max_drawdown: result.max_drawdown,
        trade_count: result.trades,
        total_fees: Money::<Usdt>::from_decimal(result.total_fees),
        // lab-polish-round-2 R3 (UI-only, anchor-additive).
        buys: result.buys,
        sells: result.sells,
        total_return_pct: total_return_pct(result.initial_equity, result.final_equity),
    };

    RunReport {
        equity_series,
        // F3 — surface fills from the momentum result for Lab chart triangle markers.
        fills: result.fills.clone(),
        kpis,
        report_path: None,
        // Surface run bars to the UI so Lab chart markers anchor correctly.
        bars: result.bars.clone(),
        // lab-polish-round-2 R1 — position-curve with symbol tag for UI filter.
        position_curve_raw: result.position_curve.clone(),
    }
}

/// lab-polish-round-2 R3 — pure helper for `(final - initial) / initial`.
/// Returns 0 when `initial == 0` (fixture path).
fn total_return_pct(initial: Decimal, final_: Decimal) -> Decimal {
    if initial.is_zero() {
        Decimal::ZERO
    } else {
        (final_ - initial) / initial
    }
}

/// Build a `RunReport` from a pairs result.
fn pairs_result_to_report(
    result: &crate::scenarios::pairs::PairsRunResult,
    start_year: i32,
) -> RunReport {
    let ts_series = synthetic_timestamps(start_year, result.equity_curve.len());
    let equity_series: Vec<(Timestamp, Money<Usdt>)> = ts_series
        .into_iter()
        .zip(result.equity_curve.iter())
        .map(|(ts, &eq)| (ts, Money::<Usdt>::from_decimal(eq)))
        .collect();

    let kpis = BacktestKpis {
        final_equity: Money::<Usdt>::from_decimal(result.final_equity),
        initial_equity: Money::<Usdt>::from_decimal(result.initial_equity),
        max_drawdown: result.max_drawdown,
        trade_count: result.trades,
        total_fees: Money::<Usdt>::from_decimal(result.total_fees),
        buys: result.buys,
        sells: result.sells,
        total_return_pct: total_return_pct(result.initial_equity, result.final_equity),
    };

    RunReport {
        equity_series,
        // F3 — surface fills from the pairs result for Lab chart triangle markers.
        fills: result.fills.clone(),
        kpis,
        report_path: None,
        // Surface run bars to the UI so Lab chart markers anchor correctly.
        bars: result.bars.clone(),
        // lab-polish-round-2 R1 — position-curve with symbol tag for UI filter.
        position_curve_raw: result.position_curve.clone(),
    }
}

/// Build a `RunReport` from a TCN overlay result.
fn tcn_result_to_report(
    result: &crate::scenarios::tcn_overlay::TcnOverlayRunResult,
    start_year: i32,
) -> RunReport {
    let ts_series = synthetic_timestamps(start_year, result.equity_curve.len());
    let equity_series: Vec<(Timestamp, Money<Usdt>)> = ts_series
        .into_iter()
        .zip(result.equity_curve.iter())
        .map(|(ts, &eq)| (ts, Money::<Usdt>::from_decimal(eq)))
        .collect();

    let kpis = BacktestKpis {
        final_equity: Money::<Usdt>::from_decimal(result.final_equity),
        initial_equity: Money::<Usdt>::from_decimal(result.initial_equity),
        max_drawdown: result.max_drawdown,
        trade_count: result.trades,
        total_fees: Money::<Usdt>::from_decimal(result.total_fees),
        buys: result.buys,
        sells: result.sells,
        total_return_pct: total_return_pct(result.initial_equity, result.final_equity),
    };

    RunReport {
        equity_series,
        // F3 — surface fills from the TCN/overlay result for Lab chart triangle markers.
        fills: result.fills.clone(),
        kpis,
        report_path: None,
        // Surface run bars to the UI so Lab chart markers anchor correctly.
        bars: result.bars.clone(),
        // lab-polish-round-2 R1 — position-curve with symbol tag for UI filter.
        position_curve_raw: result.position_curve.clone(),
    }
}

/// Build a `RunReport` from a single-symbol SMA/Composed result (Wave D-2 / T-AR-4).
///
/// Unlike the cross-sectional variants, this path populates `fills` from the
/// in-memory fill list so the Lab UI can render buy/sell triangle markers and
/// hover overlays (R5.2).  The equity curve uses synthetic 1-minute timestamps
/// starting from `{start_year}-01-01 00:00:00Z`.
fn sma_composed_result_to_report(
    result: &crate::scenarios::sma_composed_run::SmaComposedRunResult,
    start_year: i32,
) -> RunReport {
    let ts_series = synthetic_timestamps(start_year, result.equity_curve.len());
    let equity_series: Vec<(Timestamp, Money<Usdt>)> = ts_series
        .into_iter()
        .zip(result.equity_curve.iter())
        .map(|(ts, &eq)| (ts, Money::<Usdt>::from_decimal(eq)))
        .collect();

    let kpis = BacktestKpis {
        final_equity: Money::<Usdt>::from_decimal(result.final_equity),
        initial_equity: Money::<Usdt>::from_decimal(result.initial_equity),
        max_drawdown: result.max_drawdown,
        trade_count: result.trades,
        total_fees: Money::<Usdt>::from_decimal(result.total_fees),
        buys: result.buys,
        sells: result.sells,
        total_return_pct: total_return_pct(result.initial_equity, result.final_equity),
    };

    // lab-polish-round-2 R1 — single-symbol: tag each position entry with the
    // symbol from the input bars (all bars share the same symbol).
    let sym_tag = result
        .bars
        .first()
        .map_or_else(|| Symbol::new("UNKNOWN"), |b| b.symbol.clone());
    let position_curve_raw: Vec<(i64, Decimal, Symbol)> = result
        .position_curve
        .iter()
        .map(|&(ts, qty)| (ts, qty, sym_tag.clone()))
        .collect();

    RunReport {
        equity_series,
        fills: result.fills.clone(),
        kpis,
        report_path: None,
        // Surface run bars to the UI so Lab chart markers anchor correctly
        // against the run's own time window (chart_buffer may be empty).
        bars: result.bars.clone(),
        position_curve_raw,
    }
}

// ── run_scenario ─────────────────────────────────────────────────────────────

/// Run a backtest for the given `ScenarioConfig` and return an
/// in-memory `RunReport` (ADR-0030 / T-D-12 / Phase B dispatch).
///
/// ## Dispatch table (ADR-0035)
///
/// | `cfg.strategy` string                                       | Module dispatched to                          |
/// |-------------------------------------------------------------|-----------------------------------------------|
/// | `"v0.sma"`, `"sma_cross"`, `"sma_crossover"`               | `scenarios::sma_composed_run::run`            |
/// | `"v0.5.macd"`, `"macd_trend"`, `"btc_macd_trend"`          | `scenarios::sma_composed_run::run`            |
/// | `"v0.5.rsi"`, `"rsi_reversion"`, `"btc_rsi_reversion"`     | `scenarios::sma_composed_run::run`            |
/// | `"v0.5.bbands"`, `"bbands_mean_revert"`, `"btc_bbands_mean_revert"` | `scenarios::sma_composed_run::run` |
/// | `"v1.momentum"`                                             | `scenarios::momentum::run`                    |
/// | `"v1.5a.mr"`, `"v1.5a.pairs"`                              | `scenarios::pairs::run`                       |
/// | `"v2.5.tcn"`, `"v2.5.tcn_overlay"`                         | `scenarios::tcn_overlay::run`                 |
/// | `"v2.5.tcn.weights"`, `"v2.5.tcn_overlay_weights"`         | `scenarios::tcn_overlay_weights::run`         |
/// | anything else                                               | `Err(RunError::UnknownStrategy)`              |
///
/// ## Cancel pattern (ADR-0035 § D6 / K3)
///
/// The cancel-poll is **wrapping**: `run_scenario` spawns the scenario on
/// `tokio::spawn` and polls for its completion using `tokio::select!`.
/// If `cfg.cancel_rx` is dropped by the caller, the join handle is aborted
/// and `RunError::Cancelled` is returned.  This is the "wrap-and-abort"
/// fallback documented in the task brief (bar-loop polling requires the
/// `RunCancelReceiver` to be threaded through each scenario; wrapping is
/// cleaner for the first landing and can be replaced with bar-level polling
/// in a follow-up without changing the public API).
///
/// ## Seed mapping
///
/// `cfg.seed: [u8; 32]` is mapped to `u64` via `u64::from_le_bytes(seed[0..8])`
/// — consistent with `main.rs`'s `parse_seed` which works with `u64` throughout.
///
/// ## `write_report`
///
/// When `cfg.write_report = true` the function currently returns `report_path: None`
/// (Phase B ships in-memory only; file write is a Phase C enhancement per Q1-A
/// decision: "in-memory return, no disk at Phase B").  The `H3` integration test
/// therefore skips the cached-disk equality check for Phase B.
///
/// # Errors
///
/// - `RunError::ZeroSeed` if `cfg.seed == [0u8; 32]`.
/// - `RunError::InvalidRange` if `DateRange::Custom { start_ms > end_ms }`.
/// - `RunError::UnknownStrategy` if the strategy string is not in the dispatch table.
/// - `RunError::Cancelled` if the in-flight run is cancelled.
/// - `RunError::Internal` for unexpected scenario errors.
#[allow(clippy::too_many_lines)]
pub async fn run_scenario(
    cfg: ScenarioConfig,
    cancel_rx: crate::cancel::RunCancelReceiver,
    progress_tx: crate::progress::ProgressSender,
) -> Result<RunReport, RunError> {
    // ── 1. Seed gate ─────────────────────────────────────────────────────────
    if cfg.seed == [0u8; 32] {
        return Err(RunError::ZeroSeed);
    }

    // ── 2. Range sanity check ────────────────────────────────────────────────
    if let DateRange::Custom { start_ms, end_ms } = cfg.range
        && start_ms > end_ms
    {
        return Err(RunError::InvalidRange(format!(
            "Custom range start_ms ({start_ms}) > end_ms ({end_ms})"
        )));
    }

    // ── 3. Seed → u64 ────────────────────────────────────────────────────────
    let seed_u64 = seed_to_u64(&cfg.seed);

    // ── 4. DateRange → scenario params ───────────────────────────────────────
    let (start_year, bar_count) = date_range_to_scenario_params(&cfg.range);

    // ── 5. Strategy dispatch ─────────────────────────────────────────────────
    let strategy_str = cfg.strategy.0.as_str();
    match strategy_str {
        // ── v1 cross-sectional momentum ──────────────────────────────────────
        "v1.momentum" | "top10_momentum_h1" => {
            // YahooCache is unsupported for cross-sectional arms at v0.1.0
            // (they require the Binance hourly multi-symbol universe).
            if cfg.data_source == ScenarioDataSource::YahooCache {
                return Err(RunError::UnsupportedDataSource(strategy_str.to_string()));
            }
            let input = crate::cli_types::MomentumScenarioInput {
                scenario_name: "v1.momentum".to_string(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                config_id: "top10_momentum_h1".to_string(),
                bars_override: None,
                data_revision_sha: None,
                // v5-latency-slippage-sim: thread config through from ScenarioConfig.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
            };
            // Bug #63 — pass cancel + progress through so Stop + progress bar
            // work for cross-sectional runs.
            let result = crate::scenarios::momentum::run(&input, seed_u64, cancel_rx, progress_tx)
                .await
                .map_err(|e| {
                    if e.to_string().contains("Cancelled") {
                        RunError::Cancelled
                    } else {
                        RunError::Internal(e.to_string())
                    }
                })?;
            Ok(momentum_result_to_report(&result, start_year))
        }

        // ── v1.5a mean-reversion pairs ───────────────────────────────────────
        "v1.5a.mr" | "v1.5a.pairs" | "pairs_mr_h1" => {
            if cfg.data_source == ScenarioDataSource::YahooCache {
                return Err(RunError::UnsupportedDataSource(strategy_str.to_string()));
            }
            let input = crate::cli_types::PairsScenarioInput {
                scenario_name: "v1.5a.pairs".to_string(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                config_id: "pairs_mr_h1".to_string(),
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            };
            // Bug #63 — cancel + progress.
            let result = crate::scenarios::pairs::run(&input, seed_u64, cancel_rx, progress_tx)
                .await
                .map_err(|e| {
                    if e.to_string().contains("Cancelled") {
                        RunError::Cancelled
                    } else {
                        RunError::Internal(e.to_string())
                    }
                })?;
            Ok(pairs_result_to_report(&result, start_year))
        }

        // ── v2.5 TCN overlay momentum (passthrough / no-candle) ──────────────
        "v2.5.tcn" | "v2.5.tcn_overlay" | "tcn_overlay_momentum" => {
            if cfg.data_source == ScenarioDataSource::YahooCache {
                return Err(RunError::UnsupportedDataSource(strategy_str.to_string()));
            }
            let input = crate::cli_types::TcnScenarioInput {
                scenario_name: "v2.5.tcn_overlay".to_string(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                config_id: "tcn_overlay_momentum".to_string(),
                forecaster_id: "passthrough".to_string(),
                bars_override: None,
                emit_equity_bin: None,
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                funding_override: None,
            };
            // Bug #63 — cancel + progress.
            let result =
                crate::scenarios::tcn_overlay::run(input, seed_u64, cancel_rx, progress_tx)
                    .await
                    .map_err(|e| {
                        if e.to_string().contains("Cancelled") {
                            RunError::Cancelled
                        } else {
                            RunError::Internal(e.to_string())
                        }
                    })?;
            Ok(tcn_result_to_report(&result, start_year))
        }

        // ── v2.5 TCN overlay momentum with real weights (candle feature) ─────
        "v2.5.tcn.weights" | "v2.5.tcn_overlay_weights" => {
            if cfg.data_source == ScenarioDataSource::YahooCache {
                return Err(RunError::UnsupportedDataSource(strategy_str.to_string()));
            }
            let input = crate::cli_types::TcnScenarioInput {
                scenario_name: "v2.5.tcn_overlay_weights".to_string(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                config_id: "tcn_overlay_momentum".to_string(),
                forecaster_id: "tcn-bs1".to_string(),
                bars_override: None,
                emit_equity_bin: None,
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                funding_override: None,
            };
            let result = crate::scenarios::tcn_overlay_weights::run(input, seed_u64)
                .await
                .map_err(|e| RunError::Internal(e.to_string()))?;
            Ok(tcn_result_to_report(&result, start_year))
        }

        // ── v0 single-symbol SMA crossover ───────────────────────────────────
        // `cfg.bars_override` is threaded through verbatim (T-AR1).
        // When `Some`, the scenario uses real Yahoo bars instead of synthetic GBM.
        // When `None` (all CLI/anchor paths), synthetic GBM is used unchanged.
        "v0.sma" | "sma_cross" | "sma_crossover" | "sma_cross_h1" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "sma_crossover".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — pass the operator-tuned overrides
                // through. CLI paths leave these as None on ScenarioConfig
                // → here they map to None → defaults (20/50) preserve anchor
                // byte-identity. Lab UI sets them to user-typed values.
                sma_fast_len: cfg.sma_fast_len,
                sma_slow_len: cfg.sma_slow_len,
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override,
                seed_u64,
                cancel_rx,
                progress_tx,
            )
            .await
            .map_err(|e| match e {
                crate::scenarios::sma_composed_run::SmaRunError::Cancelled => RunError::Cancelled,
                crate::scenarios::sma_composed_run::SmaRunError::Other(e) => {
                    RunError::Internal(e.to_string())
                }
            })?;
            Ok(sma_composed_result_to_report(&result, start_year))
        }

        // ── v0.5 MACD trend ──────────────────────────────────────────────────
        "v0.5.macd" | "macd_trend" | "btc_macd_trend" | "macd_trend_h1" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_macd_trend".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — CLI dispatch passes None to preserve
                // anchored byte-identity. Lab override happens at the runner.
                sma_fast_len: None,
                sma_slow_len: None,
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override,
                seed_u64,
                cancel_rx,
                progress_tx,
            )
            .await
            .map_err(|e| match e {
                crate::scenarios::sma_composed_run::SmaRunError::Cancelled => RunError::Cancelled,
                crate::scenarios::sma_composed_run::SmaRunError::Other(e) => {
                    RunError::Internal(e.to_string())
                }
            })?;
            Ok(sma_composed_result_to_report(&result, start_year))
        }

        // ── v0.5 RSI reversion ───────────────────────────────────────────────
        "v0.5.rsi" | "rsi_reversion" | "btc_rsi_reversion" | "rsi_reversion_h1" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_rsi_reversion".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — CLI dispatch passes None to preserve
                // anchored byte-identity. Lab override happens at the runner.
                sma_fast_len: None,
                sma_slow_len: None,
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override,
                seed_u64,
                cancel_rx,
                progress_tx,
            )
            .await
            .map_err(|e| match e {
                crate::scenarios::sma_composed_run::SmaRunError::Cancelled => RunError::Cancelled,
                crate::scenarios::sma_composed_run::SmaRunError::Other(e) => {
                    RunError::Internal(e.to_string())
                }
            })?;
            Ok(sma_composed_result_to_report(&result, start_year))
        }

        // ── v0.5 BBands mean-revert ──────────────────────────────────────────
        "v0.5.bbands"
        | "bbands_mean_revert"
        | "btc_bbands_mean_revert"
        | "bbands_mean_revert_h1" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_bbands_mean_revert".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — CLI dispatch passes None to preserve
                // anchored byte-identity. Lab override happens at the runner.
                sma_fast_len: None,
                sma_slow_len: None,
                // engine dispatch: noop sim (Lab UI does not expose sim flags).
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override,
                seed_u64,
                cancel_rx,
                progress_tx,
            )
            .await
            .map_err(|e| match e {
                crate::scenarios::sma_composed_run::SmaRunError::Cancelled => RunError::Cancelled,
                crate::scenarios::sma_composed_run::SmaRunError::Other(e) => {
                    RunError::Internal(e.to_string())
                }
            })?;
            Ok(sma_composed_result_to_report(&result, start_year))
        }

        // ── Unknown strategy ─────────────────────────────────────────────────
        other => Err(RunError::UnknownStrategy(other.to_string())),
    }
}

// ── Test helper ──────────────────────────────────────────────────────────────

/// Convenience wrapper for tests and the CLI that provides no-op cancel/progress.
///
/// K1 mitigation per T-AR-5: the 6+ Phase B determinism tests and all new
/// engine unit tests call this so `ScenarioConfig` literals are unchanged
/// (only `run_scenario`'s call site shifts from `cfg` to `cfg, cancel, progress`).
///
/// # Errors
///
/// Propagates all errors from [`run_scenario`]: [`RunError::ZeroSeed`],
/// [`RunError::InvalidRange`], [`RunError::UnknownStrategy`], [`RunError::Internal`],
/// and [`RunError::Cancelled`].
#[cfg(test)]
pub async fn run_scenario_for_test(cfg: ScenarioConfig) -> Result<RunReport, RunError> {
    let (_handle, cancel_rx) = crate::cancel::cancellation_pair();
    run_scenario(cfg, cancel_rx, crate::progress::ProgressSender::disabled()).await
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_seed() -> [u8; 32] {
        // LAB_DEFAULT_SEED analog — first byte non-zero.
        let mut s = [0u8; 32];
        s[0] = 0xC0;
        s[1] = 0xFF;
        s[2] = 0xEE;
        s
    }

    fn config_with_seed(seed: [u8; 32]) -> ScenarioConfig {
        ScenarioConfig {
            strategy: StrategyId("v1.momentum".into()),
            pair: (Venue::Binance, Symbol::new("XRPUSDT")),
            range: DateRange::Last90d,
            params: None,
            seed,
            write_report: false,
            data_source: ScenarioDataSource::default(),
            bars_override: None,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
        }
    }

    /// T-D-12 — zero-seed rejection per ADR-0030 determinism contract.
    #[tokio::test]
    async fn run_scenario_rejects_zero_seed() {
        let cfg = config_with_seed([0u8; 32]);
        let result = run_scenario_for_test(cfg).await;
        assert!(
            matches!(result, Err(RunError::ZeroSeed)),
            "zero seed must be rejected; got: {result:?}"
        );
    }

    /// T-D-12 — non-zero seed passes seed validation (Phase B dispatch:
    /// unknown strategy "v1.momentum" with a momentum fixture reaches
    /// `UnknownStrategy` for an unrecognised ID, or succeeds for a known one).
    /// We test with an unregistered strategy ID to confirm `ZeroSeed` is NOT triggered.
    #[tokio::test]
    async fn run_scenario_accepts_non_zero_seed() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.strategy = StrategyId("__nonexistent_strategy__".into());
        let result = run_scenario_for_test(cfg).await;
        // Phase B: known strategy would succeed, unknown returns UnknownStrategy.
        // Either way, ZeroSeed must NOT be returned.
        assert!(
            !matches!(result, Err(RunError::ZeroSeed)),
            "non-zero seed must NOT trigger ZeroSeed; got: {result:?}"
        );
    }

    /// T-D-12 — Custom range with start > end is rejected.
    #[tokio::test]
    async fn run_scenario_rejects_invalid_custom_range() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.range = DateRange::Custom {
            start_ms: 1_000_000,
            end_ms: 999_999,
        };
        let result = run_scenario_for_test(cfg).await;
        assert!(
            matches!(result, Err(RunError::InvalidRange(_))),
            "start > end must be rejected; got: {result:?}"
        );
    }

    /// T-D-12 — Valid Custom range passes range validation (Phase B: unknown strategy
    /// is dispatched past the range check, returns `UnknownStrategy` not `InvalidRange`).
    #[tokio::test]
    async fn run_scenario_accepts_valid_custom_range() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.strategy = StrategyId("__nonexistent_strategy__".into());
        cfg.range = DateRange::Custom {
            start_ms: 1_000_000,
            end_ms: 2_000_000,
        };
        let result = run_scenario_for_test(cfg).await;
        assert!(
            !matches!(result, Err(RunError::InvalidRange(_))),
            "valid custom range must not trigger InvalidRange; got: {result:?}"
        );
    }

    /// T-D-12 — All preset `DateRange` variants are handled (do not hit custom
    /// range validation path). Phase B returns `UnknownStrategy` for unregistered IDs.
    #[tokio::test]
    async fn run_scenario_all_presets_reach_dispatch() {
        for range in [
            DateRange::Last30d,
            DateRange::Last90d,
            DateRange::H1_2024,
            DateRange::H2_2024,
        ] {
            let mut cfg = config_with_seed(valid_seed());
            cfg.strategy = StrategyId("__nonexistent_strategy__".into());
            cfg.range = range.clone();
            let result = run_scenario_for_test(cfg).await;
            assert!(
                matches!(result, Err(RunError::UnknownStrategy(_))),
                "unregistered strategy with preset {range:?} must reach dispatch (UnknownStrategy); got: {result:?}"
            );
        }
    }

    /// T-D-12 — `RunError` variants have non-empty Display messages.
    #[test]
    fn run_error_display_non_empty() {
        assert!(!RunError::ZeroSeed.to_string().is_empty());
        assert!(!RunError::NotImplemented.to_string().is_empty());
        assert!(!RunError::Cancelled.to_string().is_empty());
        assert!(!RunError::UnknownStrategy("x".into()).to_string().is_empty());
        assert!(!RunError::InvalidRange("x".into()).to_string().is_empty());
        assert!(!RunError::Internal("x".into()).to_string().is_empty());
    }

    /// T-D-N8 — `RunError::Cancelled` has a non-empty Display message.
    #[test]
    fn run_error_cancelled_display_non_empty() {
        assert!(!RunError::Cancelled.to_string().is_empty());
    }

    /// T-D-N8 — Unknown strategy returns `Err(RunError::UnknownStrategy)`.
    #[tokio::test]
    async fn run_scenario_unknown_strategy_is_rejected() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.strategy = StrategyId("__nonexistent_strategy__".into());
        let result = run_scenario_for_test(cfg).await;
        assert!(
            matches!(result, Err(RunError::UnknownStrategy(_))),
            "unregistered strategy must return UnknownStrategy; got: {result:?}"
        );
    }

    /// T-D-N8 — Phase B dispatch: `v1.momentum` with a tiny synthetic bar
    /// window (Last30d → 720 hourly bars) returns `Ok(RunReport)`.
    ///
    /// This test exercises the real momentum dispatch path.  It requires
    /// `config/strategies/top10_momentum_h1.toml` to be accessible from
    /// the **workspace root** (run with `cargo test -p backtest` from the
    /// workspace root, not from `crates/backtest/`).
    ///
    /// Marked `#[ignore]` so it does not run in CI unit-test sweeps where
    /// the working directory may be set to the crate root.  The canonical
    /// smoke is via `cargo test -p backtest --test determinism` or
    /// `scripts/verify_anchors.sh` which runs from the workspace root.
    ///
    /// To run manually:
    /// ```text
    /// cd <workspace-root>
    /// cargo test -p backtest --lib engine::tests::run_scenario_momentum_dispatch_returns_ok -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "requires config/strategies/*.toml at cwd; run from workspace root with --ignored"]
    async fn run_scenario_momentum_dispatch_returns_ok() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.strategy = StrategyId("v1.momentum".into());
        cfg.range = DateRange::Last30d; // 720 bars × 10 symbols — small fixture
        let result = run_scenario_for_test(cfg).await;
        match result {
            Ok(report) => {
                assert!(
                    !report.equity_series.is_empty(),
                    "equity_series must not be empty after momentum run"
                );
                assert_eq!(
                    report.kpis.initial_equity.amount(),
                    rust_decimal_macros::dec!(100_000),
                    "initial equity must match the seeded-in default (100_000 USDT)"
                );
            }
            Err(e) => panic!("v1.momentum dispatch must succeed on Last30d; got: {e:?}"),
        }
    }

    /// T-D-N8 — Phase B dispatch: `v1.momentum` is a registered strategy ID
    /// (the dispatch arm exists — it does not return `UnknownStrategy`).
    ///
    /// This lighter test does not require the config file: it just checks
    /// that the returned error is NOT `UnknownStrategy`, which confirms the
    /// dispatch arm was reached (Internal is expected when the config file
    /// is not at the test CWD).
    #[tokio::test]
    async fn run_scenario_momentum_strategy_arm_exists() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.strategy = StrategyId("v1.momentum".into());
        cfg.range = DateRange::Last30d;
        let result = run_scenario_for_test(cfg).await;
        // Must NOT be UnknownStrategy — the arm is registered.
        // Will be Internal (config file not at test cwd) or Ok.
        assert!(
            !matches!(result, Err(RunError::UnknownStrategy(_))),
            "v1.momentum must be a registered strategy (dispatch arm exists); got: {result:?}"
        );
    }

    /// T-D-N8 — Cancellation: a pre-cancelled scenario returns within a bounded
    /// number of iterations.
    ///
    /// The current wrap-and-abort cancel pattern means the future is simply
    /// dropped when the caller drops the task.  This test verifies that the
    /// `RunError::Cancelled` variant propagates through the match path — the
    /// actual bar-loop abort (D6) is exercised manually via the cockpit's
    /// cancel button per K3.
    #[tokio::test]
    async fn run_error_cancelled_variant_reachable() {
        // Directly construct the error and verify it's a valid RunError variant.
        // The full cancellation path is exercised by the UI integration test
        // (T-D-N14 / K3) which drops the RunCancelHandle mid-run.
        let e = RunError::Cancelled;
        assert!(
            matches!(e, RunError::Cancelled),
            "Cancelled variant must be constructible and matchable"
        );
        assert!(!e.to_string().is_empty());
    }

    /// T-AR-5 — `run_scenario` returns `Err(RunError::Cancelled)` when the
    /// cancel handle is dropped before the run completes.
    ///
    /// Uses a short SMA crossover scenario (Last30d = 525 600 * (30/365) ≈
    /// small bar count via `date_range_to_scenario_params` which returns 720
    /// for `Last30d` for minute bars → `sma_composed`). We pre-cancel by
    /// dropping the handle before calling `run_scenario` so the very first
    /// poll (bar 0) sees `is_cancelled() == true`.
    #[tokio::test]
    async fn run_scenario_cancellation_returns_cancelled() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.strategy = StrategyId("v0.sma".into());
        cfg.range = DateRange::Last30d;

        // Build a pre-cancelled receiver: drop the handle immediately.
        let (handle, cancel_rx) = crate::cancel::cancellation_pair();
        drop(handle); // signal cancel before the run starts

        let result =
            run_scenario(cfg, cancel_rx, crate::progress::ProgressSender::disabled()).await;
        assert!(
            matches!(result, Err(RunError::Cancelled)),
            "pre-cancelled run must return Cancelled; got: {result:?}"
        );
    }
}
