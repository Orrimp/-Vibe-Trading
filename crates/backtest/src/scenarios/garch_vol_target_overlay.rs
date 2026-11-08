//! v3.0.0-volatility GARCH vol-targeting overlay momentum — Wave D T-D-N22.
//!
//! Wraps the v1 cross-sectional momentum strategy with a GARCH(1,1)
//! vol-targeting scaler (`VolTargetingOverlay`) using the anchored
//! GARCH BS-1 checkpoint.
//!
//! This scenario requires `--features realdata` for real-data bar loading.
//! The GARCH overlay itself is always available — unlike the TCN/PatchTST
//! scenarios it does NOT require `--features candle` because the GARCH
//! parameters are loaded from a plain JSON checkpoint.
//!
//! ## Key differences from TCN overlay weights
//!
//! | Attribute          | TCN overlay weights       | GARCH vol-target overlay              |
//! |--------------------|---------------------------|---------------------------------------|
//! | model              | TCN neural net            | GARCH(1,1) recurrence                |
//! | forecaster output  | direction (+1/0/-1)       | `sigma_hat` (predicted vol)           |
//! | overlay action     | confidence-weighted scale | `clamp(target_vol / σ̂, [0.5, 2.0])` |
//! | `--features candle`| required                  | NOT required                          |
//!
//! ## Cross-references
//!
//! - ADR-0038 § D5 — strategy-side composition lock.
//! - `crates/strategy/src/vol_targeting_overlay.rs` — the strategy impl.
//! - `spec/v1/v3-volatility-forecaster/decomp.md § T-D-N22` — task spec.

use anyhow::Result;

use crate::scenarios::sim::sim_slippage_cost;
use crate::scenarios::tcn_overlay::TcnOverlayRunResult;
use risk::{resulting_gross_fraction, would_breach_gross_cap};

// ── GARCH checkpoint loader (inline) ─────────────────────────────────────────

/// Minimal deserialisation shape for the GARCH BS-1 checkpoint.
#[derive(serde::Deserialize)]
struct GarchCheckpoint {
    params: std::collections::BTreeMap<String, GarchEntry>,
}

#[derive(serde::Deserialize)]
struct GarchEntry {
    omega: f64,
    alpha: f64,
    beta: f64,
    unconditional_var: f64,
}

/// Load GARCH params from the anchored BS-1 checkpoint path.
///
/// The checkpoint lives at
/// `crates/forecast/checkpoints/anchors/garch-bs1-<hash>.json`.
/// At runtime the backtest binary is invoked from the workspace root,
/// so the relative path resolves correctly.
///
/// # Errors
///
/// Returns `Err` if the file cannot be read or the JSON is malformed.
fn load_garch_checkpoint()
-> anyhow::Result<std::collections::BTreeMap<String, strategy::GarchParams>> {
    use std::path::PathBuf;

    // Anchored filename — hash locked at v3.0.0-volatility T-D-N11.
    const CHECKPOINT_FILENAME: &str =
        "garch-bs1-991324772ba077355731c2f551e3412430070b76468f6044261161a9160c0c71.json";

    let path = PathBuf::from("crates/forecast/checkpoints/anchors").join(CHECKPOINT_FILENAME);
    let json = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("read GARCH checkpoint {}: {e}", path.display()))?;
    let ck: GarchCheckpoint = serde_json::from_str(&json)
        .map_err(|e| anyhow::anyhow!("parse GARCH checkpoint {}: {e}", path.display()))?;

    Ok(ck
        .params
        .into_iter()
        .map(|(sym, p)| {
            (
                sym,
                strategy::GarchParams {
                    omega: p.omega,
                    alpha: p.alpha,
                    beta: p.beta,
                    unconditional_var: p.unconditional_var,
                },
            )
        })
        .collect())
}

// ── Run function ──────────────────────────────────────────────────────────────

