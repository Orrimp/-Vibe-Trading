//! MR divergence e2e gate tests (M-DEV-5).
//!
//! ## R-MR.1(a) — MR-vs-momentum divergence PASS (the headline anti-no-op)
//!
//! Same path through a `Momentum` and a `Reversion` strategy at the same θ
//! (K < universe size) → equity curves diverge by ≥ 1 bp (ε).
//! Proves the inversion is not a no-op — different symbols are selected.
//!
//! ## R-MR.1(b) — same-config determinism / noise-floor control
//!
//! Runs Momentum-vs-Momentum (two IDENTICAL configs over the identical path)
//! and asserts ZERO divergence. Review 1-16 truthfix: this is a CONTROL, not a
//! detector — it establishes that the harness has no noise floor (identical
//! configs ⇒ identical curves), so the divergence observed in (a) is
//! attributable to the direction flip alone. The dropped-negation DETECTOR is
//! (a): if the D-MR.1 negation were dropped, Reversion would degenerate to
//! Momentum and (a)'s ≥ 1 bp divergence assert would go RED.
//!
//! BOTH (a) and (b) ship per the spec (D-MR.5-GATE).
//!
//! ## FP-MR.3 — two-run byte-identity of the MR θ-surface
//!
//! Same grid + same seeds twice → byte-identical formatted summaries.
//! Catches any unordered fold in the θ-loop or reducer.
//!
//! ## FP-MR.5 — anti-cherry-pick through the REAL renderer (review 1-16)
//!
//! The Reversion-arm render over the LOCKED MR grid carries the MR family
//! labeling and the family line from the production single source
//! (`family_verdict_line`). The generic family-line/C5-flag invariants are
//! gated by `tests/param_sweep_e2e.rs::fp_c3_5_real_renderer_family_line_and_c5_flags`
//! on the SAME shared renderer.
//!
//! ## Pattern references
//!
//! - `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (CLAUDE.md non-negotiable).
//! - `crates/backtest/tests/param_sweep_e2e.rs` (C3 θ-surface two-run + divergence gate).

use backtest::cli_types::TcnScenarioInput;
use backtest::scenarios::montecarlo::run_path;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::Direction;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── ADR-0051 D1 + D6.1 seed derivation (same formula as the production code) ─

fn path_seed(master: u64, j: usize) -> u64 {
    master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))
}

// ── Config builder ────────────────────────────────────────────────────────────

/// Build a config with a 3-symbol universe (K=1 < universe size = 3).
///
/// With K=1 and distinct trends, Momentum and Reversion select OPPOSITE symbols,
/// guaranteeing divergence — the cleanest possible anti-no-op signal.
fn make_config_mr(direction: Direction) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("test_mr"),
        // 3 symbols so K=1 selections are always disjoint for Momentum vs Reversion.
        universe: vec![
            SmolStr::new("AAUSDT"),
            SmolStr::new("BBUSDT"),
            SmolStr::new("CCUSDT"),
        ],
        lookback_minutes: 5,
        rebalance_minutes: 5,
        k_long: 1, // K=1 < 3 = universe size → always disjoint selections
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction,
        score_source: strategy::ScoreSource::VolAdjustedReturn,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: rust_decimal::Decimal::ZERO,
    }
}

// ── Synthetic bar builder with controlled trends ───────────────────────────────

/// Build a synthetic bar series for one symbol with a deterministic trend.
///
/// `trend_pct_per_bar`: positive = uptrend (momentum winner / MR loser),
///                       negative = downtrend (momentum loser / MR winner).
fn trending_bars(sym_name: &str, n: usize, start_price: f64, trend_pct_per_bar: f64) -> Vec<Bar> {
    let sym = Symbol::new(sym_name);
    let epoch = {
        let date =
            time::Date::from_calendar_date(2023, time::Month::January, 1).expect("valid date");
        time::OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    let mut bars = Vec::with_capacity(n);
    let mut price = start_price;

    for i in 0..n {
        let next = (price * (1.0 + trend_pct_per_bar)).max(0.01);

        #[allow(clippy::cast_possible_wrap)]
        let open_ts = Timestamp::new(epoch + time::Duration::hours(i as i64));
        #[allow(clippy::cast_possible_wrap)]
        let close_ts = Timestamp::new(
            epoch + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1),
        );

        let to_price = |v: f64| -> Price {
            Price::new(Decimal::try_from(v.max(0.01)).unwrap_or(dec!(0.01)))
                .unwrap_or_else(|_| Price::new(dec!(0.01)).expect("0.01 always valid"))
        };

        bars.push(Bar {
            symbol: sym.clone(),
            tf: Timeframe::OneHour,
            open_ts,
            close_ts,
            open: to_price(price),
            high: to_price(price.max(next) * 1.001),
            low: to_price(price.min(next) * 0.999),
            close: to_price(next),
            volume: Quantity::new(dec!(100)).expect("100 always valid"),
            trade_count: 10,
            local_recv_ts: close_ts,
            venue: Venue::Binance,
        });
        price = next;
    }
    bars
}

