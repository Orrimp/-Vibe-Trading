//! v1 cross-sectional momentum scenario execution — Phase B T-D-N3.
//!
//! Extracted from `main.rs::run_momentum_backtest` @774. Behaviour-preserving:
//! same seed derivation (`sym_seed = seed.wrapping_add(idx * 0x9E3779B9)`),
//! same merge order, same fill logic.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::{
    Bar, FillView, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol,
    TimeInForce, Timeframe, Timestamp, Venue,
};

use crate::cli_types::MomentumScenarioInput;
use crate::scenarios::sim::sim_slippage_cost;

// ── Result struct ─────────────────────────────────────────────────────────────

/// Result of the v1 multi-symbol cross-sectional momentum backtest.
pub struct MomentumRunResult {
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
    /// Per-bar equity curve (`[initial_capital, equity_after_bar_0, …]`).
    /// Populated by the engine dispatch path for `RunReport.equity_series`.
    /// The CLI report path does not require this field (report uses aggregate stats).
    pub equity_curve: Vec<Decimal>,
    /// All fills produced during the run, in bar order.
    /// Populated for `RunReport.fills` so the Lab UI can render buy/sell triangle markers.
    pub fills: Vec<FillView>,
    /// All bars from the run (Arc-shared to avoid copying).
    /// Populated for `RunReport.bars` so the Lab chart can anchor fill timestamps.
    pub bars: Arc<Vec<Bar>>,
    /// lab-polish-round-2 R1 — per-(symbol,bar) position entries.
    /// Format: `(bar.close_ts unix_millis, signed_qty)`.
    /// For cross-sectional runs emits one entry per (symbol, bar) — the Lab
    /// UI filters to the active symbol via `position_curve_for_symbol`.
    /// NOT written to Markdown reports — anchor-additive.
    pub position_curve: Vec<(i64, Decimal, trading_core::Symbol)>,
}

// ── Shared price-list helpers (used also by pairs + tcn_overlay) ──────────────

/// Universe symbol list and their start prices for the top-10 scenario.
/// Verbatim copy of `main.rs::top10_symbols_with_prices` @755.
#[must_use]
pub fn top10_symbols_with_prices() -> Vec<(Symbol, Decimal)> {
    vec![
        (Symbol::new("ADAUSDT"), dec!(0.25)),
        (Symbol::new("AVAXUSDT"), dec!(11.00)),
        (Symbol::new("BNBUSDT"), dec!(240.00)),
        (Symbol::new("BTCUSDT"), dec!(16_500.00)),
        (Symbol::new("DOGEUSDT"), dec!(0.07)),
        (Symbol::new("DOTUSDT"), dec!(4.50)),
        (Symbol::new("ETHUSDT"), dec!(1_200.00)),
        (Symbol::new("LINKUSDT"), dec!(6.00)),
        (Symbol::new("SOLUSDT"), dec!(10.00)),
        (Symbol::new("XRPUSDT"), dec!(0.34)),
    ]
}

/// 4-symbol universe for the v1.5a mean-reversion pairs scenario.
/// Verbatim copy of `main.rs::pairs_symbols_with_prices` @745.
#[must_use]
pub fn pairs_symbols_with_prices() -> Vec<(Symbol, Decimal)> {
    vec![
        (Symbol::new("BNBUSDT"), dec!(240.00)),
        (Symbol::new("BTCUSDT"), dec!(16_500.00)),
        (Symbol::new("ETHUSDT"), dec!(1_200.00)),
        (Symbol::new("SOLUSDT"), dec!(10.00)),
    ]
}

