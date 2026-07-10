//! P4 day-1 baseline-equity-divergence e2e (ADR-0087 § D5, CLAUDE.md
//! non-negotiable). Modelled on
//! `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` /
//! `crates/strategy/tests/latency_slippage_sim_e2e.rs`.
//!
//! Runs the SAME scripted buy/sell strategy through `PaperEngine::step`
//! twice — once with `venue_filter = None` (baseline, today's behaviour)
//! and once with `Some(VenueFilterMode::LotSizeAndMinNotional)` (opt-in
//! lot-size rounding + min-notional reject) — and asserts the two terminal
//! equities provably diverge on a corpus where rounding bites (a low-price,
//! coarse-lot coin at a small budget), with the direction pinned (rounding
//! down + rejects can only reduce or hold deployed capital, never increase
//! it). A negative control on a high-price major at the €200 golden-path
//! scale asserts the mode is comparatively inert there.
//!
//! # Forensic gate (the v3-vol-overlay-noop / v5-latency-slippage-sim guard)
//!
//! This is the same failure class as the 2026-05-22 vol-targeting-overlay
//! no-op: a venue-filter mode that computes a rounded qty but never applies
//! it to the `Fill` that cash/position bookkeeping reads would show **zero**
//! divergence here and fail `dogeusdt_small_budget_lot_rounding_diverges_from_baseline`.
//!
//! **Actually run BEFORE the `PaperEngine::step` wiring landed (T5):** the
//! engine carried the `venue_filter` field + `with_venue_filter_mode`
//! builder (scaffold-only — the `step` fill-qty selection did not yet
//! consult the field, unconditionally using `order.qty()`), so
//! `venue_filter = Some(..)` was structurally accepted but had no effect on
//! `fill.qty`. Verbatim captured output
//! (`cargo test -p backtest --test lot_realism_divergence_end_to_end -- --nocapture`):
//!
//! ```text
//! thread 'dogeusdt_small_budget_lot_rounding_diverges_from_baseline' panicked at
//! crates/backtest/tests/lot_realism_divergence_end_to_end.rs:245:5:
//! FORENSIC GATE FAIL — lot-size rounding is a no-op!
//! eq_baseline = 102.48212807787434490257656611
//! eq_filtered = 102.48212807787434490257656611
//! divergence  = 0.00000000000000000000000000 (0 relative)
//! required (>= 1 bp) = 0.0001
//! test dogeusdt_small_budget_lot_rounding_diverges_from_baseline ... FAILED
//! test btcusdt_major_at_200_is_negative_control ... FAILED
//! test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
//! ```
//!
//! **After the fix** (`step` rounds `qty` down via `cost::venue_filter_for`
//! before constructing the `Fill`, D1): both assertions pass. Verbatim
//! captured output:
//!
//! ```text
//! dogeusdt_small_budget: eq_baseline=102.48212807787434490257656611 \
//!   eq_filtered=102.42370740160 relative_divergence=0.0005700572126093251165474053 \
//!   skipped_min_notional(filtered)=0
//! test dogeusdt_small_budget_lot_rounding_diverges_from_baseline ... ok
//! btcusdt_major_at_200: eq_baseline=200.83200137031358596842456214 \
//!   eq_filtered=200.8277912739200 relative_divergence=0.0000209632746019544118024876 \
//!   doge_relative_divergence=0.0005700572126093251165474053
//! test btcusdt_major_at_200_is_negative_control ... ok
//! test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
//! ```
//!
//! DOGE diverges by ~57 bps (5.7×the 1bp gate) via genuine floor-rounding
//! (`skipped_min_notional == 0` — no outright rejects at this budget); the
//! BTC-major-€200 negative control diverges by ~0.21 bps — ~27× smaller than
//! the DOGE corpus and comfortably inside the golden-path "near-inert" bound.
//!
//! # Cross-references
//!
//! - ADR-0087 § D5 (day-1 divergence e2e), § D1 (the `PaperEngine::step` seam).
//! - `spec/v3/advisor-lot-realism/{feature.md,tasks.md}` — T6.

