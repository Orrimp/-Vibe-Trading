//! Chart widget — Phase 2 (T1608).
//!
//! Canvas-based per-symbol price chart for the Charts screen. Read-only:
//! no mouse handlers, no hover state, no click events emitted. Renders
//! gridlines + a polyline through `Bar.close` in `ACCENT` + buy/sell
//! markers (`UP_500` / `DOWN_500` filled triangles).
//!
//! Empty state (`bars` is empty): paints gridlines + a centred "No data"
//! label.
//!
//! **Zero string literals** — copy via `crate::strings::CHART_NO_DATA`.
//! **Zero hex colours** — tokens via `crate::theme`.
//!
//! ## Coordinate system
//!
//! - X = time, oldest-left to newest-right.
//! - Y = price, low-bottom to high-top.
//! - Y-axis range = `(min_low, max_high)` over the bars window with a
//!   5% padding above and below so markers at the edge stay visible.
//!
//! Phase 2 ships the line-series default (Q1); the OHLC variant remains
//! supportable from the same `ChartBuffer` shape but is not stubbed here
//! (no dead code).

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::widget::{container, Canvas, Container};
use iced::{mouse, Color, Length, Point, Rectangle, Renderer};
use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, FillView, Side};

use super::canvas_chart::{
    draw_gridlines, inner_rect, with_alpha, GRIDLINE_COUNT, LINE_STROKE_PX, RANGE_PAD_FRACTION,
};
use crate::state::Message;
use crate::strings::CHART_NO_DATA;
use crate::theme::{color, text, ThemeMode};

/// Triangle size for buy/sell markers.
const MARKER_SIZE_PX: f32 = 6.0;

/// Render the chart for the active `(venue, symbol)` against the current
/// `bars` window. Returns gridlines + line series + markers in one canvas.
///
/// Takes owned `Vec`s rather than borrowed slices so the canvas
/// `Program` impl can hold them across iced's render lifetime without
/// borrowing from `Cockpit`. 60 bars × ~200 B per bar = ~12 KB per
/// repaint — trivially within budget.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn view<'a>(bars: Vec<Bar>, markers: Vec<FillView>, mode: ThemeMode) -> crate::Element<'a> {
    let program = ChartProgram {
        bars,
        markers,
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

struct ChartProgram {
    bars: Vec<Bar>,
    markers: Vec<FillView>,
    mode: ThemeMode,
}

impl canvas::Program<Message> for ChartProgram {
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

        // Gridlines — 5 horizontal lines, BORDER_1 at 0.4 alpha.
        draw_gridlines(&mut frame, inner, border);

        if self.bars.is_empty() {
            // Empty-state: centred "No data" label.
            let centre = Point::new(inner.x + inner.width / 2.0, inner.y + inner.height / 2.0);
            #[allow(clippy::cast_precision_loss)]
            let micro_size = text::MICRO as f32;
            // `useless_conversion`: `align_x`/`align_y` field types in
            // `canvas::Text` are crate-private aliases that ARE distinct
            // from `iced::alignment::Horizontal/Vertical`; the `.into()`
            // is load-bearing for iced 0.14's API. clippy disagrees.
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

        // Axis labels (price values in the left gutter).
        draw_price_labels(&mut frame, bounds.size(), range, self.mode);

        // Line series — polyline through Bar.close.
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

        // Markers — filled triangles.
        if let (Some(first_bar), Some(last_bar)) = (self.bars.first(), self.bars.last()) {
            let min_ts = first_bar.open_ts.unix_millis();
            let max_ts = last_bar.close_ts.unix_millis();
            for fill in &self.markers {
                let fill_ts = fill.venue_ts.unix_millis();
                if fill_ts < min_ts || fill_ts > max_ts {
                    // Defence-in-depth clip; the marker query bounds should
                    // already match the visible window (R8.1).
                    continue;
                }
                let x_frac = ts_fraction(fill_ts, min_ts, max_ts);
                let x = inner.x + x_frac * inner.width;
                let price = decimal_to_f32(&fill.price.get());
                let y = y_for_price(price, range, inner);
                let (color, upward) = match fill.side {
                    Side::Buy => (color::UP_500.current(self.mode), true),
                    Side::Sell => (color::DOWN_500.current(self.mode), false),
                };
                draw_triangle(&mut frame, Point::new(x, y), color, upward);
            }
        }

        vec![frame.into_geometry()]
    }
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
        // `useless_conversion` — see note in the empty-state arm above.
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

fn draw_triangle(frame: &mut Frame, anchor: Point, color: Color, upward: bool) {
    let half = MARKER_SIZE_PX / 2.0;
    let height = MARKER_SIZE_PX;
    let path = Path::new(|builder| {
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
    });
    frame.fill(&path, color);
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

fn x_for_index(idx: usize, count: usize, inner: Rectangle) -> f32 {
    if count <= 1 {
        return inner.x;
    }
    #[allow(clippy::cast_precision_loss)]
    let frac = idx as f32 / (count - 1) as f32;
    inner.x + frac * inner.width
}

fn y_for_price(price: f32, range: (f32, f32), inner: Rectangle) -> f32 {
    let (min_p, max_p) = range;
    let span = (max_p - min_p).max(1e-6);
    let frac = (price - min_p) / span;
    // y axis is flipped — high prices render near the top of the canvas.
    inner.y + (1.0 - frac) * inner.height
}

fn ts_fraction(ts: i64, min_ts: i64, max_ts: i64) -> f32 {
    if max_ts <= min_ts {
        return 0.0;
    }
    let span = max_ts - min_ts;
    #[allow(clippy::cast_precision_loss)]
    let frac = (ts - min_ts) as f32 / span as f32;
    frac.clamp(0.0, 1.0)
}

fn decimal_to_f32(d: &rust_decimal::Decimal) -> f32 {
    d.to_f32().unwrap_or(0.0)
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

    /// Plain-text summary of what the chart canvas would render. Pinned via
    /// snapshot so a regression in line/marker counts or axis range is
    /// visible without a pixel-level renderer.
    #[allow(clippy::ptr_arg)]
    fn chart_summary(bars: &[Bar], markers: &[FillView]) -> String {
        let mut out = String::new();
        out.push_str("widget: chart\n");
        out.push_str(&format!("bar_count: {}\n", bars.len()));
        out.push_str(&format!("gridlines: {}\n", GRIDLINE_COUNT));
        if bars.is_empty() {
            out.push_str("empty_state: true\n");
            out.push_str(&format!("empty_label: {CHART_NO_DATA}\n"));
        } else {
            out.push_str("empty_state: false\n");
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
            out.push_str(&format!("markers_buy: {buys}\n"));
            out.push_str(&format!("markers_sell: {sells}\n"));
            out.push_str(&format!(
                "marker_buy_color: UP_500\nmarker_sell_color: DOWN_500\n"
            ));
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
        assert_snapshot!(
            "chart__btc_with_two_buys_one_sell",
            chart_summary(&bars, &markers)
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn chart__empty_state_no_data() {
        let bars: Vec<Bar> = Vec::new();
        let markers: Vec<FillView> = Vec::new();
        assert_snapshot!("chart__empty_state_no_data", chart_summary(&bars, &markers));
    }
}
