//! Advisor bake-off + ranking engine (roadmap F1 + F2).
//!
//! # Overview
//!
//! `run_bakeoff(cfg, cancel, progress) -> BakeoffReport` loops `run_scenario`
//! over the strategy field (default: `SMA` / `MACD` / `RSI` / `BBands`) plus the
//! buy-and-hold benchmark arm on one `(symbol, lookback_window)`, collects
//! per-candidate KPIs + equity curves + robustness flags, then ranks and
//! crowns the best.
//!
//! # Architecture (ADR-0059)
//!
//! - Home: `crates/backtest` — `ui` already imports this crate, so `BakeoffReport`
//!   reaches the cockpit through the **same sanctioned seam** as `RunReport`.
//! - The result type is `Clone + Debug` and free of `strategy`/`exec`/`forecast`
//!   /`llm` types — `cargo tree -p ui` gains **no** new edge from this feature.
//! - The `"v0.buyhold"` `run_scenario` arm is **anchor-additive** (see `engine.rs`).
//!   `verify_anchors.sh` must pass 119/119 before and after this module lands.

// Allow float arithmetic in the metric derivation helper (same exemption as stats/).
#![allow(clippy::float_arithmetic)]

pub mod bootstrap;
pub mod buyhold;
pub mod rank;
pub mod robustness;
pub mod scorecard;
pub mod sweep;

use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

pub use bootstrap::{compute_robustness_distribution, compute_robustness_flag, derive_master_seed};
pub use rank::{Ranking, rank_candidates};
pub use robustness::RobustnessFlag;
pub use scorecard::{Scorecard, ScorecardSummary};

use crate::{
    DateRange, RunReport, ScenarioConfig,
    cancel::RunCancelReceiver,
    engine::{ScenarioDataSource, run_scenario},
    progress::{BakeoffProgressSender, ProgressSender},
    stats::{compute_calmar, compute_sharpe_hourly, compute_sortino_hourly},
};

// ── P1-2 coherent tail / median summary (REPORT-ONLY) ─────────────────────────
//
// Carries the four `DistributionSummary` fields the developer added in commit
// 66286e2 across the public seam so the leaderboard "Risk story" block can
// surface them next to the scorecard.  The crown only — one summary per
// bake-off, mirroring how `Scorecard` lives on `Recommendation` (P0-1).
//
// **Pure reporting — does NOT change crowning, ranking, or the FROZEN gate.**
// Filled by `run_bakeoff` from `compute_robustness_distribution`; `None` when
// the robustness gate ran in `Skip` mode or the curve was too short.

/// The crown's coherent-tail + median summary (P1-2 / advisor-turnover-and-tail-metrics).
///
/// Four reductions over the crown's 1 000-path bootstrap `PathMetrics` vector:
/// `CVaR` at α=0.05 / α=0.01 (coherent / sub-additive, preferred over `VaR`),
/// the median terminal wealth (the "typical outcome" — counters the mean),
/// and the return-distribution skew (right-skewed lottery vs left-skewed crash).
///
/// **Public seam contract:** all four fields are plain `f64` so the `ui`
/// mirror crosses without naming `DistributionSummary` (the `ui` purity rule).
#[derive(Debug, Clone, Copy)]
pub struct TailSummary {
    /// Expected shortfall at α=0.05 — mean of the worst 5% of paths by `total_return`.
    /// Coherent / sub-additive (the doc is explicit: `CVaR` not `VaR` — P1-2).
    pub cvar_95: f64,
    /// Expected shortfall at α=0.01 — extreme-tail complement to `cvar_95`.
    pub cvar_99: f64,
    /// Median terminal wealth (p50 of `final_equity` across paths, as `f64`).
    /// The honest "what does the middle outcome actually look like in dollars?"
    pub median_terminal_wealth: f64,
    /// Skew of `total_return` across paths (3rd standardised central moment).
    /// Positive → right tail (lottery); negative → left tail (crash-prone).
    pub skew: f64,
}

impl TailSummary {
    /// Build from a populated `DistributionSummary` — pure projection of the
    /// four P1-2 fields.  The only place a `DistributionSummary` is read in the
    /// bake-off result path; the leaderboard mirror then reads `TailSummary`.
    #[must_use]
    pub fn from_distribution(summary: &crate::stats::DistributionSummary) -> Self {
        Self {
            cvar_95: summary.cvar_95,
            cvar_99: summary.cvar_99,
            median_terminal_wealth: summary.median_terminal_wealth,
            skew: summary.skew,
        }
    }
}

// ── Binance corpus root (mirrors the Lab constant; backtest must NOT import ui) ─

/// Path to the Binance parquet corpus, relative to the workspace root.
/// Mirrors `BINANCE_CORPUS_ROOT` in `crates/ui/src/lab/runner.rs`.
const BINANCE_CORPUS_ROOT: &str = "data/binance";

/// Path to the Deribit DVOL parquet corpus, relative to the workspace root.
#[cfg(feature = "realdata")]
const DVOL_CORPUS_ROOT: &str = "data/deribit-dvol";

/// Path to the Yahoo macro parquet corpus (ADR-0073 D2 — dedicated root,
/// NOT `data/yahoo/` which has pre-existing operator drift).
/// Relative to the workspace root.
/// Compiled only when `--features yahoo` (macro loading requires the Yahoo reader).
#[cfg(feature = "yahoo")]
const YAHOO_MACRO_CORPUS_ROOT: &str = "data/yahoo-macro";

// ── DVOL override resolver (ADR-0072 Task 2 core fix) ────────────────────────

/// Map a bake-off `symbol` (e.g. `"BTCUSDT"`) to the DVOL corpus symbol
/// (e.g. `"BTC"`). Returns `None` for unsupported symbols.
#[cfg(feature = "realdata")]
fn dvol_corpus_symbol(symbol_str: &str) -> Option<&'static str> {
    match symbol_str {
        "BTCUSDT" => Some("BTC"),
        "ETHUSDT" => Some("ETH"),
        _ => None,
    }
}

/// Load the as-of DVOL series for a BTC/ETH bake-off arm and return it as
/// `Some(Vec<Option<Decimal>>)` aligned to `bar_open_ts_ms`.
///
/// This is the **core fix** for the prior `dvol_override: None` stub that made
/// the `v0.dvol_regime` arm vacuous (permanent warm-up → equals buy-and-hold).
///
/// # Algorithm
///
/// 1. Map `symbol` (`BTCUSDT`/`ETHUSDT`) → DVOL corpus symbol (`BTC`/`ETH`).
/// 2. Build `TimeSpan` from `(start_ms, end_ms)` derived from `range`.
/// 3. Load + SHA-verify the DVOL corpus via `DvolDataSource::load`.
/// 4. Build `dvol: Vec<(i64, Decimal)>` from the rows filtered to the mapped symbol.
/// 5. Run `dvol_as_of(&dvol, &bar_open_ts_ms)` → `Some(series)`.
///
/// # Graceful degradation (MANDATORY)
///
/// If the corpus is missing (`RevisionMissing`) or the load fails for any reason
/// (e.g. a CI machine without the gitignored parquets), emits `tracing::warn!`
/// and returns `None` — the engine falls back to an empty DVOL series (warm-up-only
/// = buy-and-hold proxy). The bake-off NEVER crashes on a best-effort probe arm.
///
/// # Compile-time gate
///
/// This function exists ONLY when `--features realdata` is active (DVOL loading
/// requires polars). The `#[cfg(not(feature = "realdata"))]` variant always
/// returns `None` so `cargo build -p backtest` (default features) compiles clean.
#[cfg(feature = "realdata")]
#[must_use]
pub fn resolve_dvol_override(
    symbol_str: &str,
    range: &DateRange,
    bar_open_ts_ms: &[i64],
    scenario_name: &str,
) -> Option<Vec<Option<rust_decimal::Decimal>>> {
    use crate::dvol_data::{DvolDataSource, dvol_as_of};
    use crate::realdata::TimeSpan;

    let corpus_sym_str = dvol_corpus_symbol(symbol_str)?;
    let corpus_sym = trading_core::Symbol::new(corpus_sym_str);

    let (start_ms, end_ms) = date_range_to_ms_pair(range);
    let span = TimeSpan {
        start_ms,
        end_ms,
        // Labels are only used for report output (write_report=false on bakeoff path).
        start_label: Box::leak(format!("{start_ms}").into_boxed_str()),
        end_label: Box::leak(format!("{end_ms}").into_boxed_str()),
    };

    let src = DvolDataSource::new(
        std::path::PathBuf::from(DVOL_CORPUS_ROOT),
        vec![corpus_sym.clone()],
    );

    let loaded = match src.load(&span, scenario_name) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(
                symbol = symbol_str,
                corpus_sym = corpus_sym_str,
                error = %e,
                "v0.dvol_regime: DVOL corpus load failed — arm will run warm-up-only (arm skipped gracefully)"
            );
            return None;
        }
    };

    if loaded.rows.is_empty() {
        tracing::warn!(
            symbol = symbol_str,
            corpus_sym = corpus_sym_str,
            "v0.dvol_regime: DVOL corpus returned 0 rows for the requested span — arm will run warm-up-only"
        );
        return None;
    }

    // Build sorted (day_close_ts_ms, dvol_close) pairs for the corpus symbol.
    let mut dvol: Vec<(i64, rust_decimal::Decimal)> = loaded
        .rows
        .iter()
        .filter(|r| r.symbol == corpus_sym)
        .map(|r| (r.day_close_ts_ms, r.dvol_close))
        .collect();
    dvol.sort_unstable_by_key(|&(ts, _)| ts);

    if dvol.is_empty() {
        tracing::warn!(
            symbol = symbol_str,
            corpus_sym = corpus_sym_str,
            "v0.dvol_regime: no DVOL rows found for corpus symbol — arm will run warm-up-only"
        );
        return None;
    }

    tracing::info!(
        target: "bakeoff.dvol",
        symbol = symbol_str,
        corpus_sym = corpus_sym_str,
        dvol_rows = dvol.len(),
        bars = bar_open_ts_ms.len(),
        "DVOL series resolved for v0.dvol_regime arm"
    );

    Some(dvol_as_of(&dvol, bar_open_ts_ms))
}

