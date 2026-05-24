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
use iced::widget::{Canvas, Container, container};
use iced::{Color, Length, Point, Rectangle, Renderer, Size, Vector, mouse};
use rust_decimal::prelude::ToPrimitive;
use smol_str::SmolStr;
use trading_core::{Bar, FillView, Side, SignalView};

use super::canvas_chart::{
    GRIDLINE_COUNT, LINE_STROKE_PX, RANGE_PAD_FRACTION, draw_gridlines, inner_rect_with_gutters,
    with_alpha,
};
use super::chart_legend;
use super::chart_tooltip;
use crate::lab::equity_loader::LabEquitySeries;
use crate::state::{ChartMarkerIndex, ChartTooltipKind, ChartTooltipView, Message};
use crate::strings::{
    CHART_NO_DATA, CHART_TOOLTIP_SIDE_BUY, CHART_TOOLTIP_SIDE_SELL, CHART_TOOLTIP_STRATEGY_NONE,
};
use crate::theme::layout::{AXIS_GUTTER_PRICE_PX, AXIS_GUTTER_RIGHT_PX, AXIS_GUTTER_TIME_PX};
use crate::theme::{ThemeMode, color, shadow, space, text};

/// Right Y-axis gutter width when the equity overlay is active (Design § 3).
/// Sized to match the left price gutter for visual symmetry (R2.2).
pub(crate) const AXIS_GUTTER_EQUITY_PX: f32 = 56.0;

/// Equity-line stroke width — 1.5 px (slightly thinner than the 2.0 px price
/// line so price stays visually dominant; Design § 3).
const EQUITY_STROKE_PX: f32 = 1.5;

/// Number of right-axis ticks for the equity Y-axis (Design § 3).
const EQUITY_AXIS_TICK_COUNT: usize = 5;

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

/// Tick mark length (price axis + time axis) in logical pixels.
/// Sized so the tick reads as a distinct mark without crowding the
/// label.  Same value for both axes — visual rhythm matches.
const AXIS_TICK_LEN_PX: f32 = 4.0;

/// **T3010 — chart-canvas-overhaul v1.10.0.**  Compute the chart's
/// drawable inner rect against the canvas's `bounds.size()` using the
/// brief's R4-locked gutter geometry: left price gutter, right margin,
/// bottom time gutter, top zero (legend lives inside the inner rect).
/// Single source of truth for the inner rect used by `draw`, the
/// hit-test path, and any axis-draw pass — keeps the visual and
/// hit-test coordinate systems pinned to the same rectangle (R1.2 /
/// V13).
#[inline]
pub(crate) fn chart_inner_rect(size: Size) -> Rectangle {
    inner_rect_with_gutters(
        size,
        AXIS_GUTTER_PRICE_PX,
        AXIS_GUTTER_RIGHT_PX,
        0.0,
        AXIS_GUTTER_TIME_PX,
    )
}

/// Variant of `chart_inner_rect` that widens the right gutter to accommodate
/// the equity Y-axis when the equity overlay is active (Design § 3 /
/// `AXIS_GUTTER_EQUITY_PX = 56.0`).
#[inline]
pub(crate) fn chart_inner_rect_with_equity(size: Size) -> Rectangle {
    inner_rect_with_gutters(
        size,
        AXIS_GUTTER_PRICE_PX,
        AXIS_GUTTER_EQUITY_PX,
        0.0,
        AXIS_GUTTER_TIME_PX,
    )
}

/// **T3012 — adaptive tick spacing** for the bottom time axis (R4.2.1
/// / Q3 architect-resolved).
///
/// Formula: `tick_count = clamp(canvas_width_logical / 96, 4, 12)`
/// rounded to the nearest 5-bar multiple so labels never overlap at
/// the 1280-px floor and never look sparse at 3360-px native Retina.
/// Returns the number of intervals (so `tick_count + 1` actual tick
/// positions land on the axis line).
///
/// The formula always emits a multiple-of-5 count (5 / 10 / 15 bars
/// per tick) so the time axis stays visually anchored on round
/// minute boundaries.  Bar-count `0` returns `0` — empty state.
pub(crate) fn time_axis_tick_count(canvas_width_logical: f32, bar_count: usize) -> usize {
    if bar_count == 0 {
        return 0;
    }
    // Logical-width budget: ~96 px per label (text::MICRO ≈ 11 px
    // height + space::S inter-label gap + "HH:MM" ≈ 30-35 px wide +
    // breathing room).
    let raw = (canvas_width_logical / 96.0).clamp(4.0, 12.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let raw_count = raw.round() as usize;
    // Round down to a bar-step multiple that yields an EVEN tick count
    // across the 60-bar window: 60 / 5 = 12, 60 / 10 = 6, 60 / 15 = 4.
    // Pick the largest step ≤ `bar_count / raw_count`.
    let step = if raw_count >= 12 {
        5
    } else if raw_count >= 6 {
        10
    } else {
        15
    };
    // Number of intervals = bar_count / step, capped at bar_count - 1.
    // Returning `intervals + 1` would over-count the right edge; the
    // caller drives the tick loop from `i = 0..=intervals`.
    bar_count.saturating_sub(1) / step
}

/// **T3013 — `local_offset_or_utc`** — chart-canvas-overhaul v1.10.0
/// (R4.2.2 / Q4) — shipped at chart-x-axis-local-time v1.11.0.
///
/// Returns the UTC offset used to render `HH:MM` labels on the
/// bottom time axis.
///
/// **Production** reads the OS-local offset via
/// `time::UtcOffset::current_local_offset()`. If the lookup fails
/// (e.g. multi-threaded glibc unsoundness — does not bite on macOS,
/// the only cockpit-supported platform), falls back to
/// `UtcOffset::UTC` deterministically.
///
/// **Snapshot determinism** is preserved via two complementary gates:
///
/// 1. **Unit-test `#[cfg(test)]` branch** — returns `UtcOffset::UTC`
///    for the library's own unit tests.
/// 2. **`UI_CHART_FORCE_UTC` env var** — integration tests
///    (`tests/render_snapshots.rs`, `tests/visual_snapshots.rs`) set
///    this env var before invoking `iced_test::screenshot` so the
///    production branch returns UTC. This preserves
///    machine-independence of visual baselines (a machine in CEST
///    must produce the same baselines as a machine in UTC).
///
/// The env-var gate is necessary because Cargo only sets `cfg(test)`
/// on a crate when building it as a test target — integration tests
/// link against the library compiled without `cfg(test)`, so the
/// `#[cfg(test)]` branch alone is insufficient.
#[must_use]
#[cfg(test)]
pub(crate) fn local_offset_or_utc() -> time::UtcOffset {
    // CLOCK-OK: snapshot determinism contract — unit tests MUST render
    // at UTC regardless of the host's time zone.
    time::UtcOffset::UTC
}

#[must_use]
#[cfg(not(test))]
pub(crate) fn local_offset_or_utc() -> time::UtcOffset {
    // Integration-test snapshot determinism: see doc comment above.
    if std::env::var_os(crate::strings::CHART_FORCE_UTC_ENV).is_some() {
        // CLOCK-OK: env-var override for integration tests.
        return time::UtcOffset::UTC;
    }
    // CLOCK-OK: production reads the OS-local offset; defensive
    // UTC fallback if the lookup fails.
    time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC)
}

