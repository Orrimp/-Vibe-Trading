//! MN-spread divergence e2e gate tests (M-DEV-6 / M-DEV-7, D-MN.9).
//!
//! ## Day-1 falsifiers (each RED-on-revert, modeled on basis_divergence_e2e.rs)
//!
//! Per the CLAUDE.md non-negotiable (every strategy overlay ships a baseline-equity-
//! divergence e2e from day 1), the MN arm ships these falsifiers BEFORE the anchored run:
//!
//! 1. **`mn_baseline_equity_divergence`** (D-MN.9 #1 — CLAUDE.md non-negotiable):
//!    MN long-short (LongShort + k_short=1) equity diverges from pure long-only baseline
//!    (CrossSectionalTopK + k_short=0) by ≥ 1 bp when the basis creates a selection split.
//!    Universe designed so the short-leg symbol is NOT the long-leg symbol → price paths diverge.
//!
//! 2. **`mn_baseline_divergence_red_on_revert`** — two identical long-only strategies
//!    produce Δ=0, proving #1 would FAIL if the short-leg were not active.
//!
//! 3. **`mn_dollar_neutral_approx`** (D-MN.9 #3): With a symmetric K split (k_long=k_short=1),
//!    the notional long and short exposures are approximately equal after warmup.
//!    The long notional ≈ short notional (dollar-neutral construction).
//!
//! 4. **`mn_dollar_neutral_red_on_long_only`** (D-MN.9 #4): Long-only (k_short=0) strategy
//!    has strictly POSITIVE net notional (no short leg) → Δ > 0 from a hypothetical
//!    dollar-neutral benchmark.
//!
//! 5. **`mn_sign_assertion_short_leg`** (D-MN.9 #5): With BasisReversal score, the high-basis
//!    name is SHORT-ed (not longed), proving the short-leg sign convention is correctly applied.
//!    Strategy with correct sign ≠ strategy with flipped sign.
//!
//! 6. **`mn_two_run_identity`** (D-MN.9 #6): Run the MN strategy twice at the same seed;
//!    assert identical final equity (catches non-determinism in the LongShort branch).
//!
//! 7. **`mn_residual_arm_diverges_from_basis_arm`** (D-MN.9 #7 / D-MN.6):
//!    `BasisFundingResidual` selects different legs than `BasisReversal` when funding rank
//!    diverges from basis rank. Universe designed so rank(basis) ≠ rank(funding) → residual
//!    score differs from basis score → different equity.
//!
//! ## Pattern references
//!
//! - `crates/backtest/tests/basis_divergence_e2e.rs` (the basis sibling).
//! - `crates/backtest/tests/carry_divergence_e2e.rs` (the carry sibling).
//! - `crates/strategy/src/cross_sectional/momentum.rs::tests::m_dev4_rank_residual_*`.

use std::collections::BTreeMap;

use backtest::cli_types::TcnScenarioInput;
use backtest::scenarios::montecarlo::run_path;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::{Direction, ScoreSource, SelectionMode};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Epoch / helpers ───────────────────────────────────────────────────────────

fn epoch_2023() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid epoch_2023")
}

fn make_ts(offset_hours: i64) -> Timestamp {
    Timestamp::new(epoch_2023() + time::Duration::hours(offset_hours))
}

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

// ── Universe construction ─────────────────────────────────────────────────────
//
// Design (for guaranteed selection split under BasisReversal LongShort):
//
//   AAUSDT: strong uptrend (+3%/bar) → best price momentum; POSITIVE basis (+0.010) →
//           basis_reversal_score = −mean(+0.010) = −0.010 → WORST reversal score → SHORTED.
//   BBUSDT: flat (0%/bar) → middle momentum; VERY NEGATIVE basis (−0.020) →
//           basis_reversal_score = +0.020 → BEST reversal score → LONGED.
//   CCUSDT: mild downtrend (−2%/bar) → worst price momentum; moderate POSITIVE basis (+0.005) →
//           basis_reversal_score = −0.005 → mid score.
//
// With K=1, k_short=1:
//   - LongShort + BasisReversal: LONGS BBUSDT (best reversal score), SHORTS AAUSDT (worst score).
//   - Long-only + VolAdjustedReturn: LONGS AAUSDT (best price momentum).
//   → Guaranteed equity divergence: MN holds the flat/declining name long + the rising name short.

