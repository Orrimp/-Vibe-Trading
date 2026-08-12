//! MN-spread divergence e2e gate tests (M-DEV-6 / M-DEV-7, D-MN.9).
//!
//! ## What the story-1-21 review changed about this file (read this first)
//!
//! The suite shipped as "seven day-1 falsifiers". Three of the seven had never been
//! implemented at all, and of the four that existed, **six of the seven tests stayed
//! GREEN with the entire short-side branch deleted from `run_path`** (both `k_short > 0`
//! match arms removed; only `mn_dollar_neutral_red_on_long_only` went RED). The cause was
//! not carelessness in the assertions — it was the *comparisons*: each "divergence" test
//! changed THREE things at once (score source, selection mode, sidecar presence) and then
//! attributed the whole equity delta to the short leg. A comparison that moves three
//! variables measures none of them.
//!
//! Every falsifier below now changes **one** variable, and each one that claims to be
//! about the short leg has been observed RED under the short-branch-deleted mutation.
//! Where a comparison provably cannot isolate the short leg, that is stated in its doc
//! instead of being asserted around (see `mn_residual_arm_diverges_from_basis_arm`).
//!
//! ## The falsifier suite (D-MN.9), as actually implemented
//!
//! | # | test | isolates | RED when the short branch is deleted |
//! |---|------|----------|--------------------------------------|
//! | 1 | `mn_baseline_equity_divergence` | `k_short` 1 → 0, all else identical | **yes** |
//! | 2 | `mn_baseline_divergence_red_on_revert` | (negative control: Δ must be 0) | no — by design |
//! | 3 | `mn_dollar_neutral_net_notional_is_zero_at_settlement` | Σ notional at a settlement | **yes** |
//! | 4 | `mn_dollar_neutral_red_on_long_only` | long-only has one-sided exposure | **yes** |
//! | 5 | `mn_sign_assertion_short_leg` | the SHORT leg's identity (long leg pinned) | **yes** |
//! | 6 | `mn_two_run_identity` | determinism | no — by design |
//! | 7 | `mn_residual_arm_diverges_from_basis_arm` | residual ranking ≠ basis ranking | no — see its doc |
//! | 7b | `mn_residual_arm_short_leg_is_active` | `k_short` 1 → 0 on the residual arm | **yes** |
//! | 8 | `mn_short_leg_funding_cost_non_no_op` | the SHORT leg's funding rate | **yes** |
//! | 9 | `mn_no_look_ahead_both_sidecars` | each sidecar's as-of join, one at a time | no — by design |
//! | — | `characterization_bug75_funding_override_clobbers_injected_score_map` | documents a KNOWN DEFECT | no — by design |
//!
//! Falsifiers 3, 8 and 9 are the three that were **declared and never implemented**:
//! dollar-neutrality (nothing anywhere measured notional — the old
//! `mn_dollar_neutral_approx` asserted an equity ORDERING and this file's own header
//! claimed it checked "long notional ≈ short notional"), short-leg funding cost (every
//! test passed `funding_override: None`, so the accrual the trace row calls "the binding
//! cost" had ZERO coverage), and no-look-ahead across both sidecars.
//!
//! ## How notional is measured without adding an output field
//!
//! `PathRunResult` exposes no position book, and adding one would be a behaviour change
//! in an anchored code path. But it exposes `realized_funding`, and the accrual is
//! `Σ_sym notional_sym × (−rate_sym)` — so with **one uniform rate `r` across every
//! symbol** the accrual collapses to `(−r) × Σ_sym notional_sym`: an exact, Decimal-exact
//! read-out of the book's NET notional at a settlement boundary. Dollar-neutral ⇒ exactly
//! zero. One-sided ⇒ exactly `−r × notional`. That is the probe tests 3, 4 and 8 use.
//!
//! ## Two KNOWN DEFECTS this file must not be read as endorsing
//!
//! - **bug-log #75** — `run_path` overwrites a pre-injected SCORE map with the ACCRUAL
//!   map (`funding_map`, one field, two meanings), so the anchored `mn-basis` arm ran the
//!   FUNDING score. Fix + re-run are anchor-impacting → story 1-25.
//! - **bug-log #76** — the residual arm ranks the basis axis INVERTED versus its own spec
//!   (it longs the HIGHEST basis). Fix + re-run are anchor-impacting → story 1-25.
//!
//! Both are pinned by characterization tests (here and in
//! `strategy::cross_sectional::momentum::tests`) so the defective behaviour cannot change
//! silently before 1-25 inverts them deliberately.
//!
//! ## Pattern references
//!
//! - `crates/backtest/tests/basis_divergence_e2e.rs` (the basis sibling).
//! - `crates/backtest/tests/carry_divergence_e2e.rs` (the carry sibling).
//! - `crates/backtest/src/scenarios/montecarlo.rs::tests::funding_accrual_four_sign_cases_pinned`
//!   (the accrual sign table, four cells, literal values).
//! - `crates/strategy/src/cross_sectional/momentum.rs::tests::m_dev4_rank_residual_*`.

use std::collections::BTreeMap;

use backtest::cli_types::TcnScenarioInput;
use backtest::scenarios::montecarlo::run_path;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::{Direction, ScoreSource, SelectionMode};
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

/// Initial capital for every run in this file.
const INITIAL_CAPITAL: Decimal = dec!(100_000);

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

type SidecarMap = BTreeMap<(Symbol, Timestamp), Decimal>;

// ── Universe construction ─────────────────────────────────────────────────────
//
// Design (for a guaranteed selection split under BasisReversal LongShort):
//
//   AAUSDT: strong uptrend (+3%/bar) → best price momentum; POSITIVE basis (+0.010) →
//           basis_reversal_score = −mean(+0.010) = −0.010 → WORST reversal score → SHORTED.
//   BBUSDT: flat (0%/bar) → middle momentum; VERY NEGATIVE basis (−0.020) →
//           basis_reversal_score = +0.020 → BEST reversal score → LONGED.
//   CCUSDT: mild downtrend (−2%/bar) → worst price momentum; moderate POSITIVE basis (+0.005) →
//           basis_reversal_score = −0.005 → mid score.

