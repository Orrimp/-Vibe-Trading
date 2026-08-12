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
use crate::strings::{
    VIEWER_EQUITY_LOADING, VIEWER_EQUITY_UNAVAILABLE_PREFIX, VIEWER_NO_EQUITY_DATA,
};
use crate::theme::layout::{AXIS_GUTTER_PRICE_PX, AXIS_GUTTER_RIGHT_PX, AXIS_GUTTER_TIME_PX};
use crate::theme::{ThemeMode, color, space, text};
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
    // 2-18 review M-state-collapse: `Loading` and `Empty` used to render the
    // IDENTICAL "No equity data" body. The state→copy mapping now lives in ONE
    // production seam shared with the sibling equity curve, so the two widgets
    // cannot drift apart and the mapping is assertable against literals.
    if let Some(label) = super::equity_curve::placeholder_label(series) {
        return empty_with_label(label, mode);
    }
    match series {
        // Handled by the seam above; kept for exhaustiveness.
        PanelState::Loading => empty_with_label(VIEWER_EQUITY_LOADING, mode),
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
                canvas_view(s, mode)
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

/// 2-18 review L2 — the program BORROWS the series for the element's
/// lifetime. It used to take an owned `EquitySeries`, so `view()` deep-cloned
/// every point (the Baseline band draws 367 `EquityPoint`s, each carrying
/// three `Decimal`s) on **every** `view()` — and `view` runs on every message:
/// ticks, hovers, toasts. The sibling `equity_curve` was converted to borrow
/// for exactly this reason in the 2-15 pass (`equity_curve.rs` M7); the band
/// was missed. Same change, same rationale.
fn canvas_view<'a>(series: &'a EquitySeries, mode: ThemeMode) -> iced::Element<'a, ViewerMessage> {
    let program = DrawdownBandProgram { series, mode };
    let canvas: Canvas<DrawdownBandProgram<'a>, ViewerMessage> = Canvas::new(program)
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

struct DrawdownBandProgram<'a> {
    series: &'a EquitySeries,
    mode: ThemeMode,
}

impl canvas::Program<ViewerMessage> for DrawdownBandProgram<'_> {
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
        // T3020 — chart-canvas-overhaul v1.10.0 (Q7 viewer parity =
        // BOTH).  Mirrors `equity_curve::draw`: left price-axis
        // gutter (% drawdown labels), bottom time-axis gutter,
        // right margin, top zero.  Legend = NO (single-series).
        let inner = inner_rect_with_gutters(
            bounds.size(),
            AXIS_GUTTER_PRICE_PX,
            AXIS_GUTTER_RIGHT_PX,
            0.0,
            AXIS_GUTTER_TIME_PX,
        );
        let border = with_alpha(color::BORDER_1.current(self.mode), 0.4);

        draw_gridlines(&mut frame, inner, border);

        if self.series.points.is_empty() || inner.width <= 0.0 || inner.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        // Y range: drawdown grows downward — 0 at top,
        // `max_drawdown_frac` at the bottom. The polyline-with-fill
        // primitive paints in screen coords; we map drawdown_frac
        // directly to a (top..bottom) Y position so the fill polygon
        // closes down to the baseline naturally.
        //
        // UNITS: `y_max_frac` and every `p.drawdown_frac` are FRACTIONS
        // (0.4182 = 41.82 %). The geometry is fraction/fraction, so it was
        // always right; only the axis LABELS were wrong (2-18 review H1 —
        // they were formatted `{frac:.1}%`, printing "0.4%" for a 41.8 %
        // drawdown, directly beneath a KPI card reading "−48.95%"). The
        // fraction→percent conversion now lives in one place:
        // [`drawdown_axis_ticks`].
        let max_dd_frac = self.series.max_drawdown_frac.to_f32().unwrap_or(0.0);
        let y_max_frac = max_dd_frac.max(1e-6) * (1.0 + RANGE_PAD_FRACTION);

        // Price axis — `%`-formatted drawdown labels in the LEFT
        // gutter (T3020 / Q7).  Drawdown range is (0, y_max_frac);
        // top of axis = 0 %, bottom = `y_max_frac × 100` %.
        draw_drawdown_axis(&mut frame, inner, y_max_frac, self.mode);
        // Time axis — wall-clock labels in the BOTTOM gutter (T3020).
        draw_time_axis(&mut frame, inner, self.series, self.mode);

        let n = self.series.points.len();
        let denom = if n <= 1 { 1.0 } else { (n - 1) as f32 };
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(n);
        for (i, p) in self.series.points.iter().enumerate() {
            let frac_x = if n <= 1 { 0.0 } else { i as f32 / denom };
            let x = inner.x + frac_x * inner.width;
            let dd = p.drawdown_frac.to_f32().unwrap_or(0.0);
            let frac_y = (dd / y_max_frac).clamp(0.0, 1.0);
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

/// The drawdown price axis as `(fraction-down-the-inner-rect, label)` pairs —
/// **the single place the fraction→percent conversion happens** (2-18 review
/// H1).
///
/// `y_max_frac` is a FRACTION of the peak (0.4391 = 43.91 % below peak, i.e.
/// the padded max drawdown). The returned labels are PERCENT with a `%`
/// suffix, so `0.4391` yields a bottom label of `"43.9%"` — not `"0.4%"`,
/// which is what the pre-fix `format!("{dd:.1}%")` printed for the same input
/// on the screen whose whole purpose is the passive comparator.
///
/// The Y axis is NOT flipped: index 0 is the TOP of the band (0 % drawdown)
/// and the last index is the BOTTOM (`y_max_frac`), matching the polyline
/// orientation in `draw`.
///
/// Returned rather than drawn so the units are assertable against
/// hand-derived literals; [`draw_drawdown_axis`] consumes this verbatim and is
/// its only production caller.
fn drawdown_axis_ticks(y_max_frac: f32) -> Vec<(f32, String)> {
    #[allow(clippy::cast_precision_loss)]
    let denom = (GRIDLINE_COUNT - 1) as f32;
    (0..GRIDLINE_COUNT)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let frac = i as f32 / denom;
            // FRACTION → PERCENT. The `× 100.0` is the entire fix.
            let pct = frac * y_max_frac * 100.0;
            (frac, format!("{pct:.1}%"))
        })
        .collect()
}

