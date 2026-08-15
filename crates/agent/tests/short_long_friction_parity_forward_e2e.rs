//! **The bug-log #80 gate for the FORWARD PAPER LOOP — the half that executes
//! the operator's actual plan.**
//!
//! # Why this file exists separately from its `backtest` sibling
//!
//! `crates/backtest/tests/short_long_friction_parity_e2e.rs` pins the same
//! invariant on the *ranking* path (`run_scenario` → `sma_composed_run.rs`). It
//! drives an entry point inside the `backtest` crate and therefore **cannot
//! observe `crates/agent` at all** — which is exactly how the forward loop kept
//! the #80 bypass for three days after the ranking path was fixed
//! (`docs/dev-notes/reachability-map-2026-08-15.md`, finding **N-2**).
//!
//! The two halves are not equally important, and the forward one is not the
//! lesser:
//!
//! - the ranking half decides **what gets recommended** — a friction asymmetry
//!   there flatters a short arm's rank;
//! - this half decides **what the operator's paper account actually does** — a
//!   forward run whose fills are cheaper than the backtest that justified them
//!   drifts optimistic against its own recommendation.
//!
//! # What was wrong (pre-fix, `crates/agent/src/runtime.rs:2186-2267`)
//!
//! Both short branches called `short_exec::try_{open,cover}_short`,
//! hand-synthesized a `trading_core::Fill` priced at `bar.close` charging only
//! `notional × taker_bps`, and then `continue`d past `PaperEngine::step`:
//!
//! | execution effect                       | long legs | short legs (pre-fix) |
//! |----------------------------------------|-----------|----------------------|
//! | taker fee                              | yes       | yes                  |
//! | slippage (`MatchConfig.slippage_bps`)  | yes       | **no**               |
//! | fill-price model (`FillPriceMode`)     | yes       | **no**               |
//!
//! The comment sitting on that code claimed the shape was "the same as
//! `sma_composed_run.rs`". The #80 ranking-side fix made that claim false, and
//! nothing in the workspace could notice.
//!
//! # What this gate does
//!
//! Runs the SAME strategy through the REAL production forward loop twice over
//! the SAME symmetric fixture — once long-only (`v0.sma`) and once
//! short-enabled (`v0.sma_cross_ls`, which `build_registry_for` maps to the
//! very same `SmaCrossover`) — and asserts the friction per unit notional
//! matches.
//!
//! Three production seams are bound, not re-implemented:
//!
//! 1. `agent::build_registry_for` — the forward registry builder.
//! 2. `backtest::BakeoffConfig::is_short_enabled` — the exact predicate the
//!    production call site (`runtime.rs`, `paper_loop_supervisor`) uses to
//!    derive the flag it passes to `spawn_trading_loop`.
//! 3. `agent::runtime::spawn_trading_loop` — the forward loop itself.
//!
//! # Why it cannot pass vacuously
//!
//! 1. **Both runs must trade** (non-empty fill tape on both).
//! 2. **The short run must actually go short.** The signed position is
//!    reconstructed from the fill tape; the gate fails unless it goes negative
//!    and at least `MIN_SHORT_LEG_FILLS` fills are short legs. A long-only
//!    degeneration of the `_ls` arm would sail through a pure ratio-equality
//!    assertion — this is the trap that catches it.
//! 3. **The friction must be non-zero.** With `slippage_bps = 0` any bypass
//!    matches trivially, so the measured slippage rate is asserted `> 0`.
//!
//! # Scope — what this file deliberately does NOT assert
//!
//! The forward loop builds its `PaperEngine` WITHOUT `.with_venue_filter_mode`,
//! so neither leg family is lot-rounded there (reachability-map finding F-3,
//! about the forward loop as a whole). That absence is **symmetric** and does
//! not reopen #80's asymmetry, so the lot-parity assertion of the `backtest`
//! sibling has no counterpart here. If F-3 is ever fixed, this file should gain
//! that third mechanism test.
//!
//! # Anchor safety
//!
//! Nothing here writes a report body: `spawn_trading_loop` has no report
//! write-path at all, and `evidence/` is untouched. `verify_anchors.sh` prints
//! `ANCHORS PASS (119 / 119)` unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::build_registry_for;
use agent::config::{BacktestConfig, BusConfig, ForwardRunConfig, RiskConfig, SizingConfig};
use agent::runtime::spawn_trading_loop;
use data::MockFeed;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use trading_core::{
    Bar, Fill, Money, Price, Quantity, Side, StrategyId, Symbol, Tick, Timeframe, Timestamp, Usdt,
    Venue,
};

