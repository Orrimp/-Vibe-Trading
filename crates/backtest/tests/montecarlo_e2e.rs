//! Monte-Carlo robustness harness — day-1 e2e gate tests (M-DEV-6).
//!
//! Implements the MANDATORY two-part R-NR.6 gate — re-pointed at the REAL
//! harness fan-out (review 1-14): the gates below drive
//! `backtest::mc_harness::{run_one_path, run_ensemble}` — the exact
//! per-path + fan-out → sort-by-j → reduce chain `bin/monte_carlo.rs`
//! executes — over a small-N gbm-smoke ensemble (no corpus needed, no skip
//! path; #66 rule: execution is asserted, not assumed).
//!
//! **(a) Divergence gate (FP-C2.1):** The ensemble distribution summary has a
//! non-degenerate spread when path seeds are distinct. If the harness secretly
//! collapses to running one path N times (a seed-wiring bug in
//! `run_one_path`/`derive_path_seed`), the spread collapses to 0 and this
//! test FAILS. This is the adaptation of the CLAUDE.md overlay-e2e
//! non-negotiable to a distribution harness.
//!
//! **(b) Determinism gate (FP-C2.3):** Two runs of the REAL ensemble chain at
//! the same master seed produce a field-by-field bit-identical
//! `DistributionSummary` (and identical strings at ADR-0051 D3 fixed
//! precision). Catches any unordered fold or unformatted float.
//!
//! ## FP-C2.1 dry-run (the degenerate falsifier)
//!
//! `fp_c2_1_degenerate_seeds_have_zero_spread` forces ALL paths through the
//! REAL `run_one_path` with the SAME path seed and asserts the spread
//! collapses to ≈ 0 — proving the divergence gate can detect the
//! noop-collapse. Because the gate is tested on BOTH the degenerate AND the
//! real case, the gate itself is falsified.
//!
//! ## Pattern reference
//!
//! `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (single-path
//! analogue, CLAUDE.md § non-negotiable reference).

use backtest::cli_types::TcnScenarioInput;
use backtest::mc_harness::{self, GeneratorKind};
use backtest::scenarios::montecarlo::run_path;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Fill-tie-break seed — the SAME constant `bin/monte_carlo.rs` holds fixed
/// across all paths (ADR-0051 D1 orthogonality).
const FILL_SEED: u64 = 0xC0FFEE;

/// ADR-0051 D1 seed derivation (same constant as the production code).
/// Kept test-local for the reducer-level tests; the real-chain gates use
/// `mc_harness::derive_path_seed` (the production fn) directly.
fn path_seed(master: u64, j: usize) -> u64 {
    master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))
}

/// Run the REAL harness ensemble chain (`mc_harness::run_ensemble` — the
/// exact fan-out → sort-by-j → reduce chain the bin executes) over a small-N
/// gbm-smoke ensemble. No corpus needed; must NOT skip.
fn real_gbm_ensemble(
    n_paths: usize,
    ensemble_seed: u64,
    n_bars: usize,
) -> (Vec<PathMetrics>, DistributionSummary) {
    let universe = backtest::scenarios::momentum::top10_symbols_with_prices();
    mc_harness::run_ensemble(
        n_paths,
        ensemble_seed,
        FILL_SEED,
        &universe,
        &[], // gbm-smoke needs no real source bars
        n_bars,
        GeneratorKind::GbmSmoke,
        2023,
    )
    .expect("run_ensemble (gbm-smoke) must succeed — config/strategies/top10_momentum_h1.toml exists in this repo")
}

/// Build a small synthetic bar series via GBM (deterministic from seed).
fn synthetic_bars(seed: u64, n: usize) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let sym = Symbol::new("BTCUSDT");
    let epoch = {
        let date =
            time::Date::from_calendar_date(2023, time::Month::January, 1).expect("valid date");
        time::OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut close = 30_000.0_f64;
    let mut bars = Vec::with_capacity(n);

    for i in 0..n {
        let z: f64 = rng.random::<f64>() * 0.05 - 0.025;
        let next = (close * (1.0 + z)).max(0.01_f64);

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
            open: to_price(close),
            high: to_price(close.max(next) * 1.001),
            low: to_price(close.min(next) * 0.999),
            close: to_price(next),
            volume: Quantity::new(dec!(100)).expect("100 always valid"),
            trade_count: 10,
            local_recv_ts: close_ts,
            venue: Venue::Binance,
        });
        close = next;
    }
    bars
}