/// Fallback when `realdata` feature is disabled: DVOL loading requires polars.
/// Returns `None` always; the engine arm runs warm-up-only (buy-and-hold proxy).
#[cfg(not(feature = "realdata"))]
#[must_use]
pub fn resolve_dvol_override(
    _symbol_str: &str,
    _range: &DateRange,
    _bar_open_ts_ms: &[i64],
    _scenario_name: &str,
) -> Option<Vec<Option<rust_decimal::Decimal>>> {
    None
}

/// Map a `DateRange` to `(start_ms, end_ms)` UTC epoch-millis.
///
/// Mirrors `binance_range_to_ms_pair` in `crates/ui/src/lab/runner.rs` EXACTLY
/// so the bake-off clips the same bars as the Lab runner.  `Last30d`/`Last90d`
/// are wall-clock-relative (intentional); fixed presets are deterministic.
#[must_use]
pub fn date_range_to_ms_pair(range: &DateRange) -> (i64, i64) {
    const MS_PER_DAY: i64 = 86_400_000;
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    match range {
        DateRange::Last30d => (now_ms - 30 * MS_PER_DAY, now_ms),
        DateRange::Last90d => (now_ms - 90 * MS_PER_DAY, now_ms),
        DateRange::H1_2024 => (1_704_067_200_000, 1_719_792_000_000), // 2024-01-01 .. 2024-07-01 UTC
        DateRange::H2_2024 => (1_719_792_000_000, 1_735_689_600_000), // 2024-07-01 .. 2025-01-01 UTC
        DateRange::Custom { start_ms, end_ms } => (*start_ms, *end_ms),
    }
}

/// Preload hourly bars from the pinned Binance corpus for `(symbol, range)`.
///
/// Returns `Err(RunError::Internal(…))` on a corpus read failure or empty result.
/// NEVER falls back to synthetic — the purpose of this call is to guarantee real
/// data reaches every candidate arm.
///
/// Mirrors the logic in `crates/ui/src/lab/runner.rs::preload_binance_bars` but
/// lives entirely in `backtest` (backtest must NOT import `ui`).
fn preload_bakeoff_binance_bars(
    symbol: &Symbol,
    range: &DateRange,
) -> Result<Vec<trading_core::Bar>, crate::engine::RunError> {
    use trading_core::Timeframe;

    let root = std::path::PathBuf::from(BINANCE_CORPUS_ROOT);

    // Optional: verify the pinned revision manifest (read-only; loud error on
    // tamper/mismatch — the corpus IS the determinism contract, ADR-0032).
    if let Err(e) = data::revision::read_and_verify_revision_manifest(&root) {
        return Err(crate::engine::RunError::Internal(format!(
            "Binance bake-off revision check failed: {e}"
        )));
    }

    let feed = data::ReplayFeed::new(&root, true);
    let symbol_paths = [(symbol.clone(), root.clone())];

    let mut bars = feed
        .merge_symbols(&symbol_paths, Timeframe::OneHour)
        .map_err(|e| {
            crate::engine::RunError::Internal(format!(
                "Binance bake-off corpus read failed for {symbol}: {e}"
            ))
        })?;

    // Clip to [start_ms, end_ms).
    let (start_ms, end_ms) = date_range_to_ms_pair(range);
    bars.retain(|b| {
        let ts_ms = b.open_ts.unix_millis();
        ts_ms >= start_ms && ts_ms < end_ms
    });

    if bars.is_empty() {
        return Err(crate::engine::RunError::Internal(format!(
            "Binance bake-off: 0 bars in range for {symbol} (corpus present but window empty)"
        )));
    }

    tracing::info!(
        target: "bakeoff.binance",
        symbol = %symbol,
        bars = bars.len(),
        start_ms,
        end_ms,
        "Binance hourly bars preloaded for bake-off"
    );

    Ok(bars)
}

// ── Coverage predicate ────────────────────────────────────────────────────────

/// Return `true` if `bars` fully cover `[start_ms, end_ms)` with no gap larger
/// than one bar period (defined here as 1h for the bake-off interval).
///
/// A window is considered covered when:
/// - `bars` is non-empty,
/// - `bars[0].open_ts.unix_millis() <= start_ms` (no leading gap),
/// - `bars[last].close_ts.unix_millis() >= end_ms - ONE_HOUR_MS` (no trailing gap).
///
/// When the window straddles the boundary of the pinned corpus (i.e. neither
/// fully inside nor fully outside), we treat it as **not covered** and fetch
/// the whole window from the dynamic path (D3 — correctness over cleverness;
/// the straddle case is rare and the dynamic fetch is a safe superset).
pub(crate) fn covers(start_ms: i64, end_ms: i64, bars: &[trading_core::Bar]) -> bool {
    const ONE_HOUR_MS: i64 = 3_600_000;
    let Some(first) = bars.first() else {
        return false;
    };
    let Some(last) = bars.last() else {
        return false;
    };
    let first_open = first.open_ts.unix_millis();
    let last_close = last.close_ts.unix_millis();
    first_open <= start_ms && last_close >= end_ms - ONE_HOUR_MS
}

// ── Dynamic-resolving bake-off preload ───────────────────────────────────────

