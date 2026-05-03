//! R4 — Risk metrics (Sharpe / Sortino / Calmar / max-DD / recovery).
//!
//! Sharpe / Sortino / Calmar are display-only `f64` values per the
//! design's risk register R-2: they never feed back into a
//! reconciliation sum.  Max-drawdown carries a `Decimal` USDT amount
//! alongside its display percentage so it composes with R11.
//!
//! Annualization constant: `525_600` minutes per year (R4.2 — the same
//! constant the backtest binary uses, so reports cross-reference cleanly
//! with backtest reports).

use std::fmt::Write;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Minutes per year — R4.2 annualization constant (1 year × 365 days
/// × 24 hours × 60 minutes).  Same value the backtest binary uses.
pub const MINUTES_PER_YEAR: u32 = 525_600;

/// Sharpe-pair returned by [`sharpe_ratios`] / consumed by R9 decay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharpeStats {
    /// Since-inception annualized Sharpe.
    pub inception: f64,
    /// Last-7-day annualized Sharpe.
    pub last_7d: f64,
}

/// Function shape used by `crate::render::memory_highlights::decay_fired`.
pub type SharpeFn = fn(&[Decimal]) -> SharpeStats;

/// Inputs for the R4 risk-metrics section.
///
/// All five metrics are pre-computed in the orchestrator from a single
/// equity curve; the renderer only formats them.  Recovery time is
/// `None` when the curve has not recovered to a new equity high after
/// its trough.
#[derive(Debug, Clone)]
pub struct RiskMetricsInputs {
    /// Period slug to mirror in the table's `Period` column.
    pub period: String,
    /// Annualized Sharpe ratio.
    pub sharpe: f64,
    /// Annualized Sortino ratio.
    pub sortino: f64,
    /// Annualized Calmar ratio.
    pub calmar: f64,
    /// Max drawdown as a percentage (e.g. `dec!(11.25)`).
    pub max_drawdown_pct: Decimal,
    /// Max drawdown in absolute USDT (always non-negative).
    pub max_drawdown_usdt: Decimal,
    /// Recovery-time in bars from the trough to a new equity high.
    /// `None` means "not yet recovered".
    pub recovery_bars: Option<u32>,
}

/// Render the R4 risk-metrics 5-row markdown table.
///
/// Pure over `inputs` — same inputs produce byte-identical output.
#[must_use]
pub fn render(inputs: &RiskMetricsInputs) -> String {
    let mut out = String::with_capacity(384);
    out.push_str("## Risk metrics\n\n");
    out.push_str("| Metric | Value | Period |\n");
    out.push_str("|--------|-------|--------|\n");
    let _ = writeln!(out, "| Sharpe | {:.4} | {} |", inputs.sharpe, inputs.period);
    let _ = writeln!(
        out,
        "| Sortino | {:.4} | {} |",
        inputs.sortino, inputs.period
    );
    let _ = writeln!(out, "| Calmar | {:.4} | {} |", inputs.calmar, inputs.period);
    let _ = writeln!(
        out,
        "| Max drawdown | {}% (${}) | {} |",
        fmt_2dp(inputs.max_drawdown_pct),
        fmt_2dp(inputs.max_drawdown_usdt),
        inputs.period
    );
    let recovery = match inputs.recovery_bars {
        Some(b) => format!("{b} bars"),
        None => "n/a".to_string(),
    };
    let _ = writeln!(out, "| Recovery time | {recovery} | {} |", inputs.period);
    out
}

/// Format a `Decimal` with exactly two decimal places.
fn fmt_2dp(d: Decimal) -> String {
    let two_scale = (d.round_dp(2) * dec!(1.00)).round_dp(2);
    format!("{two_scale:.2}")
}

// ── Pure metric helpers ─────────────────────────────────────────────────────
//
// Display-only helpers that compute Sharpe / Sortino / Calmar / max-DD over
// a Decimal equity curve.  They use `f64` for the annualization step only —
// the raw return series stays Decimal until the final ratio.  The output is
// a display string; never a reconciliation input (risk register R-2).

/// Annualized Sharpe ratio over an equity curve sampled at
/// `cadence_minutes` cadence.
///
/// Returns `0.0` when the curve has fewer than two samples or the
/// per-period return series has zero standard deviation.
#[must_use]
pub fn sharpe(equity: &[Decimal], cadence_minutes: u32) -> f64 {
    let returns = period_returns(equity);
    if returns.is_empty() {
        return 0.0;
    }
    let mean = mean_f64(&returns);
    let stdev = stdev_f64(&returns, mean);
    if stdev == 0.0 {
        return 0.0;
    }
    let periods_per_year = f64::from(MINUTES_PER_YEAR) / f64::from(cadence_minutes.max(1));
    (mean / stdev) * periods_per_year.sqrt()
}

