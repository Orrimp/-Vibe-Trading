//! `RobustnessMode::Bootstrap` compute path (ADR-0063 § D4).
//!
//! ## Overview
//!
//! `compute_robustness_flag(equity, paths, master_seed)` is a pure function of
//! (realized equity curve as Decimals, seed, path count) → `RobustnessFlag`.
//!
//! ## Algorithm (ADR-0063 § D4 + ADR-0051 D1)
//!
//! 1. equity → log-returns (identical to the mapping in `compute_sharpe_hourly`).
//! 2. Block length via `data::synth::block_length::politis_white_block_length`
//!    (Politis–White PWSD — not a magic constant; the project's established choice).
//! 3. Draw `paths` moving-block-bootstrap resamples using the FROZEN ADR-0051 D1
//!    sub-seed rule: `path_seed_j = master.wrapping_add(j * 0x9E37_79B9)`,
//!    one `ChaCha20Rng::seed_from_u64(path_seed_j)` per path.
//! 4. Per path: rebuild a synthetic equity curve from the resampled returns;
//!    compute `PathMetrics` via the existing annualised stat fns.
//! 5. Reduce via `DistributionSummary::from_path_metrics`; classify via the
//!    FROZEN `classify_verdict` → `RobustnessFlag`.
//!
//! ## Determinism
//!
//! The entire compute is a pure function of (`equity_decimals`, `master_seed`, `paths`).
//! Same bake-off seed → same `master_seed` derivation → same resamples →
//! same flags.  No `SystemTime`, no `thread_rng`, no `OsRng`.
//!
//! ## Anchor safety
//!
//! This function is ONLY called from the `RobustnessMode::Bootstrap` arm in
//! `run_bakeoff`, which is opt-in (default stays `Skip`).
//! It is never called by any anchored CLI report path.

#![allow(clippy::float_arithmetic)] // statistical metric computations

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

use crate::bakeoff::robustness::{RobustnessFlag, classify_verdict};
use crate::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};

// ── ADR-0051 § D1 sub-seed constant (frozen) ─────────────────────────────────

/// Frozen sub-seed multiplier per ADR-0051 D1.
///
/// `path_seed_j = master_seed.wrapping_add((j as u64).wrapping_mul(GOLDEN_GAMMA))`
/// One `ChaCha20Rng::seed_from_u64(path_seed_j)` per path.
const GOLDEN_GAMMA: u64 = 0x9E37_79B9;

// ── Fixed per-candidate salt table (frozen constant, ADR-0063 § D4) ──────────

/// Per-candidate salt array indexed by `candidate_index % SALT_TABLE_LEN`.
///
/// Ensures two candidates in the same bake-off do NOT share resample draws
/// even if their equity curves happen to be identical.  The table is frozen
/// (not a search parameter — the only free choices are "number of salts" = 16
/// and their values).
pub(crate) const SALT_TABLE: [u64; 16] = [
    0x0000_0000_0000_0001,
    0xBEEF_CAFE_DEAD_0001,
    0x1234_5678_9ABC_DEF0,
    0xFEDC_BA98_7654_3210,
    0xA5A5_A5A5_5A5A_5A5A,
    0x0F0F_0F0F_F0F0_F0F0,
    0x1111_1111_1111_1111,
    0x2222_2222_2222_2222,
    0x3333_3333_3333_3333,
    0x4444_4444_4444_4444,
    0x5555_5555_5555_5555,
    0x6666_6666_6666_6666,
    0x7777_7777_7777_7777,
    0x8888_8888_8888_8888,
    0x9999_9999_9999_9999,
    0xAAAA_AAAA_AAAA_AAAA,
];

