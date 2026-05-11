//! Chart widget — chart-buy-sell-emphasis v1.9 (M1–M3).
//!
//! Canvas-based per-symbol price chart for the Charts screen. Rebuilt from
//! the Phase 2 read-only line+marker baseline for the buy/sell-emphasis
//! feature:
//!
//! - **Markers** are now **13-px** filled triangles (was 6-px) with a 1-px
//!   `BORDER_STRONG` outline and a `shadow_1`-derived whisper drop shadow
//!   (Q6).
//! - **Y-snap** computes the marker y by **linear interpolation** between the
//!   bracketing bars' closes so a sub-bar-cadence fill rides the line
//!   instead of jumping to the nearest bar-close (Q2).
//! - **Draw order** is `gridlines → axis labels → line stroke → ghost
//!   markers → fill markers → tooltip overlay` (R2.1, R2.2). Fills now
//!   render **after** the line so they're visible against `ACCENT`.
//! - **Ghost layer** paints `SignalView`-sourced 8-px triangles at 60 %
//!   opacity in the `_400` tier ramp — strategy-intended signals that may
//!   or may not have executed (R5, Q6).
//! - **Tooltip** is a final canvas pass driven by hover-state captured in
//!   `ChartProgram::update` — custom pointer tracking, single source of
//!   truth for centroid math (Q3).
//!
//! Empty state (`bars` is empty): paints gridlines + a centred "No data"
//! label. No markers, no tooltip.
//!
//! **Zero string literals** — copy via `crate::strings::CHART_NO_DATA`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::widget::{container, Canvas, Container};
use iced::{mouse, Color, Length, Point, Rectangle, Renderer, Vector};
use rust_decimal::prelude::ToPrimitive;
use smol_str::SmolStr;
use trading_core::{Bar, FillView, Side, SignalView};

use super::canvas_chart::{
    draw_gridlines, inner_rect, with_alpha, GRIDLINE_COUNT, LINE_STROKE_PX, RANGE_PAD_FRACTION,
};
use super::chart_tooltip;
use crate::state::{ChartMarkerIndex, ChartTooltipKind, ChartTooltipView, Message};
use crate::strings::{
    CHART_NO_DATA, CHART_TOOLTIP_SIDE_BUY, CHART_TOOLTIP_SIDE_SELL, CHART_TOOLTIP_STRATEGY_NONE,
};
use crate::theme::{color, shadow, text, ThemeMode};

/// Filled-marker triangle size for executed-fill markers (Q6, R1.1).
pub(crate) const MARKER_SIZE_PX: f32 = 13.0;

/// Ghost-marker triangle size for strategy-signal markers (Q6, R5.1).
/// `≈ 60 %` of the fill size — the perceptual ramp from intent → execution.
pub(crate) const GHOST_MARKER_SIZE_PX: f32 = 8.0;

/// Square hit-rect around a marker centroid for hover/click detection
/// (R4.3). Comfortably larger than the 13-px marker so Fitts's law works.
pub(crate) const MARKER_HIT_RECT_PX: f32 = 28.0;

/// Outline width for the fill-marker outline pass (Q6, R6.3).
const MARKER_OUTLINE_PX: f32 = 1.0;

/// Render the chart for the active `(venue, symbol)` against the current
/// `bars` window. Returns gridlines + line series + ghost-signal markers +
/// executed-fill markers + tooltip in one canvas.
///
/// Takes owned `Vec`s rather than borrowed slices so the canvas `Program`
/// impl can hold them across iced's render lifetime without borrowing from
/// `Cockpit`. 60 bars × ~200 B per bar = ~12 KB per repaint — trivially
/// within budget.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn view<'a>(
    bars: Vec<Bar>,
    markers: Vec<FillView>,
    signals: Vec<SignalView>,
    tooltip: Option<ChartTooltipView>,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let program = ChartProgram {
        bars,
        markers,
        signals,
        tooltip,
        mode,
    };
    let canvas: Canvas<ChartProgram, Message> = Canvas::new(program)
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

pub(crate) struct ChartProgram {
    pub(crate) bars: Vec<Bar>,
    pub(crate) markers: Vec<FillView>,
    pub(crate) signals: Vec<SignalView>,
    pub(crate) tooltip: Option<ChartTooltipView>,
    pub(crate) mode: ThemeMode,
}

