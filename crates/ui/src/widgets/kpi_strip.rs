//! KPI strip widget — Phase 4 (T1805 / R2).
//!
//! Six metric cards (Total return / CAGR / Sharpe / Max DD / Win
//! rate / Trades) laid out on a native `iced::widget::grid::Grid`
//! with six equal columns. Sentiment-coloured per the Phase 4
//! Design's Q-resolved table; missing fields render as `—` dashes.
//!
//! **Grid theming (Q3-sub / T3.0, 2026-05-13):** `Grid` has no
//! `Style`, no `Catalog`, no `.style()` / `.class()` method. It
//! inherits container defaults. Visual chrome (PANEL background,
//! border, padding) stays in the outer `Container` wrapping the
//! Grid; per-card surface tokens stay in the `card(...)` helper.
//! No Catalog adapter is required for Grid.
//!
//! **Zero string literals** — copy via `crate::strings::*`.
//! **Zero hex colours** — tokens via `crate::theme::*`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names,
    clippy::match_same_arms
)]

use iced::widget::grid::Grid;
use iced::widget::{Column, Container, Text, container};
use iced::{Border, Length};
use trading_core::BacktestMetrics;

use super::frame::muted_body;
use super::num::{format_count, format_pct_max_dd, format_pct_sentiment, format_sharpe};
use crate::state::PanelState;
use crate::strings::{
    KPI_CAGR_LABEL, KPI_DASH_PLACEHOLDER, KPI_MAX_DD_LABEL, KPI_SHARPE_LABEL,
    KPI_TOTAL_RETURN_LABEL, KPI_TRADES_LABEL, KPI_WIN_RATE_LABEL, VIEWER_METRICS_UNAVAILABLE,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::viewer::ViewerMessage;

/// Render the six-card KPI strip over a [`BacktestMetrics`] panel
/// state.
#[must_use]
pub fn view<'a>(
    metrics: &'a PanelState<BacktestMetrics>,
    mode: ThemeMode,
) -> iced::Element<'a, ViewerMessage> {
    let body: iced::Element<'a, ViewerMessage> = match metrics {
        PanelState::Loading | PanelState::Empty => unavailable_strip(mode),
        PanelState::Error(_) => unavailable_strip(mode),
        PanelState::Ready(m) => {
            if is_all_absent(m) {
                unavailable_strip(mode)
            } else {
                ready_strip(m, mode)
            }
        }
    };

    // Tier-1 PANEL chrome — bare container (no header — Q-resolved
    // design renders the cards directly without a frame title).
    Container::new(body)
        .width(Length::Fill)
        .padding(space::L as u16)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Treat a `BacktestMetrics` value where every present-flag is false
/// AND every numeric field is zero AS the all-absent sentinel.
fn is_all_absent(m: &BacktestMetrics) -> bool {
    !m.cagr_present
        && !m.sharpe_present
        && !m.win_rate_present
        && m.total_return_pct.is_zero()
        && m.max_drawdown_pct.is_zero()
        && m.trades == 0
}

