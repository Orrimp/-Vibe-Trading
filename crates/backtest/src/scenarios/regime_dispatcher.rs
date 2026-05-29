//! v3.0.0-regime `RegimeDispatcher` backtest scenario — Wave E T-D-E1.
//!
//! Wraps the v1 cross-sectional momentum strategy with a `RegimeDispatcher`
//! that uses a `MarkovSwitchingClassifier` trained on per-symbol log-returns.
//!
//! ## Routing (ADR-0049 § D3)
//!
//! ```text
//! Regime    → Strategy
//! ──────────────────────────────────
//! Bull      → MomentumStrategy
//! Bear      → MomentumStrategy
//! Volatile  → CashHoldStrategy
//! Calm      → CashHoldStrategy
//! ```
//!
//! ## Train/val split (Q2=(c) operator decision)
//!
//! The classifier is trained on the 2023 window (train); the 2024 window
//! is the held-out validation set.  The dispatcher runs in online mode —
//! it re-fits periodically as bars arrive — so the train/val boundary is
//! implicit in the scenario year selection:
//! - `top10-2023-fy-regime-dispatcher-realdata` → 2023 full year (train window)
//! - `top10-2024-fy-regime-dispatcher-realdata` → 2024 full year (val window)
//!
//! ## Signal semantics
//!
//! The dispatcher emits signals identically to the base `MomentumStrategy`
//! when routing to it, and `SignalKind::Hold` (suppression) when routing to
//! `CashHoldStrategy`.  The buy sizing uses `fraction = 0.10` (equal-weight
//! 10% exposure per symbol, 10 symbols → full 100% potential deployment).
//!
//! ## Determinism (ADR-0049 § D5 / CLAUDE.md non-negotiables)
//!
//! - No `SystemTime::now()` on any code path.
//! - All money arithmetic in `rust_decimal::Decimal`.
//! - Classifier is `MarkovSwitchingClassifier` with operator-set semantic
//!   priors (ADR-0049 § D1).  The EM algorithm is deterministic given the
//!   same input log-return sequence.
//!
//! ## Cross-references
//!
//! - ADR-0049 § D3 — dispatcher + cash-fallback routing contract.
//! - ADR-0049 § D6 — confidence gate (0.70).
//! - `crates/strategy/src/regime_dispatcher.rs` — dispatcher impl.
//! - `crates/forecast/src/markov_switching.rs` — classifier.
//! - `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` — structural pattern.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::DispatchedRegime;
use trading_core::{
    Bar, FillView, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol,
    TimeInForce, Timeframe, Venue,
};

use crate::engine::MatchingEngine as _;
use crate::scenarios::momentum::{synthetic_bars_hourly, top10_symbols_with_prices};
use crate::scenarios::sim::sim_slippage_cost;
use strategy::Strategy as _;

// ── Fill bar builder ──────────────────────────────────────────────────────────

/// Build a minimal `Bar` for `sym` with `close = mark` so the paper engine
/// applies slippage and fees at the CORRECT symbol price.
///
/// Without this helper, `engine.step(current_bar, order_for_other_sym)` would
/// use the current bar's close (wrong symbol's price) as the fill price, which
/// can cause catastrophic notional mismatches when cheap and expensive tokens
/// trade in the same merged stream (e.g., DOGE bought at BTC price).
///
/// The bar's timestamp is copied from the current stream bar; only `symbol`
/// and `close` (and derived OHLC) differ.
fn make_fill_bar(sym: &Symbol, mark: Decimal, ref_bar: &Bar) -> Bar {
    let price = Price::new(mark)
        .unwrap_or_else(|_| Price::new(dec!(1)).unwrap_or_else(|e| unreachable!("{e}")));
    Bar {
        symbol: sym.clone(),
        tf: Timeframe::OneHour,
        open_ts: ref_bar.open_ts,
        close_ts: ref_bar.close_ts,
        local_recv_ts: ref_bar.local_recv_ts,
        venue: Venue::Binance,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: Quantity::new(dec!(1)).unwrap_or_else(|e| unreachable!("{e}")),
        trade_count: 1,
    }
}

// ── Result struct ─────────────────────────────────────────────────────────────

/// Result of the v3.0.0-regime `RegimeDispatcher` backtest.
///
/// Reuses the `TcnOverlayRunResult` shape to share the existing report writer
/// (since the report body fields are identical to the TCN overlay shape).
pub struct RegimeDispatcherRunResult {
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
    /// Number of bars where the dispatcher routed to `CashHoldStrategy`
    /// (i.e. regime was Volatile or Calm with confidence >= 0.70).
    pub suppressed_bars: usize,
    /// Number of bars where the dispatcher routed to `MomentumStrategy`
    /// (Bull or Bear with confidence >= 0.70, or any regime below threshold).
    pub momentum_bars: usize,
    /// Number of bars during classifier warm-up (before first fit).
    pub warmup_bars: usize,
    /// Per-bar equity curve.
    pub equity_curve: Vec<Decimal>,
    /// All fills produced during the run.
    pub fills: Vec<FillView>,
    /// All bars (Arc-shared).
    pub bars: Arc<Vec<Bar>>,
    /// Descriptive label for the forecaster / classifier configuration.
    pub forecaster_label: String,
}

// ── Run function ──────────────────────────────────────────────────────────────

