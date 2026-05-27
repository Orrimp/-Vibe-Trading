//! Shared CLI-scenario types used by `main.rs` and the extracted scenario
//! modules. These types are **not** part of the public `backtest` library API
//! (they live in the binary path), but are shared across `main.rs` and the
//! `scenarios::*` / `report::*` modules via `pub` visibility.
//!
//! Defined here so the scenario modules can take typed inputs without
//! depending on `main.rs`'s internal `Scenario` struct.

use std::path::PathBuf;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use trading_core::Symbol;

// ── LatencySlippageSimConfig (v5-latency-slippage-sim R1 / ADR-0043 § D1) ─────

/// Configuration for deterministic latency and slippage simulation in backtest.
///
/// **Default is noop**: all fields are zero so no latency or slippage is
/// applied unless the operator explicitly sets non-zero values. The default
/// config produces byte-identical output to the pre-feature code, preserving
/// all 34 SHA-256 anchors in `spec/anchors.toml` (R-NR.1 / ADR-0043 § D1).
///
/// ## Latency model (Q1 = uniform jitter / ADR-0043 § D2)
///
/// When `latency_ms_min == latency_ms_max == 0`: timestamp unchanged (noop).
/// When `latency_ms_min == latency_ms_max > 0`: fixed delay.
/// When `latency_ms_min < latency_ms_max`: uniform sample from `[min, max]`.
///
/// The RNG is a seeded `ChaCha20` sub-stream keyed on `(scenario_seed, order_id)`
/// via blake3, making jitter deterministic across replay runs (D2).
///
/// ## Slippage model (Q2 = linear bps / ADR-0043 § D3)
///
/// `slippage_bps == 0`: fill price unchanged (noop).
/// `slippage_bps > 0`: `fill_price = signal_price * (1 ± bps/10_000)`,
/// sign-applied per `Side` (Buy = +, Sell = −).
///
/// ## Scope (Q4 = backtest-only / ADR-0043 § D5)
///
/// This config is consumed only by `crates/backtest`. The live-mode agent
/// (`crates/agent`) does not read it — live fills already carry real
/// latency and slippage from the venue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencySlippageSimConfig {
    /// Minimum latency added to `order_ts_ms` in milliseconds.
    /// Default: 0 (noop).
    pub latency_ms_min: u64,
    /// Maximum latency added to `order_ts_ms` in milliseconds.
    /// When equal to `latency_ms_min`: fixed delay. Default: 0 (noop).
    pub latency_ms_max: u64,
    /// Linear slippage in basis points applied to the fill price.
    /// Default: 0 (noop).
    pub slippage_bps: u32,
}

impl LatencySlippageSimConfig {
    /// Returns `true` when the config is the noop default (all zeros).
    /// Used by callers to skip RNG construction on the hot path.
    #[inline]
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.latency_ms_min == 0 && self.latency_ms_max == 0 && self.slippage_bps == 0
    }
}

// ── T-D-N1 unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
mod latency_slippage_config_tests {
    use super::*;

    /// T-D-N1: Default config is all zeros (noop / anchor-safe).
    #[test]
    fn latency_slippage_sim_config_default_is_noop() {
        let cfg = LatencySlippageSimConfig::default();
        assert_eq!(cfg.latency_ms_min, 0, "default latency_ms_min must be 0");
        assert_eq!(cfg.latency_ms_max, 0, "default latency_ms_max must be 0");
        assert_eq!(cfg.slippage_bps, 0, "default slippage_bps must be 0");
        assert!(cfg.is_noop(), "default must be noop");
    }

    /// Default config has `PartialEq` with another default.
    #[test]
    fn default_equals_default() {
        let a = LatencySlippageSimConfig::default();
        let b = LatencySlippageSimConfig::default();
        assert_eq!(a, b);
    }

