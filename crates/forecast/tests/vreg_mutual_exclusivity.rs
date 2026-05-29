//! V-REG mutual exclusivity test (T-D-E2, ADR-0049 § D4).
//!
//! Asserts:
//! 1. Five per-label fixtures: one hand-crafted stats set per V-REG-1..V-REG-5 — each
//!    fixture fires exactly the expected verdict.
//! 2. Property test: 100 random stats sets always return exactly one verdict from
//!    {V-REG-1, V-REG-2, V-REG-3, V-REG-4, V-REG-5}.
//!
//! ## Algorithm
//!
//! The verdict algorithm is inlined here (verbatim from `regime_verdict.rs`) so
//! this test file is self-contained and does NOT require the `realdata` feature.
//! This mirrors the `vol_verdict_mutual_exclusivity.rs` precedent (ADR-0038 § D1.b).
//!
//! ## Threshold notes (from ADR-0049 § D4)
//!
//! - V-REG-1: backtest did not complete (EM convergence failure proxy).
//! - V-REG-2: dominant regime > 95% of active bars (trivial classifier).
//! - V-REG-3: switch rate upper bound > 20/week.
//! - V-REG-4: calibration proxy anomaly (final_equity < 0 or total_return < -0.90).
//! - V-REG-5: fallback (all V-REG-1..4 false).
//!
//! ## Cross-references
//!
//! - T-D-E2 — this test.
//! - ADR-0049 § D4 — V-REG priority tree.
//! - ADR-0038 § D1.b — V-VOL precedent (sibling pattern).
//! - `crates/forecast/tests/vol_verdict_mutual_exclusivity.rs` — sibling test.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

// ── Inlined types (mirrors regime_verdict.rs) ─────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum VRegVerdict {
    VReg1,
    VReg2,
    VReg3,
    VReg4,
    VReg5,
}

impl VRegVerdict {
    fn label(&self) -> &'static str {
        match self {
            VRegVerdict::VReg1 => "V-REG-1",
            VRegVerdict::VReg2 => "V-REG-2",
            VRegVerdict::VReg3 => "V-REG-3",
            VRegVerdict::VReg4 => "V-REG-4",
            VRegVerdict::VReg5 => "V-REG-5",
        }
    }
}

#[derive(Debug, Clone)]
struct RunStats {
    suppressed_bars: u64,
    momentum_bars: u64,
    total_bars: u64,
    total_return: f64,
    final_equity: f64,
    completed_ok: bool,
}

impl RunStats {
    fn suppress_rate(&self) -> f64 {
        let active = self.suppressed_bars + self.momentum_bars;
        if active == 0 {
            return 0.0;
        }
        self.suppressed_bars as f64 / active as f64
    }

    fn momentum_rate(&self) -> f64 {
        1.0 - self.suppress_rate()
    }

    fn weeks_elapsed(&self) -> f64 {
        let per_symbol_bars = self.total_bars as f64 / 10.0;
        per_symbol_bars / (7.0 * 24.0)
    }

    const SYMBOL_COUNT: f64 = 10.0;

    fn estimated_switches_per_week_upper_bound(&self) -> f64 {
        let weeks = self.weeks_elapsed();
        if weeks < 1.0 {
            return 0.0;
        }
        let suppressed_portfolio_hours = self.suppressed_bars as f64 / Self::SYMBOL_COUNT;
        let estimated_blocks = (suppressed_portfolio_hours / 3.0).ceil();
        let total_estimated_switches = 2.0 * estimated_blocks;
        total_estimated_switches / weeks
    }
}

/// Verbatim copy of the ADR-0049 § D4 V-REG priority tree from `regime_verdict.rs`.
fn classify_vreg(stats: &RunStats) -> VRegVerdict {
    // V-REG-1: convergence failure.
    if !stats.completed_ok {
        return VRegVerdict::VReg1;
    }

    // V-REG-2: trivial classifier (dominant regime > 95%).
    let suppress_rate = stats.suppress_rate();
    let momentum_rate = stats.momentum_rate();
    if suppress_rate > 0.95 || momentum_rate > 0.95 {
        return VRegVerdict::VReg2;
    }

    // V-REG-3: flicker (switch rate upper bound > 20/week).
    let switch_rate_upper = stats.estimated_switches_per_week_upper_bound();
    if switch_rate_upper > 20.0 {
        return VRegVerdict::VReg3;
    }

    // V-REG-4: calibration proxy anomaly.
    let calibration_anomaly = stats.final_equity < 0.0
        || stats.total_return < -0.90
        || (suppress_rate < 0.01 && stats.total_bars > 1000);
    if calibration_anomaly {
        return VRegVerdict::VReg4;
    }

    // V-REG-5: healthy fallback.
    VRegVerdict::VReg5
}

