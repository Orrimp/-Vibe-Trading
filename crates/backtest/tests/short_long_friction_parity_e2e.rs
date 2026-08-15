//! **The bug-log #80 gate: the long legs and the short legs must pay the same
//! friction per unit notional.**
//!
//! # What this pins, and why the project needed it
//!
//! The bake-off *ranks* every arm against every other arm and crowns one. Until
//! 2026-08-15 the `_ls` (long/short) arms' Sell-when-flat and Buy-when-short
//! legs called `short_exec::try_{open,cover}_short` and then
//! `continue`d past `PaperEngine::step`, hand-synthesizing a `FillView` at
//! `mark`. Those legs paid the taker fee — and nothing else:
//!
//! | execution effect                        | long legs | short legs (pre-fix) |
//! |-----------------------------------------|-----------|----------------------|
//! | taker fee                               | yes       | yes                  |
//! | slippage (`MatchConfig.slippage_bps`)   | yes       | **no**               |
//! | venue filter (lot size / min notional)  | yes       | **no**               |
//! | fill-price model (`FillPriceMode`)      | yes       | **no**               |
//!
//! So every short-enabled arm carried a systematic, unearned advantage inside
//! the very comparison that decides what the operator is shown. Witnessed on
//! the #79 gate corpus: `v0.sma_cross_ls` emitted 194 fills of which **20 were
//! still un-rounded on the advisor path** — precisely the short legs, visible
//! only because the long legs had just been lot-filtered.
//!
//! The moral in bug-log #80 is that *forks drift*: the long path had by then
//! gained a solvency guard, a side-blind cap, slippage and a venue filter, and
//! the short path had inherited none of them **because nothing compared the
//! two**. This file is that comparison. It runs the SAME strategy (SMA
//! crossover) twice over the SAME symmetric fixture — once long-only
//! (`v0.sma`) and once short-enabled (`v0.sma_cross_ls`) — through the real
//! `run_scenario` entry point, and asserts the friction per unit notional
//! matches.
//!
//! # Why it cannot pass vacuously
//!
//! Three independent traps, each of which alone fails the "nothing to compare"
//! state:
//!
//! 1. **Both runs must trade** (`fills` non-empty on both).
//! 2. **The short run must actually go short.** The signed position is
//!    reconstructed from the fill tape; the test fails unless it is negative on
//!    at least one bar AND at least `MIN_SHORT_LEG_FILLS` fills are short legs.
//!    A long-only degeneration of the `_ls` arm would sail through a pure
//!    ratio-equality assertion — this is the trap that catches it.
//! 3. **The friction must be non-zero.** A run with `slippage_bps = 0` and
//!    `taker_fee_bps = 0` would trivially match; the ratio is asserted `> 0`.
//!
//! # RED-proof (mutation, actually run 2026-08-15)
//!
//! Reverting `scenarios/sma_composed_run.rs`'s two short branches to the exact
//! pre-#80 shape (`short_exec::try_{open,cover}_short`, accounting done at
//! `mark`, `continue` past the engine, `FillView` synthesized by hand) turns
//! **three of the four** gates RED. Verbatim captured output:
//!
//! ```text
//! thread 'long_and_short_legs_pay_identical_friction_per_unit_notional' panicked at
//! crates/backtest/tests/short_long_friction_parity_e2e.rs:407:5:
//! FRICTION PARITY FAIL — the short legs do not pay what the long legs pay!
//!   long-only  (v0.sma)          : friction/notional = 0.0006  (slippage 0.0002 + fee 0.0004)
//!   short-enabled (v0.sma_cross_ls) : friction/notional = 0.0005410376731143801540190937
//!     (slippage 0.0001410376731143801540190937 + fee 0.000400000000000000000)
//!   difference = 0.0000589623268856198459809063 (tolerance 0.000000000000000001)
//!   short-leg fills in the short run: 56 of 85
//!
//! thread 'every_fill_price_carries_the_engine_slippage' panicked at
//! crates/backtest/tests/short_long_friction_parity_e2e.rs:458:13:
//! assertion `left == right` failed: SLIPPAGE PARITY FAIL (v0.sma_cross_ls) —
//! fill 0 (Sell) priced at 0.2020, expected 0.20195960 (mark 0.2020 ± 2 bps).
//!   left: 0.2020
//!  right: 0.20195960
//!
//! thread 'every_short_leg_fill_is_lot_rounded_like_a_long_leg' panicked at
//! crates/backtest/tests/short_long_friction_parity_e2e.rs:518:9:
//! LOT PARITY FAIL (v0.sma_cross_ls) — 30 of 85 advisor-path fills are NOT
//! multiples of step_size=1: [99.00990099009900990099009901, ...]
//!
//! test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
//! ```
//!
//! The short legs under-paid slippage by 30 % of the charged rate
//! (0.000141 against the long legs' 0.000200) — the aggregate, so the per-leg
//! shortfall is larger still.
//!
//! **`every_fill_pays_the_taker_fee_on_its_slipped_notional` stays GREEN under
//! the mutation, and that is correct**: the pre-#80 short legs *did* charge the
//! taker fee — on their own un-slipped notional, self-consistently. bug-log
//! #80's table says exactly that, and a gate that went red on the fee too would
//! be over-claiming. The three that fail are precisely the three effects the
//! bypass skipped.
//!
//! # Anchor safety
//!
//! `write_report = false` on every run here (the bake-off's own setting), so no
//! Markdown body is written and no anchored body-SHA can move. `evidence/` is
//! untouched.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use backtest::cancel::cancellation_pair;
use backtest::cli_types::LatencySlippageSimConfig;
use backtest::engine::{DateRange, RunReport, ScenarioConfig, ScenarioDataSource, run_scenario};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Bar, Price, Quantity, Side, StrategyId, Symbol, Timeframe, Timestamp, Venue};

