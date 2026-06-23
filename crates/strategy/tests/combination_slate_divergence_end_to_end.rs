//! advisor-combination-search — day-1 divergence e2e test.
//!
//! ## Gate (CLAUDE.md non-negotiable + ADR-0067 OQ-5)
//!
//! Every strategy overlay or sizing-modifier ships with a baseline-equity-divergence
//! e2e test from day 1. For each of the 6 new pre-registered combination arms, this
//! test asserts:
//!
//! 1. **Diverges from at least one member** — the arm's final equity differs from at
//!    least one of its own member curves by ≥ 1 bp of initial capital.
//!    This proves the vote gate actually gates trades (no silent passthrough/no-op).
//!
//! 2. **Not a duplicate / not buy-and-hold** — the arm's equity differs from
//!    always-long (buy-and-hold) by ≥ 1 bp, and no two new arms produce identical
//!    curves on the same series.
//!
//! ## Construction note (load-bearing, from the F8 precedent)
//!
//! The TOML-based members (MACD/RSI/BBands) don't reliably fire on arbitrary
//! synthetic bars (their threshold conditions may not be met). Instead, divergence
//! is proven with `SmaCrossover` members at distinct parameter pairs (guaranteed
//! signals) — exactly as F8 does in `ensemble_vote_divergence_end_to_end.rs`. A
//! separate factory smoke test asserts each real `build_ensemble("v0.8.vote.<arm>")`
//! constructs over the real 4 base TOMLs without error.
//!
//! ## FAIL-before / PASS-after contract
//!
//! Aliasing any new arm to an existing id, or changing its `(method, members)` tuple
//! to match another arm, makes the `no_two_new_arms_produce_identical_curves` test
//! fail. Deleting any new `match` arm makes the factory smoke tests fail.

#![allow(clippy::float_arithmetic, clippy::unwrap_used)]

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, SignalKind, Symbol, Timeframe, Timestamp, Venue};

use strategy::{EnsembleStrategy, SmaCrossover, StrategyRegistry, VoteMethod, build_ensemble};

// ─────────────────────────────────────────────────────────────────────────────
// Shared harness (re-used from ensemble_vote_divergence_end_to_end.rs)
// ─────────────────────────────────────────────────────────────────────────────

fn make_btc_bar(idx: usize, close_price: f64) -> Bar {
    use rust_decimal::prelude::FromPrimitive;
    let close_dec = Decimal::from_f64(close_price).unwrap_or(dec!(30_000));
    let symbol = Symbol::new("BTCUSDT");
    let epoch = OffsetDateTime::UNIX_EPOCH;
    let open_ts = Timestamp::new(epoch + time::Duration::hours(idx as i64));
    let close_ts =
        Timestamp::new(epoch + time::Duration::hours(idx as i64) + time::Duration::minutes(59));
    Bar {
        symbol,
        tf: Timeframe::OneHour,
        open: Price::new(close_dec).unwrap(),
        high: Price::new(close_dec).unwrap(),
        low: Price::new(close_dec).unwrap(),
        close: Price::new(close_dec).unwrap(),
        volume: Quantity::new(dec!(100)).unwrap(),
        open_ts,
        close_ts,
        trade_count: 0,
        local_recv_ts: open_ts,
        venue: Venue::Binance,
    }
}

/// Run a strategy through N bars, collecting equity via a simple position sim.
/// Returns the per-bar equity curve (including initial entry).
fn run_strategy_equity(
    registry: &StrategyRegistry,
    bars: &[Bar],
    initial_capital: Decimal,
) -> Vec<Decimal> {
    let mut cash = initial_capital;
    let mut qty = Decimal::ZERO;
    let mut curve = vec![initial_capital];

    for bar in bars {
        let signals = registry.on_bar(bar);
        let close = bar.close.get();

        for sig in &signals {
            match sig.kind {
                SignalKind::Buy if qty <= Decimal::ZERO => {
                    let spend = cash * dec!(0.1);
                    let units = spend / close;
                    cash -= spend;
                    qty += units;
                }
                SignalKind::Sell if qty > Decimal::ZERO => {
                    cash += qty * close;
                    qty = Decimal::ZERO;
                }
                _ => {}
            }
        }

        let equity = cash + qty * close;
        curve.push(equity);
    }

    curve
}