/// Canvas-level hover state — promoted from `()` (Q3). Tracks which marker
/// the cursor is hovering and the marker's centroid so the tooltip
/// renderer can position itself off the same `(x, y)` the draw pass used.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ChartState {
    pub(crate) hovered_marker_idx: Option<ChartMarkerIndex>,
    pub(crate) hovered_marker_centroid: Option<Point>,
}

impl canvas::Program<Message> for ChartProgram {
    type State = ChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // T2030 — DON'T bail on `cursor.position_in(bounds)?` for
                // CursorMoved: if the operator was hovering a marker and
                // then swept the cursor off the canvas entirely (or onto
                // a sibling widget), we still need to publish
                // `ChartMarkerHoverEnded` so the tooltip clears.  The
                // pre-T2030 implementation `?`-bailed here, which is the
                // bug the operator's 2026-05-11 report surfaced — the
                // tooltip latched on whatever marker the cursor last
                // touched and never cleared.
                let cursor_pos = cursor.position_in(bounds);
                let hit = match cursor_pos {
                    Some(p) => {
                        let inner = inner_rect(bounds.size());
                        self.hit_test(p, inner)
                    }
                    None => None,
                };
                if state.hovered_marker_idx == hit.map(|(idx, _)| idx) {
                    // Idempotent — same hover state, no churn.
                    return None;
                }
                state.hovered_marker_idx = hit.map(|(idx, _)| idx);
                state.hovered_marker_centroid = hit.map(|(_, p)| p);
                let msg = match hit {
                    Some((idx, _)) => Message::ChartMarkerHovered(idx),
                    None => Message::ChartMarkerHoverEnded,
                };
                // Only `capture` when the cursor is actually over the
                // canvas — capturing a CursorMoved we never saw would
                // suppress event bubbling for cursor-on-sibling moves,
                // which sibling widgets need.
                let action = canvas::Action::publish(msg);
                Some(if cursor_pos.is_some() {
                    action.and_capture()
                } else {
                    action
                })
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // Clicks DO require the cursor to be on the canvas — a
                // click on a sibling shouldn't open a journal modal.
                let cursor_pos = cursor.position_in(bounds)?;
                let inner = inner_rect(bounds.size());
                let hit = self.hit_test(cursor_pos, inner)?;
                // Ghosts have no transaction_id — click is a no-op (R5.6).
                if let (ChartMarkerIndex::Fill(i), _) = hit {
                    let fill = self.markers.get(i)?;
                    let tx_id = fill.transaction_id.clone();
                    if tx_id.is_empty() {
                        return None;
                    }
                    return Some(
                        canvas::Action::publish(Message::TapeRowClicked(tx_id)).and_capture(),
                    );
                }
                None
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let inner = inner_rect(bounds.size());
        let border = with_alpha(color::BORDER_1.current(self.mode), 0.4);

        // Pass 1 — Gridlines.
        draw_gridlines(&mut frame, inner, border);

        if self.bars.is_empty() {
            let centre = Point::new(inner.x + inner.width / 2.0, inner.y + inner.height / 2.0);
            #[allow(clippy::cast_precision_loss)]
            let micro_size = text::MICRO as f32;
            // `useless_conversion`: `align_x`/`align_y` field types are
            // crate-private aliases distinct from iced's enums — the
            // `.into()` is load-bearing for iced 0.14's API. clippy
            // disagrees.
            #[allow(clippy::useless_conversion)]
            frame.fill_text(CanvasText {
                content: CHART_NO_DATA.to_string(),
                position: centre,
                color: color::FG_3.current(self.mode),
                size: micro_size.into(),
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center.into(),
                ..CanvasText::default()
            });
            return vec![frame.into_geometry()];
        }

        let Some(range) = price_range(&self.bars) else {
            return vec![frame.into_geometry()];
        };

        // Pass 2 — Axis labels.
        draw_price_labels(&mut frame, bounds.size(), range, self.mode);

        // Pass 3 — Line stroke.
        let line_path = Path::new(|builder| {
            for (idx, bar) in self.bars.iter().enumerate() {
                let x = x_for_index(idx, self.bars.len(), inner);
                let close = decimal_to_f32(&bar.close.get());
                let y = y_for_price(close, range, inner);
                if idx == 0 {
                    builder.move_to(Point::new(x, y));
                } else {
                    builder.line_to(Point::new(x, y));
                }
            }
        });
        frame.stroke(
            &line_path,
            Stroke::default()
                .with_color(color::ACCENT.current(self.mode))
                .with_width(LINE_STROKE_PX),
        );

