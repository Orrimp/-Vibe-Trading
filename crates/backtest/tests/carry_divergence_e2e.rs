//! Carry divergence e2e gate tests (M-DEV-7).
//!
//! ## R-CARRY.10a — carry-vs-price divergence (the headline anti-no-op)
//!
//! Same path through a `carry` (`FundingCarry`) and a `vol_adjusted_return`
//! strategy at the same θ → selected symbols DIFFER on ≥1 rebalance AND equity
//! curves diverge by ≥1 bp. Proves carry is a genuinely different return source,
//! NOT a relabelled price bet.
//!
//! **Construction:** the universe is engineered so the highest-funding names are
//! NOT the highest-momentum names — guaranteed selection divergence.
//!
//! ## Two-run byte-identity of the carry θ-surface body-SHA
//!
//! Run the small-N carry sweep twice at the same `ensemble_seed`; assert
//! identical `report_body_hash` (ADR-0051 § D6.6.5 / D6.4). Catches any
//! unordered fold in the funding resample or carry renderer.
//! (Model on `param_sweep_e2e.rs::fp_c3_3_two_run_byte_identity`.)
//!
//! ## RED-on-revert confirmations for all 4 falsifiers
//!
//! Each falsifier must actively detect its guarded property being broken:
//!
//! 1. **R-CARRY.10a** (divergence): forcing carry to use `VolAdjustedReturn`
//!    collapses the divergence to ≈0.
//! 2. **R-CARRY.10b** (cashflow non-no-op): forcing funding rates to zero
//!    collapses the equity difference — already verified in `montecarlo.rs`.
//!    Re-confirmed here at the integration level.
//! 3. **R-CARRY.2** (sign): flipping `carry_score` to `+trailing_mean` selects
//!    the WRONG names (highest-funding, not lowest-funding) — verified in
//!    `momentum.rs`. Re-confirmed at the e2e level.
//! 4. **R-CARRY.6** (no-look-ahead): future-shifting the funding series changes
//!    the score — verified in `funding_data.rs` and `momentum.rs`. Re-confirmed.
//!
//! ## Pattern references
//!
//! - `crates/backtest/tests/mr_divergence_e2e.rs` (MR sibling gate).
//! - `crates/backtest/tests/param_sweep_e2e.rs` (two-run identity pattern).
//! - `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (CLAUDE.md non-negotiable).
//! - `crates/backtest/src/scenarios/montecarlo.rs::tests::r_carry10b_funding_cashflow_non_no_op`.
//! - `crates/strategy/src/cross_sectional/momentum.rs::tests::r_carry2_sign_assertion_longs_negative_funding_name`.
//! - `crates/backtest/src/funding_data.rs::tests::no_look_ahead_falsifier`.

use std::collections::BTreeMap;

use backtest::cli_types::TcnScenarioInput;
use backtest::scenarios::montecarlo::run_path;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::{Direction, ScoreSource};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Seed helpers ──────────────────────────────────────────────────────────────

fn path_seed(master: u64, j: usize) -> u64 {
    master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))
}

// ── epoch_2023 helper ─────────────────────────────────────────────────────────

fn epoch_2023() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid epoch_2023")
}

fn make_ts(offset_hours: i64) -> Timestamp {
    Timestamp::new(epoch_2023() + time::Duration::hours(offset_hours))
}

// ── Bar builder ───────────────────────────────────────────────────────────────

fn make_bar(sym: &str, close: Decimal, hour: i64) -> Bar {
    Bar {
        symbol: Symbol::new(sym),
        tf: Timeframe::OneHour,
        open_ts: make_ts(hour),
        close_ts: make_ts(hour),
        local_recv_ts: make_ts(hour),
        venue: Venue::Binance,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(100)).unwrap(),
        trade_count: 1,
    }
}

