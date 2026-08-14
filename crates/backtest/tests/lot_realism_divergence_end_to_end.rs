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
//! # Layer 2 — the PRODUCTION-PATH gate (bug-log #79, added 2026-08-13)
//!
//! Everything above this line exercises `PaperEngine::step` through an engine
//! **this test builds itself**. That is a seam test, and on 2026-08-12 the
//! story-1-18/3-15 review proved exactly what a seam test cannot see: the
//! §"ADVISOR-PATH GATE" test in this file passed for eight days while €200 lot
//! realism was **inert in production**. `run_scenario` threaded
//! `cfg.latency_slippage_sim` for 1 of ~15 arms, and
//! `sma_composed_run::{run, run_with_strategy}` built the engine as
//! `PaperEngine::new(match_config, seed)` — so `with_venue_filter_mode` had
//! **zero** production call sites workspace-wide.
//!
//! The `advisor_bakeoff_path_*` tests below therefore call the real entry
//! point — `backtest::engine::run_scenario`, the function `run_bakeoff` loops
//! once per arm — with the ScenarioConfig the bake-off actually builds
//! (`latency_slippage_sim: LatencySlippageSimConfig::advisor_default()`,
//! `bakeoff/mod.rs`), and assert on the RETURNED fills/equity. They go RED if
//! either link is cut:
//!
//! - cut the threading (an arm hard-codes `LatencySlippageSimConfig::default()`)
//!   → that arm stops diverging → `advisor_bakeoff_path_every_arm_diverges` red;
//! - cut the application (`PaperEngine::new(..)` without
//!   `.with_venue_filter_mode(..)`) → no arm diverges → every gate red.
//!
//! # Cross-references
//!
//! - ADR-0087 § D5 (day-1 divergence e2e), § D1 (the `PaperEngine::step` seam).
//! - `docs/dev-notes/bug-log.md` § `#79` — why layer 2 exists.
//! - `spec/v3/advisor-lot-realism/{feature.md,tasks.md}` — T6.

use backtest::MatchingEngine;
use backtest::PaperEngine;
use backtest::cancel::cancellation_pair;
use backtest::cli_types::{BacktestState, LatencySlippageSimConfig, VenueFilterMode};
use backtest::engine::{DateRange, RunReport, ScenarioConfig, ScenarioDataSource, run_scenario};
use backtest::paper::{MatchConfig, SimFilterStats};
use backtest::progress::ProgressSender;
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
    make_bar_with_volume(symbol, idx, close, dec!(1.0))
}

