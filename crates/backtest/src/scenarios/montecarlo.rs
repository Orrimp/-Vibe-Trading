//! Monte-Carlo robustness harness — `run_path` cell wrapper (M-DEV-3).
//!
//! Behaviorally-preserving sibling of
//! `crates/backtest/src/scenarios/threshold_sweep::run_cell`.
//!
//! The key difference: `run_cell` accepts a caller-supplied
//! `TcnOverlayMomentumStrategy`; `run_path` accepts a caller-supplied
//! `MomentumStrategy` (plain v1, no TCN overlay) and uses
//! `input.bars_override` to inject a bootstrap-generated path.
//!
//! ## ADR-0051 D1 — seed orthogonality
//!
//! The fill-tie-break seed is a separate parameter (`fill_seed`) and is
//! HELD CONSTANT at `0xC0FFEE` across ALL paths in the harness. The path
//! is already injected by C1's ensemble; `PaperEngine` only needs the
//! fill-tie-break seed, which must not vary per path (holding it constant
//! is the key to measuring only path-variance, not path ⊕ fill-tie-break
//! combined noise).
//!
//! ## R-NR.2 compliance
//!
//! This module contains NO change to `PaperEngine`, `MatchingEngine`, or
//! any scenario `run()`. It is a new thin wrapper that reuses the existing
//! engine with a caller-supplied path and strategy.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use trading_core::Bar;

use crate::cli_types::TcnScenarioInput;

// ── Result struct ──────────────────────────────────────────────────────────────

/// Result of a single Monte-Carlo path backtest run.
///
/// Carries the equity curve (the only output the harness reducer needs)
/// plus basic counts for observability.
#[derive(Debug, Clone)]
pub struct PathRunResult {
    /// Per-bar equity curve: `[initial_capital, equity_after_bar_0, …]`.
    /// Length = `n_bars + 1`. Used by the harness reducer to compute all
    /// per-path metric scalars.
    pub equity_curve: Vec<Decimal>,
    /// Number of fills executed on this path.
    pub trades: usize,
    /// Initial capital (carried for P(loss) computation in the reducer).
    pub initial_equity: Decimal,
    /// Final equity (convenience; redundant with `equity_curve.last()`).
    pub final_equity: Decimal,
}

// ── run_path ───────────────────────────────────────────────────────────────────

/// Run one Monte-Carlo path with a caller-supplied `MomentumStrategy`.
///
/// Mirrors `threshold_sweep::run_cell` exactly, EXCEPT:
/// - Uses `strategy::MomentumStrategy` (plain v1, NOT the TCN overlay).
/// - `input.bars_override` MUST be `Some(…)` — the bootstrap path is
///   pre-loaded by the harness driver and injected here.
/// - `fill_seed` drives `PaperEngine::new` and is held CONSTANT across
///   all N paths (ADR-0051 D1 orthogonality invariant).
///
/// # Errors
///
/// Returns `Err` if `input.bars_override` is `None` (programming error —
/// the harness must always inject a path), or if the engine encounters
/// an error.
#[allow(clippy::too_many_lines)]
pub async fn run_path(
    input: TcnScenarioInput,
    fill_seed: u64,
    mut strategy: strategy::MomentumStrategy,
) -> Result<PathRunResult> {
    use std::path::PathBuf;

    use rust_decimal_macros::dec;
    use trading_core::{
        Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol, TimeInForce,
    };

    use crate::engine::MatchingEngine as _;
    use strategy::Strategy as _;

    // M-DEV-3: bars_override MUST be set — the harness injects the bootstrap path.
    let merged_bars: Vec<Bar> = input.bars_override.ok_or_else(|| {
        anyhow::anyhow!("run_path: bars_override must be Some — inject the bootstrap path")
    })?;

    let bar_count = merged_bars.len();
    tracing::debug!(
        bar_count,
        scenario = %input.scenario_name,
        fill_seed,
        "montecarlo::run_path starting"
    );

    // ── Paper matching engine (ADR-0051 D1: fill_seed is CONSTANT across paths)
    let match_config = crate::paper::MatchConfig {
        slippage_bps: input.slippage_bps,
        taker_fee_bps: input.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: crate::paper::FillPriceMode::BarClose,
    };
    let mut engine = crate::PaperEngine::new(match_config, fill_seed);

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    // Load config only to get the universe list — strategy is caller-supplied.
    let base_config_id = "top10_momentum_h1";
    let rel_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
    let toml_path = crate::paths::resolve_workspace_path(&rel_path);
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", rel_path.display()))?;
    let _ = cfg; // universe list is implicit in the merged bars

    let mut cash = input.initial_capital;
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut trades = 0usize;
    let mut equity_curve: Vec<Decimal> = vec![input.initial_capital];
    let mut peak_equity = input.initial_capital;
    let mut max_drawdown_tracking = Decimal::ZERO;

    for bar in &merged_bars {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        let signals = strategy.on_bar(bar);

        for sig in &signals {
            let Some(&mark) = mark_prices.get(&sig.symbol) else {
                continue;
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
                            trades += 1;
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
                            trades += 1;
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
        if dd > max_drawdown_tracking {
            max_drawdown_tracking = dd;
        }
    }

    let position_value: Decimal = position_book
        .iter()
        .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
        .sum();
    let final_equity = cash + position_value;

    tracing::debug!(
        scenario = %input.scenario_name,
        trades,
        final_equity = %final_equity,
        bars = bar_count,
        "montecarlo::run_path complete"
    );

    Ok(PathRunResult {
        equity_curve,
        trades,
        initial_equity: input.initial_capital,
        final_equity,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `run_path` with `None` `bars_override` returns `Err` (programming error detection).
    #[test]
    fn run_path_requires_bars_override() {
        use rust_decimal_macros::dec;
        use std::path::PathBuf;

        let rel = PathBuf::from("config/strategies/top10_momentum_h1.toml");
        let toml_path = crate::paths::resolve_workspace_path(&rel);
        // If config doesn't exist in test environment, skip gracefully.
        let Ok(cfg) = strategy::CrossSectionalMomentumConfig::from_file(&toml_path) else {
            return; // skip: no config in unit-test context
        };
        let strat = strategy::MomentumStrategy::from_config(
            cfg,
            smol_str::SmolStr::new("top10_momentum_h1"),
        );
        let input = TcnScenarioInput {
            scenario_name: "test-no-bars".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: None, // intentionally None → should error
            emit_equity_bin: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
        };
        let result = pollster::block_on(run_path(input, 0x00C0_FFEE, strat));
        assert!(
            result.is_err(),
            "run_path with None bars_override must return Err"
        );
    }
}