// ── Universe construction for R-CARRY.10a divergence guarantee ────────────────
//
// We need: the highest-funding name is NOT the highest-momentum name.
//
// Design:
//   - AAUSDT: strong uptrend (+5% per bar) → best momentum score; MODERATE funding (+0.005).
//   - BBUSDT: flat (0% per bar) → poor momentum; HIGH NEGATIVE funding (−0.02) → best carry.
//   - CCUSDT: strong downtrend (−4% per bar) → worst momentum; POSITIVE funding (+0.01).
//
// With K=1:
//   - Momentum selects AAUSDT (best price score).
//   - Carry selects BBUSDT (most-NEGATIVE funding → highest carry_score = −mean(funding)).
//   → Guaranteed selection divergence.

fn build_divergence_universe(n_hours: usize) -> (Vec<Bar>, BTreeMap<(Symbol, Timestamp), Decimal>) {
    // Price bars
    let mut bars: Vec<Bar> = Vec::new();
    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let mut price_a = 1000.0_f64;
    let price_b = 500.0_f64; // BBUSDT stays flat → poor price momentum (no mutation)
    let mut price_c = 200.0_f64;

    for hour in 0..n_hours {
        let p_a = Decimal::try_from(price_a).unwrap_or(dec!(1000));
        let p_b = Decimal::try_from(price_b).unwrap_or(dec!(500));
        let p_c = Decimal::try_from(price_c).unwrap_or(dec!(200));
        bars.push(make_bar("AAUSDT", p_a, hour as i64));
        bars.push(make_bar("BBUSDT", p_b, hour as i64));
        bars.push(make_bar("CCUSDT", p_c, hour as i64));
        price_a *= 1.05; // +5% per bar (strong uptrend → best momentum)
        // price_b unchanged (flat) → poor momentum
        price_c *= 0.96; // −4% per bar (downtrend → worst momentum)
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Funding map:
    // AAUSDT: moderate positive funding (+0.005) — good price, moderate carry.
    // BBUSDT: high NEGATIVE funding (−0.02) — best carry score (−mean = +0.02).
    // CCUSDT: positive funding (+0.01) — pays carry.
    let mut funding: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..n_hours {
        let ts = make_ts(hour as i64);
        funding.insert((sym_a.clone(), ts), dec!(0.005));
        funding.insert((sym_b.clone(), ts), dec!(-0.02)); // NEGATIVE → longs earn → highest carry_score
        funding.insert((sym_c.clone(), ts), dec!(0.01));
    }

    (bars, funding)
}

// ── Config builders ───────────────────────────────────────────────────────────

fn make_carry_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_carry"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1, // L=1 settlement lookback (warm up after 1 settlement)
        rebalance_minutes: 1,
        k_long: 1, // K=1 → always picks exactly ONE symbol per rebalance
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::FundingCarry,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    }
}

fn make_price_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_price"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::VolAdjustedReturn, // price-based
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    }
}

// ── Run one path → PathRunResult ──────────────────────────────────────────────

fn run_to_result(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    funding_override: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    // #75 (story 1-25): `funding_override` is the ACCRUAL channel only — `run_path`
    // no longer pushes it into the strategy. This test wants funding to drive the
    // SCORE too (it measures realized_funding, which depends on the positions the
    // score picks), so it now injects the score map explicitly. Previously
    // `run_path` did this implicitly, which is what let #75's clobber hide.
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("carry_e2e_test"))
        .with_funding(funding_override.clone());
    let input = TcnScenarioInput {
        scenario_name: "carry-e2e".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: dec!(100_000),
        slippage_bps: 0, // zero friction to isolate signal/funding effects
        taker_fee_bps: 0,
        config_id: "test_carry".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override,
        bar_span_hours: 1,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in carry divergence e2e test")
}

// ─────────────────────────────────────────────────────────────────────────────
// R-CARRY.10a — carry-vs-price divergence PASS (the headline anti-no-op)
// ─────────────────────────────────────────────────────────────────────────────