// ── The two arms under comparison ────────────────────────────────────────────

/// Long-only arm. `BakeoffConfig::is_short_enabled("v0.sma") == false`.
const LONG_ONLY_ARM: &str = "v0.sma";
/// The SAME `SmaCrossover` with `short_enabled == true`.
/// `build_registry_for` maps both ids to the same strategy (`runtime.rs`,
/// ADR-0068 T-D6), so any friction difference is the execution path, not the
/// signal.
const SHORT_ENABLED_ARM: &str = "v0.sma_cross_ls";

/// `MatchConfig.slippage_bps` the loop is configured with — read back as the
/// EXPECTED value every fill must carry.
const SLIPPAGE_BPS: Decimal = dec!(2);
/// `MatchConfig.taker_fee_bps` the loop is configured with.
const TAKER_FEE_BPS: Decimal = dec!(4);
const TEN_K: Decimal = dec!(10_000);

/// The short-enabled run must produce at least this many short-leg fills, or
/// the comparison has nothing to compare and the gate is vacuous.
const MIN_SHORT_LEG_FILLS: usize = 10;

/// The product's headline budget (PRD §13 Q5) — the same capital the
/// `backtest` sibling uses.
const GATE_BUDGET: Decimal = dec!(200);

/// Symbol traded by the fixture. Matches the sibling gate's corpus so the two
/// files read as one pair; the forward loop has no venue filter, so the coarse
/// lot size is inert here.
const GATE_SYMBOL: &str = "DOGEUSDT";

/// Bar pace fed to `MockFeed`. The tests run under `start_paused = true`, so
/// tokio auto-advances the clock and this costs no wall time.
const BAR_PACE: Duration = Duration::from_millis(1);

/// How long the fill tape must stay silent before a run is considered
/// finished. Under a paused clock this fires only once every other task is
/// idle — i.e. once the bar stream is exhausted.
const RUN_QUIESCENCE: Duration = Duration::from_millis(500);

// ── The symmetric fixture ────────────────────────────────────────────────────

/// Bars per half-cycle of the triangle wave (down-leg length == up-leg length).
const HALF_CYCLE: i64 = 55;
/// Number of complete down/up cycles.
const CYCLES: i64 = 8;