use backtest::MatchingEngine;
use backtest::PaperEngine;
use backtest::cli_types::{BacktestState, VenueFilterMode};
use backtest::paper::{MatchConfig, SimFilterStats};
use risk::{FixedFractionSizer, size_and_validate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    Bar, Money, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, StrategyId, Symbol,
    TimeInForce, Timeframe, Timestamp, Usdt, Venue,
};

// ── Helper builders ───────────────────────────────────────────────────────────

/// Build a minimal 1h bar at a fixed close price (open == high == low ==
/// close — sizing/fill math only reads `close` on this path).
fn make_bar(symbol: &Symbol, idx: i64, close: Decimal) -> Bar {
    let ts = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::hours(idx));
    Bar {
        symbol: symbol.clone(),
        tf: Timeframe::OneHour,
        open_ts: ts,
        close_ts: ts,
        local_recv_ts: ts,
        venue: Venue::Binance,
        open: Price::new(close).expect("positive close"),
        high: Price::new(close).expect("positive close"),
        low: Price::new(close).expect("positive close"),
        close: Price::new(close).expect("positive close"),
        volume: Quantity::new(dec!(1.0)).expect("positive volume"),
        trade_count: 1,
    }
}

/// DOGEUSDT bars: a monotonic uptrend from €0.30 to ~€0.49 over 48 hourly
/// bars — a low-price, coarse-lot (`step_size = 1`) coin at the small end
/// of the ADR-0087 § D5 €50-200 budget corpus.
fn doge_bars() -> Vec<Bar> {
    let symbol = Symbol::new("DOGEUSDT");
    (0..48i64)
        .map(|i| make_bar(&symbol, i, dec!(0.30) + Decimal::from(i) * dec!(0.004)))
        .collect()
}

/// BTCUSDT bars: a mild uptrend from $50,000 — the €200 golden-path major
/// negative control (ADR-0087 § D4).
fn btc_bars() -> Vec<Bar> {
    let symbol = Symbol::new("BTCUSDT");
    (0..24i64)
        .map(|i| make_bar(&symbol, i, dec!(50_000) + Decimal::from(i) * dec!(200)))
        .collect()
}

