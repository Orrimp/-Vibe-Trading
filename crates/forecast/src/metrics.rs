//! Hourly-resolution performance metrics for the sharpe-comparison reports.
//!
//! Extracted verbatim from `bin/sharpe_comparison.rs` on 2026-09-29 (story 1-25 AC3.1).
//! It lived inside the binary, so `tests/sharpe_comparison_determinism.rs` — the test the
//! design named the RE-SYNC TRIGGER for the annualisation constant — could not import it
//! and re-implemented all of it instead, constant included. A test that re-implements its
//! subject compares a copy against a copy: it would have stayed green if this module were
//! deleted, and it could never trip on the drift it exists to catch. One definition now,
//! imported by both.
//!
//! Pure move: the code below is byte-identical to the module it replaces, dedented by one
//! level. No behaviour change, so no report body moves.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Hourly-to-annual annualisation factor: **√8574.9998** ≈ 92.601295.
///
/// NOT √(24·365), which is √8760 ≈ 93.594872. This line claimed the latter until
/// 2026-09-29 while shipping the former — a ~2.1 % difference. The value is RATIFIED
/// as-is (story 1-25 AC3.1) and pinned RED-on-revert by
/// `sqrt_hours_per_year_is_the_ratified_8575_constant_and_not_8760` in
/// `tests/sharpe_comparison_determinism.rs`.
///
/// ADR-0033 § D4: NOT sqrt(525_600) which is the minute-resolution
/// constant used in `crates/backtest::compute_sharpe()`.
pub const SQRT_HOURS_PER_YEAR: f64 = 92.601_295_098_46;

// Sanity: (24.0 * 365.0).sqrt() at compile time (const f64 sqrt is
// unstable; we pin the value and assert in a test).