/// Build a 3-symbol merged bar list with distinct trends.
///
/// AAUSDT: strong uptrend (+5% per bar) — momentum winner.
/// BBUSDT: flat (0% per bar).
/// CCUSDT: strong downtrend (−4% per bar) — MR winner (biggest loser).
///
/// With K=1, Momentum selects AAUSDT; Reversion selects CCUSDT → guaranteed divergence.
fn build_3sym_trending_bars(n_bars: usize) -> Vec<Bar> {
    let mut bars_a = trending_bars("AAUSDT", n_bars, 1000.0, 0.05); // uptrend
    let mut bars_b = trending_bars("BBUSDT", n_bars, 500.0, 0.0); // flat
    let mut bars_c = trending_bars("CCUSDT", n_bars, 200.0, -0.04); // downtrend

    let mut all: Vec<Bar> = Vec::with_capacity(n_bars * 3);
    all.append(&mut bars_a);
    all.append(&mut bars_b);
    all.append(&mut bars_c);
    all.sort_by_key(|b| b.open_ts.inner().unix_timestamp_nanos());
    all
}

// ── Run one path → final equity ───────────────────────────────────────────────

fn run_to_final_equity(cfg: strategy::CrossSectionalMomentumConfig, bars: Vec<Bar>) -> Decimal {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("mr_test"));
    let input = TcnScenarioInput {
        scenario_name: "mr-divergence-test".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: dec!(100_000),
        slippage_bps: 2,
        taker_fee_bps: 4,
        config_id: "test_mr".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None,
        basis_override: None,
    };
    let result = pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in MR divergence test");
    result.final_equity
}

// ─────────────────────────────────────────────────────────────────────────────
// R-MR.1(a) — MR-vs-momentum divergence PASS
// ─────────────────────────────────────────────────────────────────────────────

