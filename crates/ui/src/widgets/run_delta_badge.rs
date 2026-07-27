//! Run delta badge — ui-rethink-phase-b-lab-run T-D-N13.
//!
//! Shows the **delta between two consecutive backtest runs** on the same
//! (strategy, pair, range) tuple. Appears to the right of the Run button
//! in `screens/lab.rs::run_button_row` when both `lab_state.last_run_report`
//! **and** `lab_state.prev_run_report` are `Some` and share the same tuple.
//!
//! ## Layout (Design § D5)
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  P&L        DD         SR                       │
//! │  +$1,200    -2.4%      +0.12                    │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! Three columns, each with a micro-label above a signed value:
//! - **P&L** — change in total return: `(last.final_equity - last.initial_equity)
//!   - (prev.final_equity - prev.initial_equity)`, rendered via `fmt_usdt_signed`.
//! - **DD** — change in max drawdown: `last.max_drawdown - prev.max_drawdown`,
//!   rendered as a signed percentage. **Note:** a decrease in max drawdown is
//!   good (UP_500 colour); an increase is bad (DOWN_500 colour) — sign is inverted
//!   relative to P&L.
//! - **SR** — change in annualised Sharpe ratio: computed via
//!   `backtest::compute_sharpe`, rendered as a signed float with 2 decimal places.
//!
//! **Zero hex literals** — all colours from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::Length;
use iced::widget::{Column, Row, Text};
use rust_decimal::Decimal;

use crate::lab::runner::RunReportMirror;
use crate::strings::{
    RUN_DELTA_BADGE_DD_LABEL, RUN_DELTA_BADGE_PNL_LABEL, RUN_DELTA_BADGE_SHARPE_LABEL,
};
use crate::theme::{ThemeMode, color, space, text};
use crate::widgets::num::{fmt_pct_signed, fmt_usdt_signed};

// ── Sign determination ────────────────────────────────────────────────────────

/// Sign of a delta value for colour-coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaSign {
    Up,
    Down,
    Flat,
}

impl DeltaSign {
    fn from_decimal(d: Decimal) -> Self {
        match d.cmp(&Decimal::ZERO) {
            std::cmp::Ordering::Greater => Self::Up,
            std::cmp::Ordering::Less => Self::Down,
            std::cmp::Ordering::Equal => Self::Flat,
        }
    }

    /// Same as `from_decimal` but with the semantic inverted.
    /// Used for max drawdown: a *reduction* in DD is good (Up), an
    /// *increase* is bad (Down).
    fn from_decimal_inverted(d: Decimal) -> Self {
        match d.cmp(&Decimal::ZERO) {
            std::cmp::Ordering::Less => Self::Up,
            std::cmp::Ordering::Greater => Self::Down,
            std::cmp::Ordering::Equal => Self::Flat,
        }
    }

    fn color(self, mode: ThemeMode) -> iced::Color {
        match self {
            Self::Up => color::UP_500.current(mode),
            Self::Down => color::DOWN_500.current(mode),
            Self::Flat => color::FG_3.current(mode),
        }
    }
}

// ── Delta computation ─────────────────────────────────────────────────────────

/// All three delta values extracted from the two mirrors.
#[derive(Debug, Clone)]
pub struct RunDelta {
    /// Change in gross P&L (final − initial for each run, then delta between runs).
    pub pnl_delta: Decimal,
    /// Change in max drawdown (`last.max_drawdown` − `prev.max_drawdown`).
    /// Positive = drawdown got worse; negative = drawdown improved.
    pub dd_delta: Decimal,
    /// Change in annualised Sharpe ratio (last − prev).
    pub sharpe_delta: f64,
}

