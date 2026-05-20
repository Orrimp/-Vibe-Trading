//! v2.5 TCN overlay momentum scenario execution — Phase B T-D-N5.
//!
//! Extracted from `main.rs::run_tcn_overlay_backtest` @1633. Behaviour-preserving.
//! TCN extracted last per K2 mitigation.
//!
//! Note: the model-load step happens once before the bar loop. The cancel poll
//! is inserted inside the bar loop only (not on the model load, per spec).

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    Bar, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol, TimeInForce,
};

use crate::cli_types::TcnScenarioInput;
use crate::scenarios::momentum::{synthetic_bars_hourly, top10_symbols_with_prices};

// ── Result struct ─────────────────────────────────────────────────────────────

/// Result of the v2.5 TCN overlay momentum backtest.
pub struct TcnOverlayRunResult {
    pub trades: usize,
    pub buys: usize,
    pub sells: usize,
    pub total_fees: Decimal,
    pub final_equity: Decimal,
    pub initial_equity: Decimal,
    pub max_drawdown: Decimal,
    pub bar_count: usize,
    pub elapsed_secs: f64,
    pub universe: Vec<String>,
    pub strategy_id: String,
    pub dampened_signals: u64,
    pub passed_through_signals: u64,
    pub warmup_signals: u64,
    pub forecaster_label: String,
    /// Per-bar equity curve. Populated when `emit_equity_bin` is set.
    pub equity_curve: Vec<Decimal>,
}

// ── Run function ──────────────────────────────────────────────────────────────

/// Run the v2.5 TCN overlay momentum backtest (passthrough / no-candle variant).
///
/// Extracted from `main.rs::run_tcn_overlay_backtest` @1633. Behaviour-preserving.
///
/// # Errors
///
/// Returns `Err` if the momentum config file cannot be loaded or is malformed.
#[allow(clippy::too_many_lines)]
pub async fn run(input: TcnScenarioInput, seed: u64) -> Result<TcnOverlayRunResult> {
    use crate::engine::MatchingEngine as _;
    use strategy::Strategy as _;

    let start_instant = Instant::now();

    // Load the base momentum config.
    let base_config_id = "top10_momentum_h1";
    let toml_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", toml_path.display()))?;
    let universe_list: Vec<String> = cfg.universe.iter().map(ToString::to_string).collect();
    let strategy_id_str = format!("tcn_overlay_momentum/{}", input.config_id);

    let base = strategy::MomentumStrategy::from_config(
        cfg,
        smol_str::SmolStr::new(toml_path.to_string_lossy()),
    );

    // Use passthrough forecaster (no candle dep in backtest binary).
    let mut overlay_strategy = strategy::TcnOverlayMomentumStrategy::with_passthrough(base);

    // Generate synthetic hourly bars or use pre-loaded real bars.
    let (merged_bars, bar_count) = if let Some(real_bars) = input.bars_override {
        let n = real_bars.len();
        tracing::info!(
            bar_count = n,
            "tcn-overlay realdata backtest — using pre-loaded real bars"
        );
        (real_bars, n)
    } else {
        let symbols_prices = top10_symbols_with_prices();
        // SAFETY: idx is at most 9 (10-symbol universe); cast to u64 is safe.
        #[allow(clippy::cast_possible_wrap)]
        let bars_by_symbol: Vec<Vec<Bar>> = symbols_prices
            .iter()
            .enumerate()
            .map(|(idx, (sym, start_price))| {
                let sym_seed = seed.wrapping_add(idx as u64 * 0x9E37_79B9);
                let adjusted_price = if input.start_year == 2024 {
                    *start_price * dec!(2.5)
                } else {
                    *start_price
                };
                synthetic_bars_hourly(
                    sym,
                    input.bar_count,
                    sym_seed,
                    adjusted_price,
                    input.start_year,
                )
            })
            .collect();
        // k-way merge: (venue_ts ASC, symbol ASC).
        let merged = data::ReplayFeed::merge_synthetic(bars_by_symbol);
        let n = merged.len();
        tracing::info!(
            bar_count = n,
            symbols = symbols_prices.len(),
            "merged synthetic bars for tcn-overlay backtest"
        );
        (merged, n)
    };

    // Paper matching engine.
    let match_config = crate::paper::MatchConfig {
        slippage_bps: input.slippage_bps,
        taker_fee_bps: input.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: crate::paper::FillPriceMode::BarClose,
    };
    let mut engine = crate::PaperEngine::new(match_config, seed);

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    let mut cash = input.initial_capital;
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();

    let mut trades = 0usize;
    let mut buys = 0usize;
    let mut sells = 0usize;
    let mut total_fees = Decimal::ZERO;
    let mut equity_curve: Vec<Decimal> = vec![input.initial_capital];
    let mut peak_equity = input.initial_capital;
    let mut max_drawdown = Decimal::ZERO;

    for bar in &merged_bars {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        let signals = overlay_strategy.on_bar(bar);

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
    tracing::info!(
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
        initial_equity: input.initial_capital,
        max_drawdown,
        bar_count,
        elapsed_secs,
        universe: universe_list,
        strategy_id: strategy_id_str,
        dampened_signals: stats.dampened,
        passed_through_signals: stats.passed_through,
        warmup_signals: stats.window_warming_up,
        forecaster_label: "passthrough (no-candle mode — degrades to v1 momentum)".to_string(),
        equity_curve,
    })
}