// ── The two arms under comparison ────────────────────────────────────────────

/// Long-only arm: SMA crossover, `short_enabled = false`.
const LONG_ONLY_ARM: &str = "v0.sma";
/// The SAME strategy with `short_enabled = true` (`BakeoffConfig::is_short_enabled`).
const SHORT_ENABLED_ARM: &str = "v0.sma_cross_ls";

/// `MatchConfig.slippage_bps` for both arms, as `engine.rs::run_scenario`
/// builds it. Read here as the EXPECTED value the fills must carry.
const SLIPPAGE_BPS: Decimal = dec!(2);
/// `MatchConfig.taker_fee_bps` for both arms.
const TAKER_FEE_BPS: Decimal = dec!(4);
const TEN_K: Decimal = dec!(10_000);

/// The short-enabled run must produce at least this many short-leg fills, or
/// the comparison has nothing to compare and the gate is vacuous.
const MIN_SHORT_LEG_FILLS: usize = 10;

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

// ── The symmetric fixture ────────────────────────────────────────────────────

/// Bars per half-cycle of the triangle wave (down-leg length == up-leg length).
const HALF_CYCLE: i64 = 55;
/// Number of complete down/up cycles.
const CYCLES: i64 = 8;

/// A **symmetric, down-first** DOGEUSDT triangle wave: `CYCLES` cycles of
/// `HALF_CYCLE` bars down followed by `HALF_CYCLE` bars up, both legs carrying
/// the same absolute per-bar step.
///
/// Two properties earn their place, and both were established empirically —
/// the first fixture tried here produced 85 fills and **zero** short legs, and
/// the gate's own vacuity trap caught it:
///
/// 1. **Symmetric.** Equal leg lengths and equal per-bar magnitude, so the
///    up-moves and down-moves get structurally identical price action and
///    neither leg family is exercised more gently than the other.
/// 2. **Down first.** SMA(20,50) warms up bearish, so the arm's first
///    actionable signal is Sell-while-FLAT — it opens a short before it ever
///    holds a long. On an up-first corpus the arm instead accumulates a long
///    (`Buy`-when-long extends every bar) whose notional soon exceeds
///    `per_symbol_exposure_cap`, after which `Order::new` refuses the closing
///    Sell and the arm is stuck long forever: no Sell-while-flat, no shorts,
///    nothing to compare. That pathology is pre-existing and is not what this
///    file is about; the corpus simply routes around it.
///
/// Yields ~85 fills of which ~56 are short legs — a genuine mixture, not a
/// token one.
///
/// DOGEUSDT is the ADR-0087 § D5 coarse-lot corpus (`step_size = 1`), so at the
/// €200 budget a 10 % clip (~€20 → ~70-90 DOGE) is well clear of the €5
/// `min_notional` while the lot floor still bites on every fill — which is what
/// makes the lot-parity assertion discriminating.
fn symmetric_triangle_bars() -> Vec<Bar> {
    let symbol = Symbol::new("DOGEUSDT");
    let base = dec!(0.3000);
    let step = dec!(0.0020);
    let mut out = Vec::with_capacity((CYCLES * HALF_CYCLE * 2) as usize);
    for i in 0..(CYCLES * HALF_CYCLE * 2) {
        let phase = i % (HALF_CYCLE * 2);
        let offset = if phase < HALF_CYCLE {
            -Decimal::from(phase) * step
        } else {
            -Decimal::from(HALF_CYCLE * 2 - phase) * step
        };
        // Volume alternates so the volume-gated composed families would also
        // fire; harmless for SMA crossover.
        let volume = if i % 7 == 0 { dec!(4) } else { dec!(1) };
        out.push(make_bar(&symbol, i, base + offset, volume));
    }
    out
}

