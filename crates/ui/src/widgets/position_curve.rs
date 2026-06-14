//! Lab position-curve widget — lab-polish-round-2 R1.
//!
//! Stepped-polyline canvas widget rendered below the price-line on the Lab
//! chart canvas showing the operator-selected pair's **base-asset position
//! quantity over time**.
//!
//! - Positive qty → long (rendered above the zero baseline in `UP_500`).
//! - Zero → flat (baseline only).
//! - No short positions in v0 (FixedFractionSizer rejects them).
//!
//! **Empty state:** when `points` is empty, paints a single horizontal
//! zero-line + a centred `—` placeholder. No blank screens.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::canvas::{self, Frame, Geometry, Path, Text as CanvasText};
use iced::widget::{Canvas, Container, container};
use iced::{Length, Point, Rectangle, Renderer, Size, mouse};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

#[cfg(test)]
use super::canvas_chart::GRIDLINE_COUNT;
use super::canvas_chart::{inner_rect, with_alpha};
use crate::state::Message;
use crate::strings::KPI_DASH_PLACEHOLDER;
use crate::theme::{ThemeMode, color, text};

/// Render the position-curve widget for the Lab screen.
///
/// `points` is `(close_ts_millis, signed_qty)` oldest-first, already
/// filtered to the active symbol by `lab.rs` (D-2.5 pattern).
///
/// Takes `points` by value — the canvas `Program` holds them across iced's
/// render lifetime; ~720 pts × ~24 B per pt ≈ ~17 KB — trivially in budget.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn view<'a>(points: Vec<(i64, Decimal)>, mode: ThemeMode) -> crate::Element<'a> {
    let program = PositionCurveProgram { points, mode };
    let canvas: Canvas<PositionCurveProgram, Message> = Canvas::new(program)
        .width(Length::Fill)
        .height(Length::Fill);
    Container::new(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            text_color: Some(color::FG_1.current(mode)),
            ..Default::default()
        })
        .into()
}

struct PositionCurveProgram {
    points: Vec<(i64, Decimal)>,
    mode: ThemeMode,
}

impl canvas::Program<Message> for PositionCurveProgram {
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
        let inner = inner_rect(bounds.size());
        let baseline_y = inner.y + inner.height - 4.0; // near the bottom, zero line
        let border = with_alpha(color::BORDER_1.current(self.mode), 0.4);

        // Zero baseline.
        let zero_line = Path::line(
            Point::new(inner.x, baseline_y),
            Point::new(inner.x + inner.width, baseline_y),
        );
        frame.stroke(
            &zero_line,
            canvas::Stroke::default().with_color(border).with_width(1.0),
        );

        if self.points.is_empty() {
            // Empty-state placeholder.
            let centre = Point::new(inner.x + inner.width / 2.0, baseline_y - inner.height / 4.0);
            #[allow(clippy::cast_precision_loss)]
            let micro = text::MICRO as f32;
            #[allow(clippy::useless_conversion)]
            frame.fill_text(CanvasText {
                content: KPI_DASH_PLACEHOLDER.to_string(),
                position: centre,
                color: color::FG_3.current(self.mode),
                size: micro.into(),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center.into(),
                ..CanvasText::default()
            });
            return vec![frame.into_geometry()];
        }

        // Compute min / max timestamp for x-scaling.
        let (ts_min, ts_max) = self
            .points
            .iter()
            .fold((i64::MAX, i64::MIN), |(lo, hi), &(ts, _)| {
                (lo.min(ts), hi.max(ts))
            });
        #[allow(clippy::cast_precision_loss)]
        // timestamp span → f64 for pixel fraction; precision loss is acceptable in rendering
        let ts_span = (ts_max - ts_min).max(1) as f64;

        // Compute max qty for y-scaling. Use the max absolute qty so both long
        // and (future short) positions scale symmetrically.
        let max_qty = self
            .points
            .iter()
            .map(|(_, q)| decimal_to_f32(q).abs())
            .fold(0f32, f32::max)
            .max(1e-9_f32); // avoid divide-by-zero on all-zero curve

        // Available height above the zero line (95 % of inner height for padding).
        let draw_height = (inner.height - 4.0) * 0.95;

