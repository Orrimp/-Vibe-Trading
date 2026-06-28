//! Thin `run_cell` helper for the threshold-sweep bin (D-AR-1.c).
//!
//! Behaviorally-preserving extraction of `tcn_overlay_weights::run` with the
//! strategy-construction block replaced by a caller-supplied
//! `TcnOverlayMomentumStrategy` instance. The existing `tcn_overlay_weights::run`
//! is byte-identical for all 26 predecessor anchors; this module adds only the
//! per-cell hook without touching that function.
//!
//! ## Why a separate module
//!
//! `tcn_overlay_weights::run` accepts `TcnScenarioInput` and constructs its own
//! strategy from `input.forecaster_id`. The sweep bin needs to pass an already-
//! constructed strategy with explicit (τ, ε) so each of the 45 cells gets
//! a fresh `TcnOverlayMomentumStrategy::with_tcn_bs{1,2}_tuned(τ, ε)` instance.
//! Extracting a thin helper avoids any change to the anchor-load-bearing
//! `tcn_overlay_weights::run` body (R8 / ADR-0032 § D4).
//!
//! ## Anchor invariant
//!
//! This module contains NO `run()` function that shares a name with
//! `tcn_overlay_weights::run`. The existing anchored backtest scenarios call
//! `scenarios::tcn_overlay_weights::run` directly; those callers are unaffected.
//!
//! ## Cross-references
//!
//! - `spec/v1/v25-tcn-threshold-tuning/decomp.md § D-AR-1.c` — helper design.
//! - `crates/backtest/src/scenarios/tcn_overlay_weights.rs` — source of truth for
//!   the bar-loop body (replicated here for the caller-supplied strategy path).
//! - `crates/forecast/src/bin/threshold_sweep.rs` — sweep bin that calls `run_cell`.

use anyhow::Result;

use crate::cli_types::TcnScenarioInput;
use crate::scenarios::tcn_overlay::TcnOverlayRunResult;

// ── run_cell ──────────────────────────────────────────────────────────────────

/// Run one (τ, ε) cell of the threshold sweep with a caller-supplied strategy.
///
/// Mirrors `tcn_overlay_weights::run` exactly, EXCEPT:
/// - The strategy is caller-supplied (already constructed with the cell's τ + ε).
/// - The function does NOT load any checkpoint itself (load-once-use-45-times
///   pattern is handled at the sweep bin level).
///
/// Requires `--features candle` + `--features realdata` at the `backtest` crate
/// level (real bars come from `input.bars_override`). Without the feature flag
/// the function returns a clear error.
///
/// # Errors
///
/// Returns `Err` if the momentum config cannot be loaded, or if the bar-loop
/// encounters an engine error.
#[allow(clippy::too_many_lines)]
#[allow(unused_variables)]
#[allow(clippy::unused_async)]
pub async fn run_cell(
    input: TcnScenarioInput,
    seed: u64,
    overlay_strategy: strategy::TcnOverlayMomentumStrategy,
) -> Result<TcnOverlayRunResult> {
    // Without `--features candle` this produces a clear error matching the
    // existing `tcn_overlay_weights::run` pattern.
    #[cfg(not(feature = "candle"))]
    {
        anyhow::bail!(
            "threshold_sweep::run_cell requires --features candle. \
             Rebuild with: cargo run -p backtest --release --features candle,realdata -- …"
        )
    }
    #[cfg(feature = "candle")]
    {
        use std::path::PathBuf;
        use std::sync::Arc;
        use std::time::Instant;

        use anyhow::Context;
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;
        use smol_str::SmolStr;
        use trading_core::{
            Bar, FillView, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol,
            TimeInForce,
        };

        use crate::engine::MatchingEngine as _;
        use crate::scenarios::momentum::{synthetic_bars_hourly, top10_symbols_with_prices};
        use strategy::Strategy as _;

        let start_instant = Instant::now();

        // Load the base momentum config. Bug #56 — resolve workspace-relative.
        let base_config_id = "top10_momentum_h1";
        let rel_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
        let toml_path = crate::paths::resolve_workspace_path(&rel_path);
        let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
            .with_context(|| format!("load momentum config: {}", rel_path.display()))?;
        let universe_list: Vec<String> = cfg.universe.iter().map(ToString::to_string).collect();
        let strategy_id_str = format!("threshold_sweep/{}", input.config_id);
        let forecaster_label = format!("threshold_sweep tuned cell ({})", input.forecaster_id);

        // Use the caller-supplied strategy (already constructed with this cell's τ + ε).
        let mut strategy = overlay_strategy;

        // Use pre-loaded real bars (bars_override MUST be set for the sweep path).
        let (merged_bars_raw, bar_count) = if let Some(real_bars) = input.bars_override {
            let n = real_bars.len();
            tracing::debug!(
                bar_count = n,
                "threshold_sweep::run_cell — using pre-loaded real bars"
            );
            (real_bars, n)
        } else {
            // Synthetic fallback (for unit tests / CI without real data).
            let symbols_prices = top10_symbols_with_prices();
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
            let merged = data::ReplayFeed::merge_synthetic(bars_by_symbol);
            let n = merged.len();
            tracing::debug!(
                bar_count = n,
                "threshold_sweep::run_cell — synthetic fallback bars"
            );
            (merged, n)
        };

        // Paper matching engine — same config as `tcn_overlay_weights::run`.
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

        // F3 — collect fills for `TcnOverlayRunResult.fills`.
        let mut all_fills: Vec<FillView> = Vec::new();
        // Preserve bars in an Arc BEFORE the loop so the UI Lab chart can anchor
        // fill triangle markers against the run's own time window (R5.2 pattern).
        let bars_arc: Arc<Vec<Bar>> = Arc::new(merged_bars_raw);

        for bar in bars_arc.iter() {
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
                                total_fees += fill.fee.amount();
                                trades += 1;
                                buys += 1;
                                // F3 — convert Fill → FillView for the result struct.
                                all_fills.push(FillView {
                                    symbol: fill.symbol.clone(),
                                    side: fill.side,
                                    price: fill.price,
                                    qty: fill.qty,
                                    fee: fill.fee,
                                    fee_tier: fill.fee_tier,
                                    venue_ts: fill.venue_ts,
                                    transaction_id: SmolStr::default(),
                                });
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
                                // F3 — convert Fill → FillView for the result struct.
                                all_fills.push(FillView {
                                    symbol: fill.symbol.clone(),
                                    side: fill.side,
                                    price: fill.price,
                                    qty: fill.qty,
                                    fee: fill.fee,
                                    fee_tier: fill.fee_tier,
                                    venue_ts: fill.venue_ts,
                                    transaction_id: SmolStr::default(),
                                });
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

        let stats = &strategy.stats;
        tracing::debug!(
            elapsed_s = elapsed_secs,
            trades = trades,
            final_equity = %final_equity,
            dampened = stats.dampened,
            passed_through = stats.passed_through,
            warmup = stats.window_warming_up,
            "threshold_sweep::run_cell complete"
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
            forecaster_label,
            equity_curve,
            fills: all_fills,
            bars: bars_arc,
            // v5-latency-slippage-sim v0.2.0: threshold_sweep does not produce
            // per-(symbol,bar) position entries — supply empty vec.
            position_curve: Vec::new(),
        })
    }
}