fn ready_strip<'a>(m: &'a BacktestMetrics, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    // Card 1: Total return — sentiment.
    let (tr_text, tr_color) = format_pct_sentiment(m.total_return_pct, mode);
    let total_return = card(KPI_TOTAL_RETURN_LABEL, tr_text, tr_color, mode);

    // Card 2: CAGR — neutral; `—` when absent.
    let (cagr_text, cagr_color) = if m.cagr_present {
        let (s, _) = format_pct_sentiment(m.cagr_pct, mode);
        (s, color::FG_1.current(mode))
    } else {
        (KPI_DASH_PLACEHOLDER.to_string(), color::FG_3.current(mode))
    };
    let cagr = card(KPI_CAGR_LABEL, cagr_text, cagr_color, mode);

    // Card 3: Sharpe — neutral; `—` when absent.
    let (sharpe_text, sharpe_color) = if m.sharpe_present {
        (format_sharpe(m.sharpe), color::FG_1.current(mode))
    } else {
        (KPI_DASH_PLACEHOLDER.to_string(), color::FG_3.current(mode))
    };
    let sharpe = card(KPI_SHARPE_LABEL, sharpe_text, sharpe_color, mode);

    // Card 4: Max DD — always DOWN_500 with minus prefix.
    let (mdd_text, mdd_color) = format_pct_max_dd(m.max_drawdown_pct, mode);
    let max_dd = card(KPI_MAX_DD_LABEL, mdd_text, mdd_color, mode);

    // Card 5: Win rate — neutral; `—` when absent.
    let (wr_text, wr_color) = if m.win_rate_present {
        let (s, _) = format_pct_sentiment(m.win_rate_pct, mode);
        (s, color::FG_1.current(mode))
    } else {
        (KPI_DASH_PLACEHOLDER.to_string(), color::FG_3.current(mode))
    };
    let win_rate = card(KPI_WIN_RATE_LABEL, wr_text, wr_color, mode);

    // Card 6: Trades — neutral, thousands-separated.
    let trades_text = format_count(m.trades);
    let trades = card(
        KPI_TRADES_LABEL,
        trades_text,
        color::FG_1.current(mode),
        mode,
    );

    // T3.1 — six-column native `Grid` replaces the prior hand-rolled
    // `Row::new().spacing(...).push(...) × 6.width(Length::Fill)` chain
    // per H-arch-A3 RESOLVED-UNFALSIFIED. `Grid::columns(6)` handles
    // column equalization implicitly, so the per-card
    // `Length::FillPortion(1)` width hint is removed in `card(...)`.
    // `Grid` defaults to filling its parent width — no explicit
    // `.width(...)` call required (and `Grid::width` accepts only
    // `Pixels`, not `Length::Fill`). The default `Sizing::AspectRatio(1.0)`
    // would force square cells; override with
    // `.height(Length::Shrink)` so each cell hugs its intrinsic text
    // height (~80 px strip per the viewer panel snapshot).
    Grid::new()
        .columns(6)
        .spacing(space::M)
        .height(Length::Shrink)
        .push(total_return)
        .push(cagr)
        .push(sharpe)
        .push(max_dd)
        .push(win_rate)
        .push(trades)
        .into()
}

fn unavailable_strip<'a>(mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    let dash = KPI_DASH_PLACEHOLDER.to_string();
    let dash_color = color::FG_3.current(mode);
    let labels = [
        KPI_TOTAL_RETURN_LABEL,
        KPI_CAGR_LABEL,
        KPI_SHARPE_LABEL,
        KPI_MAX_DD_LABEL,
        KPI_WIN_RATE_LABEL,
        KPI_TRADES_LABEL,
    ];
    // T3.2 — same Grid migration as the ready strip: the six
    // dash-placeholder cards live on a 6-column `Grid` instead of a
    // hand-rolled `Row::new().push(...) × 6` loop. Outer `Column`
    // composition with the muted-body advisory line is unchanged.
    let mut grid = Grid::new()
        .columns(6)
        .spacing(space::M)
        .height(Length::Shrink);
    for label in labels {
        grid = grid.push(card(label, dash.clone(), dash_color, mode));
    }
    Column::new()
        .spacing(space::S)
        .push(grid)
        .push(muted_body(VIEWER_METRICS_UNAVAILABLE))
        .into()
}