/// Run the v3.0.0-regime `RegimeDispatcher` momentum backtest.
///
/// Builds a `MomentumStrategy` wrapped in a `RegimeDispatcher` using a
/// `MarkovSwitchingClassifier` with ADR-0049 § D1 priors.
///
/// # Errors
///
/// Returns `Err` if:
/// - The base momentum config cannot be loaded.
/// - Bar data loading fails (real-data path).
#[allow(clippy::too_many_lines)]
#[allow(unused_variables)]
#[allow(clippy::unused_async)]
pub async fn run(
    input: crate::cli_types::TcnScenarioInput,
    seed: u64,
) -> Result<RegimeDispatcherRunResult> {
    use std::path::PathBuf;

    use strategy::regime_dispatcher::MarkovSwitchingClassifier;
    use strategy::{CrossSectionalMomentumConfig, MomentumStrategy, RegimeDispatcherConfig};

    let start_instant = Instant::now();

    // ── Load base momentum config ──────────────────────────────────────────────

    let base_config_id = "top10_momentum_h1";
    let rel_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
    let toml_path = crate::paths::resolve_workspace_path(&rel_path);
    let cfg = CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", rel_path.display()))?;
    let universe_list: Vec<String> = cfg.universe.iter().map(ToString::to_string).collect();
    let strategy_id_str = format!("regime_dispatcher_momentum/{}", input.config_id);

    let base_momentum =
        MomentumStrategy::from_config(cfg, SmolStr::new(rel_path.to_string_lossy()));

    // ── Build the regime dispatcher ────────────────────────────────────────────

    let classifier = MarkovSwitchingClassifier::new();
    let dispatcher_config = RegimeDispatcherConfig::default();
    let forecaster_label = format!(
        "RegimeDispatcher(MarkovSwitching 4-state, confidence_gate=0.70, \
         v3.0.0-regime, {} symbols)",
        universe_list.len()
    );

    let mut dispatcher_strategy =
        strategy::with_regime_dispatcher(base_momentum, classifier, dispatcher_config);

    tracing::info!(
        symbols = universe_list.len(),
        "built regime dispatcher with MarkovSwitchingClassifier"
    );

    // ── Load bars ─────────────────────────────────────────────────────────────

    let (merged_bars_raw, bar_count) = if let Some(real_bars) = input.bars_override {
        let n = real_bars.len();
        tracing::info!(
            bar_count = n,
            "regime-dispatcher realdata backtest — using pre-loaded real bars"
        );
        (real_bars, n)
    } else {
        // Generate synthetic hourly bars for the 10-symbol universe.
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
        tracing::info!(
            bar_count = n,
            symbols = symbols_prices.len(),
            "merged synthetic bars for regime-dispatcher backtest"
        );
        (merged, n)
    };

    // ── Paper matching engine ──────────────────────────────────────────────────

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

    // Regime dispatcher diagnostics.
    let mut suppressed_bars = 0usize;
    let mut momentum_bars = 0usize;
    let mut warmup_bars = 0usize;

    // Preserve bars in an Arc for the result struct.
    let bars_arc: Arc<Vec<Bar>> = Arc::new(merged_bars_raw);

    // ── Bar loop ───────────────────────────────────────────────────────────────

    for bar in bars_arc.iter() {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        // Track dispatcher regime before the call to inspect routing post-call.
        let pre_bar_is_fitted = dispatcher_strategy.is_fitted();
        let pre_bar_regime = dispatcher_strategy.current_regime();

        let signals = dispatcher_strategy.on_bar(bar);

        // Routing diagnostics: classify this bar.
        let post_bar_regime = dispatcher_strategy.current_regime();
        match post_bar_regime {
            DispatchedRegime::CashHold => suppressed_bars += 1,
            DispatchedRegime::Momentum => {
                if pre_bar_is_fitted {
                    momentum_bars += 1;
                } else {
                    warmup_bars += 1;
                }
            }
        }

        // Process signals.
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
                    // RegimeDispatcher does NOT apply quantity_scale — it routes
                    // full v1 momentum signals (scale = 1.0 invariant from K6 gate).
                    let fraction = dec!(0.10);
                    let notional = equity * fraction;
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    let pos_snap = Position::empty(sig.symbol.clone());
                    // Use a fill bar for `sig.symbol` at the correct mark price.
                    // The paper engine uses `bar.close` as the fill base; if we
                    // pass the stream bar (possibly a different symbol), the fill
                    // price would be catastrophically wrong when cheap and expensive
                    // tokens interleave (e.g. DOGE qty bought at BTC close price).
                    let fill_bar = make_fill_bar(&sig.symbol, mark, bar);
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
                        && let Ok(fills) = engine.step(&fill_bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
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
                        }
                    }
                }
                trading_core::SignalKind::Sell if current_qty > Decimal::ZERO => {
                    let pos_snap = Position::empty(sig.symbol.clone());
                    // Use a fill bar for `sig.symbol` at the correct mark price.
                    let fill_bar = make_fill_bar(&sig.symbol, mark, bar);
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
                        && let Ok(fills) = engine.step(&fill_bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
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
                        }
                    }
                }
                // Hold signals from CashHoldStrategy and any other — no action.
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

    // ── Final accounting ───────────────────────────────────────────────────────

    let position_value: Decimal = position_book
        .iter()
        .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
        .sum();
    let final_equity = cash + position_value;
    let elapsed_secs = start_instant.elapsed().as_secs_f64();

    tracing::info!(
        elapsed_s = elapsed_secs,
        trades = trades,
        final_equity = %final_equity,
        suppressed_bars = suppressed_bars,
        momentum_bars = momentum_bars,
        warmup_bars = warmup_bars,
        "regime-dispatcher backtest complete"
    );

    Ok(RegimeDispatcherRunResult {
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
        suppressed_bars,
        momentum_bars,
        warmup_bars,
        forecaster_label,
        equity_curve,
        fills: Vec::new(), // not needed for report; anchor-additive placeholder
        bars: bars_arc,
    })
}