fn make_bar(symbol: &Symbol, idx: i64, close: Decimal, volume: Decimal) -> Bar {
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

// ── Production entry point ───────────────────────────────────────────────────

/// Build the ScenarioConfig exactly as `bakeoff/mod.rs` does for `arm`.
fn gate_config(arm: &str, sim: LatencySlippageSimConfig) -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId(arm.into()),
        pair: (Venue::Binance, Symbol::new("DOGEUSDT")),
        range: DateRange::Last30d, // ignored — bars_override supplies the data
        params: None,
        seed: GATE_SEED,
        write_report: false, // anchor-safe: no report body is ever written
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(symmetric_triangle_bars()),
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
        dvol_override: None,
        macro_regime_series: None,
    }
}

async fn run_arm(arm: &str, sim: LatencySlippageSimConfig) -> RunReport {
    let (_handle, cancel_rx) = cancellation_pair();
    run_scenario(gate_config(arm, sim), cancel_rx, ProgressSender::disabled())
        .await
        .unwrap_or_else(|e| panic!("run_scenario({arm}) must succeed on the gate corpus: {e:?}"))
}

// ── Friction measurement ─────────────────────────────────────────────────────

/// The un-slipped mark for each fill, keyed by the bar close timestamp the
/// engine stamps onto the `Fill` (`venue_ts: bar.close_ts`).
fn mark_at(ts: Timestamp) -> Decimal {
    symmetric_triangle_bars()
        .into_iter()
        .find(|b| b.close_ts == ts)
        .map(|b| b.close.get())
        .unwrap_or_else(|| panic!("every fill must land on a bar of the fixture (ts={ts:?})"))
}

/// Friction over a run's fill tape, split into its two components.
///
/// **Each component is normalised by the notional it is actually charged on**,
/// and that split matters. Charging both against a single denominator makes the
/// aggregate ratio depend on the run's **Buy/Sell mix**: the engine applies the
/// fee to the *slipped* notional, and slippage moves price up on a Buy and down
/// on a Sell, so a Buy-heavy tape reads `0.0002 + 0.0004 × 1.0002` while a
/// Sell-heavy one reads `0.0002 + 0.0004 × 0.9998`. The long-only and
/// short-enabled runs necessarily have different mixes, so a single-denominator
/// ratio differs by ~8e-9 even when the two friction models are perfectly
/// identical — a tolerance-shaped hole the same size as the effect being
/// measured. (Observed, not theorised: that is exactly what the first version
/// of this gate reported.) Normalising each component against its own base
/// makes both exact per-fill constants and the comparison mix-independent.
struct Friction {
    /// `Σ |fill_price − mark| × qty` — the slippage the engine charged.
    slippage_cost: Decimal,
    /// `Σ mark × qty` — size at the UN-slipped mark (slippage's base).
    mark_notional: Decimal,
    /// `Σ fee` — the taker fee the engine charged.
    fee: Decimal,
    /// `Σ fill_price × qty` — size at the fill price (the fee's base).
    fill_notional: Decimal,
    fills: usize,
}