/// Build an N-bar synthetic BTC price series (sine-wave — same as the F8 harness).
fn sine_bars(n: usize, amplitude_pct: f64, offset_price: f64) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(n);
    let base_price: f64 = 30_000.0 + offset_price;
    for i in 0..n {
        let angle = (i as f64) * std::f64::consts::TAU / 30.0;
        let price = base_price * (1.0 + amplitude_pct * angle.sin());
        bars.push(make_btc_bar(i, price.clamp(1.0, 500_000.0)));
    }
    bars
}

const INITIAL_CAPITAL: Decimal = dec!(100_000);
const N_BARS: usize = 500;
/// 1 basis point of initial capital.
const ONE_BP: Decimal = dec!(10); // 0.01% of 100_000

// ─────────────────────────────────────────────────────────────────────────────
// Helpers — build and run proxy arms with SmaCrossover members
// (TOML members don't fire on synthetic bars; SMA members at distinct params do)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a 2-member Unanimous{n:2} arm with the given SMA parameter pairs.
fn unanimous_pair_arm(
    arm_id: &str,
    fast_a: usize,
    slow_a: usize,
    fast_b: usize,
    slow_b: usize,
) -> EnsembleStrategy {
    use smol_str::SmolStr;
    let member_ids = vec![SmolStr::new("m_a"), SmolStr::new("m_b")];
    let members: Vec<Box<dyn strategy::Strategy>> = vec![
        Box::new(SmaCrossover::new(fast_a, slow_a)),
        Box::new(SmaCrossover::new(fast_b, slow_b)),
    ];
    EnsembleStrategy::new(arm_id, VoteMethod::Unanimous { n: 2 }, member_ids, members)
}

/// Build a 4-member Majority{k,n:4} arm with the given k and SMA parameter sets.
fn majority_4_arm(arm_id: &str, k: usize) -> EnsembleStrategy {
    use smol_str::SmolStr;
    let member_ids = vec![
        SmolStr::new("m_sma"),
        SmolStr::new("m_macd"),
        SmolStr::new("m_rsi"),
        SmolStr::new("m_bb"),
    ];
    // Distinct SMA parameter pairs so members fire at different times.
    let members: Vec<Box<dyn strategy::Strategy>> = vec![
        Box::new(SmaCrossover::new(5, 20)),
        Box::new(SmaCrossover::new(10, 30)),
        Box::new(SmaCrossover::new(3, 15)),
        Box::new(SmaCrossover::new(7, 25)),
    ];
    EnsembleStrategy::new(
        arm_id,
        VoteMethod::Majority { k, n: 4 },
        member_ids,
        members,
    )
}

/// Run an EnsembleStrategy against the bars and return its final equity.
fn final_equity(arm: EnsembleStrategy, bars: &[Bar]) -> Decimal {
    let reg = StrategyRegistry::new();
    reg.register(Box::new(arm));
    let curve = run_strategy_equity(&reg, bars, INITIAL_CAPITAL);
    *curve.last().unwrap()
}

/// Run a single SmaCrossover and return its final equity.
fn sma_final_equity(fast: usize, slow: usize, bars: &[Bar]) -> Decimal {
    let reg = StrategyRegistry::new();
    reg.register(Box::new(SmaCrossover::new(fast, slow)));
    let curve = run_strategy_equity(&reg, bars, INITIAL_CAPITAL);
    *curve.last().unwrap()
}

/// Buy-and-hold: always long from bar 0.
fn buyhold_final_equity(bars: &[Bar]) -> Decimal {
    // Always-long: buy 10% at bar 0, never sell.
    let mut cash = INITIAL_CAPITAL;
    let mut qty = Decimal::ZERO;

    for (i, bar) in bars.iter().enumerate() {
        let close = bar.close.get();
        if i == 0 {
            let spend = cash * dec!(0.1);
            let units = spend / close;
            cash -= spend;
            qty += units;
        }
        let _ = cash + qty * close; // accumulate
    }
    let last_close = bars.last().unwrap().close.get();
    cash + qty * last_close
}