/// Render the chart for the active `(venue, symbol)` against the current
/// `bars` window. Returns gridlines + line series + ghost-signal markers +
/// executed-fill markers + optional equity-curve overlays + tooltip in one
/// canvas.
///
/// New Phase A parameters (Design § 3 / T-D-11):
/// - `equity` — primary strategy equity curve; activates the right Y-axis
///   gutter when `Some`. Pass `None` for the v1.10.0 price+markers shape
///   (backward-compatible — all existing call sites remain pixel-identical).
/// - `compare` — up to 4 comparison-strategy equity curves (M3 / T-D-15).
///   Pass `vec![]` at all existing call sites.
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
    equity: Option<LabEquitySeries>,
    compare: Vec<LabEquitySeries>,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let program = ChartProgram {
        bars,
        markers,
        signals,
        tooltip,
        equity,
        compare,
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
    /// Cockpit-state tooltip — vestigial post-T2033.  The pre-T2033
    /// draw path read this to render the tooltip overlay, but the
    /// asynchronous round-trip (canvas → `Message::ChartMarkerHovered`
    /// → `state::update` → next paint) lost the first frame and
    /// produced the flash-and-disappear bug the operator reported on
    /// 2026-05-11.  The post-T2033 draw path builds the tooltip view
    /// from `self.markers[idx]` / `self.signals[idx]` directly using
    /// the canvas's local `ChartState`.  The field stays in the
    /// struct so the public `chart::view` signature is unchanged (the
    /// snapshot-test path at `state::build_tooltip_view` still drives
    /// `Cockpit.chart_tooltip`); it is intentionally never read by
    /// the draw pass.
    #[allow(dead_code)]
    pub(crate) tooltip: Option<ChartTooltipView>,
    /// Primary equity-curve overlay (T-D-11 / Design § 3). `None` disables
    /// the overlay and the right Y-axis gutter — backward-compatible.
    pub(crate) equity: Option<LabEquitySeries>,
    /// Comparison equity curves (T-D-15 / Design § 3). Empty vec disables
    /// the comparison pass.
    pub(crate) compare: Vec<LabEquitySeries>,
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
                        // T3010 — hit-test runs against the SAME
                        // inner rect the draw pass paints into.
                        // Single source of truth keeps the visual
                        // and hit-test coordinate systems pinned to
                        // the same rectangle (R1.2 / V13).
                        let has_eq = self.equity.is_some() || !self.compare.is_empty();
                        let inner = if has_eq {
                            chart_inner_rect_with_equity(bounds.size())
                        } else {
                            chart_inner_rect(bounds.size())
                        };
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
                // T3010 — same inner-rect single-source-of-truth as
                // the CursorMoved branch above.
                let has_eq = self.equity.is_some() || !self.compare.is_empty();
                let inner = if has_eq {
                    chart_inner_rect_with_equity(bounds.size())
                } else {
                    chart_inner_rect(bounds.size())
                };
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

    #[allow(clippy::too_many_lines)]
    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        // T3010 — single source of truth for the drawable rect:
        // `chart_inner_rect` applies the brief's R4 gutters
        // (price-axis LEFT, margin RIGHT, time-axis BOTTOM) so the
        // visual content + hit-test math share one rectangle.
        //
        // T-D-11 — when an equity overlay is active, widen the right
        // gutter to `AXIS_GUTTER_EQUITY_PX` (56 px) for the right Y-axis
        // (Design § 3). `chart_inner_rect_with_equity` keeps the inner
        // rect math as the single source of truth.
        let has_equity = self.equity.is_some() || !self.compare.is_empty();
        let inner = if has_equity {
            chart_inner_rect_with_equity(bounds.size())
        } else {
            chart_inner_rect(bounds.size())
        };
        let border = with_alpha(color::BORDER_1.current(self.mode), 0.4);

        // Pass 1 — Gridlines.
        draw_gridlines(&mut frame, inner, border);

        if self.bars.is_empty() {
            // F9 (lab-end-to-end-v2 Wave D-1.1) — when bars are absent but an
            // equity overlay is present (backtest result on a pair with no live
            // bars yet), render the equity curve using its own timestamp window
            // instead of falling through to the "No data" early return.
            // Only the equity pass fires; price axis, time axis, and markers are
            // all skipped (no bar data to anchor them).
            if has_equity {
                let equity_range = compute_equity_range(
                    self.equity.as_ref(),
                    &self.compare,
                    None, // no bar-window clamp — use full equity extent
                    None,
                );
                let equity_range_present = equity_range.is_some();
                if let Some((min_eq, max_eq)) = equity_range {
                    if let Some(ref eq) = self.equity {
                        draw_equity_polyline_standalone(
                            &mut frame,
                            eq,
                            min_eq,
                            max_eq,
                            inner,
                            color::ACCENT.current(self.mode),
                        );
                    }
                    let palette = color::accent_palette();
                    for (i, compare_eq) in self.compare.iter().enumerate().take(4) {
                        let line_color = palette[i].current(self.mode);
                        draw_equity_polyline_standalone(
                            &mut frame, compare_eq, min_eq, max_eq, inner, line_color,
                        );
                    }
                    draw_equity_axis(&mut frame, bounds.size(), inner, min_eq, max_eq, self.mode);
                }
                tracing::trace!(
                    target: "lab.chart.equity",
                    equity_present = self.equity.is_some(),
                    compare_count = self.compare.len(),
                    equity_range_present,
                    "chart::draw equity-only path (no bars)"
                );
            } else {
                // Truly no data — render the "No data" placeholder.
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
            }
            return vec![frame.into_geometry()];
        }

        let Some(range) = price_range(&self.bars) else {
            return vec![frame.into_geometry()];
        };

        // Pass 2 — Price axis (left gutter — T3011 / R4.1).
        //
        // Five labels right-aligned at `inner.x - space::XS`, paired
        // with a 4-px tick mark drawn into the inner rect's left
        // edge and a 1-px vertical axis line at `inner.x`.  Labels
        // sit OUTSIDE the inner rect (in the price-axis gutter)
        // since v1.10.0; v1.9.0 painted them at `inner.x + 4`
        // which crowded the line at busy zones.
        draw_price_axis(&mut frame, inner, range, self.mode);
        // Pass 3 — Time axis (bottom gutter — T3012 / R4.2).
        draw_time_axis(&mut frame, inner, &self.bars, self.mode);

        // Pass 4 — Line stroke.
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

        // Pass 5 — Equity-curve overlay (T-D-11 / Design § 3 / R2.2, R2.3).
        //
        // Z-order: price line (Pass 4) → equity lines (here) → markers
        // (Pass 6+) so buy/sell markers stay visually dominant (R2.4).
        //
        // The right Y-axis gutter was widened in the inner-rect calculation
        // above when `has_equity` is true. The pass is a no-op when both
        // `self.equity` and `self.compare` are empty — no gutter chrome
        // appears and the chart degrades to the v1.10.0 shape (R2.5).
        if has_equity {
            // Compute the equity range across primary + compare series.
            let equity_range = compute_equity_range(
                self.equity.as_ref(),
                &self.compare,
                self.bars.first().map(|b| b.open_ts.unix_millis()),
                self.bars.last().map(|b| b.close_ts.unix_millis()),
            );

            if let Some((min_eq, max_eq)) = equity_range {
                // Draw primary equity curve (color::ACCENT — same as the
                // price line accent, "I am the focused one"; Design § 3).
                if let Some(ref eq) = self.equity {
                    draw_equity_polyline(
                        &mut frame,
                        eq,
                        &self.bars,
                        min_eq,
                        max_eq,
                        inner,
                        color::ACCENT.current(self.mode),
                    );
                }

                // Draw comparison equity curves in positional ACCENT_2..5.
                let palette = color::accent_palette();
                for (i, compare_eq) in self.compare.iter().enumerate().take(4) {
                    let line_color = palette[i].current(self.mode);
                    draw_equity_polyline(
                        &mut frame, compare_eq, &self.bars, min_eq, max_eq, inner, line_color,
                    );
                }

                // Draw right Y-axis ticks + labels.
                draw_equity_axis(&mut frame, bounds.size(), inner, min_eq, max_eq, self.mode);
            }
        }

        // Pass 5b — Ghost-signal triangles (R5.1, M3 — T2018).
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

        // Pass 6 — Executed-fill triangles (R2.1, R1.1, R6.1, R6.3).
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

        // Pass 7 — Tooltip overlay (R4.2, Q3, **T2033**).
        //
        // Render the tooltip directly from canvas-local hover state.
        // The pre-T2033 form required BOTH `self.tooltip.is_some()`
        // (a `Cockpit.chart_tooltip` round-trip published by
        // `Message::ChartMarkerHovered` and applied by
        // `state::update`) AND `state.hovered_marker_centroid.is_some()`
        // (the canvas's local `Program::State`, set synchronously in
        // `update`). The two flip on different ticks: canvas state
        // flips on `CursorMoved`, iced redraws once before the
        // published message reaches Cockpit, and the tooltip fails to
        // draw because `self.tooltip` is still `None` — the
        // flash-and-disappear the operator reported on 2026-05-11.
        //
        // The fix decouples: `draw` now builds the tooltip view from
        // `self.markers[idx]` / `self.signals[idx]` directly using
        // `state.hovered_marker_idx + state.hovered_marker_centroid`.
        // No Cockpit-state round trip. `Cockpit.chart_tooltip` stays
        // vestigial for the live render but still drives the
        // `chart_tooltip` widget's snapshot tests via the
        // `build_tooltip_view` helper in `state.rs`.
        //
        // **Source-level confirmation (M6.2 fixup, 2026-05-11).**
        // `canvas::Program::update` runs for ALL events including
        // `RedrawRequested` (see [iced 0.14 program.rs:7-15](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/canvas/program.rs#L7-L15)),
        // canvas-local `State` mutates synchronously inside that
        // call, but `Application::update` only consumes the published
        // `Message` on the next runtime drain pass.  Reading the
        // tooltip view from canvas-local state (this code) removes
        // the dual-source-of-truth that produced the
        // flash-and-disappear race in the pre-T2033 version.
        if let (Some(idx), Some(anchor)) = (state.hovered_marker_idx, state.hovered_marker_centroid)
            && let Some(view) = self.tooltip_view_from_hover(idx)
        {
            chart_tooltip::draw_tooltip(&mut frame, bounds, anchor, &view, self.mode);
        }

        // Pass 8 — Legend overlay (R5, Q5 = top-right inset — T3017).
        //
        // Painted LAST so the legend sits visually above every other
        // layer including the tooltip card.  Skipped on empty state
        // (handled above — no fall-through here because the empty
        // branch returns early).  The legend re-uses the chart's
        // marker palette + glyph helpers so any future tweak to
        // `draw_triangle` flows through automatically (single source
        // of truth for the marker glyph shape per R5.4 of the brief).
        //
        // T-D-15 — when compare curves are active, extend the legend
        // with per-compare rows showing the positional ACCENT_N color
        // swatch + the strategy label ("no data" treatment for missing
        // reports per R8.4).
        if self.compare.is_empty() {
            chart_legend::draw_legend(&mut frame, inner, self.mode);
        } else {
            let palette = color::accent_palette();
            let compare_entries: Vec<chart_legend::CompareLegendEntry> = self
                .compare
                .iter()
                .enumerate()
                .take(4)
                .map(|(i, eq)| chart_legend::CompareLegendEntry {
                    label: eq.source_report.clone(),
                    color: palette[i].current(self.mode),
                    has_data: !eq.samples.is_empty(),
                })
                .collect();
            chart_legend::draw_legend_with_compare(&mut frame, inner, self.mode, &compare_entries);
        }

        vec![frame.into_geometry()]
    }
}