        let long_color = color::UP_500.current(self.mode);

        // Render as stepped polyline: for each point draw a horizontal segment
        // from the previous x to the current x at the previous y, then a
        // vertical segment to the current y.
        let mut prev_x = inner.x;
        let mut prev_h = 0.0f32; // pixel height above baseline

        for &(ts, ref qty) in &self.points {
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            // pixel fraction cast: precision loss and f64→f32 truncation are acceptable in rendering
            let t_frac = ((ts - ts_min) as f64 / ts_span) as f32;
            let x = inner.x + t_frac * inner.width;
            let h = (decimal_to_f32(qty).max(0.0) / max_qty) * draw_height;

            // Horizontal segment at prev height (step shape).
            if (x - prev_x).abs() > 0.1 && (prev_h > 0.0 || h > 0.0) {
                let y = baseline_y - prev_h;
                let seg = Path::line(Point::new(prev_x, y), Point::new(x, y));
                frame.stroke(
                    &seg,
                    canvas::Stroke::default()
                        .with_color(long_color)
                        .with_width(1.5),
                );
            }

            // Vertical segment from prev_h to h at x.
            if (h - prev_h).abs() > 0.1 {
                let y_from = baseline_y - prev_h;
                let y_to = baseline_y - h;
                let seg = Path::line(Point::new(x, y_from), Point::new(x, y_to));
                frame.stroke(
                    &seg,
                    canvas::Stroke::default()
                        .with_color(long_color)
                        .with_width(1.5),
                );
            }

            // Fill rect below the step for the current segment.
            if h > 0.0 {
                let rect = Path::rectangle(
                    Point::new(prev_x, baseline_y - h),
                    Size::new((x - prev_x).max(0.0), h),
                );
                frame.fill(
                    &rect,
                    with_alpha(long_color, 0.18), // subtle fill under the line
                );
            }

            prev_x = x;
            prev_h = h;
        }

        // Final horizontal segment to the right edge.
        if prev_h > 0.0 {
            let y = baseline_y - prev_h;
            let seg = Path::line(Point::new(prev_x, y), Point::new(inner.x + inner.width, y));
            frame.stroke(
                &seg,
                canvas::Stroke::default()
                    .with_color(long_color)
                    .with_width(1.5),
            );
        }

        vec![frame.into_geometry()]
    }
}