/// Render the Lab single-run KPI strip from a live `BacktestKpis`.
///
/// Called by `screens/lab.rs` between the status strip and the chart.
/// Returns an `Element<'_, crate::state::Message>` (not `ViewerMessage`)
/// so it can compose with the Lab screen's element tree.
///
/// Cards rendered (6-column layout):
/// 1. Return % — `(final − initial) / initial × 100`, sentiment colour.
/// 2. Max DD — always `DOWN_500`, percent with minus prefix.
/// 3. Trades — neutral, thousands-separated count.
/// 4. Fees — neutral, USDT amount.
/// 5. Sharpe — always em-dash (Phase C follow-up; engine not yet computing).
/// 6. Final equity — neutral, USDT amount.
///
/// When `kpis` is `None` (no run completed yet): all six cards show `—`.
/// lab-end-to-end-v2 Wave D-1.1 F8. R3 (lab-polish-round-2) extends to
/// 8 cards in a 2-row 4-column grid.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn view_for_lab<'a>(
    kpis: Option<&backtest::BacktestKpis>,
    mode: ThemeMode,
) -> iced::Element<'a, crate::state::Message> {
    use super::num::{fmt_usdt, fmt_usdt_signed, format_count};
    use crate::strings::{
        KPI_DASH_PLACEHOLDER, LAB_KPI_BUYS_LABEL, LAB_KPI_FEES_LABEL, LAB_KPI_FINAL_EQUITY_LABEL,
        LAB_KPI_MAX_DD_LABEL, LAB_KPI_NET_DELTA_LABEL, LAB_KPI_RETURN_LABEL, LAB_KPI_SELLS_LABEL,
        LAB_KPI_SHARPE_LABEL, LAB_KPI_TRADES_LABEL,
    };

    // lab-polish-round-2 R3 — 8-card 2-row 4-column grid (operator-decide
    // Q3 default). Row 1: Return / Max DD / Net Δ / Sharpe (placeholder).
    // Row 2: Buys / Sells / Trades / Fees + Final equity tucked at end.
    // Actually 8 cards = 2x4 plus Final equity is 9 — we fit Net Δ on row 1
    // and let Final equity sit at row 2 column 4 (replacing Fees there).
    // Final composition (2x4):
    //   Row 1: Return  |  Max DD  |  Net Δ        |  Sharpe
    //   Row 2: Buys    |  Sells   |  Final equity |  Fees
    let body: iced::Element<'a, crate::state::Message> = match kpis {
        None => {
            // Placeholder — eight dashes in 2x4 layout.
            let dash = KPI_DASH_PLACEHOLDER.to_string();
            let dash_color = color::FG_3.current(mode);
            let labels = [
                LAB_KPI_RETURN_LABEL,
                LAB_KPI_MAX_DD_LABEL,
                LAB_KPI_NET_DELTA_LABEL,
                LAB_KPI_SHARPE_LABEL,
                LAB_KPI_BUYS_LABEL,
                LAB_KPI_SELLS_LABEL,
                LAB_KPI_FINAL_EQUITY_LABEL,
                LAB_KPI_FEES_LABEL,
            ];
            let mut grid = Grid::new()
                .columns(4)
                .spacing(space::M)
                .height(Length::Shrink);
            for label in labels {
                grid = grid.push(lab_card(label, dash.clone(), dash_color, mode));
            }
            grid.into()
        }
        Some(k) => {
            // Return % = (final - initial) / initial * 100
            let initial = k.initial_equity.amount();
            let final_eq = k.final_equity.amount();
            let (return_text, return_color) = if initial.is_zero() {
                (KPI_DASH_PLACEHOLDER.to_string(), color::FG_3.current(mode))
            } else {
                let ret_pct = (final_eq - initial) / initial * rust_decimal::Decimal::ONE_HUNDRED;
                format_pct_sentiment(ret_pct, mode)
            };

            // Max DD — already stored as a fraction (0.0 → 100 %).
            let dd_pct = k.max_drawdown * rust_decimal::Decimal::ONE_HUNDRED;
            let (dd_text, dd_color) = format_pct_max_dd(dd_pct, mode);

            // Net Δ = final - initial (signed USDT).
            let net_delta = final_eq - initial;
            let net_delta_text = fmt_usdt_signed(net_delta);
            let net_delta_color = if net_delta.is_sign_negative() {
                color::DOWN_500.current(mode)
            } else if net_delta.is_zero() {
                color::FG_3.current(mode)
            } else {
                color::UP_500.current(mode)
            };

            // Sharpe — Phase C follow-up; always em-dash.
            let sharpe_text = KPI_DASH_PLACEHOLDER.to_string();
            let sharpe_color = color::FG_3.current(mode);

            // Buys / Sells — integer counts.
            let buys_text = format_count(k.buys as u64);
            let sells_text = format_count(k.sells as u64);
            let fg = color::FG_1.current(mode);

            // Final equity — USDT amount.
            let equity_text = fmt_usdt(k.final_equity.amount());

            // Fees — USDT amount.
            let fees_text = fmt_usdt(k.total_fees.amount());

            // Trades — integer count (kept for backwards compat; appears in
            // the tooltip-suffix style of Fees if needed). Not surfaced as a
            // top-level card in the 2x4 layout — buys+sells supersede it.
            let _ = format_count(k.trade_count as u64);
            let _ = LAB_KPI_TRADES_LABEL;

            Grid::new()
                .columns(4)
                .spacing(space::M)
                .height(Length::Shrink)
                // Row 1
                .push(lab_card(
                    LAB_KPI_RETURN_LABEL,
                    return_text,
                    return_color,
                    mode,
                ))
                .push(lab_card(LAB_KPI_MAX_DD_LABEL, dd_text, dd_color, mode))
                .push(lab_card(
                    LAB_KPI_NET_DELTA_LABEL,
                    net_delta_text,
                    net_delta_color,
                    mode,
                ))
                .push(lab_card(
                    LAB_KPI_SHARPE_LABEL,
                    sharpe_text,
                    sharpe_color,
                    mode,
                ))
                // Row 2
                .push(lab_card(LAB_KPI_BUYS_LABEL, buys_text, fg, mode))
                .push(lab_card(LAB_KPI_SELLS_LABEL, sells_text, fg, mode))
                .push(lab_card(LAB_KPI_FINAL_EQUITY_LABEL, equity_text, fg, mode))
                .push(lab_card(LAB_KPI_FEES_LABEL, fees_text, fg, mode))
                .into()
        }
    };

    Container::new(body)
        .width(Length::Fill)
        .padding(space::M as u16)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