/// Derive the master seed for one candidate from the bake-off seed + candidate index.
///
/// `bakeoff_seed_u64` is the bake-off seed mapped to `u64` (via `engine::seed_to_u64`).
/// `candidate_index` is the 0-based insertion index in the bake-off field.
///
/// The XOR with the salt ensures different candidates draw different resample
/// sequences even when their equity curves are identical.
#[must_use]
pub fn derive_master_seed(bakeoff_seed_u64: u64, candidate_index: usize) -> u64 {
    let salt = SALT_TABLE[candidate_index % SALT_TABLE.len()];
    bakeoff_seed_u64.wrapping_add(salt)
}

// ── Core computation ─────────────────────────────────────────────────────────

/// Compute the full bootstrap distribution AND the verdict for one candidate.
///
/// # Arguments
///
/// - `equity_decimals`: the candidate's realized equity curve as `Decimal` values
///   in chronological order (same slice used by `compute_sharpe_hourly`).
/// - `paths`: number of bootstrap resamples (default 1000 per ADR-0063 § D4).
/// - `master_seed`: the candidate-specific master seed (from `derive_master_seed`).
///
/// # Returns
///
/// `Some((DistributionSummary, ParamRobustnessVerdict))` — the full bootstrap
/// distribution (p5/p50/p95 + the 5 gate signals) plus the verdict.
/// Returns `None` if the curve is too short to compute returns.
///
/// # Gate contract (ADR-0069 D2 — behaviour-preserving)
///
/// The gate bands (`verdict_bands`), the seed rule (ADR-0051 D1 `GOLDEN_GAMMA`),
/// the block-length policy (Politis–White PWSD), and the path count are all
/// UNCHANGED from `compute_robustness_flag`. This function is the additive
/// sibling that surfaces the distribution; `compute_robustness_flag` delegates
/// to it (bit-identical — proven by `compute_robustness_distribution_matches_flag`).
#[must_use]
pub fn compute_robustness_distribution(
    equity_decimals: &[Decimal],
    paths: usize,
    master_seed: u64,
) -> Option<(
    DistributionSummary,
    crate::bakeoff::robustness::ParamRobustnessVerdict,
)> {
    if equity_decimals.len() < 2 {
        return None;
    }

    // Step 1: equity → f64 log-returns (same mapping as compute_sharpe_hourly).
    let log_returns = equity_to_log_returns_f64(equity_decimals);
    if log_returns.is_empty() {
        return None;
    }

    // Step 2: block length via Politis–White PWSD selector.
    let block_len = data::synth::block_length::politis_white_block_length(&log_returns).max(1);

    // Initial equity (Decimal) for P(loss) comparisons.
    let initial_equity = equity_decimals[0];

    // Step 3 + 4: draw `paths` resamples; compute PathMetrics per path.
    // FROZEN: ADR-0051 D1 sub-seed rule — path_seed_j = master.wrapping_add(j * GOLDEN_GAMMA).
    let path_metrics: Vec<PathMetrics> = (0..paths)
        .map(|j| {
            // ADR-0051 D1 sub-seed rule (frozen).
            let path_seed_j = master_seed.wrapping_add((j as u64).wrapping_mul(GOLDEN_GAMMA));
            let mut rng = ChaCha20Rng::seed_from_u64(path_seed_j);

            // Resample the return series via moving-block bootstrap.
            let resampled = moving_block_resample(&log_returns, block_len, &mut rng);

            // Reconstruct equity curve from resampled returns.
            let eq_curve = returns_to_equity_decimal(&resampled, initial_equity);

            // Compute per-path metrics using the established stat fns.
            let sharpe = compute_sharpe_hourly(&eq_curve);
            let sortino = compute_sortino_hourly(&eq_curve);
            let calmar = compute_calmar(&eq_curve);
            let max_drawdown = compute_max_drawdown_f64(&eq_curve);
            let total_return = compute_total_return(&eq_curve);
            let final_equity = *eq_curve.last().unwrap_or(&initial_equity);

            PathMetrics {
                sharpe,
                sortino,
                calmar,
                max_drawdown,
                total_return,
                final_equity,
                initial_equity,
            }
        })
        .collect();

    // Step 5: reduce via DistributionSummary; classify via frozen classify_verdict.
    let summary = match DistributionSummary::from_path_metrics(&path_metrics) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "compute_robustness_distribution: DistributionSummary::from_path_metrics failed"
            );
            return None;
        }
    };

    let verdict = classify_verdict(&summary);
    Some((summary, verdict))
}