    /// Non-zero config is NOT noop.
    #[test]
    fn non_zero_is_not_noop() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 50,
            latency_ms_max: 100,
            slippage_bps: 10,
        };
        assert!(!cfg.is_noop(), "non-zero config must not be noop");
    }

    /// Serialization round-trip (Serde derives).
    #[test]
    fn serde_round_trip() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 20,
            latency_ms_max: 80,
            slippage_bps: 5,
        };
        let json = serde_json::to_string(&cfg).expect("must serialize");
        let back: LatencySlippageSimConfig = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(cfg, back);
    }

    // ── T-D-N4 plumbing tests: default-is-noop for each new struct ────────────

    /// T-D-N4a: `PairsScenarioInput.latency_slippage_sim` defaults to noop.
    #[test]
    fn pairs_scenario_input_default_sim_is_noop() {
        let input = super::PairsScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "pairs_mr_h1".to_string(),
            latency_slippage_sim: LatencySlippageSimConfig::default(),
        };
        assert!(
            input.latency_slippage_sim.is_noop(),
            "PairsScenarioInput: default sim must be noop"
        );
    }

    /// T-D-N4b: `TcnScenarioInput.latency_slippage_sim` defaults to noop.
    #[test]
    fn tcn_scenario_input_default_sim_is_noop() {
        let input = super::TcnScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: LatencySlippageSimConfig::default(),
        };
        assert!(
            input.latency_slippage_sim.is_noop(),
            "TcnScenarioInput: default sim must be noop"
        );
    }

    /// T-D-N4c: `SmaComposedRunInput.latency_slippage_sim` defaults to noop.
    #[test]
    fn sma_composed_run_input_default_sim_is_noop() {
        use trading_core::Symbol;
        let input = super::SmaComposedRunInput {
            strategy_id: "sma_crossover".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: LatencySlippageSimConfig::default(),
        };
        assert!(
            input.latency_slippage_sim.is_noop(),
            "SmaComposedRunInput: default sim must be noop"
        );
    }

    /// T-D-N4d: non-zero config flows through `PairsScenarioInput`.
    #[test]
    fn pairs_scenario_input_non_zero_sim_flows_through() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_bps: 8,
        };
        let input = super::PairsScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "pairs_mr_h1".to_string(),
            latency_slippage_sim: cfg.clone(),
        };
        assert!(!input.latency_slippage_sim.is_noop());
        assert_eq!(input.latency_slippage_sim.slippage_bps, 8);
    }

    /// T-D-N4e: non-zero config flows through `TcnScenarioInput`.
    #[test]
    fn tcn_scenario_input_non_zero_sim_flows_through() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_bps: 8,
        };
        let input = super::TcnScenarioInput {
            scenario_name: "test".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: None,
            emit_equity_bin: None,
            latency_slippage_sim: cfg.clone(),
        };
        assert!(!input.latency_slippage_sim.is_noop());
        assert_eq!(input.latency_slippage_sim.slippage_bps, 8);
    }

    /// T-D-N4f: non-zero config flows through `SmaComposedRunInput`.
    #[test]
    fn sma_composed_run_input_non_zero_sim_flows_through() {
        use trading_core::Symbol;
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_bps: 8,
        };
        let input = super::SmaComposedRunInput {
            strategy_id: "sma_crossover".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 100,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: cfg.clone(),
        };
        assert!(!input.latency_slippage_sim.is_noop());
        assert_eq!(input.latency_slippage_sim.slippage_bps, 8);
    }
}

// ── SMA / Composed scenario input ─────────────────────────────────────────────

/// Inputs for the SMA crossover + Composed scenario backtest and report writer.
///
/// Extracted from `main.rs::Scenario` and the surrounding `main()` context.
/// Used by `scenarios::sma_composed::run` and `report::sma::write`.
#[derive(Debug, Clone)]
pub struct SmaScenarioInput {
    /// Canonical scenario name (written into the YAML front-matter `scenario:` field).
    pub scenario_name: String,
    /// Canonical name written into the **report body** (may differ for alias scenarios
    /// like `btc-2023-1m-sma-baseline-refresh` which use `btc-2023-1m-sma-cross`).
    pub body_name: String,
    /// Override for the `Wall-clock time` row in the body.
    /// `Some(0.2)` for v0-anchor scenarios to preserve the locked SHA-256.
    /// `None` means use the actual elapsed time.
    pub body_elapsed_override: Option<f64>,
    pub symbol: Symbol,
    pub start_year: i32,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Path to the SMA-baseline report (for comparative scenarios).
    pub baseline_report: Option<String>,
}

// ── Momentum scenario input ────────────────────────────────────────────────────