        // Pass 4 — Ghost-signal triangles (R5.1, M3 — T2018).
        //
        // Painted **before** the executed-fill layer so a fill atop a ghost
        // at the same bar visually wins. 60 % alpha + `_400` tier ramp
        // carries the "intent, not execution" cue (Q6).
        for signal in &self.signals {
            let signal_ts = signal.signal_ts.unix_millis();
            let Some((x, y)) = anchor_for_ts(signal_ts, &self.bars, range, inner) else {
                continue;
            };
            let (fill_color, upward) = match signal.side {
                Side::Buy => (color::UP_400.current(self.mode), true),
                Side::Sell => (color::DOWN_400.current(self.mode), false),
            };
            draw_triangle(
                &mut frame,
                Point::new(x, y),
                with_alpha(fill_color, 0.6),
                upward,
                GHOST_MARKER_SIZE_PX,
                None, // no outline
                None, // no shadow
            );
        }

        // Pass 5 — Executed-fill triangles (R2.1, R1.1, R6.1, R6.3).
        //
        // Painted **after** the line so they stay legible against `ACCENT`.
        // 13-px size + 1-px `BORDER_STRONG` outline + `shadow_1` whisper
        // shadow — Q6's full visual treatment.
        let outline = Some(color::BORDER_STRONG.current(self.mode));
        let drop_shadow = Some(whisper_shadow(self.mode));
        for fill in &self.markers {
            let fill_ts = fill.venue_ts.unix_millis();
            let Some((x, y)) = anchor_for_ts(fill_ts, &self.bars, range, inner) else {
                continue;
            };
            let (fill_color, upward) = match fill.side {
                Side::Buy => (color::UP_500.current(self.mode), true),
                Side::Sell => (color::DOWN_500.current(self.mode), false),
            };
            draw_triangle(
                &mut frame,
                Point::new(x, y),
                fill_color,
                upward,
                MARKER_SIZE_PX,
                outline,
                drop_shadow,
            );
        }

        // Pass 6 — Tooltip overlay (R4.2, Q3).
        //
        // Driven by hover state captured in `ChartProgram::update`. Anchor
        // prefers the centroid the same `update` pass recorded so the
        // tooltip rides the exact pixel the marker is painted on.
        if let (Some(view), Some(anchor)) = (self.tooltip.as_ref(), state.hovered_marker_centroid) {
            chart_tooltip::draw_tooltip(&mut frame, bounds, anchor, view, self.mode);
        }

        vec![frame.into_geometry()]
    }
}

impl ChartProgram {
    /// Resolve a cursor position to a marker (fill or ghost) under the
    /// `MARKER_HIT_RECT_PX` square centred on each marker's centroid.
    /// Fills win over ghosts at the same anchor — the z-order from
    /// `draw` (ghosts then fills) is mirrored in the hit-test priority.
    fn hit_test(&self, cursor: Point, inner: Rectangle) -> Option<(ChartMarkerIndex, Point)> {
        let range = price_range(&self.bars)?;

        // Fills win over ghosts — check fills first.
        for (i, fill) in self.markers.iter().enumerate() {
            let fill_ts = fill.venue_ts.unix_millis();
            if let Some(anchor) = anchor_for_ts(fill_ts, &self.bars, range, inner) {
                let pt = Point::new(anchor.0, anchor.1);
                if marker_hit_rect(pt).contains(cursor) {
                    return Some((ChartMarkerIndex::Fill(i), pt));
                }
            }
        }

        for (i, signal) in self.signals.iter().enumerate() {
            let signal_ts = signal.signal_ts.unix_millis();
            if let Some(anchor) = anchor_for_ts(signal_ts, &self.bars, range, inner) {
                let pt = Point::new(anchor.0, anchor.1);
                if marker_hit_rect(pt).contains(cursor) {
                    return Some((ChartMarkerIndex::Signal(i), pt));
                }
            }
        }

        None
    }
}

/// Compute the on-canvas `(x, y)` for a marker at `ts` against the visible
/// bar window. Returns `None` when `ts` falls outside `[bars.first().open_ts,
/// bars.last().close_ts]` so the defence-in-depth window clip (line
/// boundary check) is hoisted here.
pub(crate) fn anchor_for_ts(
    ts: i64,
    bars: &[Bar],
    range: (f32, f32),
    inner: Rectangle,
) -> Option<(f32, f32)> {
    let first = bars.first()?;
    let last = bars.last()?;
    let min_ts = first.open_ts.unix_millis();
    let max_ts = last.close_ts.unix_millis();
    if ts < min_ts || ts > max_ts {
        return None;
    }
    let x_frac = ts_fraction(ts, min_ts, max_ts);
    let x = inner.x + x_frac * inner.width;
    let snapped_price =
        snap_price_to_line(ts, bars).unwrap_or_else(|| decimal_to_f32(&first.close.get()));
    let y = y_for_price(snapped_price, range, inner);
    Some((x, y))
}

