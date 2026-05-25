//! v1.5a mean-reversion pairs scenario execution — Phase B T-D-N4.
//!
//! Extracted from `main.rs::run_pairs_backtest` @1163. Behaviour-preserving:
//! same seed derivation, same merge order, same fill logic.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{
    Bar, FillView, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol,
    TimeInForce,
};

use crate::cli_types::PairsScenarioInput;
use crate::scenarios::momentum::{pairs_symbols_with_prices, synthetic_bars_hourly};

// ── Result struct ─────────────────────────────────────────────────────────────

/// Result of the v1.5a mean-reversion pairs backtest.
pub struct PairsRunResult {
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
    pub config_hash_hex: String,
    /// Per-pair trade counts: (`pair_key_string`, trades).
    pub pair_trades: Vec<(String, usize)>,
    /// Per-bar equity curve (`[initial_capital, equity_after_bar_0, …]`).
    /// Populated for `RunReport.equity_series` in the engine dispatch path.
    pub equity_curve: Vec<Decimal>,
    /// All fills produced during the run, in bar order.
    /// Populated for `RunReport.fills` so the Lab UI can render buy/sell triangle markers.
    pub fills: Vec<FillView>,
    /// All bars from the run (Arc-shared to avoid copying).
    /// Populated for `RunReport.bars` so the Lab chart can anchor fill timestamps.
    pub bars: Arc<Vec<Bar>>,
    /// lab-polish-round-2 R1 — per-(symbol,bar) position entries.
    /// Format: `(bar.close_ts unix_millis, signed_qty, symbol)`.
    /// NOT written to Markdown reports — anchor-additive.
    pub position_curve: Vec<(i64, Decimal, trading_core::Symbol)>,
}

// ── Run function ──────────────────────────────────────────────────────────────

