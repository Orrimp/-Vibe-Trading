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

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::widget::{Canvas, Container, container};
use iced::{Length, Point, Rectangle, Renderer, mouse};
use rust_decimal::prelude::ToPrimitive;
use trading_core::EquitySeries;

use super::canvas_chart::{
    GRIDLINE_COUNT, RANGE_PAD_FRACTION, draw_gridlines, inner_rect_with_gutters,
    polyline_with_fill, with_alpha,
};
use super::chart::{format_time_axis_label, local_offset_or_utc, time_axis_tick_count};
use super::frame::muted_body;
use crate::state::PanelState;
use crate::strings::{VIEWER_EQUITY_UNAVAILABLE_PREFIX, VIEWER_NO_EQUITY_DATA};
use crate::theme::layout::{AXIS_GUTTER_PRICE_PX, AXIS_GUTTER_RIGHT_PX, AXIS_GUTTER_TIME_PX};
use crate::theme::{ThemeMode, color, space, text};
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
                canvas_view(s, mode)
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

/// 2-15 review M7 — the program BORROWS the series for the element's
/// lifetime. It used to take an owned `EquitySeries`, so `view()` deep-cloned
/// every point (up to `LIVE_EQUITY_BUFFER_CAP` = 2 880 `EquityPoint`s, each
/// carrying three `Decimal`s) on **every** frame — and `view` runs on every
/// message: ticks, latency updates, toasts, hovers. Borrowing removes the
/// clone outright; the tessellation itself still happens per frame (there is
/// no `canvas::Cache` — one would have to live on the model, since the program
/// is rebuilt each `view`).
fn canvas_view<'a>(series: &'a EquitySeries, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    let program = EquityCurveProgram { series, mode };
    let canvas: Canvas<EquityCurveProgram<'a>, ViewerMessage> = Canvas::new(program)
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

struct EquityCurveProgram<'a> {
    series: &'a EquitySeries,
    mode: ThemeMode,
}