/// A **symmetric, down-first** triangle wave — the same shape, and for the same
/// two reasons, as the `backtest` sibling's fixture:
///
/// 1. **Symmetric.** Equal leg lengths and equal per-bar magnitude, so up-moves
///    and down-moves get structurally identical price action and neither leg
///    family is exercised more gently than the other.
/// 2. **Down first.** SMA(20,50) warms up bearish, so the arm's first
///    actionable signal is Sell-while-FLAT — it opens a short before it ever
///    holds a long. On an up-first corpus the arm instead accumulates a long
///    (Buy-when-long extends every bar) whose notional soon exceeds
///    `per_symbol_exposure_cap`, after which the closing Sell is refused and the
///    arm is stuck long forever: no Sell-while-flat, no shorts, nothing to
///    compare. That pathology is bug-log #71/#82 and is not what this file is
///    about; the corpus routes around it.
///
/// One tick per whole second, so `data::bar_aggregator` emits exactly one bar
/// per tick and `bar.close == tick.price`.
fn fixture_ticks() -> Vec<Tick> {
    let symbol = Symbol::new(GATE_SYMBOL);
    let base = dec!(0.3000);
    let step = dec!(0.0020);
    let n = CYCLES * HALF_CYCLE * 2;
    (0..n)
        .map(|i| {
            let phase = i % (HALF_CYCLE * 2);
            let offset = if phase < HALF_CYCLE {
                -Decimal::from(phase) * step
            } else {
                -Decimal::from(HALF_CYCLE * 2 - phase) * step
            };
            let dt = time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(i);
            let ts = Timestamp::new(dt);
            Tick {
                symbol: symbol.clone(),
                venue_ts: ts,
                local_recv_ts: ts,
                price: Price::new(base + offset).expect("positive close"),
                qty: Quantity::new(dec!(1)).expect("positive qty"),
                side: if i % 2 == 0 { Side::Buy } else { Side::Sell },
                trade_id: u64::try_from(i).expect("non-negative index"),
                venue: Venue::Binance,
            }
        })
        .collect()
}

/// The bars the loop will actually see — produced by the SAME aggregator
/// `MockFeed::subscribe_bars` uses, so `mark_at` cannot drift from the loop's
/// own view of the corpus.
fn fixture_bars() -> Vec<Bar> {
    data::bar_aggregator::aggregate_one_second_iter(fixture_ticks(), Venue::Binance)
}

/// The un-slipped mark for a fill, keyed by the bar close timestamp
/// `PaperEngine::step` stamps onto every `Fill` (`venue_ts: bar.close_ts`).
fn mark_at(ts: Timestamp) -> Decimal {
    fixture_bars()
        .into_iter()
        .find(|b| b.close_ts == ts)
        .map(|b| b.close.get())
        .unwrap_or_else(|| panic!("every fill must land on a bar of the fixture (ts={ts:?})"))
}

// ── Production entry point ───────────────────────────────────────────────────

fn gate_backtest_cfg() -> BacktestConfig {
    BacktestConfig {
        slippage_bps: u32::try_from(SLIPPAGE_BPS).expect("small"),
        taker_fee_bps: u32::try_from(TAKER_FEE_BPS).expect("small"),
        maker_fee_bps: 0,
        // Ignored: `budget = Some(..)` overrides the starting capital (ADR-0060 D5).
        initial_capital_usdt: 100_000.0,
    }
}

fn gate_risk_cfg() -> RiskConfig {
    RiskConfig {
        per_symbol_exposure_cap: 0.4,
        daily_loss_stop_pct: -5.0,
        max_drawdown_stop_pct: -15.0,
        sizing: SizingConfig {
            fixed_fraction: 0.1,
        },
    }
}

fn fwd_cfg(arm: &str) -> ForwardRunConfig {
    ForwardRunConfig {
        strategy: StrategyId::new(arm),
        symbol: Symbol::new(GATE_SYMBOL),
        budget: Money::<Usdt>::from_decimal(GATE_BUDGET),
        lookback: None,
        param_override: None,
        confidence: None,
    }
}

