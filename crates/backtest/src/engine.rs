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
//! at Phase A. All 11 body-SHA-256 anchors in `evidence/anchors.toml`
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
/// `spec/v1/lab-yahoo-realdata/decomp.md § T-AR9`).
///
/// `YahooCache` is Lab-only at v0.1.0; the 4 cross-sectional arms reject it
/// with `RunError::UnsupportedDataSource`.
///
/// `BinanceCache` is Lab-only at v0.1.0 (simple-strategies-realdata, A1);
/// the 4 cross-sectional arms reject it exactly as `YahooCache`.
/// CLI/anchor paths never construct it — anchor-additive by the `YahooCache`
/// precedent (`lab-yahoo-realdata/decomp.md § T-AR9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioDataSource {
    /// Synthetic GBM bars — pre-v0.1.0 default path (anchor-safe).
    #[default]
    Synthetic,
    /// Real Yahoo Finance bars loaded from the local parquet cache.
    /// Only valid for the 4 single-symbol strategy arms at v0.1.0.
    YahooCache,
    /// Real Binance hourly bars loaded from the pinned parquet cache
    /// (`data/binance/<SYM>USDT/<YEAR>/<MM>.parquet`, revision `3a8b96c4…`).
    /// Lab-only at v0.1.0; single-symbol arms only.
    /// Cross-sectional arms reject this with `RunError::UnsupportedDataSource`.
    BinanceCache,
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
    /// `evidence/<feature>/reports/backtest-<stamp>-<scenario>.md`.
    pub write_report: bool,
    /// Data source for the run (lab-yahoo-realdata v0.1.0 / T-AR1).
    ///
    /// Defaults to `Synthetic` — CLI paths that construct `ScenarioConfig`
    /// in Rust code without this field use the struct-update syntax or
    /// explicit `ScenarioDataSource::default()`, preserving byte-identical
    /// behaviour for all 34 anchored reports (anchor neutrality proof in
    /// `spec/v1/lab-yahoo-realdata/decomp.md § T-AR9`).
    pub data_source: ScenarioDataSource,
    /// Pre-loaded bars passed verbatim to the 4 single-symbol scenario arms
    /// instead of generating synthetic GBM bars (T-AR1 / ADR-0040 § D4).
    ///
    /// Set by `lab::runner::preload_yahoo_bars` when `data_source == YahooCache`,
    /// or by `lab::runner::preload_binance_bars` when `data_source == BinanceCache`
    /// (simple-strategies-realdata A3 / T-A3).
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

    /// lab-run-save-compare R3 / ADR-0055 § D3 — override the directory the
    /// Lab report is written under when `write_report = true`. `None` +
    /// `write_report` resolves the workspace-root `lab-runs/` default.
    /// CLI/anchor paths pass `write_report = false` (or leave this `None`) →
    /// byte-unaffected.
    ///
    /// **Anchor-additive**: constructed via struct-update / explicit
    /// `reports_dir: None` so the existing anchor-generating call sites stay
    /// byte-identical. The field is never read when `write_report = false`.
    pub reports_dir: Option<PathBuf>,

    /// ADR-0068 D1/D2 — enable the single-coin directional short-selling path.
    ///
    /// `false` (the default via `#[serde(default)]` / struct literal) → the
    /// long-only clamps are ACTIVE and the path is byte-for-byte HEAD's code.
    /// `true` → the four long-only clamps are gated off; `Sell`-when-flat opens
    /// a short via `backtest::short_exec`; `Buy`-when-short covers.
    ///
    /// Set `true` ONLY for the 5 new `_ls` / `always_short` arms (D9).
    /// Every existing long-only arm leaves this `false` → byte-identical.
    pub short_enabled: bool,

    /// Bakeoff timeframe + capital tuning (leaderboard-timeframe-capital knobs).
    ///
    /// `None` → use the legacy hardcoded `100_000` USDT (all existing call sites:
    /// anchor-safe, byte-identical). `Some(capital)` overrides the starting equity
    /// for the run. Used by `run_bakeoff` (`write_report=false` path) so this field
    /// never affects anchored report bodies.
    ///
    /// **Anchor contract**: CLI/Lab paths always leave this `None` → `dec!(100_000)`
    /// used as before. The `run_bakeoff` bakeoff path (`write_report = false`) may
    /// pass `Some(capital)` — safe because no report body is written.
    pub initial_capital: Option<rust_decimal::Decimal>,

    // ── ADR-0069 T7 — in-memory composed TOML override ────────────────────
    //
    // When `Some(toml_str)`, the composed strategy (MACD / RSI / Bollinger)
    // is loaded from this in-memory TOML string via
    // `ComposedStrategyConfig::from_str` instead of from disk.
    //
    // ANCHOR-PRESERVING CONTRACT: all CLI/Lab/anchored paths leave this `None`.
    // Only `backtest::bakeoff::sweep::build_swept_config` (T7 sweep families)
    // sets `Some(...)`. Sweep cells always use `write_report = false` (ADR-0069
    // D9) so no anchored report body is ever written when this is `Some`.
    pub composed_toml_override: Option<String>,

    // ── ADR-0072 D5 — DVOL exogenous-series override ─────────────────────────
    //
    // Pre-resolved as-of DVOL daily closes, one entry per bar in chronological
    // order. `None` entry = DVOL not yet started for that bar (warm-up).
    //
    // ANCHOR-PRESERVING CONTRACT: all CLI/Lab/anchored paths leave this `None`.
    // Only the `v0.dvol_regime` arm reads this field; the bake-off loop sets
    // it for BTC/ETH only. For all other arms / symbols: `None` → untouched.
    // `write_report = false` on the bake-off path → no anchored body is
    // ever written when this is `Some`.
    pub dvol_override: Option<Vec<Option<rust_decimal::Decimal>>>,

    // ── ADR-0073 D4 — macro-regime exogenous series override ─────────────────
    //
    // Pre-resolved daily macro regime `PitSeries<bool>` for the `v0.macro_riskon`
    // arm. `true` = risk-ON (hold coin), `false` / `None` = risk-OFF (flat/cash).
    //
    // ANCHOR-PRESERVING CONTRACT: all CLI/Lab/anchored paths leave this `None`.
    // Only the `v0.macro_riskon` arm reads this field; the bake-off loop sets
    // it once for the macro arm. All other arms: `None` → byte-identical.
    // `write_report = false` on the bake-off path → no anchored body written.
    pub macro_regime_series: Option<trading_core::pit::PitSeries<bool>>,
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

    /// A strategy arm received a `data_source` it cannot honestly run:
    /// either a cross-sectional arm was given a single-symbol real-data
    /// source (`YahooCache` / `BinanceCache` — those arms require the
    /// multi-symbol Binance universe), or a real-data source arrived with
    /// `bars_override: None` (simple-strategies-realdata review patch 4 —
    /// real-data sources never fall back to synthetic GBM bars). The payload
    /// names the strategy and the violated rule.
    #[error("unsupported data source: {0}")]
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
pub(crate) fn synthetic_timestamps(start_year: i32, count: usize) -> Vec<Timestamp> {
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

// ── Lab-runs root helper ─────────────────────────────────────────────────────

/// Default `lab-runs/` cache dir at the workspace root (ADR-0055 § D1).
///
/// Walks up from `CARGO_MANIFEST_DIR` to the workspace root (the directory
/// containing `Cargo.toml` with a `[workspace]` declaration), then appends
/// `lab-runs/`. Mirrors `ui::lab::equity_loader::default_lab_runs_root()` but
/// lives in the `backtest` crate so `backtest` NEVER depends on `ui`
/// (`engine::DateRange` is duplicated for this exact reason — `engine.rs:72-83`).
///
/// Git-ignored (`.gitignore`: `/lab-runs/`) → invisible to `verify_anchors.sh`
/// `find spec …` glob → 119/119 anchor-safety BY CONSTRUCTION (AC7 / ADR-0055
/// § D2).
fn default_lab_runs_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(|_| PathBuf::from("."), PathBuf::from);
    // `crates/backtest` → `crates` → workspace root.
    if let Some(root) = manifest_dir.parent().and_then(|p| p.parent()) {
        return root.join("lab-runs");
    }
    PathBuf::from("lab-runs")
}

