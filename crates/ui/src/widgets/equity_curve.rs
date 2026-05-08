//! Equity-curve canvas widget — Phase 4 (T1806 / R4).
//!
//! Composes the `widgets::canvas_chart` core. Renders a polyline +
//! filled area + 5 horizontal gridlines over an `EquitySeries`. Index-
//! based X axis (Q5 / R6.2); 5 % Y padding; `ACCENT` polyline +
//! `UP_500 @ 0.18` fill.
//!
//! **Zero string literals** — copy via `crate::strings::*`.
//! **Zero hex colours** — tokens via `crate::theme::*`.

#![allow(
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names,
    clippy::match_same_arms
)]

use iced::widget::canvas::{self, Frame, Geometry};
use iced::widget::{container, Canvas, Container};
use iced::{mouse, Length, Rectangle, Renderer};
use rust_decimal::prelude::ToPrimitive;
use trading_core::EquitySeries;

use super::canvas_chart::{
    draw_gridlines, inner_rect, polyline_with_fill, with_alpha, RANGE_PAD_FRACTION,
};
use super::frame::muted_body;
use crate::state::PanelState;
use crate::strings::{VIEWER_EQUITY_UNAVAILABLE_PREFIX, VIEWER_NO_EQUITY_DATA};
use crate::theme::{color, ThemeMode};
use crate::viewer::ViewerMessage;

/// Fixed container height (R9.4 layout — ~240 px for the equity
/// curve, contrasting with the drawdown band's ~100 px).
const CURVE_HEIGHT_PX: f32 = 240.0;

/// Render the equity-curve canvas over a `PanelState<EquitySeries>`.
#[must_use]
pub fn view<'a>(
    series: &'a PanelState<EquitySeries>,
    mode: ThemeMode,
) -> iced::Element<'a, ViewerMessage> {
    match series {
        PanelState::Loading => empty_with_label(VIEWER_NO_EQUITY_DATA, mode),
        PanelState::Empty => empty_with_label(VIEWER_NO_EQUITY_DATA, mode),
        PanelState::Error(msg) => {
            let body = format!("{VIEWER_EQUITY_UNAVAILABLE_PREFIX}{msg}");
            // `muted_body` takes &str; we have an owned string.
            // Render via a Text node with the muted FG_3 colour.
            let t = iced::widget::Text::new(body)
                .size(crate::theme::text::BODY)
                .color(color::FG_3.current(mode));
            Container::new(t)
                .width(Length::Fill)
                .height(Length::Fixed(CURVE_HEIGHT_PX))
                .into()
        }
        PanelState::Ready(s) => {
            if s.points.is_empty() {
                empty_with_label(VIEWER_NO_EQUITY_DATA, mode)
            } else {
                canvas_view(s.clone(), mode)
            }
        }
    }
}

fn empty_with_label<'a>(label: &'a str, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    Container::new(muted_body(label))
        .width(Length::Fill)
        .height(Length::Fixed(CURVE_HEIGHT_PX))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            ..Default::default()
        })
        .into()
}

fn canvas_view<'a>(series: EquitySeries, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    let program = EquityCurveProgram { series, mode };
    let canvas: Canvas<EquityCurveProgram, ViewerMessage> = Canvas::new(program)
        .width(Length::Fill)
        .height(Length::Fixed(CURVE_HEIGHT_PX));
    Container::new(canvas)
        .width(Length::Fill)
        .height(Length::Fixed(CURVE_HEIGHT_PX))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

struct EquityCurveProgram {
    series: EquitySeries,
    mode: ThemeMode,
}

impl canvas::Program<ViewerMessage> for EquityCurveProgram {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let inner = inner_rect(bounds.size());
        let border = with_alpha(color::BORDER_1.current(self.mode), 0.4);

        // Five horizontal gridlines.
        draw_gridlines(&mut frame, inner, border);