/// Run the v3.0.0 GARCH vol-targeting overlay momentum backtest.
///
/// Mirrors `tcn_overlay_weights::run` but wraps with `VolTargetingOverlay`
/// instead of the TCN overlay. GARCH parameters are loaded from the anchored
/// BS-1 checkpoint.
///
/// # Errors
///
/// Returns `Err` if:
/// - The base momentum config file cannot be loaded.
/// - The GARCH checkpoint cannot be read or parsed.
/// - Bar loading (real data) fails (feature-gated).
#[allow(clippy::too_many_lines)]
#[allow(unused_variables)]
#[allow(clippy::unused_async)]
pub async fn run(
    input: crate::cli_types::TcnScenarioInput,
    seed: u64,
) -> Result<TcnOverlayRunResult> {
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

    // ── Load base momentum config ──────────────────────────────────────────────

    let base_config_id = "top10_momentum_h1";
    // Bug #56 — resolve workspace-relative; rel_path stored for anchor identity.
    let rel_path = PathBuf::from(format!("config/strategies/{base_config_id}.toml"));
    let toml_path = crate::paths::resolve_workspace_path(&rel_path);
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config: {}", rel_path.display()))?;
    let universe_list: Vec<String> = cfg.universe.iter().map(ToString::to_string).collect();
    let strategy_id_str = format!("garch_vol_target_overlay_momentum/{}", input.config_id);

    let base = strategy::MomentumStrategy::from_config(
        cfg,
        smol_str::SmolStr::new(rel_path.to_string_lossy()),
    );

    // ── Load GARCH checkpoint ──────────────────────────────────────────────────

    let garch_params = load_garch_checkpoint()
        .with_context(|| "load GARCH BS-1 checkpoint for vol-targeting overlay")?;

    let forecaster_label = format!(
        "GARCH(1,1) vol-targeting overlay (garch-bs1, v3.0.0-volatility, {} symbols)",
        garch_params.len()
    );

    tracing::info!(
        symbols = garch_params.len(),
        "loaded GARCH BS-1 checkpoint for vol-targeting overlay"
    );

    // ── Build the vol-targeting overlay strategy ───────────────────────────────

    let config = strategy::VolTargetingConfig::default();
    let mut overlay_strategy =
        strategy::with_garch_vol_overlay_momentum(base, garch_params, config);

    // ── Load bars ─────────────────────────────────────────────────────────────

    let (merged_bars_raw, bar_count) = if let Some(real_bars) = input.bars_override {
        let n = real_bars.len();
        tracing::info!(
            bar_count = n,
            "garch-vol-target-overlay realdata backtest — using pre-loaded real bars"
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
            "merged synthetic bars for garch-vol-target-overlay backtest"
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
    // bug-log #95 (operator-ruled 2026-10-12: ENFORCE on this lane). Counted and SURFACED
    // into the report body — a refusal that leaves no trace reads exactly like a clean run,
    // which is #66's class and what #141 cost the last time a cap counter stopped one struct
    // short of the renderer.
    let mut portfolio_gross_refusals: u64 = 0;
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    // bug-log #111/#129 fix (2026-10-12, operator-ruled): the fill-side twin of
    // `mark_prices`. This lane SIZED from the per-symbol mark and FILLED against the
    // trigger bar — bug-log #67's exact shape, "a divergence between sizing and filling
    // inside ONE block" — and since the #67 guard returns Err, those cross-symbol orders
    // VANISHED with no fill, no error and no counter. Same remedy as the two lanes story
    // 1-25's AC2 fixed (`montecarlo.rs`, `threshold_sweep.rs`).
    let mut last_bar_by_symbol: std::collections::HashMap<Symbol, trading_core::Bar> =
        std::collections::HashMap::new();
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

    // ── Bar loop ───────────────────────────────────────────────────────────────

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
                    // Query the vol-targeting scale for this symbol from the overlay.
                    // The scale is cached from the most recent `on_bar` call above
                    // (ADR-0038 § D5 / § D6.b wiring-bug-fix — feature.md § R1).
                    let scale = overlay_strategy.quantity_scale(&sig.symbol);
                    // Convert f64 scale to Decimal for exact-cent compatibility
                    // (CLAUDE.md money-math rule). rust_decimal::Decimal::try_from(f64)
                    // handles NaN/Inf by returning Err; defensively floor to 1.0
                    // (treat as no-op) if conversion fails.
                    let scale_dec = Decimal::try_from(scale).unwrap_or(Decimal::ONE);
                    let notional = equity * fraction * scale_dec;
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    // ── bug-log #95: the portfolio GROSS cap, now actually enforced ──
                    //
                    // This lane declared `portfolio_exposure_cap: Some(0.50)` and nothing
                    // read it: the only enforcer is `risk::size_portfolio_target`, whose
                    // single production caller is the theta-sweep lane, and this lane builds
                    // no target vector at all. The 2026-10-12 trace found 0.50 sits INSIDE
                    // this lane's live operating range — `scale_clamp_max` is 2.0, so a
                    // 3-leg basket at `fraction` 0.10 scaled to 2.0 reaches 0.60 gross — so
                    // the declaration was both false and load-bearing.
                    //
                    // Shape: an ENTRY GATE on RESULTING exposure, which is #71's discipline
                    // (that cap was evadable by splitting precisely because it tested the
                    // INCREMENT rather than the resulting total). `position_value` is this
                    // lane's gross notional because it is long-only (`k_short = 0`), so the
                    // resulting gross fraction is `(position_value + notional) / equity`.
                    //
                    // Per-order refusal, NOT the theta lane's all-or-nothing rebalance skip:
                    // there is no rebalance batch here to skip. The order is refused, counted
                    // and logged; the book is left as it was.
                    if let Some(cap) = risk_limits.portfolio_exposure_cap
                        && equity > Decimal::ZERO
                    {
                        let resulting_gross =
                            resulting_gross_fraction(position_value, notional, equity);
                        if would_breach_gross_cap(position_value, notional, equity, cap) {
                            portfolio_gross_refusals += 1;
                            tracing::warn!(
                                symbol = %sig.symbol,
                                %resulting_gross,
                                %cap,
                                %notional,
                                "portfolio GROSS cap would be breached — Buy REFUSED and \
                                 counted (bug-log #95); the book is unchanged"
                            );
                            continue;
                        }
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
                        // A refusal is REPORTED, never dropped. The `&& let Ok(fills) = …`
                        // if-chain this replaces had no else arm, so an Err made the whole
                        // chain false and the order disappeared in silence (bug-log #129).
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
                    // Sell-to-close: full-position exit — vol-target scale does NOT apply
                    // (would leak residual exposure on regime spikes; we exit the entire
                    // open position, not a notional fraction). ADR-0038 § D6.b / T-AR-2.
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
                        // A refusal is REPORTED, never dropped. The `&& let Ok(fills) = …`
                        // if-chain this replaces had no else arm, so an Err made the whole
                        // chain false and the order disappeared in silence (bug-log #129).
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

    let stats = &overlay_strategy.stats;
    tracing::info!(
        elapsed_s = elapsed_secs,
        trades = trades,
        final_equity = %final_equity,
        bars_total = stats.bars_total,
        signals_scaled = stats.signals_scaled,
        signals_passthrough = stats.signals_passthrough,
        bars_no_model = stats.bars_no_model,
        // bug-log #174: these two were computed and rendered NOWHERE, while their
        // four siblings above were logged here. Measured in a scratch crate over
        // 600 bars: production `VolTargetingConfig::default()` pins both at 0, and
        // `p1_4_defaults()` drives them to 600 / 599 — so the ONLY configuration
        // that makes them non-zero had no way to report it.
        band_suppressed = stats.band_suppressed,
        derisk_suppressed = stats.derisk_suppressed,
        "garch-vol-target-overlay backtest complete"
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
        // Reuse TCN overlay fields for vol-targeting diagnostics.
        // dampened_signals → signals_scaled (bars where scale != 1.0)
        // passed_through_signals → signals_passthrough (bars where scale ≈ 1.0)
        // warmup_signals → bars_no_model (bars without GARCH model)
        dampened_signals: stats.signals_scaled,
        passed_through_signals: stats.signals_passthrough,
        warmup_signals: stats.bars_no_model,
        portfolio_gross_refusals,
        forecaster_label,
        equity_curve,
        fills: all_fills,
        bars: bars_arc,
        // lab-polish-round-2 R1 — position_curve not yet computed for this
        // scenario (garch overlay). Defaults to empty — anchor-additive.
        position_curve: Vec::new(),
    })
}

// ── bug-log #95: the gross-cap predicate, extracted so the arithmetic is testable ──

// The gross-cap entry gate's tests MOVED to `crates/risk/src/portfolio.rs`
// on 2026-11-08 (bug-log `#95`), together with the two helpers themselves.
//
// They were here, behind `--features realdata`, which CI does not pass — so the
// only tests of a cap that four lanes now depend on were invisible to CI
// (`#170`'s lesson). `risk` is compiled by every CI leg with no feature opt-in.