/// Drive the REAL forward paper loop for `arm` and return its fill tape.
///
/// `short_enabled` is derived by the production predicate, not hard-coded, so
/// this gate breaks if the arm ever stops being classified long/short.
async fn run_forward(arm: &str) -> Vec<Fill> {
    let symbol = Symbol::new(GATE_SYMBOL);
    let cfg = agent::config::Config::default();
    let registry = build_registry_for(&cfg, Some(&fwd_cfg(arm)))
        .unwrap_or_else(|e| panic!("build_registry_for({arm}) must succeed: {e:?}"));
    // The exact production derivation (`runtime.rs` paper_loop_supervisor).
    let short_enabled = backtest::BakeoffConfig::is_short_enabled(arm);

    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    // Subscribe BEFORE the loop starts — no late-subscriber fill loss.
    let mut fills_rx = bus.fills();

    let feed = Arc::new(MockFeed::new(fixture_ticks(), BAR_PACE, Venue::Binance));
    let mut set: JoinSet<()> = JoinSet::new();
    let cancel = CancellationToken::new();

    spawn_trading_loop(
        feed as Arc<dyn data::MarketDataSource>,
        Arc::clone(&bus),
        registry,
        &gate_backtest_cfg(),
        &gate_risk_cfg(),
        symbol,
        Timeframe::OneSecond,
        None,       // no equity store
        "research", // no journal / lesson-card side effects to fake
        &mut set,
        &cancel,
        None,
        None,
        vec![],
        Some(Money::<Usdt>::from_decimal(GATE_BUDGET)),
        short_enabled,
    );

    let mut tape: Vec<Fill> = Vec::new();
    loop {
        match timeout(RUN_QUIESCENCE, fills_rx.recv()).await {
            Ok(Ok(f)) => tape.push(f),
            Ok(Err(RecvError::Closed)) => break,
            Ok(Err(RecvError::Lagged(n))) => panic!(
                "GATE INSTRUMENTATION FAILURE ({arm}) — the fill broadcast lagged by {n}; \
                 the measured tape would be incomplete and the parity ratio meaningless"
            ),
            // Quiescence: the bar stream is exhausted and the loop is idle.
            Err(_elapsed) => break,
        }
    }

    cancel.cancel();
    set.shutdown().await;
    tape
}

// ── Friction measurement ─────────────────────────────────────────────────────

/// Friction over a run's fill tape, split into its two components.
///
/// **Each component is normalised by the notional it is actually charged on.**
/// Charging both against one denominator makes the aggregate ratio depend on
/// the run's Buy/Sell mix — the engine applies the fee to the *slipped*
/// notional, and slippage moves price up on a Buy and down on a Sell — so a
/// long-only and a short-enabled tape would differ by ~1e-8 even under perfect
/// parity. That is a tolerance-shaped hole the same size as the effect being
/// measured. (The `backtest` sibling hit exactly that; the split is the fix.)
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

    fn per_unit_notional(&self) -> Decimal {
        self.slippage_per_notional() + self.fee_per_notional()
    }
}

fn measure(tape: &[Fill]) -> Friction {
    let mut slippage_cost = Decimal::ZERO;
    let mut mark_notional = Decimal::ZERO;
    let mut fee = Decimal::ZERO;
    let mut fill_notional = Decimal::ZERO;
    for f in tape {
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
        fills: tape.len(),
    }
}

