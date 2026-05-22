//! Shared CLI-scenario types used by `main.rs` and the extracted scenario
//! modules. These types are **not** part of the public `backtest` library API
//! (they live in the binary path), but are shared across `main.rs` and the
//! `scenarios::*` / `report::*` modules via `pub` visibility.
//!
//! Defined here so the scenario modules can take typed inputs without
//! depending on `main.rs`'s internal `Scenario` struct.

use std::path::PathBuf;

use rust_decimal::Decimal;
use trading_core::Symbol;

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
}

// ── TCN overlay scenario input ─────────────────────────────────────────────────

/// Inputs for the v2.5 TCN overlay momentum backtest and report writer.
/// Shared by both the passthrough and real-weights variants.
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