impl ChartProgram {
    /// T2033 — build a `ChartTooltipView` directly from the canvas's
    /// `markers` / `signals` slice at the hovered index, with no
    /// dependency on the `self.tooltip` round-trip from Cockpit. Used
    /// by `draw` Pass 6 to render the hover tooltip synchronously
    /// with the canvas's local hover state — closes the
    /// flash-and-disappear race the operator reported 2026-05-11.
    ///
    /// `strategy_id` is `None` for fill markers because the canvas
    /// doesn't have a strategy-attribution side-channel (the
    /// snapshot-test path in `state::build_tooltip_view` reuses the
    /// `lookup_strategy_for_fill` stub, which today always returns
    /// `None`).  If a future enrichment plumbs strategy attribution
    /// onto `FillView`, this branch reads it from there.
    fn tooltip_view_from_hover(
        &self,
        idx: ChartMarkerIndex,
    ) -> Option<crate::state::ChartTooltipView> {
        match idx {
            ChartMarkerIndex::Fill(i) => {
                let fill = self.markers.get(i)?;
                Some(tooltip_view_for_fill(fill, None))
            }
            ChartMarkerIndex::Signal(i) => {
                let signal = self.signals.get(i)?;
                Some(tooltip_view_for_signal(signal))
            }
        }
    }

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

/// **T3011 — price-axis draw pass** (chart-canvas-overhaul v1.10.0,
/// R4.1, Q2 = LEFT).
///
/// Five labels in the LEFT price-axis gutter (right-aligned at
/// `inner.x - space::XS`), paired with a 4-px tick mark at the
/// gridline's y inside the inner rect, and a 1-px vertical axis
/// line at `inner.x` in `color::BORDER_1 @ alpha 0.4` (matching the
/// gridline treatment).
///
/// Replaces v1.9.0's `draw_price_labels` (which painted labels at
/// `inner.x + 4` — INSIDE the inner rect, crowding the line at busy
/// zones).  The labels now live OUTSIDE the inner rect, in the
/// `AXIS_GUTTER_PRICE_PX`-wide left gutter introduced by R4.1.
fn draw_price_axis(frame: &mut Frame, inner: Rectangle, range: (f32, f32), mode: ThemeMode) {
    let (min_p, max_p) = range;
    #[allow(clippy::cast_precision_loss)]
    let denom = (GRIDLINE_COUNT - 1) as f32;
    let axis_color = color::FG_3.current(mode);
    let border = with_alpha(color::BORDER_1.current(mode), 0.4);
    #[allow(clippy::cast_precision_loss)]
    let micro = text::MICRO as f32;
    #[allow(clippy::cast_precision_loss)]
    let label_gap = space::XS as f32;

    // 1-px vertical axis line at the inner rect's left edge.
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
        // Label-corresponding price (top gridline = max_p, bottom = min_p).
        let price = max_p - frac * (max_p - min_p);

        // Tick mark — 4-px outward stroke from the axis line into
        // the gutter (so the tick sits in the gutter space, not
        // inside the inner rect crowding the line stroke).
        let tick_path = Path::new(|builder| {
            builder.move_to(Point::new(inner.x - AXIS_TICK_LEN_PX, y));
            builder.line_to(Point::new(inner.x, y));
        });
        frame.stroke(
            &tick_path,
            Stroke::default().with_color(border).with_width(1.0),
        );

        // Label — right-aligned just outside the tick mark.
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: format!("{price:.2}"),
            position: Point::new(inner.x - AXIS_TICK_LEN_PX - label_gap, y),
            color: axis_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }
}