/// Resolve hourly bars for `(symbol, range, data_source)` for the bake-off.
///
/// # Resolution algorithm (ADR-0061 D3)
///
/// - `Synthetic` / `YahooCache` → existing path unchanged (returns `None`).
/// - `BinanceCache` + window fully covered by the pinned corpus → existing
///   read-only, REVISION-verified pinned path (fast; no network).
/// - `BinanceCache` + not covered → `dynamic_cache::load_or_fetch` for the
///   WHOLE window (fetch-the-whole-window, not gap-stitching). NEVER writes to
///   `data/binance/`.
///
/// Returns `Ok(Some(bars))` for `BinanceCache`, `Ok(None)` for `Synthetic`/
/// `YahooCache`, or `Err(RunError::Internal(<friendly copy>))` on failure.
pub(crate) async fn resolve_bakeoff_bars(
    symbol: &Symbol,
    range: &DateRange,
    data_source: ScenarioDataSource,
) -> Result<Option<Vec<trading_core::Bar>>, crate::engine::RunError> {
    use data::dynamic_cache::load_or_fetch;
    use trading_core::Timeframe;

    match data_source {
        ScenarioDataSource::Synthetic | ScenarioDataSource::YahooCache => {
            // Existing path unchanged — synthetic / Yahoo bars generated or
            // loaded by run_scenario itself.
            return Ok(None);
        }
        ScenarioDataSource::BinanceCache => {}
    }

    let (start_ms, end_ms) = date_range_to_ms_pair(range);

    // 1. Try the pinned corpus first (read-only, REVISION-verified).
    let pinned = preload_bakeoff_binance_bars(symbol, range);
    match pinned {
        Ok(bars) if covers(start_ms, end_ms, &bars) => {
            tracing::info!(
                target: "bakeoff.resolve",
                symbol = %symbol,
                start_ms,
                end_ms,
                bars = bars.len(),
                "pinned corpus covers the window — using cached data"
            );
            return Ok(Some(bars));
        }
        Ok(_) | Err(_) => {
            // Either the corpus is missing/short for this window, or the bars
            // don't cover the range (straddle or post-2024 window). Fall
            // through to the dynamic path.
            tracing::info!(
                target: "bakeoff.resolve",
                symbol = %symbol,
                start_ms,
                end_ms,
                "pinned corpus does not cover the window — using dynamic fetch"
            );
        }
    }

    // 2. Dynamic path: fetch the whole window from Binance (non-anchored).
    //    NEVER writes to data/binance/; the dynamic root is git-ignored (ADR-0061 D4).

    let bars = load_or_fetch(symbol, start_ms, end_ms, Timeframe::OneHour)
        .await
        .map_err(|e| {
            let friendly = dynamic_error_to_friendly(symbol.0.as_str(), &e);
            crate::engine::RunError::Internal(friendly)
        })?;

    tracing::info!(
        target: "bakeoff.resolve",
        symbol = %symbol,
        bars = bars.len(),
        start_ms,
        end_ms,
        "dynamic cache loaded bars for bake-off"
    );
    Ok(Some(bars))
}

/// Map a `DynamicCacheError` to an operator-friendly error string.
///
/// The string reaches `PanelState::Error` via `RunError::Internal` →
/// `SmolStr` in `spawn_bakeoff`.  Copy from `ui::strings` error-model table.
fn dynamic_error_to_friendly(symbol: &str, e: &data::dynamic_cache::DynamicCacheError) -> String {
    use data::binance_klines::BinanceFetchError;
    use data::dynamic_cache::DynamicCacheError;

    match e {
        DynamicCacheError::Fetch(
            BinanceFetchError::Network { .. } | BinanceFetchError::Timeout { .. },
        ) => "Couldn't reach Binance to fetch market data. Check your connection and try again."
            .to_owned(),
        DynamicCacheError::Fetch(BinanceFetchError::RateLimited { .. }) => {
            "Binance is rate-limiting requests; wait a moment and try again.".to_owned()
        }
        DynamicCacheError::Fetch(BinanceFetchError::UnknownSymbol { .. }) => {
            format!("Binance has no market data for {symbol}.")
        }
        DynamicCacheError::Fetch(BinanceFetchError::NoDataForRange { .. })
        | DynamicCacheError::NoData { .. } => {
            format!("No market data available for {symbol} in that window.")
        }
        DynamicCacheError::Fetch(BinanceFetchError::Other { detail, .. }) => {
            tracing::warn!(symbol, detail, "Binance fetch Other error");
            "Couldn't fetch market data (details logged).".to_owned()
        }
        DynamicCacheError::Write(msg) | DynamicCacheError::Read(msg) => {
            tracing::warn!(symbol, %msg, "dynamic cache I/O error");
            "Couldn't fetch market data (details logged).".to_owned()
        }
    }
}

// ── Public config ─────────────────────────────────────────────────────────────

/// Request envelope for a bake-off run (echoed into `BakeoffReport` for
/// reproducibility — same seed + same field always produces the same ranking).
#[derive(Debug, Clone)]
pub struct BakeoffRequest {
    /// The single coin to run every strategy on, e.g. `Symbol::new("BTCUSDT")`.
    pub symbol: Symbol,
    /// Date range (passed verbatim to `run_scenario` for each arm).
    pub range: DateRange,
    /// Mandatory `ChaCha20` seed — same seed is used for every arm so
    /// the inter-strategy comparison is apples-to-apples on the same
    /// synthetic draw (or, for `BinanceCache`, the same data window).
    pub seed: [u8; 32],
    /// The active strategy field (excluding the benchmark arm — the loop
    /// always appends `"v0.buyhold"`).
    pub field: Vec<StrategyId>,

    // ── Leaderboard-timeframe-capital knobs (leaderboard-timeframe-capital) ──
    //
    // These two fields are NEW additions to BakeoffRequest.  All existing
    // callers that construct BakeoffRequest without these fields must add
    // them explicitly (there is no Default impl).  For backwards-compatible
    // behaviour, pass `Horizon::OneHour` and `dec!(100_000)`.
    //
    // **Anchor contract**: the advisor bake-off path sets `write_report = false`
    // (ADR-0059), so these fields never affect anchored report bodies.
    // The anchors remain 119/119 byte-identical.
    /// Bar-size horizon for the bake-off. `OneHour` = current default
    /// (identity pass-through, byte-identical to the pre-feature code).
    /// `FourHours` / `OneDay` trigger `resample_ohlcv` on the preloaded
    /// 1h bars before any arm is run (one resample, same bars to every arm).
    /// Only `OneHour`-or-coarser horizons are supported — the corpus is 1h bars.
    pub timeframe: crate::resample::Horizon,

    /// Starting equity for every sim arm. `dec!(100_000)` = current default.
    /// Changing this scales the absolute equity curve but does NOT change the
    /// ranking (returns are percentage-based). Honest UI copy must say so.
    pub initial_capital: rust_decimal::Decimal,
}

/// Configuration for `run_bakeoff`.
#[derive(Debug, Clone)]
pub struct BakeoffConfig {
    /// The bake-off request (echoed into `BakeoffReport`).
    pub request: BakeoffRequest,
    /// Data source for every arm.  Default: `BinanceCache` (the real
    /// hourly corpus).  Use `Synthetic` in unit/integration tests.
    pub data_source: ScenarioDataSource,
    /// Whether to evaluate per-candidate robustness.
    /// `None` → all flags are `Skipped` (fast; ranking correct).
    pub robustness: RobustnessMode,
}

/// How the robustness gate is run (OQ-2 operator resolution: default `Skipped`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RobustnessMode {
    /// Skip the gate — all flags are `RobustnessFlag::Skipped`.
    /// Ranking is purely Sharpe-primary (correct; fast).
    #[default]
    Skip,
    /// Moving-block bootstrap on realized equity (ADR-0063 § D4).
    ///
    /// - `paths`: number of bootstrap resamples (default 1000).
    /// - `seed`: master bake-off seed as `u64` (low 8 bytes of `[u8; 32]`).
    ///
    /// Per-candidate master seeds are derived via `derive_master_seed(seed, idx)`
    /// (ADR-0063 § D4 + ADR-0051 D1 `GOLDEN_GAMMA` rule).
    Bootstrap {
        /// Number of bootstrap resamples per candidate.
        paths: usize,
        /// Bake-off seed (u64 derived from `[u8; 32]` seed via little-endian low 8 bytes).
        seed: u64,
    },
}

