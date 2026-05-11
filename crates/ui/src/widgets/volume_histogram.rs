//! Per-bar volume histogram — chart-buy-sell-emphasis v1.9 (M4 / T2023).
//!
//! Canvas-based two-color stacked-bar widget rendered **below** the
//! Charts-screen price chart at a fixed 80-px container height (R7.2).
//! Each input bar contributes a paired up/down bar split at the
//! horizontal centreline:
//!
//! - Buy volume (USDT notional) bars up in `UP_500` (sage).
//! - Sell volume (USDT notional) bars down in `DOWN_500` (clay).
//!
//! The widget is a **sibling** of [`widgets::chart`] (Q7-resolved Option
//! (b) — new widget, not a sparkline extension). Reuses
//! [`canvas_chart`]'s `inner_rect` + `with_alpha` + `GRIDLINE_COUNT`
//! primitives.
//!
//! **Empty state (R7.6):** when `bins` is empty, paints a single horizontal
//! gridline + a centred `—` placeholder. No blank screens.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::canvas::{self, Frame, Geometry, Path, Text as CanvasText};
use iced::widget::{container, Canvas, Container};
use iced::{mouse, Length, Point, Rectangle, Renderer, Size};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

#[cfg(test)]
use super::canvas_chart::GRIDLINE_COUNT;
use super::canvas_chart::{inner_rect, with_alpha};
use crate::state::Message;
use crate::strings::KPI_DASH_PLACEHOLDER;
use crate::theme::{color, text, ThemeMode};

/// One histogram bin — paired buy + sell USDT-notional totals for a bar.
/// Local to this widget; built at compose time in
/// [`crate::screens::charts`] from `model.chart_markers` aggregated per
/// `Bar.close_ts`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VolumeBin {
    pub buys_usdt: Decimal,
    pub sells_usdt: Decimal,
}

impl VolumeBin {
    /// Whether this bin has any volume (buys OR sells > 0).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buys_usdt.is_zero() && self.sells_usdt.is_zero()
    }
}

/// Render the per-bar histogram. Takes `bins` by value because the canvas
/// `Program` impl holds them across iced's render lifetime; ~60 bins ×
/// ~16 B per bin = ~1 KB per repaint — trivially in budget.
#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn view<'a>(bins: Vec<VolumeBin>, mode: ThemeMode) -> crate::Element<'a> {
    let program = HistogramProgram { bins, mode };
    let canvas: Canvas<HistogramProgram, Message> = Canvas::new(program)
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

struct HistogramProgram {
    bins: Vec<VolumeBin>,
    mode: ThemeMode,
}

impl canvas::Program<Message> for HistogramProgram {
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
        let baseline_y = inner.y + inner.height / 2.0;
        let border = with_alpha(color::BORDER_1.current(self.mode), 0.4);

        // Centreline — split between buy/sell stacks.
        let centreline = Path::line(
            Point::new(inner.x, baseline_y),
            Point::new(inner.x + inner.width, baseline_y),
        );
        frame.stroke(
            &centreline,
            canvas::Stroke::default().with_color(border).with_width(1.0),
        );