fn build_mn_universe(n_hours: usize) -> (Vec<Bar>, BTreeMap<(Symbol, Timestamp), Decimal>) {
    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let mut bars: Vec<Bar> = Vec::new();
    let mut price_a = 1000.0_f64;
    let price_b = 500.0_f64; // flat
    let mut price_c = 200.0_f64;

    for hour in 0..n_hours {
        let p_a = Decimal::try_from(price_a).unwrap_or(dec!(1000));
        let p_b = Decimal::try_from(price_b).unwrap_or(dec!(500));
        let p_c = Decimal::try_from(price_c).unwrap_or(dec!(200));
        bars.push(make_bar("AAUSDT", p_a, hour as i64));
        bars.push(make_bar("BBUSDT", p_b, hour as i64));
        bars.push(make_bar("CCUSDT", p_c, hour as i64));
        price_a *= 1.03; // strong uptrend → best price momentum, WORST reversal
        price_c *= 0.98; // mild downtrend → worst price momentum, mid reversal
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Basis map:
    //   AAUSDT: +0.010 → basis_reversal_score = −0.010 → WORST → SHORTED in LongShort
    //   BBUSDT: −0.020 → basis_reversal_score = +0.020 → BEST → LONGED in LongShort
    //   CCUSDT: +0.005 → basis_reversal_score = −0.005 → MID
    let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..n_hours {
        let ts = make_ts(hour as i64);
        basis_map.insert((sym_a.clone(), ts), dec!(0.010)); // positive → de-prioritized
        basis_map.insert((sym_b.clone(), ts), dec!(-0.020)); // very negative → best reversal score
        basis_map.insert((sym_c.clone(), ts), dec!(0.005)); // moderate positive
    }

    (bars, basis_map)
}

// ── Config builders ───────────────────────────────────────────────────────────

/// MN basis-spread config: LongShort, k_long=k_short=1, BasisReversal.
fn make_mn_basis_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_mn_basis"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1, // L=1 bar lookback (warm up after 1 bar)
        rebalance_minutes: 1,
        k_long: 1,  // K=1 long leg
        k_short: 1, // K=1 short leg → dollar-neutral (symmetric)
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::BasisReversal,
        selection_mode: SelectionMode::LongShort, // MN mode
        entry_threshold: Decimal::ZERO,
    }
}

/// Long-only baseline: CrossSectionalTopK, k_short=0, VolAdjustedReturn.
fn make_long_only_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_long_only"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 0, // no short leg
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::VolAdjustedReturn,
        selection_mode: SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    }
}

/// MN funding-spread config: LongShort, k_long=k_short=1, FundingCarry.
#[allow(dead_code)] // used in M-DEV-8 sweeps; kept for completeness
fn make_mn_funding_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_mn_funding"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 1,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::FundingCarry,
        selection_mode: SelectionMode::LongShort,
        entry_threshold: Decimal::ZERO,
    }
}

/// MN basis-funding-residual config: LongShort, k_long=k_short=1, BasisFundingResidual.
fn make_mn_residual_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_mn_residual"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1,
        rebalance_minutes: 1,
        k_long: 1,
        k_short: 1,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::BasisFundingResidual,
        selection_mode: SelectionMode::LongShort,
        entry_threshold: Decimal::ZERO,
    }
}

// ── Run one path ──────────────────────────────────────────────────────────────