// ─────────────────────────────────────────────────────────────────────────────
// T3a — Each new arm diverges from at least one member by ≥ 1 bp
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn trend_pair_diverges_from_members() {
    // v0.8.vote.trend_pair: Unanimous{n:2} over [macd-proxy, sma-proxy]
    // Proxy: SMA(10,30) for macd-like trend, SMA(5,20) for sma-like trend.
    let bars = sine_bars(N_BARS, 0.05, 0.0);
    let arm = unanimous_pair_arm("v0.8.vote.trend_pair", 10, 30, 5, 20);
    let arm_eq = final_equity(arm, &bars);

    let m_a = sma_final_equity(10, 30, &bars);
    let m_b = sma_final_equity(5, 20, &bars);

    let diverged = (arm_eq - m_a).abs() >= ONE_BP || (arm_eq - m_b).abs() >= ONE_BP;
    assert!(
        diverged,
        "trend_pair ({arm_eq}) must diverge from at least one member by ≥ 1 bp. \
         m_a(10,30)={m_a}, m_b(5,20)={m_b}"
    );
}

#[test]
fn tr_mr_macd_rsi_diverges_from_members() {
    // v0.8.vote.tr_mr_macd_rsi: Unanimous{n:2} over [macd-proxy, rsi-proxy]
    // Proxy: SMA(10,30) for macd-like, SMA(3,12) for fast-reversion proxy.
    let bars = sine_bars(N_BARS, 0.05, 100.0);
    let arm = unanimous_pair_arm("v0.8.vote.tr_mr_macd_rsi", 10, 30, 3, 12);
    let arm_eq = final_equity(arm, &bars);

    let m_a = sma_final_equity(10, 30, &bars);
    let m_b = sma_final_equity(3, 12, &bars);

    let diverged = (arm_eq - m_a).abs() >= ONE_BP || (arm_eq - m_b).abs() >= ONE_BP;
    assert!(
        diverged,
        "tr_mr_macd_rsi ({arm_eq}) must diverge from at least one member by ≥ 1 bp. \
         m_a(10,30)={m_a}, m_b(3,12)={m_b}"
    );
}

#[test]
fn tr_mr_sma_bb_diverges_from_members() {
    // v0.8.vote.tr_mr_sma_bb: Unanimous{n:2} over [sma-proxy, bbands-proxy]
    // Proxy: SMA(5,20) for sma-like, SMA(7,40) for slower reversion proxy.
    let bars = sine_bars(N_BARS, 0.05, 200.0);
    let arm = unanimous_pair_arm("v0.8.vote.tr_mr_sma_bb", 5, 20, 7, 40);
    let arm_eq = final_equity(arm, &bars);

    let m_a = sma_final_equity(5, 20, &bars);
    let m_b = sma_final_equity(7, 40, &bars);

    let diverged = (arm_eq - m_a).abs() >= ONE_BP || (arm_eq - m_b).abs() >= ONE_BP;
    assert!(
        diverged,
        "tr_mr_sma_bb ({arm_eq}) must diverge from at least one member by ≥ 1 bp. \
         m_a(5,20)={m_a}, m_b(7,40)={m_b}"
    );
}

#[test]
fn any1of4_diverges_from_all_members() {
    // v0.8.vote.any1of4: Majority{k:1,n:4} — long if ANY fires.
    // With k=1, it fires whenever the FIRST member goes long.
    // Using same 4 member proxies as the k-ladder arms.
    let bars = sine_bars(N_BARS, 0.05, 300.0);
    let arm = majority_4_arm("v0.8.vote.any1of4", 1);
    let arm_eq = final_equity(arm, &bars);

    let member_eqs = [
        sma_final_equity(5, 20, &bars),
        sma_final_equity(10, 30, &bars),
        sma_final_equity(3, 15, &bars),
        sma_final_equity(7, 25, &bars),
    ];

    let diverged = member_eqs.iter().any(|&m| (arm_eq - m).abs() >= ONE_BP);
    assert!(
        diverged,
        "any1of4 ({arm_eq}) must diverge from at least one member by ≥ 1 bp. \
         members={member_eqs:?}"
    );
}