impl Friction {
    fn slippage_per_notional(&self) -> Decimal {
        assert!(
            self.mark_notional > Decimal::ZERO,
            "a run with zero traded notional cannot witness friction parity"
        );
        self.slippage_cost / self.mark_notional
    }

    fn fee_per_notional(&self) -> Decimal {
        assert!(
            self.fill_notional > Decimal::ZERO,
            "a run with zero traded notional cannot witness friction parity"
        );
        self.fee / self.fill_notional
    }

    /// Total friction per unit notional: the sum of the two component rates.
    fn per_unit_notional(&self) -> Decimal {
        self.slippage_per_notional() + self.fee_per_notional()
    }
}

fn measure(report: &RunReport) -> Friction {
    let mut slippage_cost = Decimal::ZERO;
    let mut mark_notional = Decimal::ZERO;
    let mut fee = Decimal::ZERO;
    let mut fill_notional = Decimal::ZERO;
    for f in &report.fills {
        let qty = f.qty.get();
        let mark = mark_at(f.venue_ts);
        slippage_cost += (f.price.get() - mark).abs() * qty;
        mark_notional += mark * qty;
        fee += f.fee.amount();
        fill_notional += f.price.get() * qty;
    }
    Friction {
        slippage_cost,
        mark_notional,
        fee,
        fill_notional,
        fills: report.fills.len(),
    }
}

/// Reconstruct the signed position from the fill tape: Buy adds, Sell
/// subtracts. Returns `(short_leg_fills, most_negative_position)`.
///
/// A fill is a **short leg** when it moves the position at or below zero: a
/// Sell from a flat/short position (open/extend) or a Buy while short (cover).
fn short_leg_census(report: &RunReport) -> (usize, Decimal) {
    let mut pos = Decimal::ZERO;
    let mut short_legs = 0usize;
    let mut most_negative = Decimal::ZERO;
    for f in &report.fills {
        let qty = f.qty.get();
        let before = pos;
        match f.side {
            Side::Buy => pos += qty,
            Side::Sell => pos -= qty,
        }
        // Sell-when-flat-or-short (open/extend) or Buy-when-short (cover).
        if (matches!(f.side, Side::Sell) && before <= Decimal::ZERO)
            || (matches!(f.side, Side::Buy) && before < Decimal::ZERO)
        {
            short_legs += 1;
        }
        if pos < most_negative {
            most_negative = pos;
        }
    }
    (short_legs, most_negative)
}

/// Assert the short-enabled run genuinely went short — the anti-vacuity trap.
fn assert_actually_shorted(report: &RunReport, label: &str) -> usize {
    let (short_legs, most_negative) = short_leg_census(report);
    assert!(
        most_negative < Decimal::ZERO,
        "VACUOUS GATE ({label}) — the short-enabled arm never held a negative \
         position across {} fills, so this file compares long legs to long legs \
         and would stay green with the #80 bypass fully restored. Fix the \
         fixture, not the assertion.",
        report.fills.len()
    );
    assert!(
        short_legs >= MIN_SHORT_LEG_FILLS,
        "VACUOUS GATE ({label}) — only {short_legs} of {} fills are short legs \
         (need >= {MIN_SHORT_LEG_FILLS}); the parity ratio would be dominated by \
         the long legs and could hide a short-leg friction regression",
        report.fills.len()
    );
    short_legs
}