// ── Fixture helpers ───────────────────────────────────────────────────────────

fn make_stats(
    completed_ok: bool,
    suppressed_bars: u64,
    momentum_bars: u64,
    total_bars: u64,
    total_return: f64,
    final_equity: f64,
) -> RunStats {
    RunStats {
        suppressed_bars,
        momentum_bars,
        total_bars,
        total_return,
        final_equity,
        completed_ok,
    }
}

// ── Fixture tests: one per V-REG label ───────────────────────────────────────

/// V-REG-1 fixture: backtest did not complete (EM convergence failure proxy).
#[test]
fn vreg1_fixture_fires_vreg1() {
    let stats = make_stats(false, 100, 8000, 10000, -0.05, 95_000.0);
    let v = classify_vreg(&stats);
    assert_eq!(
        v.label(),
        "V-REG-1",
        "V-REG-1 fixture must fire V-REG-1, got {}",
        v.label()
    );
    println!(
        "[vreg1_fixture_fires_vreg1] PASS — verdict: {}",
        v.label()
    );
}

/// V-REG-2 fixture: CashHold dominant > 95%.
#[test]
fn vreg2_fixture_fires_vreg2_cashhold_dominant() {
    let stats = make_stats(true, 9700, 200, 10000, -0.01, 99_000.0);
    let v = classify_vreg(&stats);
    assert_eq!(
        v.label(),
        "V-REG-2",
        "V-REG-2 fixture (CashHold dominant) must fire V-REG-2, got {}",
        v.label()
    );
    println!(
        "[vreg2_fixture_fires_vreg2_cashhold_dominant] PASS — suppress_rate={:.4}",
        stats.suppress_rate()
    );
}

/// V-REG-2 fixture: Momentum dominant > 95%.
#[test]
fn vreg2_fixture_fires_vreg2_momentum_dominant() {
    let stats = make_stats(true, 100, 9700, 10000, -0.10, 90_000.0);
    let v = classify_vreg(&stats);
    assert_eq!(
        v.label(),
        "V-REG-2",
        "V-REG-2 fixture (Momentum dominant) must fire V-REG-2, got {}",
        v.label()
    );
    println!(
        "[vreg2_fixture_fires_vreg2_momentum_dominant] PASS — momentum_rate={:.4}",
        stats.momentum_rate()
    );
}

/// V-REG-3 fixture: high switch rate upper bound.
#[test]
fn vreg3_fixture_fires_vreg3() {
    // 5000 suppressed out of 10000 total → upper bound > 20/week.
    let stats = make_stats(true, 5000, 5000, 10000, -0.10, 90_000.0);
    let switch_ub = stats.estimated_switches_per_week_upper_bound();
    assert!(
        switch_ub > 20.0,
        "V-REG-3 fixture should have switch_ub > 20, got {switch_ub:.2}"
    );
    let v = classify_vreg(&stats);
    assert_eq!(
        v.label(),
        "V-REG-3",
        "V-REG-3 fixture must fire V-REG-3, got {}",
        v.label()
    );
    println!(
        "[vreg3_fixture_fires_vreg3] PASS — switch_ub={:.2}/wk",
        switch_ub
    );
}

/// V-REG-4 fixture: calibration proxy anomaly (final_equity < 0).
#[test]
fn vreg4_fixture_fires_vreg4() {
    let stats = make_stats(true, 1000, 8000, 10000, -1.10, -5_000.0);
    assert!(
        stats.final_equity < 0.0,
        "V-REG-4 fixture should have final_equity < 0"
    );
    let v = classify_vreg(&stats);
    assert_eq!(
        v.label(),
        "V-REG-4",
        "V-REG-4 fixture must fire V-REG-4, got {}",
        v.label()
    );
    println!("[vreg4_fixture_fires_vreg4] PASS");
}