/// Run a scripted round-trip strategy through `PaperEngine`: buy the full
/// `fixed_fraction(fraction)` clip whenever flat (every `cycle_len` bars),
/// sell the entire position `cycle_len/2` bars later. Cash/position
/// bookkeeping mirrors `crates/backtest/src/engine.rs`'s bake-off loop
/// exactly (`BacktestState::apply_buy` / `apply_sell` + a `Position` handle
/// fed to `Order::new` for the risk checks) — both legs (buy AND the
/// direct-qty sell/close leg) are exercised, matching ADR-0087 § D1's "both
/// legs" seam claim.
///
/// The trade **schedule** is bar-index-driven (not qty/fill-driven), so the
/// baseline and filtered runs always trade on the exact same bars — any
/// terminal-equity difference is attributable ONLY to the venue-filter
/// mode's effect on `qty`, never to a divergent trade count/timing.
async fn run_scripted(
    symbol: &Symbol,
    bars: &[Bar],
    initial_capital: Decimal,
    fraction: Decimal,
    cycle_len: usize,
    venue_filter: Option<VenueFilterMode>,
) -> (Decimal, SimFilterStats) {
    let sizer = FixedFractionSizer::new(fraction);
    // Loose caps: the exposure/notional gates are not what this test is
    // exercising — the venue-filter seam inside `PaperEngine::step` is.
    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(1.0),
        price_sanity_band: dec!(0.5),
        portfolio_exposure_cap: None,
    };
    let strategy_id = StrategyId::new("lot_realism_e2e");

    let mut engine =
        PaperEngine::with_default_seed(MatchConfig::default()).with_venue_filter_mode(venue_filter);
    let mut state = BacktestState::new(initial_capital);
    let mut position = Position::empty(symbol.clone());

    for (i, bar) in bars.iter().enumerate() {
        let mark = bar.close;
        let mut orders = Vec::new();

        if i % cycle_len == 0 && position.base_qty <= Decimal::ZERO {
            let equity_money: Money<Usdt> = Money::from_decimal(state.equity(mark.get()));
            if let Ok(order) = size_and_validate(
                &sizer,
                strategy_id.clone(),
                symbol.clone(),
                Side::Buy,
                equity_money,
                mark,
                &position,
                &risk_limits,
            ) {
                orders.push(order);
            }
        } else if i % cycle_len == cycle_len / 2 && position.base_qty > Decimal::ZERO {
            // Sell/close leg: built directly from `position.base_qty`, NOT
            // via the sizer (ADR-0087 grounding: the sizer is bypassed on
            // this leg on both production paths).
            if let Ok(q) = Quantity::new(position.base_qty) {
                let equity_now = state.equity(mark.get());
                if let Ok(order) = Order::new(
                    strategy_id.clone(),
                    symbol.clone(),
                    Side::Sell,
                    q,
                    OrderKind::Market,
                    TimeInForce::Ioc,
                    &position,
                    mark,
                    &risk_limits,
                    equity_now,
                ) {
                    orders.push(order);
                }
            }
        }

        if !orders.is_empty() {
            let fills = engine
                .step(bar, orders)
                .await
                .expect("step must not error for admissible sized orders");
            for fill in &fills {
                match fill.side {
                    Side::Buy => {
                        state.apply_buy(fill.qty.get(), fill.price.get(), fill.fee.amount());
                        position.base_qty += fill.qty.get();
                    }
                    Side::Sell => {
                        state.apply_sell(
                            fill.qty.get(),
                            fill.price.get(),
                            fill.fee.amount(),
                            false,
                        );
                        position.base_qty -= fill.qty.get();
                        if position.base_qty < Decimal::ZERO {
                            position.base_qty = Decimal::ZERO;
                        }
                    }
                }
            }
        }

        let eq = state.equity(mark.get());
        state.update_drawdown(eq);
        state.equity_curve.push(eq);
    }

    let final_equity = *state
        .equity_curve
        .last()
        .expect("equity_curve always has >= 1 entry (seeded with initial_capital)");
    (final_equity, engine.sim_filter_stats())
}

// ── T6 — the day-1 divergence gate ─────────────────────────────────────────────

/// DOGEUSDT, €100 budget, `fixed_fraction(0.1)`: whole-DOGE flooring on
/// every buy leg must provably shave terminal equity vs the un-rounded
/// baseline. €100 (not €50) is deliberate: at this budget each ~€10 clip
/// clears the ~5 USDT min-notional floor comfortably (this test demonstrates
/// genuine floor-ROUNDING, i.e. ADR-0087 § D4's "(a) lot rounding shaves a
/// few sats off every clip" — the outright-reject path "(b)" is exercised
/// separately by `crates/backtest/src/paper.rs`'s
/// `venue_filter_rejects_sub_min_notional_order_no_fill_no_error`).
#[tokio::test]
async fn dogeusdt_small_budget_lot_rounding_diverges_from_baseline() {
    let symbol = Symbol::new("DOGEUSDT");
    let bars = doge_bars();
    let initial_capital = dec!(100);
    let fraction = dec!(0.1);

    let (eq_baseline, stats_baseline) =
        run_scripted(&symbol, &bars, initial_capital, fraction, 6, None).await;
    let (eq_filtered, stats_filtered) = run_scripted(
        &symbol,
        &bars,
        initial_capital,
        fraction,
        6,
        Some(VenueFilterMode::LotSizeAndMinNotional),
    )
    .await;

    let divergence = (eq_baseline - eq_filtered).abs();
    let relative = divergence / eq_baseline;

    assert!(
        relative >= dec!(0.0001),
        "FORENSIC GATE FAIL — lot-size rounding is a no-op!\n\
         eq_baseline = {eq_baseline}\n\
         eq_filtered = {eq_filtered}\n\
         divergence  = {divergence} ({relative} relative)\n\
         required (>= 1 bp) = 0.0001\n\
         \n\
         This is the ADR-0087 equivalent of the v3-vol-overlay-noop-fix /\n\
         v5-latency-slippage-sim 2026-05-22 failure class: the venue-filter\n\
         mode must round `fill.qty` DOWN inside `PaperEngine::step` before the\n\
         `Fill` is constructed — check crates/backtest/src/paper.rs `step`.\n\
         Pattern reference: crates/strategy/tests/vol_targeting_overlay_end_to_end.rs"
    );
    assert!(
        eq_filtered <= eq_baseline,
        "direction violated: filtered equity ({eq_filtered}) must be <= baseline \
         ({eq_baseline}) — rounding down + rejects can only reduce or hold deployed \
         capital, never increase it"
    );
    assert_eq!(
        stats_baseline.skipped_min_notional, 0,
        "baseline (venue_filter=None) must never skip orders"
    );
    // Informational: this scenario is sized so floor-rounding (not outright
    // min-notional rejects) drives the divergence; a zero-or-small skip
    // count here is expected and is not itself a pass/fail signal.
    eprintln!(
        "dogeusdt_small_budget: eq_baseline={eq_baseline} eq_filtered={eq_filtered} \
         relative_divergence={relative} skipped_min_notional(filtered)={}",
        stats_filtered.skipped_min_notional
    );
}