// ═════════════════════════════════════════════════════════════════════════════
// The gates
// ═════════════════════════════════════════════════════════════════════════════

/// **The headline gate.** Same strategy, same symmetric fixture, long-only vs
/// short-enabled: friction per unit notional must match.
#[tokio::test]
async fn long_and_short_legs_pay_identical_friction_per_unit_notional() {
    let sim = LatencySlippageSimConfig::advisor_default();
    let long_only = run_arm(LONG_ONLY_ARM, sim.clone()).await;
    let short_enabled = run_arm(SHORT_ENABLED_ARM, sim).await;

    assert!(
        !long_only.fills.is_empty(),
        "the fixture must make {LONG_ONLY_ARM} trade"
    );
    let short_legs = assert_actually_shorted(&short_enabled, SHORT_ENABLED_ARM);

    let fl = measure(&long_only);
    let fs = measure(&short_enabled);
    let rl = fl.per_unit_notional();
    let rs = fs.per_unit_notional();

    assert!(
        rl > Decimal::ZERO,
        "TRIVIAL GATE — the long-only run paid ZERO friction per unit notional; \
         with slippage_bps and taker_fee_bps both at 0 any bypass matches"
    );
    assert!(
        fl.slippage_per_notional() > Decimal::ZERO,
        "TRIVIAL GATE — the long-only run paid ZERO slippage, so the component \
         the #80 bypass actually skipped is not being measured at all"
    );

    // Both components are per-fill constants applied by `PaperEngine::step`
    // (slippage_bps against the mark, taker_fee_bps against the fill notional),
    // so each aggregate rate is exact and equal across the two runs however
    // differently their trade sequences unfold. The tolerance absorbs Decimal
    // division rounding only.
    let tolerance = dec!(0.000000000000000001);
    let difference = (rl - rs).abs();
    assert!(
        difference <= tolerance,
        "FRICTION PARITY FAIL — the short legs do not pay what the long legs pay!\n  \
         long-only  ({LONG_ONLY_ARM})          : friction/notional = {rl}  \
         (slippage {} + fee {})\n  \
         short-enabled ({SHORT_ENABLED_ARM}) : friction/notional = {rs}  \
         (slippage {} + fee {})\n  \
         difference = {difference} (tolerance {tolerance})\n  \
         short-leg fills in the short run: {short_legs} of {}\n\n\
         This is bug-log #80: a `continue` past `PaperEngine::step` in \
         `scenarios/sma_composed_run.rs` makes the short legs skip slippage, \
         the lot-size/min-notional venue filter and the fill-price model while \
         the long legs pay all three — inside a comparison that RANKS them \
         against each other.",
        fl.slippage_per_notional(),
        fl.fee_per_notional(),
        fs.slippage_per_notional(),
        fs.fee_per_notional(),
        fs.fills
    );

    println!(
        "friction parity: long_only={rl} (slippage {} + fee {}, {} fills) \
         short_enabled={rs} (slippage {} + fee {}, {} fills, {short_legs} short legs)",
        fl.slippage_per_notional(),
        fl.fee_per_notional(),
        fl.fills,
        fs.slippage_per_notional(),
        fs.fee_per_notional(),
        fs.fills
    );
}

/// Mechanism 1 — **slippage**: every fill on BOTH runs is priced at the bar
/// mark moved by exactly `slippage_bps`, in the direction that costs money.
/// The pre-#80 short legs were priced at the raw `mark`.
#[tokio::test]
async fn every_fill_price_carries_the_engine_slippage() {
    let sim = LatencySlippageSimConfig::advisor_default();
    for arm in [LONG_ONLY_ARM, SHORT_ENABLED_ARM] {
        let report = run_arm(arm, sim.clone()).await;
        assert!(!report.fills.is_empty(), "{arm} must trade");
        if arm == SHORT_ENABLED_ARM {
            assert_actually_shorted(&report, arm);
        }
        for (i, f) in report.fills.iter().enumerate() {
            let mark = mark_at(f.venue_ts);
            let expected = match f.side {
                Side::Buy => mark * (Decimal::ONE + SLIPPAGE_BPS / TEN_K),
                Side::Sell => mark * (Decimal::ONE - SLIPPAGE_BPS / TEN_K),
            };
            assert_eq!(
                f.price.get(),
                expected,
                "SLIPPAGE PARITY FAIL ({arm}) — fill {i} ({:?}) priced at {}, expected {expected} \
                 (mark {mark} ± {SLIPPAGE_BPS} bps). A fill priced AT the mark is a leg that \
                 never went through `PaperEngine::step` — bug-log #80.",
                f.side,
                f.price.get()
            );
        }
    }
}