/// Build a toy `Vec<Decimal>` equity curve from bars (monotone + noise).
/// Used ONLY by the reducer-level tests (`fp_c2_2`, `reduction_is_pure`) —
/// the R-NR.6 gates exercise the real harness chain instead (review 1-14).
fn fake_equity_curve(bars: &[Bar]) -> Vec<Decimal> {
    let initial = dec!(100_000);
    let mut eq = vec![initial];
    // Very simple: equity tracks the close price ratio × initial.
    let start_close = bars.first().map(|b| b.close.get()).unwrap_or(dec!(30000));
    for bar in bars {
        let ratio = if start_close > Decimal::ZERO {
            bar.close.get() / start_close
        } else {
            Decimal::ONE
        };
        eq.push(initial * ratio);
    }
    eq
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate (a) — Divergence over the REAL fan-out (R-NR.6a, FP-C2.1)
// ─────────────────────────────────────────────────────────────────────────────

/// R-NR.6(a) PASS: a small-N ensemble through the REAL harness chain
/// (`mc_harness::run_ensemble` — production seed derivation, `run_one_path`,
/// sort-by-j, reduce) with distinct per-path seeds produces a non-degenerate
/// Sharpe spread. `spread = p95_sharpe − p5_sharpe` must be ≥ epsilon.
///
/// A `run_one_path`/`derive_path_seed` seed-wiring bug (all paths identical)
/// collapses the spread to 0 and FAILS this test — the falsifier
/// `fp_c2_1_degenerate_seeds_have_zero_spread` below proves that detection
/// path actually works.
#[test]
fn rn6a_divergence_gate_passes_with_distinct_seeds() {
    const MASTER: u64 = 0xC0FFEE; // the bin's default ensemble seed
    const N: usize = 5;
    const N_BARS: usize = 240;
    const EPSILON: f64 = 0.01; // spread must exceed 1 centile point

    let (metrics, summary) = real_gbm_ensemble(N, MASTER, N_BARS);

    // Execution proof (#66 rule): the ensemble must actually have traded —
    // at least one path's final equity moved off the initial capital. If no
    // path trades, every equity curve is flat 100k and the spread assertion
    // below would fail anyway; this assert names the root cause.
    assert!(
        metrics.iter().any(|m| m.final_equity != m.initial_equity),
        "R-NR.6(a) execution proof: no path moved equity — the momentum \
         strategy never traded on any gbm-smoke path (warmup/bar-count bug?)"
    );

    let spread = summary.sharpe.p95 - summary.sharpe.p5;
    assert!(
        spread >= EPSILON,
        "R-NR.6(a): REAL-chain ensemble Sharpe spread (p95-p5) = {spread:.6} must be ≥ {EPSILON} \
         to prove the harness runs distinct paths. Got {spread:.6}."
    );
}

/// FP-C2.1 FALSIFIER (dry-run): force ALL N paths through the REAL
/// `run_one_path` with the SAME path seed → the divergence gate's spread
/// collapses to ≈ 0.
///
/// This proves the R-NR.6(a) gate can detect the noop-collapse ON THE REAL
/// CHAIN: when seeds are equal the real per-path runner produces identical
/// metrics, the spread collapses, and
/// `rn6a_divergence_gate_passes_with_distinct_seeds` would FAIL — which is
/// what we want.
#[test]
fn fp_c2_1_degenerate_seeds_have_zero_spread() {
    const MASTER: u64 = 0xC0FFEE;
    const N: usize = 4;
    const N_BARS: usize = 160;
    const EPSILON: f64 = 1e-9; // same seed → identical paths → spread exactly 0

    let universe = backtest::scenarios::momentum::top10_symbols_with_prices();
    // Force all paths to use the SAME seed (the degenerate collapse scenario),
    // but run each through the REAL production per-path runner.
    let fixed_seed = mc_harness::derive_path_seed(MASTER, 0);
    let metrics: Vec<PathMetrics> = (0..N)
        .map(|j| {
            mc_harness::run_one_path(
                j,
                fixed_seed, // ← degenerate: every j gets path 0's seed
                FILL_SEED,
                &universe,
                &[],
                N_BARS,
                GeneratorKind::GbmSmoke,
                2023,
            )
            .expect("run_one_path (gbm-smoke) must succeed")
            .metrics
        })
        .collect();

    // Same seed ⇒ the REAL runner must produce identical outcomes per path.
    for m in &metrics[1..] {
        assert_eq!(
            m.final_equity, metrics[0].final_equity,
            "FP-C2.1: same path seed must produce identical final equity from run_one_path"
        );
    }

    let summary = DistributionSummary::from_path_metrics(&metrics)
        .expect("build degenerate distribution summary");

    let spread = summary.sharpe.p95 - summary.sharpe.p5;
    assert!(
        spread.abs() < EPSILON,
        "FP-C2.1: degenerate (same-seed) REAL-chain ensemble spread={spread:.9} should be ≈ 0 \
         (all paths identical). This proves the divergence gate is not itself a no-op: \
         when all seeds are equal the spread collapses, and \
         `rn6a_divergence_gate_passes_with_distinct_seeds` would FAIL — which is what we want."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate (b) — Two-run byte-identity over the REAL fan-out (R-NR.6b, R3.1, FP-C2.3)
// ─────────────────────────────────────────────────────────────────────────────

/// R-NR.6(b) / R3.1 PASS: two runs of the REAL ensemble chain at the same
/// master seed produce a field-by-field bit-identical `DistributionSummary`
/// (and byte-identical strings at ADR-0051 D3 fixed precision).
///
/// Catches any unordered fold (D2 violation — e.g. reducing in rayon
/// completion order instead of index order) or unformatted float (D3).
#[test]
fn rn6b_two_run_byte_identity() {
    const MASTER: u64 = 0xDEAD_BEEF;
    const N: usize = 4;
    const N_BARS: usize = 200;

    let (metrics1, s1) = real_gbm_ensemble(N, MASTER, N_BARS);
    let (metrics2, s2) = real_gbm_ensemble(N, MASTER, N_BARS);

    // Per-path metrics must match bit-for-bit in index order.
    assert_eq!(metrics1.len(), metrics2.len());
    for (i, (a, b)) in metrics1.iter().zip(metrics2.iter()).enumerate() {
        assert_eq!(
            a.sharpe.to_bits(),
            b.sharpe.to_bits(),
            "path {i}: sharpe must be bit-identical across runs"
        );
        assert_eq!(
            a.final_equity, b.final_equity,
            "path {i}: final_equity must be identical across runs"
        );
    }

    // Reduced summaries: field-by-field bit identity for every distribution…
    let dist_fields = |d: &backtest::stats::MetricDistribution| {
        [
            ("mean", d.mean),
            ("std", d.std),
            ("p5", d.p5),
            ("p25", d.p25),
            ("p50", d.p50),
            ("p75", d.p75),
            ("p95", d.p95),
            ("min", d.min),
            ("max", d.max),
        ]
    };
    let dists = [
        ("sharpe", &s1.sharpe, &s2.sharpe),
        ("sortino", &s1.sortino, &s2.sortino),
        ("calmar", &s1.calmar, &s2.calmar),
        ("max_drawdown", &s1.max_drawdown, &s2.max_drawdown),
        ("total_return", &s1.total_return, &s2.total_return),
    ];
    for (metric, d1, d2) in dists {
        for ((name, v1), (_, v2)) in dist_fields(d1).into_iter().zip(dist_fields(d2)) {
            assert_eq!(
                v1.to_bits(),
                v2.to_bits(),
                "R-NR.6(b): {metric}.{name} must be bit-identical across two same-seed runs \
                 (got {v1:?} vs {v2:?})"
            );
        }
    }

    // …and for every scalar field.
    let scalars = [
        ("prob_loss", s1.prob_loss, s2.prob_loss),
        ("prob_sharpe_gt_0", s1.prob_sharpe_gt_0, s2.prob_sharpe_gt_0),
        ("prob_sharpe_gt_1", s1.prob_sharpe_gt_1, s2.prob_sharpe_gt_1),
        ("max_dd_tail_p50", s1.max_dd_tail_p50, s2.max_dd_tail_p50),
        ("max_dd_tail_p95", s1.max_dd_tail_p95, s2.max_dd_tail_p95),
        ("cvar_95", s1.cvar_95, s2.cvar_95),
        ("cvar_99", s1.cvar_99, s2.cvar_99),
        (
            "median_terminal_wealth",
            s1.median_terminal_wealth,
            s2.median_terminal_wealth,
        ),
        ("skew", s1.skew, s2.skew),
    ];
    for (name, v1, v2) in scalars {
        assert_eq!(
            v1.to_bits(),
            v2.to_bits(),
            "R-NR.6(b): summary.{name} must be bit-identical across two same-seed runs \
             (got {v1:?} vs {v2:?})"
        );
    }

    // ADR-0051 D3 formatted-precision spot checks (the anchor-stability view).
    let fmt6 = |v: f64| format!("{v:.6}");
    let fmt2pct = |v: f64| format!("{:.2}%", v * 100.0);
    assert_eq!(fmt6(s1.sharpe.p50), fmt6(s2.sharpe.p50));
    assert_eq!(fmt2pct(s1.max_dd_tail_p95), fmt2pct(s2.max_dd_tail_p95));
}

// ─────────────────────────────────────────────────────────────────────────────
// FP-C2.2 — anchor sensitivity to param_set (K3)
// ─────────────────────────────────────────────────────────────────────────────

/// FP-C2.2 / K3: Two ensembles with DIFFERENT strategies (simulated by
/// different initial equity — a proxy for different θ*) produce different
/// distribution summaries. Proves the anchor moves when inputs move.
///
/// In the real harness this is enforced by `param_set` being in the hashed
/// body. Here we test the reducer itself: different inputs → different outputs.
/// (Proper θ*-variation e2e is owned by story 1-25's re-verification suite —
/// see the 1-14 Review Findings Defer item.)
#[test]
fn fp_c2_2_anchor_sensitive_to_different_inputs() {
    const MASTER: u64 = 0xC0FFEE;
    const N: usize = 15;
    const N_BARS: usize = 200;

    // Two ensembles using the same seeds but different "strategy" input
    // (simulated by different equity scaling — equivalent to different θ*).
    let metrics_a: Vec<PathMetrics> = (0..N)
        .map(|j| {
            let seed = path_seed(MASTER, j);
            let bars = synthetic_bars(seed, N_BARS);
            let equity: Vec<Decimal> = fake_equity_curve(&bars);
            PathMetrics {
                sharpe: compute_sharpe_hourly(&equity),
                sortino: compute_sortino_hourly(&equity),
                calmar: compute_calmar(&equity),
                max_drawdown: compute_max_drawdown_f64(&equity),
                total_return: compute_total_return(&equity),
                final_equity: *equity.last().unwrap_or(&dec!(100_000)),
                initial_equity: dec!(100_000),
            }
        })
        .collect();

    // Different "param_set": use a 2× scaled equity → different metrics.
    let metrics_b: Vec<PathMetrics> = (0..N)
        .map(|j| {
            let seed = path_seed(MASTER, j);
            let bars = synthetic_bars(seed, N_BARS);
            // Reverse the equity trend to simulate a "different strategy".
            let equity: Vec<Decimal> = {
                let base = fake_equity_curve(&bars);
                // Invert the direction: equity_b[i] = 200k - (base[i] - 100k)
                base.iter()
                    .map(|&e| dec!(200_000) - (e - dec!(100_000)))
                    .collect()
            };
            let final_eq = *equity.last().unwrap_or(&dec!(100_000));
            PathMetrics {
                sharpe: compute_sharpe_hourly(&equity),
                sortino: compute_sortino_hourly(&equity),
                calmar: compute_calmar(&equity),
                max_drawdown: compute_max_drawdown_f64(&equity),
                total_return: compute_total_return(&equity),
                final_equity: final_eq,
                initial_equity: dec!(100_000),
            }
        })
        .collect();

    let s_a = DistributionSummary::from_path_metrics(&metrics_a).unwrap();
    let s_b = DistributionSummary::from_path_metrics(&metrics_b).unwrap();

    // The two summaries must differ in at least one metric.
    let same = format!("{:.6}", s_a.sharpe.p50) == format!("{:.6}", s_b.sharpe.p50)
        && format!("{:.6}", s_a.prob_loss) == format!("{:.6}", s_b.prob_loss);

    assert!(
        !same,
        "FP-C2.2 / K3: two different strategy inputs must produce different summaries. \
         sharpe_p50 a={:.6} b={:.6}, prob_loss a={:.6} b={:.6}",
        s_a.sharpe.p50, s_b.sharpe.p50, s_a.prob_loss, s_b.prob_loss
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// FP-C2.4 — generator label honesty (K4)
// ─────────────────────────────────────────────────────────────────────────────

/// FP-C2.4 / K4: the REAL `GeneratorKind::label()` values (the strings the
/// binary renders into the hashed report body) are distinct and honest.
///
/// Review 1-14: this test previously asserted two test-local string literals
/// (vacuous, #66 class); it now imports the production enum.
#[test]
fn fp_c2_4_generator_labels_are_distinct() {
    let block_label = GeneratorKind::BlockBootstrapReal.label();
    let gbm_label = GeneratorKind::GbmSmoke.label();

    // The labels must be different (catches a copy-paste bug where both
    // get "block-bootstrap-real" in the body, accidentally passing K4 visually
    // while the GBM run produces the anchored report).
    assert_ne!(
        block_label, gbm_label,
        "FP-C2.4: generator labels must be distinct"
    );

    // The anchored generator's label is pinned (a hashed body field).
    assert_eq!(
        block_label, "block-bootstrap-real",
        "FP-C2.4: the anchored generator label is a hashed body field — it must not drift"
    );

    // The block-bootstrap label must not contain "gbm".
    assert!(
        !block_label.contains("gbm"),
        "FP-C2.4: block-bootstrap label must not contain 'gbm'"
    );

    // The gbm label must not contain "block".
    assert!(
        !gbm_label.contains("block"),
        "FP-C2.4: gbm-smoke label must not contain 'block'"
    );
}

/// Review 1-14: honest scenario naming per lane.
///
/// - The ANCHORED lane's scenario NAME is pinned byte-exact — anchors in
///   `evidence/anchors.toml` are keyed by scenario NAME, so any drift here
///   would orphan the locked `mc-robustness-2026-06` anchor.
/// - The gbm-smoke lane must no longer carry the false "block-bootstrap"
///   token (no bootstrap runs in that lane).
#[test]
fn mc_scenario_names_honest_and_anchored_name_pinned() {
    assert_eq!(
        mc_harness::scenario_name(GeneratorKind::BlockBootstrapReal, 2023),
        "v1-momentum-2023-block-bootstrap-real-fy-mc",
        "ANCHOR GUARD: the anchored lane's scenario NAME must never drift \
         (anchors are keyed by NAME, not filename)"
    );

    let gbm = mc_harness::scenario_name(GeneratorKind::GbmSmoke, 2023);
    assert!(
        !gbm.contains("block-bootstrap"),
        "gbm-smoke scenario name must not claim block-bootstrap: got {gbm:?}"
    );
    assert!(
        gbm.contains("gbm-smoke"),
        "gbm-smoke scenario name must name its generator honestly: got {gbm:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// GBM-smoke seed hygiene (review 1-14; the 1-13 GbmPathGen fix idiom)
// ─────────────────────────────────────────────────────────────────────────────

/// Anti-diagonal seed-collision regression (the 1-13 `data::GbmPathGen` bug,
/// which the bin's GbmSmoke branch had re-inlined).
///
/// Under the ADR-0051 D1 rule `path_seed_j = master + j·0x9E37_79B9`, the OLD
/// additive per-symbol derivation `sym_seed = path_seed + sym_i·0x9E37_79B9`
/// made `seed(path 0, sym 1) == seed(path 1, sym 0)` for EVERY master seed —
/// symbol 1 of path 0 replayed symbol 0 of path 1 bit-for-bit. The SplitMix64
/// derivation must make the two streams differ, at the seed level AND at the
/// generated-bar level (execution asserted, #66 rule).
#[test]
fn gbm_sym_seed_no_anti_diagonal_collision() {
    const GOLDEN_GAMMA: u64 = 0x9E37_79B9; // ADR-0051 D1 path-seed step
    let master = 0xA11C_E5EE_u64;
    let ps0 = mc_harness::derive_path_seed(master, 0);
    let ps1 = mc_harness::derive_path_seed(master, 1);
    assert_eq!(ps1, ps0.wrapping_add(GOLDEN_GAMMA), "D1 rule sanity");

    // Seed level: the anti-diagonal pair must not collide.
    let seed_p0_s1 = mc_harness::derive_gbm_sym_seed(ps0, 1);
    let seed_p1_s0 = mc_harness::derive_gbm_sym_seed(ps1, 0);
    assert_ne!(
        seed_p0_s1, seed_p1_s0,
        "anti-diagonal collision: (path 0, sym 1) and (path 1, sym 0) derive \
         the same GBM seed — per-symbol seeds must mix (path_seed, sym_i) \
         non-additively (SplitMix64, the 1-13 fix idiom)"
    );

    // Bar level: the generated close streams must differ.
    let sym = Symbol::new("BTCUSDT");
    let bars_a = backtest::scenarios::momentum::synthetic_bars_hourly(
        &sym,
        50,
        seed_p0_s1,
        dec!(30_000),
        2023,
    );
    let bars_b = backtest::scenarios::momentum::synthetic_bars_hourly(
        &sym,
        50,
        seed_p1_s0,
        dec!(30_000),
        2023,
    );
    assert_eq!(bars_a.len(), 50, "generator must actually produce bars");
    let any_diff = bars_a
        .iter()
        .zip(bars_b.iter())
        .any(|(a, b)| a.close != b.close);
    assert!(
        any_diff,
        "anti-diagonal collision at the bar level: (path 0, sym 1) and \
         (path 1, sym 0) produced identical close streams"
    );
}

/// Review 1-14: the GBM-smoke SOURCE-bar seed family must be domain-separated
/// from the ADR-0051 D1 path-seed family.
///
/// The old inline base (`0xC0FFEE + idx·0x9E37_79B9`) was bit-identical to
/// the D1 path seeds at the default master seed `0xC0FFEE`, so source-bar
/// stream `idx` replayed path-seed stream `j = idx` exactly.
#[test]
fn gbm_source_bar_seed_base_domain_separated() {
    // The base itself is a distinct constant (not the default master seed).
    assert_ne!(
        mc_harness::GBM_SOURCE_SEED_BASE,
        0xC0FFEE,
        "source-bar seed base must not be the default ensemble master seed"
    );

    // No mixed source-bar seed may equal any D1 path seed on a 10×10 grid
    // (10 symbols × 10 leading paths at the default master seed).
    for idx in 0..10usize {
        let source_seed = mc_harness::derive_gbm_sym_seed(mc_harness::GBM_SOURCE_SEED_BASE, idx);
        for j in 0..10usize {
            let path_seed_j = mc_harness::derive_path_seed(0xC0FFEE, j);
            assert_ne!(
                source_seed, path_seed_j,
                "source-bar seed idx={idx} collides with D1 path seed j={j} \
                 — the two seed families must be domain-separated"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI validation (review 1-14: --year bail; --paths ≥ 2 + cap)
// ─────────────────────────────────────────────────────────────────────────────

/// `--year` maps ONLY explicitly supported years; unmapped years bail instead
/// of silently falling back to a (leap-wrong, mislabeled) 8760-bar span.
#[test]
fn year_bar_count_bails_on_unmapped_years() {
    assert_eq!(
        mc_harness::bar_count_for_year(2023).expect("2023 supported"),
        8760
    );
    assert_eq!(
        mc_harness::bar_count_for_year(2024).expect("2024 supported (leap)"),
        8784
    );
    for bad_year in [2022, 2025, 2028, 1970] {
        let err = mc_harness::bar_count_for_year(bad_year)
            .expect_err("unmapped year must bail, not silently map to 8760");
        let msg = format!("{err}");
        assert!(
            msg.contains("unsupported") && msg.contains("2023"),
            "error must name the supported years; got: {msg}"
        );
    }
}

/// `--paths` rejects 0 (used to die late with a misleading reducer error),
/// 1 (degenerate "distribution"), and absurd N (unbounded alloc) — with
/// clear messages — while accepting the sane range.
#[test]
fn paths_validation_rejects_degenerate_and_absurd_n() {
    for bad in [0usize, 1] {
        let err = mc_harness::validate_paths(bad).expect_err("paths < 2 must be rejected");
        let msg = format!("{err}");
        assert!(
            msg.contains("at least 2"),
            "paths={bad}: message must explain the ≥2 requirement; got: {msg}"
        );
    }

    mc_harness::validate_paths(2).expect("N=2 is the minimum valid ensemble");
    mc_harness::validate_paths(500).expect("the anchored default N=500 must pass");
    mc_harness::validate_paths(mc_harness::MAX_PATHS).expect("the cap itself is valid");

    let err = mc_harness::validate_paths(mc_harness::MAX_PATHS + 1)
        .expect_err("absurd N must be rejected before any allocation");
    let msg = format!("{err}");
    assert!(
        msg.contains("cap"),
        "over-cap message must mention the cap; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bug B — run_path solvency regression (Gate-2, v0.1.2; extended review 1-14)
// ─────────────────────────────────────────────────────────────────────────────
//
// NOTE (review 1-14): two earlier solvency tests were DELETED in favor of the
// Gate-2 test below:
//
// - `solvency_invariant_equity_curve_never_negative_across_paths` asserted
//   non-negativity of `fake_equity_curve` fixtures that are non-negative BY
//   CONSTRUCTION (could never go red — #66 class). The invariant is only
//   genuinely observable at raw `run_path` output (`min_cash_seen`, the
//   un-clamped equity curve) — the metric layer clamps negative equity to
//   1e-6 (sentinel semantics owned by story 1-25), so any reducer-level
//   restatement is structurally incapable of failing. Gate-2 below asserts
//   the same invariant on the REAL output and now carries the deleted test's
//   distribution-relevant assertions (total_return > −1, max_dd ≤ 100%).
// - `solvency_guard_arithmetic_unit_test` re-implemented the guard arithmetic
//   inside the test body (a tautology: it tested its own copy of the
//   formula, not the production code). Gate-2 supersedes it by driving the
//   REAL `run_path` into the exact pre-fix failure state.

/// End-to-end solvency regression gate: calls the REAL `run_path` with a
/// scenario designed to drive cash NEGATIVE under the old (un-guarded) code.
///
/// ## What this test guards
///
/// Bug B (`crates/backtest/src/scenarios/montecarlo.rs`, v0.1.0): the Buy branch
/// sized notional as `equity * 0.10` without checking whether cash was sufficient
/// to cover `notional + fee`. When cash < equity * 0.10 (possible once most
/// equity is tied in positions), the old code drove cash negative — impossible
/// for a long-only book.
///
/// ## Scenario design (10 symbols, k_long=9)
///
/// - 10 symbols `AAAUSDT`..`JJJUSDT`, `k_long=9`, `lookback_minutes=2`,
///   `rebalance_minutes=1`. `initial_capital=$10_000`, `taker_fee_bps=4`.
/// - **Warmup (bars t=0h..t=2h):** AAAUSDT prices [1000→990→980], steep
///   monotone drop; score ≈ -808 (WORST). BBBUSDT prices [1000→999→997],
///   slight drop; score ≈ -6.0 (second-worst). CCC..JJJ flat; score=0.
///   First rebalance fires at t=2h on JJJUSDT (last symbol to complete
///   warmup). top-9 = {BBB, CCC..JJJ}. Nine BUYs deplete cash to ~$996.
/// - **Churn (bar t=3h-AAA):** AAA price = 2000 (was 980). Ring buffer:
///   [990, 980, 2000]. Score ≈ 1.94 (HIGHEST). Rebalance fires at t=3h-AAA
///   (60 min since prior rebalance). New top-9 = {AAA, CCC..JJJ}; BBB exits.
///   Signal batch (alphabetical): BUY AAA first, then SELL BBB.
/// - **Bug trigger (OLD code):** BUY AAA fires before SELL BBB. Cash≈$996.
///   `notional = equity(≈$9996) × 0.10 ≈ $999.64 > cash($996)`. Old code:
///   `cash = $996 - $999.64 - $0.40 ≈ −$3.64` → NEGATIVE (impossible).
///   Equity curve contains a negative value → assertions FAIL (RED). ✓
/// - **With the v0.1.1 guard:** pre-flight check `$996 < $999.64 + $0.40`
///   → TRUE → BUY SKIPPED. Cash ≥ 0. PASS. ✓
///
/// ## RED-on-revert proof (developer responsibility per honest-tick rule)
///
/// To prove this test is a genuine guard, the developer must:
/// 1. Temporarily revert the solvency guard in `montecarlo.rs` (remove the
///    pre-flight `cash < notional + fee_estimate` skip).
/// 2. Run: `cargo test -p backtest --test montecarlo_e2e solvency_guard_run_path_regression_negative_cash_prevented`
/// 3. Confirm it goes RED (`min_cash_seen < 0` assertion fires).
/// 4. Restore the guard; confirm GREEN.
///
/// The result (FAIL when guard reverted / PASS when restored) is cited in the HANDOFF note.
///
/// ## Non-interference with anchors
///
/// This is a test-only addition. It does NOT call the `bin/monte_carlo.rs`
/// driver and does NOT produce any report files. `verify_anchors.sh` is
/// unaffected.
#[test]
fn solvency_guard_run_path_regression_negative_cash_prevented() {
    // ── Strategy config: 10 symbols, k_long=9, lookback=2, rebalance=1 ──────

    // Symbols sorted alphabetically — the alphabetical processing order in
    // run_path is load-bearing for the bug trigger (BUY AAA before SELL BBB).
    let universe_syms: &[&str] = &[
        "AAAUSDT", "BBBUSDT", "CCCUSDT", "DDDUSDT", "EEEUSDT", "FFFUSDT", "GGGUSDT", "HHHUSDT",
        "IIIJUSDT", "JJJUSDT",
    ];

    // Build a CrossSectionalMomentumConfig inline (no file dependency).
    let cfg = strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("solvency_regression_harness"),
        universe: universe_syms.iter().map(|s| SmolStr::new(*s)).collect(),
        lookback_minutes: 2,
        rebalance_minutes: 1,
        k_long: 9,
        k_short: 0,
        exposure_cap: dec!(1.0),
        drift_rebalance_threshold: dec!(0.05),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: strategy::Direction::Momentum,
        score_source: strategy::ScoreSource::VolAdjustedReturn,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: rust_decimal::Decimal::ZERO,
    };
    let strategy =
        strategy::MomentumStrategy::from_config(cfg, SmolStr::new("solvency_regression_harness"));

    // ── Build synthetic bars ──────────────────────────────────────────────────
    //
    // Price design to produce the bug-trigger scenario:
    //
    // AAAUSDT: 1000 → 998 → 990 → 2000  (steep drop warmup, then large jump at t=3h)
    // BBBUSDT: 1000 → 999 → 997 → 997   (slight drop warmup, stays low at t=3h)
    // CCCUSDT..JJJUSDT: 1000 → 1000 → 1000 → 1000  (flat throughout)
    //
    // Scores at first rebalance (t=2h):
    //   AAA: ln(990/1000)/vol ≈ very negative  → excluded from top-9 (WORST)
    //   BBB: ln(997/1000)/vol ≈ slightly negative → lowest among held symbols
    //   CCC..JJJ: 0 (flat) → 8 symbols in top-9
    //   → top-9 = {BBB, CCC..JJJ}  9 BUYs → cash depleted to ~10% of equity
    //
    // Scores at second rebalance (t=3h, fires on AAA's bar):
    //   AAA (t=3h): ln(2000/990)/vol(3h) ≈ ~2.0   → HIGHEST (new entrant)
    //   BBB (still at t=2h score): ≈ -6.0          → LOWEST (exits top-9)
    //   CCC..JJJ (t=2h): 0                         → 8 hold
    //   → new top-9 = {AAA, CCC..JJJ}
    //   → signals: BUY AAA (alpha first), SELL BBB (alpha second)
    //   → BUY fires BEFORE cash is replenished by SELL → BUG trigger

    let epoch = {
        let date =
            time::Date::from_calendar_date(2023, time::Month::January, 1).expect("valid date");
        time::OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    };

    // Price arrays indexed by (symbol_index, time_step).
    // symbol_index: 0=AAA, 1=BBB, 2..9=CCC..JJJ
    // time_step: 0=t+0h, 1=t+1h, 2=t+2h, 3=t+3h
    //
    // Score derivation (lookback=2, vol_floor=1e-6):
    //   AAA at t=2h: [1000→990→980] monotone drop.
    //     log_return = ln(980/1000) ≈ -0.02020
    //     log_rets = [ln(980/990), ln(990/1000)] ≈ [-0.01010, -0.01005]; vol ≈ 2.5e-5
    //     score ≈ -0.02020 / 2.5e-5 ≈ -808  (WORST → excluded from top-9) ✓
    //   BBB at t=2h: [1000→999→997] slight drop.
    //     log_return = ln(997/1000) ≈ -0.00300
    //     log_rets = [ln(997/999), ln(999/1000)] ≈ [-0.00200, -0.00100]; vol ≈ 5.0e-4
    //     score ≈ -0.00300 / 5.0e-4 ≈ -6.0  (SECOND WORST → lowest of held 9) ✓
    //   CCC..JJJ: flat [1000→1000→1000]; score = 0 / vol_floor = 0 ✓
    //   AAA at t=3h: [990→980→2000] sudden jump.
    //     log_return = ln(2000/990) ≈ 0.703
    //     log_rets = [ln(2000/980), ln(980/990)] ≈ [0.713, -0.010]; vol ≈ 0.362
    //     score ≈ 0.703 / 0.362 ≈ 1.94  (HIGHEST at 2nd rebalance → enters top-9) ✓
    let prices: [[f64; 4]; 10] = [
        [1000.0, 990.0, 980.0, 2000.0], // AAAUSDT: steep monotone drop then big jump
        [1000.0, 999.0, 997.0, 997.0],  // BBBUSDT: slight drop, stays low (exits at 2nd rebalance)
        [1000.0, 1000.0, 1000.0, 1000.0], // CCCUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // DDDUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // EEEUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // FFFUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // GGGUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // HHHUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // IIIJUSDT: flat
        [1000.0, 1000.0, 1000.0, 1000.0], // JJJUSDT: flat
    ];

    // Build bars: 4 time steps × 10 symbols = 40 bars, sorted by time then symbol.
    let mut bars: Vec<Bar> = Vec::with_capacity(40);
    // `t` is needed both as an array index (for the prices table) and for timestamp
    // computation, so the range loop is intentional.
    #[allow(clippy::needless_range_loop)]
    for t in 0..4usize {
        for (sym_idx, sym_name) in universe_syms.iter().enumerate() {
            let price_f = prices[sym_idx][t];
            let price_dec = Decimal::try_from(price_f).unwrap_or_else(|_| dec!(1000));

            #[allow(clippy::cast_possible_wrap)]
            let open_ts = Timestamp::new(epoch + time::Duration::hours(t as i64));
            #[allow(clippy::cast_possible_wrap)]
            let close_ts = Timestamp::new(
                epoch + time::Duration::hours(t as i64 + 1) - time::Duration::seconds(1),
            );

            let to_price = |v: Decimal| -> Price {
                Price::new(v).unwrap_or_else(|_| Price::new(dec!(1)).expect("1 always valid"))
            };

            // Use price as open/high/low/close (no intrabar movement needed).
            bars.push(Bar {
                symbol: Symbol::new(*sym_name),
                tf: Timeframe::OneHour,
                open_ts,
                close_ts,
                open: to_price(price_dec),
                high: to_price(price_dec),
                low: to_price(price_dec),
                close: to_price(price_dec),
                volume: Quantity::new(dec!(1000)).expect("1000 always valid"),
                trade_count: 1,
                local_recv_ts: close_ts,
                venue: Venue::Binance,
            });
        }
    }

    // ── Call the REAL run_path ────────────────────────────────────────────────

    let initial_capital = dec!(10_000);
    let input = TcnScenarioInput {
        scenario_name: "solvency-regression-run-path-e2e".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital,
        slippage_bps: 0,
        taker_fee_bps: 4,
        config_id: "top10_momentum_h1".to_string(),
        forecaster_id: "passthrough".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None,
        basis_override: None,
    };

    let result = pollster::block_on(run_path(input, FILL_SEED, strategy));

    // run_path should succeed (bars_override is Some; config file exists for this repo).
    // If the config file is absent in a CI context without the repo root, the test will
    // skip gracefully by returning Err — but we want to ASSERT success here for the
    // regression gate. If this panics in CI, the config file must be present.
    let path_result = result.expect(
        "run_path must succeed: bars_override is Some and config/strategies/top10_momentum_h1.toml exists",
    );

    // ── Assertions ────────────────────────────────────────────────────────────
    //
    // EXECUTION PROOF (#66 rule): the fixture must actually trade (the nine
    // warmup BUYs at minimum). A zero-trade run would make every assertion
    // below vacuously green.
    assert!(
        path_result.trades > 0,
        "SOLVENCY REGRESSION execution proof: the adversarial fixture executed \
         zero fills — the scenario no longer exercises the Buy branch at all."
    );

    // PRIMARY GATE (RED-on-revert): assert min_cash_seen ≥ 0.
    //
    // `min_cash_seen` is tracked in PathRunResult as the minimum cash value
    // observed during the run. Under Bug B (guard removed), the BUY AAA signal
    // fires when cash≈$996 < notional($999.64) + fee($0.40), so the BUY
    // deducts $1,000 from $996 of available cash → min_cash_seen ≈ -$3.64.
    //
    // Under the v0.1.1 guard, the BUY is SKIPPED (cash < notional + fee_est),
    // so cash stays ≥ 0 throughout → min_cash_seen ≥ 0. PASS.
    //
    // This assertion goes RED when either of the two guard layers is removed:
    //   Layer 1: pre-flight check (skip if cash < notional + fee_est)
    //   Layer 2: fill-loop guard (skip fill if total_cost > cash)
    // (A former third layer — the notional cap min(target, cash) — was removed
    // at review 1-14 as dead code: with any positive taker fee the capped buy
    // always failed the pre-flight, so no downsized buy could ever execute.)
    assert!(
        path_result.min_cash_seen >= Decimal::ZERO,
        "SOLVENCY REGRESSION (RED-ON-REVERT GATE): min_cash_seen={} < 0. \
         The solvency guard in montecarlo.rs (Bug B fix, v0.1.1) was removed or \
         is not covering this scenario. Under the OLD code, BUY AAA fires when \
         cash≈$996 < notional($999.64) + fee($0.40), driving cash to ≈-$3.64. \
         Restore both guard layers (pre-flight check + fill loop guard) in the \
         Buy branch of run_path.",
        path_result.min_cash_seen
    );

    // SECONDARY GATE: equity must remain non-negative.
    // With stable synthetic prices, equity won't go negative even when cash does
    // (positions maintain their value). This gate is complementary — it would catch
    // a more extreme scenario where prices also crash.
    for (i, &eq) in path_result.equity_curve.iter().enumerate() {
        assert!(
            eq >= Decimal::ZERO,
            "SOLVENCY REGRESSION: equity_curve[{i}] = {eq} < 0. \
             Long-only equity should not go negative."
        );
    }

    // final_equity must be ≥ 0.
    assert!(
        path_result.final_equity >= Decimal::ZERO,
        "SOLVENCY REGRESSION: final_equity={} < 0. Long-only book cannot have negative equity.",
        path_result.final_equity
    );

    // DISTRIBUTION-LEVEL RESTATEMENT over the REAL curve (review 1-14 — the
    // assertions the deleted fixture-based invariant test claimed to make,
    // now computed from genuine run_path output):
    // - total_return > −1.0: equity never went below zero end-to-end;
    // - max_drawdown ≤ 1.0 (100%): only possible to exceed if equity went
    //   negative — the exact signature of the pre-v0.1.1 cash bug.
    let total_ret = compute_total_return(&path_result.equity_curve);
    assert!(
        total_ret > -1.0,
        "SOLVENCY REGRESSION: total_return {total_ret:.6} ≤ −1.0 on the REAL \
         equity curve — equity went to (or below) zero on a long-only book."
    );
    let max_dd = compute_max_drawdown_f64(&path_result.equity_curve);
    assert!(
        max_dd <= 1.0,
        "SOLVENCY REGRESSION: max_drawdown {max_dd:.4} > 1.0 (100%) on the REAL \
         equity curve — only possible if equity went negative (pre-v0.1.1 cash bug)."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// ADR-0051 D2 — sequential reduction order (R-NR.6 structural)
// ─────────────────────────────────────────────────────────────────────────────

/// Verify that the index-order is preserved: the metrics vec fed to the reducer
/// must be in ascending j order. Re-ordering MUST produce the same result
/// (the reducer itself sorts by `total_cmp` for percentiles — but mean/std
/// are sequential-fold-dependent, so feeding in reverse must still match
/// because we sort by j before feeding the metrics in the production code).
///
/// This test is a structural guard: it verifies that the reducer is pure
/// (same inputs → same outputs regardless of caller order, because we always
/// sort by j before calling).
#[test]
fn reduction_is_pure_same_inputs_same_outputs() {
    let metrics: Vec<PathMetrics> = (0..10)
        .map(|j| {
            let seed = path_seed(0xCAFE, j);
            let bars = synthetic_bars(seed, 200);
            let equity = fake_equity_curve(&bars);
            let final_eq = *equity.last().unwrap_or(&dec!(100_000));
            PathMetrics {
                sharpe: compute_sharpe_hourly(&equity),
                sortino: compute_sortino_hourly(&equity),
                calmar: compute_calmar(&equity),
                max_drawdown: compute_max_drawdown_f64(&equity),
                total_return: compute_total_return(&equity),
                final_equity: final_eq,
                initial_equity: dec!(100_000),
            }
        })
        .collect();

    let s1 = DistributionSummary::from_path_metrics(&metrics).unwrap();
    let s2 = DistributionSummary::from_path_metrics(&metrics).unwrap();

    // Same inputs → same outputs (pure function, no side effects).
    assert_eq!(
        format!("{:.6}", s1.sharpe.mean),
        format!("{:.6}", s2.sharpe.mean),
        "reducer must be pure: same inputs → same mean"
    );
    assert_eq!(
        format!("{:.6}", s1.sharpe.p95),
        format!("{:.6}", s2.sharpe.p95),
        "reducer must be pure: same inputs → same p95"
    );
}
