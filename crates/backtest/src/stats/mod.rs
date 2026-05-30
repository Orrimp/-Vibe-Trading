//! Shared metric calculators and the Monte-Carlo distribution-summary reducer.
//!
//! ## M-DEV-1 — verbatim calculator lift (R-NR.5)
//!
//! `compute_sharpe_hourly`, `compute_sortino_hourly`, `compute_calmar`,
//! `compute_max_drawdown_f64`, and `compute_total_return` are lifted verbatim
//! from `crates/backtest/src/bin/threshold_sweep.rs` (lines 232-347).
//! The arithmetic is byte-identical; only the module path changes.
//! `bin/threshold_sweep.rs` re-imports them from here so the existing
//! threshold-sweep report body stays byte-identical (R-NR.5).
//!
//! ## M-DEV-2 — `DistributionSummary` reducer (ADR-0051 D2)
//!
//! The only genuinely new math. Implements index-order sequential
//! mean/two-pass std + `f64::total_cmp` sort + NaN-absent assertion +
//! type-7 linear percentile.
//!
//! **LOAD-BEARING COMMENT (do not remove):**
//! ADR-0051 D2 mandates that the aggregation reduction is **NOT** parallelised.
//! `par_iter().sum()` would flap the anchor body-SHA because f64 addition is
//! non-associative. The `// ADR-0051 D2:` comment below marks the critical site.

// Float arithmetic in return-space is required for statistical metrics.
// Money math stays `Decimal` at the backtest engine boundary (ADR-0003).
#![allow(clippy::float_arithmetic)]

use rust_decimal::Decimal;

// ─────────────────────────────────────────────────────────────────────────────
// M-DEV-1 — Verbatim calculator lift from bin/threshold_sweep.rs (R-NR.5)
// These functions are byte-identical to the originals; only their path changed.
// ─────────────────────────────────────────────────────────────────────────────

/// Compute Sharpe (annualised, hourly) from an equity curve.
/// Formula: `mean_log_return` / `std_log_return` * sqrt(24*365).
///
/// Lifted verbatim from `bin/threshold_sweep.rs:232` (R-NR.5).
#[must_use]
#[allow(clippy::cast_precision_loss)] // R-NR.5: verbatim lift — cast matches original
pub fn compute_sharpe_hourly(equity: &[Decimal]) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    const SQRT_HPY: f64 = 92.601_295_098_46;
    let n = equity.len();
    if n < 2 {
        return 0.0;
    }
    let rets: Vec<f64> = equity
        .windows(2)
        .map(|w| {
            let prev = w[0].to_f64().unwrap_or(1.0);
            let curr = w[1].to_f64().unwrap_or(1.0);
            if prev <= 0.0 { 0.0 } else { (curr / prev).ln() }
        })
        .collect();
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|&r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
    let std = var.sqrt();
    if std < 1e-15 {
        0.0
    } else {
        mean / std * SQRT_HPY
    }
}

/// Compute Sortino (annualised, hourly) from an equity curve.
///
/// Lifted verbatim from `bin/threshold_sweep.rs:259` (R-NR.5).
#[must_use]
#[allow(clippy::cast_precision_loss)] // R-NR.5: verbatim lift — cast matches original
pub fn compute_sortino_hourly(equity: &[Decimal]) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    const SQRT_HPY: f64 = 92.601_295_098_46;
    let n = equity.len();
    if n < 2 {
        return 0.0;
    }
    let rets: Vec<f64> = equity
        .windows(2)
        .map(|w| {
            let prev = w[0].to_f64().unwrap_or(1.0);
            let curr = w[1].to_f64().unwrap_or(1.0);
            if prev <= 0.0 { 0.0 } else { (curr / prev).ln() }
        })
        .collect();
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let down_sq = rets.iter().map(|&r| r.min(0.0).powi(2)).sum::<f64>() / rets.len() as f64;
    let down_std = down_sq.sqrt();
    if down_std < 1e-15 {
        0.0
    } else {
        mean / down_std * SQRT_HPY
    }
}