fn build_mn_universe(n_hours: usize) -> (Vec<Bar>, SidecarMap) {
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
    let mut basis_map: SidecarMap = BTreeMap::new();
    for hour in 0..n_hours {
        let ts = make_ts(hour as i64);
        basis_map.insert((sym_a.clone(), ts), dec!(0.010)); // positive → de-prioritized
        basis_map.insert((sym_b.clone(), ts), dec!(-0.020)); // very negative → best reversal score
        basis_map.insert((sym_c.clone(), ts), dec!(0.005)); // moderate positive
    }

    (bars, basis_map)
}

// ── The EXACT-NOTIONAL universe (the dollar-neutrality probe) ─────────────────
//
// Requirements for an exact Σ-notional read-out:
//
// 1. Both symbols must carry the SAME mark at the rebalance instant. `run_path` hands
//    every order to `engine.step(bar, …)` with the CURRENT bar, so a fill is priced at
//    whichever symbol's bar is being processed (bug-log #67, owned by story 1-25). Equal
//    marks make that mispricing a no-op, so this probe measures notional, not #67.
// 2. The marks must be round, so 10% of 100 000 buys an integral quantity.
// 3. Prices must be FLAT from the rebalance onward, so the notional at the settlement
//    boundary is still the notional at open.
// 4. The two symbols must nevertheless RANK differently, or `top_k_long` and
//    `bottom_k_short` both pick the alphabetically-first name on a tie and the same
//    symbol ends up longed and shorted.
//
// Hence: AA rises 800 → 900 → 1000 and stays; BB falls 1250 → 1100 → 1000 and stays.
// Ranking is by `VolAdjustedReturn` (AA rising ⇒ long, BB falling ⇒ short) — a
// price-driven score, so the funding map is free to serve purely as the notional probe
// and the fixture is immune to bug-log #75 as a bonus. `rebalance_minutes` is huge so
// there is exactly ONE rebalance and the book cannot move afterwards.
//
// Book after the single rebalance: long 10 AA @1000 = +10 000, short 10 BB @1000 = −10 000.
// Settlement boundaries sit at hours 0, 8, 16…; the run is 9 hours, so exactly ONE
// accrual happens with a live book (hour 8).

const FLAT_N_HOURS: i64 = 9;

fn build_exact_notional_universe() -> Vec<Bar> {
    let price_a = [
        dec!(800),
        dec!(900),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
    ];
    let price_b = [
        dec!(1250),
        dec!(1100),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
        dec!(1000),
    ];
    let mut bars: Vec<Bar> = Vec::new();
    for h in 0..FLAT_N_HOURS {
        bars.push(make_bar("AAUSDT", price_a[h as usize], h));
        bars.push(make_bar("BBUSDT", price_b[h as usize], h));
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));
    bars
}

/// Per-symbol constant rates, keyed at every bar timestamp of the exact-notional universe.
fn constant_rate_map(rate_a: Decimal, rate_b: Decimal) -> SidecarMap {
    let mut map: SidecarMap = BTreeMap::new();
    for h in 0..FLAT_N_HOURS {
        map.insert((Symbol::new("AAUSDT"), make_ts(h)), rate_a);
        map.insert((Symbol::new("BBUSDT"), make_ts(h)), rate_b);
    }
    map
}

/// Price-ranked two-symbol config for the notional probe.
///
/// `VolAdjustedReturn` never reads the funding map, so the map is a pure measuring
/// instrument here. `k_short = 0` collapses this to the long-only control.
fn make_price_ranked_config(k_short: u32) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_notional_probe"),
        universe: vec![SmolStr::new("AAUSDT"), SmolStr::new("BBUSDT")],
        lookback_minutes: 2,
        rebalance_minutes: 100_000, // exactly ONE rebalance, at warm-up completion
        k_long: 1,
        k_short,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::VolAdjustedReturn,
        selection_mode: if k_short > 0 {
            SelectionMode::LongShort
        } else {
            SelectionMode::CrossSectionalTopK
        },
        entry_threshold: Decimal::ZERO,
    }
}

// ── Config builders ───────────────────────────────────────────────────────────

/// MN basis-spread config: LongShort, `k_long = 1`, BasisReversal, caller-chosen `k_short`.
///
/// `k_short` is a PARAMETER because the single-variable falsifiers below compare
/// `k_short = 1` against `k_short = 0` with literally everything else held equal —
/// same score source, same sidecar, same selection mode, same universe. That comparison
/// is the only one whose delta is attributable to the short leg alone.
fn make_mn_basis_config_k(k_short: u32) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_mn_basis"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1, // L=1 bar lookback (warm up after 1 bar)
        rebalance_minutes: 1,
        k_long: 1, // K=1 long leg
        k_short,   // 1 → dollar-neutral symmetric split; 0 → the same arm, long leg only
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum,
        score_source: ScoreSource::BasisReversal,
        selection_mode: SelectionMode::LongShort, // MN mode, both arms
        entry_threshold: Decimal::ZERO,
    }
}