/// R-MR.1(a) PASS: Momentum and Reversion at the same θ produce DIFFERENT equity curves.
///
/// Uses a 3-symbol universe with K=1:
/// - AAUSDT: strong uptrend → Momentum picks this (highest score).
/// - CCUSDT: strong downtrend → Reversion picks this (negated score highest).
///
/// The two strategies hold opposite symbols → equity curves diverge by ≥ 1 bp.
/// This proves the D-MR.1 inversion is NOT a no-op.
#[test]
fn r_mr_1a_momentum_vs_reversion_diverge() {
    // N_BARS must be large enough for warmup (lookback=5) + at least one rebalance.
    const N_BARS: usize = 50;
    const EPSILON_BPS: f64 = 0.0001; // 1 bp = 0.01% of initial_capital

    let bars = build_3sym_trending_bars(N_BARS);

    let cfg_mom = make_config_mr(Direction::Momentum);
    let cfg_rev = make_config_mr(Direction::Reversion);

    let eq_mom = run_to_final_equity(cfg_mom, bars.clone());
    let eq_rev = run_to_final_equity(cfg_rev, bars);

    let delta = (eq_mom - eq_rev).abs();
    let epsilon = Decimal::try_from(EPSILON_BPS * 100_000.0).unwrap_or(dec!(10));

    assert!(
        delta >= epsilon,
        "R-MR.1(a) FAIL: Momentum and Reversion equity curves are too similar. \
         |Δfinal_equity| = {delta} (must be ≥ {epsilon}). \
         If the inversion is a no-op, both strategies select identical symbols → identical curves.\n\
         eq_mom = {eq_mom}, eq_rev = {eq_rev}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R-MR.1(b) — same-config determinism / noise-floor control
// ─────────────────────────────────────────────────────────────────────────────

/// R-MR.1(b) CONTROL: same config + same path ⇒ byte-identical curves.
///
/// Review 1-16 truthfix: this test runs Momentum-vs-Momentum — two IDENTICAL
/// configs over the identical path — and asserts ZERO divergence. That makes
/// it a same-config determinism / noise-floor control, NOT a dropped-negation
/// detector: it cannot see whether the D-MR.1 negation exists, because no
/// Reversion config is involved. What it establishes is that the harness has
/// no noise floor — identical configs produce identical curves — so the ≥ 1 bp
/// divergence observed in R-MR.1(a) is attributable to the direction flip
/// alone.
///
/// The dropped-negation DETECTOR is (a): if the `Reversion => -score` negation
/// were dropped (the `Reversion => score` bug), Reversion would degenerate to
/// Momentum, both runs in (a) would produce identical curves, and (a)'s ≥ 1 bp
/// divergence assert would go RED.
#[test]
fn r_mr_1b_degenerate_noop_produces_identical_curves() {
    const N_BARS: usize = 50;
    const EPSILON_TINY: f64 = 1e-6; // sub-penny tolerance for "identical"

    let bars = build_3sym_trending_bars(N_BARS);

    // Both configs use Momentum — the same-config control pair (review 1-16
    // truthfix: this pair contains NO Reversion config, so it exercises the
    // noise floor, not the negation).
    let cfg_mom_1 = make_config_mr(Direction::Momentum);
    let cfg_mom_2 = make_config_mr(Direction::Momentum); // same config, not Reversion

    let eq_1 = run_to_final_equity(cfg_mom_1, bars.clone());
    let eq_2 = run_to_final_equity(cfg_mom_2, bars);

    let delta = (eq_1 - eq_2).abs();
    let epsilon = Decimal::try_from(EPSILON_TINY).unwrap_or(dec!(0.000001));

    // With same config + same path, curves MUST be byte-identical.
    assert!(
        delta < epsilon,
        "R-MR.1(b) control: same-config (both Momentum) curves MUST be identical. \
         Got |Δfinal_equity| = {delta} (must be < {epsilon}). \
         Same config + same path = byte-identical results.\n\
         eq_1 = {eq_1}, eq_2 = {eq_2}\n\n\
         What this control establishes (review 1-16 truthfix):\n\
         - The harness has NO noise floor: identical configs ⇒ identical curves,\n\
           so R-MR.1(a)'s ≥1bp divergence is attributable to the direction flip alone.\n\
         - This test CANNOT detect a dropped negation (no Reversion config runs here);\n\
           the dropped-negation detector is R-MR.1(a): with the negation dropped,\n\
           Reversion degenerates to Momentum and (a)'s divergence assert goes RED."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// FP-MR.3 — two-run byte-identity
// ─────────────────────────────────────────────────────────────────────────────

/// Run N paths of a config over trending bars → DistributionSummary.
fn run_cell_summary_mr(
    cfg: &strategy::CrossSectionalMomentumConfig,
    master_seed: u64,
    n: usize,
) -> DistributionSummary {
    let n_bars = 80; // enough bars past warmup (lookback=5) for multiple rebalances

    let mut metrics: Vec<PathMetrics> = Vec::with_capacity(n);

    for j in 0..n {
        let seed_j = path_seed(master_seed, j);
        // Build a fresh trending-bar set using the path seed for variety.
        // We use the same symbol structure but with a small random offset.
        let bars = {
            // Use seed-dependent offsets to create path variety while keeping structure.
            let offset = (seed_j % 5) as f64 * 0.01; // 0–4% offset
            let mut bars_a = trending_bars("AAUSDT", n_bars, 1000.0 + offset * 100.0, 0.05);
            let mut bars_b = trending_bars("BBUSDT", n_bars, 500.0, 0.001 * (seed_j % 3) as f64);
            let mut bars_c = trending_bars(
                "CCUSDT",
                n_bars,
                200.0 + offset * 20.0,
                -0.04 - offset * 0.001,
            );
            let mut all: Vec<Bar> = Vec::new();
            all.append(&mut bars_a);
            all.append(&mut bars_b);
            all.append(&mut bars_c);
            all.sort_by_key(|b| b.open_ts.inner().unix_timestamp_nanos());
            all
        };

        let strat = strategy::MomentumStrategy::from_config(cfg.clone(), SmolStr::new("fp_mr_3"));
        let input = TcnScenarioInput {
            scenario_name: format!("fp-mr-3-path-{j}"),
            start_year: 2023,
            bar_count: bars.len(),
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "test_mr".to_string(),
            forecaster_id: "test".to_string(),
            bars_override: Some(bars),
            emit_equity_bin: None,
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
            funding_override: None,
            basis_override: None,
        };

        let result = pollster::block_on(run_path(input, 0xC0FFEE, strat))
            .expect("run_path must succeed in FP-MR.3");

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

    DistributionSummary::from_path_metrics(&metrics).expect("build DistributionSummary in FP-MR.3")
}

/// FP-MR.3: Two runs of the MR cell with identical seeds → byte-identical formatted summaries.
///
/// Catches any unordered fold in the θ-loop or the MR reducer.
/// (The MR reducer reuses the C3 DistributionSummary::from_path_metrics path,
/// which is already proven deterministic by FP-C3.3. This test asserts the
/// MR-specific path also holds.)
#[test]
fn fp_mr_3_two_run_byte_identity() {
    const MASTER: u64 = 0xC0FFEE;
    const N: usize = 6; // small N — test speed; determinism is the property, not tail accuracy

    let cfg_mr = make_config_mr(Direction::Reversion);

    // Run 1.
    let s1 = run_cell_summary_mr(&cfg_mr, MASTER, N);
    // Run 2 — same seed, same config.
    let s2 = run_cell_summary_mr(&cfg_mr, MASTER, N);

    let fmt6 = |v: f64| format!("{v:.6}");

    assert_eq!(
        fmt6(s1.sharpe.p50),
        fmt6(s2.sharpe.p50),
        "FP-MR.3: MR cell Sharpe p50 must be byte-identical across two runs at the same seed"
    );
    assert_eq!(
        fmt6(s1.prob_loss),
        fmt6(s2.prob_loss),
        "FP-MR.3: MR cell prob_loss must be byte-identical across two runs at the same seed"
    );
    assert_eq!(
        fmt6(s1.max_dd_tail_p95),
        fmt6(s2.max_dd_tail_p95),
        "FP-MR.3: MR cell p95 MaxDD must be byte-identical across two runs at the same seed"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// FP-MR.5 — anti-cherry-pick through the REAL renderer
// ─────────────────────────────────────────────────────────────────────────────
// Review 1-16: the two fp_mr_5_* tests that lived here
// (`fp_mr_5_family_summary_always_valid_value`,
// `fp_mr_5_non_fragile_mr_cell_carries_c5_flag`) were DECORATIVE — the #66
// vacuous-test class: local re-implementations of the family-line / C5-flag
// literals that never invoked the renderer, so they could not go RED on a
// renderer regression. The REAL coverage is the shared production-seam gate in
// `crates/backtest/tests/param_sweep_e2e.rs`
// (`fp_c3_5_real_renderer_family_line_and_c5_flags`), which asserts the § R2.3
// family line + C5 flags on actual `render_surface_report` output — the MR arm
// rides the SAME renderer and the SAME `family_verdict_line` single source.
// The MR-SPECIFIC renderer surface (family labeling + single-source family
// line under direction=Reversion over the LOCKED MR grid) is asserted below.

/// Build a small finite DistributionSummary for renderer-under-test fixtures
/// (mirrors `param_sweep_e2e.rs::tiny_summary`).
fn tiny_summary(base: f64) -> DistributionSummary {
    let metrics: Vec<PathMetrics> = (0..3)
        .map(|i| {
            let x = base + f64::from(i) * 0.01;
            PathMetrics {
                sharpe: x,
                sortino: x,
                calmar: x.abs(),
                max_drawdown: 0.10,
                total_return: x / 10.0,
                final_equity: dec!(100_000) + Decimal::from(i),
                initial_equity: dec!(100_000),
            }
        })
        .collect();
    DistributionSummary::from_path_metrics(&metrics).expect("tiny fixture summary builds")
}

/// Mirror of `param_sweep_e2e.rs::tiny_cell_result` for the MR renderer gate.
fn tiny_cell_result(
    cell: backtest::sweep_harness::ThetaCell,
    base: f64,
    verdict: backtest::bakeoff::robustness::ParamRobustnessVerdict,
) -> backtest::sweep_harness::CellResult {
    backtest::sweep_harness::CellResult {
        cell,
        summary: tiny_summary(base),
        verdict,
        total_trades: 42,
        total_funding_harvested: Decimal::ZERO,
        total_time_in_market_bars: 0,
        total_bars_run: 0,
        total_liquidations: 0,
    }
}

/// Render a θ-surface through the REAL production renderer with
/// `direction = Reversion` (mirrors `param_sweep_e2e.rs::render_with_grid`,
/// Reversion arm).
fn render_reversion_with_grid(
    grid: &[backtest::sweep_harness::ThetaCell],
    cell_results: &[backtest::sweep_harness::CellResult],
) -> String {
    backtest::sweep_harness::render_surface_report(
        "2026-08-03T00:00:00Z",
        1.0,
        "testhost",
        1,
        "deadbeef",
        "test-revision-sha",
        "test-mr-scenario",
        0xC0FFEE,
        0xC0FFEE,
        3,
        "block-bootstrap-real",
        "stationary",
        "auto",
        Some(7),
        "test-source-sha",
        grid,
        cell_results,
        &tiny_summary(1.5),
        backtest::sweep_harness::SweepDirection::Reversion,
        backtest::sweep_harness::SweepScoreSource::VolAdjustedReturn,
        None,
        backtest::sweep_harness::SweepSelectionMode::CrossSectionalTopK,
        backtest::resample::Horizon::OneHour,
        4,
        2,
    )
}

/// FP-MR.5 (REAL renderer, review 1-16): a Reversion-arm render over the LOCKED
/// MR Tier-1 grid carries the MR family labeling AND the family line from the
/// production single source (`family_verdict_line`) — asserted on actual
/// `render_surface_report` output, not a local re-implementation.
#[test]
fn fp_mr_5_reversion_renderer_carries_mr_family_labeling() {
    use backtest::bakeoff::robustness::ParamRobustnessVerdict as V;
    use backtest::sweep_harness::{GridKind, family_verdict_line, grid_for_kind};

    let grid = grid_for_kind(GridKind::MrTier1);
    // All-FRAGILE MR surface (the anchored #87 shape).
    let results: Vec<_> = grid
        .iter()
        .map(|c| tiny_cell_result(*c, -0.02, V::Fragile))
        .collect();

    let body = render_reversion_with_grid(grid, &results);

    // MR family labeling: slug, heading label, MR grid header, held_constant direction.
    assert!(
        body.contains("slug: cross-sectional-mean-reversion-strategy"),
        "Reversion render must carry the MR slug"
    );
    assert!(
        body.contains("# Mean-Reversion (MR) θ-Surface"),
        "Reversion render must carry the MR family heading"
    );
    assert!(
        body.contains("## MR θ-grid definition (6-cell, 2026-05-31 LOCKED § D-MR.2-LOCKED"),
        "Reversion render must carry the LOCKED MR grid header"
    );
    assert!(
        body.contains("direction=reversion"),
        "Reversion render must carry direction=reversion in the held_constant line"
    );
    // R-MR.3 turnover legibility: the MR-only trades column is rendered.
    assert!(
        body.contains("Trades = total trade count across all N paths"),
        "Reversion render must carry the MR trades column gloss (R-MR.3)"
    );
    // The family line comes from the SINGLE production source (review 1-15 L6).
    assert!(
        body.contains(family_verdict_line(false)),
        "all-FRAGILE MR surface must render the family line from the single source"
    );
    assert!(
        body.contains("Conclusion: v1 cross-sectional mean-reversion is structurally fragile"),
        "all-FRAGILE MR surface must carry the MR conclusion text"
    );
    // Negative control: NOT the momentum family labeling.
    assert!(
        !body.contains("slug: momentum-parameter-robustness-sweep"),
        "Reversion render must NOT carry the momentum slug"
    );
    assert!(
        !body.contains(family_verdict_line(true)),
        "all-FRAGILE surface must NOT carry the non-uniform family line"
    );
}