/// Reconstruct the signed position from the fill tape: Buy adds, Sell
/// subtracts. Returns `(short_leg_fills, most_negative_position)`.
///
/// A fill is a **short leg** when it moves the position at or below zero: a
/// Sell from a flat/short position (open/extend) or a Buy while short (cover).
fn short_leg_census(tape: &[Fill]) -> (usize, Decimal) {
    let mut pos = Decimal::ZERO;
    let mut short_legs = 0usize;
    let mut most_negative = Decimal::ZERO;
    for f in tape {
        let qty = f.qty.get();
        let before = pos;
        match f.side {
            Side::Buy => pos += qty,
            Side::Sell => pos -= qty,
        }
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
fn assert_actually_shorted(tape: &[Fill], label: &str) -> usize {
    let (short_legs, most_negative) = short_leg_census(tape);
    assert!(
        most_negative < Decimal::ZERO,
        "VACUOUS GATE ({label}) — the short-enabled arm never held a negative \
         position across {} forward-loop fills, so this file compares long legs \
         to long legs and would stay green with the #80 bypass fully restored. \
         Fix the fixture, not the assertion.",
        tape.len()
    );
    assert!(
        short_legs >= MIN_SHORT_LEG_FILLS,
        "VACUOUS GATE ({label}) — only {short_legs} of {} fills are short legs \
         (need >= {MIN_SHORT_LEG_FILLS}); the parity ratio would be dominated by \
         the long legs and could hide a short-leg friction regression",
        tape.len()
    );
    short_legs
}

// ═════════════════════════════════════════════════════════════════════════════
// The gates
// ═════════════════════════════════════════════════════════════════════════════

/// **The headline gate.** Same strategy, same symmetric fixture, same
/// production forward loop, long-only vs short-enabled: friction per unit
/// notional must match.
#[tokio::test(start_paused = true)]
async fn forward_loop_long_and_short_legs_pay_identical_friction_per_unit_notional() {
    let long_only = run_forward(LONG_ONLY_ARM).await;
    let short_enabled = run_forward(SHORT_ENABLED_ARM).await;

    assert!(
        !long_only.is_empty(),
        "the fixture must make {LONG_ONLY_ARM} trade in the forward loop"
    );
    let short_legs = assert_actually_shorted(&short_enabled, SHORT_ENABLED_ARM);

    let fl = measure(&long_only);
    let fs = measure(&short_enabled);
    let rl = fl.per_unit_notional();
    let rs = fs.per_unit_notional();

    assert!(
        rl > Decimal::ZERO,
        "TRIVIAL GATE — the long-only run paid ZERO friction per unit notional"
    );
    assert!(
        fl.slippage_per_notional() > Decimal::ZERO,
        "TRIVIAL GATE — the long-only run paid ZERO slippage, so the component \
         the #80 bypass actually skipped is not being measured at all"
    );

    // Both components are per-fill constants applied by `PaperEngine::step`, so
    // each aggregate rate is exact and equal across the two runs however
    // differently their trade sequences unfold. The tolerance absorbs Decimal
    // division rounding only.
    let tolerance = dec!(0.000000000000000001);
    let difference = (rl - rs).abs();
    assert!(
        difference <= tolerance,
        "FORWARD FRICTION PARITY FAIL — in the loop that executes the operator's \
         actual plan, the short legs do not pay what the long legs pay!\n  \
         long-only  ({LONG_ONLY_ARM})            : friction/notional = {rl}  \
         (slippage {} + fee {})\n  \
         short-enabled ({SHORT_ENABLED_ARM}) : friction/notional = {rs}  \
         (slippage {} + fee {})\n  \
         difference = {difference} (tolerance {tolerance})\n  \
         short-leg fills in the short run: {short_legs} of {}\n\n\
         This is bug-log #80 in `crates/agent/src/runtime.rs` (reachability-map \
         N-2): a `continue` past `PaperEngine::step` makes the short legs skip \
         slippage and the fill-price model while the long legs pay both — on the \
         path that decides what the operator's paper account actually does, not \
         merely what gets recommended.",
        fl.slippage_per_notional(),
        fl.fee_per_notional(),
        fs.slippage_per_notional(),
        fs.fee_per_notional(),
        fs.fills
    );

    println!(
        "forward friction parity: long_only={rl} (slippage {} + fee {}, {} fills) \
         short_enabled={rs} (slippage {} + fee {}, {} fills, {short_legs} short legs)",
        fl.slippage_per_notional(),
        fl.fee_per_notional(),
        fl.fills,
        fs.slippage_per_notional(),
        fs.fee_per_notional(),
        fs.fills
    );
}

/// Mechanism 1 — **slippage**: every forward-loop fill on BOTH runs is priced
/// at the bar mark moved by exactly `slippage_bps`, in the direction that costs
/// money. The pre-fix short legs were priced at the raw `bar.close`.
#[tokio::test(start_paused = true)]
async fn forward_loop_every_fill_price_carries_the_engine_slippage() {
    for arm in [LONG_ONLY_ARM, SHORT_ENABLED_ARM] {
        let tape = run_forward(arm).await;
        assert!(!tape.is_empty(), "{arm} must trade in the forward loop");
        if arm == SHORT_ENABLED_ARM {
            assert_actually_shorted(&tape, arm);
        }
        for (i, f) in tape.iter().enumerate() {
            let mark = mark_at(f.venue_ts);
            let expected = match f.side {
                Side::Buy => mark * (Decimal::ONE + SLIPPAGE_BPS / TEN_K),
                Side::Sell => mark * (Decimal::ONE - SLIPPAGE_BPS / TEN_K),
            };
            assert_eq!(
                f.price.get(),
                expected,
                "FORWARD SLIPPAGE PARITY FAIL ({arm}) — fill {i} ({:?}) priced at {}, \
                 expected {expected} (mark {mark} ± {SLIPPAGE_BPS} bps). A fill priced AT \
                 the mark is a leg that never went through `PaperEngine::step` — \
                 bug-log #80 / reachability-map N-2.",
                f.side,
                f.price.get()
            );
        }
    }
}

/// Mechanism 2 — **fee**: every forward-loop fill on BOTH runs is charged
/// `taker_fee_bps` on its *slipped* notional.
///
/// The pre-fix short legs DID pay the fee — on their own un-slipped notional,
/// self-consistently — so this test stays GREEN under the #80 mutation, and
/// that is correct: bug-log #80's table says the fee was the one effect the
/// bypass did charge. It earns its place by pinning the *base* the fee is
/// charged on, which the bypass got wrong.
#[tokio::test(start_paused = true)]
async fn forward_loop_every_fill_pays_the_taker_fee_on_its_slipped_notional() {
    for arm in [LONG_ONLY_ARM, SHORT_ENABLED_ARM] {
        let tape = run_forward(arm).await;
        assert!(!tape.is_empty(), "{arm} must trade in the forward loop");
        for (i, f) in tape.iter().enumerate() {
            let expected = f.price.get() * f.qty.get() * (TAKER_FEE_BPS / TEN_K);
            assert_eq!(
                f.fee.amount(),
                expected,
                "FORWARD FEE PARITY FAIL ({arm}) — fill {i} paid {} on notional {}, \
                 expected {expected}",
                f.fee.amount(),
                f.price.get() * f.qty.get()
            );
        }
    }
}

/// The **cross-crate consistency** assertion the two halves of #80 exist for:
/// the forward loop must charge the same friction rate the ranking path
/// charges, because the ranking path is what justified promoting this arm.
///
/// This is deliberately expressed against the constants the ranking gate reads
/// from `MatchConfig` rather than against a second `run_scenario` invocation —
/// `backtest` is a dependency of `agent`, but re-running the whole bake-off
/// here would make this file a duplicate of its sibling rather than a bridge
/// between them.
#[tokio::test(start_paused = true)]
async fn forward_loop_friction_rate_equals_the_ranking_paths_configured_rate() {
    let tape = run_forward(SHORT_ENABLED_ARM).await;
    let short_legs = assert_actually_shorted(&tape, SHORT_ENABLED_ARM);
    let f = measure(&tape);

    let expected_slippage = SLIPPAGE_BPS / TEN_K;
    let expected_fee = TAKER_FEE_BPS / TEN_K;

    assert_eq!(
        f.slippage_per_notional(),
        expected_slippage,
        "the forward run's aggregate slippage rate ({}) is not the configured \
         {SLIPPAGE_BPS} bps ({expected_slippage}) — {short_legs} of {} fills are short \
         legs, so a shortfall here is short legs paying less than the backtest that \
         justified this arm charged them",
        f.slippage_per_notional(),
        f.fills
    );
    assert_eq!(
        f.fee_per_notional(),
        expected_fee,
        "the forward run's aggregate fee rate ({}) is not the configured \
         {TAKER_FEE_BPS} bps ({expected_fee})",
        f.fee_per_notional()
    );
}
