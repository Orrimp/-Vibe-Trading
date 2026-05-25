//! Single-symbol SMA / Composed strategy bar-loop extraction (Wave D-2 / T-AR-4).
//!
//! Extracted from `main.rs:1541-1791` — strategy-setup block + bar loop + summary.
//! Behaviour-preserving: same seed derivation, same RNG draws, same loop order,
//! same fill/equity/KPI compute as the original inline block.
//!
//! # Anchor discipline
//!
//! The four legacy CLI anchors (`btc-2023-1m-sma-cross`, `btc-2023-1m-macd-trend`,
//! `btc-2023-1m-rsi-reversion`, `btc-2023-1m-bbands-mean-revert`) MUST remain
//! byte-identical after this extraction.  The CLI in `main.rs` now calls
//! `sma_composed_run::run` and feeds the pre-generated bars so the RNG-derived
//! bar stream is identical to the inlined path.
//!
//! # Fills enrichment (R5.2)
//!
//! `SmaComposedRunResult.fills` surfaces all executed `FillView` records in
//! chronological order.  The CLI report writer (`report::sma::write`) does NOT
//! include fills in the report body, so the Markdown bytes are unchanged and
//! the anchor SHAs are unaffected.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    Bar, FillView, Money, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol,
    TimeInForce, Timeframe, Timestamp, Usdt, Venue,
};

use crate::cli_types::{BacktestState, SmaComposedRunInput, StrategyMeta};

// ── Error type ────────────────────────────────────────────────────────────────

/// Error from `sma_composed_run::run`.
///
/// Separate from `anyhow::Error` so the engine dispatch can pattern-match
/// the `Cancelled` variant and convert it to `RunError::Cancelled` rather
/// than `RunError::Internal`.
#[derive(Debug)]
pub enum SmaRunError {
    /// Operator cancelled the run before the bar loop completed.
    Cancelled,
    /// Any other error (strategy load failure, I/O, etc.).
    Other(anyhow::Error),
}

impl std::fmt::Display for SmaRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "run cancelled by operator"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SmaRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Other(e) => e.source(),
            Self::Cancelled => None,
        }
    }
}

impl From<anyhow::Error> for SmaRunError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

// ── Result struct ─────────────────────────────────────────────────────────────

/// Result of a single-symbol SMA/Composed strategy backtest run.
///
/// Mirrors `MomentumRunResult` shape.  Adds `fills` (R5.2 enrichment) and
/// `strategy_meta` so the engine dispatch can build a `RunReport` without
/// disk-hopping for the TOML hash.
pub struct SmaComposedRunResult {
    pub trades: usize,
    pub buys: usize,
    pub sells: usize,
    pub total_fees: Decimal,
    pub final_equity: Decimal,
    pub initial_equity: Decimal,
    pub max_drawdown: Decimal,
    pub bar_count: usize,
    pub elapsed_secs: f64,
    /// Per-bar equity curve (`[initial_capital, equity_after_bar_0, …]`).
    pub equity_curve: Vec<Decimal>,
    /// All executed fills in chronological order (R5.2).
    pub fills: Vec<FillView>,
    /// The bars used for this run, in chronological order.
    ///
    /// Wrapped in `Arc` for cheap cloning into `RunReport` / `RunSummary`
    /// without copying the potentially-large bar vector. The UI Lab screen
    /// uses this to anchor fill markers on the chart canvas even when the
    /// live `chart_buffer` is empty (e.g. Yahoo/Synthetic runs in 2023).
    pub bars: Arc<Vec<Bar>>,
    /// Strategy metadata populated during run (id, kind, hash, signal, notes).
    pub strategy_meta: StrategyMeta,
    /// Scenario bar state (for the CLI report writer).
    pub state: BacktestState,
    /// lab-polish-round-2 R1 — cumulative base-asset position over time.
    /// Each entry is `(bar.close_ts unix_millis, signed_qty)`.
    /// For the single-symbol case there is one entry per bar.
    /// Positive = long, zero = flat. Used by the Lab position-curve widget.
    /// NOT written to Markdown reports — anchor-additive.
    pub position_curve: Vec<(i64, Decimal)>,
}

// ── Synthetic minute-bar generator ───────────────────────────────────────────