impl canvas::Program<ViewerMessage> for EquityCurveProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        // cockpit-chart-cache Phase 1 — time the geometry-build body
        // (no-op when `chart-build-probe` is off).
        let _build_timer = super::chart_build_probe::BuildTimer::start();
        let mut frame = Frame::new(renderer, bounds.size());
        // T3019 — chart-canvas-overhaul v1.10.0 (Q7 viewer parity =
        // BOTH).  The viewer's equity curve adopts the same four-
        // sided gutter geometry as the cockpit chart so the USD
        // labels land in a dedicated LEFT gutter and the wall-clock
        // labels land in a dedicated BOTTOM gutter — no more
        // labels-overlapping-the-line at busy zones.  Legend = NO
        // for the viewer's single-series widget (architect-decided).
        let inner = inner_rect_with_gutters(
            bounds.size(),
            AXIS_GUTTER_PRICE_PX,
            AXIS_GUTTER_RIGHT_PX,
            0.0,
            AXIS_GUTTER_TIME_PX,
        );
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

        // cockpit-live-equity-render-guard (2026-06-11) — the render-verifiable
        // harness (`tests/live_equity_render.rs`) caught a panic in the
        // rasterizer (`lyon_path: assertion failed: p.y.is_finite()`) on a
        // **flat / single-point** series: when every equity is equal — e.g. the
        // FIRST live bar after boot (a 1-point Ready curve, the design renders
        // from ≥1 point), or any all-equal run — `max_eq == min_eq`, so
        // `pad = span * 0.05` underflows; near a ~100k equity value in f32,
        // `y_min` and `y_max` both round to the same float, making
        // `y_max - y_min == 0.0` and `frac_y = (v - y_min)/0 == NaN`. A NaN Y
        // is a non-finite `Point` that panics lyon → the whole cockpit crashes
        // the instant the live curve holds one point. Guard the denominator:
        // when the y-range collapses (non-finite or ≤ a tiny epsilon relative
        // to the values), render the line CENTERED (`frac_y = 0.5`) instead of
        // dividing by ~zero. A flat curve is a horizontal line through the
        // middle — correct, and crash-free.
        let y_range = y_max - y_min;
        // Epsilon scaled to the magnitude of the values so the guard fires for
        // a flat large-equity series (where absolute f32 deltas vanish) as well
        // as a flat near-zero series.
        let y_eps = max_eq.abs().max(1.0) * 1e-4;
        let y_range_degenerate = !y_range.is_finite() || y_range <= y_eps;

        // Price axis (USD labels in the LEFT gutter — T3019 / Q7).
        draw_price_axis(&mut frame, inner, (y_min, y_max), self.mode);
        // Time axis (wall-clock labels in the BOTTOM gutter — T3019).
        draw_time_axis(&mut frame, inner, self.series, self.mode);

        // Index-based X coordinates.
        let n = self.series.points.len();
        let denom = if n <= 1 { 1.0 } else { (n - 1) as f32 };
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(n);
        for (i, p) in self.series.points.iter().enumerate() {
            let frac_x = if n <= 1 { 0.0 } else { i as f32 / denom };
            let x = inner.x + frac_x * inner.width;
            let v = p.equity.amount().to_f32().unwrap_or(0.0);
            // Degenerate (flat) y-range → center the line; else map normally.
            // Both branches yield a FINITE `frac_y` (the panic fix).
            let frac_y = if y_range_degenerate {
                0.5
            } else {
                (v - y_min) / y_range
            };
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

/// Decimal places for a money axis label, chosen so that ADJACENT gridlines
/// read as different numbers (2-15 review L12).
///
/// The old fixed `{v:.0}` was sized for a multi-thousand-dollar backtest
/// equity range and is wrong for this product's headline scale: on the €200
/// forward budget a typical intraday range (200.10 → 200.60) rendered five
/// gridlines all reading "200"/"201" — an axis that carries no information at
/// exactly the scale the advisor ships with.
///
/// The step between gridlines is `span / (GRIDLINE_COUNT − 1)`; pick the
/// smallest precision that resolves it, capped at 3 (below which the label
/// stops fitting the 48 px gutter).
fn money_label_decimals(span: f32) -> usize {
    #[allow(clippy::cast_precision_loss)]
    let step = span.abs() / (GRIDLINE_COUNT - 1).max(1) as f32;
    if !step.is_finite() || step >= 10.0 {
        0
    } else if step >= 1.0 {
        1
    } else if step >= 0.1 {
        2
    } else {
        3
    }
}

/// T3019 — viewer-side price-axis draw pass.  Mirrors the cockpit
/// chart's `draw_price_axis` shape; label precision adapts to the rendered
/// range via [`money_label_decimals`] (whole dollars on a multi-thousand
/// backtest curve, cents on a €200 forward budget).
fn draw_price_axis(frame: &mut Frame, inner: Rectangle, range: (f32, f32), mode: ThemeMode) {
    let (min_v, max_v) = range;
    let decimals = money_label_decimals(max_v - min_v);
    #[allow(clippy::cast_precision_loss)]
    let denom = (GRIDLINE_COUNT - 1) as f32;
    let axis_color = color::FG_3.current(mode);
    let border = with_alpha(color::BORDER_1.current(mode), 0.4);
    #[allow(clippy::cast_precision_loss)]
    let micro = text::MICRO as f32;
    #[allow(clippy::cast_precision_loss)]
    let label_gap = space::XS as f32;
    let tick_len = 4.0_f32;

    // 1-px axis line at the inner rect's left edge.
    let axis_line = Path::new(|builder| {
        builder.move_to(Point::new(inner.x, inner.y));
        builder.line_to(Point::new(inner.x, inner.y + inner.height));
    });
    frame.stroke(
        &axis_line,
        Stroke::default().with_color(border).with_width(1.0),
    );

    for i in 0..GRIDLINE_COUNT {
        #[allow(clippy::cast_precision_loss)]
        let frac = i as f32 / denom;
        let y = inner.y + frac * inner.height;
        let v = max_v - frac * (max_v - min_v);
        let tick_path = Path::new(|builder| {
            builder.move_to(Point::new(inner.x - tick_len, y));
            builder.line_to(Point::new(inner.x, y));
        });
        frame.stroke(
            &tick_path,
            Stroke::default().with_color(border).with_width(1.0),
        );
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: format!("{v:.decimals$}"),
            position: Point::new(inner.x - tick_len - label_gap, y),
            color: axis_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }
}

/// T3019 — viewer-side time-axis draw pass.  Same adaptive tick
/// spacing as the cockpit `chart::draw_time_axis`; labels formatted
/// as `HH:MM` against UTC (per `local_offset_or_utc` deterministic
/// branch under `cfg(test)` and the production fallback).
fn draw_time_axis(frame: &mut Frame, inner: Rectangle, series: &EquitySeries, mode: ThemeMode) {
    let n = series.points.len();
    if n == 0 || inner.width <= 0.0 {
        return;
    }
    let axis_color = color::FG_3.current(mode);
    let border = with_alpha(color::BORDER_1.current(mode), 0.4);
    #[allow(clippy::cast_precision_loss)]
    let micro = text::MICRO as f32;
    #[allow(clippy::cast_precision_loss)]
    let label_gap = space::XS as f32;
    let tick_len = 4.0_f32;
    let intervals = time_axis_tick_count(inner.width, n);
    if intervals == 0 {
        return;
    }
    let offset = local_offset_or_utc();
    // Total series span drives the adaptive label granularity (HH:MM for an
    // intraday session, "MMM DD" for the multi-month Baseline curve, "MMM 'YY"
    // for the 2-year compressed replay) — cockpit-live-axis-density-fix.
    let span_seconds = match (series.points.first(), series.points.last()) {
        (Some(a), Some(b)) => {
            (b.ts.inner().unix_timestamp() - a.ts.inner().unix_timestamp()).max(0)
        }
        _ => 0,
    };

    for i in 0..=intervals {
        let idx = if intervals == 0 {
            0
        } else {
            (i * (n - 1)) / intervals
        };
        let Some(pt) = series.points.get(idx) else {
            continue;
        };
        let frac_x = if n <= 1 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                idx as f32 / (n - 1) as f32
            }
        };
        let x = inner.x + frac_x * inner.width;
        let tick_path = Path::new(|builder| {
            builder.move_to(Point::new(x, inner.y + inner.height));
            builder.line_to(Point::new(x, inner.y + inner.height + tick_len));
        });
        frame.stroke(
            &tick_path,
            Stroke::default().with_color(border).with_width(1.0),
        );
        let local_ts = pt.ts.inner().to_offset(offset);
        let label = format_time_axis_label(local_ts, span_seconds);
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: label,
            position: Point::new(x, inner.y + inner.height + tick_len + label_gap),
            color: axis_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Top.into(),
            ..CanvasText::default()
        });
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

    /// **2-15 review L12.** The money-axis label precision must adapt to the
    /// rendered range, or the €200 forward budget draws five gridlines that
    /// all read "200".
    #[test]
    fn money_label_decimals_adapt_to_range() {
        // A multi-thousand backtest equity range keeps whole dollars.
        assert_eq!(money_label_decimals(60_000.0), 0);
        assert_eq!(money_label_decimals(40.1), 0); // step ≈ 10.03
        // A ~€200 forward budget's typical intraday range → cents.
        assert_eq!(money_label_decimals(0.5), 2); // step 0.125 → 200.10 … 200.60
        // Mid ranges resolve to tenths…
        assert_eq!(money_label_decimals(20.0), 1); // step 5.0
        // …and a pathologically tight range gets the 3-decimal cap.
        assert_eq!(money_label_decimals(0.004), 3);
        // Degenerate input must not panic or produce a silly precision.
        assert_eq!(money_label_decimals(f32::NAN), 0);
        assert_eq!(money_label_decimals(0.0), 3);
    }

    /// The rendered label STRINGS at the €200 scale are distinct — the actual
    /// operator-visible symptom, asserted on the same expression
    /// `draw_price_axis` formats with (not on the helper's return value).
    #[test]
    fn forward_budget_scale_gridlines_are_distinguishable() {
        let (min_v, max_v) = (200.10_f32, 200.60_f32);
        let decimals = money_label_decimals(max_v - min_v);
        #[allow(clippy::cast_precision_loss)]
        let denom = (GRIDLINE_COUNT - 1) as f32;
        let labels: Vec<String> = (0..GRIDLINE_COUNT)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let frac = i as f32 / denom;
                let v = max_v - frac * (max_v - min_v);
                format!("{v:.decimals$}")
            })
            .collect();
        let unique: std::collections::BTreeSet<&String> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "every gridline label must be distinct at the €200 forward-budget \
             scale; got {labels:?}"
        );
        assert!(labels.contains(&"200.60".to_string()), "{labels:?}");
        assert!(labels.contains(&"200.10".to_string()), "{labels:?}");
    }
}