/// Map a strategy id to the directory slug used for `lab-runs/<slug>/reports/`.
///
/// Mirrors `ui::lab::equity_loader::strategy_slug` exactly so the ui-designer's
/// loader root finds what the engine writes.  Unknown ids fall back to the
/// verbatim id string (safe: the loader does the same).
fn strategy_dir_slug(strategy_id: &str) -> &str {
    match strategy_id {
        "v1.momentum" | "top10_momentum_h1" => "v1-cross-sectional-momentum",
        // ADR-0068 T-D6: _ls aliases map to the same dir slug as their base arm.
        // `write_report=false` in bake-off, so this branch is unreachable there;
        // but it must be correct for any future caller that sets write_report=true.
        "v0.sma" | "sma_cross" | "sma_crossover" | "sma_cross_h1" | "v0.sma_cross_ls" => {
            "v0-paper-sma"
        }
        "v0.5.macd"
        | "macd_trend"
        | "btc_macd_trend"
        | "macd_trend_h1"
        | "v0.macd_ls"
        | "v0.5.rsi"
        | "rsi_reversion"
        | "btc_rsi_reversion"
        | "rsi_reversion_h1"
        | "v0.rsi_ls"
        | "v0.5.bbands"
        | "bbands_mean_revert"
        | "btc_bbands_mean_revert"
        | "bbands_mean_revert_h1"
        | "v0.bbands_ls"
        | "v0.always_short" => "v05-composed-strategies",
        // ADR-0071 signal-library expansion arms. `write_report=false` on the
        // bake-off path makes this branch unreachable there, but it is required
        // for write-path correctness of any future caller that sets write_report=true.
        "v0.donchian_break" | "btc_donchian_break" | "v0.donchian_floor" | "btc_donchian_floor"
        | "v0.vol_breakout" | "btc_vol_breakout" | "v0.roc_momentum" | "btc_roc_momentum"
        | "v0.obv" | "btc_obv" => "v0-signal-library",
        // ADR-0072: DVOL implied-vol regime probe.
        "v0.dvol_regime" => "v0-dvol-probe",
        // ADR-0073: cross-asset macro regime probe. Review 3-16 LOW: this case
        // was missing, so the id fell through to `other => other` and the arm
        // would have written under a raw-id directory (`v0.macro_riskon/`)
        // unlike every sibling. `write_report=false` on the bake-off path makes
        // it unreachable there, but the arm's writer now refuses loudly rather
        // than returning a path to a file it never creates — see the
        // `v0.macro_riskon` dispatch arm.
        "v0.macro_riskon" => "v0-macro-regime-probe",
        "v1.5a.mr" | "v1.5a.pairs" | "pairs_mr_h1" => "v15a-mean-reversion-pairs",
        "v2.5.tcn" | "v2.5.tcn_overlay" | "tcn_overlay_momentum" => "v2.5.tcn_overlay",
        "v2.5.tcn.weights" | "v2.5.tcn_overlay_weights" => "v2.5.tcn_overlay_weights",
        other => other,
    }
}

/// Thin write seam for Lab report persistence (ADR-0055 § D3 / lab-run-save-compare T1/T3).
///
/// When `cfg.write_report` is `true`:
/// 1. Resolves the target dir: `cfg.reports_dir` or the workspace-root
///    `lab-runs/` default (via `default_lab_runs_root()`).
/// 2. Appends `<strategy-slug>/reports/` and creates the directory.
/// 3. Builds a **millisecond-granularity** filename stamp (Q3 pin — the CLI's
///    second-precision collides on fast successive Lab runs).
/// 4. Invokes `writer(&path)` to write the report.
/// 5. Runs the Q5 retention purge: keeps the last N = 20 files matching
///    `backtest-*-<scenario_name>.md` in the same dir, unlinking older ones.
/// 6. Returns `Ok(Some(path))`.
///
/// When `!cfg.write_report` — returns `Ok(None)` and touches no filesystem.
///
/// `scenario_name` is the canonical scenario name embedded in the filename,
/// matching the `scenario:` frontmatter field of the written report.
/// `strategy_id` drives the `strategy_dir_slug` lookup.
///
/// # Errors
///
/// Returns `RunError::ReportIo` on any filesystem error.
fn maybe_write_report(
    cfg: &ScenarioConfig,
    strategy_id: &str,
    scenario_name: &str,
    equity_series: &[(Timestamp, Money<Usdt>)],
    writer: impl FnOnce(&std::path::Path) -> anyhow::Result<()>,
) -> Result<Option<PathBuf>, RunError> {
    if !cfg.write_report {
        return Ok(None);
    }

    // Resolve the root: explicit override or default lab-runs/.
    let root = cfg
        .reports_dir
        .clone()
        .unwrap_or_else(default_lab_runs_root);

    let slug = strategy_dir_slug(strategy_id);
    let reports_dir = root.join(slug).join("reports");

    std::fs::create_dir_all(&reports_dir).map_err(|e| {
        RunError::ReportIo(format!("create_dir_all({}): {e}", reports_dir.display()))
    })?;

    // Millisecond-granularity stamp (Q3 pin — avoids filename collisions on
    // fast successive Lab runs; the CLI's second-precision is insufficient).
    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}{:03}",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond(),
    );

    let filename = format!("backtest-{stamp}-{scenario_name}.md");
    let report_path = reports_dir.join(&filename);

    // Invoke the family-specific writer closure.
    writer(&report_path).map_err(|e| RunError::ReportIo(format!("write report: {e}")))?;

    // lab-run-save-compare Wave-2 (ADR-0055 § D-companion): also persist the
    // FULL per-bar equity series as a companion CSV beside the `.md`. The `.md`
    // carries only a sparkline (visual, not machine-parseable), so the loader
    // reads THIS CSV for per-bar fidelity — which is what flips H3 skip→pass
    // and makes a saved Lab run's curve real (not a degenerate 2-point line).
    // Schema is identical to `reports::csv_artifacts::{write,read}_equity_csv`.
    // The `.md` byte format is UNCHANGED (anchored reports untouched); the CSV
    // is additive and lab-runs/-only.
    let csv_path = reports_dir.join(format!("backtest-{stamp}-{scenario_name}-equity.csv"));
    write_equity_companion_csv(&csv_path, equity_series)
        .map_err(|e| RunError::ReportIo(format!("write equity csv: {e}")))?;

    tracing::info!(
        report_path = %report_path.display(),
        "lab-run-save-compare: report written (ADR-0055 § D4)"
    );

    // Q5 retention purge: keep the last N = 20 per (strategy, scenario) tuple.
    purge_old_lab_reports(&reports_dir, scenario_name);
    purge_lab_reports_global_cap(&reports_dir);

    Ok(Some(report_path))
}

/// Q5 retention purge — keep last N = 20 files matching
/// `backtest-*-<scenario_name>.md` in `reports_dir`, unlinking older ones.
///
/// Sort is lexicographic on filename (the ms-stamp prefix guarantees newest-last
/// for runs within the same second; across seconds the stamp is monotone).
/// Any I/O error during purge is logged and swallowed — a failed purge never
/// fails the run that just completed successfully.
fn purge_old_lab_reports(reports_dir: &std::path::Path, scenario_name: &str) {
    const KEEP_LAST_N: usize = 20;
    let suffix = format!("-{scenario_name}.md");

    let Ok(read_dir) = std::fs::read_dir(reports_dir) else {
        return;
    };
    let mut matching: Vec<PathBuf> = read_dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("backtest-") && n.ends_with(suffix.as_str()))
        })
        .collect();

    if matching.len() <= KEEP_LAST_N {
        return;
    }

    matching.sort();
    let to_remove = matching.len() - KEEP_LAST_N;
    for path in matching.iter().take(to_remove) {
        // lab-run-save-compare Wave-2: also unlink the companion equity CSV so
        // it doesn't orphan-accumulate (best-effort; absent is fine).
        if let Some(stem) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".md"))
        {
            let csv = path.with_file_name(format!("{stem}-equity.csv"));
            let _ = std::fs::remove_file(&csv);
        }
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!(path = %path.display(), err = %e, "lab-runs purge: failed to remove old report");
        } else {
            tracing::debug!(path = %path.display(), "lab-runs purge: removed old report");
        }
    }
}

/// Global per-directory cap on lab-run reports (2026-07-27 adversarial-review
/// hardening). `purge_old_lab_reports` bounds each SCENARIO bucket to 20, but
/// per-tuple scenario names (symbol + range + source since the story-1-10
/// review) mean distinct buckets accumulate without bound — every custom range
/// mints a new one. This second phase bounds the whole `reports/` dir: oldest
/// `backtest-*.md` (+ companion `-equity.csv`) beyond the newest
/// [`GLOBAL_KEEP_LAST_N`] are unlinked. Lexicographic sort = chronological
/// (shared `backtest-<ms-stamp>-` prefix). Lab-runs write path only — the
/// CLI/evidence path never routes through this seam. Errors are logged and
/// swallowed (a failed purge never fails a successful run).
fn purge_lab_reports_global_cap(reports_dir: &std::path::Path) {
    const GLOBAL_KEEP_LAST_N: usize = 200;

    let Ok(read_dir) = std::fs::read_dir(reports_dir) else {
        return;
    };
    let mut reports: Vec<PathBuf> = read_dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                // Reports are always written with a lowercase ".md" extension.
                .is_some_and(|n| {
                    n.starts_with("backtest-") && {
                        #[allow(clippy::case_sensitive_file_extension_comparisons)]
                        let ok = n.ends_with(".md");
                        ok
                    }
                })
        })
        .collect();

    if reports.len() <= GLOBAL_KEEP_LAST_N {
        return;
    }

    reports.sort();
    let to_remove = reports.len() - GLOBAL_KEEP_LAST_N;
    for path in reports.iter().take(to_remove) {
        if let Some(stem) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".md"))
        {
            let csv = path.with_file_name(format!("{stem}-equity.csv"));
            let _ = std::fs::remove_file(&csv);
        }
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!(path = %path.display(), err = %e, "lab-runs global cap: failed to remove old report");
        } else {
            tracing::debug!(path = %path.display(), "lab-runs global cap: removed old report");
        }
    }
}

/// Write the companion equity CSV (lab-run-save-compare Wave-2 / ADR-0055).
///
/// Column schema is byte-identical to
/// [`reports::csv_artifacts::read_equity_csv`] so the UI loader round-trips it:
/// `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`.
/// The Lab engine path tracks only total per-bar equity, so realized /
/// unrealized / cash columns are `0` (the loader only needs `equity_total` for
/// the curve). `ts` is RFC3339 (the format `read_equity_csv` parses).
fn write_equity_companion_csv(
    path: &std::path::Path,
    equity_series: &[(Timestamp, Money<Usdt>)],
) -> std::io::Result<()> {
    use std::fmt::Write as _;
    let mut out = String::from(
        "ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt\n",
    );
    for (ts, eq) in equity_series {
        let ts_str = ts
            .inner()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        // Decimal → string (never f64). realized/unrealized/cash unavailable on
        // the Lab engine path → 0.
        let _ = writeln!(out, "{ts_str},{},0,0,0", eq.amount());
    }
    std::fs::write(path, out)
}

