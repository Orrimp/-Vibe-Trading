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

use async_trait::async_trait;
use rust_decimal::Decimal;
use thiserror::Error;
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

    /// Catch-all for internal errors.
    #[error("internal backtest error: {0}")]
    Internal(String),
}

// ── `run_scenario` implementation ────────────────────────────────────────────

/// Run a backtest for the given `ScenarioConfig` and return an
/// in-memory `RunReport` (ADR-0030 / T-D-12).
///
/// # Phase A stub
///
/// The full implementation is gated behind Phase B (when all scenario
/// types are extractable from `main.rs` into reusable helpers). At
/// Phase A the function validates the seed and range, then returns
/// `Err(RunError::NotImplemented)`. The `lab::runner::spawn_lab_run`
/// in `crates/ui` catches this and resolves with a placeholder summary
/// so the cockpit smoke test observes a completed run without hanging.
///
/// # Errors
///
/// - `RunError::ZeroSeed` if `cfg.seed == [0u8; 32]`.
/// - `RunError::InvalidRange` if `DateRange::Custom { start_ms, end_ms }` has `start_ms > end_ms`.
/// - `RunError::NotImplemented` for all non-error inputs (Phase A).
pub async fn run_scenario(cfg: ScenarioConfig) -> Result<RunReport, RunError> {
    // Seed gate — mandatory per ADR-0030 determinism contract.
    if cfg.seed == [0u8; 32] {
        return Err(RunError::ZeroSeed);
    }

    // Range sanity check — catch obvious operator errors early.
    if let DateRange::Custom { start_ms, end_ms } = cfg.range
        && start_ms > end_ms
    {
        return Err(RunError::InvalidRange(format!(
            "Custom range start_ms ({start_ms}) > end_ms ({end_ms})"
        )));
    }

    // Phase A stub — the lab runner catches this and resolves with a
    // placeholder summary so the cockpit smoke test completes without
    // the full Phase B engine wiring. Phase B replaces this with the
    // real scenario dispatch.
    Err(RunError::NotImplemented)
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
        }
    }

    /// T-D-12 — zero-seed rejection per ADR-0030 determinism contract.
    #[tokio::test]
    async fn run_scenario_rejects_zero_seed() {
        let cfg = config_with_seed([0u8; 32]);
        let result = run_scenario(cfg).await;
        assert!(
            matches!(result, Err(RunError::ZeroSeed)),
            "zero seed must be rejected; got: {result:?}"
        );
    }

    /// T-D-12 — non-zero seed passes seed validation (Phase A returns
    /// `NotImplemented` as the next error level, not `ZeroSeed`).
    #[tokio::test]
    async fn run_scenario_accepts_non_zero_seed() {
        let cfg = config_with_seed(valid_seed());
        let result = run_scenario(cfg).await;
        // Phase A stub returns NotImplemented, NOT ZeroSeed.
        assert!(
            matches!(result, Err(RunError::NotImplemented)),
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
        let result = run_scenario(cfg).await;
        assert!(
            matches!(result, Err(RunError::InvalidRange(_))),
            "start > end must be rejected; got: {result:?}"
        );
    }

    /// T-D-12 — Valid Custom range passes range validation.
    #[tokio::test]
    async fn run_scenario_accepts_valid_custom_range() {
        let mut cfg = config_with_seed(valid_seed());
        cfg.range = DateRange::Custom {
            start_ms: 1_000_000,
            end_ms: 2_000_000,
        };
        let result = run_scenario(cfg).await;
        assert!(
            matches!(result, Err(RunError::NotImplemented)),
            "valid custom range must not be rejected; got: {result:?}"
        );
    }

    /// T-D-12 — All preset `DateRange` variants are handled (do not hit custom
    /// range validation path). Phase A returns `NotImplemented` for all.
    #[tokio::test]
    async fn run_scenario_all_presets_reach_stub() {
        for range in [
            DateRange::Last30d,
            DateRange::Last90d,
            DateRange::H1_2024,
            DateRange::H2_2024,
        ] {
            let mut cfg = config_with_seed(valid_seed());
            cfg.range = range.clone();
            let result = run_scenario(cfg).await;
            assert!(
                matches!(result, Err(RunError::NotImplemented)),
                "preset {range:?} must reach the stub (NotImplemented); got: {result:?}"
            );
        }
    }

    /// T-D-12 — `RunError::ZeroSeed` has a non-empty Display message.
    #[test]
    fn run_error_display_non_empty() {
        assert!(!RunError::ZeroSeed.to_string().is_empty());
        assert!(!RunError::NotImplemented.to_string().is_empty());
    }
}