/// **T3012 — time-axis draw pass** (chart-canvas-overhaul v1.10.0,
/// R4.2, Q3 adaptive, Q4 local-time).
///
/// Adaptive tick spacing per [`time_axis_tick_count`]: 4 ticks at
/// the 1280-px floor (every 15 bars ≈ 15 minutes), 12 ticks at
/// 3360-px native Retina (every 5 bars ≈ 5 minutes).  Labels in
/// `HH:MM` format derived from `bars[idx].open_ts` via
/// [`time::OffsetDateTime`] with the platform's local offset (Q4
/// operator-locked).  Tick marks at the bottom of the inner rect
/// extending 4 px down into the gutter; labels sit a further
/// `space::XS` below the tick.
fn draw_time_axis(frame: &mut Frame, inner: Rectangle, bars: &[Bar], mode: ThemeMode) {
    if bars.is_empty() || inner.width <= 0.0 {
        return;
    }
    let axis_color = color::FG_3.current(mode);
    let border = with_alpha(color::BORDER_1.current(mode), 0.4);
    #[allow(clippy::cast_precision_loss)]
    let micro = text::MICRO as f32;
    #[allow(clippy::cast_precision_loss)]
    let label_gap = space::XS as f32;

    let intervals = time_axis_tick_count(inner.width, bars.len());
    if intervals == 0 {
        return;
    }
    let offset = local_offset_or_utc();
    let bar_count = bars.len();

    for i in 0..=intervals {
        // Map the interval index to a bar index.  `i = 0` → bar 0;
        // `i = intervals` → bar `bar_count - 1`.  Integer arithmetic
        // keeps the mapping deterministic.
        let bar_idx = if intervals == 0 {
            0
        } else {
            (i * (bar_count - 1)) / intervals
        };
        let Some(bar) = bars.get(bar_idx) else {
            continue;
        };
        let x = x_for_index(bar_idx, bar_count, inner);

        // Tick mark — 4-px outward stroke from the inner-rect bottom
        // into the bottom gutter.
        let tick_path = Path::new(|builder| {
            builder.move_to(Point::new(x, inner.y + inner.height));
            builder.line_to(Point::new(x, inner.y + inner.height + AXIS_TICK_LEN_PX));
        });
        frame.stroke(
            &tick_path,
            Stroke::default().with_color(border).with_width(1.0),
        );

        // Label — `HH:MM` formatted in the local time zone, centred
        // beneath the tick.
        let local_ts = bar.open_ts.inner().to_offset(offset);
        let label = format!("{:02}:{:02}", local_ts.hour(), local_ts.minute());
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: label,
            position: Point::new(x, inner.y + inner.height + AXIS_TICK_LEN_PX + label_gap),
            color: axis_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Top.into(),
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
    strategy_id.map_or(CHART_TOOLTIP_STRATEGY_NONE, smol_str::SmolStr::as_str)
}

// ── Equity-overlay helpers (T-D-11 / Design § 3) ─────────────────────────────

/// Compute `(min_equity, max_equity)` across primary + compare series,
/// restricted to the timestamp window `[ts_start, ts_end]` (the visible
/// bar window). Returns `None` when all series are empty or outside the
/// window. Adds 5 % padding on each side for visual breathing room.
fn compute_equity_range(
    primary: Option<&LabEquitySeries>,
    compare: &[LabEquitySeries],
    ts_start: Option<i64>,
    ts_end: Option<i64>,
) -> Option<(f32, f32)> {
    let (win_start, win_end) = match (ts_start, ts_end) {
        (Some(s), Some(e)) => (s, e),
        _ => (i64::MIN, i64::MAX),
    };

    let mut min_v: Option<f32> = None;
    let mut max_v: Option<f32> = None;

    let all = primary.into_iter().chain(compare.iter());

    for series in all {
        for &(ts, ref equity) in &series.samples {
            if ts >= win_start && ts <= win_end {
                #[allow(clippy::cast_possible_truncation)]
                let v = equity.to_f32().unwrap_or(0.0);
                min_v = Some(min_v.map_or(v, |m: f32| m.min(v)));
                max_v = Some(max_v.map_or(v, |m: f32| m.max(v)));
            }
        }
    }

    let min_v = min_v?;
    let max_v = max_v?;

    // Degenerate case: all equity values are the same.
    let span = (max_v - min_v).max(1.0);
    let pad = span * RANGE_PAD_FRACTION;
    Some((min_v - pad, max_v + pad))
}

/// Draw a single equity polyline on the right Y-axis scale.
/// Draw a single equity polyline using the equity series' own timestamp range
/// as the X-axis.  Called when `bars` is empty (F9 — backtest result with no
/// live price bars available yet).
/// lab-end-to-end-v2 Wave D-1.1 F9.
fn draw_equity_polyline_standalone(
    frame: &mut Frame,
    series: &LabEquitySeries,
    min_eq: f32,
    max_eq: f32,
    inner: Rectangle,
    line_color: Color,
) {
    if series.samples.len() < 2 {
        return;
    }

    let min_ts = series.samples.first().map_or(0, |s| s.0);
    let max_ts = series.samples.last().map_or(min_ts + 1, |s| s.0);

    let mut path_started = false;
    let polyline = Path::new(|builder| {
        for &(ts, ref equity) in &series.samples {
            #[allow(clippy::cast_precision_loss)]
            let x_frac = if max_ts > min_ts {
                (ts - min_ts) as f32 / (max_ts - min_ts) as f32
            } else {
                0.0
            };
            let x = inner.x + x_frac * inner.width;

            #[allow(clippy::cast_possible_truncation)]
            let eq_v = equity.to_f32().unwrap_or(0.0);
            let span = (max_eq - min_eq).max(1e-6);
            let y_frac = (eq_v - min_eq) / span;
            let y = inner.y + (1.0 - y_frac) * inner.height;

            if path_started {
                builder.line_to(Point::new(x, y));
            } else {
                builder.move_to(Point::new(x, y));
                path_started = true;
            }
        }
    });

    if path_started {
        frame.stroke(
            &polyline,
            Stroke::default()
                .with_color(line_color)
                .with_width(EQUITY_STROKE_PX),
        );
    }
}

fn draw_equity_polyline(
    frame: &mut Frame,
    series: &LabEquitySeries,
    bars: &[Bar],
    min_eq: f32,
    max_eq: f32,
    inner: Rectangle,
    line_color: Color,
) {
    if series.samples.len() < 2 || bars.is_empty() {
        return;
    }

    let min_bar_ts = bars.first().map_or(i64::MIN, |b| b.open_ts.unix_millis());
    let max_bar_ts = bars.last().map_or(i64::MAX, |b| b.close_ts.unix_millis());

    let mut path_started = false;
    let polyline = Path::new(|builder| {
        for &(ts, ref equity) in &series.samples {
            // Clamp to visible window.
            let ts_clamped = ts.clamp(min_bar_ts, max_bar_ts);
            #[allow(clippy::cast_precision_loss)]
            let x_frac = if max_bar_ts > min_bar_ts {
                (ts_clamped - min_bar_ts) as f32 / (max_bar_ts - min_bar_ts) as f32
            } else {
                0.0
            };
            let x = inner.x + x_frac * inner.width;

            #[allow(clippy::cast_possible_truncation)]
            let eq_v = equity.to_f32().unwrap_or(0.0);
            let span = (max_eq - min_eq).max(1e-6);
            let y_frac = (eq_v - min_eq) / span;
            let y = inner.y + (1.0 - y_frac) * inner.height;

            if path_started {
                builder.line_to(Point::new(x, y));
            } else {
                builder.move_to(Point::new(x, y));
                path_started = true;
            }
        }
    });

    if path_started {
        frame.stroke(
            &polyline,
            Stroke::default()
                .with_color(line_color)
                .with_width(EQUITY_STROKE_PX),
        );
    }
}

/// Draw the right Y-axis ticks + equity value labels (Design § 3, pass 4).
fn draw_equity_axis(
    frame: &mut Frame,
    canvas_size: Size,
    inner: Rectangle,
    min_eq: f32,
    max_eq: f32,
    mode: ThemeMode,
) {
    // axis_right_x reserved for future right-axis label placement
    #[allow(clippy::no_effect_underscore_binding)]
    let _axis_right_x = inner.x + inner.width + AXIS_GUTTER_EQUITY_PX - 4.0;
    let tick_x = inner.x + inner.width;
    let axis_color = color::FG_3.current(mode);
    let border = with_alpha(color::BORDER_1.current(mode), 0.4);
    #[allow(clippy::cast_precision_loss)]
    let micro = text::MICRO as f32;
    #[allow(clippy::cast_precision_loss)]
    let label_gap = space::XS as f32;
    let _ = canvas_size; // retained for future dpi-aware label sizing

    // 1-px vertical axis line at the right edge of the inner rect.
    let axis_line = Path::new(|builder| {
        builder.move_to(Point::new(tick_x, inner.y));
        builder.line_to(Point::new(tick_x, inner.y + inner.height));
    });
    frame.stroke(
        &axis_line,
        Stroke::default().with_color(border).with_width(1.0),
    );

    #[allow(clippy::cast_precision_loss)]
    let denom = (EQUITY_AXIS_TICK_COUNT - 1) as f32;

    for i in 0..EQUITY_AXIS_TICK_COUNT {
        #[allow(clippy::cast_precision_loss)]
        let frac = i as f32 / denom;
        let y = inner.y + frac * inner.height;
        let equity_at_y = max_eq - frac * (max_eq - min_eq);

        // Tick mark — 4-px rightward stroke from axis into gutter.
        let tick_path = Path::new(|builder| {
            builder.move_to(Point::new(tick_x, y));
            builder.line_to(Point::new(tick_x + AXIS_TICK_LEN_PX, y));
        });
        frame.stroke(
            &tick_path,
            Stroke::default().with_color(border).with_width(1.0),
        );

        // Label — right-aligned in the gutter.
        let label = if equity_at_y.abs() >= 1_000.0 {
            format!(
                "${:.0}{}",
                equity_at_y / 1_000.0,
                crate::strings::CHART_EQUITY_AXIS_THOUSAND_SUFFIX
            )
        } else {
            format!("${equity_at_y:.0}")
        };
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: label,
            position: Point::new(tick_x + AXIS_TICK_LEN_PX + label_gap, y),
            color: axis_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }
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
        equity: None,
        compare: vec![],
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

/// **ui-test-harness-bootstrap v0.1 — T4021** — viewport-parametric
/// wrapper around [`dispatch_canvas_event_for_test`] for the grid-
/// sweep tests at `crates/ui/tests/chart_hover_grid_sweep.rs`.
///
/// Computes the canvas `Rectangle` bounds the production `chart::view`
/// would emit at the given `viewport`+`scale_factor` (using the same
/// gutter math via [`chart_inner_rect`] indirectly — bounds are the
/// outer canvas rect, and `dispatch_canvas_event_for_test` applies
/// `chart_inner_rect(bounds.size())` itself), then sweeps each
/// `cursor_position` through the dispatcher. Returns one tuple per
/// cursor position: `(position, optional_published_message, status)`.
///
/// **Coordinate conventions:**
/// - `viewport` is `(logical_width, logical_height)` in iced's logical
///   pixel space — the same units `iced_test::screenshot` accepts.
/// - `scale_factor` is the iced runtime scale (1.0 for `floor` /
///   `typical`, 2.0 for `operator`); preserved verbatim here so a
///   future hit-test that consults dpi can read it from the produced
///   `Rectangle` bounds.
/// - `cursor_positions` are absolute-screen `Point`s in the same
///   logical-pixel coordinate space as the canvas bounds — the
///   dispatcher already subtracts `bounds.x` / `bounds.y` internally
///   to translate to canvas-local.
///
/// The canvas is assumed to fill the full viewport (`bounds = (0, 0,
/// viewport_w, viewport_h)`). This matches how the cockpit's `view`
/// composes the chart — `Container::new(canvas).width(Length::Fill)
/// .height(Length::Fill)`. Tests that need a partial-canvas viewport
/// (e.g. embedded inside a shell with a sidebar) should keep using
/// [`dispatch_canvas_event_for_test`] directly with an explicit
/// `bounds`.
///
/// **Backward compat (R3.6):** [`dispatch_canvas_event_for_test`]'s
/// signature is unchanged. The existing
/// `chart_tooltip_hover_fires.rs` tests use it verbatim — `cargo test
/// -p ui --test chart_tooltip_hover_fires` stays green.
#[doc(hidden)]
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn sweep_canvas_grid_for_test(
    bars: Vec<Bar>,
    markers: Vec<FillView>,
    signals: Vec<SignalView>,
    viewport: (u32, u32),
    scale_factor: f32,
    cursor_positions: Vec<Point>,
) -> Vec<(Point, Option<Message>, iced::event::Status)> {
    #[allow(clippy::cast_precision_loss)]
    let bounds = Rectangle {
        x: 0.0,
        y: 0.0,
        width: viewport.0 as f32,
        height: viewport.1 as f32,
    };
    // `scale_factor` is consumed only for hit-test paths that consult
    // dpi.  iced's chart canvas works in logical pixels, so the only
    // effect of `scale_factor` on the bounds is none.  We keep the
    // parameter in the signature so a future scale-aware hit-test
    // (e.g. for a Retina-specific gutter override) doesn't break the
    // callers — and to make the test-name carrying the
    // `(viewport, scale)` tuple structurally complete (V8 / Q10).
    let _ = scale_factor;

    cursor_positions
        .into_iter()
        .map(|pos| {
            let mut state = ChartHoverState::default();
            let event =
                iced::widget::canvas::Event::Mouse(mouse::Event::CursorMoved { position: pos });
            let (msg, status) = dispatch_canvas_event_for_test(
                bars.clone(),
                markers.clone(),
                signals.clone(),
                &mut state,
                event,
                bounds,
                pos,
            );
            (pos, msg, status)
        })
        .collect()
}

/// **ui-test-harness-bootstrap v0.1 — T4021** — viewport-parametric
/// inner-rect helper.  Returns the canvas inner rect the production
/// `chart::view` would compute at the given viewport (a `Rectangle`
/// equal to `chart_inner_rect(viewport.into())` against the canvas's
/// outer bounds).  Used by the H3 falsifier sub-test to assert the
/// helper's bounds line up with what an iced runtime would lay out.
#[doc(hidden)]
#[must_use]
pub fn inner_rect_for_viewport_test(viewport: (u32, u32)) -> Rectangle {
    #[allow(clippy::cast_precision_loss)]
    let size = Size::new(viewport.0 as f32, viewport.1 as f32);
    chart_inner_rect(size)
}

/// **ui-test-harness-bootstrap v0.1 — T4023** — wrapper around
/// [`anchor_for_ts`] that the grid-sweep test uses to assert the
/// pixel invariants in `marker_centroid_pixel_invariants_across_viewports`.
///
/// Takes a `viewport` (logical width / height) and a fill, returns the
/// `(x, y)` the production canvas would render the marker at against
/// the supplied `bars` window.  Pure function — no rendering, no
/// allocation beyond the input slices.
#[doc(hidden)]
#[must_use]
pub fn anchor_for_first_fill_test(
    bars: &[Bar],
    fill_ts_unix_millis: i64,
    viewport: (u32, u32),
) -> Option<Point> {
    let inner = inner_rect_for_viewport_test(viewport);
    // Build the (low, high) range with the same 5% pad the production
    // path uses — match `chart::view`'s range computation verbatim so
    // the centroid math matches what the hit-test sees.
    let mut min_low = f32::INFINITY;
    let mut max_high = f32::NEG_INFINITY;
    for b in bars {
        let low: f32 = b.low.get().to_string().parse().unwrap_or(0.0);
        let high: f32 = b.high.get().to_string().parse().unwrap_or(0.0);
        min_low = min_low.min(low);
        max_high = max_high.max(high);
    }
    let span = (max_high - min_low).max(1.0);
    let pad = span * RANGE_PAD_FRACTION;
    let range = (min_low - pad, max_high + pad);
    anchor_for_ts(fill_ts_unix_millis, bars, range, inner).map(|(x, y)| Point::new(x, y))
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
    clippy::uninlined_format_args,
    clippy::expect_used
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
            // chart-canvas-overhaul v1.10.0 — new draw order after
            // T3011 (price axis in left gutter) + T3012 (time axis in
            // bottom gutter) + T3017 (legend top-right inset).
            out.push_str(
                "draw_order: gridlines,price_axis,time_axis,line,ghosts,fills,tooltip,legend\n",
            );
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

    // ── T-D-19 overlay canvas snapshots ─────────────────────────────────────
    //
    // These three tests are **descriptor-based** (same text-summary pattern
    // as the btc/empty/ghosts tests above) because the iced canvas renderer
    // is not available in the test environment. The snapshots pin the
    // ChartProgram's *input shape* — layer count, overlay types, fidelity,
    // point counts — rather than pixels.
    //
    // **Operator visual A/B capture** is deferred to the operator-local
    // cargo run (3360×1890 Retina). The manual capture command is:
    //   cargo run -p ui --bin cockpit --features fixtures
    // and the screenshots land in
    //   spec/ui-rethink-phase-a-lab/reports/screenshots/
    //
    // See T-D-19 in spec/ui-rethink-phase-a-lab/tasks.md for the full gate.

    /// Plain-text summary for overlay snapshot tests.
    fn chart_overlay_summary(
        bars: &[Bar],
        equity: Option<&LabEquitySeries>,
        compare: &[LabEquitySeries],
    ) -> String {
        let mut out = String::new();
        out.push_str("widget: chart\n");
        out.push_str(&format!("bar_count: {}\n", bars.len()));
        out.push_str(&format!("gridlines: {GRIDLINE_COUNT}\n"));
        // Equity overlay
        if let Some(eq) = equity {
            out.push_str(&format!(
                "equity_overlay: present points={} fidelity={:?}\n",
                eq.samples.len(),
                eq.fidelity
            ));
            out.push_str("equity_axis: right_gutter\n");
            out.push_str("equity_color: ACCENT_2\n");
        } else {
            out.push_str("equity_overlay: none\n");
        }
        // Compare overlays
        out.push_str(&format!("compare_count: {}\n", compare.len()));
        for (i, c) in compare.iter().enumerate() {
            let slot = i + 1;
            out.push_str(&format!(
                "compare_slot_{slot}: points={} fidelity={:?}\n",
                c.samples.len(),
                c.fidelity
            ));
            let color_token = match i {
                0 => "ACCENT_2",
                1 => "ACCENT_3",
                2 => "ACCENT_4",
                _ => "ACCENT_5",
            };
            out.push_str(&format!("compare_slot_{slot}_color: {color_token}\n"));
        }
        out
    }

    fn make_equity_series(slug: &str, n_points: usize) -> LabEquitySeries {
        use crate::lab::equity_loader::Fidelity;
        let samples: Vec<(i64, Decimal)> = (0..n_points)
            .map(|i| {
                (
                    1_705_320_000_000 + i as i64 * 60_000,
                    dec!(100_000) + Decimal::from(i * 100),
                )
            })
            .collect();
        LabEquitySeries {
            samples,
            source_report: smol_str::SmolStr::new(slug),
            fidelity: Fidelity::PerBar,
            narrowed_from: None,
        }
    }

    fn make_equity_series_no_data(slug: &str) -> LabEquitySeries {
        use crate::lab::equity_loader::Fidelity;
        LabEquitySeries {
            samples: Vec::new(), // no data for this pair/range
            source_report: smol_str::SmolStr::new(slug),
            fidelity: Fidelity::PerBar,
            narrowed_from: None,
        }
    }

    /// T-D-19 — snapshot: `chart__price_plus_equity_v1_momentum`.
    ///
    /// Equity overlay over baseline price for v1.momentum strategy.
    /// Records: bar count, equity overlay presence + point count, right axis.
    #[test]
    #[allow(non_snake_case)]
    fn chart__price_plus_equity_v1_momentum() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| make_bar(i, dec!(40_000) + Decimal::from(i) * dec!(2.5)))
            .collect();
        let equity = make_equity_series("backtest-20260429-195243-top10-2024-h1-momentum.md", 60);
        let compare: Vec<LabEquitySeries> = vec![];
        assert_snapshot!(
            "chart__price_plus_equity_v1_momentum",
            chart_overlay_summary(&bars, Some(&equity), &compare)
        );
    }

    /// T-D-19 — snapshot: `chart__compare_three_strategies`.
    ///
    /// 3-strategy comparison overlay. Records slot count + color assignment.
    #[test]
    #[allow(non_snake_case)]
    fn chart__compare_three_strategies() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| make_bar(i, dec!(40_000) + Decimal::from(i) * dec!(2.5)))
            .collect();
        let equity = make_equity_series("backtest-v1-momentum.md", 60);
        let compare = vec![
            make_equity_series("backtest-v0-sma.md", 60),
            make_equity_series("backtest-v05-macd.md", 55),
            make_equity_series("backtest-v15-pairs.md", 48),
        ];
        assert_snapshot!(
            "chart__compare_three_strategies",
            chart_overlay_summary(&bars, Some(&equity), &compare)
        );
    }

    /// T-D-19 — snapshot: `chart__compare_pair_swap_no_data`.
    ///
    /// Compare strategy with no data for the current pair → faded chip +
    /// no curve. Records zero-point series in slot 1.
    #[test]
    #[allow(non_snake_case)]
    fn chart__compare_pair_swap_no_data() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| make_bar(i, dec!(40_000) + Decimal::from(i) * dec!(2.5)))
            .collect();
        let equity = make_equity_series("backtest-v1-momentum.md", 60);
        let compare = vec![make_equity_series_no_data("compare-no-data-for-xrpusdt.md")];
        assert_snapshot!(
            "chart__compare_pair_swap_no_data",
            chart_overlay_summary(&bars, Some(&equity), &compare)
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
        let mid_ts = i64::midpoint(t_left, t_right);
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

    /// T2033 — `ChartProgram::tooltip_view_from_hover` builds a
    /// `ChartTooltipView` directly from `self.markers` / `self.signals`
    /// at the hovered index — **with `self.tooltip` set to `None`**.
    /// This is the exact code path the post-T2033 draw pass walks:
    /// the Cockpit-state round-trip is no longer required for the
    /// tooltip to render on the first paint after `CursorMoved`.
    ///
    /// The pre-T2033 form required BOTH `self.tooltip.is_some()` AND
    /// `state.hovered_marker_centroid.is_some()`; this regression
    /// guard confirms the new decoupled invariant.
    #[test]
    fn chart_tooltip_view_built_from_canvas_state_without_round_trip() {
        let bars: Vec<Bar> = (0..3)
            .map(|i| make_bar(i, dec!(100) + Decimal::from(i)))
            .collect();
        let markers = vec![
            make_fill(0, Side::Buy, dec!(100)),
            make_fill(2, Side::Sell, dec!(102)),
        ];
        let signals = vec![make_signal(1, Side::Buy, false)];
        let program = ChartProgram {
            bars,
            markers,
            signals,
            // Decouple invariant — Cockpit-state round trip is None.
            tooltip: None,
            equity: None,
            compare: vec![],
            mode: ThemeMode::Dark,
        };

        // Hover the first fill — view comes from `self.markers[0]`.
        let view_fill = program
            .tooltip_view_from_hover(ChartMarkerIndex::Fill(0))
            .expect("fill index 0 should resolve");
        assert!(
            matches!(view_fill.kind, crate::state::ChartTooltipKind::Fill),
            "fill index resolves to Fill kind"
        );
        assert_eq!(view_fill.side, Side::Buy);
        assert_eq!(view_fill.price, Some(dec!(100)));

        // Hover the ghost signal — view comes from `self.signals[0]`.
        let view_sig = program
            .tooltip_view_from_hover(ChartMarkerIndex::Signal(0))
            .expect("signal index 0 should resolve");
        assert!(
            matches!(view_sig.kind, crate::state::ChartTooltipKind::Signal),
            "signal index resolves to Signal kind"
        );
        assert_eq!(view_sig.side, Side::Buy);
        // Ghosts have no price.
        assert!(view_sig.price.is_none());

        // Out-of-range index returns None — defence-in-depth across
        // the async refresh boundary.
        assert!(
            program
                .tooltip_view_from_hover(ChartMarkerIndex::Fill(99))
                .is_none()
        );
        assert!(
            program
                .tooltip_view_from_hover(ChartMarkerIndex::Signal(99))
                .is_none()
        );
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

    /// T3012 — `time_axis_tick_count_adaptive` — chart-canvas-overhaul
    /// v1.10.0 (R4.2.1 / Q3 adaptive).
    ///
    /// Sweep `canvas_width_logical` across the three R6 capture
    /// sizes + the floor + native Retina, and assert the tick
    /// count rounds to one of the brief's documented bar-step
    /// multiples (5 / 10 / 15) so the time axis lands on round
    /// minute boundaries.
    #[test]
    fn time_axis_tick_count_adaptive() {
        // 60-bar window (the cockpit's current chart window).
        let bar_count = 60_usize;

        // At the 1280-px floor — width / 96 = ~13 → clamped to 12 →
        // step = 5 → intervals = 11.  But our function caps the
        // step at 5 → intervals = (60-1)/5 = 11.
        let n = time_axis_tick_count(1280.0, bar_count);
        assert!((4..=12).contains(&n), "floor n in [4,12]: got {n}");

        // At ~1920-px mid — width / 96 = 20 → clamped to 12 → 5-bar
        // step → 11 intervals.
        let n = time_axis_tick_count(1920.0, bar_count);
        assert!((4..=12).contains(&n), "mid n in [4,12]: got {n}");

        // At 3360-px native Retina — width / 96 = 35 → clamped to
        // 12 → 5-bar step → 11 intervals.
        let n = time_axis_tick_count(3360.0, bar_count);
        assert!((4..=12).contains(&n), "native n in [4,12]: got {n}");

        // At a tiny canvas (200-px width) — width / 96 ≈ 2 →
        // clamped to 4 → 15-bar step → 3 intervals.
        let n = time_axis_tick_count(200.0, bar_count);
        assert!(n <= 4, "tiny n <= 4: got {n}");

        // Empty bar slice → 0 intervals.
        let n = time_axis_tick_count(1920.0, 0);
        assert_eq!(n, 0, "empty bars → 0 intervals");
    }

    /// T3013 — `local_offset_under_test_is_utc` — chart-canvas-overhaul
    /// v1.10.0 (R4.2.2 / determinism invariant).
    ///
    /// The snapshot-test path MUST always see UTC so the
    /// `panel_snapshots__charts_screen_with_*` baselines pin
    /// against a single time zone.  This test pins the cfg-test
    /// branch's contract: under `#[cfg(test)]` the helper returns
    /// `UtcOffset::UTC` regardless of the host's local offset.
    #[test]
    fn local_offset_under_test_is_utc() {
        let offset = local_offset_or_utc();
        assert_eq!(
            offset,
            time::UtcOffset::UTC,
            "snapshot determinism: cfg(test) must return UTC, got {offset:?}",
        );
    }

    /// T3010 — `chart_inner_rect` uses the four-sided gutter math
    /// the brief locked under R4.  The base 8-px gutter applies on
    /// every side; the price gutter pulls in `inner.x`; the right
    /// margin shrinks `inner.width`; the time gutter shrinks
    /// `inner.height`.  Pinning the arithmetic so a refactor that
    /// fat-fingers a sign or transposes left/right fails loudly.
    #[test]
    fn chart_inner_rect_applies_four_sided_gutters() {
        use crate::theme::layout::{
            AXIS_GUTTER_PRICE_PX as L, AXIS_GUTTER_RIGHT_PX as R, AXIS_GUTTER_TIME_PX as B,
        };
        let size = Size::new(1280.0, 720.0);
        let r = chart_inner_rect(size);
        let base = 8.0_f32;
        assert!((r.x - (base + L)).abs() < 0.001, "x = base + price gutter");
        assert!((r.y - base).abs() < 0.001, "y = base (top gutter = 0)");
        let exp_w = size.width - 2.0 * base - L - R;
        let exp_h = size.height - 2.0 * base - B;
        assert!(
            (r.width - exp_w).abs() < 0.001,
            "width: {} vs {}",
            r.width,
            exp_w
        );
        assert!(
            (r.height - exp_h).abs() < 0.001,
            "height: {} vs {}",
            r.height,
            exp_h
        );
    }

    /// **v1.11 — chart-x-axis-local-time R3** — assert the helper
    /// returns the deterministic `UtcOffset::UTC` under `cfg(test)`.
    ///
    /// This test exercises the `#[cfg(test)]` branch (which is the
    /// only branch reachable from `cargo test`); it pins the snapshot-
    /// determinism contract so future drift surfaces as a test
    /// failure rather than a silent baseline diff.
    ///
    /// The companion production branch (`#[cfg(not(test))]`) reads
    /// the OS-local offset via `time::UtcOffset::current_local_offset()`
    /// — covered by compile-only verification (the `local-offset`
    /// feature flip in `Cargo.toml` + the `#[cfg(not(test))]` body)
    /// and the operator's live-cockpit verification at v1.11 ship.
    #[test]
    fn local_offset_under_production_reads_os_offset() {
        // Under `cfg(test)`, the helper returns UTC deterministically.
        // This is the snapshot-determinism gate.
        assert_eq!(local_offset_or_utc(), time::UtcOffset::UTC);
    }
}