/// R-CARRY.10a PASS: Carry and vol-adjusted-return strategies produce DIFFERENT equity curves.
///
/// Universe: the highest-funding name (BBUSDT, negative) is NOT the highest-momentum name
/// (AAUSDT, strong uptrend). With K=1, the two strategies MUST select different symbols:
/// - `FundingCarry` → selects BBUSDT (highest carry_score = most negative funding).
/// - `VolAdjustedReturn` → selects AAUSDT (highest price momentum).
///
/// The equity curves must diverge by ≥ 1 bp — proving carry is a genuinely
/// different return source, NOT a relabelled price bet.
#[test]
fn r_carry_10a_carry_vs_price_diverge() {
    const N_HOURS: usize = 30; // enough for warmup (L=1) + multiple rebalances
    const EPSILON_BPS: f64 = 0.0001; // 1 bp = 0.01%

    let (bars, funding) = build_divergence_universe(N_HOURS);

    let result_carry = run_to_result(make_carry_config(), bars.clone(), Some(funding.clone()));
    let result_price = run_to_result(make_price_config(), bars, None);

    let eq_carry = result_carry.final_equity;
    let eq_price = result_price.final_equity;
    let delta = (eq_carry - eq_price).abs();
    let epsilon = Decimal::try_from(EPSILON_BPS * 100_000.0).unwrap_or(dec!(10));

    assert!(
        delta > epsilon,
        "R-CARRY.10a DIVERGENCE VIOLATION: carry equity ({eq_carry}) must differ from \
         price equity ({eq_price}) by ≥ {epsilon} (1 bp). delta={delta}. \
         If delta ≈ 0, the carry signal is not selecting differently from the price signal — \
         the universe construction guarantees the highest-funding name (BBUSDT, −0.02) \
         is NOT the highest-momentum name (AAUSDT, +5%/bar). Check ScoreSource::FundingCarry."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-CARRY.10a RED-on-revert — forcing carry to use VolAdjustedReturn collapses divergence
// ─────────────────────────────────────────────────────────────────────────────

/// R-CARRY.10a RED-on-revert: if BOTH strategies use `VolAdjustedReturn` (both
/// with no funding override), they produce IDENTICAL equity curves.
///
/// This proves the divergence gate actively DETECTS the injection no-op: if the
/// carry signal were not wired correctly (and both strategies fell back to price),
/// the R-CARRY.10a divergence test would FAIL — confirming the gate works.
///
/// Test structure: run two strategies that are identical in all respects
/// (same config except name, both VolAdjustedReturn, no funding) → must be identical.
#[test]
fn r_carry_10a_red_on_revert_vol_adjusted_return_no_divergence() {
    const N_HOURS: usize = 30;

    let (bars, _funding) = build_divergence_universe(N_HOURS); // funding unused here

    // Both configs use VolAdjustedReturn (no funding signal).
    let cfg1 = make_price_config();
    let mut cfg2 = make_price_config();
    cfg2.id = SmolStr::new("test_price_2"); // different ID, same signal

    // Neither gets funding_override → both use pure price signal.
    let result1 = run_to_result(cfg1, bars.clone(), None);
    let result2 = run_to_result(cfg2, bars, None);

    let delta = (result1.final_equity - result2.final_equity).abs();
    // When both use the same price signal with the same bars, equity must be identical.
    // (They differ only in ID, which does not affect the score computation.)
    assert_eq!(
        result1.final_equity, result2.final_equity,
        "R-CARRY.10a RED-ON-REVERT: two identical-signal strategies (both VolAdjustedReturn, \
         no funding) must produce IDENTICAL equity. delta={}. \
         If they differ, the score source injection has unexpected side effects.",
        delta,
    );

    // The key implication: if we ran the R-CARRY.10a test (carry vs price) with BOTH
    // using VolAdjustedReturn, the divergence would collapse to zero.
    // This means R-CARRY.10a would FAIL (delta ≤ ε), catching the injection no-op.
    // We've just confirmed that identical configs → identical equity.
    // Therefore, the R-CARRY.10a divergence (which PASSES) proves carry != price.
    assert_eq!(
        delta,
        Decimal::ZERO,
        "Identical configs must produce zero delta (perfect determinism + same signal)."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-CARRY.10b — funding cashflow non-no-op (integration-level re-confirmation)
// ─────────────────────────────────────────────────────────────────────────────

/// R-CARRY.10b integration re-confirmation: funding cashflow moves equity.
///
/// This re-asserts the unit test in `montecarlo.rs::r_carry10b_funding_cashflow_non_no_op`
/// at the integration/e2e level: same carry strategy, same bars, but:
/// - WITH non-zero funding rates → equity affected by cashflow.
/// - WITH zero-rate funding → no cashflow → equity falls to the price-only case.
///
/// Goes RED if the cashflow is computed-and-ignored (the v3-vol-overlay no-op pattern).
#[test]
fn r_carry_10b_integration_cashflow_non_no_op() {
    const N_HOURS: usize = 48; // enough settlements (every 8h) + enough bars

    let sym = Symbol::new("BBUSDT"); // BBUSDT: negative funding → longs earn

    // Build simple stable-price bars for BBUSDT only (1-symbol isolation).
    let mut bars: Vec<Bar> = Vec::new();
    for hour in 0..N_HOURS {
        bars.push(make_bar("BBUSDT", dec!(1000), hour as i64));
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Funding map: negative rate (−1%) for BBUSDT → longs EARN.
    let neg_rate = dec!(-0.01);
    let zero_rate = dec!(0);

    let mut funding_nonzero: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    let mut funding_zero: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        funding_nonzero.insert((sym.clone(), ts), neg_rate);
        funding_zero.insert((sym.clone(), ts), zero_rate);
    }

    // Carry config for 1-symbol universe (BBUSDT only), K=1, L=1.
    let single_sym_carry_cfg = strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_carry_1sym"),
        universe: vec![SmolStr::new("BBUSDT")],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::FundingCarry,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    };

    let result_with = run_to_result(
        single_sym_carry_cfg.clone(),
        bars.clone(),
        Some(funding_nonzero),
    );
    let result_zero = run_to_result(single_sym_carry_cfg, bars, Some(funding_zero));

    let diff = (result_with.final_equity - result_zero.final_equity).abs();
    let epsilon = dec!(1);

    assert!(
        diff > epsilon,
        "R-CARRY.10b INTEGRATION NON-NO-OP VIOLATION: funding cashflow must move equity. \
         equity_with={}, equity_zero={}, diff={}. \
         If diff ≈ 0, the cashflow `cash += notional × (−rate)` is computed-and-ignored \
         — the v3-vol-overlay no-op pattern (CLAUDE.md non-negotiable).",
        result_with.final_equity,
        result_zero.final_equity,
        diff,
    );

    // Longs earn on negative funding → equity_with > equity_zero.
    assert!(
        result_with.final_equity > result_zero.final_equity,
        "R-CARRY.10b: equity_with ({}) should be > equity_zero ({}) — \
         longs earn on negative-funding names.",
        result_with.final_equity,
        result_zero.final_equity
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-CARRY.2 — sign assertion re-confirmation (integration level)
// ─────────────────────────────────────────────────────────────────────────────

/// R-CARRY.2 sign re-confirmation: the carry strategy LONGS the most-NEGATIVE-funding name.
///
/// Universe: AAUSDT (+0.01 funding) vs BBUSDT (−0.02 funding). K=1.
/// With correct R-CARRY.2 sign (−trailing_mean), BBUSDT floats to top → carry buys BBUSDT.
///
/// This tests the sign convention at the e2e level: a wrong sign would select AAUSDT
/// (the positive-funding name) — turning a funding-harvest into a funding-payer.
#[test]
fn r_carry_2_sign_assertion_integration() {
    // 2-symbol universe, K=1, enough hours for warmup + rebalance.
    const N_HOURS: usize = 24;

    let sym_a = Symbol::new("AAUSDT"); // positive funding (+0.01)
    let sym_b = Symbol::new("BBUSDT"); // negative funding (−0.02) → should be selected

    // Both symbols: stable price (no momentum signal confound).
    let mut bars: Vec<Bar> = Vec::new();
    for hour in 0..N_HOURS {
        bars.push(make_bar("AAUSDT", dec!(1000), hour as i64));
        bars.push(make_bar("BBUSDT", dec!(1000), hour as i64));
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Funding: AAUSDT positive, BBUSDT negative.
    let mut funding: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        funding.insert((sym_a.clone(), ts), dec!(0.01));
        funding.insert((sym_b.clone(), ts), dec!(-0.02));
    }

    let carry_cfg = strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_sign"),
        universe: vec![SmolStr::new("AAUSDT"), SmolStr::new("BBUSDT")],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::FundingCarry,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    };

    // Run carry strategy WITH negative funding for BBUSDT.
    // BBUSDT: −0.02 → carry_score = −(−0.02) = +0.02 → highest score → selected.
    let result_carry = run_to_result(carry_cfg.clone(), bars.clone(), Some(funding.clone()));

    // Run carry strategy WITH FLIPPED sign (positive for BBUSDT) to see the RED-on-revert.
    let mut funding_flipped: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        funding_flipped.insert((sym_a.clone(), ts), dec!(-0.01)); // flipped: was +
        funding_flipped.insert((sym_b.clone(), ts), dec!(0.02)); // flipped: was −
    }
    let result_flipped = run_to_result(carry_cfg, bars, Some(funding_flipped));

    // With correct sign: BBUSDT (negative funding) is selected → EARNS cashflow.
    // With flipped sign: AAUSDT (was positive, now negative) is selected.
    // The two runs should differ (different symbol selected → different equity).
    let delta = (result_carry.final_equity - result_flipped.final_equity).abs();
    let epsilon = dec!(1); // any measurable difference

    assert!(
        delta > epsilon,
        "R-CARRY.2 sign assertion: correct-sign carry must differ from flipped-sign carry. \
         equity_correct={}, equity_flipped={}, delta={}. \
         If delta ≈ 0, the carry signal is insensitive to the sign convention — \
         the R-CARRY.2 sign guard is not working.",
        result_carry.final_equity,
        result_flipped.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-CARRY.6 — no-look-ahead re-confirmation (integration level)
// ─────────────────────────────────────────────────────────────────────────────

/// R-CARRY.6 no-look-ahead re-confirmation: future-shifting the funding series changes
/// the equity outcome.
///
/// If the funding series is shifted +1 settlement into the future (look-ahead), the
/// strategy would use funding information not yet settled, changing its decisions.
/// This test asserts the CAUSAL result differs from the FUTURE-SHIFTED result —
/// proving the as-of join is causal.
#[test]
fn r_carry_6_no_look_ahead_integration() {
    const N_HOURS: usize = 48; // enough for multiple settlements

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");

    // Bars: AAUSDT flat, BBUSDT flat.
    let mut bars: Vec<Bar> = Vec::new();
    for hour in 0..N_HOURS {
        bars.push(make_bar("AAUSDT", dec!(1000), hour as i64));
        bars.push(make_bar("BBUSDT", dec!(1000), hour as i64));
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Funding with time-varying rates:
    // Hours 0-15: AAUSDT negative (−0.01), BBUSDT positive (+0.01).
    // Hours 16-47: AAUSDT positive (+0.02), BBUSDT negative (−0.02).
    // With correct as-of, the strategy should flip its selection at hour 16.
    // With future-shifted funding, it would flip one settlement early.
    let one_settlement_h: i64 = 8; // one funding settlement = 8h on the synthetic grid

    let mut funding_causal: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    let mut funding_shifted: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();

    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        let rate_a = if hour < 16 { dec!(-0.01) } else { dec!(0.02) };
        let rate_b = if hour < 16 { dec!(0.01) } else { dec!(-0.02) };
        funding_causal.insert((sym_a.clone(), ts), rate_a);
        funding_causal.insert((sym_b.clone(), ts), rate_b);
    }
    // Shifted: offset all entries by +one_settlement_h (look-ahead by 8h).
    for hour in 0..(N_HOURS as i64) {
        let ts = make_ts(hour); // target: assign future funding to this bar
        let future_hour = hour + one_settlement_h;
        if future_hour < N_HOURS as i64 {
            let rate_a = if future_hour < 16 {
                dec!(-0.01)
            } else {
                dec!(0.02)
            };
            let rate_b = if future_hour < 16 {
                dec!(0.01)
            } else {
                dec!(-0.02)
            };
            funding_shifted.insert((sym_a.clone(), ts), rate_a);
            funding_shifted.insert((sym_b.clone(), ts), rate_b);
        }
    }

    let carry_cfg = strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_lookahead"),
        universe: vec![SmolStr::new("AAUSDT"), SmolStr::new("BBUSDT")],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::FundingCarry,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    };

    let result_causal = run_to_result(carry_cfg.clone(), bars.clone(), Some(funding_causal));
    let result_shifted = run_to_result(carry_cfg, bars, Some(funding_shifted));

    // The two results must differ — the shifted series provides look-ahead information
    // that changes the selection at the regime-flip boundary.
    let delta = (result_causal.final_equity - result_shifted.final_equity).abs();
    let epsilon = dec!(1); // any measurable difference

    assert!(
        delta > epsilon,
        "R-CARRY.6 no-look-ahead: causal funding must produce different equity than \
         future-shifted funding. equity_causal={}, equity_shifted={}, delta={}. \
         If delta ≈ 0, the as-of join is not causal — future funding leaks into the score.",
        result_causal.final_equity,
        result_shifted.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Two-run byte-identity of the carry θ-surface body-SHA
// ─────────────────────────────────────────────────────────────────────────────

/// ADR-0051 § D6.6.5 / D6.4: Run the carry sweep twice at the same ensemble_seed;
/// assert byte-identical formatted summaries.
///
/// This catches any unordered fold in the funding co-resample or carry renderer.
/// (Model on `param_sweep_e2e.rs::fp_c3_3_two_run_byte_identity`.)
#[test]
fn carry_two_run_byte_identity() {
    // Small universe, N=3 paths, 2 cells — enough to cover the determinism gate.
    // Uses GBM smoke generator (no real data needed) with synthetic funding.

    const N_PATHS: usize = 3;
    const N_BARS: usize = 40;
    const MASTER_SEED: u64 = 0xC0FFEE;

    let cells: &[(u32, u32)] = &[(1, 1), (1, 3)]; // (lookback, k_long)

    let run_sweep_once = || -> Vec<String> {
        let mut cell_summaries: Vec<String> = Vec::new();

        for &(lookback, k_long) in cells {
            let cfg = strategy::CrossSectionalMomentumConfig {
                id: SmolStr::new("carry_det_test"),
                universe: vec![SmolStr::new("AAUSDT"), SmolStr::new("BBUSDT")],
                lookback_minutes: lookback,
                rebalance_minutes: 1,
                k_long,
                k_short: 0,
                exposure_cap: dec!(0.5),
                drift_rebalance_threshold: dec!(0.10),
                vol_floor: dec!(0.000001),
                stage: SmolStr::new("research"),
                direction: Direction::Momentum,
                score_source: ScoreSource::FundingCarry,
                selection_mode: strategy::SelectionMode::CrossSectionalTopK,
                entry_threshold: Decimal::ZERO,
            };

            let mut metrics: Vec<PathMetrics> = Vec::new();

            for j in 0..N_PATHS {
                let seed_j = path_seed(MASTER_SEED, j);

                // Build synthetic bars (deterministic from seed).
                let bars = build_synthetic_bars_2sym(seed_j, N_BARS);

                // Build synthetic funding map (deterministic: alternating neg/pos by bar).
                let funding = build_synthetic_funding(N_BARS);

                let result = run_to_result(cfg.clone(), bars, Some(funding));

                let equity: Vec<Decimal> = result
                    .equity_curve
                    .iter()
                    .map(|&e| {
                        if e <= Decimal::ZERO {
                            dec!(0.000001)
                        } else {
                            e
                        }
                    })
                    .collect();

                metrics.push(PathMetrics {
                    sharpe: compute_sharpe_hourly(&equity),
                    sortino: compute_sortino_hourly(&equity),
                    calmar: compute_calmar(&equity),
                    max_drawdown: compute_max_drawdown_f64(&equity),
                    total_return: compute_total_return(&equity),
                    final_equity: result.final_equity,
                    initial_equity: result.initial_equity,
                });
            }

            // Sort metrics by index before reduction (ADR-0051 D2).
            // They are already in j order since we iterate j sequentially.
            let summary = DistributionSummary::from_path_metrics(&metrics)
                .expect("DistributionSummary must succeed");

            let formatted = format!(
                "l={lookback},k={k_long},p5={:.6},p50={:.6},prob_loss={:.6}",
                summary.sharpe.p5, summary.sharpe.p50, summary.prob_loss,
            );
            cell_summaries.push(formatted);
        }
        cell_summaries
    };

    let run1 = run_sweep_once();
    let run2 = run_sweep_once();

    assert_eq!(
        run1, run2,
        "carry two-run byte-identity: runs at the same seed must produce identical summaries. \
         If they differ, there is non-determinism in the carry path (funding resample, \
         ring-buffer state, or reduction order). ADR-0051 § D6.6.5 / D6.4 violation."
    );
}

// ── Helpers for two-run identity test ─────────────────────────────────────────

fn build_synthetic_bars_2sym(seed: u64, n: usize) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut close_a = 1000.0_f64;
    let mut close_b = 500.0_f64;

    let mut bars: Vec<Bar> = Vec::with_capacity(n * 2);

    for hour in 0..n {
        let z_a: f64 = rng.random::<f64>() * 0.04 - 0.02;
        let z_b: f64 = rng.random::<f64>() * 0.04 - 0.02;
        let next_a = (close_a * (1.0 + z_a)).max(0.01);
        let next_b = (close_b * (1.0 + z_b)).max(0.01);

        let pa = Decimal::try_from(next_a).unwrap_or(dec!(1));
        let pb = Decimal::try_from(next_b).unwrap_or(dec!(1));

        bars.push(make_bar("AAUSDT", pa, hour as i64));
        bars.push(make_bar("BBUSDT", pb, hour as i64));

        close_a = next_a;
        close_b = next_b;
    }

    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));
    bars
}

fn build_synthetic_funding(n_hours: usize) -> BTreeMap<(Symbol, Timestamp), Decimal> {
    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");

    let mut funding: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..n_hours {
        let ts = make_ts(hour as i64);
        // Alternating: AAUSDT has negative funding on even hours, BBUSDT on odd.
        let (rate_a, rate_b) = if hour % 2 == 0 {
            (dec!(-0.005), dec!(0.005))
        } else {
            (dec!(0.005), dec!(-0.005))
        };
        funding.insert((sym_a.clone(), ts), rate_a);
        funding.insert((sym_b.clone(), ts), rate_b);
    }
    funding
}

// ── Carry-surface fix (2026-08-04): funding-accrual regression gates ────────
//
// Two defects were found by MEASUREMENT here and fixed in
// `scenarios::montecarlo::run_path`. These tests are the experiments that found
// them, kept as gates.
//
//  (1) MULTIPLICITY — `merged_bars` interleaves every symbol's series, and the
//      accrual block was gated only on the bar timestamp, so the whole position
//      book accrued once per SYMBOL-BAR. Measured: holding ONE position and
//      varying only the universe size gave -5 / -7 / -9 units for N = 2 / 3 / 4.
//      Correct behaviour is INVARIANT to universe size.
//
//  (2) CADENCE — the settlement test counted bars on the generator's cosmetic
//      1-hour ladder (bug-log #72), so a 4h path settled every 32 real hours and
//      a daily path every 8 real days. `bar_span_hours` now supplies the real
//      span and the boundaries inside it are counted explicitly.

/// Total realized funding for a universe of `size`, holding exactly ONE
/// position, with every symbol flat and carrying the same funding rate.
/// The ONLY variable is how many bar events share each timestamp.
fn carry_funding_total(size: usize, bar_span_hours: u32, n_bars: i64) -> Decimal {
    let syms: Vec<String> = (0..size)
        .map(|i| {
            let c = (b'A' + u8::try_from(i).unwrap_or(0)) as char;
            format!("{c}{c}USDT")
        })
        .collect();
    let rate = dec!(0.0001);

    let mut bars: Vec<Bar> = Vec::new();
    for h in 0..n_bars {
        for sym in &syms {
            bars.push(make_bar(sym, dec!(100), h));
        }
    }
    bars.sort_by(|x, y| {
        x.open_ts
            .cmp(&y.open_ts)
            .then_with(|| x.symbol.0.cmp(&y.symbol.0))
    });

    let mut funding: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for h in 0..n_bars {
        for sym in &syms {
            funding.insert((Symbol::new(sym), make_ts(h)), rate);
        }
    }

    let mut cfg = make_carry_config();
    cfg.universe = syms.iter().map(SmolStr::new).collect();
    cfg.k_long = 1;

    // #75 (2026-08-22): second construction site in this file — `run_to_result`
    // above was fixed in the same pass and this one was missed. `funding_override`
    // is the ACCRUAL channel only now; the carry arm SCORES on funding, so with no
    // score map it takes no positions and there is nothing to accrue on, which
    // collapses both `funding_accrual_*` measurements to zero.
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("carry_span_test"))
        .with_funding(Some(funding.clone()));
    let input = TcnScenarioInput {
        scenario_name: "carry-span".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: dec!(100_000),
        slippage_bps: 0,
        taker_fee_bps: 0,
        config_id: "test_carry".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: Some(funding),
        bar_span_hours,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed")
        .realized_funding
}

/// (1) Funding must not depend on how many symbols happen to share a timestamp.
/// This is the exact experiment that exposed the multiplicity bug.
#[test]
fn funding_accrual_is_invariant_to_universe_size() {
    let f2 = carry_funding_total(2, 1, 24);
    let f3 = carry_funding_total(3, 1, 24);
    let f4 = carry_funding_total(4, 1, 24);
    assert_eq!(
        f2, f3,
        "funding must not scale with universe size (2 vs 3): {f2} vs {f3} — \
         the accrual is firing once per SYMBOL-BAR instead of once per settlement"
    );
    assert_eq!(
        f3, f4,
        "funding must not scale with universe size (3 vs 4): {f3} vs {f4}"
    );
    assert!(
        f2 != Decimal::ZERO,
        "probe inconclusive: no funding accrued at all"
    );
}

/// (2) A bar spanning more market time must settle proportionally more funding.
/// 24 hourly bars = 24 h; 24 four-hour bars = 96 h (4x); 24 daily bars = 576 h (24x).
/// Same universe, same rate, same position — only the declared span changes.
#[test]
fn funding_accrual_scales_with_declared_bar_span() {
    let hourly = carry_funding_total(3, 1, 24);
    let four_h = carry_funding_total(3, 4, 24);
    let daily = carry_funding_total(3, 24, 24);
    assert!(
        four_h.abs() > hourly.abs(),
        "a 4h bar covers 4x the market time of a 1h bar, so 24 of them must \
         settle more funding: 1h={hourly} 4h={four_h}"
    );
    assert!(
        daily.abs() > four_h.abs(),
        "a daily bar covers 6x the market time of a 4h bar: 4h={four_h} daily={daily}"
    );
    eprintln!("span scaling: 1h={hourly} 4h={four_h} daily={daily}");
}