/// Generate synthetic hourly bars for a single symbol.
/// Verbatim copy of `main.rs::synthetic_bars_hourly` @653.
// Float arithmetic is required for the GBM price simulation (log-normal return).
// Per ADR-0003: Decimal for money/price/qty; f64 for statistical simulation.
#[allow(clippy::float_arithmetic)]
// Bar index cast: `i` is bounded by `count` (≤8760 hours/year) — cannot wrap.
#[allow(clippy::cast_possible_wrap)]
#[must_use]
pub fn synthetic_bars_hourly(
    symbol: &Symbol,
    count: usize,
    seed: u64,
    start_price: Decimal,
    start_year: i32,
) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::seed_from_u64(seed);

    let epoch_base = {
        let date = time::Date::from_calendar_date(start_year, time::Month::January, 1)
            .unwrap_or_else(|_| {
                time::Date::from_calendar_date(2023, time::Month::January, 1)
                    .unwrap_or_else(|e| unreachable!("2023-01-01 is always valid: {e}"))
            });
        OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    let mut bars = Vec::with_capacity(count);
    let per_hour_vol: f64 = 0.012;
    let per_hour_drift: f64 = 0.000_03;
    let mut close: f64 = start_price.to_string().parse::<f64>().unwrap_or(30_000.0);

    for i in 0..count {
        let u1: f64 = rng.random::<f64>().max(1e-10_f64);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos();
        let ret = per_hour_drift + per_hour_vol * z;
        let next = (close * (1.0 + ret)).clamp(0.01_f64, 10_000_000.0_f64);

        let intra_vol = close * 0.002_f64;
        let noise1: f64 = rng.random::<f64>() * intra_vol;
        let noise2: f64 = rng.random::<f64>() * intra_vol;

        let open = close;
        let high = open.max(next) + noise1;
        let low = (open.min(next) - noise2).max(0.01_f64);
        let vol_base: f64 = rng.random::<f64>() * 500.0_f64 + 10.0_f64;

        let open_ts = Timestamp::new(epoch_base + time::Duration::hours(i as i64));
        let close_ts = Timestamp::new(
            epoch_base + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1),
        );

        let to_dec =
            |v: f64| -> Decimal { Decimal::try_from(v.max(0.01_f64)).unwrap_or(dec!(0.01)) };
        let price_or_one = |v: f64| -> Price {
            Price::new(to_dec(v)).unwrap_or_else(|_| {
                Price::new(dec!(1)).unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            })
        };

        bars.push(Bar {
            symbol: symbol.clone(),
            tf: Timeframe::OneHour,
            open_ts,
            close_ts,
            open: price_or_one(open),
            high: price_or_one(high.max(open).max(next)),
            low: price_or_one(low.min(open).min(next).max(0.01)),
            close: price_or_one(next),
            volume: Quantity::new(to_dec(vol_base)).unwrap_or_else(|_| {
                Quantity::new(dec!(1))
                    .unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            }),
            trade_count: rng.random_range(100_u32..5000_u32),
            local_recv_ts: close_ts,
            venue: Venue::Binance,
        });

        close = next;
    }

    bars
}

// ── Run function ──────────────────────────────────────────────────────────────