/// Inputs for the v1 cross-sectional momentum backtest and report writer.
#[derive(Debug, Clone)]
pub struct MomentumScenarioInput {
    /// CLI scenario name (written into the YAML `scenario:` field).
    pub scenario_name: String,
    pub start_year: i32,
    pub bar_count: usize,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Strategy config ID, e.g. `"top10_momentum_h1"`.
    pub config_id: String,
    /// Pre-loaded real bars (`RealData` path). `None` → generate synthetic bars.
    ///
    /// Added for `v3.0.0-volatility-rebaseline` (`top10-2023-fy-momentum-realdata`)
    /// to support real Binance data as the un-targeted v1 momentum baseline.
    pub bars_override: Option<Vec<trading_core::Bar>>,
    /// Dataset revision SHA (from `data/binance/REVISION.toml`) for the
    /// `data_revision_sha:` frontmatter field. `None` for synthetic scenarios.
    pub data_revision_sha: Option<String>,
    /// v5-latency-slippage-sim R1 / ADR-0043 § D1 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    ///
    /// **Anchor contract**: CLI paths that construct `MomentumScenarioInput`
    /// without this field use `..Default::default()` or `LatencySlippageSimConfig::default()`
    /// explicitly, ensuring byte-identical output for all 34 anchored scenarios.
    pub latency_slippage_sim: LatencySlippageSimConfig,
}

// ── Pairs scenario input ───────────────────────────────────────────────────────

/// Inputs for the v1.5a mean-reversion pairs backtest and report writer.
#[derive(Debug, Clone)]
pub struct PairsScenarioInput {
    /// CLI scenario name (written into the YAML `scenario:` field).
    pub scenario_name: String,
    pub start_year: i32,
    pub bar_count: usize,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Strategy config ID, e.g. `"pairs_mr_h1"`.
    pub config_id: String,
    /// v5-latency-slippage-sim R1 / ADR-0047 D2 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    pub latency_slippage_sim: LatencySlippageSimConfig,
}

// ── TCN overlay scenario input ─────────────────────────────────────────────────

/// Inputs for the v2.5 TCN overlay momentum backtest and report writer.
/// Shared by both the passthrough and real-weights variants.
/// Also used by `patchtst_overlay_weights`, `garch_vol_target_overlay`, and
/// `threshold_sweep` (all share this struct — field added once, auto-propagates).
#[derive(Debug, Clone)]
pub struct TcnScenarioInput {
    /// CLI scenario name (written into the YAML `scenario:` field).
    pub scenario_name: String,
    pub start_year: i32,
    pub bar_count: usize,
    pub initial_capital: Decimal,
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    /// Strategy config ID, e.g. `"tcn_overlay_momentum"`.
    pub config_id: String,
    /// Forecaster ID, e.g. `"passthrough"` or `"tcn-bs1"`.
    pub forecaster_id: String,
    /// Pre-loaded real bars (`RealData` path). `None` → generate synthetic bars.
    pub bars_override: Option<Vec<trading_core::Bar>>,
    /// Optional output path for the equity-curve text file (`--emit-equity-bin`).
    pub emit_equity_bin: Option<PathBuf>,
    /// v5-latency-slippage-sim R1 / ADR-0047 D2 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    ///
    /// NOTE: `threshold_sweep::run_cell` consumes this field but the sim is
    /// structurally noop for analysis sweeps (no equity surface). Deferred per
    /// ADR-0047 D2.
    pub latency_slippage_sim: LatencySlippageSimConfig,
}

// ── BacktestState (SMA / Composed run state) ──────────────────────────────────

/// Single-symbol backtest accumulator state.
/// Mirrors `main.rs::BacktestState` @2380.
/// Lives here so `report::sma::write` can reference it.
pub struct BacktestState {
    pub cash: rust_decimal::Decimal,
    pub position_qty: rust_decimal::Decimal,
    pub position_cost: rust_decimal::Decimal,
    pub trades: usize,
    pub buys: usize,
    pub sells: usize,
    pub total_fees: rust_decimal::Decimal,
    pub peak_equity: rust_decimal::Decimal,
    pub max_drawdown: rust_decimal::Decimal,
    pub ledger_imbalance_events: usize,
    pub equity_curve: Vec<rust_decimal::Decimal>,
}

impl BacktestState {
    #[must_use]
    pub fn new(initial_capital: rust_decimal::Decimal) -> Self {
        Self {
            cash: initial_capital,
            position_qty: rust_decimal::Decimal::ZERO,
            position_cost: rust_decimal::Decimal::ZERO,
            trades: 0,
            buys: 0,
            sells: 0,
            total_fees: rust_decimal::Decimal::ZERO,
            peak_equity: initial_capital,
            max_drawdown: rust_decimal::Decimal::ZERO,
            ledger_imbalance_events: 0,
            equity_curve: vec![initial_capital],
        }
    }

    #[must_use]
    pub fn equity(&self, mark: rust_decimal::Decimal) -> rust_decimal::Decimal {
        self.cash + self.position_qty * mark
    }