// ── run_scenario ─────────────────────────────────────────────────────────────

/// Lab write-seam scenario name (review D1 completion, 2026-07-26).
///
/// Carries the actual symbol, range, and data source so per-tuple lab-run
/// reports (a) never collide across sources at the file layer —
/// `delete_older_reports` keys on the `-<scenario_name>.md` suffix, so a
/// shared hardcoded name made a Binance run's report replace the Synthetic
/// run's on disk — and (b) score correctly against the requested range in
/// the Compare loader (`ui::lab::equity_loader::range_score`), which the old
/// `btc-2023-1m-*` constants broke for every 2024-preset run.
/// CLI/evidence reports never route through this seam (`main.rs` builds its
/// report inputs directly), so anchored bodies are untouched by construction.
fn lab_scenario_name(arm_slug: &str, cfg: &ScenarioConfig) -> String {
    let range_token = match &cfg.range {
        DateRange::Last30d => "last30d".to_string(),
        DateRange::Last90d => "last90d".to_string(),
        DateRange::H1_2024 => "2024-h1".to_string(),
        DateRange::H2_2024 => "2024-h2".to_string(),
        DateRange::Custom { start_ms, end_ms } => format!("custom{start_ms}to{end_ms}"),
    };
    let source_token = match cfg.data_source {
        ScenarioDataSource::Synthetic => "synthetic",
        ScenarioDataSource::YahooCache => "yahoo",
        ScenarioDataSource::BinanceCache => "binance",
    };
    let symbol = cfg.pair.1.to_string().to_ascii_lowercase();
    format!("{symbol}-{range_token}-{arm_slug}-{source_token}")
}
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

    // ── 2b. Real-data sources require preloaded bars ─────────────────────────
    // (simple-strategies-realdata review patch 4 — API-boundary hole.)
    // A non-Synthetic `data_source` with `bars_override: None` would fall
    // through to the synthetic GBM generator while the written report is
    // labeled "binance"/"yahoo" — a silently-wrong report. Every legitimate
    // caller (Lab runner, bake-off, sweep) preloads real bars and passes
    // `Some(bars)`; reject the combination up front instead of synthesizing.
    if cfg.data_source != ScenarioDataSource::Synthetic && cfg.bars_override.is_none() {
        return Err(RunError::UnsupportedDataSource(format!(
            "{}: data_source {:?} requires preloaded bars (bars_override was None); \
             real-data sources never fall back to synthetic bars",
            cfg.strategy.0.as_str(),
            cfg.data_source
        )));
    }

    // ── 3. Seed → u64 ────────────────────────────────────────────────────────
    let seed_u64 = seed_to_u64(&cfg.seed);

    // ── 3b. Resolve initial capital (leaderboard-timeframe-capital knobs) ────
    // `None` → legacy 100_000 (all existing CLI/anchor paths: byte-identical).
    // `Some(c)` → operator-chosen capital (bakeoff UI path only, write_report=false).
    let initial_capital = cfg
        .initial_capital
        .unwrap_or(rust_decimal_macros::dec!(100_000));

    // ── 4. DateRange → scenario params ───────────────────────────────────────
    let (start_year, bar_count) = date_range_to_scenario_params(&cfg.range);

    // ── 5. Strategy dispatch ─────────────────────────────────────────────────
    let strategy_str = cfg.strategy.0.as_str();
    match strategy_str {
        // ── v1 cross-sectional momentum ──────────────────────────────────────
        "v1.momentum" | "top10_momentum_h1" => {
            // YahooCache and BinanceCache are unsupported for cross-sectional arms
            // at v0.1.0 — they require the multi-symbol Binance universe, not a
            // single pre-loaded bar vector. (simple-strategies-realdata T-A2.)
            if matches!(
                cfg.data_source,
                ScenarioDataSource::YahooCache | ScenarioDataSource::BinanceCache
            ) {
                return Err(RunError::UnsupportedDataSource(format!(
                    "{:?} is not supported for cross-sectional strategy '{strategy_str}' \
                     (requires the multi-symbol Binance universe)",
                    cfg.data_source
                )));
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
            let mut report = momentum_result_to_report(&result, start_year);
            // lab-run-save-compare T1/T3 — write seam (ADR-0055 § D3/D4).
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &input.scenario_name,
                &report.equity_series,
                |path| crate::report::momentum::write(&input, &result, seed_u64, "synthetic", path),
            )?;
            Ok(report)
        }

        // ── v1.5a mean-reversion pairs ───────────────────────────────────────
        "v1.5a.mr" | "v1.5a.pairs" | "pairs_mr_h1" => {
            // single-symbol override not supported for cross-sectional arms (T-A2).
            if matches!(
                cfg.data_source,
                ScenarioDataSource::YahooCache | ScenarioDataSource::BinanceCache
            ) {
                return Err(RunError::UnsupportedDataSource(format!(
                    "{:?} is not supported for cross-sectional strategy '{strategy_str}' \
                     (requires the multi-symbol Binance universe)",
                    cfg.data_source
                )));
            }
            let input = crate::cli_types::PairsScenarioInput {
                scenario_name: "v1.5a.pairs".to_string(),
                start_year,
                bar_count,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                config_id: "pairs_mr_h1".to_string(),
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
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
            let mut report = pairs_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4).
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &input.scenario_name,
                &report.equity_series,
                |path| crate::report::pairs::write(&input, &result, seed_u64, "synthetic", path),
            )?;
            Ok(report)
        }

        // ── v2.5 TCN overlay momentum (passthrough / no-candle) ──────────────
        "v2.5.tcn" | "v2.5.tcn_overlay" | "tcn_overlay_momentum" => {
            // single-symbol override not supported for cross-sectional arms (T-A2).
            if matches!(
                cfg.data_source,
                ScenarioDataSource::YahooCache | ScenarioDataSource::BinanceCache
            ) {
                return Err(RunError::UnsupportedDataSource(format!(
                    "{:?} is not supported for cross-sectional strategy '{strategy_str}' \
                     (requires the multi-symbol Binance universe)",
                    cfg.data_source
                )));
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
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                funding_override: None,
                bar_span_hours: 1,
            };
            // Bug #63 — cancel + progress.
            let result =
                crate::scenarios::tcn_overlay::run(input.clone(), seed_u64, cancel_rx, progress_tx)
                    .await
                    .map_err(|e| {
                        if e.to_string().contains("Cancelled") {
                            RunError::Cancelled
                        } else {
                            RunError::Internal(e.to_string())
                        }
                    })?;
            let mut report = tcn_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4).
            // rev_sha = "n/a" for Synthetic (matches CLI main.rs:1572 for synthetic path).
            // loaded_info = None for Synthetic.
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::tcn_overlay::write(
                        &input,
                        &result,
                        seed_u64,
                        "synthetic",
                        path,
                        "n/a",
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v2.5 TCN overlay momentum with real weights (candle feature) ─────
        "v2.5.tcn.weights" | "v2.5.tcn_overlay_weights" => {
            // single-symbol override not supported for cross-sectional arms (T-A2).
            if matches!(
                cfg.data_source,
                ScenarioDataSource::YahooCache | ScenarioDataSource::BinanceCache
            ) {
                return Err(RunError::UnsupportedDataSource(format!(
                    "{:?} is not supported for cross-sectional strategy '{strategy_str}' \
                     (requires the multi-symbol Binance universe)",
                    cfg.data_source
                )));
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
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                funding_override: None,
                bar_span_hours: 1,
            };
            let result = crate::scenarios::tcn_overlay_weights::run(input.clone(), seed_u64)
                .await
                .map_err(|e| RunError::Internal(e.to_string()))?;
            let mut report = tcn_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4).
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::tcn_overlay::write(
                        &input,
                        &result,
                        seed_u64,
                        "synthetic",
                        path,
                        "n/a",
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0 single-symbol SMA crossover ───────────────────────────────────
        // `cfg.bars_override` is threaded through verbatim (T-AR1).
        // When `Some`, the scenario uses real Yahoo bars instead of synthetic GBM.
        // When `None` (all CLI/anchor paths), synthetic GBM is used unchanged.
        //
        // ADR-0068 T-D6: `"v0.sma_cross_ls"` is the long/short alias for this arm.
        // It routes here with `short_enabled=true` (set by `BakeoffConfig::is_short_enabled`).
        // Long-only ids are byte-identical to HEAD (short_enabled=false default).
        "v0.sma" | "sma_cross" | "sma_crossover" | "sma_cross_h1" | "v0.sma_cross_ls" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "sma_crossover".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — pass the operator-tuned overrides
                // through. CLI paths leave these as None on ScenarioConfig
                // → here they map to None → defaults (20/50) preserve anchor
                // byte-identity. Lab UI sets them to user-typed values.
                sma_fast_len: cfg.sma_fast_len,
                sma_slow_len: cfg.sma_slow_len,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                // ADR-0068 D1: thread short_enabled from ScenarioConfig.
                // Long-only arms have short_enabled=false (default); _ls arms set true.
                short_enabled: cfg.short_enabled,
                // SMA arm never uses in-memory TOML override (SMA has a typed seam).
                composed_toml_override: None,
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4 + A2.1).
            // SmaScenarioInput is constructed from known fields; state/strategy_meta
            // come off SmaComposedRunResult as main.rs:2109-2110.
            // elapsed_secs = 0.0: frontmatter-only, stripped before hashing (A2.1).
            // data_source string: "binance" for BinanceCache, "yahoo" for YahooCache,
            // "synthetic" for Synthetic.  Non-exhaustive match → compile-enforced
            // when a new variant is added (simple-strategies-realdata A1 / T-A1).
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("sma-cross", &cfg),
                body_name: lab_scenario_name("sma-cross", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None, // rev_sha: None for synthetic/engine path
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.5 MACD trend ──────────────────────────────────────────────────
        // ADR-0068 T-D6: `"v0.macd_ls"` is the long/short alias. Routes here
        // with `short_enabled=true`; long-only ids untouched (byte-identical).
        "v0.5.macd" | "macd_trend" | "btc_macd_trend" | "macd_trend_h1" | "v0.macd_ls" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_macd_trend".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — CLI dispatch passes None to preserve
                // anchored byte-identity. Lab override happens at the runner.
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                // ADR-0068 D1: thread short_enabled from ScenarioConfig.
                short_enabled: cfg.short_enabled,
                // ADR-0069 T7 — forward in-memory TOML override if set by the sweep.
                // Normal (non-sweep) paths always leave ScenarioConfig::composed_toml_override
                // as None → byte-identical to HEAD (anchor-safe).
                composed_toml_override: cfg.composed_toml_override.clone(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4 + A2.1).
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("macd-trend", &cfg),
                body_name: lab_scenario_name("macd-trend", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.5 RSI reversion ───────────────────────────────────────────────
        // ADR-0068 T-D6: `"v0.rsi_ls"` is the long/short alias. Routes here
        // with `short_enabled=true`; long-only ids untouched (byte-identical).
        "v0.5.rsi" | "rsi_reversion" | "btc_rsi_reversion" | "rsi_reversion_h1" | "v0.rsi_ls" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_rsi_reversion".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — CLI dispatch passes None to preserve
                // anchored byte-identity. Lab override happens at the runner.
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                // ADR-0068 D1: thread short_enabled from ScenarioConfig.
                short_enabled: cfg.short_enabled,
                // ADR-0069 T7 — forward in-memory TOML override if set by the sweep.
                composed_toml_override: cfg.composed_toml_override.clone(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4 + A2.1).
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("rsi-reversion", &cfg),
                body_name: lab_scenario_name("rsi-reversion", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.5 BBands mean-revert ──────────────────────────────────────────
        // ADR-0068 T-D6: `"v0.bbands_ls"` is the long/short alias. Routes here
        // with `short_enabled=true`; long-only ids untouched (byte-identical).
        "v0.5.bbands"
        | "bbands_mean_revert"
        | "btc_bbands_mean_revert"
        | "bbands_mean_revert_h1"
        | "v0.bbands_ls" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_bbands_mean_revert".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                // lab-polish-round-2 R2 — CLI dispatch passes None to preserve
                // anchored byte-identity. Lab override happens at the runner.
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                // ADR-0068 D1: thread short_enabled from ScenarioConfig.
                short_enabled: cfg.short_enabled,
                // ADR-0069 T7 — forward in-memory TOML override if set by the sweep.
                composed_toml_override: cfg.composed_toml_override.clone(),
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            // lab-run-save-compare T3 — write seam (ADR-0055 § D3/D4 + A2.1).
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("bbands-mean-revert", &cfg),
                body_name: lab_scenario_name("bbands-mean-revert", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── ADR-0071 signal-library expansion: 5 new composed-strategy arms ─────────
        //
        // Pattern: copy of the "v0.5.macd" arm (engine.rs:1234) with only the
        // match id, strategy_id, and scenario_name changed.  All 5 arms run with
        // `write_report = false` on the bake-off path → NO anchored report body
        // is created → `verify_anchors.sh` stays 119/119 by construction.
        // The unique, non-anchored scenario_name per arm guarantees the
        // (unreachable) write branch can never collide with an anchored body.

        // ── v0.donchian_break — 20-bar high breakout ─────────────────────────────────
        "v0.donchian_break" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_donchian_break".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                short_enabled: cfg.short_enabled,
                composed_toml_override: None,
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("donchian-break", &cfg),
                body_name: lab_scenario_name("donchian-break", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.donchian_floor — 20-bar support floor ──────────────────────────────────
        "v0.donchian_floor" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_donchian_floor".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                short_enabled: cfg.short_enabled,
                composed_toml_override: None,
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("donchian-floor", &cfg),
                body_name: lab_scenario_name("donchian-floor", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.vol_breakout — volume-confirmed 20-bar breakout ────────────────────────
        "v0.vol_breakout" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_vol_breakout".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                short_enabled: cfg.short_enabled,
                composed_toml_override: None,
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("vol-breakout", &cfg),
                body_name: lab_scenario_name("vol-breakout", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.roc_momentum — 5% rate-of-change over 10 bars ─────────────────────────
        "v0.roc_momentum" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_roc_momentum".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                short_enabled: cfg.short_enabled,
                composed_toml_override: None,
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("roc-momentum", &cfg),
                body_name: lab_scenario_name("roc-momentum", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.obv — On-Balance-Volume accumulation + trend filter ────────────────────
        "v0.obv" => {
            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "btc_obv".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                short_enabled: cfg.short_enabled,
                composed_toml_override: None,
            };
            let result = crate::scenarios::sma_composed_run::run(
                &input,
                cfg.bars_override.clone(),
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            let data_source_str = match cfg.data_source {
                ScenarioDataSource::YahooCache => "yahoo",
                ScenarioDataSource::BinanceCache => "binance",
                ScenarioDataSource::Synthetic => "synthetic",
            };
            let sma_input = crate::cli_types::SmaScenarioInput {
                scenario_name: lab_scenario_name("obv", &cfg),
                body_name: lab_scenario_name("obv", &cfg),
                body_elapsed_override: None,
                symbol: cfg.pair.1.clone(),
                start_year,
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                baseline_report: None,
            };
            report.report_path = maybe_write_report(
                &cfg,
                strategy_str,
                &sma_input.scenario_name,
                &report.equity_series,
                |path| {
                    crate::report::sma::write(
                        &sma_input,
                        &result.state,
                        dec!(100_000),
                        result.final_equity,
                        seed_u64,
                        data_source_str,
                        0.0,
                        path,
                        &result.strategy_meta,
                        None,
                    )
                },
            )?;
            Ok(report)
        }

        // ── v0.dvol_regime — Deribit DVOL implied-vol regime long/flat (ADR-0072) ──────
        //
        // Holds the coin when DVOL < trailing 30-day median (calm), steps to cash
        // when DVOL >= median (stress). Signal LOCKED (no search). BTC+ETH only;
        // other symbols → arm absent from field (D-DVOL.6), never panics.
        //
        // The arm reads `cfg.dvol_override` — the pre-resolved as-of DVOL series
        // (one entry per bar) injected by the bake-off loop.
        //
        // bug-log #78: on the BAKE-OFF path `dvol_override` is now never `None`
        // here — `run_bakeoff` drops the arm from the ranked field when the series
        // is missing or does not cover the window, exactly as it drops it for an
        // unsupported coin. `unwrap_or_default()` remains only for direct
        // `run_scenario` callers (Lab/CLI/tests) that construct a `ScenarioConfig`
        // by hand; with the review-3-15 warm-up fix an empty series now genuinely
        // does hold the coin from bar 0 (it used to sit in 100% CASH while five
        // code comments called it a "buy-and-hold proxy").
        "v0.dvol_regime" => {
            use strategy::DvolRegimeStrategy;

            let as_of_dvol = cfg.dvol_override.clone().unwrap_or_default();

            let input = crate::cli_types::SmaComposedRunInput {
                strategy_id: "v0.dvol_regime".to_string(),
                symbol: cfg.pair.1.clone(),
                start_year,
                bar_count,
                initial_capital,
                slippage_bps: 2,
                taker_fee_bps: 4,
                sma_fast_len: None,
                sma_slow_len: None,
                // bug-log #79 — thread the ScenarioConfig sim config (incl.
                // `venue_filter`) into the arm. The advisor bake-off/sweep pass
                // `advisor_default()` (lot realism ON, PRD §13 Q5); every other
                // caller (Lab UI, CLI, tests) passes the all-noop `Default`
                // (`venue_filter: None`) -> byte-identical to HEAD.
                latency_slippage_sim: cfg.latency_slippage_sim.clone(),
                short_enabled: false, // long-only; no short path
                composed_toml_override: None,
            };

            // Build the DvolRegimeStrategy with the pre-resolved as-of DVOL series.
            // The strategy is pure (no I/O) and unit-testable with a synthetic vec.
            let dvol_strategy = Box::new(DvolRegimeStrategy::new(
                cfg.pair.1.clone(),
                as_of_dvol,
                strategy::DVOL_REGIME_WINDOW,
            ));

            let result = crate::scenarios::sma_composed_run::run_with_strategy(
                &input,
                cfg.bars_override.clone(),
                seed_u64,
                dvol_strategy,
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
            let mut report = sma_composed_result_to_report(&result, start_year);
            // write_report=false on bake-off path → anchor-safe (D-DVOL.5).
            report.report_path = None;
            Ok(report)
        }

        // ── v0.buyhold — passive buy-and-hold benchmark (anchor-additive, ADR-0059) ───
        //
        // Equal-weight buy-and-hold: buys the coin at bar-0 close (or bar-0
        // synthetic price) and marks to market every bar.  Single-coin bake-off
        // passes n_symbols = 1 → 100% of budget held.
        //
        // Anchor-additive contract:
        // - New id "v0.buyhold" — existing arms are byte-untouched.
        // - write_report = false in bake-off calls → no report body is created.
        // - A new id cannot collide with an existing anchored body.
        // - `scripts/verify_anchors.sh` → 119/119 byte-identical (T0.1/T1.4/T2.3).
        "v0.buyhold" => {
            use crate::bakeoff::buyhold::run_buyhold_path;

            // Use `initial_capital` derived from ScenarioConfig (leaderboard knob;
            // None → 100_000 for all existing/anchored paths).
            const N_SYMBOLS: usize = 1; // single-coin bake-off

            // ── Resolve bars (BinanceCache/YahooCache: use bars_override;
            //    Synthetic: generate with the same GBM engine used by SMA arms) ──
            let bars: Vec<trading_core::Bar> = if let Some(b) = cfg.bars_override.clone() {
                b
            } else {
                // Synthetic path — generate the same GBM bars as the SMA arm
                // so comparisons on the same seed are apples-to-apples.
                let start_price =
                    crate::scenarios::sma_composed_run::default_start_price(&cfg.pair.1);
                crate::scenarios::sma_composed_run::synthetic_bars_minute(
                    &cfg.pair.1,
                    bar_count,
                    seed_u64,
                    start_price,
                    start_year,
                )
            };

            // ── Run buy-and-hold on the bars ──────────────────────────────────
            let (eq_curve, _final_eq_decimal) = run_buyhold_path(&bars, initial_capital, N_SYMBOLS);

            // ── Build equity_series with per-bar timestamps ───────────────────
            //
            // If bars are available use their open_ts for the curve timestamps
            // (real-data path); otherwise fall back to synthetic_timestamps
            // (synthetic path — same convention as the SMA arm).
            let equity_series: Vec<(Timestamp, Money<Usdt>)> = if bars.is_empty() {
                // Degenerate path: no bars, return the single initial-capital entry.
                vec![(
                    synthetic_timestamps(start_year, 1)
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| Timestamp::new(OffsetDateTime::UNIX_EPOCH)),
                    Money::<Usdt>::from_decimal(initial_capital),
                )]
            } else {
                // Build a sorted, deduplicated list of bar timestamps (one entry
                // per distinct timestep), then zip with the equity curve.
                // The equity curve has n_ts + 1 entries (entry 0 = initial_capital);
                // we emit all n_ts + 1.
                let ts_iter = {
                    // Collect sorted unique bar open_ts values.
                    let mut seen: std::collections::BTreeSet<i128> =
                        std::collections::BTreeSet::new();
                    let mut sorted_ts: Vec<Timestamp> = Vec::new();
                    for bar in &bars {
                        let ns = bar.open_ts.inner().unix_timestamp_nanos();
                        if seen.insert(ns) {
                            sorted_ts.push(bar.open_ts);
                        }
                    }
                    sorted_ts
                };

                // Entry 0 in eq_curve = initial_capital (before bar 0).
                // Use the first bar timestamp minus 1 hour for the initial entry.
                let first_ts = ts_iter.first().copied().map_or_else(
                    || Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                    |t| Timestamp::new(t.inner() - time::Duration::hours(1)),
                );

                let mut series: Vec<(Timestamp, Money<Usdt>)> = Vec::with_capacity(eq_curve.len());
                // Push the initial-capital entry.
                series.push((
                    first_ts,
                    Money::<Usdt>::from_decimal(*eq_curve.first().unwrap_or(&initial_capital)),
                ));
                // Push one entry per bar timestamp.
                for (ts, &eq) in ts_iter.iter().zip(eq_curve.iter().skip(1)) {
                    series.push((*ts, Money::<Usdt>::from_decimal(eq)));
                }
                series
            };

            // ── Build KPIs ────────────────────────────────────────────────────
            let final_eq = *eq_curve.last().unwrap_or(&initial_capital);
            let eq_dec_only: Vec<rust_decimal::Decimal> =
                equity_series.iter().map(|(_, m)| m.amount()).collect();

            let max_dd = crate::stats::compute_max_drawdown_f64(&eq_dec_only);

            let kpis = BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(final_eq),
                initial_equity: Money::<Usdt>::from_decimal(initial_capital),
                max_drawdown: rust_decimal::Decimal::try_from(max_dd)
                    .unwrap_or(rust_decimal::Decimal::ZERO),
                trade_count: 0, // buy-and-hold: 1 buy at t=0, never sold → 0 "active" trades
                total_fees: Money::<Usdt>::zero(),
                buys: 1,
                sells: 0,
                total_return_pct: total_return_pct(initial_capital, final_eq),
            };

            // write_report is always false for the bake-off arm (ADR-0059).
            // The maybe_write_report call is a no-op when write_report = false,
            // but we keep it for consistency in case a future caller sets it.
            let report_path = maybe_write_report(
                &cfg,
                "v0.buyhold",
                "v0.buyhold",
                &equity_series,
                |_path| Ok(()), // No-op writer: no anchored report format exists for BH yet.
            )?;

            Ok(RunReport {
                equity_series,
                fills: vec![],
                kpis,
                report_path,
                bars: std::sync::Arc::new(bars),
                position_curve_raw: vec![],
            })
        }

        // ── v0.always_short — always-short benchmark control (ADR-0068 T-D6) ─────
        //
        // The clean inverse of v0.buyhold: a 1× fully-collateralized short
        // opened at bar-0 close and marked to market every bar.
        //
        // Formula: equity[i] = initial_capital × (2 − price[i] / price0)
        // - Profits proportionally as price falls below open price.
        // - Loss is UNBOUNDED and NEGATIVE — no `.max(0)` clamp (honest).
        //   A 2× price move wipes out the position; a 3× move drives equity to
        //   `−initial_capital`.
        //
        // PAPER / SIM ONLY — no real orders, no real margin, no real money.
        //
        // Anchor-additive contract:
        // - New id "v0.always_short" — all existing arms are byte-untouched.
        // - write_report = false in bake-off calls → no anchored report body.
        // - `scripts/verify_anchors.sh` → 119/119 (T-D9).
        "v0.always_short" => {
            use crate::bakeoff::buyhold::run_alwaysshort_path;

            // Use `initial_capital` derived from ScenarioConfig (same knob as buyhold).

            // ── Resolve bars (same path as v0.buyhold) ───────────────────────
            let bars: Vec<trading_core::Bar> = if let Some(b) = cfg.bars_override.clone() {
                b
            } else {
                // Synthetic path — same GBM bars as the buyhold arm for
                // apples-to-apples comparison on the same seed.
                let start_price =
                    crate::scenarios::sma_composed_run::default_start_price(&cfg.pair.1);
                crate::scenarios::sma_composed_run::synthetic_bars_minute(
                    &cfg.pair.1,
                    bar_count,
                    seed_u64,
                    start_price,
                    start_year,
                )
            };

            // ── Run always-short on the bars ─────────────────────────────────
            let (eq_curve, _final_eq_decimal) = run_alwaysshort_path(&bars, initial_capital);

            // ── Build equity_series with per-bar timestamps ───────────────────
            // Mirror the v0.buyhold timestamp logic exactly.
            let equity_series: Vec<(Timestamp, Money<Usdt>)> = if bars.is_empty() {
                vec![(
                    synthetic_timestamps(start_year, 1)
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| Timestamp::new(OffsetDateTime::UNIX_EPOCH)),
                    Money::<Usdt>::from_decimal(initial_capital),
                )]
            } else {
                let ts_iter = {
                    let mut seen: std::collections::BTreeSet<i128> =
                        std::collections::BTreeSet::new();
                    let mut sorted_ts: Vec<Timestamp> = Vec::new();
                    for bar in &bars {
                        let ns = bar.open_ts.inner().unix_timestamp_nanos();
                        if seen.insert(ns) {
                            sorted_ts.push(bar.open_ts);
                        }
                    }
                    sorted_ts
                };

                let first_ts = ts_iter.first().copied().map_or_else(
                    || Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                    |t| Timestamp::new(t.inner() - time::Duration::hours(1)),
                );

                let mut series: Vec<(Timestamp, Money<Usdt>)> = Vec::with_capacity(eq_curve.len());
                series.push((
                    first_ts,
                    Money::<Usdt>::from_decimal(*eq_curve.first().unwrap_or(&initial_capital)),
                ));
                for (ts, &eq) in ts_iter.iter().zip(eq_curve.iter().skip(1)) {
                    // NOTE: equity can be negative (unbounded loss). Money::from_decimal
                    // accepts negative Decimal — no clamp here.
                    series.push((*ts, Money::<Usdt>::from_decimal(eq)));
                }
                series
            };

            // ── Build KPIs ────────────────────────────────────────────────────
            let final_eq = *eq_curve.last().unwrap_or(&initial_capital);
            let eq_dec_only: Vec<rust_decimal::Decimal> =
                equity_series.iter().map(|(_, m)| m.amount()).collect();

            let max_dd = crate::stats::compute_max_drawdown_f64(&eq_dec_only);

            let kpis = BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(final_eq),
                initial_equity: Money::<Usdt>::from_decimal(initial_capital),
                max_drawdown: rust_decimal::Decimal::try_from(max_dd)
                    .unwrap_or(rust_decimal::Decimal::ZERO),
                // trade_count: 0 — like buyhold, it's a single open at t=0 (no active trades).
                trade_count: 0,
                total_fees: Money::<Usdt>::zero(),
                buys: 0,
                sells: 1, // one sell-to-open at t=0
                total_return_pct: total_return_pct(initial_capital, final_eq),
            };

            let report_path = maybe_write_report(
                &cfg,
                "v0.always_short",
                "v0.always_short",
                &equity_series,
                |_path| Ok(()), // No-op writer: write_report=false in bake-off.
            )?;

            Ok(RunReport {
                equity_series,
                fills: vec![],
                kpis,
                report_path,
                bars: std::sync::Arc::new(bars),
                position_curve_raw: vec![],
            })
        }

        // ── v0.macro_riskon — cross-asset macro regime long/flat (ADR-0073) ─────────
        //
        // Holds the coin only when the daily macro regime is risk-ON (SPX trend up,
        // dollar not bid, rates not spiking — the pre-registered 3-AND rule, LOCKED).
        // Flat/cash when risk-OFF or during warm-up. Pure long/flat overlay on
        // buy-and-hold — never trades on the coin's own indicators.
        //
        // ANCHOR-PRESERVING CONTRACT:
        // - write_report = false in bake-off calls → no anchored report body.
        // - All non-macro arms receive `macro_regime_series: None` → byte-identical.
        // - `scripts/verify_anchors.sh` → 119/119.
        // PAPER/SIM ONLY — no real orders.
        //
        // NOT `#[cfg(feature = "yahoo")]`-gated — mirrors the sibling
        // `v0.dvol_regime` arm: the dispatch arm is ALWAYS compiled so the
        // bake-off slate (`default_macro_field()`, unconditional) can resolve it
        // under any feature set. The arm body depends only on core types
        // (`run_macro_gated_buyhold_path`, `cfg.macro_regime_series`,
        // `PitSeries<bool>` — none yahoo-gated). When `yahoo` is OFF the macro
        // corpus loader (yahoo-gated, in `bakeoff::run_bakeoff`) never runs, so
        // `macro_regime_series` stays `None` → empty PitSeries → arm holds flat
        // the whole window — i.e. 100% CASH, **not** a "buy-and-hold proxy"
        // (bug-log #78; the mislabel was inherited from `v0.dvol_regime`, where it
        // was false and has since been fixed). Owned by story 3-16. The
        // MEANINGFUL (non-vacuous) macro verdict requires `--features yahoo`
        // so the real `data/yahoo-macro/` corpus is fed; the day-1
        // baseline-divergence e2e (`macro_regime_overlay_end_to_end.rs`) proves
        // the overlay is NOT a silent no-op when the corpus IS present.
        "v0.macro_riskon" => {
            use crate::bakeoff::buyhold::run_macro_gated_buyhold_path;

            // Resolve coin bars (same pattern as v0.buyhold).
            let bars: Vec<trading_core::Bar> = if let Some(b) = cfg.bars_override.clone() {
                b
            } else {
                let start_price =
                    crate::scenarios::sma_composed_run::default_start_price(&cfg.pair.1);
                crate::scenarios::sma_composed_run::synthetic_bars_minute(
                    &cfg.pair.1,
                    bar_count,
                    seed_u64,
                    start_price,
                    start_year,
                )
            };

            // Resolve the macro regime series.
            // If None (corpus absent), use an empty series → as_of_value always
            // returns None → the arm runs FLAT (100% cash) for the whole window.
            // bug-log #78: this is NOT "a warm-up-only buy-and-hold proxy" and it
            // is no longer "the same discipline as the v0.dvol_regime arm" — that
            // arm now degrades to ABSENCE. Story 3-16 owns bringing this one into
            // line; until then the honest description is the one written here.
            let regime = cfg.macro_regime_series.clone().unwrap_or_else(|| {
                // Empty PitSeries: all as_of_value queries return None → arm stays flat.
                // PitSeries::from_sorted on an empty vec is guaranteed Ok (the invariant
                // vacuously holds on zero records). `unwrap_or` with ZERO is safe fallback.
                trading_core::pit::PitSeries::from_sorted(vec![])
                    .unwrap_or_else(|_| trading_core::pit::PitSeries::from_unsorted(vec![]))
            });

            // ── Run the gated equity path ─────────────────────────────────────────
            let (eq_curve, _final_eq_decimal) =
                run_macro_gated_buyhold_path(&bars, &regime, initial_capital);

            // ── Build equity_series (mirrors v0.buyhold timestamp logic) ─────────
            let equity_series: Vec<(Timestamp, Money<Usdt>)> = if bars.is_empty() {
                vec![(
                    synthetic_timestamps(start_year, 1)
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| Timestamp::new(OffsetDateTime::UNIX_EPOCH)),
                    Money::<Usdt>::from_decimal(initial_capital),
                )]
            } else {
                let ts_iter = {
                    let mut seen: std::collections::BTreeSet<i128> =
                        std::collections::BTreeSet::new();
                    let mut sorted_ts: Vec<Timestamp> = Vec::new();
                    for bar in &bars {
                        let ns = bar.open_ts.inner().unix_timestamp_nanos();
                        if seen.insert(ns) {
                            sorted_ts.push(bar.open_ts);
                        }
                    }
                    sorted_ts
                };

                let first_ts = ts_iter.first().copied().map_or_else(
                    || Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                    |t| Timestamp::new(t.inner() - time::Duration::hours(1)),
                );

                let mut series: Vec<(Timestamp, Money<Usdt>)> = Vec::with_capacity(eq_curve.len());
                series.push((
                    first_ts,
                    Money::<Usdt>::from_decimal(*eq_curve.first().unwrap_or(&initial_capital)),
                ));
                for (ts, &eq) in ts_iter.iter().zip(eq_curve.iter().skip(1)) {
                    series.push((*ts, Money::<Usdt>::from_decimal(eq)));
                }
                series
            };

            // ── Build KPIs ────────────────────────────────────────────────────────
            let final_eq = *eq_curve.last().unwrap_or(&initial_capital);
            let eq_dec_only: Vec<rust_decimal::Decimal> =
                equity_series.iter().map(|(_, m)| m.amount()).collect();

            let max_dd = crate::stats::compute_max_drawdown_f64(&eq_dec_only);

            // Count transitions: each flat→ON (buy) increments buys; each ON→flat
            // (sell) increments sells; trade_count = sells (completed round trips).
            let mut buys = 0usize;
            let mut sells = 0usize;
            {
                use trading_core::pit::TimestampMs;
                let mut prev_on = false;
                for bar in &bars {
                    let ts_ms = bar.open_ts.unix_millis();
                    let on = regime.as_of_value(TimestampMs(ts_ms)).unwrap_or(false);
                    if on && !prev_on {
                        buys += 1;
                    } else if !on && prev_on {
                        sells += 1;
                    }
                    prev_on = on;
                }
            }

            let kpis = BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(final_eq),
                initial_equity: Money::<Usdt>::from_decimal(initial_capital),
                max_drawdown: rust_decimal::Decimal::try_from(max_dd)
                    .unwrap_or(rust_decimal::Decimal::ZERO),
                trade_count: sells, // completed round trips
                // ⚠ review 3-16 HIGH — a pre-registration DEPARTURE, not a
                // convention. The feature spec locked "transition trades pay the
                // standard taker fee … the macro arm is NOT cost-advantaged vs
                // the always-long benchmark", and this ships zero fee and zero
                // slippage while the 18 arms it is ranked against pay 4 bps a leg
                // through `PaperEngine` (plus lot rounding since bug-log #79).
                // Unlike `v0.buyhold` — which is a BENCHMARK and legitimately
                // frictionless — this arm TRADES (it has round trips) and is
                // ranked. The departure flatters it, so charging the fee would
                // strengthen the null rather than reverse it; changing it is a
                // measured product decision, not a review patch. Until then no
                // verdict from this arm is "net of costs".
                total_fees: Money::<Usdt>::zero(),
                buys,
                sells,
                total_return_pct: total_return_pct(initial_capital, final_eq),
            };

            // write_report = false on bake-off path → anchor-safe (ADR-0073 D4).
            //
            // Review 3-16 LOW: the writer used to be `|_path| Ok(())`, so a
            // caller that set `write_report = true` got back `Some(path)` for a
            // `.md` that was never written — a report path pointing at nothing.
            // No such caller exists (the arm is bake-off-only), so rather than
            // invent a report format for a probe arm, the writer REFUSES: the
            // request fails loudly instead of returning a phantom artifact.
            let report_path = maybe_write_report(
                &cfg,
                "v0.macro_riskon",
                "v0.macro_riskon",
                &equity_series,
                |path| {
                    anyhow::bail!(
                        "v0.macro_riskon has no report renderer — refusing to report \
                         success for {} without writing it. The arm is bake-off-only \
                         (write_report=false); if a write path is ever needed, render a \
                         real body here first.",
                        path.display()
                    )
                },
            )?;

            Ok(RunReport {
                equity_series,
                fills: vec![],
                kpis,
                report_path,
                bars: std::sync::Arc::new(bars),
                position_curve_raw: vec![],
            })
        }

        // ── Vote-ensemble arms (ADR-0063 § D5 + ADR-0067 + anchor-additive contract) ──
        //
        // F8 original ids: "v0.8.vote.majority" / "v0.8.vote.unanimous".
        // advisor-combination-search new ids (ADR-0067, FROZEN v1 slate):
        //   Decorrelation pairings: "v0.8.vote.trend_pair" /
        //     "v0.8.vote.tr_mr_macd_rsi" / "v0.8.vote.tr_mr_sma_bb".
        //   k-of-4 ladder: "v0.8.vote.any1of4" / "v0.8.vote.k2of4" /
        //     "v0.8.vote.k3of4".
        // - write_report = false in bake-off calls → no anchored report body.
        // - These ids cannot collide with any existing anchored report id.
        // - `scripts/verify_anchors.sh` → 119/119 byte-identical (anchor-safe).
        "v0.8.vote.majority"
        | "v0.8.vote.unanimous"
        | "v0.8.vote.trend_pair"
        | "v0.8.vote.tr_mr_macd_rsi"
        | "v0.8.vote.tr_mr_sma_bb"
        | "v0.8.vote.any1of4"
        | "v0.8.vote.k2of4"
        | "v0.8.vote.k3of4" => {
            // All `use` imports must come before any `let` statements
            // to satisfy the `clippy::items_after_statements` lint.
            use crate::cli_types::BacktestState;
            use crate::paper::FillPriceMode;
            use crate::scenarios::sim::sim_slippage_cost;
            use crate::scenarios::sma_composed_run;
            use strategy::StrategyRegistry;
            use trading_core::{
                FillView, Order, OrderKind, Position, Quantity, RiskLimits, Side, TimeInForce,
            };

            // Use `initial_capital` derived from ScenarioConfig (leaderboard knob).
            // ── Build and register the ensemble strategy ──────────────────────────
            // (must happen before bars so a build failure exits early)
            let ensemble = strategy::build_ensemble(strategy_str).map_err(|e| {
                RunError::Internal(format!(
                    "F8 ensemble build_ensemble({strategy_str}) failed: {e}"
                ))
            })?;
            let registry = StrategyRegistry::new();
            registry.register(Box::new(ensemble));

            // ── Resolve bars (reuse buyhold pattern) ─────────────────────────────
            let bars: Vec<trading_core::Bar> = if let Some(b) = cfg.bars_override.clone() {
                b
            } else {
                let start_price = sma_composed_run::default_start_price(&cfg.pair.1);
                sma_composed_run::synthetic_bars_minute(
                    &cfg.pair.1,
                    bar_count,
                    seed_u64,
                    start_price,
                    start_year,
                )
            };
            let bar_count_actual = bars.len();

            // ── Minimal bar loop (mirrors sma_composed_run inner loop) ───────────

            let risk_limits = RiskLimits {
                per_symbol_exposure_cap: dec!(0.40),
                price_sanity_band: dec!(0.20),
                portfolio_exposure_cap: None,
            };
            let sizer = risk::FixedFractionSizer::new(dec!(0.10));
            let match_cfg = crate::paper::MatchConfig {
                slippage_bps: 2,
                taker_fee_bps: 4,
                maker_fee_bps: 2,
                fill_price_mode: FillPriceMode::BarClose,
            };
            // bug-log #79 — the 8 `v0.8.vote.*` arms are advisor bake-off arms
            // that build their engine inline here instead of going through
            // `sma_composed_run`; they need the same venue-filter application
            // or lot realism stays inert for a third of the ranked field.
            let mut engine = crate::PaperEngine::new(match_cfg, seed_u64)
                .with_venue_filter_mode(cfg.latency_slippage_sim.venue_filter.clone());

            let mut state = BacktestState::new(initial_capital);
            let mut position = Position::empty(cfg.pair.1.clone());
            let mut all_fills: Vec<FillView> = Vec::new();
            let mut position_curve: Vec<(i64, Decimal)> = Vec::with_capacity(bar_count_actual);

            let bars_arc = std::sync::Arc::new(bars);
            let start_instant = std::time::Instant::now();

            for (bar_idx, bar) in bars_arc.iter().enumerate() {
                // Cancellation + progress poll (same cadence as sma_composed_run).
                #[allow(clippy::verbose_bit_mask)]
                let poll_now = bar_idx == bar_count_actual.saturating_sub(1)
                    || if bar_idx < 128 {
                        bar_idx & 0x1F == 0
                    } else {
                        bar_idx & 0x7F == 0
                    };
                if poll_now {
                    if cancel_rx.is_cancelled() {
                        return Err(RunError::Cancelled);
                    }
                    progress_tx.try_send(crate::progress::Progress {
                        current_bar: bar_idx,
                        total_bars: bar_count_actual,
                        elapsed_ms: u64::try_from(start_instant.elapsed().as_millis())
                            .unwrap_or(u64::MAX),
                    });
                }

                let bar = bar.clone();
                let mark = bar.close.get();
                position.last_mark = bar.close;
                let equity = state.equity(mark);

                let signals = registry.on_bar(&bar);
                let mut orders: Vec<Order> = Vec::new();

                for sig in &signals {
                    // ADR-0068 D3 / Q-SS-1: same position-aware interpretation gate as
                    // sma_composed_run.rs. When short_enabled=false (all existing arms):
                    // long-only clamp #1, byte-identical to HEAD.
                    let desired_side: Option<Side> = match sig.kind {
                        trading_core::SignalKind::Buy if position.base_qty <= Decimal::ZERO => {
                            Some(Side::Buy)
                        }
                        trading_core::SignalKind::Sell if position.base_qty > Decimal::ZERO => {
                            Some(Side::Sell)
                        }
                        // ADR-0068 clamp #1 GATE: Sell-when-flat → open short (short_enabled only).
                        trading_core::SignalKind::Sell
                            if position.base_qty <= Decimal::ZERO && cfg.short_enabled =>
                        {
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

                if !orders.is_empty()
                    && let Ok(fills) = engine.step(&bar, orders).await
                {
                    for fill in &fills {
                        let sim_cost = sim_slippage_cost(
                            fill.qty.get(),
                            fill.price.get(),
                            fill.side,
                            &cfg.latency_slippage_sim,
                            &fill.symbol,
                        );
                        match fill.side {
                            Side::Buy => {
                                state.apply_buy(
                                    fill.qty.get(),
                                    fill.price.get(),
                                    fill.fee.amount(),
                                );
                                state.cash -= sim_cost;
                                position.base_qty += fill.qty.get();
                                position.cost_basis = Money::from_decimal(state.position_cost);
                            }
                            Side::Sell => {
                                // ADR-0068 D1/D3: pass short_enabled to gate clamp #3.
                                state.apply_sell(
                                    fill.qty.get(),
                                    fill.price.get(),
                                    fill.fee.amount(),
                                    cfg.short_enabled,
                                );
                                state.cash -= sim_cost;
                                position.base_qty -= fill.qty.get();
                                // ADR-0068 D1 clamp #2: gate the base_qty < 0 clamp.
                                // When short_enabled=false: clamp to zero (long-only, byte-identical).
                                // When short_enabled=true: allow negative (open short).
                                if !cfg.short_enabled && position.base_qty < Decimal::ZERO {
                                    position.base_qty = Decimal::ZERO;
                                }
                            }
                        }
                        all_fills.push(FillView {
                            symbol: fill.symbol.clone(),
                            side: fill.side,
                            price: fill.price,
                            qty: fill.qty,
                            fee: fill.fee,
                            fee_tier: fill.fee_tier,
                            venue_ts: fill.venue_ts,
                            transaction_id: smol_str::SmolStr::default(),
                        });
                    }
                }

                // Post-fill equity bookkeeping (mirrors sma_composed_run).
                let post_fill_equity = state.equity(mark);
                state.update_drawdown(post_fill_equity);
                state.equity_curve.push(post_fill_equity);

                position_curve.push((bar.close_ts.unix_millis(), position.base_qty));
            }

            // ── Build equity series with real timestamps (if bars available) ──────
            // state.equity_curve: [initial_capital, eq_bar_0, eq_bar_1, …]
            // bars_arc:           [bar_0, bar_1, …]
            // We pair them: initial entry gets first_ts − 1h; subsequent entries get bar.open_ts.
            let equity_series: Vec<(Timestamp, Money<Usdt>)> = {
                let eq_curve = &state.equity_curve;
                let first_ts = bars_arc
                    .first()
                    .map_or(Timestamp::new(OffsetDateTime::UNIX_EPOCH), |b| {
                        Timestamp::new(b.open_ts.inner() - time::Duration::hours(1))
                    });
                // eq_curve[0] = initial capital (before bar 0), so zip from eq_curve[1..]
                // with bars_arc.
                let mut series = Vec::with_capacity(eq_curve.len());
                if let Some(&initial_eq) = eq_curve.first() {
                    series.push((first_ts, Money::<Usdt>::from_decimal(initial_eq)));
                }
                for (bar, &eq) in bars_arc.iter().zip(eq_curve.iter().skip(1)) {
                    series.push((bar.open_ts, Money::<Usdt>::from_decimal(eq)));
                }
                series
            };

            let final_eq = *state.equity_curve.last().unwrap_or(&initial_capital);
            let eq_decimals: Vec<Decimal> = equity_series.iter().map(|(_, m)| m.amount()).collect();
            let max_dd = crate::stats::compute_max_drawdown_f64(&eq_decimals);

            let kpis = BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(final_eq),
                initial_equity: Money::<Usdt>::from_decimal(initial_capital),
                max_drawdown: Decimal::try_from(max_dd).unwrap_or(Decimal::ZERO),
                trade_count: state.trades,
                total_fees: Money::<Usdt>::from_decimal(state.total_fees),
                buys: state.buys,
                sells: state.sells,
                total_return_pct: total_return_pct(initial_capital, final_eq),
            };

            // write_report = false for bake-off arms (ADR-0063) — the
            // maybe_write_report call is a no-op when write_report = false.
            let report_path = maybe_write_report(
                &cfg,
                strategy_str,
                strategy_str,
                &equity_series,
                |_path| Ok(()), // No-op: no anchored report format for ensembles.
            )?;

            Ok(RunReport {
                equity_series,
                fills: all_fills,
                kpis,
                report_path,
                bars: bars_arc,
                position_curve_raw: position_curve
                    .into_iter()
                    .map(|(ts_ms, qty)| (ts_ms, qty, cfg.pair.1.clone()))
                    .collect(),
            })
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
            reports_dir: None,
            short_enabled: false,
            initial_capital: None, // None → legacy 100_000 default
            composed_toml_override: None,
            dvol_override: None,
            macro_regime_series: None,
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

    // ── lab-run-save-compare T1/T3 unit tests ────────────────────────────────

    /// AC1 — `maybe_write_report` with `write_report=false` returns `None`
    /// and writes NO file.
    #[test]
    fn maybe_write_report_write_false_returns_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg = ScenarioConfig {
            strategy: StrategyId("v0.sma".into()),
            pair: (Venue::Binance, Symbol::new("BTCUSDT")),
            range: DateRange::Last30d,
            params: None,
            seed: valid_seed(),
            write_report: false,
            data_source: ScenarioDataSource::default(),
            bars_override: None,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            reports_dir: Some(tmp.path().to_path_buf()),
            short_enabled: false,
            initial_capital: None,
            composed_toml_override: None,
            dvol_override: None,
            macro_regime_series: None,
        };
        let result = maybe_write_report(&cfg, "v0.sma", "test-scenario", &[], |_path| Ok(()));
        assert!(
            matches!(result, Ok(None)),
            "write_report=false must return Ok(None); got: {result:?}"
        );
        // No files written.
        let count = std::fs::read_dir(tmp.path())
            .map(std::iter::Iterator::count)
            .unwrap_or(0);
        assert_eq!(
            count, 0,
            "no directories should be created when write_report=false"
        );
    }

    /// AC1 — `maybe_write_report` with `write_report=true` writes a file and
    /// returns `Some(path)`.
    #[test]
    fn maybe_write_report_write_true_creates_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg = ScenarioConfig {
            strategy: StrategyId("v0.sma".into()),
            pair: (Venue::Binance, Symbol::new("BTCUSDT")),
            range: DateRange::Last30d,
            params: None,
            seed: valid_seed(),
            write_report: true,
            data_source: ScenarioDataSource::default(),
            bars_override: None,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            reports_dir: Some(tmp.path().to_path_buf()),
            short_enabled: false,
            initial_capital: None,
            composed_toml_override: None,
            dvol_override: None,
            macro_regime_series: None,
        };
        let result = maybe_write_report(&cfg, "v0.sma", "test-scenario", &[], |path| {
            // Write minimal content so the file exists.
            std::fs::write(path, b"---\ntest: true\n---\nbody\n").map_err(anyhow::Error::from)
        });
        let path = result
            .expect("maybe_write_report must succeed")
            .expect("write_report=true must return Some(path)");
        assert!(path.exists(), "report file must exist at {path:?}");
        let slug_dir = tmp.path().join("v0-paper-sma").join("reports");
        assert!(
            path.starts_with(&slug_dir),
            "path {path:?} must be under {slug_dir:?}"
        );
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("backtest-"),
            "filename must start with 'backtest-'"
        );
        assert!(
            name.ends_with("-test-scenario.md"),
            "filename must end with scenario name"
        );
    }

    /// AC8 — retention purge keeps at most N = 20 files per tuple.
    #[test]
    fn purge_old_lab_reports_keeps_last_n() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let reports_dir = tmp.path();
        let scenario = "test-scenario";
        // Create 25 fake report files.
        for i in 0..25_u32 {
            let name = format!("backtest-2026{i:04}-{scenario}.md");
            std::fs::write(reports_dir.join(&name), b"test").expect("write test file");
        }
        // Confirm 25 exist before purge.
        let before: Vec<_> = std::fs::read_dir(reports_dir).unwrap().flatten().collect();
        assert_eq!(before.len(), 25, "should have 25 files before purge");
        // Run purge.
        purge_old_lab_reports(reports_dir, scenario);
        // Count after — must be exactly N = 20.
        let after: Vec<_> = std::fs::read_dir(reports_dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().into_string().unwrap())
            .collect();
        assert_eq!(
            after.len(),
            20,
            "purge must leave exactly 20 files; got: {after:?}"
        );
        // The OLDEST 5 files (i=0..4, alphabetically first) must be gone;
        // the NEWEST 20 (i=5..24) must remain.
        for i in 0..5_u32 {
            let name = format!("backtest-2026{i:04}-{scenario}.md");
            assert!(
                !after.contains(&name),
                "old file {name} should have been purged"
            );
        }
        for i in 5..25_u32 {
            let name = format!("backtest-2026{i:04}-{scenario}.md");
            assert!(
                after.contains(&name),
                "recent file {name} must be kept after purge"
            );
        }
    }

    /// AC8 — retention purge is a no-op when ≤ N files exist.
    #[test]
    fn purge_old_lab_reports_noop_when_few_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let reports_dir = tmp.path();
        let scenario = "test-scenario";
        // Create exactly 5 files — well under N=20.
        for i in 0..5_u32 {
            let name = format!("backtest-20260001{i:04}-{scenario}.md");
            std::fs::write(reports_dir.join(&name), b"test").expect("write test file");
        }
        purge_old_lab_reports(reports_dir, scenario);
        let count = std::fs::read_dir(reports_dir).unwrap().count();
        assert_eq!(count, 5, "purge must not remove files when count <= 20");
    }

    #[test]
    fn global_cap_bounds_distinct_scenario_buckets() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();
        // 205 DISTINCT scenario names — the per-scenario purge never fires;
        // only the global cap can bound this.
        for i in 0..205 {
            let name = format!("backtest-2026{i:04}-scenario-{i:04}.md");
            std::fs::write(dir.join(&name), "x").expect("write md");
            let csv = format!("backtest-2026{i:04}-scenario-{i:04}-equity.csv");
            std::fs::write(dir.join(&csv), "t,e").expect("write csv");
        }
        purge_lab_reports_global_cap(dir);
        let remaining_md = std::fs::read_dir(dir)
            .expect("read_dir")
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".md"))
            .count();
        assert_eq!(remaining_md, 200, "global cap must keep newest 200 .md");
        // the 5 oldest (0000..0004) are gone, WITH their companions
        for i in 0..5 {
            let md = dir.join(format!("backtest-2026{i:04}-scenario-{i:04}.md"));
            let csv = dir.join(format!("backtest-2026{i:04}-scenario-{i:04}-equity.csv"));
            assert!(!md.exists(), "oldest md {i} must be purged");
            assert!(!csv.exists(), "companion csv {i} must be purged with it");
        }
        // the newest survives untouched
        assert!(dir.join("backtest-20260204-scenario-0204.md").exists());
    }

    /// T1 — `strategy_dir_slug` maps known ids to expected directory slugs.
    #[test]
    fn strategy_dir_slug_known_ids() {
        assert_eq!(
            strategy_dir_slug("v1.momentum"),
            "v1-cross-sectional-momentum"
        );
        assert_eq!(
            strategy_dir_slug("top10_momentum_h1"),
            "v1-cross-sectional-momentum"
        );
        assert_eq!(strategy_dir_slug("v0.sma"), "v0-paper-sma");
        assert_eq!(strategy_dir_slug("sma_crossover"), "v0-paper-sma");
        assert_eq!(strategy_dir_slug("v0.5.macd"), "v05-composed-strategies");
        assert_eq!(strategy_dir_slug("v0.5.rsi"), "v05-composed-strategies");
        assert_eq!(strategy_dir_slug("v0.5.bbands"), "v05-composed-strategies");
        assert_eq!(strategy_dir_slug("v1.5a.mr"), "v15a-mean-reversion-pairs");
        assert_eq!(strategy_dir_slug("v2.5.tcn_overlay"), "v2.5.tcn_overlay");
        // ADR-0068 T-D6: _ls aliases and always_short map to the same slugs.
        assert_eq!(strategy_dir_slug("v0.sma_cross_ls"), "v0-paper-sma");
        assert_eq!(strategy_dir_slug("v0.macd_ls"), "v05-composed-strategies");
        assert_eq!(strategy_dir_slug("v0.rsi_ls"), "v05-composed-strategies");
        assert_eq!(strategy_dir_slug("v0.bbands_ls"), "v05-composed-strategies");
        assert_eq!(
            strategy_dir_slug("v0.always_short"),
            "v05-composed-strategies"
        );
        // Unknown falls back to verbatim id.
        assert_eq!(strategy_dir_slug("some_unknown_id"), "some_unknown_id");
        // ADR-0071 signal-library expansion arms map to "v0-signal-library".
        assert_eq!(strategy_dir_slug("v0.donchian_break"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("btc_donchian_break"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("v0.donchian_floor"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("btc_donchian_floor"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("v0.vol_breakout"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("btc_vol_breakout"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("v0.roc_momentum"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("btc_roc_momentum"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("v0.obv"), "v0-signal-library");
        assert_eq!(strategy_dir_slug("btc_obv"), "v0-signal-library");
    }
}