/// T3020 — drawdown-band price-axis pass.  Labels come from
/// [`drawdown_axis_ticks`] (percent, one decimal — matches the equity-curve
/// USD labels' one-decimal convention while still resolving sub-percent
/// drawdowns).  Y axis is NOT flipped (0 % at top, `y_max_frac × 100` % at
/// the bottom) — matches the polyline orientation.
fn draw_drawdown_axis(frame: &mut Frame, inner: Rectangle, y_max_frac: f32, mode: ThemeMode) {
    let axis_color = color::FG_3.current(mode);
    let border = with_alpha(color::BORDER_1.current(mode), 0.4);
    #[allow(clippy::cast_precision_loss)]
    let micro = text::MICRO as f32;
    #[allow(clippy::cast_precision_loss)]
    let label_gap = space::XS as f32;
    let tick_len = 4.0_f32;

    // Axis line at the inner rect's left edge.
    let axis_line = Path::new(|builder| {
        builder.move_to(Point::new(inner.x, inner.y));
        builder.line_to(Point::new(inner.x, inner.y + inner.height));
    });
    frame.stroke(
        &axis_line,
        Stroke::default().with_color(border).with_width(1.0),
    );

    // Top gridline = 0 %, bottom = `y_max_frac × 100` % — NOT flipped. The
    // labels are built by the tick seam so the units cannot drift back.
    for (frac, label) in drawdown_axis_ticks(y_max_frac) {
        let y = inner.y + frac * inner.height;
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
            content: label,
            position: Point::new(inner.x - tick_len - label_gap, y),
            color: axis_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }
}