/// V-REG-5 fixture: healthy stats (real 2024 run numbers).
#[test]
fn vreg5_fixture_fires_vreg5() {
    // From actual 2024 run: suppressed=11816, momentum=75524, total=87840.
    let stats = make_stats(true, 11816, 75524, 87840, -0.059, 94_000.96);
    let v = classify_vreg(&stats);
    assert_eq!(
        v.label(),
        "V-REG-5",
        "V-REG-5 fixture must fire V-REG-5, got {}",
        v.label()
    );
    println!(
        "[vreg5_fixture_fires_vreg5] PASS — suppress_rate={:.4}, switch_ub={:.2}/wk",
        stats.suppress_rate(),
        stats.estimated_switches_per_week_upper_bound()
    );
}

// ── Mutual exclusivity ────────────────────────────────────────────────────────

/// Each fixture fires exactly one verdict (trivially true by construction,
/// but verified explicitly).
#[test]
fn vreg1_vreg2_vreg3_vreg4_vreg5_all_distinct() {
    let fixtures = [
        ("V-REG-1", make_stats(false, 100, 8000, 10000, -0.05, 95_000.0)),
        ("V-REG-2", make_stats(true, 9700, 200, 10000, -0.01, 99_000.0)),
        ("V-REG-3", make_stats(true, 5000, 5000, 10000, -0.10, 90_000.0)),
        ("V-REG-4", make_stats(true, 1000, 8000, 10000, -1.10, -5_000.0)),
        ("V-REG-5", make_stats(true, 11816, 75524, 87840, -0.059, 94_000.96)),
    ];

    for (expected, stats) in &fixtures {
        let v = classify_vreg(stats);
        assert_eq!(
            v.label(),
            *expected,
            "fixture[{expected}] returned {} instead of {expected}",
            v.label()
        );
        println!("[vreg1_vreg2_vreg3_vreg4_vreg5_all_distinct] {expected}: PASS");
    }
}

/// Property test: 100 random stats sets each return exactly one of
/// {V-REG-1, V-REG-2, V-REG-3, V-REG-4, V-REG-5}.
#[test]
fn property_exactly_one_verdict_on_random_stats() {
    let mut rng = ChaCha20Rng::from_seed([42u8; 32]);
    let valid_labels = ["V-REG-1", "V-REG-2", "V-REG-3", "V-REG-4", "V-REG-5"];
    let mut counts = [0usize; 5];

    for trial in 0..100 {
        // Random stats with wide variation.
        let completed_ok = rng.random_bool(0.9); // 90% completion rate
        let total_bars: u64 = rng.random_range(1000_u64..100_000_u64);
        let suppress_frac = rng.random_range(0.0_f64..1.0);
        let active_bars = total_bars;
        let suppressed_bars = (active_bars as f64 * suppress_frac) as u64;
        let momentum_bars = active_bars - suppressed_bars;
        let total_return = rng.random_range(-2.0_f64..2.0);
        let final_equity = rng.random_range(-50_000.0_f64..200_000.0);

        let stats = make_stats(
            completed_ok,
            suppressed_bars,
            momentum_bars,
            total_bars,
            total_return,
            final_equity,
        );

        let v = classify_vreg(&stats);
        let label = v.label();

        // Verify label is valid.
        assert!(
            valid_labels.contains(&label),
            "trial {trial}: got invalid verdict label '{label}'"
        );

        // Count verdicts.
        match label {
            "V-REG-1" => counts[0] += 1,
            "V-REG-2" => counts[1] += 1,
            "V-REG-3" => counts[2] += 1,
            "V-REG-4" => counts[3] += 1,
            "V-REG-5" => counts[4] += 1,
            _ => unreachable!(),
        }
    }

    println!(
        "[property_exactly_one_verdict_on_random_stats] PASS — 100 trials: \
         V-REG-1={}, V-REG-2={}, V-REG-3={}, V-REG-4={}, V-REG-5={}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );

    // Sanity: all 100 trials returned exactly one verdict.
    let total: usize = counts.iter().sum();
    assert_eq!(total, 100, "expected 100 total, got {total}");
}