/// Run the v1 multi-symbol cross-sectional momentum backtest.
///
/// Extracted from `main.rs::run_momentum_backtest` @774. Behaviour-preserving.
///
/// # Errors
///
/// Returns `Err` if the strategy config file cannot be loaded or is malformed.
///
/// Bug #63 (2026-05-25): now threads `cancel_rx` + `progress_tx` into the
/// bar loop at the standard 128-bar poll boundary so the Lab Stop button
/// works and the progress bar updates.
#[allow(clippy::too_many_lines)]
pub async fn run(
    input: &MomentumScenarioInput,
    seed: u64,
    cancel_rx: crate::cancel::RunCancelReceiver,
    progress_tx: crate::progress::ProgressSender,
) -> Result<MomentumRunResult> {
    use crate::engine::MatchingEngine as _;
    use strategy::Strategy as _;

    let start_instant = Instant::now();

    // Load strategy config. Bug #56 — resolve workspace-relative so the
    // Lab cockpit launched from any CWD can find the config. The
    // ORIGINAL rel_path is used for source identifiers + error messages
    // so anchored Markdown report bytes stay byte-identical.
    let rel_path = PathBuf::from(format!("config/strategies/{}.toml", input.config_id));
    let toml_path = crate::paths::resolve_workspace_path(&rel_path);
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", rel_path.display()))?;
    let universe_list: Vec<String> = cfg.universe.iter().map(ToString::to_string).collect();
    let strategy_id_str = cfg.id.to_string();

    let mut momentum = strategy::MomentumStrategy::from_config(
        cfg,
        smol_str::SmolStr::new(rel_path.to_string_lossy()),
    );
    let config_hash_hex = {
        use std::fmt::Write as _;
        momentum.hash.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    // Generate synthetic bars for each universe symbol, or use pre-loaded real bars.
    let symbols_prices = top10_symbols_with_prices();
    let (merged_bars, bar_count) = if let Some(real_bars) = input.bars_override.clone() {
        // v3.0.0-volatility-rebaseline: real Binance data pre-loaded by caller.
        let n = real_bars.len();
        tracing::info!(
            bar_count = n,
            "momentum realdata backtest — using pre-loaded real bars"
        );
        (real_bars, n)
    } else {
        // Each symbol gets a unique seed derived from the master seed + index.
        // Sensitive line — must be preserved verbatim (per T-D-N3 risk note).
        // SAFETY: idx is at most 9 (10-symbol universe); cast to u64 is safe.
        #[allow(clippy::cast_possible_wrap)]
        let bars_by_symbol: Vec<Vec<Bar>> = symbols_prices
            .iter()
            .enumerate()
            .map(|(idx, (sym, start_price))| {
                let sym_seed = seed.wrapping_add(idx as u64 * 0x9E37_79B9);
                // For 2024 scenario, scale start prices up.
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
            "merged synthetic bars for momentum backtest"
        );
        (merged, n)
    };

    // ── Paper matching engine ─────────────────────────────────────────────────
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

    // F3 — collect fills for `MomentumRunResult.fills`.
    let mut all_fills: Vec<FillView> = Vec::new();
    // lab-polish-round-2 R1 — per-(symbol,bar) position entries for the
    // position-curve widget.  Format: (close_ts_ms, qty, symbol).
    let mut position_curve: Vec<(i64, Decimal, trading_core::Symbol)> = Vec::new();
    // Preserve bars in an Arc BEFORE the loop so the UI Lab chart can anchor
    // fill triangle markers against the run's own time window (R5.2 pattern).
    let bars_arc: Arc<Vec<Bar>> = Arc::new(merged_bars);

    let total_bars = bars_arc.len();
    for (bar_idx, bar) in bars_arc.iter().enumerate() {
        // Bug #63 — cancel + progress poll boundary every 128 bars.
        // CLI passes RunCancelReceiver::never_cancelled() + ProgressSender::disabled()
        // so anchored runs see no behaviour change. Lab passes real handles.
        //
        // Bug #64 — also force-emit at the FINAL bar so short cross-sectional
        // runs (Yahoo daily, custom narrow ranges) visibly reach 100% before
        // completion. Without this, a sub-128-bar run emits only at bar 0.
        if bar_idx.trailing_zeros() >= 7 || bar_idx == total_bars.saturating_sub(1) {
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

        let signals = momentum.on_bar(bar);

        for sig in &signals {
            let Some(&mark) = mark_prices.get(&sig.symbol) else {
                continue;
            };
            if mark <= Decimal::ZERO {
                continue;
            }

            // Compute current equity for sizing.
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
                            // v5-latency-slippage-sim: apply additional simulated slippage
                            // on top of the PaperEngine's spread model. At zero bps this
                            // is a noop (byte-identical to pre-feature code).
                            let sim_slip_cost = sim_slippage_cost(
                                fill.qty.get(),
                                fill.price.get(),
                                Side::Buy,
                                &input.latency_slippage_sim,
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
                        && let Ok(fills) = engine.step(bar, vec![ord]).await
                    {
                        for fill in fills {
                            let notional_fill = fill.qty.get() * fill.price.get();
                            // v5-latency-slippage-sim: apply additional simulated slippage.
                            let sim_slip_cost = sim_slippage_cost(
                                fill.qty.get(),
                                fill.price.get(),
                                Side::Sell,
                                &input.latency_slippage_sim,
                            );
                            cash += notional_fill - fill.fee.amount() - sim_slip_cost;
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
        // Each symbol in the universe gets a (close_ts_ms, qty, sym) row so
        // the Lab UI can filter to the active pair and render its curve.
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
        "momentum backtest complete"
    );

    Ok(MomentumRunResult {
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
        equity_curve,
        fills: all_fills,
        bars: bars_arc,
        position_curve,
    })
}

// ── v5-latency-slippage-sim helpers ──────────────────────────────────────────
//
// `sim_slippage_cost` has been lifted to `crates/backtest/src/scenarios/sim.rs`
// (ADR-0047 D2 / anchor-additive per ADR-0038 § D6.a). This module imports it
// from the shared location above. The function body is byte-identical.
//
// grep gate: `grep -r "fn sim_slippage_cost" crates/backtest/src` → 1 line only.