/// Compute the `RunDelta` from two mirrors.
#[must_use]
pub fn compute_delta(last: &RunReportMirror, prev: &RunReportMirror) -> RunDelta {
    let last_pnl = last.kpis.final_equity.amount() - last.kpis.initial_equity.amount();
    let prev_pnl = prev.kpis.final_equity.amount() - prev.kpis.initial_equity.amount();
    let pnl_delta = last_pnl - prev_pnl;

    let dd_delta = last.kpis.max_drawdown - prev.kpis.max_drawdown;

    let last_curve: Vec<Decimal> = last.equity_series.iter().map(|(_, eq)| *eq).collect();
    let prev_curve: Vec<Decimal> = prev.equity_series.iter().map(|(_, eq)| *eq).collect();
    let sharpe_last = backtest::compute_sharpe(&last_curve);
    let sharpe_prev = backtest::compute_sharpe(&prev_curve);
    let sharpe_delta = sharpe_last - sharpe_prev;

    RunDelta {
        pnl_delta,
        dd_delta,
        sharpe_delta,
    }
}

// ── View ──────────────────────────────────────────────────────────────────────

/// Render the run delta badge.
///
/// Shows three signed delta columns (P&L / DD / SR). Visibility gate lives in
/// the caller (`screens/lab.rs`) — this function always renders.
///
/// Width is unconstrained — the caller wraps in a ~180 px Fixed container.
#[must_use]
pub fn view<'a>(
    last: &RunReportMirror,
    prev: &RunReportMirror,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let delta = compute_delta(last, prev);

    // ── P&L column ────────────────────────────────────────────────────────────
    let pnl_sign = DeltaSign::from_decimal(delta.pnl_delta);
    let pnl_label = Text::new(RUN_DELTA_BADGE_PNL_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));
    let pnl_value = Text::new(fmt_usdt_signed(delta.pnl_delta))
        .size(text::SMALL)
        .color(pnl_sign.color(mode));
    let pnl_col = Column::new()
        .spacing(space::XXS)
        .push(pnl_label)
        .push(pnl_value);

    // ── Drawdown column (inverted sign — decrease is good) ────────────────────
    let dd_sign = DeltaSign::from_decimal_inverted(delta.dd_delta);
    let dd_label = Text::new(RUN_DELTA_BADGE_DD_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));
    let dd_value_str = fmt_pct_signed(delta.dd_delta);
    let dd_value = Text::new(dd_value_str)
        .size(text::SMALL)
        .color(dd_sign.color(mode));
    let dd_col = Column::new()
        .spacing(space::XXS)
        .push(dd_label)
        .push(dd_value);

    // ── Sharpe column ─────────────────────────────────────────────────────────
    let sharpe_dec = Decimal::try_from(delta.sharpe_delta).unwrap_or(Decimal::ZERO);
    let sr_sign = DeltaSign::from_decimal(sharpe_dec);
    let sr_label = Text::new(RUN_DELTA_BADGE_SHARPE_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));
    let sr_str = if delta.sharpe_delta > 0.0 {
        format!("+{:.2}", delta.sharpe_delta)
    } else {
        format!("{:.2}", delta.sharpe_delta)
    };
    let sr_value = Text::new(sr_str)
        .size(text::SMALL)
        .color(sr_sign.color(mode));
    let sr_col = Column::new()
        .spacing(space::XXS)
        .push(sr_label)
        .push(sr_value);

    Row::new()
        .spacing(space::M)
        .push(pnl_col)
        .push(dd_col)
        .push(sr_col)
        .width(Length::Shrink)
        .into()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use backtest::engine::BacktestKpis;
    use rust_decimal_macros::dec;
    use std::sync::Arc;
    use trading_core::{Money, Usdt};

    /// Build a `RunReportMirror` fixture with explicit KPIs and equity series.
    fn make_mirror(
        initial: Decimal,
        final_eq: Decimal,
        max_dd: Decimal,
        equity_series: Vec<(i64, Decimal)>,
    ) -> RunReportMirror {
        use crate::lab::equity_loader::LabTuple;
        use crate::lab::state::{DateRange, LabDataSource, Preset};
        RunReportMirror {
            tuple: LabTuple {
                strategy: smol_str::SmolStr::new("v1.momentum"),
                symbol: smol_str::SmolStr::new("XRPUSDT"),
                range: DateRange::Preset(Preset::H1_2024),
                source: LabDataSource::Synthetic,
            },
            equity_series: Arc::new(equity_series),
            kpis: BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(final_eq),
                initial_equity: Money::<Usdt>::from_decimal(initial),
                max_drawdown: max_dd,
                trade_count: 0,
                total_fees: Money::<Usdt>::from_decimal(Decimal::ZERO),
                buys: 0,
                sells: 0,
                total_return_pct: Decimal::ZERO,
            },
            generated_at: time::OffsetDateTime::UNIX_EPOCH,
            bars: Arc::new(Vec::new()),
            position_curve: Arc::new(Vec::new()),
        }
    }

    /// Helper: build a flat equity series (initial → same value repeated).
    fn flat_series(initial: Decimal, n: usize) -> Vec<(i64, Decimal)> {
        #[allow(clippy::cast_possible_wrap)] // test-only: n is always tiny, no wrap risk
        (0..n).map(|i| (i as i64, initial)).collect()
    }

    /// All 8 sign combinations for (Δ P&L sign × Δ `MaxDD` sign × Δ Sharpe sign).
    ///
    /// Convention for DD sign: `delta.dd_delta` > 0 means drawdown got worse
    /// (colour = DOWN). `delta.dd_delta` < 0 means drawdown improved (colour = UP).
    ///
    /// For Sharpe, a rising equity curve vs flat gives a positive Sharpe delta.
    /// For Pnl, the difference in (final − initial) between last and prev runs.

    // Case 1: P&L ↑, DD ↓ (better), Sharpe ↑
    #[test]
    fn delta_pnl_up_dd_down_sharpe_up() {
        // last: +$2000 pnl, dd=0.10, rising equity
        let last = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.10),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        // prev: +$1000 pnl, dd=0.15, flat equity
        let prev = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.15),
            flat_series(dec!(100_000), 3),
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta > Decimal::ZERO, "P&L delta should be positive");
        assert!(
            d.dd_delta < Decimal::ZERO,
            "DD delta should be negative (improved)"
        );
        assert!(d.sharpe_delta > 0.0, "Sharpe delta should be positive");
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Up);
        assert_eq!(
            DeltaSign::from_decimal_inverted(d.dd_delta),
            DeltaSign::Up,
            "DD improved = Up sign"
        );
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Up
        );
    }

    // Case 2: P&L ↑, DD ↓ (better), Sharpe ↓
    #[test]
    fn delta_pnl_up_dd_down_sharpe_down() {
        let last = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.10),
            flat_series(dec!(100_000), 3),
        ); // flat = low/zero sharpe
        let prev = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.15),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        ); // rising = higher sharpe
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta > Decimal::ZERO, "P&L delta should be positive");
        assert!(
            d.dd_delta < Decimal::ZERO,
            "DD delta should be negative (improved)"
        );
        assert!(d.sharpe_delta < 0.0, "Sharpe delta should be negative");
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Up);
        assert_eq!(DeltaSign::from_decimal_inverted(d.dd_delta), DeltaSign::Up);
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Down
        );
    }

    // Case 3: P&L ↑, DD ↑ (worse), Sharpe ↑
    #[test]
    fn delta_pnl_up_dd_up_sharpe_up() {
        let last = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.20),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        let prev = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.10),
            flat_series(dec!(100_000), 3),
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta > Decimal::ZERO);
        assert!(d.dd_delta > Decimal::ZERO, "DD got worse");
        assert!(d.sharpe_delta > 0.0);
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Up);
        assert_eq!(
            DeltaSign::from_decimal_inverted(d.dd_delta),
            DeltaSign::Down,
            "DD worse = Down sign"
        );
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Up
        );
    }

    // Case 4: P&L ↑, DD ↑ (worse), Sharpe ↓
    #[test]
    fn delta_pnl_up_dd_up_sharpe_down() {
        let last = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.20),
            flat_series(dec!(100_000), 3),
        );
        let prev = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.10),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta > Decimal::ZERO);
        assert!(d.dd_delta > Decimal::ZERO, "DD got worse");
        assert!(d.sharpe_delta < 0.0);
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Up);
        assert_eq!(
            DeltaSign::from_decimal_inverted(d.dd_delta),
            DeltaSign::Down
        );
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Down
        );
    }

    // Case 5: P&L ↓, DD ↓ (better), Sharpe ↑
    #[test]
    fn delta_pnl_down_dd_down_sharpe_up() {
        let last = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.05),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        let prev = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.20),
            flat_series(dec!(100_000), 3),
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta < Decimal::ZERO);
        assert!(d.dd_delta < Decimal::ZERO);
        assert!(d.sharpe_delta > 0.0);
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Down);
        assert_eq!(DeltaSign::from_decimal_inverted(d.dd_delta), DeltaSign::Up);
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Up
        );
    }

    // Case 6: P&L ↓, DD ↓ (better), Sharpe ↓
    #[test]
    fn delta_pnl_down_dd_down_sharpe_down() {
        let last = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.05),
            flat_series(dec!(100_000), 3),
        );
        let prev = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.20),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta < Decimal::ZERO);
        assert!(d.dd_delta < Decimal::ZERO);
        assert!(d.sharpe_delta < 0.0);
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Down);
        assert_eq!(DeltaSign::from_decimal_inverted(d.dd_delta), DeltaSign::Up);
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Down
        );
    }

    // Case 7: P&L ↓, DD ↑ (worse), Sharpe ↑
    #[test]
    fn delta_pnl_down_dd_up_sharpe_up() {
        let last = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.25),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        let prev = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.10),
            flat_series(dec!(100_000), 3),
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta < Decimal::ZERO);
        assert!(d.dd_delta > Decimal::ZERO, "DD got worse");
        assert!(d.sharpe_delta > 0.0);
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Down);
        assert_eq!(
            DeltaSign::from_decimal_inverted(d.dd_delta),
            DeltaSign::Down
        );
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Up
        );
    }

    // Case 8: P&L ↓, DD ↑ (worse), Sharpe ↓
    #[test]
    fn delta_pnl_down_dd_up_sharpe_down() {
        let last = make_mirror(
            dec!(100_000),
            dec!(101_000),
            dec!(0.25),
            flat_series(dec!(100_000), 3),
        );
        let prev = make_mirror(
            dec!(100_000),
            dec!(102_000),
            dec!(0.10),
            vec![(0, dec!(100_000)), (1, dec!(101_000)), (2, dec!(102_000))],
        );
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta < Decimal::ZERO);
        assert!(d.dd_delta > Decimal::ZERO, "DD got worse");
        assert!(d.sharpe_delta < 0.0);
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Down);
        assert_eq!(
            DeltaSign::from_decimal_inverted(d.dd_delta),
            DeltaSign::Down
        );
        assert_eq!(
            DeltaSign::from_decimal(Decimal::try_from(d.sharpe_delta).unwrap()),
            DeltaSign::Down
        );
    }

    // Flat test: zero deltas → all Flat
    #[test]
    fn delta_all_flat() {
        let series = flat_series(dec!(100_000), 3);
        let last = make_mirror(dec!(100_000), dec!(100_000), dec!(0.0), series.clone());
        let prev = make_mirror(dec!(100_000), dec!(100_000), dec!(0.0), series);
        let d = compute_delta(&last, &prev);
        assert!(d.pnl_delta.is_zero());
        assert!(d.dd_delta.is_zero());
        assert_eq!(DeltaSign::from_decimal(d.pnl_delta), DeltaSign::Flat);
        assert_eq!(
            DeltaSign::from_decimal_inverted(d.dd_delta),
            DeltaSign::Flat
        );
    }
}
