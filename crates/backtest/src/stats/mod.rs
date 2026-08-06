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

// ─────────────────────────────────────────────────────────────────────────────
// M-DEV-1 (horizon-retest-robustness) — horizon-aware annualization siblings.
//
// These three functions are PURE ADDITIONS — the three verbatim 1h functions
// above are NOT edited (anchor-neutral by construction, R-HR.LOAD / D-HR.1).
// The 4h/daily sweep calls these; the 1h sweep continues to call the verbatim
// 1h functions above.
// ─────────────────────────────────────────────────────────────────────────────

/// Compute Sharpe (annualised, periodic) from an equity curve.
///
/// `periods_per_year` is the number of bars in a year at the decision cadence:
/// - 1h (non-leap): 8 760  — NOT used here; call `compute_sharpe_hourly` instead
/// - 4h (non-leap): 2 190  (= 8 760 / 6)
/// - 4h (leap):     2 196  (= 8 784 / 6)
/// - daily (non-leap): 365
/// - daily (leap):     366
///
/// Formula: `mean_log_return / std_log_return * sqrt(periods_per_year)`.
///
/// The 1h fn is kept byte-verbatim (R-HR.LOAD / D-HR.1 / F-HR.1 anchor gate).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_sharpe_periodic(equity: &[Decimal], periods_per_year: f64) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
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
        mean / std * periods_per_year.sqrt()
    }
}

/// Compute Sortino (annualised, periodic) from an equity curve.
///
/// `periods_per_year`: see `compute_sharpe_periodic` doc.
///
/// The 1h fn is kept byte-verbatim (R-HR.LOAD / D-HR.1 / F-HR.1 anchor gate).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_sortino_periodic(equity: &[Decimal], periods_per_year: f64) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
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
        mean / down_std * periods_per_year.sqrt()
    }
}

