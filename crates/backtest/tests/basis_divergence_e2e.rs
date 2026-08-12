//! Basis-reversal divergence e2e gate tests (M-DEV-6 / M-DEV-7, R-BR.7).
//!
//! ## Day-1 falsifiers (each RED-on-revert, modeled on carry_divergence_e2e.rs)
//!
//! Per the CLAUDE.md non-negotiable (every strategy overlay ships a baseline-equity-
//! divergence e2e from day 1), the basis arm ships these falsifiers BEFORE the
//! anchored run:
//!
//! 1. **`r_br_baseline_equity_divergence`** (R-BR.7 #3 — CLAUDE.md non-negotiable):
//!    The basis arm's output equity diverges from the un-tilted baseline (same path,
//!    `VolAdjustedReturn`) by ≥ 1 bp when the basis decision variable is non-trivial.
//!    The universe is engineered so the low-basis names are NOT the high-momentum names
//!    → guaranteed selection divergence.
//!
//! 2. **`r_br_baseline_divergence_red_on_revert`** — two identical-signal strategies
//!    (both `VolAdjustedReturn`, no basis) produce Δ=0, proving #1 would FAIL if the
//!    basis were not load-bearing.
//!
//! 3. **`r_br_basis_non_no_op`** (R-BR.7 #4): Force the basis signal to a CONSTANT
//!    (no cross-sectional dispersion) → the arm's selection collapses to the baseline
//!    (Δ < ε), proving the basis is load-bearing, not decorative.
//!
//! 4. **`r_br_sign_assertion_integration`** (R-BR.2 / R-BR.7 #2): Correct-sign vs
//!    flipped-sign basis → different equity, proving the sign convention is active at
//!    the integration level. HIGH-basis name (positive basis) should be AVOIDED;
//!    LOW-basis name (negative basis) should be SELECTED.
//!
//! 5. **`r_br_no_look_ahead_integration`** (R-BR.5 / R-BR.7 #5): Future-shifted basis
//!    → different equity from causal basis, proving the as-of join is causal.
//!
//! 6. **`basis_two_run_byte_identity`** (R-BR.7 #6 / M-DEV-7): Run the small-N basis
//!    sweep twice at the same seed; assert identical formatted summaries (catches any
//!    unordered fold in the basis co-resampling or renderer).
//!
//! ## NOT present (by design — D-BR.1)
//!
//! There is NO `r_br_cashflow_non_no_op` test. The basis arm is a SELECTION signal
//! with NO cashflow accrual. The `run_path` accrual block (`montecarlo.rs:322`) is
//! NEVER entered for the basis arm (`TcnScenarioInput.funding_override = None`).
//! The non-no-op guard is the SELECTION collapse (#3 above), not a cashflow collapse.
//!
//! ## Pattern references
//!
//! - `crates/backtest/tests/carry_divergence_e2e.rs` (the carry sibling pattern).
//! - `crates/strategy/src/cross_sectional/momentum.rs::tests::r_br2_sign_assertion_*`.
//! - `crates/backtest/src/basis_data.rs::tests::no_look_ahead_falsifier`.

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

// ── Universe construction for R-BR baseline-divergence guarantee ──────────────
//
// We need: the lowest-basis name is NOT the highest-momentum name.
//
// Design:
//   - AAUSDT: strong uptrend (+5% per bar) → best momentum score; MODERATE basis (+0.002).
//   - BBUSDT: flat (0% per bar) → poor momentum; VERY NEGATIVE basis (−0.015) → best basis_reversal_score.
//   - CCUSDT: strong downtrend (−4% per bar) → worst momentum; POSITIVE basis (+0.01).
//
// With K=1:
//   - Momentum (VolAdjustedReturn) selects AAUSDT (best price score).
//   - BasisReversal selects BBUSDT (most-NEGATIVE basis →
//     basis_reversal_score = −mean(−0.015) = +0.015 → highest score).
//   → Guaranteed selection divergence.

