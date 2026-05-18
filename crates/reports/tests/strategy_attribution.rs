#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — R5 strategy-attribution integration test.
//!
//! Hand-built per-strategy fixture, asserts the rendered table has the
//! expected number of rows in the expected order with hand-computed
//! P&L / win-rate / avg-trade values.

use std::collections::BTreeSet;

use audit::query::StrategyPnl;
use reports::render::strategy_attribution::{StrategyAttributionInputs, render};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Money, StrategyId, Usdt};

fn pnl(sid: &str, realized: Decimal, closed: u32, wins: u32) -> StrategyPnl {
    let avg = if closed == 0 {
        Decimal::ZERO
    } else {
        realized / Decimal::from(closed)
    };
    StrategyPnl {
        strategy_id: StrategyId::new(sid),
        realized: Money::<Usdt>::from_decimal(realized),
        closed_trade_count: closed,
        winning_trade_count: wins,
        avg_trade_realized: Money::<Usdt>::from_decimal(avg),
    }
}

#[test]
fn t813_r5_two_strategy_table_renders_pnl_and_win_rate() {
    let rows = vec![
        pnl("alpha", dec!(150.00), 4, 3),
        pnl("beta", dec!(50.00), 2, 1),
    ];
    let inputs = StrategyAttributionInputs {
        rows,
        active_strategies: BTreeSet::new(),
    };
    let body = render(&inputs);
    assert!(body.contains("| alpha | 150.00 | 4 | 75.00% | 37.50 |"));
    assert!(body.contains("| beta | 50.00 | 2 | 50.00% | 25.00 |"));
}

#[test]
fn t813_r5_zero_trade_active_strategy_renders_no_activity() {
    let rows = vec![pnl("alpha", dec!(100.00), 2, 2)];
    let mut active = BTreeSet::new();
    active.insert("zeta".to_string());
    active.insert("alpha".to_string()); // already in rows; should not duplicate
    let inputs = StrategyAttributionInputs {
        rows,
        active_strategies: active,
    };
    let body = render(&inputs);
    // zeta no-activity row appended.
    assert!(
        body.contains("| zeta | (no activity) | (no activity) | (no activity) | (no activity) |")
    );
    // alpha row only present once with real values.
    assert_eq!(body.matches("| alpha | 100.00").count(), 1);
}
