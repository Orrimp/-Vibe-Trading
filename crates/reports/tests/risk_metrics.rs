#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
//! T813 — R4 risk-metrics integration test.
//!
//! Hand-computed Sharpe / max-DD / recovery-time on synthetic curves.

use reports::render::risk_metrics::{
    RiskMetricsInputs, max_drawdown, recovery_bars, render, sharpe,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn t813_r4_max_drawdown_on_v_curve_returns_known_pct_and_usdt() {
    let curve = vec![dec!(100), dec!(80), dec!(100)];
    let (pct, usdt) = max_drawdown(&curve);
    assert_eq!(pct, dec!(20));
    assert_eq!(usdt, dec!(20));
}

#[test]
fn t813_r4_recovery_bars_v_curve_one_bar() {
    let curve = vec![dec!(100), dec!(80), dec!(100)];
    assert_eq!(recovery_bars(&curve), Some(1));
}

#[test]
fn t813_r4_sharpe_constant_returns_zero_stdev() {
    // All-positive constant returns → stdev = 0 → Sharpe = 0.
    let curve = vec![dec!(100), dec!(101), dec!(102.01), dec!(103.0301)];
    let s = sharpe(&curve, 1440);
    assert_eq!(s, 0.0);
}

#[test]
fn t813_r4_render_table_contains_period_and_5_metric_rows() {
    let body = render(&RiskMetricsInputs {
        period: "7d".into(),
        sharpe: 1.2345,
        sortino: 1.5000,
        calmar: 0.7500,
        max_drawdown_pct: dec!(11.25),
        max_drawdown_usdt: dec!(1125.50),
        recovery_bars: Some(42),
    });
    // Five metric rows.
    assert!(body.contains("Sharpe"));
    assert!(body.contains("Sortino"));
    assert!(body.contains("Calmar"));
    assert!(body.contains("Max drawdown"));
    assert!(body.contains("Recovery time"));
    // Period column carries the slug on every row.
    assert_eq!(body.matches("| 7d |").count(), 5);
}

#[test]
fn t813_r4_render_recovery_n_a_when_not_yet_recovered() {
    let body = render(&RiskMetricsInputs {
        period: "30d".into(),
        sharpe: 0.0,
        sortino: 0.0,
        calmar: 0.0,
        max_drawdown_pct: Decimal::ZERO,
        max_drawdown_usdt: Decimal::ZERO,
        recovery_bars: None,
    });
    assert!(body.contains("| Recovery time | n/a | 30d |"));
}
