//! v2.5 TCN overlay momentum with real anchor weights — Phase B T-D-N6.
//!
//! Extracted from `main.rs::run_tcn_overlay_weights_backtest` @1902. Behaviour-preserving.
//! TCN weights extracted last per K2 mitigation.
//!
//! Requires `--features candle` at compile time. Without the feature the
//! function returns an error rather than a silent passthrough fallback.

use anyhow::Result;

use crate::cli_types::TcnScenarioInput;
use crate::scenarios::tcn_overlay::TcnOverlayRunResult;

// ── Run function ──────────────────────────────────────────────────────────────

/// Run the v2.5 TCN overlay momentum backtest with real anchor weights.
///
/// Extracted from `main.rs::run_tcn_overlay_weights_backtest` @1902. Behaviour-preserving.
/// Requires `--features candle`. Without the feature returns `Err(...)`.
///
/// # Errors
///
/// Returns `Err` if `--features candle` is not enabled, if the TCN anchor
/// checkpoint cannot be loaded, or if an unknown `forecaster_id` is supplied.
// `async` is required by the `#[cfg(feature = "candle")]` block which calls
// `engine.step(...).await`. Without the feature the compiler sees no `.await`
// and flags `unused_async`; the suppression is intentional.
#[allow(clippy::too_many_lines)]
#[allow(unused_variables)]
#[allow(clippy::unused_async)]
pub async fn run(input: TcnScenarioInput, seed: u64) -> Result<TcnOverlayRunResult> {
    // ── Candle-gated forecaster construction ─────────────────────────────────
    // Without `--features candle` this produces a clear error rather than a
    // silent fallback — per the operator-locked decision in the M3 punch list.
    #[cfg(not(feature = "candle"))]
    {
        anyhow::bail!(
            "scenario '{name}' requires --features candle (real TCN weights). \
             Rebuild with: cargo run -p backtest --release --features candle -- \
             --scenario {name} --seed 0xC0FFEE",
            name = input.scenario_name,
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
        use crate::scenarios::sim::sim_slippage_cost;
        use strategy::Strategy as _;
        let start_instant = Instant::now();

        // Load the base momentum config. Bug #56 — resolve workspace-relative.
        let base_config_id = "top10_momentum_h1";
        let rel_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
        let toml_path = crate::paths::resolve_workspace_path(&rel_path);
        let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
            .with_context(|| format!("load momentum config: {}", rel_path.display()))?;
        let universe_list: Vec<String> = cfg.universe.iter().map(ToString::to_string).collect();
        let strategy_id_str = format!("tcn_overlay_momentum_weights/{}", input.config_id);

        let base = strategy::MomentumStrategy::from_config(
            cfg,
            // Bug #56 — keep rel_path (not resolved abs path) for anchor identity.
            smol_str::SmolStr::new(rel_path.to_string_lossy()),
        );

        // Load real anchor checkpoint.
        let forecaster_label = format!(
            "real TCN weights ({}, v2.5.0-tcn-weights)",
            input.forecaster_id
        );
        let overlay_strategy_result = match input.forecaster_id.as_str() {
            "tcn-bs1" => strategy::TcnOverlayMomentumStrategy::with_tcn_bs1(base),
            "tcn-bs2" => strategy::TcnOverlayMomentumStrategy::with_tcn_bs2(base),
            other => anyhow::bail!("unknown forecaster_id: {other}"),
        };
        let mut overlay_strategy = overlay_strategy_result
            .map_err(|e| anyhow::anyhow!("load TCN anchor checkpoint: {e}"))?;

        // Use pre-loaded real bars or generate synthetic bars.
        let (merged_bars_raw, bar_count) = if let Some(real_bars) = input.bars_override {
            let n = real_bars.len();
            tracing::info!(
                bar_count = n,
                "tcn-overlay-weights realdata backtest — using pre-loaded real bars"
            );
            (real_bars, n)
        } else {
            // Generate synthetic hourly bars for the 10-symbol universe.
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
                "merged synthetic bars for tcn-overlay-weights backtest"
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
        // bug-log #111/#129 fix (2026-10-12, operator-ruled): the fill-side twin of
        // `mark_prices`. This lane SIZED from the per-symbol mark and FILLED against the
        // trigger bar — bug-log #67's exact shape — and since the #67 guard returns Err,
        // those cross-symbol orders VANISHED with no fill, no error and no counter.
        let mut last_bar_by_symbol: std::collections::HashMap<Symbol, trading_core::Bar> =
            std::collections::HashMap::new();
        let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
            std::collections::BTreeMap::new();

        let mut trades = 0usize;
        let mut buys = 0usize;
        let mut sells = 0usize;
        let mut total_fees = Decimal::ZERO;
        // bug-log #95 MEASURE-FIRST: see the tracking site below.
        let mut peak_gross_observed = Decimal::ZERO;
        let mut portfolio_gross_refusals: u64 = 0;
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
            last_bar_by_symbol.insert(bar.symbol.clone(), bar.clone());

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
                        // ── bug-log #95: the portfolio GROSS cap, now actually ENFORCED ──
                        //
                        // Operator-ruled 2026-11-08 ("wire the three lanes, then re-emit"). Until this
                        // line the lane declared `portfolio_exposure_cap: Some(0.50)` and nothing read
                        // it: the only enforcer in the tree is `risk::size_portfolio_target`, which
                        // needs a target VECTOR, and this lane emits one order at a time. So the
                        // declaration was inert — and, per the MEASURE-FIRST instrumentation further
                        // down, 0.50 sits inside the range this lane can actually reach by
                        // appreciation (three legs at `fraction` each, cash held flat).
                        //
                        // Shape: an ENTRY GATE on RESULTING exposure — bug-log #71's discipline, since
                        // a cap that tests the INCREMENT is evadable by splitting (at 0.39 of equity a
                        // top-up ending at 0.44 passes on its own 0.05). Per-order refusal, NOT the
                        // theta lane's all-or-nothing rebalance skip: there is no batch here to skip.
                        // The order is refused, counted into the report's `Gross-cap refusals` row and
                        // logged; the book is left as it was.
                        if let Some(cap) = risk_limits.portfolio_exposure_cap
                            && risk::would_breach_gross_cap(position_value, notional, equity, cap)
                        {
                            portfolio_gross_refusals += 1;
                            tracing::warn!(
                                symbol = %sig.symbol,
                                resulting_gross =
                                    %risk::resulting_gross_fraction(position_value, notional, equity),
                                %cap,
                                %notional,
                                "portfolio GROSS cap would be breached — Buy REFUSED and counted \
                                 (bug-log #95); the book is unchanged"
                            );
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
                        {
                            // Route to the ORDER's symbol bar, not the trigger bar (bug-log #67).
                            let Some(fill_bar) = last_bar_by_symbol.get(&sig.symbol) else {
                                tracing::warn!(
                                    symbol = %sig.symbol,
                                    "no bar seen yet for this symbol — order SKIPPED, and said so"
                                );
                                continue;
                            };
                            // A refusal is REPORTED, never dropped. The `&& let Ok(fills) = …` if-chain
                            // this replaces had no else arm, so an Err made the whole chain false and the
                            // order disappeared in silence (bug-log #129).
                            let fills = match engine.step(fill_bar, vec![ord]).await {
                                Ok(f) => f,
                                Err(err) => {
                                    tracing::warn!(
                                        order_symbol = %sig.symbol,
                                        bar_symbol = %fill_bar.symbol,
                                        error = %err,
                                        "engine REFUSED the order — reported, not dropped"
                                    );
                                    continue;
                                }
                            };
                            for fill in fills {
                                let notional_fill = fill.qty.get() * fill.price.get();
                                // v5-latency-slippage-sim R1 wiring (ADR-0047 D2).
                                let sim_slip_cost = sim_slippage_cost(
                                    fill.qty.get(),
                                    fill.price.get(),
                                    Side::Buy,
                                    &input.latency_slippage_sim,
                                    &sig.symbol,
                                );
                                cash -= notional_fill + fill.fee.amount() + sim_slip_cost;
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
                        {
                            // Route to the ORDER's symbol bar, not the trigger bar (bug-log #67).
                            let Some(fill_bar) = last_bar_by_symbol.get(&sig.symbol) else {
                                tracing::warn!(
                                    symbol = %sig.symbol,
                                    "no bar seen yet for this symbol — order SKIPPED, and said so"
                                );
                                continue;
                            };
                            // A refusal is REPORTED, never dropped. The `&& let Ok(fills) = …` if-chain
                            // this replaces had no else arm, so an Err made the whole chain false and the
                            // order disappeared in silence (bug-log #129).
                            let fills = match engine.step(fill_bar, vec![ord]).await {
                                Ok(f) => f,
                                Err(err) => {
                                    tracing::warn!(
                                        order_symbol = %sig.symbol,
                                        bar_symbol = %fill_bar.symbol,
                                        error = %err,
                                        "engine REFUSED the order — reported, not dropped"
                                    );
                                    continue;
                                }
                            };
                            for fill in fills {
                                let notional_fill = fill.qty.get() * fill.price.get();
                                // v5-latency-slippage-sim R1 wiring (ADR-0047 D2).
                                let sim_slip_cost = sim_slippage_cost(
                                    fill.qty.get(),
                                    fill.price.get(),
                                    Side::Sell,
                                    &input.latency_slippage_sim,
                                    &sig.symbol,
                                );
                                cash += notional_fill - fill.fee.amount() - sim_slip_cost;
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
            // bug-log #95 MEASURE-FIRST (2026-11-07). This lane declares a
            // `portfolio_exposure_cap` that nothing on it enforces, and the 2026-10-12 trace
            // could NOT establish whether that cap would bind — its two challengers either
            // refuted the estimate or contradicted each other. Ruling on it would have been
            // ruling on a guess, so the operator's disposition was: measure first.
            //
            // Instrumentation ONLY: peak gross exposure to stderr via tracing, never into the
            // report body, so no anchored body-SHA moves and this needs no re-lock. The lane
            // is long-only here, so `position_value` IS gross notional. Logging each new peak
            // rather than a total at the end means the LAST such line is the answer and this
            // needs exactly one insertion point.
            if equity > Decimal::ZERO {
                let gross = position_value / equity;
                if gross > peak_gross_observed {
                    peak_gross_observed = gross;
                    tracing::info!(
                        lane = "tcn_overlay_weights",
                        %gross,
                        "MEASURE-95: new peak gross exposure (fraction of equity)"
                    );
                }
            }
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
            "tcn-overlay-weights backtest complete"
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
            // No portfolio gross cap enforced on this lane (bug-log #95).
            portfolio_gross_refusals,
            portfolio_exposure_cap: risk_limits.portfolio_exposure_cap,
            per_symbol_exposure_cap: risk_limits.per_symbol_exposure_cap,
            forecaster_label,
            equity_curve,
            fills: all_fills,
            bars: bars_arc,
            // lab-polish-round-2 R1 — position_curve not yet computed for the
            // weights variant (candle-gated). Defaults to empty — anchor-additive.
            position_curve: Vec::new(),
        })
    }
}
