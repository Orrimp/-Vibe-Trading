//! R3 — Equity curve (sparkline + downsampling + companion CSV).
//!
//! The renderer is pure over its inputs: the orchestrator pre-samples
//! the equity curve once at the right cadence (1m for windows ≤ 7d,
//! 5m for > 7d per R3.5) and passes the resulting `Vec<Decimal>` here.
//! The accompanying CSV writer lives in `crate::csv_artifacts`.

use std::fmt::Write;

use rust_decimal::Decimal;

use crate::sparkline;

/// Inputs for the R3 equity-curve section.
#[derive(Debug, Clone)]
pub struct EquityCurveInputs {
    /// Period slug used as the second sparkline label (e.g. `7d`, `weekly`).
    pub window_label: String,
    /// Equity curve sampled at 1m or 5m cadence over the report period
    /// (the per-window curve).
    pub period_curve: Vec<Decimal>,
    /// Equity curve since inception (sampled at the same cadence).
    pub since_inception_curve: Vec<Decimal>,
}

/// Render the R3 equity-curve section.
///
/// Two sparklines, each preceded by a label.  The shape matches R3.1
/// (since-inception + last-N-days windows).
#[must_use]
pub fn render(inputs: &EquityCurveInputs) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("## Equity curve\n\n");

    let inception_line = sparkline::encode(&inputs.since_inception_curve, sparkline::DEFAULT_WIDTH);
    let _ = writeln!(out, "Since inception:");
    let _ = writeln!(out, "`{inception_line}`");
    out.push('\n');

    let period_line = sparkline::encode(&inputs.period_curve, sparkline::DEFAULT_WIDTH);
    let _ = writeln!(out, "Window {}:", inputs.window_label);
    let _ = writeln!(out, "`{period_line}`");
    out.push('\n');

    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t813_equity_curve_section_renders_both_sparklines() {
        let inputs = EquityCurveInputs {
            window_label: "7d".to_string(),
            period_curve: vec![dec!(100), dec!(101), dec!(102), dec!(103)],
            since_inception_curve: vec![dec!(100), dec!(110), dec!(120)],
        };
        let body = render(&inputs);
        assert!(body.contains("## Equity curve"));
        assert!(body.contains("Since inception:"));
        assert!(body.contains("Window 7d:"));
        // Two sparkline lines (in backticks).
        assert_eq!(body.matches('`').count(), 4);
    }

    #[test]
    fn t813_equity_curve_byte_stable_across_runs() {
        let inputs = EquityCurveInputs {
            window_label: "weekly".to_string(),
            period_curve: vec![dec!(50), dec!(60), dec!(55), dec!(70)],
            since_inception_curve: vec![dec!(50), dec!(80)],
        };
        let a = render(&inputs);
        let b = render(&inputs);
        assert_eq!(a, b);
    }

    #[test]
    fn t813_equity_curve_empty_curve_renders_spaces() {
        let inputs = EquityCurveInputs {
            window_label: "7d".to_string(),
            period_curve: vec![],
            since_inception_curve: vec![],
        };
        let body = render(&inputs);
        assert!(body.contains("`                                                            `"));
    }
}
