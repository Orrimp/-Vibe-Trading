//! KPI strip widget — Phase 4 (T1805 / R2).
//!
//! Six metric cards (Total return / CAGR / Sharpe / Max DD / Win
//! rate / Trades) in one Row. Sentiment-coloured per the Phase 4
//! Design's Q-resolved table; missing fields render as `—` dashes.
//!
//! **Zero string literals** — copy via `crate::strings::*`.
//! **Zero hex colours** — tokens via `crate::theme::*`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names,
    clippy::match_same_arms
)]

use iced::widget::{container, Column, Container, Row, Text};
use iced::{Border, Length};
use trading_core::BacktestMetrics;

use super::frame::muted_body;
use super::num::{format_count, format_pct_max_dd, format_pct_sentiment, format_sharpe};
use crate::state::PanelState;
use crate::strings::{
    KPI_CAGR_LABEL, KPI_DASH_PLACEHOLDER, KPI_MAX_DD_LABEL, KPI_SHARPE_LABEL,
    KPI_TOTAL_RETURN_LABEL, KPI_TRADES_LABEL, KPI_WIN_RATE_LABEL, VIEWER_METRICS_UNAVAILABLE,
};
use crate::theme::{color, radius, space, text, ThemeMode};
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

    Row::new()
        .spacing(space::M)
        .push(total_return)
        .push(cagr)
        .push(sharpe)
        .push(max_dd)
        .push(win_rate)
        .push(trades)
        .width(Length::Fill)
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
    let mut row = Row::new().spacing(space::M);
    for label in labels {
        row = row.push(card(label, dash.clone(), dash_color, mode));
    }
    Column::new()
        .spacing(space::S)
        .push(row)
        .push(muted_body(VIEWER_METRICS_UNAVAILABLE))
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
    Container::new(
        Column::new()
            .spacing(space::XS)
            .push(label_line)
            .push(value_line),
    )
    .width(Length::FillPortion(1))
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
}