/// Compute Calmar ratio from an equity curve.
///
/// Lifted verbatim from `bin/threshold_sweep.rs:285` (R-NR.5).
#[must_use]
#[allow(clippy::cast_precision_loss)] // R-NR.5: verbatim lift — cast matches original
pub fn compute_calmar(equity: &[Decimal]) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    let n = equity.len();
    if n < 2 {
        return 0.0;
    }
    let initial = equity[0].to_f64().unwrap_or(0.0);
    let final_eq = equity[n - 1].to_f64().unwrap_or(0.0);
    if initial <= 0.0 {
        return 0.0;
    }
    let years = (n as f64 - 1.0) / 8760.0;
    if years <= 0.0 {
        return 0.0;
    }
    let cagr = (final_eq / initial).powf(1.0 / years) - 1.0;
    let max_dd = compute_max_drawdown_f64(equity);
    if max_dd.abs() < 1e-15 {
        0.0
    } else {
        cagr / max_dd.abs()
    }
}

/// Compute max drawdown from an equity curve (returns positive fraction).
///
/// Lifted verbatim from `bin/threshold_sweep.rs:310` (R-NR.5).
#[must_use]
pub fn compute_max_drawdown_f64(equity: &[Decimal]) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    if equity.len() < 2 {
        return 0.0;
    }
    let mut peak = equity[0].to_f64().unwrap_or(0.0);
    let mut max_dd = 0.0_f64;
    for e in &equity[1..] {
        let eq = e.to_f64().unwrap_or(0.0);
        if eq > peak {
            peak = eq;
        }
        if peak > 0.0 {
            let dd = (peak - eq) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// Total return from initial to final equity (as a fraction, NOT percentage).
///
/// Lifted verbatim from `bin/threshold_sweep.rs:333` (R-NR.5).
#[must_use]
pub fn compute_total_return(equity: &[Decimal]) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    let n = equity.len();
    if n < 2 {
        return 0.0;
    }
    let initial = equity[0].to_f64().unwrap_or(1.0);
    let final_eq = equity[n - 1].to_f64().unwrap_or(initial);
    if initial <= 0.0 {
        0.0
    } else {
        (final_eq - initial) / initial
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M-DEV-2 — DistributionSummary reducer (ADR-0051 D2)
// ─────────────────────────────────────────────────────────────────────────────

/// Per-metric distribution statistics (mean, std, percentiles, min, max).
///
/// All fields are computed by [`reduce_samples`] using the frozen ADR-0051 D2
/// reduction order: index-order mean/two-pass std + `f64::total_cmp` sort +
/// type-7 linear percentile. NaN values cause a panic (a NaN metric is a
/// strategy or data bug, not a tail — fail loudly).
#[derive(Debug, Clone)]
pub struct MetricDistribution {
    pub mean: f64,
    pub std: f64,
    pub p5: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
    pub min: f64,
    pub max: f64,
}

/// Full distribution summary across an N-path ensemble.
///
/// Built by [`DistributionSummary::from_path_metrics`]. The `max_dd_tail_*`
/// fields are the headline `paper→live` gate numbers (ADR-0051 D4 / feature R2.2).
#[derive(Debug, Clone)]
pub struct DistributionSummary {
    /// Distribution of annualised Sharpe ratio across N paths.
    pub sharpe: MetricDistribution,
    /// Distribution of annualised Sortino ratio across N paths.
    pub sortino: MetricDistribution,
    /// Distribution of Calmar ratio across N paths.
    pub calmar: MetricDistribution,
    /// Distribution of max drawdown (positive fraction) across N paths.
    pub max_drawdown: MetricDistribution,
    /// Distribution of total return (fraction) across N paths.
    pub total_return: MetricDistribution,
    /// `P(final_equity < initial)` — probability of a net loss.
    pub prob_loss: f64,
    /// P(Sharpe > 0) across N paths.
    pub prob_sharpe_gt_0: f64,
    /// P(Sharpe > 1.0) — the `paper→live` gate fraction.
    pub prob_sharpe_gt_1: f64,
    /// Headline gate number: p50 of max-drawdown across paths.
    pub max_dd_tail_p50: f64,
    /// Headline gate number: p95 of max-drawdown across paths (tail risk).
    pub max_dd_tail_p95: f64,
}

/// Per-path metrics collected by the harness for one ensemble path.
///
/// All fields are plain `f64` (the statistical metric layer per ADR-0003/R-NR.3).
/// `final_equity` and `initial_equity` stay as `Decimal` for the probability
/// counts (integer comparisons, not arithmetic, so no f64 drift).
#[derive(Debug, Clone)]
pub struct PathMetrics {
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub max_drawdown: f64,
    pub total_return: f64,
    /// Final equity (Decimal, for P(loss) integer count).
    pub final_equity: Decimal,
    /// Initial equity (Decimal, for P(loss) integer count).
    pub initial_equity: Decimal,
}

impl DistributionSummary {
    /// Build a distribution summary from an ordered slice of path metrics.
    ///
    /// The slice MUST be in path-index order (j=0..N ascending). The reduction
    /// is sequential in that order per ADR-0051 D2 — do NOT sort or rearrange
    /// before calling this function.
    ///
    /// # Panics
    ///
    /// Panics if any metric sample is `NaN` (a NaN Sharpe is a strategy/data
    /// bug, not a tail — fail loudly rather than silently sorting it to an end,
    /// per ADR-0051 D2 mandate).
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::EmptyMetrics`] if `metrics` is empty, or
    /// [`DistributionError::NanValue`] if any per-path metric is `NaN`.
    pub fn from_path_metrics(metrics: &[PathMetrics]) -> Result<Self, DistributionError> {
        if metrics.is_empty() {
            return Err(DistributionError::EmptyMetrics);
        }

        // ── Extract per-metric vectors in index order (j = 0..N) ──────────────
        // ADR-0051 D2: index-order reduction is load-bearing — do NOT parallelize.
        let n = metrics.len();
        let sharpe_vals: Vec<f64> = metrics.iter().map(|m| m.sharpe).collect();
        let sortino_vals: Vec<f64> = metrics.iter().map(|m| m.sortino).collect();
        let calmar_vals: Vec<f64> = metrics.iter().map(|m| m.calmar).collect();
        let max_dd_vals: Vec<f64> = metrics.iter().map(|m| m.max_drawdown).collect();
        let total_ret_vals: Vec<f64> = metrics.iter().map(|m| m.total_return).collect();

        // ── Probability counts (integer arithmetic — platform-independent) ─────
        let loss_count = metrics
            .iter()
            .filter(|m| m.final_equity < m.initial_equity)
            .count();
        let sharpe_gt_0_count = sharpe_vals.iter().filter(|&&s| s > 0.0).count();
        let sharpe_gt_1_count = sharpe_vals.iter().filter(|&&s| s > 1.0).count();

        // ── Reduce each metric vector ─────────────────────────────────────────
        let sharpe_dist = reduce_samples(&sharpe_vals)?;
        let sortino_dist = reduce_samples(&sortino_vals)?;
        let calmar_dist = reduce_samples(&calmar_vals)?;
        let max_dd_dist = reduce_samples(&max_dd_vals)?;
        let total_ret_dist = reduce_samples(&total_ret_vals)?;

        let max_dd_tail_p50 = max_dd_dist.p50;
        let max_dd_tail_p95 = max_dd_dist.p95;

        // ADR-0051 D2 step 5: integer count / N (integer-count is platform-independent;
        // only the final division is f64 — acceptable precision for N ≤ ~5000).
        #[allow(clippy::cast_precision_loss)]
        let prob_loss = loss_count as f64 / n as f64;
        #[allow(clippy::cast_precision_loss)]
        let prob_sharpe_gt_0 = sharpe_gt_0_count as f64 / n as f64;
        #[allow(clippy::cast_precision_loss)]
        let prob_sharpe_gt_1 = sharpe_gt_1_count as f64 / n as f64;

        Ok(Self {
            sharpe: sharpe_dist,
            sortino: sortino_dist,
            calmar: calmar_dist,
            max_drawdown: max_dd_dist,
            total_return: total_ret_dist,
            prob_loss,
            prob_sharpe_gt_0,
            prob_sharpe_gt_1,
            max_dd_tail_p50,
            max_dd_tail_p95,
        })
    }
}

/// Errors from the distribution reducer.
#[derive(Debug, thiserror::Error)]
pub enum DistributionError {
    /// No path metrics provided — cannot compute a distribution over zero paths.
    #[error("cannot build DistributionSummary from empty metrics slice")]
    EmptyMetrics,
    /// A NaN value was found in the samples — this is a strategy/data bug.
    #[error("NaN value in metric sample at index {index}")]
    NanValue { index: usize },
}

/// ADR-0051 D2 reduction: index-order mean, two-pass std, `total_cmp` sort,
/// type-7 linear percentile, NaN-absent assertion.
///
/// # ADR-0051 D2: index-order reduction is load-bearing — do NOT parallelize.
///
/// `f64` addition is non-associative; an unordered parallel fold would flap the
/// anchor body-SHA between runs. The reduction here is sequential in ascending
/// index order, which is deterministic and byte-stable on the canonical box
/// (ADR-0051 D5).
#[allow(clippy::cast_precision_loss)] // N is at most a few thousand — precision is acceptable
fn reduce_samples(samples: &[f64]) -> Result<MetricDistribution, DistributionError> {
    let n = samples.len();
    debug_assert!(n > 0, "reduce_samples called with empty slice — caller bug");

    // ── NaN-absent assertion ──────────────────────────────────────────────────
    // ADR-0051 D2: assert NaN absent before sorting (a NaN Sharpe is a
    // strategy/data bug, not a tail value — fail loudly).
    for (idx, &v) in samples.iter().enumerate() {
        if v.is_nan() {
            return Err(DistributionError::NanValue { index: idx });
        }
    }

    // ── Sequential mean (index order, left fold) ──────────────────────────────
    // ADR-0051 D2: index-order reduction is load-bearing — do NOT parallelize.
    let sum: f64 = samples.iter().copied().fold(0.0_f64, |acc, x| acc + x);
    let mean = sum / n as f64;

    // ── Two-pass population std ───────────────────────────────────────────────
    // ADR-0051 D2 specifies two-pass (compute mean first, then centered-square
    // sum) to avoid catastrophic-cancellation variance between formulae.
    let var_sum: f64 = samples
        .iter()
        .copied()
        .fold(0.0_f64, |acc, x| acc + (x - mean).powi(2));
    let std = (var_sum / n as f64).sqrt();

    // ── Sort with total_cmp (NaN-safe total order) ────────────────────────────
    // ADR-0051 D2: NEVER `partial_cmp` + `unwrap` — undefined on NaN.
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);

    // ── Type-7 linear percentile (the R/NumPy "linear" method) ───────────────
    // h = (N-1)*p/100; value = sorted[floor(h)] + (h - floor(h)) * (sorted[ceil(h)] - sorted[floor(h)])
    let pct = |p: f64| -> f64 { linear_percentile(&sorted, p) };

    let p5 = pct(5.0);
    let p25 = pct(25.0);
    let p50 = pct(50.0);
    let p75 = pct(75.0);
    let p95 = pct(95.0);
    let min = sorted[0];
    let max = sorted[n - 1];

    Ok(MetricDistribution {
        mean,
        std,
        p5,
        p25,
        p50,
        p75,
        p95,
        min,
        max,
    })
}

/// Type-7 linear percentile (R default / `NumPy` `linear` method).
///
/// `sorted` must be sorted ascending (enforced by caller via `total_cmp`).
/// `p` is in [0.0, 100.0].
///
/// `h = (N-1) * p/100`; value = `sorted[floor(h)] + (h - floor(h)) * (sorted[ceil(h)] - sorted[floor(h)])`.
///
/// ADR-0051 D2 freezes this method so p50 is byte-stable run-to-run.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn linear_percentile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let h = (n - 1) as f64 * p / 100.0;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    let frac = h - h.floor();
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + frac * (sorted[hi] - sorted[lo])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests — M-DEV-2 acceptance gate
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::assertions_on_constants
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── Calculator lift smoke tests (R-NR.5) ──────────────────────────────────

    #[test]
    fn sharpe_zero_on_trivial() {
        let eq: Vec<Decimal> = vec![dec!(100)];
        assert_eq!(compute_sharpe_hourly(&eq), 0.0);
    }

    #[test]
    fn sharpe_positive_on_uptrend() {
        let eq: Vec<Decimal> = (0..1000)
            .scan(dec!(100), |s, _| {
                *s += dec!(0.1);
                Some(*s)
            })
            .collect();
        let s = compute_sharpe_hourly(&eq);
        assert!(
            s > 0.0,
            "sharpe should be positive on monotone uptrend, got {s}"
        );
    }

    #[test]
    fn max_dd_zero_on_uptrend() {
        let eq: Vec<Decimal> = (0..100)
            .scan(dec!(100), |s, _| {
                *s += dec!(1);
                Some(*s)
            })
            .collect();
        assert_eq!(compute_max_drawdown_f64(&eq), 0.0);
    }

    #[test]
    fn total_return_fraction() {
        let eq = vec![dec!(100), dec!(110)];
        let tr = compute_total_return(&eq);
        assert!((tr - 0.1).abs() < 1e-12, "expected 0.1, got {tr}");
    }

    // ── DistributionSummary unit tests (M-DEV-2 acceptance) ──────────────────

    /// Hand-verified N=9 case.
    /// samples = [1,2,3,4,5,6,7,8,9]
    /// mean = 5.0, std = sqrt(6.666...) ≈ 2.581988897
    /// sorted = [1..9]
    /// p5:  h=(9-1)*5/100=0.4 → 1 + 0.4*(2-1) = 1.4
    /// p25: h=(9-1)*25/100=2.0 → sorted[2] = 3.0
    /// p50: h=(9-1)*50/100=4.0 → sorted[4] = 5.0
    /// p75: h=(9-1)*75/100=6.0 → sorted[6] = 7.0
    /// p95: h=(9-1)*95/100=7.6 → 8 + 0.6*(9-8) = 8.6
    #[test]
    fn reducer_hand_verified_n9() {
        let samples: Vec<f64> = (1..=9).map(|i| i as f64).collect();
        let dist = reduce_samples(&samples).unwrap();
        assert!((dist.mean - 5.0).abs() < 1e-10, "mean: {}", dist.mean);
        let expected_std = (20.0_f64 / 3.0_f64).sqrt(); // population std = sqrt(60/9)
        assert!((dist.std - expected_std).abs() < 1e-10, "std: {}", dist.std);
        assert!((dist.p5 - 1.4).abs() < 1e-10, "p5: {}", dist.p5);
        assert!((dist.p25 - 3.0).abs() < 1e-10, "p25: {}", dist.p25);
        assert!((dist.p50 - 5.0).abs() < 1e-10, "p50: {}", dist.p50);
        assert!((dist.p75 - 7.0).abs() < 1e-10, "p75: {}", dist.p75);
        assert!((dist.p95 - 8.6).abs() < 1e-10, "p95: {}", dist.p95);
        assert_eq!(dist.min, 1.0);
        assert_eq!(dist.max, 9.0);
    }

    /// N=1 — must not panic, percentiles = the single value.
    #[test]
    fn reducer_n1() {
        let dist = reduce_samples(&[42.0]).unwrap();
        assert_eq!(dist.mean, 42.0);
        assert_eq!(dist.p50, 42.0);
        assert_eq!(dist.min, 42.0);
        assert_eq!(dist.max, 42.0);
    }

    /// NaN causes `DistributionError::NanValue`.
    #[test]
    fn reducer_nan_fails() {
        let samples = vec![1.0, f64::NAN, 3.0];
        let result = reduce_samples(&samples);
        assert!(matches!(
            result,
            Err(DistributionError::NanValue { index: 1 })
        ));
    }

    /// Empty metrics → `DistributionError::EmptyMetrics`.
    #[test]
    fn distribution_summary_empty_fails() {
        let result = DistributionSummary::from_path_metrics(&[]);
        assert!(matches!(result, Err(DistributionError::EmptyMetrics)));
    }

    /// Two identical ensembles produce byte-identical summaries (determinism smoke).
    #[test]
    fn distribution_summary_deterministic() {
        let metrics: Vec<PathMetrics> = (0..20)
            .map(|i| PathMetrics {
                sharpe: i as f64 * 0.1,
                sortino: i as f64 * 0.12,
                calmar: i as f64 * 0.05,
                max_drawdown: i as f64 * 0.02,
                total_return: i as f64 * 0.01,
                final_equity: dec!(100000) + Decimal::from(i * 100),
                initial_equity: dec!(100000),
            })
            .collect();

        let s1 = DistributionSummary::from_path_metrics(&metrics).unwrap();
        let s2 = DistributionSummary::from_path_metrics(&metrics).unwrap();

        // Format at ADR-0051 D3 fixed precision and compare.
        assert_eq!(
            format!("{:.6}", s1.sharpe.p50),
            format!("{:.6}", s2.sharpe.p50),
            "p50 must be deterministic"
        );
        assert_eq!(
            format!("{:.6}", s1.prob_sharpe_gt_1),
            format!("{:.6}", s2.prob_sharpe_gt_1),
            "prob_sharpe_gt_1 must be deterministic"
        );
    }

    /// Probability counts are correct on a hand-verifiable set.
    #[test]
    fn probability_counts_correct() {
        // 4 paths: sharpe in [−1, 0.5, 1.5, 2.0]
        // P(>0) = 3/4 = 0.75,  P(>1) = 2/4 = 0.5
        // final_equity: two paths lose (80k < 100k), two gain (110k, 120k)
        let metrics = vec![
            PathMetrics {
                sharpe: -1.0,
                sortino: 0.0,
                calmar: 0.0,
                max_drawdown: 0.3,
                total_return: -0.2,
                final_equity: dec!(80000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 0.5,
                sortino: 0.6,
                calmar: 0.1,
                max_drawdown: 0.1,
                total_return: 0.1,
                final_equity: dec!(110000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 1.5,
                sortino: 1.8,
                calmar: 0.5,
                max_drawdown: 0.08,
                total_return: 0.2,
                final_equity: dec!(120000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 2.0,
                sortino: 2.4,
                calmar: 0.8,
                max_drawdown: 0.05,
                total_return: 0.5,
                final_equity: dec!(150000),
                initial_equity: dec!(100000),
            },
        ];

        let summary = DistributionSummary::from_path_metrics(&metrics).unwrap();
        assert!(
            (summary.prob_sharpe_gt_0 - 0.75).abs() < 1e-12,
            "P(Sharpe>0)={:.6} expected 0.75",
            summary.prob_sharpe_gt_0
        );
        assert!(
            (summary.prob_sharpe_gt_1 - 0.5).abs() < 1e-12,
            "P(Sharpe>1)={:.6} expected 0.5",
            summary.prob_sharpe_gt_1
        );
        assert!(
            (summary.prob_loss - 0.25).abs() < 1e-12,
            "P(loss)={:.6} expected 0.25",
            summary.prob_loss
        );
    }
}
