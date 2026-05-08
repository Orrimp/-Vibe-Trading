//! Compact sparkline canvas widget — Phase 4 (T1809 / R13.2).
//!
//! Cockpit Strategies-detail surface — line-only, no fill, no
//! gridlines, no axes. 120 × 36 px. Composes the
//! `widgets::canvas_chart` core with `fill_alpha = 0.0`.
//!
//! Caller (T1811) is responsible for the empty / loading / error
//! branches; this widget assumes a non-empty `EquitySeries`.

#![allow(
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names
)]

use iced::widget::canvas::{self, Frame, Geometry};
use iced::widget::{container, Canvas, Container};
use iced::{mouse, Length, Rectangle, Renderer};
use rust_decimal::prelude::ToPrimitive;
use trading_core::EquitySeries;

use super::canvas_chart::polyline_with_fill;
use crate::state::Message;
use crate::theme::{color, ThemeMode};

const SPARKLINE_WIDTH_PX: f32 = 120.0;
const SPARKLINE_HEIGHT_PX: f32 = 36.0;
/// Tighter padding than the full equity curve — 3 % matches the
/// design's "minimal padding" call (R13.2).
const SPARKLINE_PAD_FRACTION: f32 = 0.03;

/// Render the cockpit Strategies-detail sparkline. Consumer is the
/// cockpit (`crate::state::Message`), not the viewer.
#[must_use]
pub fn view<'a>(series: &'a EquitySeries, mode: ThemeMode) -> iced::Element<'a, Message> {
    let program = SparklineProgram {
        series: series.clone(),
        mode,
    };
    let canvas: Canvas<SparklineProgram, Message> = Canvas::new(program)
        .width(Length::Fixed(SPARKLINE_WIDTH_PX))
        .height(Length::Fixed(SPARKLINE_HEIGHT_PX));
    Container::new(canvas)
        .width(Length::Fixed(SPARKLINE_WIDTH_PX))
        .height(Length::Fixed(SPARKLINE_HEIGHT_PX))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            ..Default::default()
        })
        .into()
}

struct SparklineProgram {
    series: EquitySeries,
    mode: ThemeMode,
}

impl canvas::Program<Message> for SparklineProgram {
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
        let inner = Rectangle {
            x: 1.0,
            y: 1.0,
            width: (bounds.size().width - 2.0).max(0.0),
            height: (bounds.size().height - 2.0).max(0.0),
        };

        if self.series.points.is_empty() || inner.width <= 0.0 || inner.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

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
        let pad = span * SPARKLINE_PAD_FRACTION;
        let y_min = min_eq - pad;
        let y_max = max_eq + pad;

        let n = self.series.points.len();
        let denom = if n <= 1 { 1.0 } else { (n - 1) as f32 };
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(n);
        for (i, p) in self.series.points.iter().enumerate() {
            let frac_x = if n <= 1 { 0.0 } else { i as f32 / denom };
            let x = inner.x + frac_x * inner.width;
            let v = p.equity.amount().to_f32().unwrap_or(0.0);
            let frac_y = (v - y_min) / (y_max - y_min);
            let y = inner.y + (1.0 - frac_y) * inner.height;
            points.push((x, y));
        }

        // Line-only: fill_alpha = 0.0 (R13.2 — sparkline reads
        // cleanest without fill).
        polyline_with_fill(
            &mut frame,
            inner,
            &points,
            color::ACCENT.current(self.mode),
            color::ACCENT.current(self.mode),
            0.0,
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

    fn fixture_120pt_series() -> EquitySeries {
        // Deterministic 120-point series: ramp up then down.
        let mut pts = Vec::with_capacity(120);
        for i in 0..60i64 {
            pts.push((
                Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(i * 60)),
                Money::<Usdt>::from_decimal(dec!(1000) + Decimal::from(i) * dec!(10)),
            ));
        }
        for i in 0..60i64 {
            pts.push((
                Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds((60 + i) * 60)),
                Money::<Usdt>::from_decimal(dec!(1600) - Decimal::from(i) * dec!(5)),
            ));
        }
        EquitySeries::from_points(pts).expect("from_points ok")
    }

    fn sparkline_summary(s: &EquitySeries) -> String {
        let mut out = String::new();
        out.push_str("widget: sparkline\n");
        out.push_str(&format!("width_px: {SPARKLINE_WIDTH_PX}\n"));
        out.push_str(&format!("height_px: {SPARKLINE_HEIGHT_PX}\n"));
        out.push_str(&format!("points: {}\n", s.points.len()));
        out.push_str("line_color: ACCENT\n");
        out.push_str("fill_alpha: 0.0\n");
        out.push_str(&format!(
            "peak: {}\ntrough: {}\n",
            s.peak.amount(),
            s.trough.amount()
        ));
        out
    }

    #[test]
    fn sparkline__120pt() {
        let s = fixture_120pt_series();
        assert_snapshot!("widgets__sparkline__120pt", sparkline_summary(&s));
    }
}