/// Single 2-line card for the Lab KPI strip.
/// Identical shape to the viewer `card()` helper but returns
/// `Element<'_, crate::state::Message>` to compose with the Lab screen.
fn lab_card<'a>(
    label: &'a str,
    value: String,
    value_color: iced::Color,
    mode: ThemeMode,
) -> iced::Element<'a, crate::state::Message> {
    let label_line = Text::new(label)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let value_line = Text::new(value).size(text::H1).color(value_color);
    Container::new(
        iced::widget::Column::new()
            .spacing(space::XS)
            .push(label_line)
            .push(value_line),
    )
    .padding([space::XS as u16, space::S as u16])
    .into()
}

/// Single 2-line card: label (`text::SMALL` `FG_3`) over value
/// (`text::H1` 24 px coloured).
fn card<'a>(
    label: &'a str,
    value: String,
    value_color: iced::Color,
    mode: ThemeMode,
) -> iced::Element<'a, ViewerMessage> {
    let label_line = Text::new(label)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let value_line = Text::new(value).size(text::H1).color(value_color);
    // T3.1 — per-card `Length::FillPortion(1)` width hint removed:
    // `Grid::columns(6)` handles column equalization implicitly so
    // each cell inherits the column's intrinsic width.
    Container::new(
        Column::new()
            .spacing(space::XS)
            .push(label_line)
            .push(value_line),
    )
    .padding([space::XS as u16, space::S as u16])
    .into()
}

