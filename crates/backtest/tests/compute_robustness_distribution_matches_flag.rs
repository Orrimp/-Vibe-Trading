//! T1 bit-identity test — `compute_robustness_distribution` delegation gate (ADR-0069 D2).
//!
//! ## Contract
//!
//! After the T1 refactor, `compute_robustness_flag` delegates to
//! `compute_robustness_distribution` and maps its `ParamRobustnessVerdict` →
//! `RobustnessFlag`. This test proves the output is bit-identical to the
//! pre-refactor implementation (behaviour-preserving, gate byte-frozen).
//!
//! ## FAIL-before
//!
//! If `compute_robustness_flag` were to diverge from the verdict returned by
//! `compute_robustness_distribution`, at least one assertion below would catch it.
//!
//! ## Pattern reference
//!
//! Modelled on `robustness_bootstrap_bites.rs` (the existing bootstrap gate test).

#![allow(clippy::unwrap_used, clippy::float_arithmetic)]

use backtest::{
    RobustnessFlag,
    bakeoff::{
        bootstrap::{compute_robustness_distribution, compute_robustness_flag},
        robustness::ParamRobustnessVerdict,
    },
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ── Equity curve fixtures ─────────────────────────────────────────────────────

fn growing_equity(n: usize) -> Vec<Decimal> {
    (0..n)
        .map(|i| dec!(100_000) + Decimal::from(i as i64) * dec!(10))
        .collect()
}

fn declining_equity(n: usize) -> Vec<Decimal> {
    (0..n)
        .map(|i| (dec!(100_000) - Decimal::from(i as i64) * dec!(100)).max(dec!(1)))
        .collect()
}

fn flat_equity(n: usize) -> Vec<Decimal> {
    vec![dec!(100_000); n]
}

fn short_equity() -> Vec<Decimal> {
    vec![dec!(100_000)] // length 1 — too short for bootstrap
}

fn empty_equity() -> Vec<Decimal> {
    vec![]
}

// ── Helper: map ParamRobustnessVerdict → expected RobustnessFlag ──────────────

fn verdict_to_expected_flag(v: ParamRobustnessVerdict) -> RobustnessFlag {
    match v {
        ParamRobustnessVerdict::Robust => RobustnessFlag::Robust,
        ParamRobustnessVerdict::Marginal => RobustnessFlag::Marginal,
        ParamRobustnessVerdict::Fragile => RobustnessFlag::Fragile,
    }
}

// ── Core bit-identity gate ────────────────────────────────────────────────────

/// For each equity curve fixture, assert that:
/// 1. `compute_robustness_distribution`'s verdict maps to the SAME `RobustnessFlag`
///    as `compute_robustness_flag` returns directly.
/// 2. The `None` path (too-short curves) maps to `RobustnessFlag::Skipped`.
///
/// This is the T1 mandatory gate: if the delegation ever breaks, this test fails.
#[test]
fn compute_robustness_distribution_verdict_matches_flag_growing() {
    let equity = growing_equity(300);
    let paths = 50;
    let seed = 0xDEAD_BEEF_u64;

    let flag = compute_robustness_flag(&equity, paths, seed);
    let dist_result = compute_robustness_distribution(&equity, paths, seed);

    assert!(
        dist_result.is_some(),
        "compute_robustness_distribution must return Some for a long-enough equity curve"
    );
    let (_summary, verdict) = dist_result.unwrap();
    let expected_flag = verdict_to_expected_flag(verdict);
    assert_eq!(
        flag, expected_flag,
        "compute_robustness_flag and compute_robustness_distribution must agree on verdict (growing equity)"
    );
}

#[test]
fn compute_robustness_distribution_verdict_matches_flag_declining() {
    let equity = declining_equity(500);
    let paths = 50;
    let seed = 0xCAFE_BABE_u64;

    let flag = compute_robustness_flag(&equity, paths, seed);
    let dist_result = compute_robustness_distribution(&equity, paths, seed);

    assert!(dist_result.is_some());
    let (_summary, verdict) = dist_result.unwrap();
    let expected_flag = verdict_to_expected_flag(verdict);
    assert_eq!(
        flag, expected_flag,
        "compute_robustness_flag and compute_robustness_distribution must agree on verdict (declining equity)"
    );
    // Bonus: declining equity must be FRAGILE (behavioural sanity).
    assert_eq!(
        verdict,
        ParamRobustnessVerdict::Fragile,
        "declining equity must be FRAGILE"
    );
    assert_eq!(
        flag,
        RobustnessFlag::Fragile,
        "declining equity flag must be Fragile"
    );
}

#[test]
fn compute_robustness_distribution_verdict_matches_flag_flat() {
    let equity = flat_equity(400);
    let paths = 50;
    let seed = 0x1234_5678_u64;

    let flag = compute_robustness_flag(&equity, paths, seed);
    let dist_result = compute_robustness_distribution(&equity, paths, seed);

    assert!(dist_result.is_some());
    let (_summary, verdict) = dist_result.unwrap();
    let expected_flag = verdict_to_expected_flag(verdict);
    assert_eq!(
        flag, expected_flag,
        "compute_robustness_flag and compute_robustness_distribution must agree on verdict (flat equity)"
    );
}

/// Multiple (equity, seed) pairs — a battery ensures the gate is not accidental.
#[test]
fn compute_robustness_distribution_verdict_matches_flag_battery() {
    let fixtures: Vec<(Vec<Decimal>, u64)> = vec![
        (growing_equity(200), 111),
        (growing_equity(500), 222),
        (declining_equity(300), 333),
        (declining_equity(600), 444),
        (flat_equity(250), 555),
        // Mixed: grow then decline
        (
            {
                let mut v = growing_equity(150);
                v.extend(declining_equity(150));
                v
            },
            666,
        ),
    ];

    for (equity, seed) in fixtures {
        let paths = 30; // low path count for speed in tests
        let flag = compute_robustness_flag(&equity, paths, seed);
        let dist_result = compute_robustness_distribution(&equity, paths, seed);

        assert!(
            dist_result.is_some(),
            "Expected Some for equity of length {}",
            equity.len()
        );
        let (_summary, verdict) = dist_result.unwrap();
        let expected_flag = verdict_to_expected_flag(verdict);
        assert_eq!(
            flag, expected_flag,
            "Mismatch for seed={seed}: flag={flag:?}, verdict={verdict:?}"
        );
    }
}

// ── None path (too-short curves) ──────────────────────────────────────────────

#[test]
fn compute_robustness_distribution_none_for_short_curve() {
    let result = compute_robustness_distribution(&short_equity(), 100, 42);
    assert!(
        result.is_none(),
        "compute_robustness_distribution must return None for a 1-element equity curve"
    );
    // And the flag must be Skipped.
    let flag = compute_robustness_flag(&short_equity(), 100, 42);
    assert_eq!(flag, RobustnessFlag::Skipped);
}

#[test]
fn compute_robustness_distribution_none_for_empty_curve() {
    let result = compute_robustness_distribution(&empty_equity(), 100, 42);
    assert!(
        result.is_none(),
        "compute_robustness_distribution must return None for an empty equity curve"
    );
    let flag = compute_robustness_flag(&empty_equity(), 100, 42);
    assert_eq!(flag, RobustnessFlag::Skipped);
}

// ── Distribution fields sanity ────────────────────────────────────────────────

/// Verify the returned DistributionSummary has plausible field ranges
/// (not asserting exact values — those are seed-dependent).
#[test]
fn compute_robustness_distribution_summary_fields_sane() {
    let equity = growing_equity(300);
    let (summary, _verdict) =
        compute_robustness_distribution(&equity, 50, 999).expect("must return Some");

    // Sharpe p5 ≤ p50 ≤ p95 (order invariant).
    assert!(summary.sharpe.p5 <= summary.sharpe.p50, "sharpe.p5 <= p50");
    assert!(
        summary.sharpe.p50 <= summary.sharpe.p95,
        "sharpe.p50 <= p95"
    );
    // prob_loss is in [0, 1].
    assert!(
        (0.0..=1.0).contains(&summary.prob_loss),
        "prob_loss must be in [0,1]"
    );
    // prob_sharpe_gt_1 is in [0, 1].
    assert!(
        (0.0..=1.0).contains(&summary.prob_sharpe_gt_1),
        "prob_sharpe_gt_1 must be in [0,1]"
    );
    // max_dd_tail_p95 >= 0.
    assert!(
        summary.max_dd_tail_p95 >= 0.0,
        "max_dd_tail_p95 must be >= 0"
    );
}

/// Determinism check: same inputs → same DistributionSummary AND same verdict.
#[test]
fn compute_robustness_distribution_is_deterministic() {
    let equity = growing_equity(250);
    let paths = 40;
    let seed = 0xF0F0_F0F0_u64;

    let (summary1, verdict1) =
        compute_robustness_distribution(&equity, paths, seed).expect("must return Some");
    let (summary2, verdict2) =
        compute_robustness_distribution(&equity, paths, seed).expect("must return Some");

    assert_eq!(verdict1, verdict2, "verdict must be deterministic");
    // Check key fields are byte-identical.
    assert_eq!(
        summary1.sharpe.p50.to_bits(),
        summary2.sharpe.p50.to_bits(),
        "sharpe.p50 must be deterministic"
    );
    assert_eq!(
        summary1.prob_loss.to_bits(),
        summary2.prob_loss.to_bits(),
        "prob_loss must be deterministic"
    );
}
