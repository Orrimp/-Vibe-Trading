//! Drawdown-band canvas widget — Phase 4 (T1807 / R7).
//!
//! Composes the same `widgets::canvas_chart` core as the equity
//! curve. Inverted Y axis — 0 at top, `max_drawdown_pct` at bottom
//! (drawdown grows downward). `DOWN_500` polyline + `DOWN_500 @ 0.18`
//! fill. ~100 px container height per R7.3 / R9.4.
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

/// Fixed container height (R7.3 / R9.4 — Lumen ratio: ~240 px curve
/// over ~100 px band).
const BAND_HEIGHT_PX: f32 = 100.0;

/// Render the drawdown-band canvas over a `PanelState<EquitySeries>`.
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
            let t = iced::widget::Text::new(body)
                .size(crate::theme::text::BODY)
                .color(color::FG_3.current(mode));
            Container::new(t)
                .width(Length::Fill)
                .height(Length::Fixed(BAND_HEIGHT_PX))
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
        .height(Length::Fixed(BAND_HEIGHT_PX))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            ..Default::default()
        })
        .into()
}

fn canvas_view<'a>(series: EquitySeries, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    let program = DrawdownBandProgram { series, mode };
    let canvas: Canvas<DrawdownBandProgram, ViewerMessage> = Canvas::new(program)
        .width(Length::Fill)
        .height(Length::Fixed(BAND_HEIGHT_PX));
    Container::new(canvas)
        .width(Length::Fill)
        .height(Length::Fixed(BAND_HEIGHT_PX))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

struct DrawdownBandProgram {
    series: EquitySeries,
    mode: ThemeMode,
}

impl canvas::Program<ViewerMessage> for DrawdownBandProgram {
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

        draw_gridlines(&mut frame, inner, border);

        if self.series.points.is_empty() || inner.width <= 0.0 || inner.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        // Y range: drawdown grows downward — 0 at top,
        // `max_drawdown_pct` at the bottom. The polyline-with-fill
        // primitive paints in screen coords; we map drawdown_pct
        // directly to a (top..bottom) Y position so the fill polygon
        // closes down to the baseline naturally.
        let max_dd = self.series.max_drawdown_pct.to_f32().unwrap_or(0.0);
        let y_max = max_dd.max(1e-6) * (1.0 + RANGE_PAD_FRACTION);

        let n = self.series.points.len();
        let denom = if n <= 1 { 1.0 } else { (n - 1) as f32 };
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(n);
        for (i, p) in self.series.points.iter().enumerate() {
            let frac_x = if n <= 1 { 0.0 } else { i as f32 / denom };
            let x = inner.x + frac_x * inner.width;
            let dd = p.drawdown_pct.to_f32().unwrap_or(0.0);
            let frac_y = (dd / y_max).clamp(0.0, 1.0);
            // 0 drawdown lands at the top (inner.y); max-DD lands at
            // the bottom (inner.y + inner.height). NOT flipped.
            let y = inner.y + frac_y * inner.height;
            points.push((x, y));
        }

        polyline_with_fill(
            &mut frame,
            inner,
            &points,
            color::DOWN_500.current(self.mode),
            color::DOWN_500.current(self.mode),
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
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Money, Timestamp, Usdt};

    fn fixture_series() -> EquitySeries {
        let mut pts = Vec::with_capacity(60);
        for i in 0..30i64 {
            pts.push((
                Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(i * 60)),
                Money::<Usdt>::from_decimal(dec!(100000) - dec!(1000) * Decimal::from(i)),
            ));
        }
        for i in 0..30i64 {
            let v = dec!(70000) - dec!(1000) * Decimal::from(i);
            pts.push((
                Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds((30 + i) * 60)),
                Money::<Usdt>::from_decimal(v.max(dec!(42195))),
            ));
        }
        EquitySeries::from_points(pts).expect("from_points ok")
    }

    fn band_summary(state: &PanelState<EquitySeries>) -> String {
        let mut out = String::new();
        out.push_str("widget: drawdown_band\n");
        out.push_str("height_px: 100\n");
        out.push_str("gridlines: 5\n");
        out.push_str("y_axis: inverted (0 top, max_dd bottom)\n");
        match state {
            PanelState::Ready(s) if !s.points.is_empty() => {
                out.push_str("state: ready\n");
                out.push_str(&format!("points: {}\n", s.points.len()));
                out.push_str(&format!("max_dd: {}\n", s.max_drawdown_pct));
                out.push_str("line_color: DOWN_500\n");
                out.push_str("fill_color: DOWN_500\n");
                out.push_str("fill_alpha: 0.18\n");
            }
            _ => {
                out.push_str("state: empty\n");
                out.push_str(&format!("body: {VIEWER_NO_EQUITY_DATA}\n"));
            }
        }
        out
    }

    #[test]
    fn drawdown_band__sample_report() {
        let state = PanelState::Ready(fixture_series());
        assert_snapshot!("viewer__drawdown_band__sample_report", band_summary(&state));
    }
}