        if self.series.points.is_empty() || inner.width <= 0.0 || inner.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        // Y range over equity values + 5 % padding.
        let mut min_eq = f32::MAX;
        let mut max_eq = f32::MIN;
        for p in &self.series.points {
            let v = p.equity.amount().to_f32().unwrap_or(0.0);
            if v < min_eq {
                min_eq = v;
            }
            if v > max_eq {
                max_eq = v;
            }
        }
        let span = (max_eq - min_eq).max(1e-6);
        let pad = span * RANGE_PAD_FRACTION;
        let y_min = min_eq - pad;
        let y_max = max_eq + pad;

        // Index-based X coordinates.
        let n = self.series.points.len();
        let denom = if n <= 1 { 1.0 } else { (n - 1) as f32 };
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(n);
        for (i, p) in self.series.points.iter().enumerate() {
            let frac_x = if n <= 1 { 0.0 } else { i as f32 / denom };
            let x = inner.x + frac_x * inner.width;
            let v = p.equity.amount().to_f32().unwrap_or(0.0);
            let frac_y = (v - y_min) / (y_max - y_min);
            // Y axis flipped: high values render near the top.
            let y = inner.y + (1.0 - frac_y) * inner.height;
            points.push((x, y));
        }

        polyline_with_fill(
            &mut frame,
            inner,
            &points,
            color::ACCENT.current(self.mode),
            color::UP_500.current(self.mode),
            0.18,
        );

        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
#[allow(
    non_snake_case,
    clippy::format_push_string,
    clippy::useless_format,
    clippy::uninlined_format_args,
    clippy::expect_used,
    clippy::unwrap_used
)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Money, Timestamp, Usdt};

    fn fixture_series() -> EquitySeries {
        // 60-point series matching the RSI report shape: peak
        // 100_000, trough 42_195, max-DD ≈ 0.5781.
        let mut pts = Vec::with_capacity(60);
        for i in 0..30i64 {
            pts.push((
                Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(i * 60)),
                Money::<Usdt>::from_decimal(dec!(100000) - dec!(1000) * Decimal::from(i)),
            ));
        }
        for i in 0..30i64 {
            // Continued slide to ~42195 at idx 59.
            let v = dec!(70000) - dec!(1000) * Decimal::from(i);
            pts.push((
                Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds((30 + i) * 60)),
                Money::<Usdt>::from_decimal(v.max(dec!(42195))),
            ));
        }
        EquitySeries::from_points(pts).expect("from_points ok")
    }

    fn curve_summary(state: &PanelState<EquitySeries>) -> String {
        let mut out = String::new();
        out.push_str("widget: equity_curve\n");
        out.push_str("height_px: 240\n");
        out.push_str("gridlines: 5\n");
        match state {
            PanelState::Ready(s) if !s.points.is_empty() => {
                out.push_str("state: ready\n");
                out.push_str(&format!("points: {}\n", s.points.len()));
                out.push_str(&format!(
                    "peak: {}\ntrough: {}\n",
                    s.peak.amount(),
                    s.trough.amount()
                ));
                out.push_str(&format!("max_dd: {}\n", s.max_drawdown_pct));
                out.push_str("line_color: ACCENT\n");
                out.push_str("fill_color: UP_500\n");
                out.push_str("fill_alpha: 0.18\n");
            }
            PanelState::Error(msg) => {
                out.push_str("state: error\n");
                out.push_str(&format!("body: Equity curve unavailable: {msg}\n"));
            }
            _ => {
                out.push_str("state: empty\n");
                out.push_str(&format!("body: {VIEWER_NO_EQUITY_DATA}\n"));
            }
        }
        out
    }

    use rust_decimal::Decimal;

    #[test]
    fn equity_curve__sample_report() {
        let state = PanelState::Ready(fixture_series());
        assert_snapshot!("viewer__equity_curve__sample_report", curve_summary(&state));
    }

    #[test]
    fn equity_curve__no_equity_data() {
        let state: PanelState<EquitySeries> = PanelState::Empty;
        assert_snapshot!(
            "viewer__equity_curve__no_equity_data",
            curve_summary(&state)
        );
    }
}