/// Run the v1.5a mean-reversion pairs backtest.
///
/// Extracted from `main.rs::run_pairs_backtest` @1163. Behaviour-preserving.
///
/// # Errors
///
/// Returns `Err` if the strategy config file cannot be loaded or is malformed.
#[allow(clippy::too_many_lines)]
/// Bug #63 — cancel + progress threading added so the Lab Stop button + progress
/// bar work for pairs runs.
pub async fn run(
    input: &PairsScenarioInput,
    seed: u64,
    cancel_rx: crate::cancel::RunCancelReceiver,
    progress_tx: crate::progress::ProgressSender,
) -> Result<PairsRunResult> {
    use crate::engine::MatchingEngine as _;
    use strategy::Strategy as _;

    let start_instant = Instant::now();

    // Load strategy config. Bug #56 — resolve workspace-relative so the
    // Lab cockpit launched from any CWD can still find the config.
    let rel_path = PathBuf::from(format!("config/strategies/{}.toml", input.config_id));
    let toml_path = crate::paths::resolve_workspace_path(&rel_path);
    let cfg = strategy::pairs::config::MeanReversionPairsConfig::from_file(&toml_path)
        .with_context(|| format!("load pairs config: {}", rel_path.display()))?;
    let universe_list: Vec<String> = {
        let mut syms: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in &cfg.pairs {
            syms.insert(p.key.a.to_string());
            syms.insert(p.key.b.to_string());
        }
        syms.into_iter().collect()
    };
    let strategy_id_str = cfg.id.to_string();

    let mut pairs_strategy =
        strategy::pairs::mean_reversion::MeanReversionPairsStrategy::from_config(
            cfg,
            // Bug #56 — keep rel_path (not resolved abs path) for anchor identity.
            smol_str::SmolStr::new(rel_path.to_string_lossy()),
        );
    let config_hash_hex = {
        use std::fmt::Write as _;
        pairs_strategy.hash.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    // Generate synthetic hourly bars for the 4-symbol universe.
    let symbols_prices = pairs_symbols_with_prices();
    // SAFETY: idx is at most 3 (4-symbol universe); cast to u64 is safe.
    #[allow(clippy::cast_possible_wrap)]
    let bars_by_symbol: Vec<Vec<trading_core::Bar>> = symbols_prices
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
    let merged_bars_raw = data::ReplayFeed::merge_synthetic(bars_by_symbol);
    let bar_count = merged_bars_raw.len();

    tracing::info!(
        bar_count = bar_count,
        symbols = symbols_prices.len(),
        "merged synthetic bars for pairs backtest"
    );

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
        portfolio_exposure_cap: Some(dec!(0.75)), // v1.5a: lifted per T714 comment
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
    let mut pair_trade_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    // F3 — collect fills for `PairsRunResult.fills`.
    let mut all_fills: Vec<FillView> = Vec::new();
    // lab-polish-round-2 R1 — per-(symbol,bar) position entries.
    let mut position_curve: Vec<(i64, Decimal, trading_core::Symbol)> = Vec::new();
    // Preserve bars in an Arc BEFORE the loop so the UI Lab chart can anchor
    // fill triangle markers against the run's own time window (R5.2 pattern).
    let bars_arc: Arc<Vec<Bar>> = Arc::new(merged_bars_raw);

    let total_bars = bars_arc.len();
    for (bar_idx, bar) in bars_arc.iter().enumerate() {
        // Bug #63 — cancel + progress poll at the 128-bar boundary.
        if bar_idx.trailing_zeros() >= 7 {
            if cancel_rx.is_cancelled() {
                return Err(anyhow::anyhow!("Cancelled"));
            }
            progress_tx.try_send(crate::progress::Progress {
                current_bar: bar_idx,
                total_bars,
                elapsed_ms: u64::try_from(start_instant.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
        }
        mark_prices.insert(bar.symbol.clone(), bar.close.get());

        let signals = pairs_strategy.on_bar(bar);

        for sig in &signals {
            let Some(&mark) = mark_prices.get(&sig.symbol) else {
                continue;
            };
            if mark <= Decimal::ZERO {
                continue;
            }

            // Only process OpenPairLong and ClosePair signals (formulation C).
            match sig.kind {
                trading_core::SignalKind::OpenPairLong => {
                    let current_qty = position_book
                        .get(&sig.symbol)
                        .copied()
                        .unwrap_or(Decimal::ZERO);
                    if current_qty > Decimal::ZERO {
                        continue;
                    }
                    let position_value: Decimal = position_book
                        .iter()
                        .map(|(sym, &qty)| {
                            qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO)
                        })
                        .sum();
                    let equity = cash + position_value;
                    if equity <= Decimal::ZERO {
                        continue;
                    }
                    let fraction = dec!(0.25);
                    let notional = equity * fraction;
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    if let (Ok(qty), Ok(price)) = (Quantity::new(qty_raw), Price::new(mark)) {
                        let pos_snap = Position::empty(sig.symbol.clone());
                        if let Ok(ord) = Order::new(
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
                        ) && let Ok(fills) = engine.step(bar, vec![ord]).await
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
                                if let Some(meta) = &sig.pair_data {
                                    let key_str = meta.pair_key.to_string();
                                    *pair_trade_counts.entry(key_str).or_insert(0) += 1;
                                }
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
                }
                trading_core::SignalKind::ClosePair => {
                    let current_qty = position_book
                        .get(&sig.symbol)
                        .copied()
                        .unwrap_or(Decimal::ZERO);
                    if current_qty <= Decimal::ZERO {
                        continue;
                    }
                    if let (Ok(qty), Ok(price)) = (Quantity::new(current_qty), Price::new(mark)) {
                        let pos_snap = Position::empty(sig.symbol.clone());
                        let position_value: Decimal = position_book
                            .iter()
                            .map(|(sym, &q)| {
                                q * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO)
                            })
                            .sum();
                        let equity = cash + position_value;
                        if let Ok(ord) = Order::new(
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
                        ) && let Ok(fills) = engine.step(bar, vec![ord]).await
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
                                if let Some(meta) = &sig.pair_data {
                                    let key_str = meta.pair_key.to_string();
                                    *pair_trade_counts.entry(key_str).or_insert(0) += 1;
                                }
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
                }
                // PairShortObservation: formulation C — no order emitted.
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

        // lab-polish-round-2 R1 — emit one position entry per (symbol, bar).
        let close_ts_ms = bar.close_ts.unix_millis();
        position_curve.push((
            close_ts_ms,
            position_book
                .get(&bar.symbol)
                .copied()
                .unwrap_or(Decimal::ZERO),
            bar.symbol.clone(),
        ));
    }

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
        "pairs backtest complete"
    );

    let pair_trades: Vec<(String, usize)> = pair_trade_counts.into_iter().collect();

    Ok(PairsRunResult {
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
        config_hash_hex,
        pair_trades,
        equity_curve,
        fills: all_fills,
        bars: bars_arc,
        position_curve,
    })
}