fn build_divergence_universe(n_hours: usize) -> (Vec<Bar>, BTreeMap<(Symbol, Timestamp), Decimal>) {
    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");

    let mut bars: Vec<Bar> = Vec::new();
    let mut price_a = 1000.0_f64;
    let price_b = 500.0_f64; // flat
    let mut price_c = 200.0_f64;

    for hour in 0..n_hours {
        // Review 1-20 wave-2 L: `unwrap_or(dec!(1000))` silently RESET the
        // series to its start value if the f64→Decimal conversion ever failed,
        // which would flatten the "strong uptrend" premise every assertion in
        // this file rests on — and every test would still pass, on a fixture
        // that no longer means what its doc-comment says. Safe at N_HOURS = 30,
        // but raise the bar count far enough and 1000·1.05^n leaves Decimal's
        // range. A panic in a TEST fixture is the correct behaviour: it is a
        // broken premise, not a runtime condition to absorb.
        let p_a = Decimal::try_from(price_a)
            .expect("AAUSDT price must stay inside Decimal's range — raise nothing, lower N_HOURS");
        let p_b = Decimal::try_from(price_b).expect("BBUSDT price must convert (flat series)");
        let p_c = Decimal::try_from(price_c)
            .expect("CCUSDT price must stay inside Decimal's range — lower N_HOURS");
        bars.push(make_bar("AAUSDT", p_a, hour as i64));
        bars.push(make_bar("BBUSDT", p_b, hour as i64));
        bars.push(make_bar("CCUSDT", p_c, hour as i64));
        price_a *= 1.05; // strong uptrend → best momentum
        // price_b: flat → poor momentum
        price_c *= 0.96; // downtrend → worst momentum
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Basis map (the sidecar injected into the strategy via with_funding):
    // AAUSDT: moderate POSITIVE basis (+0.002) — good price, so the basis arm would de-prioritize.
    // BBUSDT: very NEGATIVE basis (−0.015) — worst actual basis → best basis_reversal_score
    //   (= −mean(−0.015) = +0.015 → floats to TOP of top_k_long ranking).
    // CCUSDT: moderate POSITIVE basis (+0.01) — mid basis_reversal_score.
    //
    // With K=1:
    //   - BasisReversal selects BBUSDT (highest basis_reversal_score).
    //   - VolAdjustedReturn selects AAUSDT (best price momentum).
    //   → Guaranteed divergence.
    let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..n_hours {
        let ts = make_ts(hour as i64);
        basis_map.insert((sym_a.clone(), ts), dec!(0.002)); // positive basis → de-prioritized
        basis_map.insert((sym_b.clone(), ts), dec!(-0.015)); // VERY negative → highest reversal score
        basis_map.insert((sym_c.clone(), ts), dec!(0.01)); // moderate positive
    }

    (bars, basis_map)
}

// ── Config builders ───────────────────────────────────────────────────────────

/// Basis-reversal config: K=1 (guaranteed single selection), L=1 (warm up fast).
fn make_basis_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_basis"),
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 1, // L=1 bar lookback (warm up after 1 bar)
        rebalance_minutes: 1,
        k_long: 1, // K=1 → always picks exactly ONE symbol
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: Direction::Momentum, // identity — sign is in the score
        score_source: ScoreSource::BasisReversal,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    }
}

/// Price-only (VolAdjustedReturn) config — the baseline.
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
        score_source: ScoreSource::VolAdjustedReturn,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    }
}

// ── Run one path → PathRunResult ──────────────────────────────────────────────

/// Run one path with the basis map injected as a score sidecar.
///
/// **Integration test design note:** In production sweeps (`param_robustness_sweep.rs`),
/// the basis map is injected ONLY into the strategy score (via `with_funding` before
/// `run_path`), and `TcnScenarioInput.funding_override` is `None` to prevent the
/// `run_path` accrual block from triggering. This preserves D-BR.1 (no cashflow).
///
/// For integration tests, we pass the basis map as `TcnScenarioInput.funding_override`
/// so that `run_path` injects it into the strategy (line 155: `strategy.with_funding(funding_override)`).
/// This means the accrual block IS entered, but its effect on equity is minor and
/// consistent across compared runs. What matters for these tests is SELECTION divergence
/// (which symbol is chosen), not cashflow precision.
///
/// The "no cashflow in production" constraint is enforced by `param_robustness_sweep.rs`,
/// not by these unit/integration tests.
fn run_to_result(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    // The basis sidecar map (basis values keyed by (Symbol, Timestamp)).
    // Passed via funding_override so run_path injects it into the strategy.
    basis_override: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("basis_e2e_test"));
    let input = TcnScenarioInput {
        bar_span_hours: 1,
        scenario_name: "basis-e2e".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: INITIAL_CAPITAL,
        slippage_bps: 0, // zero friction to isolate signal effects
        taker_fee_bps: 0,
        config_id: "test_basis".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        // Pass the basis map via funding_override so run_path injects it into
        // the strategy's funding_map (line 155: `strategy.with_funding(funding_override)`).
        // The accrual block will run but its effect is minor for these selection tests.
        funding_override: basis_override,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in basis divergence e2e test")
}

/// Run one path with the **PRODUCTION** basis wiring (review 1-20 H2).
///
/// This is the inverse of [`run_to_result`] and it is the wiring the anchored
/// surfaces actually ran:
///
/// - the basis map is pre-injected into the STRATEGY (`.with_funding(Some(map))`)
///   by the sweep driver, exactly as `param_robustness_sweep.rs` does before it
///   calls `run_path`; and
/// - `TcnScenarioInput.funding_override` is **`None`**, so the `run_path`
///   accrual block is never entered — the basis is a selection signal with NO
///   cashflow (D-BR.1).
///
/// Why this helper had to exist: every falsifier in this file used to inject the
/// basis through `funding_override`, which makes `run_path` call
/// `strategy.with_funding(funding_override)` itself. Under that wiring the
/// preservation branch at `scenarios/montecarlo.rs` — the story's ONLY
/// production `run_path` change — is never taken, so reverting it to an
/// unconditional `with_funding(None)` left the whole suite green while
/// production clobbered the pre-injected map and scored `None` for every
/// symbol on every bar. Tests built on this helper go RED on that revert.
fn run_to_result_production_wiring(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    // The basis sidecar, pre-injected into the STRATEGY (never into the input).
    basis_map: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("basis_e2e_test"))
        .with_funding(basis_map);
    let input = TcnScenarioInput {
        bar_span_hours: 1,
        scenario_name: "basis-e2e-production-wiring".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: INITIAL_CAPITAL,
        slippage_bps: 0, // zero friction to isolate signal effects
        taker_fee_bps: 0,
        config_id: "test_basis".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        // PRODUCTION: None — the basis rides the strategy, not the input, and
        // the accrual block stays unreachable (D-BR.1, no cashflow).
        funding_override: None,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in basis divergence e2e test")
}