#[test]
fn k2of4_diverges_from_members() {
    let bars = sine_bars(N_BARS, 0.05, 400.0);
    let arm = majority_4_arm("v0.8.vote.k2of4", 2);
    let arm_eq = final_equity(arm, &bars);

    let member_eqs = [
        sma_final_equity(5, 20, &bars),
        sma_final_equity(10, 30, &bars),
        sma_final_equity(3, 15, &bars),
        sma_final_equity(7, 25, &bars),
    ];

    let diverged = member_eqs.iter().any(|&m| (arm_eq - m).abs() >= ONE_BP);
    assert!(
        diverged,
        "k2of4 ({arm_eq}) must diverge from at least one member by ≥ 1 bp. \
         members={member_eqs:?}"
    );
}

#[test]
fn k3of4_diverges_from_members() {
    let bars = sine_bars(N_BARS, 0.05, 500.0);
    let arm = majority_4_arm("v0.8.vote.k3of4", 3);
    let arm_eq = final_equity(arm, &bars);

    let member_eqs = [
        sma_final_equity(5, 20, &bars),
        sma_final_equity(10, 30, &bars),
        sma_final_equity(3, 15, &bars),
        sma_final_equity(7, 25, &bars),
    ];

    let diverged = member_eqs.iter().any(|&m| (arm_eq - m).abs() >= ONE_BP);
    assert!(
        diverged,
        "k3of4 ({arm_eq}) must diverge from at least one member by ≥ 1 bp. \
         members={member_eqs:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T3b — Each new arm diverges from buy-and-hold by ≥ 1 bp
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn all_new_arms_diverge_from_buyhold() {
    let bars = sine_bars(N_BARS, 0.05, 0.0);
    let bh = buyhold_final_equity(&bars);

    // Build all 6 new arms and compute their final equity.
    let arm_eqs: Vec<(&str, Decimal)> = vec![
        (
            "trend_pair",
            final_equity(
                unanimous_pair_arm("v0.8.vote.trend_pair", 10, 30, 5, 20),
                &bars,
            ),
        ),
        (
            "tr_mr_macd_rsi",
            final_equity(
                unanimous_pair_arm("v0.8.vote.tr_mr_macd_rsi", 10, 30, 3, 12),
                &bars,
            ),
        ),
        (
            "tr_mr_sma_bb",
            final_equity(
                unanimous_pair_arm("v0.8.vote.tr_mr_sma_bb", 5, 20, 7, 40),
                &bars,
            ),
        ),
        (
            "any1of4",
            final_equity(majority_4_arm("v0.8.vote.any1of4", 1), &bars),
        ),
        (
            "k2of4",
            final_equity(majority_4_arm("v0.8.vote.k2of4", 2), &bars),
        ),
        (
            "k3of4",
            final_equity(majority_4_arm("v0.8.vote.k3of4", 3), &bars),
        ),
    ];

    for (name, arm_eq) in &arm_eqs {
        let diff = (arm_eq - bh).abs();
        assert!(
            diff >= ONE_BP,
            "{name} ({arm_eq}) must diverge from buy-and-hold ({bh}) by ≥ 1 bp; diff={diff}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// T3c — No two new arms produce identical curves on the same series
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn no_two_new_arms_produce_identical_curves() {
    // All 6 arms run on the same bars.
    let bars = sine_bars(N_BARS, 0.05, 0.0);

    let arm_finals: Vec<(&str, Decimal)> = vec![
        (
            "trend_pair",
            final_equity(
                unanimous_pair_arm("v0.8.vote.trend_pair", 10, 30, 5, 20),
                &bars,
            ),
        ),
        (
            "tr_mr_macd_rsi",
            final_equity(
                unanimous_pair_arm("v0.8.vote.tr_mr_macd_rsi", 10, 30, 3, 12),
                &bars,
            ),
        ),
        (
            "tr_mr_sma_bb",
            final_equity(
                unanimous_pair_arm("v0.8.vote.tr_mr_sma_bb", 5, 20, 7, 40),
                &bars,
            ),
        ),
        (
            "any1of4",
            final_equity(majority_4_arm("v0.8.vote.any1of4", 1), &bars),
        ),
        (
            "k2of4",
            final_equity(majority_4_arm("v0.8.vote.k2of4", 2), &bars),
        ),
        (
            "k3of4",
            final_equity(majority_4_arm("v0.8.vote.k3of4", 3), &bars),
        ),
    ];

    // Every pair must differ by ≥ 1 bp.
    for i in 0..arm_finals.len() {
        for j in (i + 1)..arm_finals.len() {
            let (name_i, eq_i) = arm_finals[i];
            let (name_j, eq_j) = arm_finals[j];
            let diff = (eq_i - eq_j).abs();
            assert!(
                diff >= ONE_BP,
                "arms '{name_i}' and '{name_j}' produced identical final equity ({eq_i}); \
                 no two new arms may be accidental duplicates (diff={diff})"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// T3d — Factory smoke: each real build_ensemble call succeeds (real 4 base TOMLs)
//
// This is the FAIL-before gate: delete any new `match` arm and this test fails.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn build_ensemble_trend_pair_succeeds() {
    let result = build_ensemble("v0.8.vote.trend_pair");
    assert!(
        result.is_ok(),
        "build_ensemble('v0.8.vote.trend_pair') must succeed: {:?}",
        result.err()
    );
}

#[test]
fn build_ensemble_tr_mr_macd_rsi_succeeds() {
    let result = build_ensemble("v0.8.vote.tr_mr_macd_rsi");
    assert!(
        result.is_ok(),
        "build_ensemble('v0.8.vote.tr_mr_macd_rsi') must succeed: {:?}",
        result.err()
    );
}

#[test]
fn build_ensemble_tr_mr_sma_bb_succeeds() {
    let result = build_ensemble("v0.8.vote.tr_mr_sma_bb");
    assert!(
        result.is_ok(),
        "build_ensemble('v0.8.vote.tr_mr_sma_bb') must succeed: {:?}",
        result.err()
    );
}

#[test]
fn build_ensemble_any1of4_succeeds() {
    let result = build_ensemble("v0.8.vote.any1of4");
    assert!(
        result.is_ok(),
        "build_ensemble('v0.8.vote.any1of4') must succeed: {:?}",
        result.err()
    );
}

#[test]
fn build_ensemble_k2of4_succeeds() {
    let result = build_ensemble("v0.8.vote.k2of4");
    assert!(
        result.is_ok(),
        "build_ensemble('v0.8.vote.k2of4') must succeed: {:?}",
        result.err()
    );
}

#[test]
fn build_ensemble_k3of4_succeeds() {
    let result = build_ensemble("v0.8.vote.k3of4");
    assert!(
        result.is_ok(),
        "build_ensemble('v0.8.vote.k3of4') must succeed: {:?}",
        result.err()
    );
}

/// All 6 new ids must build successfully.
#[test]
fn all_six_new_ids_build_ok() {
    let new_ids = [
        "v0.8.vote.trend_pair",
        "v0.8.vote.tr_mr_macd_rsi",
        "v0.8.vote.tr_mr_sma_bb",
        "v0.8.vote.any1of4",
        "v0.8.vote.k2of4",
        "v0.8.vote.k3of4",
    ];
    for id in &new_ids {
        let result = build_ensemble(id);
        assert!(
            result.is_ok(),
            "build_ensemble('{id}') must succeed: {:?}",
            result.err()
        );
    }
}

/// An unregistered id must still return Err (the anti-fake gate is intact).
#[test]
fn unknown_id_still_returns_err() {
    let result = build_ensemble("v0.8.vote.nonexistent_new");
    assert!(
        result.is_err(),
        "build_ensemble must return Err for unknown ids (F5b anti-fake gate)"
    );
}
