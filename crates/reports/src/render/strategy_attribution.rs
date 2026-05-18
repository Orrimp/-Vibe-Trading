//! R5 — Per-strategy P&L attribution table.
//!
//! Builds the per-strategy table from `audit::query::pnl_by_strategy`
//! results (T803) plus the active-strategy set (`Load`/`Swap` events
//! within the period).  Strategies that fired zero closed trades but
//! were active in the window render the `(no activity)` placeholder
//! per R5.2.

use std::collections::BTreeSet;
use std::fmt::Write;

use audit::query::StrategyPnl;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Inputs for the R5 strategy-attribution section.
#[derive(Debug, Clone)]
pub struct StrategyAttributionInputs {
    /// Per-strategy P&L rows from [`audit::query::pnl_by_strategy`].
    /// Already sorted realized DESC, ties broken by `strategy_id` ASC
    /// (R5.5 guaranteed by the query).
    pub rows: Vec<StrategyPnl>,
    /// Active-strategy set from the `Load`/`Swap` events within the
    /// period.  Used to surface zero-trade strategies as `(no
    /// activity)` per R5.2.  Strategy ids stored as plain strings so
    /// the orchestrator can `.collect()` from `Option<StrategyId>`
    /// without an `Ord` bound on the newtype.
    pub active_strategies: BTreeSet<String>,
}

/// Render the R5 per-strategy attribution table.
///
/// Columns: `strategy_id, P&L (USDT), trade count, win rate,
/// avg trade P&L`.  Sort order: `pnl_by_strategy` rows first (in
/// realized-DESC order, R5.5), then any active-but-zero-trade
/// strategies appended in lex order with `(no activity)` placeholders.
#[must_use]
pub fn render(inputs: &StrategyAttributionInputs) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("## Strategy attribution\n\n");
    out.push_str("| strategy_id | P&L (USDT) | trade count | win rate | avg trade P&L |\n");
    out.push_str("|-------------|------------|-------------|----------|---------------|\n");

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for row in &inputs.rows {
        seen.insert(row.strategy_id.0.as_str());
        let pnl = fmt_2dp(row.realized.amount());
        let win_rate = if row.closed_trade_count == 0 {
            "n/a".to_string()
        } else {
            let denom = Decimal::from(row.closed_trade_count);
            let num = Decimal::from(row.winning_trade_count);
            let pct = (num / denom) * Decimal::from(100u32);
            format!("{}%", fmt_2dp(pct))
        };
        let avg_pnl = fmt_2dp(row.avg_trade_realized.amount());
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            row.strategy_id, pnl, row.closed_trade_count, win_rate, avg_pnl,
        );
    }

    // Append active-but-zero-trade strategies as `(no activity)` rows.
    for sid in &inputs.active_strategies {
        if seen.contains(sid.as_str()) {
            continue;
        }
        let _ = writeln!(
            out,
            "| {sid} | (no activity) | (no activity) | (no activity) | (no activity) |",
        );
    }

    out
}

/// Format a `Decimal` with exactly two decimal places (preserves trailing
/// zeros so `dec!(100)` renders as `100.00`).
fn fmt_2dp(d: Decimal) -> String {
    let two_scale = (d.round_dp(2) * dec!(1.00)).round_dp(2);
    format!("{two_scale:.2}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
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
    fn t813_strategy_attribution_renders_rows_with_win_rate() {
        let rows = vec![
            pnl("alpha", dec!(150.00), 4, 3),
            pnl("beta", dec!(50.00), 2, 1),
        ];
        let inputs = StrategyAttributionInputs {
            rows,
            active_strategies: BTreeSet::new(),
        };
        let body = render(&inputs);
        assert!(body.contains("## Strategy attribution"));
        assert!(body.contains("| alpha | 150.00 | 4 | 75.00% | 37.50 |"));
        assert!(body.contains("| beta | 50.00 | 2 | 50.00% | 25.00 |"));
    }

    #[test]
    fn t813_strategy_attribution_zero_trades_render_no_activity() {
        let rows = vec![pnl("alpha", dec!(100.00), 2, 2)];
        let mut active = BTreeSet::new();
        active.insert("zeta".to_string());
        active.insert("alpha".to_string()); // already in rows
        let inputs = StrategyAttributionInputs {
            rows,
            active_strategies: active,
        };
        let body = render(&inputs);
        // alpha row present (real numbers); zeta no-activity row appended.
        assert!(body.contains("| alpha | 100.00 | 2 | 100.00% | 50.00 |"));
        assert!(
            body.contains(
                "| zeta | (no activity) | (no activity) | (no activity) | (no activity) |"
            )
        );
    }

    #[test]
    fn t813_strategy_attribution_byte_stable_across_runs() {
        let rows = vec![pnl("alpha", dec!(10.00), 1, 1)];
        let inputs = StrategyAttributionInputs {
            rows,
            active_strategies: BTreeSet::new(),
        };
        let a = render(&inputs);
        let b = render(&inputs);
        assert_eq!(a, b);
    }
}