        let active_bins = self.bins.iter().filter(|b| !b.is_empty()).count();
        if self.bins.is_empty() || active_bins == 0 {
            // Empty-state placeholder per R7.6.
            let centre = Point::new(inner.x + inner.width / 2.0, baseline_y);
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

        // Range — max(buy_usdt, sell_usdt) across all bins controls the
        // vertical scale. Half-height per side so the largest bar fills
        // ~95 % of half-height (5 % top padding).
        let mut max_v = 0f32;
        for b in &self.bins {
            let bu = decimal_to_f32(&b.buys_usdt).abs();
            let sd = decimal_to_f32(&b.sells_usdt).abs();
            if bu > max_v {
                max_v = bu;
            }
            if sd > max_v {
                max_v = sd;
            }
        }
        if max_v <= 0.0 {
            max_v = 1.0;
        }
        let half_h = (inner.height / 2.0) * 0.95;

        // Bar width — divide inner.width equally; pad ~30 % per bin so the
        // bars read as bars, not as a solid block.
        #[allow(clippy::cast_precision_loss)]
        let n = self.bins.len() as f32;
        let slot = inner.width / n;
        let bar_w = (slot * 0.7).max(1.0);

        let buy_color = color::UP_500.current(self.mode);
        let sell_color = color::DOWN_500.current(self.mode);

        for (i, bin) in self.bins.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let x_centre = inner.x + (i as f32 + 0.5) * slot;
            let x_left = x_centre - bar_w / 2.0;

            // Buy bar — UP from baseline.
            let bu = decimal_to_f32(&bin.buys_usdt).abs();
            if bu > 0.0 {
                let h = (bu / max_v) * half_h;
                let rect = Path::rectangle(Point::new(x_left, baseline_y - h), Size::new(bar_w, h));
                frame.fill(&rect, buy_color);
            }

            // Sell bar — DOWN from baseline.
            let sd = decimal_to_f32(&bin.sells_usdt).abs();
            if sd > 0.0 {
                let h = (sd / max_v) * half_h;
                let rect = Path::rectangle(Point::new(x_left, baseline_y), Size::new(bar_w, h));
                frame.fill(&rect, sell_color);
            }
        }

        vec![frame.into_geometry()]
    }
}

fn decimal_to_f32(d: &Decimal) -> f32 {
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
    use rust_decimal_macros::dec;

    fn summary(bins: &[VolumeBin]) -> String {
        let mut out = String::new();
        out.push_str("widget: volume_histogram\n");
        out.push_str(&format!("bin_count: {}\n", bins.len()));
        let active = bins.iter().filter(|b| !b.is_empty()).count();
        out.push_str(&format!("active_bins: {active}\n"));
        if active == 0 {
            out.push_str("empty_state: true\n");
            out.push_str(&format!("empty_placeholder: {KPI_DASH_PLACEHOLDER}\n"));
        } else {
            out.push_str("empty_state: false\n");
            out.push_str("draw_order: centreline,bars\n");
            out.push_str(&format!("gridlines: {}\n", GRIDLINE_COUNT));
            out.push_str("buy_color: UP_500\n");
            out.push_str("sell_color: DOWN_500\n");
            let total_buy: Decimal = bins.iter().map(|b| b.buys_usdt).sum();
            let total_sell: Decimal = bins.iter().map(|b| b.sells_usdt).sum();
            out.push_str(&format!("total_buy_usdt: {total_buy}\n"));
            out.push_str(&format!("total_sell_usdt: {total_sell}\n"));
        }
        out
    }

    #[test]
    fn volume_histogram_empty_renders_placeholder() {
        let bins: Vec<VolumeBin> = Vec::new();
        let s = summary(&bins);
        assert!(s.contains("empty_state: true"));
        assert!(s.contains("empty_placeholder"));
    }

    #[test]
    fn volume_histogram_zero_bins_render_placeholder() {
        let bins = vec![VolumeBin::default(), VolumeBin::default()];
        let s = summary(&bins);
        assert!(s.contains("empty_state: true"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn volume_histogram__btc_three_buys_two_sells() {
        let bins = vec![
            VolumeBin {
                buys_usdt: dec!(10_000),
                sells_usdt: dec!(0),
            },
            VolumeBin {
                buys_usdt: dec!(0),
                sells_usdt: dec!(8_000),
            },
            VolumeBin {
                buys_usdt: dec!(15_000),
                sells_usdt: dec!(0),
            },
            VolumeBin {
                buys_usdt: dec!(5_000),
                sells_usdt: dec!(12_000),
            },
            VolumeBin::default(),
        ];
        assert_snapshot!("volume_histogram__btc_three_buys_two_sells", summary(&bins));
    }
}