/// Compute the robustness flag for one candidate via moving-block bootstrap.
///
/// # Arguments
///
/// - `equity_decimals`: the candidate's realized equity curve as `Decimal` values
///   in chronological order (same slice used by `compute_sharpe_hourly`).
/// - `paths`: number of bootstrap resamples (default 1000 per ADR-0063 § D4).
/// - `master_seed`: the candidate-specific master seed (from `derive_master_seed`).
///
/// # Returns
///
/// `RobustnessFlag` — `Robust`, `Marginal`, or `Fragile`.
/// Returns `RobustnessFlag::Skipped` if the curve is too short to compute returns.
///
/// # Delegation (ADR-0069 D2)
///
/// Delegates to `compute_robustness_distribution` and discards the summary.
/// The output is bit-identical to the pre-refactor implementation — proven by
/// `compute_robustness_distribution_matches_flag` in `crates/backtest/tests/`.
#[must_use]
pub fn compute_robustness_flag(
    equity_decimals: &[Decimal],
    paths: usize,
    master_seed: u64,
) -> RobustnessFlag {
    match compute_robustness_distribution(equity_decimals, paths, master_seed) {
        None => RobustnessFlag::Skipped,
        Some((_summary, verdict)) => RobustnessFlag::from(verdict),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convert an equity curve (Decimal) to f64 log-returns.
///
/// Returns `ln(eq[i] / eq[i-1])` for i in 1..n, matching `compute_sharpe_hourly`.
fn equity_to_log_returns_f64(equity: &[Decimal]) -> Vec<f64> {
    if equity.len() < 2 {
        return vec![];
    }
    equity
        .windows(2)
        .map(|w| {
            let prev = w[0].to_f64().unwrap_or(1.0);
            let curr = w[1].to_f64().unwrap_or(1.0);
            if prev <= 0.0 { 0.0 } else { (curr / prev).ln() }
        })
        .collect()
}

/// Moving-block bootstrap resample of a return series.
///
/// Draws `n = returns.len()` returns by sampling blocks of length `block_len`
/// uniformly at random (with replacement, wrap-around).
///
/// Uses the caller-supplied `rng` so determinism is controlled upstream.
fn moving_block_resample(returns: &[f64], block_len: usize, rng: &mut ChaCha20Rng) -> Vec<f64> {
    let n = returns.len();
    if n == 0 {
        return vec![];
    }
    let block_len = block_len.max(1).min(n);
    let mut out = Vec::with_capacity(n);

    while out.len() < n {
        // Wrap-around block start: sample any index 0..n.
        let start: usize = rng.random_range(0..n);
        for k in 0..block_len {
            if out.len() >= n {
                break;
            }
            out.push(returns[(start + k) % n]);
        }
    }

    out.truncate(n);
    out
}

/// Reconstruct a Decimal equity curve from f64 log-returns with given initial equity.
///
/// Returns a `Vec<Decimal>` of length `returns.len() + 1` (including initial equity
/// at index 0).  Uses `Decimal::from_f64` → falls back to `initial_equity` on
/// conversion failure (avoids panics on extreme resampled paths).
fn returns_to_equity_decimal(log_returns: &[f64], initial_equity: Decimal) -> Vec<Decimal> {
    let mut curve = Vec::with_capacity(log_returns.len() + 1);
    curve.push(initial_equity);

    let mut current_f64 = initial_equity.to_f64().unwrap_or(0.0);

    for &r in log_returns {
        let factor = r.exp();
        current_f64 *= factor;
        let next = Decimal::from_f64(current_f64).unwrap_or(initial_equity);
        curve.push(next);
    }

    curve
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn growing_equity(n: usize) -> Vec<Decimal> {
        (0..n)
            .map(|i| dec!(100_000) + Decimal::from(i) * dec!(10))
            .collect()
    }

    fn declining_equity(n: usize) -> Vec<Decimal> {
        (0..n)
            .map(|i| (dec!(100_000) - Decimal::from(i) * dec!(100)).max(dec!(1)))
            .collect()
    }

    #[test]
    fn derive_master_seed_deterministic() {
        let s1 = derive_master_seed(0xDEAD_BEEF, 0);
        let s2 = derive_master_seed(0xDEAD_BEEF, 0);
        assert_eq!(s1, s2, "derive_master_seed must be deterministic");
    }

    #[test]
    fn derive_master_seed_different_candidates() {
        let s0 = derive_master_seed(12345, 0);
        let s1 = derive_master_seed(12345, 1);
        assert_ne!(
            s0, s1,
            "different candidate indices must produce different master seeds"
        );
    }

    #[test]
    fn compute_robustness_flag_short_curve_returns_skipped() {
        let equity = vec![dec!(100_000)];
        let flag = compute_robustness_flag(&equity, 100, 42);
        assert_eq!(
            flag,
            RobustnessFlag::Skipped,
            "equity curve with 1 point must return Skipped"
        );
    }

    #[test]
    fn compute_robustness_flag_empty_curve_returns_skipped() {
        let flag = compute_robustness_flag(&[], 100, 42);
        assert_eq!(flag, RobustnessFlag::Skipped);
    }

    #[test]
    fn compute_robustness_flag_deterministic() {
        let equity = growing_equity(300);
        let f1 = compute_robustness_flag(&equity, 50, 99999);
        let f2 = compute_robustness_flag(&equity, 50, 99999);
        assert_eq!(f1, f2, "same inputs must produce same RobustnessFlag");
    }

    #[test]
    fn compute_robustness_flag_different_seeds_run_cleanly() {
        let equity = growing_equity(200);
        let _ = compute_robustness_flag(&equity, 20, 1111);
        let _ = compute_robustness_flag(&equity, 20, 2222);
        // Both must complete without panic.
    }

    #[test]
    fn moving_block_resample_length_preserved() {
        use rand::SeedableRng;
        let returns = vec![0.01_f64, -0.005, 0.02, -0.01, 0.015];
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let resampled = moving_block_resample(&returns, 2, &mut rng);
        assert_eq!(resampled.len(), returns.len());
    }

    #[test]
    fn moving_block_resample_deterministic() {
        use rand::SeedableRng;
        let returns = vec![0.01_f64, -0.005, 0.02, -0.01, 0.015];
        let mut rng1 = ChaCha20Rng::seed_from_u64(77);
        let mut rng2 = ChaCha20Rng::seed_from_u64(77);
        let r1 = moving_block_resample(&returns, 2, &mut rng1);
        let r2 = moving_block_resample(&returns, 2, &mut rng2);
        assert_eq!(r1, r2);
    }

    #[test]
    fn declining_equity_not_robust() {
        // A sharply declining curve should NOT be classified Robust.
        let equity = declining_equity(500);
        let flag = compute_robustness_flag(&equity, 100, 42);
        assert_ne!(
            flag,
            RobustnessFlag::Robust,
            "sharply declining equity must not be Robust"
        );
    }

    #[test]
    fn returns_to_equity_decimal_round_trip() {
        // log(1.01) → exp(log(1.01)) = 1.01 round-trip.
        let returns = vec![(1.01_f64).ln()];
        let initial = dec!(1000);
        let curve = returns_to_equity_decimal(&returns, initial);
        assert_eq!(curve.len(), 2);
        assert_eq!(curve[0], initial);
        // curve[1] ≈ 1010.0 — allow small floating-point delta.
        let diff = (curve[1] - dec!(1010)).abs();
        assert!(
            diff < dec!(0.01),
            "round-trip equity: expected ~1010, got {curve:?}"
        );
    }
}