/// T3020 — drawdown-band time-axis pass.  Identical shape to the
/// equity-curve `draw_time_axis`; deduplication into a shared helper
/// is a M5 follow-up (architect-call:
/// `canvas_chart::draw_viewer_time_axis` once the brief lands).
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
    // Total series span drives adaptive label granularity — shared with the
    // equity curve (cockpit-live-axis-density-fix).
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
                out.push_str(&format!("max_dd_frac: {}\n", s.max_drawdown_frac));
                // The axis labels the draw pass writes, from the SAME seam it
                // calls (review H1) — so the committed record shows the
                // operator-visible units, in percent.
                let y_max_frac =
                    s.max_drawdown_frac.to_f32().unwrap_or(0.0) * (1.0 + RANGE_PAD_FRACTION);
                let labels: Vec<String> = drawdown_axis_ticks(y_max_frac)
                    .into_iter()
                    .map(|(_, l)| l)
                    .collect();
                out.push_str(&format!("axis_labels: {}\n", labels.join(" ")));
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

    // ── 2-18 review H1 — the axis labels are PERCENT, not fractions ─────────
    //
    // These assert the LABEL TEXT the draw pass writes to the canvas, against
    // literals derived BY HAND from the series (bug-log #77's rule: the
    // expected value must come from somewhere the implementation cannot
    // reach — never a regenerated snapshot, never a value read off the
    // output).

    /// The exact five gridline labels for the fixture series, hand-derived:
    ///
    /// * fixture peak = $100,000 (i=0), trough = $42,195 ⇒
    ///   `max_drawdown_frac = (100000 − 42195) / 100000 = 0.57805`
    ///   (**57.805 %** below peak);
    /// * `y_max_frac = 0.57805 × 1.05 = 0.6069525` (`RANGE_PAD_FRACTION` 5 %);
    /// * five gridlines at fractions 0, ¼, ½, ¾, 1 of that range ⇒
    ///   0 %, 15.174 %, 30.348 %, 45.521 %, 60.695 % ⇒ rounded to one
    ///   decimal: `0.0%`, `15.2%`, `30.3%`, `45.5%`, `60.7%`.
    ///
    /// Pre-fix the same five ticks rendered `0.0%`, `0.2%`, `0.3%`, `0.5%`,
    /// `0.6%` — the fraction printed with a `%` suffix, 100× too small.
    #[test]
    fn drawdown_axis_labels_are_percent_not_fraction() {
        let s = fixture_series();
        assert_eq!(
            s.max_drawdown_frac,
            dec!(0.57805),
            "fixture precondition: a 57.805 % drawdown"
        );

        let y_max_frac = s.max_drawdown_frac.to_f32().expect("f32") * (1.0 + RANGE_PAD_FRACTION);
        let labels: Vec<String> = drawdown_axis_ticks(y_max_frac)
            .into_iter()
            .map(|(_, l)| l)
            .collect();

        assert_eq!(
            labels,
            vec!["0.0%", "15.2%", "30.3%", "45.5%", "60.7%"],
            "drawdown axis must be labelled in PERCENT (see hand-derivation above)"
        );
    }

    /// The units gate, stated as a RELATION so it holds for any series: the
    /// bottom gridline must read the series' max drawdown (plus the 5 % pad)
    /// **in percent**. A fraction-labelled axis fails this by ~100×.
    ///
    /// Checked on the two real Baseline curves' drawdowns as well as the
    /// fixture, so the test speaks about the screen the operator opens.
    #[test]
    fn bottom_axis_label_tracks_max_drawdown_in_percent() {
        // (max_drawdown_frac, expected bottom label) — the second element is
        // `frac × 1.05 × 100`, rounded to one decimal, computed by hand.
        // 0.4181760895 → the committed 2024 BH curve; 0.3330605849 → 2023.
        for (frac, expected_bottom) in [
            (dec!(0.57805), "60.7%"),
            (dec!(0.4181760895317619), "43.9%"),
            (dec!(0.3330605848692920), "35.0%"),
        ] {
            let y_max_frac = frac.to_f32().expect("f32") * (1.0 + RANGE_PAD_FRACTION);
            let ticks = drawdown_axis_ticks(y_max_frac);
            let (top_frac, top_label) = ticks.first().expect("5 ticks").clone();
            let (bottom_frac, bottom_label) = ticks.last().expect("5 ticks").clone();

            assert!(
                (top_frac - 0.0).abs() < f32::EPSILON,
                "the top gridline is 0 drawdown"
            );
            assert_eq!(top_label, "0.0%", "top gridline is always 0 %");
            assert!(
                (bottom_frac - 1.0).abs() < f32::EPSILON,
                "the bottom gridline is the full padded range"
            );
            assert_eq!(
                bottom_label, expected_bottom,
                "bottom gridline must read the max drawdown in PERCENT for {frac}"
            );

            // The relation, independent of the literals above: the bottom
            // label parsed back must sit within one point of frac × 105 %.
            let parsed: f32 = bottom_label
                .trim_end_matches('%')
                .parse()
                .expect("label parses");
            let expected = frac.to_f32().expect("f32") * (1.0 + RANGE_PAD_FRACTION) * 100.0;
            assert!(
                (parsed - expected).abs() < 0.1,
                "bottom label {parsed} must be the padded max drawdown in percent \
                 ({expected}); a fraction-labelled axis is ~100× off"
            );
        }
    }
}