/// As [`make_bar`], with an explicit bar volume — the composed arms
/// (`bbands`, `vol_breakout`, `obv`) gate on `volume` vs `avg(volume, 20)`,
/// so a constant-volume corpus silently never trades them.
fn make_bar_with_volume(symbol: &Symbol, idx: i64, close: Decimal, volume: Decimal) -> Bar {
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
        volume: Quantity::new(volume).expect("positive volume"),
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

/// Seam-level companion to the production gates below: the two CONSTRUCTORS
/// (`advisor_default()` vs `Default`) must produce a measurable fill
/// difference when handed to `PaperEngine` directly.
///
/// **This test alone proves nothing about the advisor** — it builds its own
/// engine. It is kept only because it pins the two constructor values against
/// a "simplification" of `advisor_default()` back to `Self::default()`. The
/// claim "lot realism is ON for the advisor path" is owned exclusively by the
/// `advisor_bakeoff_path_*` tests further down, which call `run_scenario`.
#[tokio::test]
async fn advisor_default_constructor_diverges_from_plain_default_at_the_fill_seam() {
    let symbol = Symbol::new("DOGEUSDT");
    let bars = doge_bars();
    let initial_capital = dec!(200); // the product's headline budget
    let fraction = dec!(0.1);

    // The values under test come from the CONSTRUCTORS, not hand-written.
    let advisor_filter = LatencySlippageSimConfig::advisor_default().venue_filter;
    let plain_filter = LatencySlippageSimConfig::default().venue_filter;
    assert!(
        plain_filter.is_none(),
        "the plain Default must stay the byte-identity arm (anchored bodies depend on it)"
    );

    let (eq_plain, stats_plain) =
        run_scripted(&symbol, &bars, initial_capital, fraction, 6, plain_filter).await;
    let (eq_advisor, stats_advisor) =
        run_scripted(&symbol, &bars, initial_capital, fraction, 6, advisor_filter).await;

    let divergence = (eq_plain - eq_advisor).abs();
    let relative = divergence / eq_plain;

    assert!(
        relative >= dec!(0.0001),
        "CONSTRUCTOR SEAM FAIL — advisor_default() is behaviourally a no-op!\n\
         eq_plain(default)   = {eq_plain}\n\
         eq_advisor          = {eq_advisor}\n\
         divergence          = {divergence} ({relative} relative)\n\
         required (>= 1 bp)  = 0.0001\n\
         \n\
         Red here means advisor_default() lost its venue_filter or the filter\n\
         stopped biting at PaperEngine::step."
    );
    assert!(
        eq_advisor <= eq_plain,
        "direction violated: the realistic (advisor) path can only deploy less \
         capital than the un-filtered baseline, never more"
    );
    assert_eq!(
        stats_plain.skipped_min_notional, 0,
        "the plain default must never skip orders"
    );
    eprintln!(
        "constructor seam: eq_plain={eq_plain} eq_advisor={eq_advisor} \
         relative_divergence={relative} skipped_min_notional(advisor)={}",
        stats_advisor.skipped_min_notional
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  LAYER 2 — the PRODUCTION-PATH advisor gate (bug-log #79)
//
//  Everything below calls `backtest::engine::run_scenario` — the function
//  `run_bakeoff` loops once per arm — and asserts on the fills/equity it
//  RETURNS. Nothing below constructs a `PaperEngine`.
// ═══════════════════════════════════════════════════════════════════════════

/// Fixed non-zero ChaCha seed (`[0u8; 32]` is rejected by `run_scenario`).
const GATE_SEED: [u8; 32] = {
    let mut s = [0u8; 32];
    s[0] = 0xC0;
    s[1] = 0xFF;
    s[2] = 0xEE;
    s
};

/// The product's headline budget (PRD §13 Q5).
const GATE_CAPITAL: Decimal = dec!(200);

/// Number of bars in the production-path corpus (6 × 60-bar regime cycles).
const GATE_BARS: i64 = 360;

/// DOGEUSDT regime-cycle bars — the corpus that makes the whole bake-off field
/// trade.
///
/// DOGE is the ADR-0087 § D5 divergence corpus: `step_size = 1` (whole units),
/// so a €20 clip (10 % of the €200 budget) buys ~50-100 DOGE and the lot floor
/// always bites. Each 60-bar cycle walks four regimes so that every signal
/// family in the bake-off fires at least once:
///
/// | bars  | regime           | fires                                     |
/// |-------|------------------|-------------------------------------------|
/// | 0-14  | steady uptrend   | SMA cross, MACD, donchian break, ROC, OBV |
/// | 15-28 | quiet drift      | (compresses the Bollinger bands)          |
/// | 29    | crash + vol spike| `close < bollinger_lower(20,2)`, vol breakout |
/// | 30-49 | slide            | `rsi(14) < 30`                            |
/// | 50-59 | recovery         | the RSI/BB re-entry edge                  |
///
/// An arm that never trades cannot witness lot realism, so the gates assert
/// `fills` is non-empty rather than skipping.
fn doge_production_bars() -> Vec<Bar> {
    let symbol = Symbol::new("DOGEUSDT");
    let mut price = dec!(0.2013);
    let mut out = Vec::with_capacity(GATE_BARS as usize);
    for i in 0..GATE_BARS {
        let (step, volume) = match i % 60 {
            14 => (dec!(0.0075), dec!(9)),     // breakout bar: new high + volume
            0..=13 => (dec!(0.0075), dec!(1)), // steady uptrend
            15..=28 => (dec!(0.0002), dec!(0.6)), // quiet: bands compress
            29 => (dec!(-0.0700), dec!(12)),   // crash: pierces the lower band
            30..=49 => (dec!(-0.0011), dec!(1.1)), // slide: drives RSI < 30
            _ => (dec!(0.0035), dec!(2)),      // recovery
        };
        price += step;
        out.push(make_bar_with_volume(&symbol, i, price, volume));
    }
    out
}

/// Synthetic as-of DVOL series for the `v0.dvol_regime` arm, one entry per bar.
///
/// Bars 0-29 carry 30 DISTINCT closes (50…79) to fill the strategy's ring;
/// thereafter the series alternates 40 (calm → weight 1) / 90 (stress →
/// weight 0) in 10-bar blocks, which straddles the trailing median in both
/// directions and drives a Buy/Sell round trip every 20 bars.
fn dvol_regime_series() -> Vec<Option<Decimal>> {
    (0..GATE_BARS)
        .map(|i| {
            if i < 30 {
                Some(Decimal::from(50 + i))
            } else if ((i - 30) / 10) % 2 == 0 {
                Some(dec!(40))
            } else {
                Some(dec!(90))
            }
        })
        .collect()
}

/// Build the ScenarioConfig for `arm` exactly as `bakeoff/mod.rs` does, with
/// `latency_slippage_sim` supplied by the caller — the ONLY difference between
/// the two runs each gate performs.
fn gate_config(arm: &str, sim: LatencySlippageSimConfig) -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId(arm.into()),
        pair: (Venue::Binance, Symbol::new("DOGEUSDT")),
        range: DateRange::Last30d, // ignored — bars_override supplies the data
        params: None,
        seed: GATE_SEED,
        write_report: false, // anchor-safe: no report body is ever written
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(doge_production_bars()),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: sim,
        reports_dir: None,
        // Mirrors `BakeoffConfig::is_short_enabled` (bakeoff/mod.rs).
        short_enabled: matches!(
            arm,
            "v0.sma_cross_ls" | "v0.macd_ls" | "v0.rsi_ls" | "v0.bbands_ls" | "v0.always_short"
        ),
        initial_capital: Some(GATE_CAPITAL),
        composed_toml_override: None,
        dvol_override: if arm == "v0.dvol_regime" {
            Some(dvol_regime_series())
        } else {
            None
        },
        macro_regime_series: None,
    }
}

/// Run one arm through the PRODUCTION entry point.
///
/// `sim` is sourced from a constructor, never hand-written: the advisor run
/// passes `LatencySlippageSimConfig::advisor_default()` — the literal value
/// `bakeoff/mod.rs` and `bakeoff/sweep.rs` put on every bake-off arm.
async fn run_arm(arm: &str, sim: LatencySlippageSimConfig) -> RunReport {
    let (_handle, cancel_rx) = cancellation_pair();
    run_scenario(gate_config(arm, sim), cancel_rx, ProgressSender::disabled())
        .await
        .unwrap_or_else(|e| panic!("run_scenario({arm}) must succeed on the gate corpus: {e:?}"))
}

/// DOGEUSDT's checked-in venue step, read from the production table (never
/// hard-coded here) so a table edit cannot silently defeat the gate.
fn doge_step() -> Decimal {
    cost::venue_filter_for(&Symbol::new("DOGEUSDT"))
        .expect("DOGEUSDT must be in the venue-filter table")
        .step_size
}

fn final_equity(report: &RunReport) -> Decimal {
    report
        .equity_series
        .last()
        .expect("equity_series is never empty")
        .1
        .amount()
}

/// Count fills whose qty is NOT an exact multiple of the venue `step_size`,
/// i.e. fills the lot filter demonstrably did not touch.
fn unrounded_fills(report: &RunReport, step: Decimal) -> Vec<Decimal> {
    report
        .fills
        .iter()
        .map(|f| f.qty.get())
        .filter(|q| !(q % step).is_zero())
        .collect()
}

/// Arms whose fills are NOT all produced by `PaperEngine::step`.
///
/// The long/short arms route Sell-when-flat (open short) and Buy-when-short
/// (cover) through `short_exec::{try_open_short, try_cover_short}`, which
/// `continue`s past the matching engine and synthesizes a `FillView` directly
/// (`scenarios/sma_composed_run.rs`). Those legs therefore cannot be
/// lot-filtered by any amount of `venue_filter` wiring — a SIBLING of bug-log
/// #79 at a different site, reported 2026-08-13 and deliberately NOT fixed in
/// the #79 patch-pass. For these arms the gate asserts the engine-routed
/// portion got filtered (strictly fewer unrounded fills) instead of demanding
/// that every fill did.
fn has_engine_bypassing_short_legs(arm: &str) -> bool {
    matches!(
        arm,
        "v0.sma_cross_ls" | "v0.macd_ls" | "v0.rsi_ls" | "v0.bbands_ls" | "v0.always_short"
    )
}

/// The per-arm assertion bundle. Returns `(eq_plain, eq_advisor)`.
///
/// Three independent witnesses, each of which alone fails the "inert feature"
/// state:
///
/// 1. **Both runs traded.** A silent zero-trade arm proves nothing.
/// 2. **Mechanism.** The plain-default path emits fills that are NOT multiples
///    of the venue `step_size`; the advisor path emits strictly fewer such
///    fills — zero, for every arm whose fills all come from
///    `PaperEngine::step`. The filter demonstrably reached the fill seam on
///    one path and demonstrably did not on the other.
/// 3. **Effect.** The rounding propagated into cash/equity bookkeeping:
///    terminal equities differ.
async fn assert_arm_diverges(arm: &str) -> (Decimal, Decimal) {
    let step = doge_step();

    let plain = run_arm(arm, LatencySlippageSimConfig::default()).await;
    let advisor = run_arm(arm, LatencySlippageSimConfig::advisor_default()).await;

    assert!(
        !plain.fills.is_empty() && !advisor.fills.is_empty(),
        "{arm}: the gate corpus must make this arm TRADE on both paths \
         (plain fills={}, advisor fills={}) — a zero-trade arm cannot witness \
         lot realism and would let bug-log #79 pass unnoticed",
        plain.fills.len(),
        advisor.fills.len()
    );

    let plain_unrounded = unrounded_fills(&plain, step);
    assert!(
        !plain_unrounded.is_empty(),
        "{arm}: the PLAIN default produced {} fills and every one of them was \
         already a multiple of step_size={step} — this gate cannot discriminate \
         on this corpus (or `LatencySlippageSimConfig::default()` grew a \
         venue_filter, which would move every anchored report body — ADR-0087 § D6)",
        plain.fills.len()
    );

    let advisor_unrounded = unrounded_fills(&advisor, step);
    let inert_diagnosis = format!(
        "ADVISOR-PATH GATE FAIL ({arm}) — €200 lot realism is INERT in production!\n\
         advisor-path fills NOT multiples of step_size={step}: {}/{} {:?}\n\
         plain-default unrounded fills: {}/{} (so the corpus IS discriminating)\n\
         \n\
         `advisor_default()` sets venue_filter=Some(LotSizeAndMinNotional). For it to\n\
         BITE, two links must both hold (bug-log #79 — both were broken):\n\
           (a) engine.rs `run_scenario` must thread `cfg.latency_slippage_sim` into\n\
               THIS arm's scenario input (not `LatencySlippageSimConfig::default()`);\n\
           (b) the runner must APPLY it — `PaperEngine::new(..)\n\
               .with_venue_filter_mode(input.latency_slippage_sim.venue_filter)` in\n\
               `scenarios/sma_composed_run.rs` `run`/`run_with_strategy`, or the\n\
               inline `v0.8.vote.*` engine in engine.rs.\n\
         A green `with_venue_filter_mode` unit test proves NEITHER.",
        advisor_unrounded.len(),
        advisor.fills.len(),
        advisor_unrounded,
        plain_unrounded.len(),
        plain.fills.len(),
    );

    if has_engine_bypassing_short_legs(arm) {
        assert!(
            advisor_unrounded.len() < plain_unrounded.len(),
            "{inert_diagnosis}\n(this arm's short legs bypass PaperEngine::step, so \
             the gate only requires the engine-routed portion to be filtered)"
        );
    } else {
        assert!(advisor_unrounded.is_empty(), "{inert_diagnosis}");
    }

    let eq_plain = final_equity(&plain);
    let eq_advisor = final_equity(&advisor);
    let relative = (eq_plain - eq_advisor).abs() / eq_plain;
    assert_ne!(
        eq_plain, eq_advisor,
        "ADVISOR-PATH GATE FAIL ({arm}) — lot realism reached the fills but moved \
         no equity: the rounded qty never propagated into cash/position bookkeeping \
         (eq={eq_plain})"
    );

    eprintln!(
        "advisor-path [{arm}]: eq_plain={eq_plain} eq_advisor={eq_advisor} \
         relative={relative} fills={} unrounded_plain={} unrounded_advisor={}",
        advisor.fills.len(),
        plain_unrounded.len(),
        advisor_unrounded.len()
    );
    (eq_plain, eq_advisor)
}

/// The headline gate: the flagship bake-off arm, through `run_scenario`.
///
/// This is the test bug-log #79 says did not exist. It fails if EITHER link
/// (a) threading or (b) application is cut.
#[tokio::test]
async fn advisor_bakeoff_path_v0_sma_diverges_from_plain_default() {
    let (eq_plain, eq_advisor) = assert_arm_diverges("v0.sma").await;
    // The two runs differ ONLY in ScenarioConfig.latency_slippage_sim, so on
    // the ADR-0087 § D5 coarse-lot corpus the €200 budget must move by a
    // materially visible amount, not merely a rounding whisker.
    let relative = (eq_plain - eq_advisor).abs() / eq_plain;
    assert!(
        relative >= dec!(0.0001),
        "v0.sma: expected >= 1 bp of terminal-equity divergence on the DOGE \
         coarse-lot corpus; got {relative} (eq_plain={eq_plain} eq_advisor={eq_advisor})"
    );
}

/// Every `sma_composed_run`-shaped bake-off arm — the ~13 arms bug-log #79
/// found hard-coding `LatencySlippageSimConfig::default()` in `run_scenario`.
///
/// Reverting the threading for ONE arm turns this RED (that is mutation 2 in
/// the #79 patch-pass); the single-arm test above would stay green.
#[tokio::test]
async fn advisor_bakeoff_path_every_sma_shaped_arm_diverges() {
    for arm in [
        "v0.sma",
        "v0.5.macd",
        "v0.5.rsi",
        "v0.5.bbands",
        "v0.donchian_break",
        "v0.donchian_floor",
        "v0.vol_breakout",
        "v0.roc_momentum",
        "v0.obv",
        "v0.sma_cross_ls",
        "v0.macd_ls",
        "v0.rsi_ls",
        "v0.bbands_ls",
    ] {
        assert_arm_diverges(arm).await;
    }
}

/// `v0.dvol_regime` is the only arm dispatched through
/// `sma_composed_run::run_with_strategy` — the SECOND of the two engine
/// constructors bug-log #79 found unwired. Without this case a fix that
/// touched only `run` would ship half-inert.
#[tokio::test]
async fn advisor_bakeoff_path_dvol_arm_diverges_via_run_with_strategy() {
    assert_arm_diverges("v0.dvol_regime").await;
}

/// The 8 `v0.8.vote.*` ensemble arms build their `PaperEngine` INLINE in
/// `run_scenario` rather than going through `sma_composed_run` — a third
/// construction site, and a third of the ranked bake-off field.
///
/// All 8 ids share ONE `run_scenario` match arm and ONE engine construction,
/// so the four exercised here cover that site completely. The other four
/// (`majority`, `unanimous`, `tr_mr_macd_rsi`, `k3of4`) need MACD **and** RSI
/// **and** Bollinger to hold a Long stance on the same bar; they stay flat on
/// this synthetic corpus, and a zero-trade arm is exactly the kind of silent
/// pass this gate exists to refuse — so they are named here rather than
/// listed and skipped.
#[tokio::test]
async fn advisor_bakeoff_path_every_vote_arm_diverges() {
    for arm in [
        "v0.8.vote.trend_pair",
        "v0.8.vote.tr_mr_sma_bb",
        "v0.8.vote.any1of4",
        "v0.8.vote.k2of4",
    ] {
        assert_arm_diverges(arm).await;
    }
}

/// ADR-0087 § D6 byte-identity guard, asserted through the PRODUCTION path:
/// a ScenarioConfig carrying `LatencySlippageSimConfig::default()` must reach
/// the fill seam with the filter OFF.
///
/// This is the anchor contract. Every anchored CLI lane builds its config with
/// `venue_filter: None`; if threading ever started forcing the filter on, the
/// fills below would all be lot-rounded and this test goes red BEFORE
/// `verify_anchors.sh` has to.
#[tokio::test]
async fn plain_default_config_still_reaches_the_engine_with_the_filter_off() {
    let step = doge_step();
    let plain = run_arm("v0.sma", LatencySlippageSimConfig::default()).await;
    assert!(!plain.fills.is_empty(), "the corpus must trade");
    assert!(
        plain.fills.iter().any(|f| !(f.qty.get() % step).is_zero()),
        "ANCHOR CONTRACT BREACH — a plain-`Default` ScenarioConfig produced only \
         lot-rounded fills. `LatencySlippageSimConfig::default()` MUST stay \
         `venue_filter: None` (ADR-0087 § D6): every anchored report body depends \
         on that arm being byte-identical to the pre-ADR-0087 fill path."
    );
}