/// Initial capital for every run in this file (mirrors `run_to_result`).
const INITIAL_CAPITAL: Decimal = dec!(100_000);

// ─────────────────────────────────────────────────────────────────────────────
// R-BR.7 #3 — baseline equity divergence (CLAUDE.md non-negotiable)
// ─────────────────────────────────────────────────────────────────────────────

/// R-BR.7 #3 PASS: Basis-reversal and vol-adjusted-return strategies produce DIFFERENT
/// equity curves when the low-basis name differs from the high-momentum name.
///
/// Universe: BBUSDT has the most-negative basis → BasisReversal selects BBUSDT.
///           AAUSDT has the best price momentum → VolAdjustedReturn selects AAUSDT.
/// With K=1, the two strategies MUST select different symbols → different equity.
///
/// The equity curves must diverge by ≥ 1 bp — the CLAUDE.md non-negotiable guard
/// against a v3-vol-overlay-style no-op where the signal is computed but never applied.
///
/// # Which reverts this gate catches (review 1-20 H3)
///
/// The price-baseline comparison ALONE was **vacuous** against the exact failure
/// class the CLAUDE.md non-negotiable exists to catch. Under the canonical no-op
/// — `basis_reversal_score` returning `None` for every symbol on every bar, the
/// v3-vol-overlay "computed but never applied" shape — the basis arm never ranks
/// anything, never trades, and its equity stays pinned at the initial capital,
/// while the price baseline compounds the +5%/bar uptrend to roughly +165 k. The
/// two are maximally far apart, so `|Δ| > 10` PASSED on a completely inert arm.
///
/// The gate now additionally requires:
///
/// 1. **the basis arm actually TRADED** — non-zero fills. A no-op scores `None`
///    everywhere, emits no signals, and lands here at `trades == 0`; and
/// 2. **its equity differs from a basis-DISABLED but otherwise IDENTICAL arm** —
///    same `ScoreSource::BasisReversal` config, same bars, same seed, only the
///    sidecar withheld. A no-op collapses the armed run onto that control
///    exactly, because both then score `None` for every symbol.
///
/// Together these two are RED under the no-op and GREEN only when the basis is
/// genuinely load-bearing. The price-baseline leg is kept as the literal AD-16
/// "diverges from the un-targeted baseline" statement.
#[test]
fn r_br_baseline_equity_divergence() {
    const N_HOURS: usize = 30; // enough for warmup (L=1) + multiple rebalances
    const EPSILON_BPS: f64 = 0.0001; // 1 bp = 0.01%

    let (bars, basis_map) = build_divergence_universe(N_HOURS);

    // Basis-reversal: injects basis via with_funding (score-only), funding_override=None.
    let result_basis = run_to_result(make_basis_config(), bars.clone(), Some(basis_map));
    // Price-only: no basis sidecar.
    let result_price = run_to_result(make_price_config(), bars.clone(), None);
    // Basis-DISABLED control: byte-identical config to `result_basis` — same
    // ScoreSource::BasisReversal, same universe, same K, same L, same seed,
    // same bars — with ONLY the sidecar withheld.
    let result_basis_disabled = run_to_result(make_basis_config(), bars, None);

    let eq_basis = result_basis.final_equity;
    let eq_price = result_price.final_equity;
    let eq_disabled = result_basis_disabled.final_equity;
    let delta = (eq_basis - eq_price).abs();
    // A silent fallback here would swap the 1 bp threshold for a magic 10 and
    // nobody would know which one the assertion used (1-20 wave-2 L).
    let epsilon = Decimal::try_from(EPSILON_BPS * 100_000.0)
        .expect("1 bp of the initial capital must convert to Decimal");

    // ── (1) NON-VACUITY: the basis arm must actually have traded ─────────────
    assert!(
        result_basis.trades > 0,
        "R-BR.7 #3 VACUITY VIOLATION: the basis arm executed {} fills — it never traded. \
         An arm that never trades trivially 'diverges' from the compounding price \
         baseline, which is exactly how the v3-vol-overlay no-op class slips through a \
         divergence gate. If basis_reversal_score returns None for every symbol (no \
         sidecar reaching the ring, or the score wired but never consumed), this is \
         where it must stop.",
        result_basis.trades
    );

    // ── (2) NON-VACUITY: differ from the basis-DISABLED, otherwise-identical arm ─
    assert_ne!(
        eq_basis, eq_disabled,
        "R-BR.7 #3 VACUITY VIOLATION: the basis arm ({eq_basis}) produced the SAME equity \
         as the basis-DISABLED control ({eq_disabled}) — identical config, identical bars, \
         identical seed, sidecar withheld. Withholding the signal changed nothing, so the \
         signal is decorative: it is computed and then not applied. This is the AD-16 \
         no-op the CLAUDE.md non-negotiable exists to catch."
    );
    assert_eq!(
        result_basis_disabled.trades, 0,
        "control sanity: with no sidecar every basis_reversal_score is None, nothing is \
         ranked and nothing trades. Got {} fills — if this is non-zero the control is not \
         a basis-disabled arm and assertion (2) above proves nothing.",
        result_basis_disabled.trades
    );

    // ── (3) AD-16 literal: diverge from the un-targeted price baseline ───────
    assert!(
        delta > epsilon,
        "R-BR.7 #3 DIVERGENCE VIOLATION: basis equity ({eq_basis}) must differ from \
         price equity ({eq_price}) by ≥ {epsilon} (1 bp). delta={delta}. \
         If delta ≈ 0, the basis signal is not selecting differently from the price signal. \
         Universe design: BBUSDT (most-negative basis = −0.015) should be selected by BasisReversal; \
         AAUSDT (strong uptrend +5%/bar) should be selected by VolAdjustedReturn. \
         Check ScoreSource::BasisReversal and the with_funding injection."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Review 1-20 H2 — the PRODUCTION run_path wiring (the only production change)
// ─────────────────────────────────────────────────────────────────────────────

/// The story's only production `run_path` change, under test at last.
///
/// `scenarios/montecarlo.rs` replaced an unconditional
/// `strategy.with_funding(funding_override)` with a preservation branch:
///
/// ```ignore
/// let mut strategy = if let Some(map) = funding_override {
///     strategy.with_funding(Some(map))
/// } else {
///     strategy            // do NOT clobber a pre-injected sidecar
/// };
/// ```
///
/// That `else` arm is the ONLY thing stopping `run_path` from wiping the basis
/// map the sweep driver pre-injects into the strategy. Every other falsifier in
/// this file passes the basis through `funding_override` — the INVERSE of the
/// production wiring — so all of them take the `if` arm and none of them can
/// observe the `else` arm at all. Reverting the branch to an unconditional
/// `with_funding(None)` left this entire suite green while every production
/// surface scored `None` for every symbol on every bar.
///
/// This test drives the production wiring directly: the map goes into the
/// STRATEGY, `funding_override` stays `None`, and the run must still trade and
/// still diverge from a no-sidecar control. Under the revert, `armed` is handed
/// `funding_map = None`, scores `None` everywhere, trades zero times, and lands
/// exactly on the control — both assertions below go RED.
#[test]
fn r_br_production_wiring_preserves_pre_injected_basis_map() {
    const N_HOURS: usize = 30;

    let (bars, basis_map) = build_divergence_universe(N_HOURS);

    // PRODUCTION: basis pre-injected into the strategy; funding_override = None.
    let armed = run_to_result_production_wiring(make_basis_config(), bars.clone(), Some(basis_map));
    // Control: same wiring, no sidecar at all.
    let control = run_to_result_production_wiring(make_basis_config(), bars, None);

    assert!(
        armed.trades > 0,
        "PRODUCTION-WIRING REGRESSION: with the basis map pre-injected into the strategy \
         and TcnScenarioInput.funding_override = None, run_path must PRESERVE the map — \
         got {} fills, i.e. the arm never traded. This is exactly what an unconditional \
         `strategy.with_funding(funding_override)` produces: the pre-injected sidecar is \
         overwritten with None, every basis_reversal_score returns None, nothing is ranked. \
         See the preservation branch in crates/backtest/src/scenarios/montecarlo.rs.",
        armed.trades
    );
    assert_eq!(
        control.trades, 0,
        "control sanity: with no sidecar anywhere the basis arm cannot rank or trade. \
         Got {} fills.",
        control.trades
    );
    assert_ne!(
        armed.final_equity, control.final_equity,
        "PRODUCTION-WIRING REGRESSION: the pre-injected basis map made NO difference to \
         equity (armed={}, control={}). Under the production wiring the map reaches the \
         score ONLY via the run_path preservation branch; if it is clobbered, the armed \
         run degenerates to the control exactly, which is what this equality means.",
        armed.final_equity, control.final_equity
    );
    // D-BR.1: the basis is a SELECTION signal — the accrual block must stay
    // unreachable under the production wiring (funding_override = None).
    assert_eq!(
        armed.realized_funding,
        Decimal::ZERO,
        "D-BR.1 VIOLATION: the basis arm must accrue ZERO cashflow — the run_path accrual \
         block is entered only when TcnScenarioInput.funding_override is Some, and the \
         production basis wiring leaves it None. Got {}.",
        armed.realized_funding
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-BR.7 #3 RED-on-revert — identical strategies produce Δ=0
// ─────────────────────────────────────────────────────────────────────────────

/// RED-on-revert proof: two identical-signal strategies (both VolAdjustedReturn, no basis)
/// produce IDENTICAL equity curves.
///
/// This proves the divergence gate actively DETECTS the injection no-op: if the basis
/// signal were not wired correctly (and both strategies fell back to price), the
/// r_br_baseline_equity_divergence test would FAIL — confirming the gate works.
#[test]
fn r_br_baseline_divergence_red_on_revert() {
    const N_HOURS: usize = 30;

    let (bars, _basis_map) = build_divergence_universe(N_HOURS);

    // Both configs use VolAdjustedReturn (no basis signal).
    let cfg1 = make_price_config();
    let mut cfg2 = make_price_config();
    cfg2.id = SmolStr::new("test_price_2"); // different ID, same signal

    let result1 = run_to_result(cfg1, bars.clone(), None);
    let result2 = run_to_result(cfg2, bars, None);

    let delta = (result1.final_equity - result2.final_equity).abs();

    assert_eq!(
        result1.final_equity, result2.final_equity,
        "R-BR.7 #3 RED-ON-REVERT: two identical-signal strategies (both VolAdjustedReturn, \
         no basis) must produce IDENTICAL equity. delta={}. \
         If they differ, the score source injection has unexpected side effects.",
        delta,
    );
    assert_eq!(
        delta,
        Decimal::ZERO,
        "Identical configs must produce zero delta (perfect determinism + same signal)."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-BR.7 #4 — basis-signal non-no-op (SELECTION collapse on constant basis)
// ─────────────────────────────────────────────────────────────────────────────

/// R-BR.7 #4 basis non-no-op: force the basis to a CONSTANT across all symbols
/// → no cross-sectional dispersion → the arm's selection collapses to the baseline.
///
/// **Construction:** all symbols get the SAME basis value at every bar → the
/// `basis_reversal_score` ranks are all equal (no signal) → the two strategies
/// (basis-reversal with constant basis vs price baseline) produce IDENTICAL equity
/// because the selection is the same (the basis signal carries no cross-sectional info).
///
/// **Contrast:** The real-basis run (disparate basis) DOES diverge from price (verified
/// in r_br_baseline_equity_divergence). The constant-basis run does NOT.
/// This proves the basis is load-bearing, not decorative.
///
/// **Universe design:** AAUSDT has the SAME basis as BBUSDT and CCUSDT → no basis signal.
/// With VolAdjustedReturn (momentum) the strategy selects AAUSDT (strong +5%/bar uptrend).
/// With BasisReversal + CONSTANT basis, all `basis_reversal_score`s are equal → selection
/// depends on tie-breaking, which converges to the same behavior as the base case.
///
/// NOTE: The basis arm has NO cashflow (D-BR.1). The non-no-op guard is the
/// SELECTION collapse, not a cashflow collapse (unlike carry's R-CARRY.10b).
#[test]
fn r_br_basis_non_no_op() {
    const N_HOURS: usize = 30;

    let (bars, real_basis_map) = build_divergence_universe(N_HOURS);

    // Build a CONSTANT basis map: all symbols have the SAME basis value.
    // No cross-sectional dispersion → no signal → selection identical to any other basis arm.
    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");
    let sym_c = Symbol::new("CCUSDT");
    let mut constant_basis_same_sym: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    // Same value for all — we test TWO constant-basis runs with the same value.
    // They must produce identical equity (since both have the same signal dispersion = 0).
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        constant_basis_same_sym.insert((sym_a.clone(), ts), dec!(0.005));
        constant_basis_same_sym.insert((sym_b.clone(), ts), dec!(0.005));
        constant_basis_same_sym.insert((sym_c.clone(), ts), dec!(0.005));
    }

    // Run 1: BasisReversal with CONSTANT basis (no dispersion).
    let result_const1 = run_to_result(
        make_basis_config(),
        bars.clone(),
        Some(constant_basis_same_sym.clone()),
    );

    // Run 2: Another BasisReversal with SAME constant basis (different config ID, same signal).
    let mut cfg2 = make_basis_config();
    cfg2.id = SmolStr::new("test_basis_2");
    let result_const2 = run_to_result(cfg2, bars.clone(), Some(constant_basis_same_sym));

    // Both constant-basis runs must be IDENTICAL (same signal → same selection → same equity).
    // This proves that the strategy IS influenced by the basis dispersion.
    assert_eq!(
        result_const1.final_equity, result_const2.final_equity,
        "R-BR.7 #4 BASIS NON-NO-OP: two BasisReversal strategies with IDENTICAL constant \
         basis must produce IDENTICAL equity. If they differ, the score has non-determinism. \
         eq_const1={}, eq_const2={}",
        result_const1.final_equity, result_const2.final_equity,
    );

    // The REAL-basis run (disparate signal) must produce DIFFERENT equity from the
    // constant-basis run. If they're identical, the basis signal is decorative.
    let result_real = run_to_result(make_basis_config(), bars, Some(real_basis_map));
    let delta = (result_real.final_equity - result_const1.final_equity).abs();
    let epsilon = dec!(1); // any measurable difference

    assert!(
        delta > epsilon,
        "R-BR.7 #4 BASIS NON-NO-OP VIOLATION: real basis (disparate signal) must produce \
         DIFFERENT equity than constant basis (no cross-sectional signal). \
         eq_real={}, eq_const={}, delta={}. \
         If delta ≈ 0, the basis signal is not load-bearing — changing the signal does not \
         change the selection. Check that basis_reversal_score drives selection in on_bar().",
        result_real.final_equity,
        result_const1.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-BR.2 / R-BR.7 #2 — sign assertion (integration level)
// ─────────────────────────────────────────────────────────────────────────────

/// R-BR.2 sign re-confirmation: the basis-reversal strategy LONGS the most-NEGATIVE-
/// basis name (the reversal-favored leg: low-basis names outperform).
///
/// Universe: AAUSDT (strong uptrend +5%/bar, positive basis +0.01) vs BBUSDT (flat,
/// very negative basis −0.02). K=1.
///
/// With correct R-BR.2 sign (−trailing_mean):
///   - Correct basis: BBUSDT (−0.02) → basis_reversal_score = −(−0.02) = +0.02 → selected.
///     BBUSDT is FLAT → poor equity gain from price.
///   - Flipped basis: AAUSDT (was +0.01, flipped to −0.01) → basis_reversal_score = −(−0.01)
///     = +0.01 → AAUSDT is selected. AAUSDT is strong uptrend → much better equity.
///
/// The two runs produce DIFFERENT equity because they select DIFFERENT symbols with
/// very different price trajectories.
///
/// # Why "different" is not enough (review 1-20 H4)
///
/// This test used to assert only `|Δ| > 1` between the correct-sign and
/// flipped-sign runs. That assertion is **symmetric**, so it cannot see a sign
/// flip at all: mutate `basis_reversal_score` from `Some(-mean)` to
/// `Some(mean)` and the two runs simply swap roles — the correct-basis run now
/// picks AAUSDT and the flipped-basis run picks BBUSDT. `|Δ|` is bit-for-bit
/// the same and the test stays GREEN. The only genuine sign guards in the tree
/// were the two `momentum.rs` unit tests, which assert literal score values;
/// nothing at the integration level could go RED on a `+mean` mutant.
///
/// The assertions below are now **directional**. The universe is built so the
/// held symbol is readable straight off the final equity: BBUSDT is flat at 500
/// for all 30 bars, AAUSDT compounds +5%/bar, friction is zero, and the run
/// uses the production wiring (`funding_override = None`) so there is no
/// accrual term muddying the reading. Therefore
///
/// - holding the LOW-basis name (BBUSDT, flat) ⇒ final equity ≈ initial; and
/// - holding the HIGH-basis name (AAUSDT, +5%/bar) ⇒ final equity ≫ initial.
///
/// The correct sign must produce the FIRST outcome and the flipped basis the
/// SECOND. Under a `+mean` mutant every one of the three assertions inverts.
#[test]
fn r_br_sign_assertion_integration() {
    const N_HOURS: usize = 30;

    let sym_a = Symbol::new("AAUSDT"); // strong uptrend; positive basis (+0.01)
    let sym_b = Symbol::new("BBUSDT"); // flat price; very negative basis (−0.02)

    // AAUSDT: strong uptrend (+5%/bar). BBUSDT: flat.
    let mut bars: Vec<Bar> = Vec::new();
    let mut price_a = 1000.0_f64;
    for hour in 0..N_HOURS {
        // See build_divergence_universe: a silent reset here would flatten the
        // +5%/bar uptrend that the DIRECTION assertions below use to identify
        // which symbol the arm held (review 1-20 wave-2 L).
        let pa = Decimal::try_from(price_a)
            .expect("AAUSDT uptrend must stay inside Decimal's range at N_HOURS bars");
        bars.push(make_bar("AAUSDT", pa, hour as i64));
        bars.push(make_bar("BBUSDT", dec!(500), hour as i64));
        price_a *= 1.05; // +5% per bar
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Correct basis: AAUSDT positive (+0.01), BBUSDT negative (−0.02).
    // → With BasisReversal: BBUSDT scores +0.02 > AAUSDT score −0.01 → BBUSDT selected.
    // → BBUSDT is flat → smaller equity gain than holding AAUSDT.
    let mut basis_correct: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        basis_correct.insert((sym_a.clone(), ts), dec!(0.01));
        basis_correct.insert((sym_b.clone(), ts), dec!(-0.02));
    }

    // Flipped basis: AAUSDT negative (−0.01), BBUSDT positive (+0.02).
    // → With BasisReversal: AAUSDT scores −(−0.01) = +0.01 → AAUSDT selected.
    // → AAUSDT has strong +5% uptrend → much larger equity gain.
    let mut basis_flipped: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        basis_flipped.insert((sym_a.clone(), ts), dec!(-0.01)); // flipped
        basis_flipped.insert((sym_b.clone(), ts), dec!(0.02)); // flipped
    }

    let two_sym_cfg = strategy::CrossSectionalMomentumConfig {
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
        score_source: ScoreSource::BasisReversal,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    };

    // Production wiring: the basis rides the strategy, not `funding_override`,
    // so there is NO accrual term and final equity reads purely as "which
    // symbol did the arm hold".
    let result_correct =
        run_to_result_production_wiring(two_sym_cfg.clone(), bars.clone(), Some(basis_correct));
    let result_flipped = run_to_result_production_wiring(two_sym_cfg, bars, Some(basis_flipped));

    let eq_correct = result_correct.final_equity;
    let eq_flipped = result_flipped.final_equity;

    // Both arms must actually have traded — otherwise the comparison below is
    // between two inert runs and proves nothing about the sign.
    assert!(
        result_correct.trades > 0 && result_flipped.trades > 0,
        "R-BR.2 sign assertion (integration) VACUITY: both arms must trade. \
         correct={} fills, flipped={} fills. Two inert arms compare equal and would \
         mask any sign behaviour whatsoever.",
        result_correct.trades,
        result_flipped.trades,
    );

    // ── DIRECTION 1: correct sign LONGS the LOW-basis name (BBUSDT, flat) ────
    // Holding a flat asset with zero friction leaves equity at initial capital.
    // Riding AAUSDT's +5%/bar for 30 bars cannot land inside this band.
    let flat_band = INITIAL_CAPITAL + dec!(100);
    assert!(
        eq_correct <= flat_band,
        "R-BR.2 SIGN VIOLATION (integration): with the CORRECT sign (−mean) the arm must \
         long the LOW-basis name — BBUSDT at basis −0.02 — which is FLAT at 500 for all \
         {N_HOURS} bars, so final equity must stay at ~{INITIAL_CAPITAL} (≤ {flat_band} \
         with zero friction). Got {eq_correct}, which is the signature of holding AAUSDT \
         (+5%/bar, basis +0.01 — the HIGH-basis, crowded-long name). \
         basis_reversal_score is returning +mean: the arm is a basis-MOMENTUM payer."
    );

    // ── DIRECTION 2: flipped basis LONGS AAUSDT (the +5%/bar trend) ──────────
    // With the signs on the map inverted, AAUSDT becomes the low-basis name and
    // the SAME correct `−mean` score must now pick it up.
    // Observed at 30 bars, K=1, exposure_cap 0.5, zero friction: ~136 k when
    // AAUSDT is held, exactly 100 k when flat BBUSDT is held. The 1.10×
    // threshold sits far from BOTH — flat can never reach it (the price never
    // moves), and the trend clears it with a wide margin.
    let trend_floor = INITIAL_CAPITAL * dec!(1.10);
    assert!(
        eq_flipped > trend_floor,
        "R-BR.2 SIGN VIOLATION (integration): with the basis MAP flipped, AAUSDT becomes \
         the LOW-basis name (−0.01) and the correct −mean score must select it. AAUSDT \
         compounds +5%/bar over {N_HOURS} bars, so final equity must clear {trend_floor}. \
         Got {eq_flipped} — at/near {INITIAL_CAPITAL} the arm is still holding flat \
         BBUSDT, so the score is not responding to the basis sign at all."
    );

    // ── DIRECTION 3: the ORDERING itself (what a symmetric |Δ| could never see) ─
    assert!(
        eq_correct < eq_flipped,
        "R-BR.2 SIGN VIOLATION (integration): correct-sign equity ({eq_correct}) must be \
         BELOW flipped-basis equity ({eq_flipped}) — the correct sign deliberately buys \
         the un-crowded, flat name and forgoes the crowded name's trend. If this ordering \
         inverts, `basis_reversal_score` returns +mean and the two runs have swapped \
         roles: the arm now longs the crowded-long names that subsequently underperform. \
         NOTE: an `|eq_correct − eq_flipped| > ε` assertion is symmetric and stays GREEN \
         under exactly that mutation — which is why this test asserts the ORDER."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-BR.5 / R-BR.7 #5 — no-look-ahead (integration level)
// ─────────────────────────────────────────────────────────────────────────────

/// R-BR.5 no-look-ahead re-confirmation: future-shifting the basis series changes
/// the equity outcome.
///
/// The basis at the open of bar t must use only past basis (basis_close[t-1]).
/// If the basis series is shifted +8 bars into the future (look-ahead), the strategy
/// uses future basis, flipping its selection at the regime-change boundary 8 bars early.
///
/// **Universe design:** AAUSDT has a strong uptrend (+5%/bar); BBUSDT has a strong
/// downtrend (−4%/bar). The basis flips at bar 24:
///   - Bars 0-23: AAUSDT has negative basis → causal arm longs AAUSDT (uptrending) → big gains.
///   - Bars 24-47: BBUSDT has negative basis → causal arm switches to BBUSDT (downtrending).
///
/// With FUTURE-SHIFTED basis (+8 bars look-ahead): the arm switches FROM AAUSDT to BBUSDT
/// 8 bars EARLIER (at bar 16 instead of 24), missing 8 bars of AAUSDT uptrend → lower equity.
///
/// Causal equity ≠ future-shifted equity → the as-of join is causal.
#[test]
fn r_br_no_look_ahead_integration() {
    const N_HOURS: usize = 48;

    let sym_a = Symbol::new("AAUSDT"); // strong uptrend +5%/bar
    let sym_b = Symbol::new("BBUSDT"); // strong downtrend −4%/bar

    // Bars: AAUSDT uptrend, BBUSDT downtrend.
    // Different price trajectories ensure selection matters for equity.
    let mut bars: Vec<Bar> = Vec::new();
    let mut price_a = 1000.0_f64;
    let mut price_b = 500.0_f64;
    for hour in 0..N_HOURS {
        // Same fixture landmine as above (review 1-20 wave-2 L): a silent reset
        // would flatten both trends and the causal/shifted runs would compare
        // equal for the wrong reason.
        let pa = Decimal::try_from(price_a).expect("AAUSDT uptrend must convert");
        let pb = Decimal::try_from(price_b).expect("BBUSDT downtrend must convert");
        bars.push(make_bar("AAUSDT", pa, hour as i64));
        bars.push(make_bar("BBUSDT", pb, hour as i64));
        price_a *= 1.05; // +5% per bar (strong uptrend)
        price_b *= 0.96; // −4% per bar (downtrend)
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

    // Causal basis: AAUSDT negative (selected) for bars 0-23, BBUSDT negative for bars 24-47.
    let mut basis_causal: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..N_HOURS {
        let ts = make_ts(hour as i64);
        let (rate_a, rate_b) = if hour < 24 {
            (dec!(-0.01), dec!(0.01)) // bars 0-23: AAUSDT negative → selected → uptrending
        } else {
            (dec!(0.02), dec!(-0.02)) // bars 24-47: BBUSDT negative → selected → downtrending
        };
        basis_causal.insert((sym_a.clone(), ts), rate_a);
        basis_causal.insert((sym_b.clone(), ts), rate_b);
    }

    // Shifted basis: offset all entries +8 bars into the future (look-ahead).
    // The arm sees the "bars 24-47" basis 8 bars early → switches to BBUSDT at bar 16.
    // Missing 8 bars of AAUSDT uptrend → lower equity than the causal run.
    let shift: i64 = 8;
    let mut basis_shifted: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..(N_HOURS as i64) {
        let ts = make_ts(hour);
        let future_hour = hour + shift;
        if future_hour < N_HOURS as i64 {
            let (rate_a, rate_b) = if future_hour < 24 {
                (dec!(-0.01), dec!(0.01))
            } else {
                (dec!(0.02), dec!(-0.02))
            };
            basis_shifted.insert((sym_a.clone(), ts), rate_a);
            basis_shifted.insert((sym_b.clone(), ts), rate_b);
        }
    }

    let two_sym_cfg = strategy::CrossSectionalMomentumConfig {
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
        score_source: ScoreSource::BasisReversal,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    };

    let result_causal = run_to_result(two_sym_cfg.clone(), bars.clone(), Some(basis_causal));
    let result_shifted = run_to_result(two_sym_cfg, bars, Some(basis_shifted));

    // The two results must differ — the causal arm holds AAUSDT longer (uptrend)
    // than the shifted arm, which exits early to BBUSDT (downtrend).
    let delta = (result_causal.final_equity - result_shifted.final_equity).abs();
    let epsilon = dec!(1);

    assert!(
        delta > epsilon,
        "R-BR.5 no-look-ahead (integration): causal basis must produce different equity \
         than future-shifted basis. eq_causal={}, eq_shifted={}, delta={}. \
         If delta ≈ 0, the as-of join is not causal — future basis leaks into the score. \
         Expected: causal arm holds AAUSDT (uptrend) longer; shifted arm exits 8 bars early.",
        result_causal.final_equity,
        result_shifted.final_equity,
        delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-BR.7 #6 / M-DEV-7 — two-run byte-identity
// ─────────────────────────────────────────────────────────────────────────────

/// ADR-0051 § D6.9 / R-BR.7 #6: Run the small-N basis sweep twice at the same
/// `ensemble_seed`; assert byte-identical formatted summaries.
///
/// This catches any unordered fold in the basis co-resampling or the renderer.
/// (Model on `carry_divergence_e2e.rs::carry_two_run_byte_identity`.)
#[test]
fn basis_two_run_byte_identity() {
    // Small universe, N=3 paths, 2 cells — enough to cover the determinism gate.
    // Uses synthetic bars + synthetic basis (no real data needed).

    const N_PATHS: usize = 3;
    const N_BARS: usize = 40;
    const MASTER_SEED: u64 = 0xC0FFEE;

    let cells: &[(u32, u32)] = &[(1, 1), (1, 3)]; // (lookback, k_long)

    let run_sweep_once = || -> Vec<String> {
        let mut cell_summaries: Vec<String> = Vec::new();

        for &(lookback, k_long) in cells {
            let cfg = strategy::CrossSectionalMomentumConfig {
                id: SmolStr::new("basis_det_test"),
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
                score_source: ScoreSource::BasisReversal,
                selection_mode: strategy::SelectionMode::CrossSectionalTopK,
                entry_threshold: Decimal::ZERO,
            };

            let mut metrics: Vec<PathMetrics> = Vec::new();

            for j in 0..N_PATHS {
                let seed_j = path_seed(MASTER_SEED, j);

                // Build synthetic bars (deterministic from seed).
                let bars = build_synthetic_bars_2sym(seed_j, N_BARS);

                // Build synthetic basis map (deterministic: alternating neg/pos by bar).
                let basis = build_synthetic_basis(N_BARS);

                let result = run_to_result(cfg.clone(), bars, Some(basis));

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

            // Metrics are already in j order (sequential iteration above).
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
        "basis two-run byte-identity: runs at the same seed must produce identical summaries. \
         If they differ, there is non-determinism in the basis path (basis co-resampling, \
         ring-buffer state, or reduction order). ADR-0051 § D6.9 violation."
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

        // Silent resets here would make the two determinism runs agree for the
        // wrong reason — both pinned to 1 (1-20 wave-2 L).
        let pa = Decimal::try_from(next_a).expect("synthetic AAUSDT close must convert");
        let pb = Decimal::try_from(next_b).expect("synthetic BBUSDT close must convert");

        bars.push(make_bar("AAUSDT", pa, hour as i64));
        bars.push(make_bar("BBUSDT", pb, hour as i64));

        close_a = next_a;
        close_b = next_b;
    }

    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));
    bars
}

fn build_synthetic_basis(n_bars: usize) -> BTreeMap<(Symbol, Timestamp), Decimal> {
    let sym_a = Symbol::new("AAUSDT");
    let sym_b = Symbol::new("BBUSDT");

    let mut basis: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
    for hour in 0..n_bars {
        let ts = make_ts(hour as i64);
        // Alternating: AAUSDT has negative basis on even hours, BBUSDT on odd.
        let (rate_a, rate_b) = if hour % 2 == 0 {
            (dec!(-0.005), dec!(0.005)) // even: AAUSDT selected (negative → higher reversal score)
        } else {
            (dec!(0.005), dec!(-0.005)) // odd: BBUSDT selected
        };
        basis.insert((sym_a.clone(), ts), rate_a);
        basis.insert((sym_b.clone(), ts), rate_b);
    }
    basis
}