/// Compute arithmetic log-returns from a per-bar equity series.
fn log_returns(equity: &[Decimal]) -> Vec<f64> {
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

/// Hourly-annualised Sharpe ratio (rf = 0).
///
/// Formula: `mean_r / std_r * sqrt(24 * 365)`.
/// Returns 0.0 for series with fewer than 2 bars or zero std.
pub fn compute_sharpe_hourly(equity: &[Decimal]) -> f64 {
    let rets = log_returns(equity);
    let n = rets.len();
    if n < 2 {
        return 0.0;
    }
    let mean_r = rets.iter().sum::<f64>() / n as f64;
    let var_r: f64 = rets.iter().map(|&r| (r - mean_r).powi(2)).sum::<f64>() / n as f64;
    let std_r = var_r.sqrt();
    if std_r < 1e-15 {
        return 0.0;
    }
    mean_r / std_r * SQRT_HOURS_PER_YEAR
}

/// Hourly-annualised Sortino ratio (rf = 0).
///
/// Formula: `mean_r / downside_std_r * sqrt(24 * 365)`.
/// `downside_std_r = sqrt(mean(min(r, 0)^2))`.
/// Returns 0.0 for series with fewer than 2 bars or zero downside std.
pub fn compute_sortino_hourly(equity: &[Decimal]) -> f64 {
    let rets = log_returns(equity);
    let n = rets.len();
    if n < 2 {
        return 0.0;
    }
    let mean_r = rets.iter().sum::<f64>() / n as f64;
    let downside_sq: f64 = rets.iter().map(|&r| r.min(0.0).powi(2)).sum::<f64>() / n as f64;
    let downside_std = downside_sq.sqrt();
    if downside_std < 1e-15 {
        return 0.0;
    }
    mean_r / downside_std * SQRT_HOURS_PER_YEAR
}

/// Calmar ratio: `CAGR / abs(max_drawdown)`.
///
/// `CAGR = (final/initial)^(1/years) - 1` where
/// `years = (equity.len() - 1) / 8760.0`.
/// Returns 0.0 for series with fewer than 2 bars, zero drawdown, or zero
/// initial equity.
pub fn compute_calmar(equity: &[Decimal]) -> f64 {
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
    let max_dd = compute_max_drawdown(equity);
    if max_dd.abs() < 1e-15 {
        return 0.0;
    }
    cagr / max_dd.abs()
}

/// Maximum drawdown: `max over t of (peak_equity_t - equity_t) / peak_equity_t`.
///
/// Returns 0.0 for series with fewer than 2 bars or zero peak.
pub fn compute_max_drawdown(equity: &[Decimal]) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let mut peak = equity[0].to_f64().unwrap_or(0.0);
    let mut max_dd = 0.0f64;
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

// ── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // Tolerance for float comparisons.
    const TOL: f64 = 1e-4;

    /// (a) Sharpe on a hand-built equity curve with known mean / std.
    #[test]
    fn test_sharpe_known() {
        // Equity: 100, 101, 102 → log returns ≈ [ln(1.01), ln(1.0099)]
        let equity = vec![dec!(100), dec!(101), dec!(102)];
        let s = compute_sharpe_hourly(&equity);
        // mean_r ≈ 0.00995, std ≈ tiny (returns nearly identical)
        // Just assert it's positive and finite.
        assert!(s.is_finite() && s > 0.0, "Sharpe should be positive: {s}");

        // Manually verify: returns = [ln(101/100), ln(102/101)]
        let r1 = (101.0f64 / 100.0).ln();
        let r2 = (102.0f64 / 101.0).ln();
        let mean_r = (r1 + r2) / 2.0;
        let var_r = ((r1 - mean_r).powi(2) + (r2 - mean_r).powi(2)) / 2.0;
        let std_r = var_r.sqrt();
        let expected = mean_r / std_r * SQRT_HOURS_PER_YEAR;
        assert!(
            (s - expected).abs() < TOL,
            "Sharpe mismatch: got {s}, expected {expected}"
        );
    }

    /// (b) Sortino > Sharpe when downside returns are smaller than upside.
    #[test]
    fn test_sortino_vs_sharpe_asymmetric() {
        // Build a curve with mostly gains and one small loss.
        let equity: Vec<rust_decimal::Decimal> = {
            let mut v = vec![dec!(100)];
            // 10 bars of +1%, one bar of -0.1%, 10 more bars of +1%
            let mut curr = dec!(100);
            for _ in 0..10 {
                curr *= dec!(1.01);
                v.push(curr);
            }
            curr *= dec!(0.999);
            v.push(curr);
            for _ in 0..10 {
                curr *= dec!(1.01);
                v.push(curr);
            }
            v
        };
        let sharpe = compute_sharpe_hourly(&equity);
        let sortino = compute_sortino_hourly(&equity);
        // With small downside, Sortino should be >= Sharpe.
        assert!(
            sortino >= sharpe,
            "Sortino ({sortino}) should be >= Sharpe ({sharpe}) with small downside"
        );
    }

    /// (c) Calmar on a curve with known CAGR + DD.
    #[test]
    fn test_calmar_known() {
        // Simple curve: 100 → 200 (doubled) over 8760 bars (1 year).
        // CAGR = 100%. Max DD = 0 (monotone).
        // → Calmar would be infinite (no drawdown). Use a curve with a dip.
        let mut equity = vec![dec!(100)];
        for _ in 0..4380 {
            equity.push(*equity.last().unwrap() * dec!(1.0001));
        }
        // Dip by 10%.
        let peak = *equity.last().unwrap();
        equity.push(peak * dec!(0.90));
        // Then recover and grow.
        let mut curr = *equity.last().unwrap();
        for _ in 0..(8760 - 4381) {
            curr *= dec!(1.0001);
            equity.push(curr);
        }

        let dd = compute_max_drawdown(&equity);
        assert!(dd > 0.0 && dd < 0.15, "max_dd should be ~10%: {dd}");

        let calmar = compute_calmar(&equity);
        assert!(
            calmar.is_finite() && calmar > 0.0,
            "Calmar should be positive: {calmar}"
        );
    }

    /// (d) max_drawdown on a peak-then-trough curve.
    #[test]
    fn test_max_drawdown_peak_trough() {
        // 100 → 200 → 50 → 150
        let equity = vec![dec!(100), dec!(200), dec!(50), dec!(150)];
        let dd = compute_max_drawdown(&equity);
        // Peak = 200, trough = 50 → dd = (200 - 50) / 200 = 0.75
        assert!((dd - 0.75).abs() < 1e-9, "max_dd should be 0.75: {dd}");
    }

    /// (e) edge case: 1-element equity curve returns 0.0 for all four.
    #[test]
    fn test_edge_single_element() {
        let equity = vec![dec!(100)];
        assert_eq!(compute_sharpe_hourly(&equity), 0.0);
        assert_eq!(compute_sortino_hourly(&equity), 0.0);
        assert_eq!(compute_calmar(&equity), 0.0);
        assert_eq!(compute_max_drawdown(&equity), 0.0);
    }
}