/// Generate synthetic minute-resolution OHLCV bars.
///
/// Verbatim copy of `main.rs::synthetic_bars` @637.  Keeping an identical
/// copy inside this module ensures the engine dispatch path produces
/// byte-for-byte-identical bars to the CLI path without a binary→library
/// dependency inversion.
///
/// **DO NOT** modify the RNG parameters or loop logic — any change will
/// break the 4 anchored single-symbol report SHAs.
// Float arithmetic is required for GBM price simulation (log-normal return).
// Per ADR-0003: Decimal for money/price/qty; f64 for statistical simulation.
#[allow(clippy::float_arithmetic)]
// Bar index `i` is bounded by `count` (≤525 600 minutes/year); cast is safe.
#[allow(clippy::cast_possible_wrap)]
#[must_use]
pub fn synthetic_bars_minute(
    symbol: &Symbol,
    count: usize,
    seed: u64,
    start_price: Decimal,
    start_year: i32,
) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    // F11 fix 2026-05-24 — mix symbol into RNG seed so the Lab UI's pair picker
    // produces different synthetic random walks per pair. Without this, every
    // pair produced identical percent-return sequences (only the start_price
    // differed), making the pair picker strategically meaningless.
    //
    // Anchor-preserving contract: BTCUSDT returns offset = 0 so legacy fixed-
    // symbol anchored reports (4 single-symbol body-SHA-256s under
    // spec/v0-paper-sma/reports etc.) remain byte-identical. Other symbols get
    // a deterministic FNV-1a-64 hash of the symbol bytes XORed in.
    //
    // ADR-0038 § D6 compliance: this is behavior-preserving for BTCUSDT (the
    // only anchored symbol); strictly additive variation for other symbols.
    let effective_seed = seed ^ symbol_seed_offset(symbol);
    let mut rng = ChaCha20Rng::seed_from_u64(effective_seed);
    let mut bars = Vec::with_capacity(count);

    let per_min_vol: f64 = 0.001_10;
    let per_min_drift: f64 = 0.000_001_9;

    let epoch_base = {
        let date = time::Date::from_calendar_date(start_year, time::Month::January, 1)
            .unwrap_or_else(|_| {
                // 2023-01-01 is always valid; unreachable branch
                time::Date::from_calendar_date(2023, time::Month::January, 1)
                    .unwrap_or_else(|e| unreachable!("2023-01-01 is always valid: {e}"))
            });
        OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    let mut close: f64 = start_price.to_string().parse::<f64>().unwrap_or(30_000.0);

    for i in 0..count {
        // Box-Muller for Gaussian noise
        let u1: f64 = rng.random::<f64>().max(1e-10_f64);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos();
        let ret = per_min_drift + per_min_vol * z;
        let next = (close * (1.0 + ret)).clamp(1_000.0_f64, 500_000.0_f64);

        let intra_vol = close * 0.000_5_f64;
        let noise1: f64 = rng.random::<f64>() * intra_vol;
        let noise2: f64 = rng.random::<f64>() * intra_vol;

        let open = close;
        let high = open.max(next) + noise1;
        let low = (open.min(next) - noise2).max(0.01_f64);
        let vol_btc: f64 = rng.random::<f64>() * 50.0_f64 + 1.0_f64;

        let open_ts = Timestamp::new(epoch_base + time::Duration::minutes(i as i64));
        let close_ts = Timestamp::new(
            epoch_base + time::Duration::minutes(i as i64 + 1) - time::Duration::seconds(1),
        );

        let to_dec =
            |v: f64| -> Decimal { Decimal::try_from(v.max(0.01_f64)).unwrap_or(dec!(0.01)) };
        let price_or_one = |v: f64| -> Price {
            Price::new(to_dec(v)).unwrap_or_else(|_| {
                // dec!(1) is always positive; this branch is unreachable
                Price::new(dec!(1)).unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            })
        };

        bars.push(Bar {
            symbol: symbol.clone(),
            tf: Timeframe::OneMinute,
            open_ts,
            close_ts,
            open: price_or_one(open),
            high: price_or_one(high.max(open).max(next)),
            low: price_or_one(low.min(open).min(next).max(0.01)),
            close: price_or_one(next),
            volume: Quantity::new(to_dec(vol_btc)).unwrap_or_else(|_| {
                // dec!(1) is always positive; this branch is unreachable
                Quantity::new(dec!(1))
                    .unwrap_or_else(|e| unreachable!("dec!(1) is always valid: {e}"))
            }),
            trade_count: rng.random_range(10_u32..500_u32),
            local_recv_ts: close_ts,
            venue: Venue::Binance,
        });

        close = next;
    }

    bars
}