/// Mechanism 2 — **fee**: every fill on BOTH runs is charged
/// `taker_fee_bps` on its slipped notional. (The pre-#80 short legs DID pay
/// the fee — but on the un-slipped notional, so this also pins the base.)
#[tokio::test]
async fn every_fill_pays_the_taker_fee_on_its_slipped_notional() {
    let sim = LatencySlippageSimConfig::advisor_default();
    for arm in [LONG_ONLY_ARM, SHORT_ENABLED_ARM] {
        let report = run_arm(arm, sim.clone()).await;
        assert!(!report.fills.is_empty(), "{arm} must trade");
        for (i, f) in report.fills.iter().enumerate() {
            let expected = f.price.get() * f.qty.get() * (TAKER_FEE_BPS / TEN_K);
            assert_eq!(
                f.fee.amount(),
                expected,
                "FEE PARITY FAIL ({arm}) — fill {i} paid {} on notional {}, expected {expected}",
                f.fee.amount(),
                f.price.get() * f.qty.get()
            );
        }
    }
}

/// Mechanism 3 — **venue filter**: under the advisor config every fill on BOTH
/// runs is an exact multiple of the venue `step_size`. This is the assertion
/// that was explicitly WEAKENED for the `_ls` arms in
/// `lot_realism_divergence_end_to_end.rs` while #80 was open (20 of
/// `v0.sma_cross_ls`'s 194 fills stayed un-rounded); the exemption is now gone
/// from both files.
#[tokio::test]
async fn every_short_leg_fill_is_lot_rounded_like_a_long_leg() {
    let step = cost::venue_filter_for(&Symbol::new("DOGEUSDT"))
        .expect("DOGEUSDT must be in the venue-filter table")
        .step_size;
    let sim = LatencySlippageSimConfig::advisor_default();

    for arm in [LONG_ONLY_ARM, SHORT_ENABLED_ARM] {
        let report = run_arm(arm, sim.clone()).await;
        assert!(!report.fills.is_empty(), "{arm} must trade");
        if arm == SHORT_ENABLED_ARM {
            assert_actually_shorted(&report, arm);
        }
        let unrounded: Vec<Decimal> = report
            .fills
            .iter()
            .map(|f| f.qty.get())
            .filter(|q| !(q % step).is_zero())
            .collect();
        assert!(
            unrounded.is_empty(),
            "LOT PARITY FAIL ({arm}) — {} of {} advisor-path fills are NOT multiples of \
             step_size={step}: {unrounded:?}\nThose legs never reached the venue filter, \
             i.e. they never reached `PaperEngine::step` — bug-log #80.",
            unrounded.len(),
            report.fills.len()
        );
    }

    // Negative control: with the plain default (no venue filter) the SAME
    // short-enabled run must leave fills un-rounded — otherwise the corpus is
    // not discriminating and the assertion above proves nothing.
    let plain = run_arm(SHORT_ENABLED_ARM, LatencySlippageSimConfig::default()).await;
    let plain_unrounded = plain
        .fills
        .iter()
        .filter(|f| !(f.qty.get() % step).is_zero())
        .count();
    assert!(
        plain_unrounded > 0,
        "NON-DISCRIMINATING CORPUS — every one of {SHORT_ENABLED_ARM}'s {} plain-default \
         fills was already a multiple of step_size={step}, so the advisor-path assertion \
         above would pass even with the filter switched off",
        plain.fills.len()
    );
}