/// MN basis-spread config with a live short leg (the production shape).
fn make_mn_basis_config() -> strategy::CrossSectionalMomentumConfig {
    make_mn_basis_config_k(1)
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

/// MN basis-funding-residual config: LongShort, k_long=1, caller-chosen `k_short`.
fn make_mn_residual_config_k(k_short: u32) -> strategy::CrossSectionalMomentumConfig {
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
        k_short,
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

fn make_mn_residual_config() -> strategy::CrossSectionalMomentumConfig {
    make_mn_residual_config_k(1)
}

// ── Run one path ──────────────────────────────────────────────────────────────

/// SELECTION-only wiring: sidecars pre-injected into the strategy, no accrual.
///
/// `funding_override` is `None`, so the `run_path` accrual block is never entered and
/// the measured equity delta is pure selection. That is deliberate for the selection
/// falsifiers — an accrual running underneath them would be a second variable, which is
/// exactly the defect bug-log #74 recorded for the basis suite.
///
/// It is NOT production's MN wiring (production always passes `funding_override: Some`).
/// The tests that must speak about production's configuration use [`run_with_accrual`].
fn run_to_result(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    // The score sidecar (basis or funding). For MN basis arm: basis values.
    // For MN residual arm: funding values (funding_map for funding ring); basis
    //   goes via with_basis_score below.
    score_override: Option<SidecarMap>,
    // For BasisFundingResidual: the basis sidecar (for basis_score_map).
    // For all other arms: None.
    basis_score_override: Option<SidecarMap>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    run_with_accrual(cfg, bars, score_override, basis_score_override, None)
}

/// PRODUCTION wiring: sidecars pre-injected AND `funding_override` supplied.
///
/// The MN lane always passes `funding_override: Some(funding_map)` — it is the accrual
/// channel and, per bug-log #75, it also (wrongly) becomes the score channel. Tests that
/// make a claim about what the anchored surfaces DID must run through here; a test that
/// takes the `None` branch is exercising a configuration production never uses, which is
/// precisely how #75 survived a full falsifier suite.
fn run_with_accrual(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    score_override: Option<SidecarMap>,
    basis_score_override: Option<SidecarMap>,
    funding_override: Option<SidecarMap>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("mn_e2e_test"))
        .with_funding(score_override)
        .with_basis_score(basis_score_override);
    let n_bars = bars.len();
    let input = TcnScenarioInput {
        scenario_name: "mn-e2e".to_string(),
        start_year: 2023,
        bar_count: n_bars,
        initial_capital: INITIAL_CAPITAL,
        slippage_bps: 0, // zero friction to isolate signal effects
        taker_fee_bps: 0,
        config_id: "test_mn".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override,
        bar_span_hours: 1,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in MN divergence e2e test")
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 1: MN baseline equity divergence (CLAUDE.md non-negotiable, AD-16)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #1: the MN arm's equity diverges from its own un-shorted baseline by ≥ 1 bp.
///
/// # What changed (review 1-21) and why
///
/// This used to compare `LongShort + BasisReversal + basis sidecar` against
/// `CrossSectionalTopK + VolAdjustedReturn + no sidecar` — three variables at once — and
/// call the resulting delta evidence for the short leg. It was not: with the short-side
/// branch deleted from `run_path` the MN arm degenerates to a single flat long while the
/// momentum baseline compounds, so the delta got LARGER and the test stayed green.
///
/// The un-targeted baseline for a market-neutral OVERLAY is the same arm without the
/// overlay: identical score source, identical sidecar, identical selection mode,
/// `k_short = 0`. That is the AD-16 comparison, and it moves exactly one variable.
///
/// RED-on-mutation: verified. Delete the two `k_short > 0` arms from `run_path` and both
/// runs become the same long-only book ⇒ Δ = 0 ⇒ this fails.
#[test]
fn mn_baseline_equity_divergence() {
    const N_HOURS: usize = 40;
    const EPSILON_BPS: f64 = 0.0001; // 1 bp

    let (bars, basis_map) = build_mn_universe(N_HOURS);

    // WITH the short leg.
    let result_mn = run_to_result(
        make_mn_basis_config_k(1),
        bars.clone(),
        Some(basis_map.clone()),
        None,
    );
    // WITHOUT it — everything else byte-identical.
    let result_no_short = run_to_result(make_mn_basis_config_k(0), bars, Some(basis_map), None);

    // Liveness (the bug-log #74 lesson): an arm that never traded sits at exactly its
    // initial capital and "diverges" from anything by being dead.
    assert!(
        result_mn.trades > 0,
        "D-MN.9 #1: the MN arm executed 0 fills — it never traded, so any equity delta \
         is an artifact of a dead arm, not of the short leg"
    );
    assert!(
        result_no_short.trades > 0,
        "D-MN.9 #1: the k_short=0 baseline executed 0 fills — it never traded"
    );

    let delta = (result_mn.final_equity - result_no_short.final_equity).abs();
    let epsilon = Decimal::try_from(EPSILON_BPS * 100_000.0).unwrap_or(dec!(10));

    assert!(
        delta > epsilon,
        "D-MN.9 #1 DIVERGENCE VIOLATION: the short leg must move equity. \
         k_short=1 equity={}, k_short=0 equity={}, delta={} (need > {epsilon}). \
         The ONLY difference between these two runs is `k_short`; if delta ≈ 0 the short \
         leg is a no-op — the v3-vol-overlay failure class (AD-16) inside the MN arm.",
        result_mn.final_equity,
        result_no_short.final_equity,
        delta,
    );

    // Direction, not just magnitude (bug-log #74: a symmetric |Δ| cannot see a swap).
    // The MN arm shorts AAUSDT, the +3%/bar name — shorting a riser LOSES money, so the
    // shorted book must end BELOW the same book without the short leg.
    assert!(
        result_mn.final_equity < result_no_short.final_equity,
        "D-MN.9 #1 DIRECTION: the MN arm shorts AAUSDT (+3%/bar), so it must end BELOW \
         the identical k_short=0 arm. mn={}, no_short={}. If MN is higher, the short leg \
         is being booked with the wrong sign.",
        result_mn.final_equity,
        result_no_short.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 2: RED-on-revert — two identical long-only produce Δ=0
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #2 RED-ON-REVERT: Two identical long-only strategies produce IDENTICAL equity.
///
/// The negative control for falsifier 1: it shows that the harness returns Δ = 0 when
/// there is genuinely nothing to see, so falsifier 1's Δ ≠ 0 is a signal and not noise.
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
// Falsifier 3: DOLLAR NEUTRALITY — Σ notional ≈ 0 at a settlement boundary
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #3: with `k_long = k_short = 1`, the book's NET notional is exactly zero.
///
/// # This falsifier was DECLARED and never implemented (review 1-21)
///
/// The trace row and this file's header claimed a dollar-neutrality check with "long
/// notional ≈ short notional". Nothing in the workspace measured notional. What existed
/// (`mn_dollar_neutral_approx`) asserted `mn_equity < long_only_equity` — an equity
/// ORDERING between two differently-configured arms, which is not a neutrality property
/// at all and which stayed green with the short branch deleted.
///
/// # How notional is measured with no new output field
///
/// `realized_funding = Σ_sym notional_sym × (−rate_sym)`. Set ONE uniform rate `r` for
/// every symbol and it collapses to `(−r) × Σ_sym notional_sym` — an exact Decimal
/// read-out of net notional at the settlement boundary. No new field, no epsilon, no
/// change to any production code path.
///
/// Book under test: long 10 AAUSDT @1000 = **+10 000**, short 10 BBUSDT @1000 = **−10 000**
/// ⇒ Σ = 0 ⇒ `realized_funding` must be **exactly zero** even though `r ≠ 0`.
///
/// RED-on-mutation: verified. With the short branch deleted the book is +10 000 only, so
/// `realized_funding = −0.01 × 10 000 = −100 ≠ 0`.
#[test]
fn mn_dollar_neutral_net_notional_is_zero_at_settlement() {
    let bars = build_exact_notional_universe();
    let r = dec!(0.01);

    let mn = run_with_accrual(
        make_price_ranked_config(1),
        bars,
        None,
        None,
        Some(constant_rate_map(r, r)),
    );

    // Fixture guards: both legs must actually be on the book, or "Σ notional = 0" is
    // satisfied trivially by an empty book.
    assert_eq!(
        mn.trades, 2,
        "D-MN.9 #3 fixture: the probe needs exactly one long open + one short open \
         (trades={}). Σ notional = 0 on an EMPTY book proves nothing.",
        mn.trades
    );
    assert_eq!(
        mn.final_equity, INITIAL_CAPITAL,
        "D-MN.9 #3 fixture: flat prices + a dollar-neutral book ⇒ equity is untouched. \
         Got {}. If this moved, the legs are not the exact ±10 000 this test assumes.",
        mn.final_equity
    );

    assert_eq!(
        mn.realized_funding,
        Decimal::ZERO,
        "D-MN.9 #3 DOLLAR-NEUTRALITY VIOLATION: with a uniform funding rate across the \
         universe, realized_funding = (−r) × Σ notional. It came back {} instead of 0, \
         i.e. Σ notional = {} ≠ 0 — the book is NOT dollar-neutral. Either a leg failed \
         to fill (see the order-rejection warnings in run_path) or the two legs are not \
         sized symmetrically.",
        mn.realized_funding,
        -mn.realized_funding / r,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 4: RED-on-long-only — a one-sided book is NOT neutral
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #4 RED-ON-LONG-ONLY: the same probe on a long-only book returns a NON-zero
/// net notional — proving falsifier 3's zero is a measurement and not a constant.
///
/// Two independent legs:
///
/// 1. the exact one (new): the long-only book's net notional is `+10 000`, so the probe
///    reads exactly `−100` where the neutral book reads `0`; and
/// 2. the equity leg (original): on the adversarial 3-symbol universe, the long-only
///    baseline gains from the +3%/bar name while the MN arm — which SHORTS it — loses.
///    This is the one assertion in the original suite that already went RED when the
///    short-side branch was deleted, so it is preserved verbatim in substance.
#[test]
fn mn_dollar_neutral_red_on_long_only() {
    // ── Leg 1: the exact notional read-out on a one-sided book ────────────────
    let bars_flat = build_exact_notional_universe();
    let r = dec!(0.01);
    let long_only = run_with_accrual(
        make_price_ranked_config(0),
        bars_flat,
        None,
        None,
        Some(constant_rate_map(r, r)),
    );
    assert_eq!(
        long_only.trades, 1,
        "D-MN.9 #4 fixture: the long-only control must open exactly one leg (trades={})",
        long_only.trades
    );
    assert_eq!(
        long_only.realized_funding,
        dec!(-100),
        "D-MN.9 #4: a long-only book has net notional +10 000, so the uniform-rate probe \
         must read −0.01 × 10 000 = −100, NOT 0. Got {}. If this reads 0, the probe is \
         blind and falsifier 3's zero means nothing.",
        long_only.realized_funding
    );

    // ── Leg 2: the equity-direction leg on the adversarial universe ───────────
    const N_HOURS: usize = 40;
    let (bars, basis_map) = build_mn_universe(N_HOURS);
    let result_mn = run_to_result(make_mn_basis_config(), bars.clone(), Some(basis_map), None);
    let result_long_only = run_to_result(make_long_only_config(), bars, None, None);

    // Long-only: holds AAUSDT (rising +3%/bar) → equity grows above 100,000.
    assert!(
        result_long_only.final_equity > INITIAL_CAPITAL,
        "D-MN.9 #4: Long-only must gain equity (AAUSDT rising +3%/bar for {N_HOURS} bars). \
         final_equity={}",
        result_long_only.final_equity,
    );

    // MN: shorts AAUSDT (rising) → equity declines below 100,000.
    assert!(
        result_mn.final_equity < INITIAL_CAPITAL,
        "D-MN.9 #4: MN must LOSE equity by shorting AAUSDT (+3%/bar) while longing flat BBUSDT. \
         final_equity={}. If MN ≥ 100,000, the short leg is not applying losses from the short.",
        result_mn.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 5: Sign assertion — the SHORT leg's identity, long leg pinned
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #5 SIGN ASSERTION: swapping which name is the WORST basis changes the equity,
/// with the long leg held fixed.
///
/// # What changed (review 1-21)
///
/// The old version negated the whole basis map. That flips the LONG leg too (BBUSDT →
/// AAUSDT), so its Δ was dominated by the long book and it stayed green with the short
/// branch deleted.
///
/// This version swaps only AAUSDT's and CCUSDT's basis values. BBUSDT keeps the best
/// reversal score in both runs, so it is LONGED in both; only the shorted name changes:
///
/// - run A: AA = +0.010 (worst score) ⇒ SHORT AAUSDT, the +3%/bar riser;
/// - run B: CC = +0.010 (worst score) ⇒ SHORT CCUSDT, the −2%/bar faller.
///
/// Any delta is therefore attributable to the short leg alone, and the DIRECTION is
/// predictable: shorting a riser loses, shorting a faller gains ⇒ B > A.
///
/// RED-on-mutation: verified. With no short leg both runs are "long BBUSDT" ⇒ Δ = 0.
#[test]
fn mn_sign_assertion_short_leg() {
    const N_HOURS: usize = 40;

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let (bars, basis_short_aa) = build_mn_universe(N_HOURS);

    // Swap AA's and CC's basis. BBUSDT (−0.020) still has the best reversal score, so it
    // stays the LONG leg; the WORST score moves from AA (+0.010) to CC (+0.010).
    let mut basis_short_cc: SidecarMap = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        basis_short_cc.insert((sym_a.clone(), ts), dec!(0.005)); // was +0.010 → now mid
        basis_short_cc.insert((sym_b.clone(), ts), dec!(-0.020)); // unchanged → still LONGED
        basis_short_cc.insert((sym_c.clone(), ts), dec!(0.010)); // was +0.005 → now WORST → SHORTED
    }

    let result_short_aa = run_to_result(
        make_mn_basis_config(),
        bars.clone(),
        Some(basis_short_aa),
        None,
    );
    let result_short_cc = run_to_result(make_mn_basis_config(), bars, Some(basis_short_cc), None);

    assert!(
        result_short_aa.trades > 0 && result_short_cc.trades > 0,
        "D-MN.9 #5: both arms must trade (trades={} / {})",
        result_short_aa.trades,
        result_short_cc.trades
    );

    let delta = (result_short_aa.final_equity - result_short_cc.final_equity).abs();
    let epsilon = dec!(1); // any measurable difference

    assert!(
        delta > epsilon,
        "D-MN.9 #5 SIGN ASSERTION: moving the WORST basis from AAUSDT to CCUSDT changes \
         which name is SHORTED and must change equity. eq_short_aa={}, eq_short_cc={}, \
         delta={}. The LONG leg (BBUSDT) is identical in both runs, so Δ ≈ 0 means the \
         short leg is not being applied at all.",
        result_short_aa.final_equity,
        result_short_cc.final_equity,
        delta,
    );

    assert!(
        result_short_cc.final_equity > result_short_aa.final_equity,
        "D-MN.9 #5 DIRECTION: shorting CCUSDT (−2%/bar, a faller) must beat shorting \
         AAUSDT (+3%/bar, a riser). eq_short_cc={}, eq_short_aa={}. If the ordering is \
         reversed the short leg's P&L sign is inverted — a magnitude-only assertion \
         cannot see that (bug-log #74).",
        result_short_cc.final_equity,
        result_short_aa.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 6: Two-run byte identity (determinism)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #6 TWO-RUN IDENTITY: Two identical MN runs produce identical equity.
///
/// Catches any non-determinism in the BTreeMap-ordered rank computation,
/// the LongShort branch, or the short-leg notional calculation.
///
/// Scope note: this is a DETERMINISM check. It cannot see a wrong-but-deterministic
/// short leg — both runs would be wrong identically. (The same confusion is what made
/// `run_path_k_short_zero_byte_identical_to_head` claim to be a neutrality proof; see
/// its rewritten doc in `scenarios::montecarlo`.)
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
// Falsifier 7: Residual arm ranks differently from the basis arm (D-MN.6)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #7 RESIDUAL DIVERGENCE: `BasisFundingResidual` selects different legs than
/// `BasisReversal` when the funding rank diverges from the basis rank (D-MN.6).
///
/// # Scope, stated honestly (review 1-21)
///
/// This comparison changes the SCORE SOURCE, and it therefore moves BOTH legs at once:
/// with three symbols, `k_long = k_short = 1` and `residual = rank(basis) − rank(funding)`,
/// it is arithmetically IMPOSSIBLE for the residual arm to long the same name as the
/// basis arm while shorting a different one. The basis arm longs the symbol with
/// `rank(basis) = 1`, which can never be the strict maximum of
/// `rank(basis) − rank(funding)` unless every residual ties. So this test cannot be
/// re-pointed to isolate the short leg, and it does NOT go RED when the short branch is
/// deleted — the long legs still differ.
///
/// It stays because it tests something real (the residual ranking is not the basis
/// ranking). The short leg of the residual arm is covered by the single-variable
/// `mn_residual_arm_short_leg_is_active` below, and the residual arm's DIRECTION is
/// pinned by the bug-log #76 characterization test in the strategy crate.
///
/// # The actual selection under this fixture (recomputed at review 1-21)
///
/// The old comment block here claimed "BasisReversal LONGS CCUSDT, SHORTS BBUSDT;
/// BasisFundingResidual LONGS CCUSDT (residual −1), SHORTS AAUSDT (residual +1)". That is
/// backwards on the residual arm: `top_k_long` takes the HIGHEST score, so the +1 name is
/// LONGED and the −1 name is SHORTED. Recomputed:
///
/// | sym | basis  | basis score | rank(basis) | funding | funding score | rank(funding) | residual |
/// |-----|--------|-------------|-------------|---------|---------------|---------------|----------|
/// | AA  | +0.005 | −0.005      | 2           | −0.050  | +0.050        | 1             | **+1**   |
/// | BB  | +0.010 | −0.010      | 3           | +0.020  | −0.020        | 3             | 0        |
/// | CC  | −0.020 | +0.020      | 1           | −0.010  | +0.010        | 2             | **−1**   |
///
/// - `BasisReversal`: LONGS CCUSDT (best score), SHORTS BBUSDT (worst score).
/// - `BasisFundingResidual`: LONGS **AAUSDT** (residual +1), SHORTS **CCUSDT** (residual −1).
///
/// Note what that means, and why bug-log #76 exists: the residual arm longs AAUSDT, whose
/// basis (+0.005) is HIGHER than the name the basis arm longs (CCUSDT, −0.020). The
/// documented intent is "long = low basis relative to funding". The arm does the
/// opposite. Not fixed here — the fix re-prices anchors #116-#119 and belongs to 1-25.
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

    // Basis map — see the rank table in this test's doc comment.
    let mut basis_map: SidecarMap = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        basis_map.insert((sym_a.clone(), ts), dec!(0.005)); // rank(basis) = 2
        basis_map.insert((sym_b.clone(), ts), dec!(0.010)); // rank(basis) = 3 → SHORTED by BasisReversal
        basis_map.insert((sym_c.clone(), ts), dec!(-0.020)); // rank(basis) = 1 → LONGED by BasisReversal
    }

    // Funding map — see the rank table in this test's doc comment.
    let mut funding_map: SidecarMap = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        funding_map.insert((sym_a.clone(), ts), dec!(-0.050)); // rank(funding) = 1
        funding_map.insert((sym_b.clone(), ts), dec!(0.020)); // rank(funding) = 3
        funding_map.insert((sym_c.clone(), ts), dec!(-0.010)); // rank(funding) = 2
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

    assert!(
        result_basis.trades > 0 && result_residual.trades > 0,
        "D-MN.9 #7: both arms must trade (basis={} / residual={}); a dead arm 'diverges' \
         from anything by sitting at its initial capital (bug-log #74)",
        result_basis.trades,
        result_residual.trades
    );

    let delta = (result_basis.final_equity - result_residual.final_equity).abs();
    let epsilon = dec!(1);

    assert!(
        delta > epsilon,
        "D-MN.9 #7 RESIDUAL DIVERGENCE: BasisFundingResidual equity ({}) must differ from \
         BasisReversal equity ({}) by ≥ {epsilon} when funding rank ≠ basis rank. \
         delta={}. \
         Design: BasisReversal longs CCUSDT / shorts BBUSDT; BasisFundingResidual longs \
         AAUSDT (residual +1) / shorts CCUSDT (residual −1). \
         If delta ≈ 0, the residual arm is not applying the rank-difference score correctly.",
        result_residual.final_equity,
        result_basis.final_equity,
        delta,
    );
}

/// D-MN.9 #7b (review 1-21): the residual arm's SHORT leg is live — one variable.
///
/// `mn_residual_arm_diverges_from_basis_arm` cannot isolate the short leg (see its doc).
/// This one can: same score source, same both sidecars, same universe, `k_short` 1 → 0.
///
/// RED-on-mutation: verified. Delete the short arms from `run_path` and the two runs
/// become the same long-only book ⇒ Δ = 0.
#[test]
fn mn_residual_arm_short_leg_is_active() {
    const N_HOURS: usize = 40;

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let (bars, _unused) = build_mn_universe(N_HOURS);

    let mut basis_map: SidecarMap = BTreeMap::new();
    let mut funding_map: SidecarMap = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        basis_map.insert((sym_a.clone(), ts), dec!(0.005));
        basis_map.insert((sym_b.clone(), ts), dec!(0.010));
        basis_map.insert((sym_c.clone(), ts), dec!(-0.020));
        funding_map.insert((sym_a.clone(), ts), dec!(-0.050));
        funding_map.insert((sym_b.clone(), ts), dec!(0.020));
        funding_map.insert((sym_c.clone(), ts), dec!(-0.010));
    }

    let with_short = run_to_result(
        make_mn_residual_config_k(1),
        bars.clone(),
        Some(funding_map.clone()),
        Some(basis_map.clone()),
    );
    let without_short = run_to_result(
        make_mn_residual_config_k(0),
        bars,
        Some(funding_map),
        Some(basis_map),
    );

    assert!(
        with_short.trades > 0 && without_short.trades > 0,
        "D-MN.9 #7b: both arms must trade (with_short={} / without_short={})",
        with_short.trades,
        without_short.trades
    );

    let delta = (with_short.final_equity - without_short.final_equity).abs();
    assert!(
        delta > dec!(1),
        "D-MN.9 #7b: the residual arm's short leg must move equity. k_short=1 equity={}, \
         k_short=0 equity={}, delta={}. `k_short` is the ONLY difference between these \
         runs; Δ ≈ 0 means the residual arm's short book is a no-op.",
        with_short.final_equity,
        without_short.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 8: the SHORT LEG'S FUNDING COST is not a no-op (production wiring)
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #8: changing ONLY the shorted name's funding rate moves equity.
///
/// # This falsifier was DECLARED and never implemented (review 1-21)
///
/// The trace row calls the short-leg funding accrual "the binding cost" of the MN arm.
/// Every test in this file passed `funding_override: None`, which is the branch where
/// the accrual block is never entered — so the binding cost had **zero** coverage, and
/// the tests ran a configuration production never uses. (That is bug-log #74's mechanism
/// exactly: the test channel differed from the production channel in the way that hides
/// the defect.) This test uses `Some`, which is production's configuration.
///
/// # Construction — one variable
///
/// The exact-notional universe, ranked by price (`VolAdjustedReturn` never reads the
/// funding map, so the rate cannot move the selection). The LONG leg's rate is 0 in both
/// runs; only the SHORT leg's rate changes, 0 → +0.01. Expected accrual on a −10 000
/// short: `(−10 000) × (−0.01) = +100` — the short RECEIVES, which is the correct perp
/// mechanic (longs pay shorts on positive funding) and the cell the accrual comments used
/// to state backwards. See `montecarlo::tests::funding_accrual_four_sign_cases_pinned`.
///
/// RED-on-mutation: verified. No short leg ⇒ no short notional ⇒ both runs accrue 0.
#[test]
fn mn_short_leg_funding_cost_non_no_op() {
    let bars = build_exact_notional_universe();

    // Control: short leg's rate is zero.
    let zero_rate = run_with_accrual(
        make_price_ranked_config(1),
        bars.clone(),
        None,
        None,
        Some(constant_rate_map(Decimal::ZERO, Decimal::ZERO)),
    );
    // Only the SHORT leg's (BBUSDT's) rate changes.
    let short_rate = run_with_accrual(
        make_price_ranked_config(1),
        bars,
        None,
        None,
        Some(constant_rate_map(Decimal::ZERO, dec!(0.01))),
    );

    assert_eq!(
        zero_rate.trades, 2,
        "D-MN.9 #8 fixture: both legs must fill in the control (trades={})",
        zero_rate.trades
    );
    assert_eq!(
        short_rate.trades, zero_rate.trades,
        "D-MN.9 #8 fixture: the funding rate must not change the BOOK (selection is \
         price-driven here), or the equity delta would not be attributable to the accrual. \
         trades: control={}, short-rate={}",
        zero_rate.trades, short_rate.trades
    );

    assert_eq!(
        zero_rate.realized_funding,
        Decimal::ZERO,
        "D-MN.9 #8: the zero-rate control must accrue exactly nothing, got {}",
        zero_rate.realized_funding
    );
    assert_eq!(
        short_rate.realized_funding,
        dec!(100),
        "D-MN.9 #8 SHORT-LEG ACCRUAL NO-OP: a −10 000 short at rate +0.01 must accrue \
         (−10 000) × (−0.01) = +100 (the short RECEIVES on positive funding). Got {}. \
         0 means the short leg is invisible to the accrual — the cost the trace row calls \
         binding would not exist. A NEGATIVE value means the sign convention was inverted.",
        short_rate.realized_funding
    );

    let delta = (short_rate.final_equity - zero_rate.final_equity).abs();
    assert!(
        delta > dec!(1),
        "D-MN.9 #8: the short leg's funding must MOVE equity, not just be computed. \
         eq_zero_rate={}, eq_short_rate={}, delta={}",
        zero_rate.final_equity,
        short_rate.final_equity,
        delta,
    );
    assert_eq!(
        short_rate.final_equity,
        INITIAL_CAPITAL + dec!(100),
        "D-MN.9 #8: with flat prices the whole equity move IS the accrual: \
         100 000 + 100. Got {}",
        short_rate.final_equity
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Falsifier 9: no look-ahead — BOTH sidecars, one at a time
// ─────────────────────────────────────────────────────────────────────────────

/// D-MN.9 #9: future-shifting EITHER sidecar changes the outcome.
///
/// # This falsifier was DECLARED and never implemented (review 1-21)
///
/// The basis arm has `r_br_no_look_ahead_integration` and the funding loader has
/// `funding_data::tests::no_look_ahead_falsifier`, but the MN lane consumes TWO sidecars
/// through two different fields, and nothing tested that. A join that leaked the future
/// on the second sidecar only would have been invisible.
///
/// # Construction — one sidecar at a time
///
/// The residual arm (the only arm that reads both channels) over a two-regime fixture:
/// for the first half AAUSDT is the favoured name, for the second half BBUSDT is.
/// Shifting a sidecar +8 bars into the future makes the arm see the regime change 8 bars
/// early, so the equity must differ from the causal run. Run A shifts only the basis; run
/// B shifts only the funding; both must diverge from causal.
///
/// This is a causality/time-sensitivity gate, not a short-leg gate — it does not go RED
/// when the short branch is deleted, and it does not claim to.
#[test]
fn mn_no_look_ahead_both_sidecars() {
    const N_HOURS: i64 = 48;
    const SHIFT: i64 = 8;

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");

    // AAUSDT uptrend, BBUSDT downtrend — so WHICH name is held changes the equity.
    let mut bars: Vec<Bar> = Vec::new();
    let mut price_a = 1000.0_f64;
    let mut price_b = 500.0_f64;
    for hour in 0..N_HOURS {
        let pa = Decimal::try_from(price_a).expect("AAUSDT uptrend must convert");
        let pb = Decimal::try_from(price_b).expect("BBUSDT downtrend must convert");
        bars.push(make_bar("AAUSDT", pa, hour));
        bars.push(make_bar("BBUSDT", pb, hour));
        price_a *= 1.05;
        price_b *= 0.96;
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Regime: before bar 24 AAUSDT carries the favourable value, after it BBUSDT does.
    let regime = |hour: i64| -> (Decimal, Decimal) {
        if hour < 24 {
            (dec!(-0.01), dec!(0.01))
        } else {
            (dec!(0.02), dec!(-0.02))
        }
    };

    // `shift = 0` → causal; `shift = SHIFT` → the value that belongs to bar `h + SHIFT`
    // is served at bar `h` (a look-ahead by construction).
    let build_map = |shift: i64| -> SidecarMap {
        let mut map: SidecarMap = BTreeMap::new();
        for hour in 0..N_HOURS {
            let src = hour + shift;
            if src < N_HOURS {
                let (ra, rb) = regime(src);
                map.insert((sym_a.clone(), make_ts(hour)), ra);
                map.insert((sym_b.clone(), make_ts(hour)), rb);
            }
        }
        map
    };

    // The residual arm needs a two-symbol universe here.
    let mut cfg = make_mn_residual_config();
    cfg.universe = vec![SmolStr::new("AAUSDT"), SmolStr::new("BBUSDT")];

    let causal = run_to_result(
        cfg.clone(),
        bars.clone(),
        Some(build_map(0)),
        Some(build_map(0)),
    );
    let basis_shifted = run_to_result(
        cfg.clone(),
        bars.clone(),
        Some(build_map(0)),     // funding causal
        Some(build_map(SHIFT)), // BASIS sees the future
    );
    let funding_shifted = run_to_result(
        cfg,
        bars,
        Some(build_map(SHIFT)), // FUNDING sees the future
        Some(build_map(0)),     // basis causal
    );

    assert!(
        causal.trades > 0,
        "D-MN.9 #9: the causal run must trade (trades={})",
        causal.trades
    );

    let d_basis = (causal.final_equity - basis_shifted.final_equity).abs();
    assert!(
        d_basis > dec!(1),
        "D-MN.9 #9 (BASIS sidecar): future-shifting the basis map by {SHIFT} bars must \
         change the outcome. eq_causal={}, eq_basis_shifted={}, delta={}. Δ ≈ 0 means the \
         basis half of the residual is not time-sensitive — either it is not being read \
         at all, or the as-of join ignores the timestamp.",
        causal.final_equity,
        basis_shifted.final_equity,
        d_basis,
    );

    let d_funding = (causal.final_equity - funding_shifted.final_equity).abs();
    assert!(
        d_funding > dec!(1),
        "D-MN.9 #9 (FUNDING sidecar): future-shifting the funding map by {SHIFT} bars must \
         change the outcome. eq_causal={}, eq_funding_shifted={}, delta={}. Δ ≈ 0 means the \
         funding half of the residual is not time-sensitive.",
        causal.final_equity,
        funding_shifted.final_equity,
        d_funding,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// CHARACTERIZATION — bug-log #75 (KNOWN DEFECT, do not read as an endorsement)
// ─────────────────────────────────────────────────────────────────────────────

/// **DOCUMENTS A KNOWN DEFECT — bug-log #75. MUST BE INVERTED AT THE 1-25 RE-LOCK.**
///
/// # Intended behaviour (what this test will assert after the fix)
///
/// `MomentumStrategy` should carry the SCORE sidecar and the ACCRUAL sidecar in two
/// separate fields. Pre-injecting a basis map via `with_funding(Some(basis))` and then
/// handing `run_path` a different `funding_override` for the accrual should leave the
/// score map ALONE: the arm scores on the basis and accrues on the funding.
///
/// # Actual behaviour today (what this test pins)
///
/// `run_path` does `strategy.with_funding(Some(map))` whenever `funding_override` is
/// `Some`, and `with_funding` is a whole-field replacement. The MN lane always passes
/// `Some`. So the pre-injected score map is **always clobbered by the accrual map**, and
/// the `mn-basis` arm (anchors #108-#111) scored on FUNDING, not on the basis.
///
/// The test asserts the defect directly: two runs that differ ONLY in the pre-injected
/// score map produce byte-identical equity, because neither pre-injection survives. A
/// third run, pre-injected with the accrual map itself, produces the same number again —
/// that is the leg that names WHICH map won.
///
/// # At the 1-25 re-lock
///
/// Invert it: the two pre-injections must produce DIFFERENT equity, and the
/// basis-injected run must stop matching the funding-injected run. Do not delete this
/// test — flip its assertions, so the flip is visible in the diff that re-prices the
/// anchors.
#[test]
fn characterization_bug75_funding_override_clobbers_injected_score_map() {
    const N_HOURS: usize = 40;

    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let (bars, basis_map) = build_mn_universe(N_HOURS);

    // A funding map whose ranking is the OPPOSITE of the basis map's, so if the basis
    // map survived, the two would select different legs and could not tie…
    let mut funding_map: SidecarMap = BTreeMap::new();
    // …and a THIRD map, different again, to show the pre-injection is simply ignored
    // rather than merged.
    let mut other_map: SidecarMap = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        funding_map.insert((sym_a.clone(), ts), dec!(-0.030)); // best score under −mean → LONGED
        funding_map.insert((sym_b.clone(), ts), dec!(0.030)); // worst → SHORTED
        funding_map.insert((sym_c.clone(), ts), dec!(0.001));
        other_map.insert((sym_a.clone(), ts), dec!(0.777));
        other_map.insert((sym_b.clone(), ts), dec!(-0.555));
        other_map.insert((sym_c.clone(), ts), dec!(0.123));
    }

    // Production's MN-basis wiring: basis pre-injected as the SCORE, real funding handed
    // to run_path for the ACCRUAL.
    let injected_basis = run_with_accrual(
        make_mn_basis_config(),
        bars.clone(),
        Some(basis_map),
        None,
        Some(funding_map.clone()),
    );
    // Same run, but the pre-injected map is nonsense. If the pre-injection mattered at
    // all, this could not possibly land on the same equity.
    let injected_nonsense = run_with_accrual(
        make_mn_basis_config(),
        bars.clone(),
        Some(other_map),
        None,
        Some(funding_map.clone()),
    );
    // Same run, pre-injected with the accrual map itself — this is what the arm ACTUALLY
    // scored on.
    let injected_funding = run_with_accrual(
        make_mn_basis_config(),
        bars,
        Some(funding_map.clone()),
        None,
        Some(funding_map),
    );

    assert!(
        injected_basis.trades > 0,
        "bug-log #75 characterization: the fixture must trade (trades={})",
        injected_basis.trades
    );

    assert_eq!(
        injected_basis.final_equity, injected_nonsense.final_equity,
        "bug-log #75 CHARACTERIZATION (KNOWN DEFECT): pre-injecting the basis map vs \
         pre-injecting a nonsense map currently makes NO difference — `run_path` \
         overwrites the score map with `funding_override` whenever it is `Some`, which \
         on the MN lane is always. basis-injected={}, nonsense-injected={}. \
         If this assertion FAILS, the two channels have been separated (story 1-25) — \
         that is the FIX, not a regression: invert this test and re-lock anchors \
         #108-#111.",
        injected_basis.final_equity, injected_nonsense.final_equity,
    );

    assert_eq!(
        injected_basis.final_equity, injected_funding.final_equity,
        "bug-log #75 CHARACTERIZATION (KNOWN DEFECT): the arm scores on the ACCRUAL map. \
         Pre-injecting the basis and pre-injecting the funding map land on the same \
         equity ({} vs {}) because only `funding_override` reaches the score. This is why \
         `mn-basis` (#108-#111) and `mn-funding` (#112-#115) came out bit-identical. \
         Invert at the 1-25 re-lock.",
        injected_basis.final_equity, injected_funding.final_equity,
    );
}