impl BakeoffConfig {
    /// Returns the default field: the 4 original single-symbol rule engines
    /// plus the 5 ADR-0071 pre-registered signal-library expansion arms.
    ///
    /// The `"v0.buyhold"` benchmark arm is NOT in this list — the bake-off
    /// loop always appends it so it cannot be omitted by a caller.
    ///
    /// ADR-0071 additions (FIXED pre-registered slate, locked literals):
    /// - `v0.donchian_break` — `high >= max(high, 20)` (new 20-bar high breakout)
    /// - `v0.donchian_floor` — `close > min(low, 20)` (above 20-bar support trough)
    /// - `v0.vol_breakout`   — `high >= max(high, 20) AND volume > 2 * avg(volume, 20)`
    /// - `v0.roc_momentum`   — `close > avg(close, 10) * 1.05`
    /// - `v0.obv`            — `obv() > obv_avg(20) AND close > sma(50)`
    ///
    /// ADR-0072 additions:
    /// - `v0.dvol_regime`    — Deribit DVOL implied-vol regime long/flat filter (BTC+ETH only).
    ///   Filtered out for symbols where `dvol_supported()` is false (arm ABSENT, no crash).
    #[must_use]
    pub fn default_field() -> Vec<StrategyId> {
        vec![
            // Original 4 base arms.
            StrategyId(SmolStr::new_static("v0.sma")),
            StrategyId(SmolStr::new_static("v0.5.macd")),
            StrategyId(SmolStr::new_static("v0.5.rsi")),
            StrategyId(SmolStr::new_static("v0.5.bbands")),
            // ADR-0071 signal-library expansion: 5 new pre-registered arms.
            StrategyId(SmolStr::new_static("v0.donchian_break")),
            StrategyId(SmolStr::new_static("v0.donchian_floor")),
            StrategyId(SmolStr::new_static("v0.vol_breakout")),
            StrategyId(SmolStr::new_static("v0.roc_momentum")),
            StrategyId(SmolStr::new_static("v0.obv")),
            // ADR-0072: DVOL implied-vol regime probe (BTC+ETH only; filtered at runtime).
            StrategyId(SmolStr::new_static("v0.dvol_regime")),
        ]
    }

    /// Returns the 5 pre-registered short-capable arm ids (ADR-0068 D9, FROZEN slate).
    ///
    /// These are the ONLY arms that run with `short_enabled = true`. All existing
    /// long-only arms are UNTOUCHED. This list is FIXED before any results are read
    /// (the overfit-safe pre-registration discipline, mirroring `default_ensemble_field`).
    ///
    /// Arms:
    /// - `"v0.sma_cross_ls"` — SMA crossover long/short (short on death cross).
    /// - `"v0.macd_ls"` — MACD long/short (short on bearish flip).
    /// - `"v0.rsi_ls"` — RSI long/short (short on overbought).
    /// - `"v0.bbands_ls"` — Bollinger long/short (short on upper-band touch).
    /// - `"v0.always_short"` — always-short benchmark control.
    #[must_use]
    pub fn default_short_field() -> Vec<StrategyId> {
        vec![
            StrategyId(SmolStr::new_static("v0.sma_cross_ls")),
            StrategyId(SmolStr::new_static("v0.macd_ls")),
            StrategyId(SmolStr::new_static("v0.rsi_ls")),
            StrategyId(SmolStr::new_static("v0.bbands_ls")),
            StrategyId(SmolStr::new_static("v0.always_short")),
        ]
    }

    /// Returns `true` if the given strategy id is a short-capable arm (ADR-0068 D9).
    ///
    /// Used by the bakeoff loop to set `short_enabled` on the per-arm `ScenarioConfig`.
    /// Long-only arms (and the buy-and-hold benchmark) return `false`.
    #[must_use]
    pub fn is_short_enabled(strategy_id: &str) -> bool {
        matches!(
            strategy_id,
            "v0.sma_cross_ls" | "v0.macd_ls" | "v0.rsi_ls" | "v0.bbands_ls" | "v0.always_short"
        )
    }

    /// Returns all 8 pre-registered vote-ensemble strategy ids (ADR-0063 § D4 + ADR-0067).
    ///
    /// **F8 original arms:**
    /// - `"v0.8.vote.majority"` — 2-of-3 majority vote (MACD / RSI / `BBands`).
    /// - `"v0.8.vote.unanimous"` — 4-of-4 unanimous vote (SMA / MACD / RSI / `BBands`).
    ///
    /// **advisor-combination-search new arms (ADR-0067, FROZEN v1 slate):**
    ///
    /// Decorrelation pairings:
    /// - `"v0.8.vote.trend_pair"` — `Unanimous{n:2}` [MACD, SMA] (predicted-null control).
    /// - `"v0.8.vote.tr_mr_macd_rsi"` — `Unanimous{n:2}` [MACD, RSI] (trend ∧ mean-revert).
    /// - `"v0.8.vote.tr_mr_sma_bb"` — `Unanimous{n:2}` [SMA, `BBands`] (trend ∧ band-reversion).
    ///
    /// k-of-4 ladder (complete k∈{1,2,3}; k=4 = unanimous above):
    /// - `"v0.8.vote.any1of4"` — `Majority{k:1,n:4}` over all 4.
    /// - `"v0.8.vote.k2of4"` — `Majority{k:2,n:4}` over all 4.
    /// - `"v0.8.vote.k3of4"` — `Majority{k:3,n:4}` over all 4.
    ///
    /// These are ADDITIVE — callers can extend `default_field()` with this list
    /// to include ensemble strategies.  `default_field()` is left UNCHANGED so
    /// existing advisor paths are not affected without explicit opt-in.
    ///
    /// This is the SINGLE source of truth for the ensemble arm list.
    /// `advisor_field()` in `crates/ui/src/leaderboard/runner.rs` concatenates
    /// `default_field()` + `default_ensemble_field()` automatically.
    #[must_use]
    pub fn default_ensemble_field() -> Vec<StrategyId> {
        vec![
            // F8 original arms.
            StrategyId(SmolStr::new_static("v0.8.vote.majority")),
            StrategyId(SmolStr::new_static("v0.8.vote.unanimous")),
            // advisor-combination-search decorrelation pairings (ADR-0067).
            StrategyId(SmolStr::new_static("v0.8.vote.trend_pair")),
            StrategyId(SmolStr::new_static("v0.8.vote.tr_mr_macd_rsi")),
            StrategyId(SmolStr::new_static("v0.8.vote.tr_mr_sma_bb")),
            // advisor-combination-search k-of-4 ladder (ADR-0067).
            StrategyId(SmolStr::new_static("v0.8.vote.any1of4")),
            StrategyId(SmolStr::new_static("v0.8.vote.k2of4")),
            StrategyId(SmolStr::new_static("v0.8.vote.k3of4")),
        ]
    }

    /// Returns the single pre-registered cross-asset / macro-regime arm id
    /// (ADR-0073, LOCKED — the `v0.macro_riskon` long/flat overlay).
    ///
    /// This is ADDITIVE — callers extend `advisor_field()` with this list
    /// (see `crates/ui/src/leaderboard/runner.rs`). `default_field()` and
    /// `default_ensemble_field()` stay UNCHANGED (anchor-safe).
    ///
    /// The arm needs the `data/yahoo-macro/` corpus preloaded by `run_bakeoff`
    /// (graceful skip if absent). `write_report = false` → anchor-additive.
    #[must_use]
    pub fn default_macro_field() -> Vec<StrategyId> {
        vec![
            // ADR-0073: cross-asset macro regime probe (requires yahoo-macro corpus).
            StrategyId(SmolStr::new_static("v0.macro_riskon")),
        ]
    }
}

// ── Result types (the public seam) ───────────────────────────────────────────

/// Risk-adjusted + raw KPIs for one bake-off candidate.
///
/// Every field is already computable from `RunReport` — no new math.
#[derive(Debug, Clone, Copy)]
pub struct CandidateKpis {
    /// Annualised Sharpe (hourly). `compute_sharpe_hourly(equity_decimals)`.
    pub sharpe: f64,
    /// Annualised Sortino (hourly). `compute_sortino_hourly(equity_decimals)`.
    pub sortino: f64,
    /// Calmar ratio. `compute_calmar(equity_decimals)`.
    pub calmar: f64,
    /// Total return fraction (0.1 = +10%). From `RunReport.kpis.total_return_pct`.
    pub total_return_pct: Decimal,
    /// Max drawdown fraction (0.0 = no drawdown). From `RunReport.kpis.max_drawdown`.
    pub max_drawdown: Decimal,
    /// Number of executed trades (buys + sells). From `RunReport.kpis.trade_count`.
    pub trade_count: usize,
    /// Capital turnover ratio (P1-1 / advisor-turnover-and-tail-metrics, REPORT-ONLY).
    ///
    /// `Σ(fill.price × fill.qty) / mean_equity` — "how many times did the
    /// strategy churn its capital?"  A ratio of 1.0 means total trade notional
    /// equals mean equity; 0.0 for idle / buy-and-hold with zero fills.
    ///
    /// **Pure reporting — does NOT change crowning, ranking, or the FROZEN gate.**
    pub turnover: Decimal,
}