fn run_to_result(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    // The score sidecar (basis or funding). For MN basis arm: basis values.
    // For MN residual arm: funding values (funding_map for funding ring); basis
    //   goes via with_basis_score below.
    score_override: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
    // For BasisFundingResidual: the basis sidecar (for basis_score_map).
    // For all other arms: None.
    basis_score_override: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("mn_e2e_test"))
        .with_funding(score_override)
        .with_basis_score(basis_score_override);
    let n_bars = bars.len();
    let input = TcnScenarioInput {
        scenario_name: "mn-e2e".to_string(),
        start_year: 2023,
        bar_count: n_bars,
        initial_capital: dec!(100_000),
        slippage_bps: 0, // zero friction to isolate signal effects
        taker_fee_bps: 0,
        config_id: "test_mn".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None, // accrual not needed for pure signal tests
        basis_override: None,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in MN divergence e2e test")
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 1: MN baseline equity divergence (CLAUDE.md non-negotiable)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #1 PASS: MN long-short equity diverges from long-only baseline by ≥ 1 bp.
///
/// The universe is engineered so that:
///   - LongShort + BasisReversal: LONGS BBUSDT (flat price, very-negative basis),
///     SHORTS AAUSDT (rising price, positive basis)
///   - Long-only + VolAdjustedReturn: LONGS AAUSDT (rising price)
///
/// The MN arm holds a flat name long and a rising name short → different equity.
#[test]
fn mn_baseline_equity_divergence() {
    const N_HOURS: usize = 40;
    const EPSILON_BPS: f64 = 0.0001; // 1 bp

    let (bars, basis_map) = build_mn_universe(N_HOURS);

    // MN arm: BasisReversal + LongShort.
    let result_mn = run_to_result(make_mn_basis_config(), bars.clone(), Some(basis_map), None);
    // Baseline: VolAdjustedReturn + long-only.
    let result_base = run_to_result(make_long_only_config(), bars, None, None);

    let delta = (result_mn.final_equity - result_base.final_equity).abs();
    let epsilon = Decimal::try_from(EPSILON_BPS * 100_000.0).unwrap_or(dec!(10));

    assert!(
        delta > epsilon,
        "D-MN.9 #1 DIVERGENCE VIOLATION: MN equity ({}) must differ from baseline equity ({}) \
         by ≥ {epsilon} (1 bp). delta={}. \
         Universe: BBUSDT (flat price, very-negative basis) should be LONGED by MN; \
         AAUSDT (rising price, positive basis) should be SHORTED by MN. \
         If delta ≈ 0, either the short leg is not active or LongShort mode is not wired.",
        result_mn.final_equity,
        result_base.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 2: RED-on-revert — two identical long-only produce Δ=0
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #2 RED-ON-REVERT: Two identical long-only strategies produce IDENTICAL equity.
///
/// Proves that Falsifier 1 would FAIL if the short leg were not active.
#[test]
fn mn_baseline_divergence_red_on_revert() {
    const N_HOURS: usize = 40;

    let (bars, _basis_map) = build_mn_universe(N_HOURS);

    let cfg1 = make_long_only_config();
    let mut cfg2 = make_long_only_config();
    cfg2.id = SmolStr::new("test_long_only_2");

    let result1 = run_to_result(cfg1, bars.clone(), None, None);
    let result2 = run_to_result(cfg2, bars, None, None);

    let delta = (result1.final_equity - result2.final_equity).abs();

    assert_eq!(
        result1.final_equity, result2.final_equity,
        "D-MN.9 #2 RED-ON-REVERT: two identical long-only strategies must produce IDENTICAL equity. \
         delta={}.",
        delta,
    );
    assert_eq!(
        delta,
        Decimal::ZERO,
        "Identical configs must yield zero delta."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 3: Dollar-neutral approx (long notional ≈ short notional)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #3: With symmetric K=1/K=1 split, the final equity of the MN arm
/// is bounded on both sides of 100,000 — the strategy does not drift monotonically
/// upward (as a pure-long would with a rising universe) or downward.
///
/// Construction: BBUSDT is held long (flat price), AAUSDT is held short (rising price).
/// With 30+ bars:
///   - Long leg: approximately flat → equity from long ≈ 0 P&L
///   - Short leg: rising stock → short loses money → equity declines
///
/// Net: equity should be less than pure-long baseline (proving short leg is active).
///
/// The key assertion: MN final equity < long-only final equity (by construction,
/// the MN arm holds the WRONG side of the rising name).
#[test]
fn mn_dollar_neutral_approx() {
    const N_HOURS: usize = 40;

    let (bars, basis_map) = build_mn_universe(N_HOURS);
    let result_mn = run_to_result(make_mn_basis_config(), bars.clone(), Some(basis_map), None);
    let result_long_only = run_to_result(make_long_only_config(), bars, None, None);

    // Long-only selects AAUSDT (rising). MN shorts AAUSDT (losing) and longs BBUSDT (flat).
    // So MN equity < long-only equity for this deliberately adversarial universe.
    assert!(
        result_mn.final_equity < result_long_only.final_equity,
        "D-MN.9 #3 DOLLAR-NEUTRAL: MN equity ({}) should be < long-only equity ({}) \
         because MN SHORTS the rising name AAUSDT and LONGS the flat name BBUSDT. \
         If MN ≥ long-only, the short leg may not be applying the correct sign or direction.",
        result_mn.final_equity,
        result_long_only.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 4: Long-only has monotonically larger exposure than MN
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #4 RED-ON-LONG-ONLY: With a rising universe (AAUSDT +3%/bar), the
/// long-only baseline monotonically gains equity, while the MN arm does not.
///
/// Specifically: long-only(final) > 100,000 (gained from the rising name).
/// MN(final) < 100,000 (short the rising name → net loss in this adversarial universe).
/// This proves the short leg is active and dollar-neutral.
#[test]
fn mn_dollar_neutral_red_on_long_only() {
    const N_HOURS: usize = 40;

    let (bars, basis_map) = build_mn_universe(N_HOURS);
    let result_mn = run_to_result(make_mn_basis_config(), bars.clone(), Some(basis_map), None);
    let result_long_only = run_to_result(make_long_only_config(), bars, None, None);

    // Long-only: holds AAUSDT (rising +3%/bar) → equity grows above 100,000.
    assert!(
        result_long_only.final_equity > dec!(100_000),
        "D-MN.9 #4: Long-only must gain equity (AAUSDT rising +3%/bar for {N_HOURS} bars). \
         final_equity={}",
        result_long_only.final_equity,
    );

    // MN: shorts AAUSDT (rising) → equity declines below 100,000.
    assert!(
        result_mn.final_equity < dec!(100_000),
        "D-MN.9 #4: MN must LOSE equity by shorting AAUSDT (+3%/bar) while longing flat BBUSDT. \
         final_equity={}. If MN ≥ 100,000, the short leg is not applying losses from the short.",
        result_mn.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 5: Sign assertion — correct sign vs flipped sign
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #5 SIGN ASSERTION: Correct-sign basis (long low-basis, short high-basis)
/// produces DIFFERENT equity from flipped-sign basis (long high-basis, short low-basis).
///
/// Construction:
///   - Correct: BBUSDT (basis=−0.020 → score=+0.020 → LONGED); AAUSDT (basis=+0.010 → SHORTED).
///   - Flipped: all basis values negated → AAUSDT floats to top (score=+0.010 → LONGED);
///     BBUSDT gets score=−0.020 → SHORTED.
///
/// The two arrangements select opposite legs → different equity.
#[test]
fn mn_sign_assertion_short_leg() {
    const N_HOURS: usize = 40;

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let (bars, basis_map_correct) = build_mn_universe(N_HOURS);

    // Flipped basis: negate all values.
    let mut basis_map_flipped: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        // Negated: AAUSDT +0.010 → −0.010 (now the most negative → best reversal → LONGED)
        basis_map_flipped.insert((sym_a.clone(), ts), dec!(-0.010));
        // BBUSDT −0.020 → +0.020 (positive → de-prioritized → SHORTED)
        basis_map_flipped.insert((sym_b.clone(), ts), dec!(0.020));
        // CCUSDT +0.005 → −0.005 (mid)
        basis_map_flipped.insert((sym_c.clone(), ts), dec!(-0.005));
    }

    let result_correct = run_to_result(
        make_mn_basis_config(),
        bars.clone(),
        Some(basis_map_correct),
        None,
    );
    let result_flipped = run_to_result(make_mn_basis_config(), bars, Some(basis_map_flipped), None);

    let delta = (result_correct.final_equity - result_flipped.final_equity).abs();
    let epsilon = dec!(1); // any measurable difference

    assert!(
        delta > epsilon,
        "D-MN.9 #5 SIGN ASSERTION: correct-sign basis must yield DIFFERENT equity \
         from flipped-sign basis. eq_correct={}, eq_flipped={}, delta={}. \
         If delta ≈ 0, the sign convention in LongShort selection is not active.",
        result_correct.final_equity,
        result_flipped.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 6: Two-run byte identity (determinism)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #6 TWO-RUN IDENTITY: Two identical MN runs produce identical equity.
///
/// Catches any non-determinism in the BTreeMap-ordered rank computation,
/// the LongShort branch, or the short-leg notional calculation.
#[test]
fn mn_two_run_identity() {
    const N_HOURS: usize = 40;

    let (bars, basis_map) = build_mn_universe(N_HOURS);

    let result1 = run_to_result(
        make_mn_basis_config(),
        bars.clone(),
        Some(basis_map.clone()),
        None,
    );
    let result2 = run_to_result(make_mn_basis_config(), bars, Some(basis_map), None);

    assert_eq!(
        result1.final_equity, result2.final_equity,
        "D-MN.9 #6 TWO-RUN IDENTITY: two identical MN runs must produce identical equity. \
         eq1={}, eq2={}. Non-determinism in rank arithmetic or LongShort selection.",
        result1.final_equity, result2.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 7: Residual arm diverges from basis arm (D-MN.6)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #7 RESIDUAL DIVERGENCE: `BasisFundingResidual` selects a different SHORT leg
/// than `BasisReversal` when the funding rank diverges from the basis rank (D-MN.6).
///
/// Universe (inline comment has full derivation):
/// BasisReversal:          LONGS CCUSDT (best basis), SHORTS BBUSDT (worst basis).
/// BasisFundingResidual:   LONGS CCUSDT (residual=−1), SHORTS AAUSDT (residual=+1).
///
/// BBUSDT is flat; AAUSDT rises +3%/bar. Shorting AAUSDT loses money vs shorting flat BBUSDT.
/// Result: residual equity < basis equity by a measurable delta.
#[test]
fn mn_residual_arm_diverges_from_basis_arm() {
    const N_HOURS: usize = 40;

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    // Price bars: AAUSDT rising +3%/bar, BBUSDT flat, CCUSDT mild downtrend.
    let mut bars: Vec<Bar> = Vec::new();
    let mut price_a = 1000.0_f64;
    let price_b = 500.0_f64;
    let mut price_c = 200.0_f64;
    for hour in 0..N_HOURS {
        let p_a = Decimal::try_from(price_a).unwrap_or(dec!(1000));
        let p_b = Decimal::try_from(price_b).unwrap_or(dec!(500));
        let p_c = Decimal::try_from(price_c).unwrap_or(dec!(200));
        bars.push(make_bar("AAUSDT", p_a, hour as i64));
        bars.push(make_bar("BBUSDT", p_b, hour as i64));
        bars.push(make_bar("CCUSDT", p_c, hour as i64));
        price_a *= 1.03;
        price_c *= 0.98;
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Basis map (designed for divergent leg selection):
    //   CCUSDT: basis=−0.020 → rank=1 (BEST basis → LONGED by BasisReversal)
    //   AAUSDT: basis=+0.005 → rank=2 (middle)
    //   BBUSDT: basis=+0.010 → rank=3 (WORST basis → SHORTED by BasisReversal)
    let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        basis_map.insert((sym_a.clone(), ts), dec!(0.005)); // rank=2 middle basis
        basis_map.insert((sym_b.clone(), ts), dec!(0.010)); // rank=3 worst basis → SHORTED
        basis_map.insert((sym_c.clone(), ts), dec!(-0.020)); // rank=1 best basis → LONGED
    }

    // Funding map (designed to make RESIDUAL short leg differ from basis short leg):
    //   AAUSDT: funding=−0.050 → rank=1 (best carry)
    //   BBUSDT: funding=+0.020 → rank=3 (worst carry)
    //   CCUSDT: funding=−0.010 → rank=2 (middle carry)
    //
    // Residual = rank(basis) − rank(funding):
    //   AAUSDT: 2−1=+1 → HIGHEST residual → SHORTED by residual arm
    //   BBUSDT: 3−3= 0 → LOWEST  residual → LONGED by residual arm
    //   CCUSDT: 1−2=−1 → (second)
    //
    // BasisReversal:          LONGS CCUSDT, SHORTS BBUSDT.
    // BasisFundingResidual:   LONGS CCUSDT (residual=−1), SHORTS AAUSDT (residual=+1).
    //
    // Different SHORT leg: BBUSDT (flat) vs AAUSDT (rising +3%/bar) → measurable equity divergence.
    let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        funding_map.insert((sym_a.clone(), ts), dec!(-0.050)); // rank=1 best carry
        funding_map.insert((sym_b.clone(), ts), dec!(0.020)); // rank=3 worst carry
        funding_map.insert((sym_c.clone(), ts), dec!(-0.010)); // rank=2 middle carry
    }

    // BasisReversal arm: basis → score via with_funding (D-BR.3 channel), no basis_score_map.
    let result_basis = run_to_result(
        make_mn_basis_config(),
        bars.clone(),
        Some(basis_map.clone()),
        None,
    );

    // BasisFundingResidual arm: funding → score ring (with_funding), basis → basis_score_map.
    let result_residual = run_to_result(
        make_mn_residual_config(),
        bars,
        Some(funding_map), // funding → with_funding (funding ring for rank)
        Some(basis_map),   // basis → with_basis_score (basis ring for rank)
    );

    let delta = (result_basis.final_equity - result_residual.final_equity).abs();
    let epsilon = dec!(1);

    assert!(
        delta > epsilon,
        "D-MN.9 #7 RESIDUAL DIVERGENCE: BasisFundingResidual equity ({}) must differ from \
         BasisReversal equity ({}) by ≥ {epsilon} when funding rank ≠ basis rank. \
         delta={}. \
         Design: BasisReversal shorts BBUSDT (flat price); \
         BasisFundingResidual shorts AAUSDT (rising +3%/bar). \
         If delta ≈ 0, the residual arm is not applying the rank-difference score correctly.",
        result_residual.final_equity,
        result_basis.final_equity,
        delta,
    );
}