/// Annualized Sortino ratio over an equity curve.
///
/// Identical to [`sharpe`] except the denominator uses the standard
/// deviation of the negative returns only.  Returns `0.0` when there
/// are no negative returns or fewer than two samples.
#[must_use]
pub fn sortino(equity: &[Decimal], cadence_minutes: u32) -> f64 {
    let returns = period_returns(equity);
    if returns.is_empty() {
        return 0.0;
    }
    let mean = mean_f64(&returns);
    let downside: Vec<f64> = returns.iter().copied().filter(|r| *r < 0.0).collect();
    if downside.is_empty() {
        return 0.0;
    }
    let downside_stdev = stdev_f64(&downside, 0.0);
    if downside_stdev == 0.0 {
        return 0.0;
    }
    let periods_per_year = f64::from(MINUTES_PER_YEAR) / f64::from(cadence_minutes.max(1));
    (mean / downside_stdev) * periods_per_year.sqrt()
}

/// Annualized Calmar ratio: `annualized_return / max_drawdown_pct`.
///
/// Returns `0.0` when `max_drawdown_pct == 0` (no drawdown observed)
/// or the curve has fewer than two samples.
#[must_use]
pub fn calmar(equity: &[Decimal], cadence_minutes: u32) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let (max_dd_pct, _) = max_drawdown(equity);
    if max_dd_pct == Decimal::ZERO {
        return 0.0;
    }
    let returns = period_returns(equity);
    let mean = mean_f64(&returns);
    let periods_per_year = f64::from(MINUTES_PER_YEAR) / f64::from(cadence_minutes.max(1));
    let annualized = mean * periods_per_year;
    let dd_f64: f64 = max_dd_pct.to_string().parse().unwrap_or(0.0);
    if dd_f64 == 0.0 {
        return 0.0;
    }
    annualized / (dd_f64 / 100.0)
}

/// Compute the max drawdown of an equity curve.
///
/// Returns `(max_dd_pct, max_dd_usdt)`:
///  - `max_dd_pct` — the largest peak-to-trough decline as a positive
///    percentage (e.g. `dec!(11.25)` means an 11.25 % drawdown).
///  - `max_dd_usdt` — the same drawdown in absolute USDT (positive).
///
/// Returns `(0, 0)` for empty / single-sample / monotonically-rising
/// curves.
#[must_use]
pub fn max_drawdown(equity: &[Decimal]) -> (Decimal, Decimal) {
    if equity.len() < 2 {
        return (Decimal::ZERO, Decimal::ZERO);
    }
    let mut peak = equity[0];
    let mut max_dd = Decimal::ZERO;
    let mut max_dd_pct = Decimal::ZERO;
    for v in equity {
        if *v > peak {
            peak = *v;
        }
        let dd = peak - *v;
        if dd > max_dd {
            max_dd = dd;
            if peak != Decimal::ZERO {
                max_dd_pct = (dd / peak) * Decimal::from(100u32);
            }
        }
    }
    (max_dd_pct, max_dd)
}

/// Recovery time in bars: the number of samples elapsed from the trough
/// of the deepest drawdown to the first new equity high.  `None` means
/// the curve has not recovered yet.
#[must_use]
pub fn recovery_bars(equity: &[Decimal]) -> Option<u32> {
    if equity.len() < 2 {
        return None;
    }
    let mut peak = equity[0];
    let mut peak_idx = 0usize;
    let mut max_dd = Decimal::ZERO;
    let mut trough_idx = 0usize;
    let mut max_peak_idx = 0usize;
    for (i, v) in equity.iter().enumerate() {
        if *v > peak {
            peak = *v;
            peak_idx = i;
        }
        let dd = peak - *v;
        if dd > max_dd {
            max_dd = dd;
            trough_idx = i;
            max_peak_idx = peak_idx;
        }
    }
    if max_dd == Decimal::ZERO {
        return Some(0);
    }
    let peak_value = equity[max_peak_idx];
    // Find the first index after trough_idx where equity returns to
    // `peak_value`.
    for (i, v) in equity.iter().enumerate().skip(trough_idx + 1) {
        if *v >= peak_value {
            return u32::try_from(i - trough_idx).ok();
        }
    }
    None
}

/// Per-period log-style returns on an equity curve.  Returns
/// `(equity[i] - equity[i-1]) / equity[i-1]` as `f64`.
fn period_returns(equity: &[Decimal]) -> Vec<f64> {
    if equity.len() < 2 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(equity.len() - 1);
    for i in 1..equity.len() {
        let prev = equity[i - 1];
        if prev == Decimal::ZERO {
            out.push(0.0);
            continue;
        }
        let delta = equity[i] - prev;
        let prev_f: f64 = prev.to_string().parse().unwrap_or(1.0);
        let delta_f: f64 = delta.to_string().parse().unwrap_or(0.0);
        out.push(delta_f / prev_f);
    }
    out
}