/// F11 — Per-symbol seed offset for `synthetic_bars_minute`.
///
/// Returns a deterministic `u64` mixed into the `ChaCha20Rng` seed so the Lab
/// UI's pair picker produces different synthetic random walks per pair.
///
/// **Anchor-preservation contract**: returns 0 for `BTCUSDT` so the legacy
/// fixed-symbol anchored reports (4 single-symbol body-SHA-256s under
/// `spec/v0-paper-sma/reports/` etc.) stay byte-identical. All other symbols
/// receive an FNV-1a-64 hash of the symbol bytes.
///
/// ADR-0038 § D6 compliance: behavior-preserving for the only anchored
/// symbol; additive variation for everything else.
#[must_use]
pub fn symbol_seed_offset(symbol: &Symbol) -> u64 {
    if symbol.0.as_str() == "BTCUSDT" {
        return 0;
    }
    // FNV-1a 64-bit (deterministic across Rust versions; never changes).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in symbol.0.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Default start price for a symbol, used when the engine dispatch provides
/// no `start_price` override.  Matches the v0/v0.5 scenario defaults in
/// `main.rs::Scenario::from_name`.
#[must_use]
pub fn default_start_price(symbol: &Symbol) -> Decimal {
    match symbol.0.as_str() {
        "BTCUSDT" => dec!(16_500),
        "ETHUSDT" => dec!(1_200),
        "BNBUSDT" => dec!(240),
        "SOLUSDT" => dec!(10),
        "XRPUSDT" => dec!(0.34),
        "ADAUSDT" => dec!(0.25),
        "AVAXUSDT" => dec!(11),
        "DOGEUSDT" => dec!(0.07),
        "DOTUSDT" => dec!(4.50),
        "LINKUSDT" => dec!(6),
        _ => dec!(30_000), // safe fallback
    }
}

// ── Run function ──────────────────────────────────────────────────────────────

/// Run a single-symbol SMA/Composed strategy backtest.
///
/// Extracted from `main.rs:1541-1791`. Behaviour-preserving: the CLI path
/// passes pre-generated bars (same seed → identical RNG sequence) so the
/// written Markdown report body remains byte-identical to the v0/v0.5 anchor
/// hashes.
///
/// The `bars_override` field allows the CLI to pass in its pre-generated bars,
/// avoiding a second RNG traversal.  When `None`, the function generates bars
/// using `synthetic_bars_minute` with `start_price` from
/// `default_start_price(symbol)`.
///
/// # Errors
///
/// Returns `Err(SmaRunError::Other(_))` if the composed strategy TOML cannot be
/// loaded or is malformed.  Compiled-in `sma_crossover` never fails.
/// Returns `Err(SmaRunError::Cancelled)` if the cancel handle is dropped
/// before or during the run (polled at the 32/128-bar boundary per K4 mitigation).
#[allow(clippy::too_many_lines)]
pub async fn run(
    input: &SmaComposedRunInput,
    bars_override: Option<Vec<Bar>>,
    seed: u64,
    cancel_rx: crate::cancel::RunCancelReceiver,
    progress_tx: crate::progress::ProgressSender,
) -> Result<SmaComposedRunResult, SmaRunError> {
    use crate::engine::MatchingEngine as _;

    let start_instant = Instant::now();

    // ── 1. Load strategy into registry ───────────────────────────────────────
    let registry = strategy::StrategyRegistry::new();

    let strategy_meta = if input.strategy_id == "sma_crossover" {
        // lab-polish-round-2 R2 — Lab UI may override the (20, 50) defaults.
        // Anchored CLI scenarios pass None → byte-identical to pre-R2 behavior.
        let fast_len = input.sma_fast_len.unwrap_or(20usize);
        let slow_len = input.sma_slow_len.unwrap_or(50usize);
        registry.register(Box::new(strategy::SmaCrossover::new(fast_len, slow_len)));
        StrategyMeta {
            id: "sma_crossover".to_string(),
            kind: "compiled-in".to_string(),
            hash_hex: "n/a".to_string(),
            source_path: "compiled-in".to_string(),
            signal: format!("sma_crossover(fast={fast_len}, slow={slow_len})"),
            notes: format!("v0 SMA crossover: fast={fast_len}, slow={slow_len}"),
        }
    } else {
        // Bug #56 — resolve workspace-relative so the Lab cockpit launched
        // from any CWD can still load the config. The relative path is
        // preserved in `source_path` for anchor byte-identity (ADR-0038 §
        // D6 — report writer renders source_path into the Markdown body).
        let rel_path = PathBuf::from(format!("config/strategies/{}.toml", input.strategy_id));
        let toml_path = crate::paths::resolve_workspace_path(&rel_path);
        let cfg = strategy::ComposedStrategyConfig::from_file(&toml_path)
            .with_context(|| format!("load strategy config: {}", rel_path.display()))?;
        let hash_hex = {
            use std::fmt::Write as _;
            cfg.hash.iter().fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            })
        };
        let source_path = rel_path.display().to_string();
        let signal = cfg.signal_raw.to_string();
        let meta = StrategyMeta {
            id: input.strategy_id.clone(),
            kind: "composed".to_string(),
            hash_hex,
            source_path,
            signal,
            notes: format!("Composed strategy: {}", input.strategy_id),
        };
        let composed = strategy::ComposedStrategy::from_config(
            cfg,
            // Bug #56 — keep rel_path (not resolved abs path) for anchor identity.
            smol_str::SmolStr::new(rel_path.to_string_lossy()),
        );
        registry.register(Box::new(composed));
        meta
    };

    tracing::info!(
        strategy_id = %strategy_meta.id,
        strategy_kind = %strategy_meta.kind,
        "strategy resolved"
    );

    // ── 2. Generate or use provided bars ─────────────────────────────────────
    let bars = bars_override.unwrap_or_else(|| {
        let start_price = default_start_price(&input.symbol);
        synthetic_bars_minute(
            &input.symbol,
            input.bar_count,
            seed,
            start_price,
            input.start_year,
        )
    });

    let bar_count = bars.len();
    tracing::info!("running backtest loop ({bar_count} bars)");

    // Preserve bars in an Arc BEFORE the loop consumes them by iteration.
    // The UI Lab screen uses `result.bars` to anchor fill triangle markers
    // against the run's own time window (chart_buffer may be empty for
    // Yahoo/Synthetic runs whose timestamps differ from the live feed).
    let bars_arc: Arc<Vec<Bar>> = Arc::new(bars);

    // ── 3. Risk + matching engine setup ──────────────────────────────────────
    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: None,
    };
    let sizer = risk::FixedFractionSizer::new(dec!(0.10));

    let match_config = crate::paper::MatchConfig {
        slippage_bps: input.slippage_bps,
        taker_fee_bps: input.taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: crate::paper::FillPriceMode::BarClose,
    };
    let mut engine = crate::PaperEngine::new(match_config, seed);

    let mut state = BacktestState::new(input.initial_capital);
    let mut position = Position::empty(input.symbol.clone());
    let tolerance = dec!(0.01);

    // ── 4. Bar loop (verbatim extraction from main.rs:1629-1728) ─────────────

    // R5.2 — collect fills for `SmaComposedRunResult.fills`.
    let mut all_fills: Vec<FillView> = Vec::new();
    // lab-polish-round-2 R1 — position-curve: cumulative signed qty over time.
    let mut position_curve: Vec<(i64, Decimal)> = Vec::with_capacity(bar_count);

    for (bar_idx, bar) in bars_arc.iter().enumerate() {
        // R6.2 + R7.2 — cancellation + progress at the poll boundary.
        // K4 mitigation: every 32 bars during the first 128 bars (warmup),
        // then every 128 bars steady-state.
        #[allow(clippy::verbose_bit_mask)] // bitmask is more readable than trailing_zeros here
        let poll_now = if bar_idx < 128 {
            bar_idx & 0x1F == 0 // every 32 bars during warmup
        } else {
            bar_idx & 0x7F == 0 // every 128 bars steady-state
        };
        if poll_now {
            if cancel_rx.is_cancelled() {
                return Err(SmaRunError::Cancelled);
            }
            progress_tx.try_send(crate::progress::Progress {
                current_bar: bar_idx,
                total_bars: bar_count,
                elapsed_ms: u64::try_from(start_instant.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
        }

        let bar = bar.clone();
        let mark = bar.close.get();
        position.last_mark = bar.close;

        // Record pre-fill equity for sizing / drawdown reference
        let equity = state.equity(mark);

        let signals = registry.on_bar(&bar);
        let mut orders: Vec<Order> = Vec::new();

        for sig in &signals {
            let desired_side: Option<Side> = match sig.kind {
                trading_core::SignalKind::Buy if position.base_qty <= Decimal::ZERO => {
                    Some(Side::Buy)
                }
                trading_core::SignalKind::Sell if position.base_qty > Decimal::ZERO => {
                    Some(Side::Sell)
                }
                _ => None,
            };

            if let Some(side) = desired_side {
                let order_opt = match side {
                    Side::Buy => {
                        let eq_money: Money<Usdt> = Money::from_decimal(equity);
                        risk::size_and_validate(
                            &sizer,
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            side,
                            eq_money,
                            bar.close,
                            &position,
                            &risk_limits,
                        )
                        .ok()
                    }
                    Side::Sell => Quantity::new(position.base_qty)
                        .ok()
                        .filter(|q| q.get() > Decimal::ZERO)
                        .and_then(|q| {
                            Order::new(
                                sig.strategy_id.clone(),
                                sig.symbol.clone(),
                                Side::Sell,
                                q,
                                OrderKind::Market,
                                TimeInForce::Ioc,
                                &position,
                                bar.close,
                                &risk_limits,
                                equity,
                            )
                            .ok()
                        }),
                };
                if let Some(ord) = order_opt {
                    orders.push(ord);
                }
            }
        }

        if !orders.is_empty()
            && let Ok(fills) = engine.step(&bar, orders).await
        {
            for fill in &fills {
                match fill.side {
                    Side::Buy => {
                        state.apply_buy(fill.qty.get(), fill.price.get(), fill.fee.amount());
                        position.base_qty += fill.qty.get();
                        position.cost_basis = Money::from_decimal(state.position_cost);
                    }
                    Side::Sell => {
                        state.apply_sell(fill.qty.get(), fill.price.get(), fill.fee.amount());
                        position.base_qty -= fill.qty.get();
                        if position.base_qty < Decimal::ZERO {
                            position.base_qty = Decimal::ZERO;
                        }
                    }
                }

                // R5.2 — convert Fill → FillView for the result struct.
                all_fills.push(FillView {
                    symbol: fill.symbol.clone(),
                    side: fill.side,
                    price: fill.price,
                    qty: fill.qty,
                    fee: fill.fee,
                    fee_tier: fill.fee_tier,
                    venue_ts: fill.venue_ts,
                    transaction_id: smol_str::SmolStr::default(),
                });
            }
        }

        // Push post-fill equity to the equity curve
        let post_fill_equity = state.equity(mark);
        state.update_drawdown(post_fill_equity);
        state.equity_curve.push(post_fill_equity);

        // lab-polish-round-2 R1 — record (close_ts_ms, cumulative_qty) for
        // the position-curve widget.  `position.base_qty` is updated by the
        // fill loop above, so this snapshot is post-fill for this bar.
        position_curve.push((bar.close_ts.unix_millis(), position.base_qty));

        // Minute-boundary reconciliation check (every 1440 bars ≈ 1 day)
        // Invariant: cash + position_qty * mark == equity_curve.last()
        if bar_idx % 1440 == 0 {
            let recomputed = state.cash + state.position_qty * mark;
            let recorded = post_fill_equity;
            if (recomputed - recorded).abs() > tolerance {
                state.ledger_imbalance_events += 1;
                tracing::warn!(bar = bar_idx, diff = %(recomputed - recorded).abs(), "reconciliation mismatch");
            }
        }
    }

    let elapsed = start_instant.elapsed().as_secs_f64();
    let final_equity = state.equity(position.last_mark.get());

    tracing::info!(
        elapsed_s = elapsed,
        trades = state.trades,
        final_equity = %final_equity,
        imbalances = state.ledger_imbalance_events,
        "backtest complete"
    );

    Ok(SmaComposedRunResult {
        trades: state.trades,
        buys: state.buys,
        sells: state.sells,
        total_fees: state.total_fees,
        final_equity,
        initial_equity: input.initial_capital,
        max_drawdown: state.max_drawdown,
        bar_count,
        elapsed_secs: elapsed,
        equity_curve: state.equity_curve.clone(),
        fills: all_fills,
        bars: bars_arc,
        strategy_meta,
        state,
        position_curve,
    })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::Symbol;

    const TEST_SEED: u64 = 0xC0FFEE;

    /// SMA crossover run produces a deterministic result (same seed → same trades).
    #[tokio::test]
    async fn run_sma_crossover_deterministic() {
        let input = SmaComposedRunInput {
            strategy_id: "sma_crossover".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 1_440, // one day of minute bars — fast test
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
        };
        let (_handle1, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result1 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        let (_handle2, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result2 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        assert_eq!(
            result1.trades, result2.trades,
            "trades must be deterministic"
        );
        assert_eq!(
            result1.final_equity, result2.final_equity,
            "final_equity must be deterministic"
        );
        assert_eq!(result1.bar_count, 1_440);
        assert_eq!(
            result1.fills.len(),
            result1.trades,
            "each trade produces one fill"
        );
    }

    /// MACD trend run produces a deterministic result.
    ///
    /// Requires `config/strategies/btc_macd_trend.toml` — run from workspace root:
    /// ```text
    /// cargo test -p backtest --lib scenarios::sma_composed_run::tests::run_macd_trend_deterministic -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "requires config/strategies/*.toml at cwd; run from workspace root with --ignored"]
    async fn run_macd_trend_deterministic() {
        let input = SmaComposedRunInput {
            strategy_id: "btc_macd_trend".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 1_440,
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
        };
        let (_handle1, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result1 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        let (_handle2, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result2 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        assert_eq!(
            result1.trades, result2.trades,
            "trades must be deterministic"
        );
        assert_eq!(result1.final_equity, result2.final_equity);
    }

    /// RSI reversion run produces a deterministic result.
    ///
    /// Requires `config/strategies/btc_rsi_reversion.toml` — run from workspace root:
    /// ```text
    /// cargo test -p backtest --lib scenarios::sma_composed_run::tests::run_rsi_reversion_deterministic -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "requires config/strategies/*.toml at cwd; run from workspace root with --ignored"]
    async fn run_rsi_reversion_deterministic() {
        let input = SmaComposedRunInput {
            strategy_id: "btc_rsi_reversion".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 1_440,
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
        };
        let (_handle1, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result1 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        let (_handle2, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result2 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        assert_eq!(
            result1.trades, result2.trades,
            "trades must be deterministic"
        );
        assert_eq!(result1.final_equity, result2.final_equity);
    }

    /// BBands mean-revert run produces a deterministic result.
    ///
    /// Requires `config/strategies/btc_bbands_mean_revert.toml` — run from workspace root:
    /// ```text
    /// cargo test -p backtest --lib scenarios::sma_composed_run::tests::run_bbands_mean_revert_deterministic -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "requires config/strategies/*.toml at cwd; run from workspace root with --ignored"]
    async fn run_bbands_mean_revert_deterministic() {
        let input = SmaComposedRunInput {
            strategy_id: "btc_bbands_mean_revert".to_string(),
            symbol: Symbol::new("BTCUSDT"),
            start_year: 2023,
            bar_count: 1_440,
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            sma_fast_len: None,
            sma_slow_len: None,
        };
        let (_handle1, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result1 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        let (_handle2, cancel_rx) = crate::cancel::cancellation_pair();
        let progress_tx = crate::progress::ProgressSender::disabled();
        let result2 = run(&input, None, TEST_SEED, cancel_rx, progress_tx)
            .await
            .expect("run should succeed");
        assert_eq!(
            result1.trades, result2.trades,
            "trades must be deterministic"
        );
        assert_eq!(result1.final_equity, result2.final_equity);
    }
}