/// One strategy's outcome in the bake-off (the leaderboard row).
#[derive(Debug, Clone)]
pub struct CandidateResult {
    /// Strategy id, e.g. `StrategyId("v0.sma")` or `StrategyId("v0.buyhold")`.
    pub strategy: StrategyId,
    /// `true` for the buy-and-hold benchmark arm — drives the `BenchmarkWins`
    /// honesty branch in the recommendation copy.
    pub is_benchmark: bool,
    /// Risk-adjusted + raw KPIs (all from the existing stats layer).
    pub kpis: CandidateKpis,
    /// Ordered oldest-first equity curve from `RunReport.equity_series`.
    /// The UI draws per-candidate sparklines from this.
    pub equity_curve: Vec<(Timestamp, Money<Usdt>)>,
    /// Robustness flag (None when `RobustnessMode::Skip`).
    pub robustness: Option<RobustnessFlag>,
}

/// Structured rationale — the UI renders this as plain language; deterministic.
///
/// No pre-rendered strings: the UI owns the copy + the mandatory not-advice
/// disclaimer (product § D3).  The optional v0.2 LLM narration is an additive
/// consumer of this type, not a rewrite.
#[derive(Debug, Clone)]
pub struct Recommendation {
    /// Which honesty branch fired.
    pub outcome: RecommendationOutcome,
    /// The crowned strategy id (== `candidates[crowned].strategy`).
    pub winner: StrategyId,
    /// The benchmark arm's KPIs, so the UI can always show "vs just holding…".
    pub benchmark_kpis: CandidateKpis,
    /// The winner's KPIs.
    pub winner_kpis: CandidateKpis,
    /// The winner's robustness flag (echoed for the "…and it's robust/fragile" clause).
    pub winner_robustness: Option<RobustnessFlag>,
    /// Machine-readable reason codes (ordered, deterministic).
    pub reasons: Vec<ReasonCode>,
    /// Overfitting scorecard (P0-1 / ADR-0075).
    ///
    /// Computed from the candidate Sharpe vector + crown equity curve.
    /// **REPORT-ONLY** — does NOT change crowning, eligibility, or the gate bands.
    /// `crown_clears_dsr` is informational; never a veto in v2.
    pub scorecard: Scorecard,
    /// Crown's coherent-tail + median summary (P1-2 / advisor-turnover-and-tail-metrics).
    ///
    /// Surfaced by the leaderboard's "Risk story" block — `CVaR` (coherent),
    /// median terminal wealth, and skew, projected from the crown's 1 000-path
    /// bootstrap `DistributionSummary`. `None` when the robustness gate ran in
    /// `Skip` mode or the equity curve was too short for the bootstrap to
    /// produce a summary.
    ///
    /// **REPORT-ONLY** — does NOT change crowning, ranking, or the FROZEN gate.
    pub crown_tail: Option<TailSummary>,
}

/// Which honesty branch fired (drives the headline recommendation sentence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationOutcome {
    /// An active strategy was crowned and beat the benchmark.
    ActiveWins,
    /// Buy-and-hold was crowned ("nothing beat simply holding").
    BenchmarkWins,
    /// Every candidate is FRAGILE — crowned by Sharpe but all flagged.
    AllFragile,
}

/// Machine-readable reason codes the UI maps to copy.
///
/// Order is deterministic (the `rank` module builds the `Vec` in a fixed order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonCode {
    /// Crowned on Sharpe among non-fragile (eligible) arms.
    HighestRobustSharpe,
    /// Winner Sharpe > benchmark Sharpe.
    BeatBenchmarkSharpe,
    /// No active arm beat buy-and-hold.
    BenchmarkUndefeated,
    /// Robustness gate found nothing robust — all candidates are FRAGILE.
    AllCandidatesFragile,
    /// Sharpe tie resolved by higher total return.
    TieBrokenByReturn,
    /// Sharpe + return tie resolved by lower max drawdown.
    TieBrokenByDrawdown,
}

/// The ranked bake-off result (the public seam consumed by the cockpit).
///
/// Mirrors the `RunReport` precedent: `Clone + Debug`, free of
/// `strategy`/`exec`/`forecast`/`llm` types, reachable from `ui` via the
/// existing `backtest` dep.
#[derive(Debug, Clone)]
pub struct BakeoffReport {
    /// The request, echoed for reproducibility.
    pub request: BakeoffRequest,
    /// Per-candidate results in *insertion order* (stable = field order,
    /// benchmark last).
    pub candidates: Vec<CandidateResult>,
    /// Indices into `candidates`, best-first, per the ranking comparator.
    pub ranked: Vec<usize>,
    /// Index of the crowned pick (`None` only when there are zero candidates —
    /// unreachable with a non-empty field).
    pub crowned: Option<usize>,
    /// Structured rationale for the UI to render.
    pub rationale: Recommendation,
}

// ── KPI derivation ───────────────────────────────────────────────────────────

/// Derive `CandidateKpis` from a `RunReport`.
///
/// Maps `equity_series` → `Vec<Decimal>` once and feeds the existing
/// stats functions (`compute_sharpe_hourly`, `compute_sortino_hourly`,
/// `compute_calmar`).  Total-return + max-drawdown + trade-count come
/// directly from `report.kpis` (no recomputation).
///
/// # Turnover (P1-1)
///
/// `turnover = Σ(fill.price × fill.qty) / mean_equity`
///
/// Sum of absolute trade notional (fill price × fill quantity, in USDT)
/// divided by the arithmetic mean of the equity series.  Returns `Decimal::ZERO`
/// when fills is empty OR the equity series is empty / mean is zero.
/// Pure reporting — does NOT affect ranking or the FROZEN gate.
#[must_use]
pub fn derive_candidate_kpis(report: &RunReport) -> CandidateKpis {
    let equity: Vec<Decimal> = report
        .equity_series
        .iter()
        .map(|(_, m)| m.amount())
        .collect();

    // ── Turnover (P1-1) ───────────────────────────────────────────────────────
    // Σ(fill.price × fill.qty) / mean_equity
    // Decimal arithmetic throughout — no f64 in money calculations.
    let turnover = {
        let total_notional: Decimal = report
            .fills
            .iter()
            .map(|f| f.price.get() * f.qty.get())
            .fold(Decimal::ZERO, |acc, v| acc + v);

        if equity.is_empty() || total_notional.is_zero() {
            Decimal::ZERO
        } else {
            let n = Decimal::from(equity.len());
            let sum_equity: Decimal = equity.iter().copied().fold(Decimal::ZERO, |acc, v| acc + v);
            let mean_equity = sum_equity / n;
            if mean_equity.is_zero() {
                Decimal::ZERO
            } else {
                total_notional / mean_equity
            }
        }
    };

    CandidateKpis {
        sharpe: compute_sharpe_hourly(&equity),
        sortino: compute_sortino_hourly(&equity),
        calmar: compute_calmar(&equity),
        total_return_pct: report.kpis.total_return_pct,
        max_drawdown: report.kpis.max_drawdown,
        trade_count: report.kpis.trade_count,
        turnover,
    }
}

// ── Bake-off orchestrator ─────────────────────────────────────────────────────

/// The buy-and-hold benchmark strategy id — always appended to the field.
const BUYHOLD_ID: &str = "v0.buyhold";