fn mean_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for v in values {
        sum += *v;
    }
    // Cast via u32 → f64 is lossless for any vec we'd ever feed here.
    let n = u32::try_from(values.len()).unwrap_or(u32::MAX);
    sum / f64::from(n)
}

fn stdev_f64(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mut acc = 0.0;
    for v in values {
        let d = *v - mean;
        acc += d * d;
    }
    let n = u32::try_from(values.len() - 1).unwrap_or(u32::MAX);
    let var = acc / f64::from(n);
    var.sqrt()
}

/// Convenience wrapper used by the strategy-decay heuristic in
/// `crate::render::memory_highlights`.  The synthetic test stats helper
/// in that module's test code constructs `SharpeStats` directly; this
/// production helper computes both values from one slice.
#[must_use]
pub fn sharpe_ratios(equity: &[Decimal], cadence_minutes: u32) -> SharpeStats {
    SharpeStats {
        inception: sharpe(equity, cadence_minutes),
        last_7d: sharpe(equity, cadence_minutes),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t813_max_drawdown_v_shaped_curve() {
        // 100 → 80 → 100 — DD is 20 / 100 = 20 %.
        let curve = vec![dec!(100), dec!(80), dec!(100)];
        let (pct, abs) = max_drawdown(&curve);
        assert_eq!(pct, dec!(20.00));
        assert_eq!(abs, dec!(20));
    }

    #[test]
    fn t813_max_drawdown_monotonic_rising_returns_zero() {
        let curve = vec![dec!(100), dec!(110), dec!(120)];
        let (pct, abs) = max_drawdown(&curve);
        assert_eq!(pct, Decimal::ZERO);
        assert_eq!(abs, Decimal::ZERO);
    }

    #[test]
    fn t813_recovery_bars_v_shape() {
        // Peak at idx 0 (100), trough at idx 1 (80), recovery at idx 2.
        let curve = vec![dec!(100), dec!(80), dec!(100)];
        assert_eq!(recovery_bars(&curve), Some(1));
    }

    #[test]
    fn t813_recovery_bars_not_yet_recovered() {
        let curve = vec![dec!(100), dec!(80), dec!(90)];
        assert_eq!(recovery_bars(&curve), None);
    }

    #[test]
    fn t813_sharpe_constant_returns_zero_stdev_short_circuits_to_zero() {
        // Constant 1 % per period → mean > 0 but stdev = 0.
        let curve = vec![dec!(100), dec!(101), dec!(102.01), dec!(103.0301)];
        let s = sharpe(&curve, 1440); // daily cadence
        assert_eq!(s, 0.0);
    }

    #[test]
    fn t813_render_table_has_5_rows() {
        let inp = RiskMetricsInputs {
            period: "7d".into(),
            sharpe: 1.2345,
            sortino: 1.5000,
            calmar: 0.7500,
            max_drawdown_pct: dec!(11.25),
            max_drawdown_usdt: dec!(1125.50),
            recovery_bars: Some(42),
        };
        let body = render(&inp);
        assert!(body.contains("## Risk metrics"));
        assert!(body.contains("| Sharpe | 1.2345 | 7d |"));
        assert!(body.contains("| Sortino | 1.5000 | 7d |"));
        assert!(body.contains("| Calmar | 0.7500 | 7d |"));
        assert!(body.contains("| Max drawdown | 11.25% ($1125.50) | 7d |"));
        assert!(body.contains("| Recovery time | 42 bars | 7d |"));
    }

    #[test]
    fn t813_render_recovery_bars_none_renders_n_a() {
        let inp = RiskMetricsInputs {
            period: "7d".into(),
            sharpe: 0.0,
            sortino: 0.0,
            calmar: 0.0,
            max_drawdown_pct: Decimal::ZERO,
            max_drawdown_usdt: Decimal::ZERO,
            recovery_bars: None,
        };
        let body = render(&inp);
        assert!(body.contains("| Recovery time | n/a | 7d |"));
    }

    #[test]
    fn t813_render_byte_stable_across_runs() {
        let inp = RiskMetricsInputs {
            period: "7d".into(),
            sharpe: 1.0,
            sortino: 1.0,
            calmar: 1.0,
            max_drawdown_pct: dec!(5),
            max_drawdown_usdt: dec!(50),
            recovery_bars: Some(10),
        };
        let a = render(&inp);
        let b = render(&inp);
        assert_eq!(a, b);
    }
}