    pub fn update_drawdown(&mut self, equity: rust_decimal::Decimal) {
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }
        if self.peak_equity > rust_decimal::Decimal::ZERO {
            let dd = (self.peak_equity - equity) / self.peak_equity;
            if dd > self.max_drawdown {
                self.max_drawdown = dd;
            }
        }
    }

    pub fn apply_buy(
        &mut self,
        qty: rust_decimal::Decimal,
        fill_price: rust_decimal::Decimal,
        fee: rust_decimal::Decimal,
    ) {
        let notional = qty * fill_price;
        self.cash -= notional + fee;
        self.position_qty += qty;
        self.position_cost += notional;
        self.total_fees += fee;
        self.trades += 1;
        self.buys += 1;
    }

    pub fn apply_sell(
        &mut self,
        qty: rust_decimal::Decimal,
        fill_price: rust_decimal::Decimal,
        fee: rust_decimal::Decimal,
    ) {
        let notional = qty * fill_price;
        self.cash += notional - fee;
        self.position_qty -= qty;
        if self.position_qty < rust_decimal::Decimal::ZERO {
            self.position_qty = rust_decimal::Decimal::ZERO;
            self.position_cost = rust_decimal::Decimal::ZERO;
        }
        self.total_fees += fee;
        self.trades += 1;
        self.sells += 1;
    }
}

// ── SmaComposedRun scenario input (Wave D-2 / T-AR-4) ────────────────────────

/// Input for the single-symbol SMA/Composed strategy bar-loop extracted into
/// `scenarios::sma_composed_run`. Carries the strategy id and the Scenario
/// parameters needed by both the engine dispatch path and the CLI call path.
///
/// `strategy_id` selects which composed TOML is loaded
/// (`config/strategies/<strategy_id>.toml`) — or `"sma_crossover"` for the
/// compiled-in crossover.
#[derive(Debug, Clone)]
pub struct SmaComposedRunInput {
    /// Strategy id, e.g. `"sma_crossover"`, `"btc_macd_trend"`,
    /// `"btc_rsi_reversion"`, `"btc_bbands_mean_revert"`.
    pub strategy_id: String,
    /// Symbol to trade, e.g. `Symbol::new("BTCUSDT")`.
    pub symbol: trading_core::Symbol,
    /// Calendar year for synthetic-bar epoch base (2023 or 2024).
    pub start_year: i32,
    /// Total bars to replay (e.g. `525_600` for a full 2023 minute run).
    pub bar_count: usize,
    /// Starting portfolio capital in USDT.
    pub initial_capital: rust_decimal::Decimal,
    /// Matching engine slippage in basis points.
    pub slippage_bps: u32,
    /// Matching engine taker fee in basis points.
    pub taker_fee_bps: u32,

    // ── lab-polish-round-2 R2 — SMA param overrides ──────────────────────
    //
    // ANCHOR-PRESERVING CONTRACT: when both fields are None the run uses
    // the legacy (20, 50) hardcoded defaults so anchored CLI scenarios stay
    // byte-identical. The Lab UI sets these to user-chosen values to enable
    // in-cockpit A/B testing without editing TOML.
    //
    /// Optional override for the fast SMA window length (default 20).
    /// Only honoured when `strategy_id == "sma_crossover"`. `None` → 20.
    pub sma_fast_len: Option<usize>,
    /// Optional override for the slow SMA window length (default 50).
    /// Only honoured when `strategy_id == "sma_crossover"`. `None` → 50.
    pub sma_slow_len: Option<usize>,
    /// v5-latency-slippage-sim R1 / ADR-0047 D2 — optional deterministic
    /// latency + slippage simulation. Default is noop (all zeros).
    pub latency_slippage_sim: LatencySlippageSimConfig,
}

// ── Strategy metadata (SMA / Composed report) ─────────────────────────────────

/// Strategy metadata for the SMA/Composed report.
/// Mirrors `main.rs::StrategyMeta` @538 plus `strategy_notes` from `write_report` @2538.
#[derive(Debug, Clone)]
pub struct StrategyMeta {
    pub id: String,
    pub kind: String,
    pub hash_hex: String,
    pub source_path: String,
    pub signal: String,
    /// Fragment for the `## Notes` section, e.g. "v0 SMA crossover: fast=20, slow=50".
    /// Mirrors `main.rs::write_report::strategy_notes` @2538.
    pub notes: String,
}