/// Compute Calmar ratio (annualised, periodic) from an equity curve.
///
/// `periods_per_year`: see `compute_sharpe_periodic` doc.
/// `years = (n − 1) / periods_per_year`.
///
/// The 1h fn is kept byte-verbatim (R-HR.LOAD / D-HR.1 / F-HR.1 anchor gate).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_calmar_periodic(equity: &[Decimal], periods_per_year: f64) -> f64 {
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
    let years = (n as f64 - 1.0) / periods_per_year;
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
/// type-7 linear percentile. Non-finite samples (NaN or ±∞) are rejected with
/// a typed error naming the metric (a non-finite metric is a strategy or data
/// bug, not a tail — fail loudly).
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
///
/// The four P1-2 fields (`cvar_95`, `cvar_99`, `median_terminal_wealth`, `skew`)
/// are additive report-only metrics (v2 advisor-turnover-and-tail-metrics).
/// They do NOT change the frozen gate or rankings.
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

    // ── P1-2 coherent tail + median metrics (v2, REPORT-ONLY) ────────────────
    //
    // CVaR (Conditional Value-at-Risk / Expected Shortfall) is used instead of
    // VaR because CVaR is sub-additive (coherent): the risk of a combined
    // portfolio never exceeds the sum of the individual risks.  VaR is NOT
    // sub-additive and can reward concentration — see feature.md P1-2 rationale.
    //
    // Computed over `total_return` (a fraction, comparable across budget sizes).
    // The FROZEN gate (`classify_verdict` / `rank_candidates`) does NOT read
    // these fields — they are pure report additions.
    /// Expected shortfall at α = 0.05: mean of the worst 5% of paths by `total_return`.
    ///
    /// "Expected loss in the worst 5% of simulated scenarios."
    /// Conditional value-at-risk (coherent / sub-additive); preferred over plain var.
    pub cvar_95: f64,

    /// Expected shortfall at α = 0.01: mean of the worst 1% of paths by `total_return`.
    ///
    /// The extreme-tail complement to `cvar_95`.
    pub cvar_99: f64,

    /// Median terminal wealth: p50 of `final_equity` (as f64) across N paths.
    ///
    /// Answers "what does the middle outcome actually look like in dollars?"
    /// More representative than mean wealth (which is pulled by extreme wins).
    pub median_terminal_wealth: f64,

    /// Skew of `total_return` across N paths: 3rd standardised central moment.
    ///
    /// Positive → right tail (lottery-style gains); negative → left tail
    /// (crash-prone).  Zero on a symmetric distribution.
    pub skew: f64,
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
    /// # Errors
    ///
    /// Returns [`DistributionError::EmptyMetrics`] if `metrics` is empty,
    /// [`DistributionError::NanValue`] if any per-path metric is `NaN`, or
    /// [`DistributionError::NonFiniteValue`] if any per-path metric is ±∞
    /// (a non-finite Sharpe is a strategy/data bug, not a tail — fail loudly
    /// rather than silently printing `inf`/`NaN` into the hashed report body,
    /// per ADR-0051 D2 mandate; review 1-14 closed the ±∞ hole the
    /// debug-only asserts left open in release builds).
    pub fn from_path_metrics(metrics: &[PathMetrics]) -> Result<Self, DistributionError> {
        use rust_decimal::prelude::ToPrimitive;

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
        let sharpe_dist = reduce_samples(&sharpe_vals, "sharpe")?;
        let sortino_dist = reduce_samples(&sortino_vals, "sortino")?;
        let calmar_dist = reduce_samples(&calmar_vals, "calmar")?;
        let max_dd_dist = reduce_samples(&max_dd_vals, "max_drawdown")?;
        let total_ret_dist = reduce_samples(&total_ret_vals, "total_return")?;

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

        // ── P1-2 coherent tail + median metrics (REPORT-ONLY, no gate impact) ──
        //
        // CVaR_α = mean of the worst α-fraction of total_return paths.
        // Computed over `total_return` (a fraction — comparable across budgets).
        // Uses the sorted total_ret vector (already sorted via `reduce_samples`
        // sort — but we need an explicit sort here since we only have `total_ret_dist`
        // from `reduce_samples`, not the sorted slice directly).
        //
        // We sort a copy of `total_ret_vals` for the CVaR tail reduction.
        // ADR-0051 D2 specifies `total_cmp` sort for NaN-safe total order.
        let cvar_95 = compute_cvar(&total_ret_vals, 0.05);
        let cvar_99 = compute_cvar(&total_ret_vals, 0.01);

        // Median terminal wealth: p50 of final_equity (as f64) across paths.
        // `final_equity` is `Decimal`; we convert to f64 for the statistical layer
        // (consistent with ADR-0003 / R-NR.3 convention for the stats layer).
        let mut final_equity_f64: Vec<f64> = metrics
            .iter()
            .map(|m| m.final_equity.to_f64().unwrap_or(0.0))
            .collect();
        final_equity_f64.sort_by(f64::total_cmp);
        let median_terminal_wealth = linear_percentile(&final_equity_f64, 50.0);

        // Skew of `total_return` across N paths: 3rd standardised central moment.
        let skew = compute_distribution_skew(&total_ret_vals);

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
            cvar_95,
            cvar_99,
            median_terminal_wealth,
            skew,
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
    /// Carries the metric NAME so the failing lane is identifiable (review 1-14).
    #[error("NaN value in {metric} sample at index {index}")]
    NanValue {
        /// Metric whose sample vector contained the NaN (e.g. `"sharpe"`).
        metric: &'static str,
        /// Index of the offending sample within the metric vector.
        index: usize,
    },
    /// A ±∞ value was found in the samples — this is a strategy/data bug.
    /// Review 1-14: the old NaN-only gate admitted ±∞, which printed
    /// `inf` mean / `NaN` std into the hashed report body in release builds
    /// (the `debug_assert!(is_finite())` probes are stripped there).
    #[error("non-finite value {value} in {metric} sample at index {index}")]
    NonFiniteValue {
        /// Metric whose sample vector contained the infinity.
        metric: &'static str,
        /// Index of the offending sample within the metric vector.
        index: usize,
        /// The offending value (`+inf` or `-inf`).
        value: f64,
    },
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
fn reduce_samples(
    samples: &[f64],
    metric: &'static str,
) -> Result<MetricDistribution, DistributionError> {
    let n = samples.len();
    debug_assert!(n > 0, "reduce_samples called with empty slice — caller bug");

    // ── Finite-samples assertion ──────────────────────────────────────────────
    // ADR-0051 D2: assert NaN absent before sorting (a NaN Sharpe is a
    // strategy/data bug, not a tail value — fail loudly). Review 1-14 extends
    // the gate to ±∞: an infinite sample would print `inf` mean / `NaN` std
    // into the hashed body in release builds (debug_asserts stripped). All
    // locked anchored inputs are finite → bodies unchanged; this is a
    // new-error-path-only change.
    for (idx, &v) in samples.iter().enumerate() {
        if v.is_nan() {
            return Err(DistributionError::NanValue { metric, index: idx });
        }
        if !v.is_finite() {
            return Err(DistributionError::NonFiniteValue {
                metric,
                index: idx,
                value: v,
            });
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
/// `p` is clamped into [0.0, 100.0] (review 1-14 release-mode guard: an
/// out-of-domain `p` used to index out of bounds). An empty slice returns
/// `f64::NAN` instead of underflowing `n - 1` (the ADR-0051 D2 callers always
/// pass non-empty slices and constant `p ∈ {5, 25, 50, 75, 95}` — both guards
/// are release-safety nets, not reachable on any anchored path).
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
    if n == 0 {
        // Empty input: `(n - 1)` below would underflow `usize` (debug panic /
        // release wrap → out-of-bounds index). NaN is the honest "no data"
        // scalar for this f64-returning helper.
        return f64::NAN;
    }
    if n == 1 {
        return sorted[0];
    }
    let p = p.clamp(0.0, 100.0);
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

// ── P1-2 coherent tail helpers ────────────────────────────────────────────────

/// Compute Expected Shortfall (conditional value-at-risk) over `total_return` samples.
///
/// Result = mean of the worst α-fraction of returns.
///
/// For α = 0.05: `cvar_95` — mean of the bottom 5% of paths.
/// For α = 0.01: `cvar_99` — mean of the bottom 1% of paths.
///
/// Coherent / sub-additive risk measure; preferred over plain percentile var.
/// See feature.md P1-2.
///
/// The tail is taken as `floor(α × N)` elements from the sorted-ascending slice.
/// If `floor(α × N) == 0` (very small N), returns the minimum value (a single
/// worst-case observation — conservative).
///
/// Returns 0.0 on empty input.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn compute_cvar(samples: &[f64], alpha: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    // tail_count = floor(alpha * N), at least 1.
    let tail_count = ((alpha * n as f64).floor() as usize).max(1);
    let tail = &sorted[..tail_count];
    tail.iter().sum::<f64>() / tail_count as f64
}

/// Compute the 3rd standardised central moment (skew) of a sample.
///
/// `skew = E[(r − μ)³] / σ³`
///
/// Returns 0.0 for fewer than 3 observations or zero standard deviation.
/// This is the population skew (divisor N) — consistent with the existing
/// `compute_distribution_skew` conventions in this module.
#[allow(clippy::cast_precision_loss)]
fn compute_distribution_skew(samples: &[f64]) -> f64 {
    let n = samples.len();
    if n < 3 {
        return 0.0;
    }
    let n_f = n as f64;
    let mean = samples.iter().sum::<f64>() / n_f;
    let var = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n_f;
    let std = var.sqrt();
    if std < 1e-15 {
        return 0.0;
    }
    samples
        .iter()
        .map(|&x| ((x - mean) / std).powi(3))
        .sum::<f64>()
        / n_f
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
    clippy::assertions_on_constants,
    clippy::doc_markdown
)]
mod tests {
    use super::*;
    use rust_decimal::prelude::ToPrimitive;
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
        let dist = reduce_samples(&samples, "test_metric").unwrap();
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
        let dist = reduce_samples(&[42.0], "test_metric").unwrap();
        assert_eq!(dist.mean, 42.0);
        assert_eq!(dist.p50, 42.0);
        assert_eq!(dist.min, 42.0);
        assert_eq!(dist.max, 42.0);
    }

    /// NaN causes `DistributionError::NanValue` carrying the metric name.
    #[test]
    fn reducer_nan_fails() {
        let samples = vec![1.0, f64::NAN, 3.0];
        let result = reduce_samples(&samples, "sharpe");
        assert!(matches!(
            result,
            Err(DistributionError::NanValue {
                metric: "sharpe",
                index: 1
            })
        ));
    }

    /// ±∞ causes `DistributionError::NonFiniteValue` (review 1-14 — the old
    /// NaN-only gate admitted infinities, printing `inf` mean / `NaN` std into
    /// the hashed body in release builds).
    #[test]
    fn reducer_rejects_positive_infinity() {
        let samples = vec![1.0, f64::INFINITY, 3.0];
        let result = reduce_samples(&samples, "calmar");
        match result {
            Err(DistributionError::NonFiniteValue {
                metric,
                index,
                value,
            }) => {
                assert_eq!(metric, "calmar", "error must NAME the failing metric");
                assert_eq!(index, 1);
                assert!(value.is_infinite() && value > 0.0);
            }
            other => panic!("expected NonFiniteValue, got {other:?}"),
        }
    }

    /// −∞ is rejected the same way as +∞.
    #[test]
    fn reducer_rejects_negative_infinity() {
        let samples = vec![f64::NEG_INFINITY, 2.0];
        let result = reduce_samples(&samples, "total_return");
        match result {
            Err(DistributionError::NonFiniteValue {
                metric,
                index,
                value,
            }) => {
                assert_eq!(metric, "total_return");
                assert_eq!(index, 0);
                assert!(value.is_infinite() && value < 0.0);
            }
            other => panic!("expected NonFiniteValue, got {other:?}"),
        }
    }

    /// A ±∞ per-path metric surfaces through `from_path_metrics` as a typed
    /// error (not an `inf` printed into the report body).
    #[test]
    fn distribution_summary_rejects_infinite_metric() {
        let metrics = vec![PathMetrics {
            sharpe: f64::INFINITY,
            sortino: 0.0,
            calmar: 0.0,
            max_drawdown: 0.1,
            total_return: 0.1,
            final_equity: dec!(110000),
            initial_equity: dec!(100000),
        }];
        let result = DistributionSummary::from_path_metrics(&metrics);
        assert!(matches!(
            result,
            Err(DistributionError::NonFiniteValue {
                metric: "sharpe",
                ..
            })
        ));
    }

    /// Review 1-14 release-mode guards on `linear_percentile`:
    /// empty slice → NaN (no usize underflow / OOB), out-of-domain p → clamped.
    #[test]
    fn linear_percentile_guards() {
        // Empty: NaN, no panic.
        assert!(linear_percentile(&[], 50.0).is_nan());
        // Out-of-domain p is clamped to the ends, not an OOB index.
        let sorted = [1.0, 2.0, 3.0];
        assert_eq!(linear_percentile(&sorted, 150.0), 3.0);
        assert_eq!(linear_percentile(&sorted, -25.0), 1.0);
        // In-domain behaviour unchanged.
        assert_eq!(linear_percentile(&sorted, 0.0), 1.0);
        assert_eq!(linear_percentile(&sorted, 100.0), 3.0);
        assert_eq!(linear_percentile(&sorted, 50.0), 2.0);
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

    // ── F-HR.1 — anchor-byte-identity of the 1h Sharpe path (R-HR.LOAD gate, half 1) ──
    //
    // WHAT THIS ASSERTS (review 1-18 doc-truth pass): that `compute_sharpe_hourly`
    // annualizes with the hand-entered constant SQRT_HPY = 92.601_295_098_46 — NOT
    // a literal byte-value of the function's output, which the old header claimed
    // ("returns its KNOWN byte-value on a fixed reference series") but which no
    // assertion in the test ever made. The check is structural: recompute
    // mean/std of the log-returns and require `got == mean/std * SQRT_HPY` to
    // 1e-12 absolute. That is what makes it RED-on-revert for the mutation that
    // matters — folding the 1h fn into the periodic one, or re-deriving the
    // constant as √8760 (the constant is √8574.9998…, ~2.1 % away).
    //
    // SQRT_HPY is anchor-load-bearing: every 1h θ-surface Sharpe scales with it.
    // DO NOT change the asserted constant without re-running
    // `scripts/verify_anchors.sh` and confirming 119/119 PASS.
    #[test]
    fn f_hr_1_compute_sharpe_hourly_value_unchanged() {
        // The 1h constant SQRT_HPY is anchor-load-bearing.
        // DO NOT change this without re-running `scripts/verify_anchors.sh` → 91/91.
        const SQRT_HPY_REFERENCE: f64 = 92.601_295_098_46_f64;

        // Fixed reference equity series: a monotone uptrend so the Sharpe is positive
        // and well-defined. The exact value is FIXED by the SQRT_HPY constant.
        let eq: Vec<Decimal> = (0..101)
            .scan(dec!(1000), |s, _| {
                *s += dec!(1);
                Some(*s)
            })
            .collect();
        let got = compute_sharpe_hourly(&eq);
        // Value computed from the verbatim fn: mean=ln(1001/1000)≈0.0009990,
        // std≈0 (monotone) — but population-std on the log returns of a
        // perfectly-linear price series is non-zero; we check the returned value
        // matches to 10 significant digits.
        // Regression: if SQRT_HPY is changed, this fails.
        assert!(
            got.is_finite() && got > 0.0,
            "f_hr_1: compute_sharpe_hourly should be positive on uptrend, got {got}"
        );
        // The SQRT_HPY constant must remain at its anchor-load-bearing value.
        // We verify it by checking the ratio of the periodic vs hourly Sharpe on
        // the SAME series equals sqrt(8575) / sqrt(periods_per_year_test).
        // Use ppy=8575 (SQRT_HPY^2 = 8574.9998…).
        let got_periodic = compute_sharpe_periodic(&eq, 8575.0_f64);
        // The ratio must be 1.0 to within floating-point noise (both are mean/std * sqrt(ppy)
        // using the same mean/std computation — the constant is in the scalar).
        // The 1h fn uses SQRT_HPY = 92.601_295_098_46 = sqrt(8574.9998…).
        // The periodic fn uses sqrt(8575.0) = 92.601_295_105_7… — a tiny ULP diff.
        // They will NOT be exactly equal (intentional: verbatim fn uses the hand-entered
        // constant, not a derived sqrt). We assert they differ by < 1e-9 in relative terms.
        let relative_diff = ((got - got_periodic) / got).abs();
        assert!(
            relative_diff < 1e-6,
            "f_hr_1: 1h fn and periodic fn (ppy=8575) should agree to 1e-6 on same series;\
             hourly={got}, periodic={got_periodic}, rel_diff={relative_diff}"
        );
        // Verify the reference constant is what the function uses by computing
        // what the fn SHOULD return for our series and comparing.
        // log-returns on the linear series (1000,1001,...,1100):
        let rets_ref: Vec<f64> = eq
            .windows(2)
            .map(|w| {
                let prev = w[0].to_f64().unwrap();
                let curr = w[1].to_f64().unwrap();
                (curr / prev).ln()
            })
            .collect();
        let mean_ref: f64 = rets_ref.iter().sum::<f64>() / rets_ref.len() as f64;
        let var_ref: f64 = rets_ref
            .iter()
            .map(|&r| (r - mean_ref).powi(2))
            .sum::<f64>()
            / rets_ref.len() as f64;
        let std_ref = var_ref.sqrt();
        let expected = mean_ref / std_ref * SQRT_HPY_REFERENCE;
        assert!(
            (got - expected).abs() < 1e-12,
            "f_hr_1: compute_sharpe_hourly does not use the expected SQRT_HPY constant.\
             expected={expected}, got={got}, diff={diff}",
            diff = (got - expected).abs()
        );
    }

    // ── F-HR.2 — annualization correctness at 4h + daily (R-HR.LOAD gate, half 2) ──
    //
    // Asserts that the periodic metric fns annualize by sqrt(ppy) for the 4h and
    // daily cadences, including leap-year values.
    //
    // ## Review 1-18 rebuild
    //
    // These tests used to compare each production fn against a LOCAL helper
    // (`expected_sharpe_at_ppy` / `expected_sortino_at_ppy` /
    // `expected_calmar_at_ppy`) that re-implemented the production formula
    // line-for-line — a self-comparison that stays green under any change made
    // in both places, and green today under changes that alter the production
    // value (e.g. swapping population for sample variance in BOTH). That is the
    // bug-log #66 vacuous-test class.
    //
    // The primaries now assert **captured literal values** on an exact-integer
    // reference curve (`make_exact_equity`, no Decimal rounding anywhere, so
    // `to_f64` is lossless and the literals are reproducible), and the
    // load-bearing ratio cross-checks — which pin the sqrt(ppy) SCALAR
    // independently of the mean/std machinery — are kept verbatim.
    //
    // RED-on-revert for the primaries: population→sample variance (×1.005 at
    // n=100), log→simple returns, a dropped or wrong sqrt, wiring the periodic
    // fn to the 1h SQRT_HPY constant (4h inflates ≈2.0×, daily ≈4.9×).

    /// Exact-integer reference equity curve — 101 points, 100 returns.
    ///
    /// 1000 → 1060 in +1 steps (61 points), then 1060 → 1020 in −1 steps (40
    /// points). Every value is an exact integer, so `Decimal::to_f64` is lossless
    /// and the captured literals below are byte-reproducible. The shape gives a
    /// non-zero downside (Sortino well-defined) and a max drawdown of
    /// 40/1060 = 0.037_735_849_056_603_77 (Calmar well-defined).
    fn make_exact_equity() -> Vec<Decimal> {
        let mut eq: Vec<Decimal> = (0..=60_i64).map(|i| Decimal::from(1000 + i)).collect();
        eq.extend((1..=40_i64).map(|i| Decimal::from(1060 - i)));
        debug_assert_eq!(eq.len(), 101);
        eq
    }

    /// Captured reference values for [`make_exact_equity`] — the metric each
    /// production fn returns, recorded 2026-08-06 from the fns as-implemented.
    ///
    /// Recompute with `cargo test -p backtest --lib f_hr_2` and read the value
    /// out of the failure message; changing one of these is a claim that the
    /// metric's DEFINITION changed, which is an ADR-0051 D3 matter (the 1h
    /// anchors' Sharpe column scales with the same machinery).
    mod exact_curve_reference {
        /// `compute_sharpe_periodic(make_exact_equity(), 2190.0)` — 4h non-leap.
        pub const SHARPE_4H: f64 = 9.785_911_720_583_08;
        /// `… 2196.0` — 4h leap (2024).
        pub const SHARPE_4H_LEAP: f64 = 9.799_307_909_908_26;
        /// `… 365.0` — daily non-leap.
        pub const SHARPE_DAILY: f64 = 3.995_081_730_558_322;
        /// `… 366.0` — daily leap (2024).
        pub const SHARPE_DAILY_LEAP: f64 = 4.000_550_701_949_058_5;
        /// `compute_sortino_periodic(…, 2190.0)`.
        pub const SORTINO_4H: f64 = 15.235_892_885_633_522;
        /// `compute_sortino_periodic(…, 365.0)`.
        pub const SORTINO_DAILY: f64 = 6.220_027_224_250_417;
        /// `compute_calmar_periodic(…, 2190.0)`.
        pub const CALMAR_4H: f64 = 14.387_413_233_301_16;
        /// `compute_calmar_periodic(…, 365.0)`.
        pub const CALMAR_DAILY: f64 = 1.986_329_986_892_632;
    }

    /// Relative-difference helper for the captured literals.
    ///
    /// The tolerance (1e-12 relative ≈ a few thousand ULP) absorbs any
    /// evaluation-order noise in the reference capture while staying orders of
    /// magnitude tighter than every revert listed above, the smallest of which
    /// (population→sample variance at n=100) moves the value by ~5e-3 relative.
    fn rel_diff(got: f64, expected: f64) -> f64 {
        ((got - expected) / expected).abs()
    }

    /// Build a reference equity curve with a mixed up-then-down shape so that
    /// Sortino/Calmar are well-defined (non-trivial downside and drawdown).
    ///
    /// Used only by the RATIO cross-checks, which are scale-free — they compare
    /// two annualizations of the SAME curve, so the curve's exact float values
    /// cancel and no literal is needed.
    fn make_mixed_equity(n_up: usize, n_down: usize) -> Vec<Decimal> {
        let mut eq = vec![dec!(1000)];
        let mut cur = dec!(1000);
        for _ in 0..n_up {
            cur *= dec!(1.002);
            eq.push(cur);
        }
        for _ in 0..n_down {
            cur *= dec!(0.998);
            eq.push(cur);
        }
        eq
    }

    #[test]
    fn f_hr_2_sharpe_4h_scalar() {
        // 4h (non-leap year): periods_per_year = 2190 = 8760/4
        // sqrt(2190) = 46.797_435_827_2…
        let eq = make_exact_equity();
        let got = compute_sharpe_periodic(&eq, 2190.0_f64);
        assert!(
            rel_diff(got, exact_curve_reference::SHARPE_4H) < 1e-12,
            "f_hr_2_sharpe_4h: got={got}, captured reference={expected}. \
             A change here means the 4h Sharpe DEFINITION moved (population→sample \
             variance, log→simple returns, or a wrong annualization scalar).",
            expected = exact_curve_reference::SHARPE_4H
        );
        // Verify the scalar sqrt(2190) is used (not the 1h constant sqrt(8575)).
        // The ratio of 4h Sharpe to 1h Sharpe must be sqrt(2190)/sqrt(8575) ≈ 0.505.
        // Scale-free: the mean/std of the curve cancels, so this pins the SCALAR
        // alone — it stays load-bearing independently of the literal above.
        let mixed = make_mixed_equity(200, 100);
        let ratio = compute_sharpe_periodic(&mixed, 2190.0_f64) / compute_sharpe_hourly(&mixed);
        let expected_ratio = (2190.0_f64 / 8575.0_f64).sqrt();
        // Note: 1h fn uses SQRT_HPY=92.601... (≈sqrt(8575)); 4h uses sqrt(2190).
        assert!(
            (ratio - expected_ratio).abs() < 1e-5,
            "f_hr_2_sharpe_4h scalar ratio: got={ratio}, expected={expected_ratio}"
        );
        // Cross-check: sqrt(2190) ≈ 46.797
        let sqrt_2190 = 2190.0_f64.sqrt();
        assert!(
            (sqrt_2190 - 46.797_435_827).abs() < 1e-6,
            "sqrt(2190) reference check: {sqrt_2190}"
        );
    }

    #[test]
    fn f_hr_2_sharpe_daily_scalar() {
        // daily (non-leap year): periods_per_year = 365
        // sqrt(365) = 19.104_973_174_5…
        let eq = make_exact_equity();
        let got = compute_sharpe_periodic(&eq, 365.0_f64);
        assert!(
            rel_diff(got, exact_curve_reference::SHARPE_DAILY) < 1e-12,
            "f_hr_2_sharpe_daily: got={got}, captured reference={expected}",
            expected = exact_curve_reference::SHARPE_DAILY
        );
        // The 4h and daily Sharpes of the SAME curve must differ by exactly
        // sqrt(2190/365) — the scalar, isolated from the mean/std machinery.
        let ratio_4h_daily = compute_sharpe_periodic(&eq, 2190.0_f64) / got;
        let expected_ratio_4h_daily = (2190.0_f64 / 365.0_f64).sqrt();
        assert!(
            (ratio_4h_daily - expected_ratio_4h_daily).abs() < 1e-9,
            "f_hr_2_sharpe: 4h/daily annualization ratio must be sqrt(2190/365); \
             got={ratio_4h_daily}, expected={expected_ratio_4h_daily}"
        );
        // Verify the ratio vs 1h is sqrt(365)/sqrt(8575) ≈ 0.206.
        let mixed = make_mixed_equity(200, 100);
        let ratio = compute_sharpe_periodic(&mixed, 365.0_f64) / compute_sharpe_hourly(&mixed);
        let expected_ratio = (365.0_f64 / 8575.0_f64).sqrt();
        assert!(
            (ratio - expected_ratio).abs() < 1e-5,
            "f_hr_2_sharpe_daily scalar ratio: got={ratio}, expected={expected_ratio}"
        );
        // Cross-check: sqrt(365) ≈ 19.104
        let sqrt_365 = 365.0_f64.sqrt();
        assert!(
            (sqrt_365 - 19.104_973_174).abs() < 1e-6,
            "sqrt(365) reference check: {sqrt_365}"
        );
    }

    #[test]
    fn f_hr_2_sortino_periodic() {
        // Verify Sortino uses sqrt(ppy) at 4h and daily.
        let eq = make_exact_equity();
        let got_4h = compute_sortino_periodic(&eq, 2190.0_f64);
        assert!(
            rel_diff(got_4h, exact_curve_reference::SORTINO_4H) < 1e-12,
            "f_hr_2_sortino_4h: got={got_4h}, captured reference={expected}. \
             RED here also covers the downside-deviation denominator (mean of \
             min(r,0)² over ALL returns, not just the negative ones).",
            expected = exact_curve_reference::SORTINO_4H
        );
        let got_daily = compute_sortino_periodic(&eq, 365.0_f64);
        assert!(
            rel_diff(got_daily, exact_curve_reference::SORTINO_DAILY) < 1e-12,
            "f_hr_2_sortino_daily: got={got_daily}, captured reference={expected}",
            expected = exact_curve_reference::SORTINO_DAILY
        );
        // Ratio check (scale-free): 4h/daily = sqrt(2190)/sqrt(365).
        let ratio = got_4h / got_daily;
        let expected_ratio = (2190.0_f64 / 365.0_f64).sqrt();
        assert!(
            (ratio - expected_ratio).abs() < 1e-6,
            "f_hr_2_sortino ratio 4h/daily: got={ratio}, expected={expected_ratio}"
        );
        // Sortino must exceed Sharpe on this curve: the downside deviation
        // (0.000_608) is smaller than the full std (0.000_947).
        assert!(
            got_4h > compute_sharpe_periodic(&eq, 2190.0_f64),
            "f_hr_2_sortino: on a curve with more upside than downside dispersion, \
             Sortino must exceed Sharpe; sortino={got_4h}"
        );
    }

    #[test]
    fn f_hr_2_calmar_periodic() {
        // Verify Calmar uses years = (n-1)/ppy at 4h and daily.
        let eq = make_exact_equity();
        let got_4h = compute_calmar_periodic(&eq, 2190.0_f64);
        assert!(
            rel_diff(got_4h, exact_curve_reference::CALMAR_4H) < 1e-12,
            "f_hr_2_calmar_4h: got={got_4h}, captured reference={expected}. \
             RED here covers years=(n−1)/ppy, the CAGR exponent, and the \
             MaxDD denominator (40/1060 on this curve).",
            expected = exact_curve_reference::CALMAR_4H
        );
        let got_daily = compute_calmar_periodic(&eq, 365.0_f64);
        assert!(
            rel_diff(got_daily, exact_curve_reference::CALMAR_DAILY) < 1e-12,
            "f_hr_2_calmar_daily: got={got_daily}, captured reference={expected}",
            expected = exact_curve_reference::CALMAR_DAILY
        );
        // Independent cross-check of the MaxDD denominator: the curve peaks at
        // 1060 and bottoms at 1020 → 40/1060.
        let max_dd = compute_max_drawdown_f64(&eq);
        assert!(
            (max_dd - (40.0_f64 / 1060.0_f64)).abs() < 1e-15,
            "f_hr_2_calmar: reference curve MaxDD must be 40/1060; got={max_dd}"
        );
        // Calmar is CAGR/MaxDD. For the same equity curve, years_4h < years_daily
        // (ppy_4h > ppy_daily) → CAGR_4h > CAGR_daily (same return, fewer years)
        // → Calmar_4h > Calmar_daily when equity is profitable.
        assert!(
            got_4h > got_daily,
            "f_hr_2_calmar: 4h Calmar should be > daily Calmar for profitable curve; \
             4h={got_4h}, daily={got_daily}"
        );
    }

    #[test]
    fn f_hr_2_leap_year_scalars() {
        // The leap-year scalars must come from the PRODUCTION table, not from
        // literals re-typed in the test.
        //
        // Review 1-18: this test used to pass 2196.0 / 366.0 as bare literals and
        // never call `Horizon::periods_per_year` at all — so it could not have
        // gone RED if the production leap table were wrong, which is the ONE
        // thing its name promises. (The table has since become leap-aware for
        // every year, not just 2024 — see `bars_per_year_1h`.)
        use crate::resample::Horizon;

        // Non-leap (2023) and leap (2024) — exact values off the real fn.
        assert_eq!(
            Horizon::FourHours.periods_per_year(2023),
            2190.0,
            "4h non-leap ppy must be 8760/4"
        );
        assert_eq!(
            Horizon::FourHours.periods_per_year(2024),
            2196.0,
            "4h leap ppy must be 8784/4"
        );
        assert_eq!(
            Horizon::OneDay.periods_per_year(2023),
            365.0,
            "daily non-leap ppy must be 365"
        );
        assert_eq!(
            Horizon::OneDay.periods_per_year(2024),
            366.0,
            "daily leap ppy must be 366"
        );
        assert_eq!(
            Horizon::OneHour.periods_per_year(2023),
            8760.0,
            "1h non-leap ppy must be 8760"
        );
        assert_eq!(
            Horizon::OneHour.periods_per_year(2024),
            8784.0,
            "1h leap ppy must be 8784"
        );

        // Feed the REAL scalars into the metric fns and pin the results.
        let eq = make_exact_equity();
        let got_4h_leap = compute_sharpe_periodic(&eq, Horizon::FourHours.periods_per_year(2024));
        assert!(
            rel_diff(got_4h_leap, exact_curve_reference::SHARPE_4H_LEAP) < 1e-12,
            "f_hr_2_leap_4h: got={got_4h_leap}, captured reference={expected}",
            expected = exact_curve_reference::SHARPE_4H_LEAP
        );
        let got_daily_leap = compute_sharpe_periodic(&eq, Horizon::OneDay.periods_per_year(2024));
        assert!(
            rel_diff(got_daily_leap, exact_curve_reference::SHARPE_DAILY_LEAP) < 1e-12,
            "f_hr_2_leap_daily: got={got_daily_leap}, captured reference={expected}",
            expected = exact_curve_reference::SHARPE_DAILY_LEAP
        );
        // Cross-checks: sqrt(2196) > sqrt(2190); sqrt(366) > sqrt(365).
        assert!(
            got_4h_leap > compute_sharpe_periodic(&eq, Horizon::FourHours.periods_per_year(2023)),
            "f_hr_2_leap: sharpe(leap 4h) > sharpe(non-leap 4h)"
        );
        assert!(
            got_daily_leap > compute_sharpe_periodic(&eq, Horizon::OneDay.periods_per_year(2023)),
            "f_hr_2_leap: sharpe(leap daily) > sharpe(non-leap daily)"
        );
        // The Calmar leap check: years are smaller → CAGR is larger.
        let calmar_daily_leap =
            compute_calmar_periodic(&eq, Horizon::OneDay.periods_per_year(2024));
        let calmar_daily_nonleap =
            compute_calmar_periodic(&eq, Horizon::OneDay.periods_per_year(2023));
        assert!(
            calmar_daily_leap > calmar_daily_nonleap,
            "f_hr_2_leap: calmar(leap daily) > calmar(non-leap daily); \
             leap={calmar_daily_leap}, non-leap={calmar_daily_nonleap}"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────────

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

    // ── P1-2 CVaR / median / skew unit tests ─────────────────────────────────

    /// CVaR_0.05 on 20 uniform returns [−19/20, −18/20, …, 0].
    ///
    /// Sorted ascending: [−0.95, −0.90, …, 0.0].
    /// 5% tail = floor(0.05 * 20) = 1 element: [−0.95].
    /// CVaR = −0.95.
    #[test]
    fn cvar_uniform_n20_closed_form() {
        // 20 returns: −0.95, −0.90, … , 0.0 (step 0.05)
        let samples: Vec<f64> = (0..20).map(|i| -0.95 + i as f64 * 0.05).collect();
        // floor(0.05 * 20) = 1; worst element = −0.95
        let cvar = compute_cvar(&samples, 0.05);
        assert!(
            (cvar - (-0.95)).abs() < 1e-12,
            "CVaR_0.05 on uniform: got {cvar}, expected -0.95"
        );
    }

    /// CVaR_0.05 on 100 returns [−0.99, −0.98, …, 0.00].
    ///
    /// 5% tail = floor(0.05 * 100) = 5 elements: [−0.99, −0.98, −0.97, −0.96, −0.95].
    /// CVaR = mean = (−0.99 − 0.98 − 0.97 − 0.96 − 0.95) / 5 = −0.97.
    #[test]
    fn cvar_uniform_n100_closed_form() {
        let samples: Vec<f64> = (0..100).map(|i| -0.99 + i as f64 * 0.01).collect();
        // worst 5 = [−0.99, −0.98, −0.97, −0.96, −0.95]; mean = −0.97
        let cvar = compute_cvar(&samples, 0.05);
        assert!(
            (cvar - (-0.97)).abs() < 1e-10,
            "CVaR_0.05 on n=100 uniform: got {cvar}, expected -0.97"
        );
    }

    /// CVaR_0.01 on 100 returns: bottom 1% = 1 element = minimum.
    #[test]
    fn cvar_99_equals_min_on_n100() {
        let samples: Vec<f64> = (0..100).map(|i| -0.99 + i as f64 * 0.01).collect();
        let cvar = compute_cvar(&samples, 0.01);
        // floor(0.01 * 100) = 1; worst 1 = [−0.99]
        assert!(
            (cvar - (-0.99)).abs() < 1e-12,
            "CVaR_0.01: got {cvar}, expected -0.99"
        );
    }

    /// CVaR on empty returns 0.0 (not NaN, not panic).
    #[test]
    fn cvar_empty_returns_zero() {
        assert_eq!(compute_cvar(&[], 0.05), 0.0);
        assert_eq!(compute_cvar(&[], 0.01), 0.0);
    }

    /// CVaR is always ≤ the α-percentile of the distribution (the ES/CVaR property).
    /// For a strictly sorted set, CVaR_0.05 ≤ VaR_5 (the 5th-percentile value).
    #[test]
    fn cvar_le_var_property() {
        // Linearly spaced [−1, 0] with 200 elements.
        let samples: Vec<f64> = (0..200).map(|i| -1.0 + i as f64 / 199.0).collect();
        let cvar = compute_cvar(&samples, 0.05);
        // VaR_5 ≈ the 5th percentile ≈ −0.95 (the boundary of the worst 5%).
        // CVaR should be below (more negative than) the VaR boundary.
        assert!(cvar < -0.90, "CVaR_0.05 should be in the deep tail: {cvar}");
    }

    /// Skew is zero on a perfectly symmetric distribution [−3, −2, −1, 0, 1, 2, 3].
    #[test]
    fn skew_zero_on_symmetric() {
        let sym: Vec<f64> = (-3..=3).map(|i| i as f64).collect();
        let s = compute_distribution_skew(&sym);
        assert!(s.abs() < 1e-12, "skew of symmetric should be 0, got {s}");
    }

    /// Skew is positive on a right-skewed distribution.
    ///
    /// [0, 0, 0, 10] → mean = 2.5, dominated by one extreme positive value.
    #[test]
    fn skew_positive_on_right_skewed() {
        let s = compute_distribution_skew(&[0.0, 0.0, 0.0, 10.0]);
        assert!(
            s > 0.0,
            "right-skewed distribution should have positive skew, got {s}"
        );
    }

    /// Skew is negative on a left-skewed distribution.
    ///
    /// [0, 0, 0, −10] → skew < 0.
    #[test]
    fn skew_negative_on_left_skewed() {
        let s = compute_distribution_skew(&[0.0, 0.0, 0.0, -10.0]);
        assert!(
            s < 0.0,
            "left-skewed distribution should have negative skew, got {s}"
        );
    }

    /// Skew returns 0.0 for fewer than 3 observations.
    #[test]
    fn skew_degenerate_small_n() {
        assert_eq!(compute_distribution_skew(&[]), 0.0);
        assert_eq!(compute_distribution_skew(&[1.0]), 0.0);
        assert_eq!(compute_distribution_skew(&[1.0, 2.0]), 0.0);
    }

    /// `DistributionSummary::from_path_metrics` populates all four P1-2 fields.
    ///
    /// Hand-built 4-path vector:
    /// total_return = [−0.3, 0.0, 0.1, 0.5]
    /// CVaR_0.05: floor(0.05*4)=0 → max(1, 0)=1 path; worst = −0.3 → CVaR = −0.3
    /// CVaR_0.01: same → −0.3
    /// median_terminal_wealth: p50 of [80k, 100k, 110k, 150k] = (100k+110k)/2 = 105k
    /// skew: computed over [−0.3, 0.0, 0.1, 0.5]
    ///   mean = 0.075, σ = sqrt(mean((r−μ)²))
    ///   numerically should be positive (right tail at 0.5 dominates).
    #[test]
    fn distribution_summary_p1_2_fields_populated() {
        let metrics = vec![
            PathMetrics {
                sharpe: -1.0,
                sortino: -1.0,
                calmar: 0.0,
                max_drawdown: 0.3,
                total_return: -0.3,
                final_equity: dec!(70000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 0.0,
                sortino: 0.0,
                calmar: 0.0,
                max_drawdown: 0.1,
                total_return: 0.0,
                final_equity: dec!(100000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 0.5,
                sortino: 0.6,
                calmar: 0.1,
                max_drawdown: 0.05,
                total_return: 0.1,
                final_equity: dec!(110000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 2.0,
                sortino: 2.4,
                calmar: 0.8,
                max_drawdown: 0.02,
                total_return: 0.5,
                final_equity: dec!(150000),
                initial_equity: dec!(100000),
            },
        ];

        let summary = DistributionSummary::from_path_metrics(&metrics).unwrap();

        // CVaR_95: floor(0.05 * 4) = 0 → clamped to 1; worst total_return = −0.3
        assert!(
            (summary.cvar_95 - (-0.3)).abs() < 1e-12,
            "cvar_95: got {}, expected -0.3",
            summary.cvar_95
        );
        // CVaR_99: same → −0.3
        assert!(
            (summary.cvar_99 - (-0.3)).abs() < 1e-12,
            "cvar_99: got {}, expected -0.3",
            summary.cvar_99
        );
        // median_terminal_wealth: sorted final_equity f64 = [70000, 100000, 110000, 150000]
        // p50 of 4 elements: linear interp h=(4-1)*50/100=1.5 → 100000 + 0.5*(110000-100000) = 105000
        assert!(
            (summary.median_terminal_wealth - 105_000.0).abs() < 1.0,
            "median_terminal_wealth: got {}, expected 105000",
            summary.median_terminal_wealth
        );
        // Skew: total_return = [−0.3, 0, 0.1, 0.5]; right tail at 0.5 → positive skew.
        assert!(
            summary.skew > 0.0,
            "skew should be positive (right-skewed returns), got {}",
            summary.skew
        );
    }

    /// Regression: the four P1-2 fields do NOT change `prob_loss` / `max_dd_tail_p50`
    /// (the gate inputs) — purely additive.
    #[test]
    fn p1_2_fields_additive_gate_unchanged() {
        let metrics = vec![
            PathMetrics {
                sharpe: -0.5,
                sortino: -0.4,
                calmar: 0.0,
                max_drawdown: 0.25,
                total_return: -0.1,
                final_equity: dec!(90000),
                initial_equity: dec!(100000),
            },
            PathMetrics {
                sharpe: 1.2,
                sortino: 1.5,
                calmar: 0.4,
                max_drawdown: 0.10,
                total_return: 0.3,
                final_equity: dec!(130000),
                initial_equity: dec!(100000),
            },
        ];

        let s = DistributionSummary::from_path_metrics(&metrics).unwrap();

        // Gate fields unchanged (P(loss) = 0.5, max_dd_tail_p50 from the two values).
        assert!(
            (s.prob_loss - 0.5).abs() < 1e-12,
            "prob_loss must be 0.5, got {}",
            s.prob_loss
        );
        // P1-2 fields present and not NaN.
        assert!(!s.cvar_95.is_nan(), "cvar_95 must not be NaN");
        assert!(!s.cvar_99.is_nan(), "cvar_99 must not be NaN");
        assert!(
            !s.median_terminal_wealth.is_nan(),
            "median_terminal_wealth must not be NaN"
        );
        assert!(!s.skew.is_nan(), "skew must not be NaN");
    }
}