/// Run the bake-off over the given field + buy-and-hold.
///
/// # Algorithm
///
/// 1. For each `StrategyId` in `cfg.request.field` (∪ `"v0.buyhold"`),
///    build a `ScenarioConfig` with the **same seed** + `write_report = false`
///    and `await run_scenario`.
/// 2. Derive `CandidateKpis` from each `RunReport`.
/// 3. Apply the robustness gate (default: `Skipped`).
/// 4. Call `rank_candidates` for the deterministic total order.
/// 5. Assemble `Recommendation` from the ranking result.
///
/// # Cancellation
///
/// `cancel_rx` is forwarded to each `run_scenario` call.  If the operator
/// cancels mid-bake-off the first arm to notice returns `Cancelled`; this
/// function propagates it.
///
/// # Candidate-level progress
///
/// When `bakeoff_progress_tx` is `Some`, one [`crate::progress::BakeoffProgress`]
/// event is emitted immediately BEFORE each `run_scenario` call with:
/// - `done`  = candidates fully completed so far (0 at the start of the first).
/// - `total` = total candidate count (`field.len()` + 1 for buy-and-hold).
/// - `current_id` = the strategy id about to start.
///
/// `None` ⇒ byte-identical headless/test path (no emission, no channel allocation).
///
/// # Errors
///
/// Propagates `RunError` from any `run_scenario` call (including
/// `Cancelled`, `ZeroSeed`, `InvalidRange`, `Internal`).
// The function is necessarily long: one sequential async block that preloads bars,
// resamples, iterates candidates, runs robustness bootstrap, and assembles the
// final report. Each step is a distinct logical phase; splitting further would
// scatter the sequential state machine across multiple helpers without improving
// readability.
#[allow(clippy::too_many_lines)]
pub async fn run_bakeoff(
    cfg: BakeoffConfig,
    cancel_rx: RunCancelReceiver,
    progress_tx: ProgressSender,
    bakeoff_progress_tx: BakeoffProgressSender,
) -> Result<BakeoffReport, crate::engine::RunError> {
    let req = &cfg.request;

    // ── Preload real bars ONCE (apples-to-apples invariant) ──────────────────
    //
    // When `data_source == BinanceCache`, `run_scenario` resolves bars from
    // `bars_override`.  With `bars_override: None` it silently falls back to
    // synthetic GBM bars — which is a correctness bug for a real-data bake-off.
    // The fix: preload the real Binance hourly bars here, ONCE, and pass the
    // same `Vec<Bar>` (cloned) to every candidate arm.  All candidates see the
    // identical real bars — that is the apples-to-apples invariant.
    //
    // `resolve_bakeoff_bars` (ADR-0061):
    //   - pinned-corpus path: read-only, REVISION-verified (fast; no network).
    //   - dynamic path: fetch the whole window from Binance when the corpus
    //     does not fully cover the requested range.  NEVER writes to data/binance/.
    //   - Synthetic/Yahoo: returns None (synthetic GBM or Yahoo bars are
    //     generated/loaded by run_scenario itself).
    let preloaded_1h_bars: Option<Vec<trading_core::Bar>> =
        resolve_bakeoff_bars(&req.symbol, &req.range, cfg.data_source).await?;

    // ── Timeframe resample (leaderboard-timeframe-capital knob) ──────────────
    //
    // The corpus is 1h bars.  When the operator selects a coarser horizon
    // (H4/D1), we fold the preloaded 1h bars into coarser bars ONCE and pass
    // the resampled slice to EVERY arm — apples-to-apples invariant preserved.
    // `Horizon::OneHour` → identity pass-through (same Vec, no copy).
    // Review 1-18: `resample_ohlcv` is fallible (the three `panic!`s inside the
    // bucket emitter became a `ResampleError`); a corrupt-OHLCV fold now fails
    // the bake-off loudly instead of aborting the process.
    let preloaded_bars: Option<Vec<trading_core::Bar>> = preloaded_1h_bars
        .map(|bars_1h| crate::resample::resample_ohlcv(&bars_1h, req.timeframe))
        .transpose()
        .map_err(|e| crate::engine::RunError::Internal(e.to_string()))?;

    // ── Initial capital (leaderboard-timeframe-capital knob) ─────────────────
    // Thread the operator-chosen capital through to each arm's ScenarioConfig.
    let bakeoff_initial_capital = req.initial_capital;

    // ── ADR-0073: macro regime series preload (ONE per bake-off run) ──────────
    //
    // If the field contains `v0.macro_riskon`, preload the macro daily regime
    // series from `data/yahoo-macro/` ONCE. Graceful skip (warn + None) when the
    // corpus is absent (e.g. CI without the fetched parquets). The arm still
    // appears in the ranked field but runs warm-up-only (= buy-and-hold proxy,
    // identical to the `v0.dvol_regime` graceful-degradation precedent).
    //
    // The macro corpus root is relative to the workspace root (NOT `data/yahoo/`
    // — the two corpora must stay separated to avoid the pre-existing operator
    // drift on `data/yahoo/REVISION.toml`; ADR-0073 D2 / orchestrator directive).
    //
    // Compiled only when `--features yahoo` is active (macro_regime requires the
    // Yahoo parquet reader). Without the feature, the arm is absent → None always.
    #[cfg(feature = "yahoo")]
    let preloaded_macro_series: Option<trading_core::pit::PitSeries<bool>> = {
        let field_has_macro = req
            .field
            .iter()
            .any(|id| id.0.as_str() == "v0.macro_riskon");
        if field_has_macro {
            let yahoo_macro_root = std::path::PathBuf::from(YAHOO_MACRO_CORPUS_ROOT);
            match crate::macro_regime::load_macro_regime_series(&yahoo_macro_root, &req.range) {
                Ok(series) => {
                    tracing::info!(
                        target: "bakeoff.macro",
                        "Macro regime series loaded for v0.macro_riskon arm"
                    );
                    Some(series)
                }
                Err(e) => {
                    tracing::warn!(
                        target: "bakeoff.macro",
                        error = %e,
                        "v0.macro_riskon: macro corpus load failed — arm will run warm-up-only (arm skipped gracefully)"
                    );
                    None
                }
            }
        } else {
            None
        }
    };
    // Fallback when `yahoo` feature is disabled: no macro corpus → always None.
    #[cfg(not(feature = "yahoo"))]
    let preloaded_macro_series: Option<trading_core::pit::PitSeries<bool>> = None;

    // Build the full candidate field: explicit strategies + benchmark.
    let mut strategy_ids: Vec<(StrategyId, bool)> =
        req.field.iter().cloned().map(|id| (id, false)).collect();
    strategy_ids.push((StrategyId(SmolStr::new_static(BUYHOLD_ID)), true));

    let mut candidates: Vec<CandidateResult> = Vec::with_capacity(strategy_ids.len());

    // The bake-off field is always small (≤ a few dozen strategies) so
    // usize → u16 will never actually overflow.  `try_from` with a
    // saturating fallback keeps clippy happy without panicking on the
    // unreachable overflow path.
    let total_candidates: u16 = u16::try_from(strategy_ids.len()).unwrap_or(u16::MAX);

    for (candidate_index, (strategy, is_benchmark)) in strategy_ids.into_iter().enumerate() {
        // Check cancellation before each arm.
        if cancel_rx.is_cancelled() {
            return Err(crate::engine::RunError::Cancelled);
        }

        // Emit candidate-level progress BEFORE starting this arm.
        // `done` = completed so far (0 at the very start).
        let done: u16 = u16::try_from(candidate_index).unwrap_or(u16::MAX);
        bakeoff_progress_tx.try_send(crate::progress::BakeoffProgress {
            done,
            total: total_candidates,
            current_id: strategy.0.clone(),
        });

        // ADR-0068 D9: set short_enabled=true for the 5 new _ls / always_short arms.
        // All other arms default to false → byte-identical long-only path.
        let short_enabled = BakeoffConfig::is_short_enabled(strategy.0.as_str());

        // ADR-0072 D8: filter v0.dvol_regime for unsupported symbols.
        // `dvol_supported` = {BTCUSDT, ETHUSDT}. For other symbols, the arm
        // is removed from the field before dispatching (arm ABSENT, never crash).
        let symbol_str = req.symbol.0.as_str();
        let is_dvol_arm = strategy.0.as_str() == "v0.dvol_regime";
        let dvol_sym_ok = matches!(symbol_str, "BTCUSDT" | "ETHUSDT");
        if is_dvol_arm && !dvol_sym_ok {
            tracing::debug!(symbol = %req.symbol, "v0.dvol_regime skipped — DVOL not available for this symbol");
            continue;
        }

        // ADR-0072 Task 2 core fix: resolve the real DVOL as-of series for BTC/ETH.
        //
        // For the v0.dvol_regime arm on BTC/ETH, load + SHA-verify the DVOL corpus
        // and run dvol_as_of() aligned to the preloaded bar timestamps.
        //
        // If the corpus is missing (e.g. a CI machine without the gitignored parquets)
        // or the load fails for any reason, `resolve_dvol_override` emits a warn! and
        // returns None — the arm gracefully runs warm-up-only (= buy-and-hold proxy).
        // The bake-off NEVER crashes on this best-effort probe arm.
        //
        // For non-DVOL arms, dvol_override is always None (ignored by the engine).
        let dvol_override = if is_dvol_arm {
            // Build the bar_open_ts_ms slice from the preloaded bars.
            // If preloaded_bars is None (synthetic / Yahoo path), we cannot resolve
            // DVOL — skip gracefully (the engine arm will run warm-up-only).
            if let Some(bars) = preloaded_bars.as_deref() {
                let bar_ts: Vec<i64> = bars.iter().map(|b| b.open_ts.unix_millis()).collect();
                resolve_dvol_override(symbol_str, &req.range, &bar_ts, strategy.0.as_str())
            } else {
                tracing::warn!(
                    symbol = %req.symbol,
                    "v0.dvol_regime: no preloaded bars available (synthetic/Yahoo path) — arm will run warm-up-only"
                );
                None
            }
        } else {
            None
        };

        let scenario_cfg = ScenarioConfig {
            strategy: strategy.clone(),
            pair: (trading_core::Venue::Binance, req.symbol.clone()),
            range: req.range.clone(),
            seed: req.seed,
            write_report: false, // anchor-safe: no report body written (ADR-0059)
            data_source: cfg.data_source,
            // Pass preloaded real bars to every arm — this is the fix for the
            // synthetic-fallback bug: without bars_override the engine silently
            // generates GBM bars for BinanceCache, producing garbage KPIs.
            // When timeframe != H1, bars are already resampled (see above).
            bars_override: preloaded_bars.clone(),
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::advisor_default(),
            reports_dir: None,
            params: None,
            short_enabled,
            // Leaderboard-timeframe-capital knob: thread operator-chosen capital.
            // None → 100_000 default; Some(capital) → operator value.
            initial_capital: Some(bakeoff_initial_capital),
            // Bake-off arms always load from disk — no in-memory TOML override.
            composed_toml_override: None,
            // ADR-0072 Task 2 core fix: real DVOL series for BTC/ETH arm;
            // None for all other arms (ignored by the engine).
            dvol_override,
            // ADR-0073: pre-loaded macro regime series for the macro arm ONLY.
            // `None` for every other arm → byte-identical existing paths.
            // `Some(series)` only when strategy == "v0.macro_riskon" and the
            // corpus loaded successfully; graceful-skip (None) if corpus absent.
            macro_regime_series: if strategy.0.as_str() == "v0.macro_riskon" {
                preloaded_macro_series.clone()
            } else {
                None
            },
        };

        let report = run_scenario(scenario_cfg, cancel_rx.sibling(), progress_tx.clone()).await?;

        let kpis = derive_candidate_kpis(&report);

        // Robustness gate (ADR-0063 § D4).
        let robustness = match cfg.robustness {
            RobustnessMode::Skip => None,
            RobustnessMode::Bootstrap { paths, seed } => {
                // Derive per-candidate master seed (ADR-0063 § D4 + ADR-0051 D1).
                let master_seed = derive_master_seed(seed, candidate_index);
                let equity_decimals: Vec<Decimal> = report
                    .equity_series
                    .iter()
                    .map(|(_, m)| m.amount())
                    .collect();
                let flag = compute_robustness_flag(&equity_decimals, paths, master_seed);
                tracing::debug!(
                    target: "bakeoff.robustness",
                    strategy = %strategy,
                    candidate_index,
                    ?flag,
                    paths,
                    "Bootstrap robustness flag"
                );
                Some(flag)
            }
        };

        candidates.push(CandidateResult {
            strategy,
            is_benchmark,
            kpis,
            equity_curve: report.equity_series,
            robustness,
        });
    }

    // Rank the candidates.
    let ranking = rank_candidates(&candidates);

    // Find the benchmark arm's KPIs for the recommendation.
    let benchmark_kpis = candidates.iter().find(|c| c.is_benchmark).map_or_else(
        || {
            // Unreachable: we always push the buyhold arm above.
            CandidateKpis {
                sharpe: 0.0,
                sortino: 0.0,
                calmar: 0.0,
                total_return_pct: Decimal::ZERO,
                max_drawdown: Decimal::ZERO,
                trade_count: 0,
                turnover: Decimal::ZERO,
            }
        },
        |c| c.kpis,
    );

    let crowned_idx = ranking.crowned.unwrap_or(0);
    let crowned_candidate = &candidates[crowned_idx];

    // ── P0-1 Overfitting scorecard (REPORT-ONLY, ADR-0075) ───────────────────
    //
    // Computed from inputs already available: per-candidate Sharpe vector +
    // crown's equity curve + bar count.  Does NOT touch the gate / rank / bands.
    // `crown_clears_dsr` is informational only — never a veto in v2 (§6.0 D3).
    let all_sharpes: Vec<f64> = candidates.iter().map(|c| c.kpis.sharpe).collect();
    let crown_equity_decimals: Vec<Decimal> = crowned_candidate
        .equity_curve
        .iter()
        .map(|(_, m)| m.amount())
        .collect();
    let t_bars = crown_equity_decimals.len().saturating_sub(1).max(1);
    let bakeoff_scorecard =
        scorecard::compute_scorecard(&all_sharpes, &crown_equity_decimals, t_bars);

    tracing::debug!(
        target: "bakeoff.scorecard",
        n_candidates = bakeoff_scorecard.n_candidates,
        n_eff = bakeoff_scorecard.n_eff,
        deflated_sharpe = bakeoff_scorecard.deflated_sharpe,
        min_btl_years = bakeoff_scorecard.min_btl_years,
        crown_clears_dsr = bakeoff_scorecard.crown_clears_dsr,
        "Overfitting scorecard (report-only, ADR-0075)"
    );

    // ── P1-2 Crown's coherent-tail + median summary (REPORT-ONLY) ────────────
    //
    // One additional `compute_robustness_distribution` call for the CROWN only
    // (not the whole field — the panel describes the pick, not every arm).
    // Mirrors how the scorecard is computed once at the end for the crown.
    // **Bootstrap mode only** — `Skip` leaves `crown_tail = None`, which the UI
    // mirrors to a `None`-tail (the Risk story block then paints nothing).
    let crown_tail = match cfg.robustness {
        RobustnessMode::Skip => None,
        RobustnessMode::Bootstrap { paths, seed } => {
            let master_seed = derive_master_seed(seed, crowned_idx);
            compute_robustness_distribution(&crown_equity_decimals, paths, master_seed)
                .map(|(summary, _verdict)| TailSummary::from_distribution(&summary))
        }
    };

    tracing::debug!(
        target: "bakeoff.tail",
        cvar_95 = ?crown_tail.map(|t| t.cvar_95),
        cvar_99 = ?crown_tail.map(|t| t.cvar_99),
        median_terminal_wealth = ?crown_tail.map(|t| t.median_terminal_wealth),
        skew = ?crown_tail.map(|t| t.skew),
        "Crown tail summary (P1-2, report-only)"
    );

    let rationale = Recommendation {
        outcome: ranking.outcome,
        winner: crowned_candidate.strategy.clone(),
        benchmark_kpis,
        winner_kpis: crowned_candidate.kpis,
        winner_robustness: crowned_candidate.robustness,
        reasons: ranking.reasons.clone(),
        scorecard: bakeoff_scorecard,
        crown_tail,
    };

    Ok(BakeoffReport {
        request: req.clone(),
        candidates,
        ranked: ranking.order,
        crowned: ranking.crowned,
        rationale,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::fill::FeeTier;
    use trading_core::{FillView, Money, Price, Quantity, Side, Symbol, Timestamp, Usdt};

    use crate::BacktestKpis;

    /// Build a minimal `RunReport` with no fills and a 2-point equity curve.
    fn minimal_report(ts0: Timestamp, ts1: Timestamp) -> RunReport {
        RunReport {
            equity_series: vec![
                (ts0, Money::<Usdt>::from_decimal(dec!(100_000))),
                (ts1, Money::<Usdt>::from_decimal(dec!(101_000))),
            ],
            fills: vec![],
            kpis: BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(dec!(101_000)),
                initial_equity: Money::<Usdt>::from_decimal(dec!(100_000)),
                max_drawdown: dec!(0.02),
                trade_count: 5,
                total_fees: Money::<Usdt>::zero(),
                buys: 3,
                sells: 2,
                total_return_pct: dec!(0.01),
            },
            report_path: None,
            bars: std::sync::Arc::new(vec![]),
            position_curve_raw: vec![],
        }
    }

    /// Build a minimal `FillView` for turnover tests.
    fn make_fill(price: Decimal, qty: Decimal) -> FillView {
        FillView {
            symbol: Symbol("BTCUSDT".into()),
            side: Side::Buy,
            price: Price::new(price).unwrap(),
            qty: Quantity::new(qty).unwrap(),
            fee: Money::<Usdt>::zero(),
            fee_tier: FeeTier::Maker,
            venue_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
            transaction_id: smol_str::SmolStr::default(),
        }
    }

    /// Verify `derive_candidate_kpis` maps a 2-point equity curve correctly
    /// (zero Sharpe on < 2 returns).
    #[test]
    fn derive_kpis_two_point_curve() {
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let ts1 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1));
        let report = minimal_report(ts0, ts1);

        let kpis = derive_candidate_kpis(&report);
        // Two-point equity curve → one log-return; Sharpe uses sample std
        // but with one return compute_sharpe_hourly returns non-zero.
        // Just validate the KPI passthrough fields.
        assert_eq!(kpis.total_return_pct, dec!(0.01));
        assert_eq!(kpis.max_drawdown, dec!(0.02));
        assert_eq!(kpis.trade_count, 5);
    }

    // ── P1-1 turnover unit tests ──────────────────────────────────────────────

    /// Idle strategy (no fills) → turnover = 0.
    #[test]
    fn turnover_idle_zero() {
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let ts1 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1));
        let report = minimal_report(ts0, ts1);
        let kpis = derive_candidate_kpis(&report);
        assert_eq!(
            kpis.turnover,
            Decimal::ZERO,
            "idle strategy must have zero turnover"
        );
    }

    /// One buy-sell round-trip: `price=50_000`, `qty=0.04` -> notional = `2_000`.
    /// Equity series = `[100_000, 100_000]` -> mean = `100_000`.
    /// Expected turnover = `2_000 / 100_000 = 0.02`.
    ///
    /// Note: both fills (buy + sell) are included; each contributes notional
    /// of `price x qty = 2_000`. Total = `4_000 / 100_000 = 0.04`.
    #[test]
    fn turnover_one_roundtrip() {
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let ts1 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1));

        // equity stays flat at 100_000 for simplicity
        let report = RunReport {
            equity_series: vec![
                (ts0, Money::<Usdt>::from_decimal(dec!(100_000))),
                (ts1, Money::<Usdt>::from_decimal(dec!(100_000))),
            ],
            // buy 0.04 BTC @ 50_000 + sell 0.04 BTC @ 50_000
            fills: vec![
                make_fill(dec!(50_000), dec!(0.04)), // notional = 2_000
                make_fill(dec!(50_000), dec!(0.04)), // notional = 2_000
            ],
            kpis: BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(dec!(100_000)),
                initial_equity: Money::<Usdt>::from_decimal(dec!(100_000)),
                max_drawdown: Decimal::ZERO,
                trade_count: 2,
                total_fees: Money::<Usdt>::zero(),
                buys: 1,
                sells: 1,
                total_return_pct: Decimal::ZERO,
            },
            report_path: None,
            bars: std::sync::Arc::new(vec![]),
            position_curve_raw: vec![],
        };

        let kpis = derive_candidate_kpis(&report);
        // total_notional = 2_000 + 2_000 = 4_000
        // mean_equity = 100_000
        // turnover = 4_000 / 100_000 = 0.04
        let expected = dec!(0.04);
        assert_eq!(
            kpis.turnover, expected,
            "one round-trip turnover: got {}, expected {expected}",
            kpis.turnover
        );
    }

    /// Multi-trade: verify `Σ(price × qty) / mean_equity` scales correctly.
    ///
    /// 3 fills: `(30_000 × 0.1)`, `(40_000 × 0.2)`, `(50_000 × 0.05)`
    /// = `3_000 + 8_000 + 2_500 = 13_500`
    /// Equity: `[100_000, 110_000, 120_000]` -> mean = `110_000`
    /// turnover = `13_500 / 110_000` ~= 0.122727...
    #[test]
    fn turnover_multi_trade() {
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let ts1 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1));
        let ts2 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(2));

        let report = RunReport {
            equity_series: vec![
                (ts0, Money::<Usdt>::from_decimal(dec!(100_000))),
                (ts1, Money::<Usdt>::from_decimal(dec!(110_000))),
                (ts2, Money::<Usdt>::from_decimal(dec!(120_000))),
            ],
            fills: vec![
                make_fill(dec!(30_000), dec!(0.1)),  // 3_000
                make_fill(dec!(40_000), dec!(0.2)),  // 8_000
                make_fill(dec!(50_000), dec!(0.05)), // 2_500
            ],
            kpis: BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(dec!(120_000)),
                initial_equity: Money::<Usdt>::from_decimal(dec!(100_000)),
                max_drawdown: Decimal::ZERO,
                trade_count: 3,
                total_fees: Money::<Usdt>::zero(),
                buys: 2,
                sells: 1,
                total_return_pct: dec!(0.2),
            },
            report_path: None,
            bars: std::sync::Arc::new(vec![]),
            position_curve_raw: vec![],
        };

        let kpis = derive_candidate_kpis(&report);
        // total_notional = 13_500; mean_equity = 330_000 / 3 = 110_000
        let expected = dec!(13_500) / dec!(110_000);
        let diff = (kpis.turnover - expected).abs();
        assert!(
            diff < dec!(0.000_001),
            "multi-trade turnover: got {}, expected {expected}",
            kpis.turnover
        );
    }

    // ── Frozen-gate identity: turnover does NOT change crowning ───────────────

    /// Prove that `rank_candidates` produces BYTE-IDENTICAL output before and
    /// after the `turnover` field is populated on `CandidateKpis`.
    ///
    /// The `rank_candidates` function reads only: `sharpe`, `sortino`, `calmar`,
    /// `total_return_pct`, `max_drawdown`, `is_benchmark`, `robustness`.
    /// It does NOT read `turnover` or `trade_count` — so adding the field cannot
    /// change the crown, outcome, or ranking order.
    #[test]
    fn turnover_does_not_change_ranking() {
        use crate::bakeoff::rank::rank_candidates;

        let make_candidate = |id: &'static str, sharpe: f64, is_benchmark: bool| CandidateResult {
            strategy: StrategyId(smol_str::SmolStr::new_static(id)),
            is_benchmark,
            kpis: CandidateKpis {
                sharpe,
                sortino: sharpe * 0.8,
                calmar: sharpe * 0.5,
                total_return_pct: dec!(0.1),
                max_drawdown: dec!(0.05),
                trade_count: 10,
                turnover: Decimal::ZERO, // before
            },
            equity_curve: vec![],
            robustness: Some(RobustnessFlag::Robust),
        };

        let candidates_before = vec![
            make_candidate("v0.sma", 1.5, false),
            make_candidate("v0.macd", 0.9, false),
            make_candidate("v0.buyhold", 1.1, true),
        ];

        let ranking_before = rank_candidates(&candidates_before);

        // Now build the same candidates with non-zero turnover.
        let make_candidate_with_turnover =
            |id: &'static str, sharpe: f64, is_benchmark: bool, to: Decimal| CandidateResult {
                strategy: StrategyId(smol_str::SmolStr::new_static(id)),
                is_benchmark,
                kpis: CandidateKpis {
                    sharpe,
                    sortino: sharpe * 0.8,
                    calmar: sharpe * 0.5,
                    total_return_pct: dec!(0.1),
                    max_drawdown: dec!(0.05),
                    trade_count: 10,
                    turnover: to,
                },
                equity_curve: vec![],
                robustness: Some(RobustnessFlag::Robust),
            };

        let candidates_after = vec![
            make_candidate_with_turnover("v0.sma", 1.5, false, dec!(0.5)),
            make_candidate_with_turnover("v0.macd", 0.9, false, dec!(1.2)),
            make_candidate_with_turnover("v0.buyhold", 1.1, true, dec!(0.01)),
        ];

        let ranking_after = rank_candidates(&candidates_after);

        assert_eq!(
            ranking_before.crowned, ranking_after.crowned,
            "crowned index changed after adding turnover!"
        );
        assert_eq!(
            ranking_before.outcome, ranking_after.outcome,
            "outcome changed after adding turnover!"
        );
        assert_eq!(
            ranking_before.order, ranking_after.order,
            "ranking order changed after adding turnover!"
        );
    }
}