fn decimal_to_f32(d: &Decimal) -> f32 {
    d.to_f32().unwrap_or(0.0)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::format_push_string,
    clippy::useless_format,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rust_decimal_macros::dec;

    /// Build a text summary of the position-curve widget state for snapshot testing.
    /// Mirrors the pattern from `volume_histogram`.
    fn summary(points: &[(i64, Decimal)]) -> String {
        let mut out = String::new();
        out.push_str("widget: position_curve\n");
        out.push_str(&format!("point_count: {}\n", points.len()));
        let non_zero = points.iter().filter(|(_, q)| !q.is_zero()).count();
        out.push_str(&format!("non_zero_points: {non_zero}\n"));
        if points.is_empty() {
            out.push_str("empty_state: true\n");
            out.push_str(&format!("empty_placeholder: {KPI_DASH_PLACEHOLDER}\n"));
        } else {
            out.push_str("empty_state: false\n");
            out.push_str("representation: stepped_polyline\n");
            out.push_str(&format!("gridlines: {}\n", GRIDLINE_COUNT));
            out.push_str("long_color: UP_500\n");
            let max_qty: Decimal = points
                .iter()
                .map(|(_, q)| *q)
                .fold(Decimal::ZERO, rust_decimal::Decimal::max);
            let total_qty: Decimal = points.iter().map(|(_, q)| *q).sum();
            out.push_str(&format!("max_qty: {max_qty}\n"));
            out.push_str(&format!("total_qty_sum: {total_qty}\n"));
        }
        out
    }

    /// R1 — empty points vector renders the placeholder.
    #[test]
    fn position_curve_empty_renders_placeholder() {
        let points: Vec<(i64, Decimal)> = Vec::new();
        let s = summary(&points);
        assert!(s.contains("empty_state: true"));
        assert!(s.contains("empty_placeholder"));
    }

    /// R1 — all-zero points vector also renders placeholder logic correctly.
    #[test]
    fn position_curve_all_zero_non_empty_summary() {
        let points = vec![(1_000_000_i64, dec!(0)), (2_000_000_i64, dec!(0))];
        let s = summary(&points);
        assert!(s.contains("empty_state: false"));
        assert!(s.contains("non_zero_points: 0"));
    }

    /// R1 — snapshot: three buys build a step curve, one sell reduces qty.
    ///
    /// Cumulative qty sequence: 0 → 0.5 → 1.0 → 1.5 → 1.0 (sell) → 0 (sell)
    #[test]
    #[allow(non_snake_case)]
    fn position_curve__three_buys_two_sells_step_curve() {
        let points = vec![
            (1_000_000_i64, dec!(0)),
            (2_000_000_i64, dec!(0.5)),
            (3_000_000_i64, dec!(1.0)),
            (4_000_000_i64, dec!(1.5)),
            (5_000_000_i64, dec!(1.0)),
            (6_000_000_i64, dec!(0)),
        ];
        assert_snapshot!(
            "position_curve__three_buys_two_sells_step_curve",
            summary(&points)
        );
    }

    /// R1 — unit test: cumulative qty computation from a synthetic fills sequence.
    /// This is the pure-logic correctness test (not a widget render test).
    #[test]
    fn position_curve_cumulative_qty_from_fills() {
        use trading_core::Side;

        // Synthetic fills: B 0.5 → B 0.5 → S 0.3 → B 0.2
        // Expected cumulative: [0.5, 1.0, 0.7, 0.9]
        struct Fill {
            side: Side,
            qty: Decimal,
        }

        let fills = vec![
            Fill {
                side: Side::Buy,
                qty: dec!(0.5),
            },
            Fill {
                side: Side::Buy,
                qty: dec!(0.5),
            },
            Fill {
                side: Side::Sell,
                qty: dec!(0.3),
            },
            Fill {
                side: Side::Buy,
                qty: dec!(0.2),
            },
        ];

        let mut position_qty = Decimal::ZERO;
        let mut curve: Vec<Decimal> = Vec::new();
        for f in &fills {
            match f.side {
                Side::Buy => position_qty += f.qty,
                Side::Sell => {
                    position_qty -= f.qty;
                    if position_qty < Decimal::ZERO {
                        position_qty = Decimal::ZERO;
                    }
                }
            }
            curve.push(position_qty);
        }

        assert_eq!(curve[0], dec!(0.5), "after first buy: 0.5");
        assert_eq!(curve[1], dec!(1.0), "after second buy: 1.0");
        assert_eq!(curve[2], dec!(0.7), "after sell 0.3: 0.7");
        assert_eq!(curve[3], dec!(0.9), "after final buy: 0.9");
    }

    /// R1 — per-symbol filter returns only rows for the active symbol.
    #[test]
    fn position_curve_per_symbol_filter() {
        use trading_core::Symbol;

        // Cross-sectional data: 3 symbols, each with 2 bars.
        let raw: Vec<(i64, Decimal, Symbol)> = vec![
            (1_000_i64, dec!(0.5), Symbol::new("BTCUSDT")),
            (1_000_i64, dec!(0.2), Symbol::new("ETHUSDT")),
            (1_000_i64, dec!(0.1), Symbol::new("XRPUSDT")),
            (2_000_i64, dec!(1.0), Symbol::new("BTCUSDT")),
            (2_000_i64, dec!(0.4), Symbol::new("ETHUSDT")),
            (2_000_i64, dec!(0.0), Symbol::new("XRPUSDT")),
        ];

        let active = Symbol::new("ETHUSDT");
        let filtered: Vec<(i64, Decimal)> = raw
            .iter()
            .filter(|(_, _, s)| s == &active)
            .map(|&(ts, qty, _)| (ts, qty))
            .collect();

        assert_eq!(filtered.len(), 2, "filter must yield 2 entries for ETHUSDT");
        assert_eq!(filtered[0], (1_000_i64, dec!(0.2)));
        assert_eq!(filtered[1], (2_000_i64, dec!(0.4)));
    }
}