/// Negative control (ADR-0087 § D4 golden scenario): BTCUSDT, €200 budget,
/// `fixed_fraction(0.1)` — at this scale the venue-filter mode must be
/// (comparatively) inert: no rejects, and terminal-equity divergence is far
/// below the DOGE small-budget divergence.
#[tokio::test]
async fn btcusdt_major_at_200_is_negative_control() {
    let doge_symbol = Symbol::new("DOGEUSDT");
    let doge_bars = doge_bars();
    let (doge_baseline, _) =
        run_scripted(&doge_symbol, &doge_bars, dec!(100), dec!(0.1), 6, None).await;
    let (doge_filtered, _) = run_scripted(
        &doge_symbol,
        &doge_bars,
        dec!(100),
        dec!(0.1),
        6,
        Some(VenueFilterMode::LotSizeAndMinNotional),
    )
    .await;
    let doge_relative = (doge_baseline - doge_filtered).abs() / doge_baseline;

    let symbol = Symbol::new("BTCUSDT");
    let bars = btc_bars();
    let initial_capital = dec!(200);
    let fraction = dec!(0.1);

    let (eq_baseline, _stats_baseline) =
        run_scripted(&symbol, &bars, initial_capital, fraction, 6, None).await;
    let (eq_filtered, stats_filtered) = run_scripted(
        &symbol,
        &bars,
        initial_capital,
        fraction,
        6,
        Some(VenueFilterMode::LotSizeAndMinNotional),
    )
    .await;

    let divergence = (eq_baseline - eq_filtered).abs();
    let relative = divergence / eq_baseline;

    assert!(
        relative < dec!(0.005),
        "negative control violated: BTC-major-at-€200 diverged by {relative} \
         (eq_baseline={eq_baseline}, eq_filtered={eq_filtered}) — expected the \
         venue-filter mode to be near-inert at this scale (ADR-0087 § D4 golden \
         scenario, < 50 bps)"
    );
    assert!(
        relative < doge_relative,
        "the €200-major negative control ({relative}) must diverge far less than \
         the DOGE small-budget corpus ({doge_relative}) — same mechanism, \
         comparatively quiet at the golden-path scale"
    );
    assert_eq!(
        stats_filtered.skipped_min_notional, 0,
        "BTC clips at €200/10% (~€20 notional) are far above the ~5 USDT \
         min-notional floor — must never reject"
    );
    eprintln!(
        "btcusdt_major_at_200: eq_baseline={eq_baseline} eq_filtered={eq_filtered} \
         relative_divergence={relative} doge_relative_divergence={doge_relative}"
    );
}