/// Linear-interpolate the marker's y-price between the bracketing bars'
/// close prices (Q2 = Option (b)). For a marker at `t` between bar `i`
/// (close at `t_i`, close-price `p_i`) and bar `i+1` (close at `t_{i+1}`,
/// close-price `p_{i+1}`), returns
///
/// ```text
/// frac = (t - t_i) / (t_{i+1} - t_i)
/// p_snapped = p_i + frac * (p_{i+1} - p_i)
/// ```
///
/// Edge cases (R3.3 verbatim):
/// - First-bar marker → `bars[0].close`.
/// - Last-bar marker → `bars[N-1].close`.
/// - Out-of-window marker → callers must clip first; this fn returns the
///   nearest endpoint's close.
pub(crate) fn snap_price_to_line(fill_ts: i64, bars: &[Bar]) -> Option<f32> {
    if bars.is_empty() {
        return None;
    }
    if bars.len() == 1 {
        return Some(decimal_to_f32(&bars[0].close.get()));
    }

    // Binary search for the rightmost bar whose close_ts <= fill_ts.
    let mut lo = 0_usize;
    let mut hi = bars.len();
    while lo < hi {
        let mid = usize::midpoint(lo, hi);
        let mid_ts = bars[mid].close_ts.unix_millis();
        if mid_ts <= fill_ts {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    // `lo` is the count of bars with close_ts <= fill_ts. `lo - 1` is the
    // left bracket; `lo` is the right bracket. Handle edges.
    if lo == 0 {
        return Some(decimal_to_f32(&bars[0].close.get()));
    }
    if lo >= bars.len() {
        return Some(decimal_to_f32(&bars[bars.len() - 1].close.get()));
    }
    let left = &bars[lo - 1];
    let right = &bars[lo];
    let t_l = left.close_ts.unix_millis();
    let t_r = right.close_ts.unix_millis();
    let p_l = decimal_to_f32(&left.close.get());
    let p_r = decimal_to_f32(&right.close.get());
    if t_r <= t_l {
        return Some(p_l);
    }
    #[allow(clippy::cast_precision_loss)]
    let frac = (fill_ts - t_l) as f32 / (t_r - t_l) as f32;
    Some(p_l + frac * (p_r - p_l))
}

/// `MARKER_HIT_RECT_PX`-side square centred on `anchor`. Used by both the
/// canvas `update` hit-test and the tooltip-anchor render math (Q3, R4.3).
pub(crate) fn marker_hit_rect(anchor: Point) -> Rectangle {
    let half = MARKER_HIT_RECT_PX / 2.0;
    Rectangle {
        x: anchor.x - half,
        y: anchor.y - half,
        width: MARKER_HIT_RECT_PX,
        height: MARKER_HIT_RECT_PX,
    }
}

/// Whisper drop shadow derived from `theme::shadow::shadow_1(mode)` — same
/// alpha-per-mode as Lumen panel chrome, repurposed for marker shadows so
/// no new theme token is introduced (Q6 explicit reuse).
pub(crate) fn whisper_shadow(mode: ThemeMode) -> (Vector, Color) {
    let s = shadow::shadow_1(mode);
    (Vector::new(0.0, 1.5), s.color)
}

fn draw_price_labels(frame: &mut Frame, size: iced::Size, range: (f32, f32), mode: ThemeMode) {
    let (min_p, max_p) = range;
    #[allow(clippy::cast_precision_loss)]
    let denom = (GRIDLINE_COUNT - 1) as f32;
    let inner = inner_rect(size);
    for i in 0..GRIDLINE_COUNT {
        #[allow(clippy::cast_precision_loss)]
        let frac = i as f32 / denom;
        let y = inner.y + frac * inner.height;
        // Label-corresponding price (top gridline = max_p, bottom = min_p).
        let price = max_p - frac * (max_p - min_p);
        #[allow(clippy::cast_precision_loss)]
        let micro = text::MICRO as f32;
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: format!("{price:.2}"),
            position: Point::new(inner.x + 4.0, y),
            color: color::FG_3.current(mode),
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }
}

/// Filled-triangle helper. Renders an optional drop shadow first, then the
/// fill, then an optional outline pass. All three layers share the same
/// triangle geometry computed from `size`.
///
/// `size = MARKER_SIZE_PX` for fills, `GHOST_MARKER_SIZE_PX` for ghosts.
/// Ghosts pass `outline = None, shadow = None`; fills pass both `Some`.
pub(crate) fn draw_triangle(
    frame: &mut Frame,
    anchor: Point,
    color: Color,
    upward: bool,
    size: f32,
    outline: Option<Color>,
    shadow: Option<(Vector, Color)>,
) {
    let half = size / 2.0;
    let height = size;
    let make_path = |anchor: Point| {
        Path::new(|builder| {
            if upward {
                builder.move_to(Point::new(anchor.x, anchor.y - height));
                builder.line_to(Point::new(anchor.x - half, anchor.y));
                builder.line_to(Point::new(anchor.x + half, anchor.y));
            } else {
                builder.move_to(Point::new(anchor.x, anchor.y + height));
                builder.line_to(Point::new(anchor.x - half, anchor.y));
                builder.line_to(Point::new(anchor.x + half, anchor.y));
            }
            builder.close();
        })
    };

    if let Some((offset, shadow_color)) = shadow {
        let shadow_anchor = Point::new(anchor.x + offset.x, anchor.y + offset.y);
        let shadow_path = make_path(shadow_anchor);
        frame.fill(&shadow_path, shadow_color);
    }

    let fill_path = make_path(anchor);
    frame.fill(&fill_path, color);

    if let Some(outline_color) = outline {
        frame.stroke(
            &fill_path,
            Stroke::default()
                .with_color(outline_color)
                .with_width(MARKER_OUTLINE_PX),
        );
    }
}

fn price_range(bars: &[Bar]) -> Option<(f32, f32)> {
    let first = bars.first()?;
    let mut min_low = decimal_to_f32(&first.low.get());
    let mut max_high = decimal_to_f32(&first.high.get());
    for bar in bars.iter().skip(1) {
        let low = decimal_to_f32(&bar.low.get());
        let high = decimal_to_f32(&bar.high.get());
        if low < min_low {
            min_low = low;
        }
        if high > max_high {
            max_high = high;
        }
    }
    let span = (max_high - min_low).max(1.0);
    let pad = span * RANGE_PAD_FRACTION;
    Some((min_low - pad, max_high + pad))
}

pub(crate) fn x_for_index(idx: usize, count: usize, inner: Rectangle) -> f32 {
    if count <= 1 {
        return inner.x;
    }
    #[allow(clippy::cast_precision_loss)]
    let frac = idx as f32 / (count - 1) as f32;
    inner.x + frac * inner.width
}

pub(crate) fn y_for_price(price: f32, range: (f32, f32), inner: Rectangle) -> f32 {
    let (min_p, max_p) = range;
    let span = (max_p - min_p).max(1e-6);
    let frac = (price - min_p) / span;
    inner.y + (1.0 - frac) * inner.height
}

pub(crate) fn ts_fraction(ts: i64, min_ts: i64, max_ts: i64) -> f32 {
    if max_ts <= min_ts {
        return 0.0;
    }
    let span = max_ts - min_ts;
    #[allow(clippy::cast_precision_loss)]
    let frac = (ts - min_ts) as f32 / span as f32;
    frac.clamp(0.0, 1.0)
}

pub(crate) fn decimal_to_f32(d: &rust_decimal::Decimal) -> f32 {
    d.to_f32().unwrap_or(0.0)
}

/// Helper for the tooltip widget — render the side badge text per `Side`.
#[must_use]
pub(crate) fn side_badge_label(side: Side) -> &'static str {
    match side {
        Side::Buy => CHART_TOOLTIP_SIDE_BUY,
        Side::Sell => CHART_TOOLTIP_SIDE_SELL,
    }
}

/// Helper for the tooltip widget — colour the side badge per `Side`.
#[must_use]
pub(crate) fn side_badge_color(side: Side, mode: ThemeMode) -> Color {
    match side {
        Side::Buy => color::UP_500.current(mode),
        Side::Sell => color::DOWN_500.current(mode),
    }
}

/// Build the tooltip view for a click on the `idx`-th fill marker. Pure
/// function — called from `cockpit_live`'s `Task::done` shim on
/// `Message::ChartMarkerHovered(Fill(idx))`.
#[must_use]
pub fn tooltip_view_for_fill(fill: &FillView, strategy_id: Option<SmolStr>) -> ChartTooltipView {
    ChartTooltipView {
        kind: ChartTooltipKind::Fill,
        side: fill.side,
        price: Some(fill.price.get()),
        qty: fill.qty.get(),
        notional: Some(fill.price.get().saturating_mul(fill.qty.get())),
        ts: fill.venue_ts,
        strategy_id,
        was_clamped: false,
        clamp_reason: None,
    }
}

/// Build the tooltip view for a hover on the `idx`-th ghost-signal marker.
#[must_use]
pub fn tooltip_view_for_signal(signal: &SignalView) -> ChartTooltipView {
    ChartTooltipView {
        kind: ChartTooltipKind::Signal,
        side: signal.side,
        price: None,
        qty: signal.intended_qty.get(),
        notional: None,
        ts: signal.signal_ts,
        strategy_id: Some(signal.strategy_id.0.clone()),
        was_clamped: signal.was_clamped,
        clamp_reason: signal.clamp_reason.clone(),
    }
}

/// Helper for the tooltip widget — "—" placeholder for an absent strategy
/// id (R4.7).
#[must_use]
#[allow(dead_code)]
pub(crate) fn strategy_label_or_none(strategy_id: Option<&SmolStr>) -> &str {
    strategy_id
        .map(smol_str::SmolStr::as_str)
        .unwrap_or(CHART_TOOLTIP_STRATEGY_NONE)
}

/// Test-only helper exposing the canvas `Program::update` pipeline to
/// integration tests so the **actual hover-event-detection path** can be
/// exercised without an iced runtime (T2030).
///
/// The previous-pass tooltip integration test
/// (`crates/ui/tests/chart_tooltip_integration.rs`) exercised
/// `Message::ChartMarkerHovered` against `ui::state::update` — i.e.
/// render-given-hover-state — but never proved that
/// `canvas::Program::update` ACTUALLY publishes that message on a
/// `mouse::Event::CursorMoved`. Operator feedback 2026-05-11 surfaced
/// the gap (tooltips invisible on hover despite green tests); this
/// helper closes it.
///
/// Returns `(Option<Message>, event::Status)` mirroring the
/// `canvas::Action` two-value contract: `Some(msg)` when the program
/// published a message, `None` otherwise; `Status::Captured` when the
/// program "captured" the event (preventing further bubbling),
/// `Status::Ignored` otherwise.
///
/// `bars` / `markers` / `signals` follow the same shape `chart::view`
/// takes; `bounds` is the canvas's absolute-screen rectangle and
/// `cursor_pos` is the cursor's absolute-screen `(x, y)` (i.e. the
/// caller has already added `bounds.x` and `bounds.y` to the
/// canvas-local coordinate).
///
/// **Test-only**: this is `#[doc(hidden)]` and not part of the stable
/// widget API.  Production code MUST NOT call it.
#[doc(hidden)]
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn dispatch_canvas_event_for_test(
    bars: Vec<Bar>,
    markers: Vec<FillView>,
    signals: Vec<SignalView>,
    state: &mut ChartHoverState,
    event: iced::widget::canvas::Event,
    bounds: Rectangle,
    cursor_pos: Point,
) -> (Option<Message>, iced::event::Status) {
    let program = ChartProgram {
        bars,
        markers,
        signals,
        tooltip: None,
        mode: ThemeMode::Dark,
    };
    let cursor = mouse::Cursor::Available(cursor_pos);
    let action = canvas::Program::<Message>::update(&program, &mut state.0, &event, bounds, cursor);
    match action {
        Some(a) => {
            let (msg, _redraw, status) = a.into_inner();
            (msg, status)
        }
        None => (None, iced::event::Status::Ignored),
    }
}

/// Opaque wrapper around the chart's `Program::State` so integration
/// tests can keep a hover-state cookie across calls to
/// [`dispatch_canvas_event_for_test`] without depending on the
/// crate-private `ChartState` struct.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ChartHoverState(ChartState);

impl ChartHoverState {
    /// Test-only — returns `true` iff this state currently records a
    /// hovered marker.
    #[doc(hidden)]
    #[must_use]
    pub fn is_hovering(&self) -> bool {
        self.0.hovered_marker_idx.is_some()
    }

    /// Test-only — returns the recorded marker centroid, if any.
    #[doc(hidden)]
    #[must_use]
    pub fn hovered_marker_centroid(&self) -> Option<Point> {
        self.0.hovered_marker_centroid
    }
}

#[cfg(test)]
#[allow(
    clippy::format_push_string,
    clippy::useless_format,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use trading_core::{FeeTier, Money, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

    fn fixed_ts(offset_min: i64) -> Timestamp {
        let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000 + offset_min * 60)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        Timestamp::new(dt)
    }

    fn make_bar(offset_min: i64, close: Decimal) -> Bar {
        let p = |d: Decimal| Price::new(d).unwrap_or_else(|_| unreachable!());
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open_ts: fixed_ts(offset_min),
            close_ts: fixed_ts(offset_min),
            open: p(close - dec!(50)),
            high: p(close + dec!(80)),
            low: p(close - dec!(80)),
            close: p(close),
            volume: Quantity::new(dec!(12.5)).unwrap_or_else(|_| unreachable!()),
            trade_count: 100,
            local_recv_ts: fixed_ts(offset_min),
            venue: Venue::Binance,
        }
    }

    fn make_fill(offset_min: i64, side: Side, price: Decimal) -> FillView {
        FillView {
            symbol: Symbol::new("BTCUSDT"),
            side,
            price: Price::new(price).unwrap_or_else(|_| unreachable!()),
            qty: Quantity::new(dec!(0.1)).unwrap_or_else(|_| unreachable!()),
            fee: Money::from_decimal(dec!(0.5)),
            fee_tier: FeeTier::Taker,
            venue_ts: fixed_ts(offset_min),
            transaction_id: smol_str::SmolStr::new(format!("fixture-{offset_min}-{side:?}")),
        }
    }

    fn make_signal(offset_min: i64, side: Side, was_clamped: bool) -> SignalView {
        SignalView {
            signal_id: smol_str::SmolStr::new(format!("sig-{offset_min}-{side:?}")),
            symbol: Symbol::new("BTCUSDT"),
            side,
            intended_qty: Quantity::new(dec!(0.05)).unwrap_or_else(|_| unreachable!()),
            signal_ts: fixed_ts(offset_min),
            strategy_id: trading_core::StrategyId::new("sma_crossover"),
            was_clamped,
            clamp_reason: if was_clamped {
                Some(smol_str::SmolStr::new("per_symbol_cap"))
            } else {
                None
            },
        }
    }

    /// Plain-text summary of what the chart canvas would render. Pinned via
    /// snapshot so a regression in line/marker counts, draw-order, or axis
    /// range is visible without a pixel-level renderer.
    #[allow(clippy::ptr_arg)]
    fn chart_summary(bars: &[Bar], markers: &[FillView], signals: &[SignalView]) -> String {
        let mut out = String::new();
        out.push_str("widget: chart\n");
        out.push_str(&format!("bar_count: {}\n", bars.len()));
        out.push_str(&format!("gridlines: {}\n", GRIDLINE_COUNT));
        if bars.is_empty() {
            out.push_str("empty_state: true\n");
            out.push_str(&format!("empty_label: {CHART_NO_DATA}\n"));
        } else {
            out.push_str("empty_state: false\n");
            out.push_str("draw_order: gridlines,labels,line,ghosts,fills,tooltip\n");
            out.push_str("line_color: ACCENT\n");
            if let Some((min_p, max_p)) = price_range(bars) {
                out.push_str(&format!("axis_range: min={min_p:.2} max={max_p:.2}\n"));
            }
            let buys = markers
                .iter()
                .filter(|f| matches!(f.side, Side::Buy))
                .count();
            let sells = markers
                .iter()
                .filter(|f| matches!(f.side, Side::Sell))
                .count();
            let ghost_buys = signals
                .iter()
                .filter(|s| matches!(s.side, Side::Buy))
                .count();
            let ghost_sells = signals
                .iter()
                .filter(|s| matches!(s.side, Side::Sell))
                .count();
            out.push_str(&format!("fill_count: {}\n", markers.len()));
            out.push_str(&format!("ghost_count: {}\n", signals.len()));
            out.push_str(&format!("markers_buy: {buys}\n"));
            out.push_str(&format!("markers_sell: {sells}\n"));
            out.push_str(&format!("ghost_buy: {ghost_buys}\n"));
            out.push_str(&format!("ghost_sell: {ghost_sells}\n"));
            out.push_str(&format!("marker_size_px: {MARKER_SIZE_PX}\n"));
            out.push_str(&format!("ghost_marker_size_px: {GHOST_MARKER_SIZE_PX}\n"));
            out.push_str(&format!(
                "marker_buy_color: UP_500\nmarker_sell_color: DOWN_500\n"
            ));
            out.push_str(&format!(
                "ghost_buy_color: UP_400\nghost_sell_color: DOWN_400\nghost_alpha: 0.6\n"
            ));
            out.push_str("marker_outline: BORDER_STRONG\n");
            out.push_str("marker_shadow: shadow_1\n");
        }
        out
    }

    #[test]
    #[allow(non_snake_case)]
    fn chart__btc_with_two_buys_one_sell() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| make_bar(i, dec!(40_000) + Decimal::from(i) * dec!(2.5)))
            .collect();
        let markers = vec![
            make_fill(5, Side::Buy, dec!(40_010)),
            make_fill(20, Side::Buy, dec!(40_055)),
            make_fill(45, Side::Sell, dec!(40_120)),
        ];
        let signals: Vec<SignalView> = Vec::new();
        assert_snapshot!(
            "chart__btc_with_two_buys_one_sell",
            chart_summary(&bars, &markers, &signals)
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn chart__empty_state_no_data() {
        let bars: Vec<Bar> = Vec::new();
        let markers: Vec<FillView> = Vec::new();
        let signals: Vec<SignalView> = Vec::new();
        assert_snapshot!(
            "chart__empty_state_no_data",
            chart_summary(&bars, &markers, &signals)
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn chart__with_ghosts_and_fills() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| make_bar(i, dec!(40_000) + Decimal::from(i) * dec!(2.5)))
            .collect();
        let markers = vec![make_fill(30, Side::Buy, dec!(40_075))];
        let signals = vec![
            make_signal(10, Side::Buy, false),
            make_signal(40, Side::Sell, true),
        ];
        assert_snapshot!(
            "chart__with_ghosts_and_fills",
            chart_summary(&bars, &markers, &signals)
        );
    }

    /// T2003 — `snap_price_to_line` linearly interpolates between bracket
    /// bars. Fixture: two bars whose closes differ by `100`; a fill at the
    /// exact midpoint timestamp must snap to the midpoint price.
    #[test]
    fn chart_marker_y_snaps_to_line() {
        let bars = vec![make_bar(0, dec!(100)), make_bar(10, dec!(200))];
        let t_left = bars[0].close_ts.unix_millis();
        let t_right = bars[1].close_ts.unix_millis();
        let mid_ts = (t_left + t_right) / 2;
        let snapped = snap_price_to_line(mid_ts, &bars).expect("midpoint snap");
        // 100 + 0.5*(200-100) == 150 within float tolerance.
        assert!((snapped - 150.0).abs() < 0.5, "snapped={snapped}");

        // Edge: at left bar ts → left close.
        let left_snap = snap_price_to_line(t_left, &bars).expect("left snap");
        assert!((left_snap - 100.0).abs() < 0.5, "left={left_snap}");

        // Edge: at right bar ts → right close.
        let right_snap = snap_price_to_line(t_right, &bars).expect("right snap");
        assert!((right_snap - 200.0).abs() < 0.5, "right={right_snap}");
    }

    /// T2002 — `whisper_shadow` reads from `shadow_1` for both modes.
    #[test]
    fn chart_draw_triangle_outline_and_shadow() {
        let (dark_v, dark_c) = whisper_shadow(ThemeMode::Dark);
        let (light_v, light_c) = whisper_shadow(ThemeMode::Light);
        assert!((dark_v.x - 0.0).abs() < f32::EPSILON);
        assert!((dark_v.y - 1.5).abs() < f32::EPSILON);
        assert!((light_v.x - 0.0).abs() < f32::EPSILON);
        assert!((light_v.y - 1.5).abs() < f32::EPSILON);
        // Same alpha-per-mode as `shadow::shadow_1`.
        let s_dark = shadow::shadow_1(ThemeMode::Dark);
        let s_light = shadow::shadow_1(ThemeMode::Light);
        assert!((dark_c.a - s_dark.color.a).abs() < f32::EPSILON);
        assert!((light_c.a - s_light.color.a).abs() < f32::EPSILON);
    }

    /// T2008 — `marker_hit_rect` returns a 28-px square centred on the
    /// anchor.
    #[test]
    fn chart_marker_hit_rect_is_28px_centred() {
        let r = marker_hit_rect(Point::new(100.0, 50.0));
        assert!((r.width - 28.0).abs() < f32::EPSILON);
        assert!((r.height - 28.0).abs() < f32::EPSILON);
        // Centre falls inside.
        assert!(r.contains(Point::new(100.0, 50.0)));
        // 1 px outside the rect doesn't.
        assert!(!r.contains(Point::new(115.0, 50.0)));
    }
}