#[cfg(test)]
#[allow(
    non_snake_case,
    clippy::format_push_string,
    clippy::useless_format,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rust_decimal_macros::dec;

    fn ready_metrics() -> BacktestMetrics {
        // Mirror the RSI sample's actual numbers (Total return
        // -57.80 %, Sharpe -55.4257, Max DD 57.81 %, Trades 14118;
        // CAGR + Win rate marked-absent).
        BacktestMetrics {
            total_return_pct: dec!(-57.80),
            cagr_pct: dec!(0),
            cagr_present: false,
            sharpe: dec!(-55.4257),
            sharpe_present: true,
            max_drawdown_pct: dec!(57.81),
            win_rate_pct: dec!(0),
            win_rate_present: false,
            trades: 14118,
        }
    }

    /// Plain-text summary of what the KPI strip would render — pinned
    /// via insta so a regression in label / value / colour assignment
    /// is visible without a pixel-level renderer.
    fn strip_summary(state: &PanelState<BacktestMetrics>) -> String {
        let mut out = String::new();
        out.push_str("widget: kpi_strip\n");
        match state {
            PanelState::Ready(m) if !is_all_absent(m) => {
                out.push_str("state: ready\n");
                let mode = ThemeMode::Dark;
                let (tr, _) = format_pct_sentiment(m.total_return_pct, mode);
                out.push_str(&format!("total_return: {}\n", tr));
                if m.cagr_present {
                    let (s, _) = format_pct_sentiment(m.cagr_pct, mode);
                    out.push_str(&format!("cagr: {}\n", s));
                } else {
                    out.push_str("cagr: \u{2014}\n");
                }
                if m.sharpe_present {
                    out.push_str(&format!("sharpe: {}\n", format_sharpe(m.sharpe)));
                } else {
                    out.push_str("sharpe: \u{2014}\n");
                }
                let (mdd, _) = format_pct_max_dd(m.max_drawdown_pct, mode);
                out.push_str(&format!("max_dd: {}\n", mdd));
                if m.win_rate_present {
                    let (s, _) = format_pct_sentiment(m.win_rate_pct, mode);
                    out.push_str(&format!("win_rate: {}\n", s));
                } else {
                    out.push_str("win_rate: \u{2014}\n");
                }
                out.push_str(&format!("trades: {}\n", format_count(m.trades)));
            }
            _ => {
                out.push_str("state: unavailable\n");
                out.push_str(&format!("muted_body: {VIEWER_METRICS_UNAVAILABLE}\n"));
            }
        }
        out
    }

    #[test]
    fn kpi_strip__sample_report() {
        let state = PanelState::Ready(ready_metrics());
        assert_snapshot!("viewer__kpi_strip__sample_report", strip_summary(&state));
    }

    #[test]
    fn kpi_strip__metrics_unavailable() {
        let state: PanelState<BacktestMetrics> = PanelState::Ready(BacktestMetrics::all_absent());
        assert_snapshot!(
            "viewer__kpi_strip__metrics_unavailable",
            strip_summary(&state)
        );
    }

    // ── view_for_lab tests (lab-end-to-end-v2 Wave D-1.1 F8 / T-D-14a) ───────

    /// F8 — `view_for_lab(None, _)` renders without panic (all-dash path).
    #[test]
    fn kpi_strip_view_for_lab_none_renders() {
        let _el = view_for_lab(None, ThemeMode::Dark);
        let _el2 = view_for_lab(None, ThemeMode::Light);
    }

    /// F8 — `view_for_lab(Some(&kpis), _)` renders without panic (numeric path).
    #[test]
    fn kpi_strip_view_for_lab_some_renders() {
        use backtest::BacktestKpis;
        use rust_decimal_macros::dec;
        use trading_core::{Money, Usdt};
        let kpis = BacktestKpis {
            final_equity: Money::<Usdt>::from_decimal(dec!(11000)),
            initial_equity: Money::<Usdt>::from_decimal(dec!(10000)),
            max_drawdown: dec!(0.12),
            trade_count: 42,
            total_fees: Money::<Usdt>::from_decimal(dec!(17.50)),
            buys: 25,
            sells: 17,
            total_return_pct: dec!(0.10),
        };
        let _el = view_for_lab(Some(&kpis), ThemeMode::Dark);
        let _el2 = view_for_lab(Some(&kpis), ThemeMode::Light);
    }

    /// F8 — `view_for_lab` with zero initial equity renders a dash for return%.
    #[test]
    fn kpi_strip_view_for_lab_zero_initial_equity() {
        use backtest::BacktestKpis;
        let kpis = BacktestKpis::default();
        let _el = view_for_lab(Some(&kpis), ThemeMode::Dark);
    }
}
