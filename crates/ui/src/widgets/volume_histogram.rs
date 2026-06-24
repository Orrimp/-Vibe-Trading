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
use iced::widget::{Canvas, Container, container};
use iced::{Length, Point, Rectangle, Renderer, Size, mouse};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

#[cfg(test)]
use super::canvas_chart::GRIDLINE_COUNT;
use super::canvas_chart::with_alpha;
// lab-buy-sell-overlay-align — reuse the chart's own per-bar x mapping AND its
// equity-gutter width so the histogram's plot area stays the chart's single
// source of truth (no drift if the chart retunes either).
use super::chart::{AXIS_GUTTER_EQUITY_PX, x_for_index};
use crate::state::Message;
use crate::strings::KPI_DASH_PLACEHOLDER;
use crate::theme::layout::{AXIS_GUTTER_PRICE_PX, AXIS_GUTTER_RIGHT_PX};
use crate::theme::{ThemeMode, color, space, text};

/// How the histogram's x-axis relates to a sibling price chart.
///
/// `ChartAligned` shares the chart's HORIZONTAL plot geometry so each
/// per-bar volume bar sits directly beneath its buy/sell triangle marker
/// (the Lab composition — [`view_aligned`]). `Standalone` keeps the
/// symmetric `space::S` gutter the widget shipped with, for an isolated
/// showcase cell with no chart above it (the gallery — [`view`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment {
    /// Symmetric gutter — no sibling chart to line up with.
    Standalone,
    /// Match the sibling chart's plot area; `true` ⇒ the chart reserved
    /// its wide right equity gutter (an equity overlay was present).
    ChartAligned { equity_axis: bool },
}

/// Build the histogram's drawable plot rectangle.
///
/// ## `Alignment::ChartAligned` — why (lab-buy-sell-overlay-align, 2026-06-24)
///
/// The operator reported the per-bar buy/sell volume bars "are not over
/// the triangles" — they sat at the bottom strip but did **not** line up
/// vertically with the chart's buy/sell triangle markers above them.
/// Root cause, confirmed at the pixel layer: the chart paints into
/// `inner_rect_with_gutters(size, AXIS_GUTTER_PRICE_PX, …)` (left origin
/// `space::S + 48`, a right gutter, etc.) while this histogram painted
/// into the symmetric `canvas_chart::inner_rect(size)` (left origin
/// `space::S = 8`). So bar `i`'s triangle and bar `i`'s volume bar landed
/// at different `x` — the leftmost fill was off by ~27 px in the 1280-px
/// render. Sharing the chart's **horizontal** gutters (and the chart's
/// `x_for_index` per-bar mapping) puts each volume bar directly beneath
/// its triangle.
///
/// Vertically the histogram keeps its own full strip (only the base
/// `space::S` gutter top + bottom) — it has no price/time axis of its
/// own; the centreline + up/down stacks own the whole height.
fn histogram_plot_rect(size: Size, align: Alignment) -> Rectangle {
    #[allow(clippy::cast_precision_loss)]
    let base = space::S as f32;
    let (left, right) = match align {
        Alignment::Standalone => (0.0, 0.0),
        Alignment::ChartAligned { equity_axis } => (
            AXIS_GUTTER_PRICE_PX,
            if equity_axis {
                AXIS_GUTTER_EQUITY_PX
            } else {
                AXIS_GUTTER_RIGHT_PX
            },
        ),
    };
    let x = base + left;
    let width = (size.width - 2.0 * base - left - right).max(0.0);
    Rectangle {
        x,
        y: base,
        width,
        height: (size.height - 2.0 * base).max(0.0),
    }
}

/// One histogram bin — paired buy + sell USDT-notional totals for a bar.
/// Local to this widget; built at compose time in
/// [`crate::screens::lab`] from `model.chart_markers` aggregated per
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

/// Render the per-bar histogram **standalone** (symmetric gutter, no
/// sibling chart to align with — the gallery showcase cell).
///
/// Takes `bins` by value because the canvas `Program` impl holds them
/// across iced's render lifetime; ~60 bins × ~16 B per bin = ~1 KB per
/// repaint — trivially in budget.
#[must_use]
pub fn view<'a>(bins: Vec<VolumeBin>, mode: ThemeMode) -> crate::Element<'a> {
    render(bins, Alignment::Standalone, mode)
}

/// Render the per-bar histogram **aligned to the sibling price chart**
/// (the Lab composition).  Each per-bar volume bar is placed at the same
/// `x` the chart maps that bar to, so it sits directly beneath its
/// buy/sell triangle marker (lab-buy-sell-overlay-align).
///
/// `equity_axis` must mirror the chart: pass `true` when the chart
/// reserved its wide right equity gutter (an equity overlay was present),
/// `false` for the plain price+markers shape.
#[must_use]
pub fn view_aligned<'a>(
    bins: Vec<VolumeBin>,
    equity_axis: bool,
    mode: ThemeMode,
) -> crate::Element<'a> {
    render(bins, Alignment::ChartAligned { equity_axis }, mode)
}

#[allow(clippy::needless_pass_by_value)]
fn render<'a>(bins: Vec<VolumeBin>, align: Alignment, mode: ThemeMode) -> crate::Element<'a> {
    let program = HistogramProgram { bins, align, mode };
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
    /// How the x-axis relates to a sibling price chart — drives
    /// [`histogram_plot_rect`] so the Lab plot areas stay aligned.
    align: Alignment,
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
        // cockpit-chart-cache Phase 1 — time the geometry-build body
        // (no-op when `chart-build-probe` is off).
        let _build_timer = super::chart_build_probe::BuildTimer::start();
        let mut frame = Frame::new(renderer, bounds.size());
        // lab-buy-sell-overlay-align — share the sibling chart's HORIZONTAL
        // plot geometry so each bar's buy/sell volume bar lines up under its
        // triangle marker (was `canvas_chart::inner_rect`, a symmetric gutter
        // that did NOT match the chart's price-axis gutter).
        let inner = histogram_plot_rect(bounds.size(), self.align);
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
        let bin_count = self.bins.len();

        for (i, bin) in self.bins.iter().enumerate() {
            // x-centre per bin:
            //  - ChartAligned (Lab): use the SAME mapping the chart paints bar
            //    `i` with (`chart::x_for_index`, endpoints AT the plot-rect
            //    edges). `bins` and the chart's `bars` are 1:1 by construction
            //    (`lab::compute_volume_bins`), so this puts the volume bar
            //    directly under its buy/sell triangle marker
            //    (lab-buy-sell-overlay-align, 2026-06-24).
            //  - Standalone (gallery): keep the original `(i + 0.5)/n` centred
            //    slot so the isolated showcase cell stays evenly distributed.
            #[allow(clippy::cast_precision_loss)]
            let x_centre = match self.align {
                Alignment::ChartAligned { .. } => x_for_index(i, bin_count, inner),
                Alignment::Standalone => inner.x + (i as f32 + 0.5) * slot,
            };
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

    /// lab-buy-sell-overlay-align — the CHART-ALIGNED histogram plot rect must
    /// share the sibling price chart's HORIZONTAL plot geometry (x + width) so
    /// each volume bar sits under its triangle marker. Pin the equivalence to
    /// `chart::chart_inner_rect` / `chart_inner_rect_with_equity` directly (no
    /// renderer) — the load-bearing invariant behind the pixel test
    /// `tests/lab_buy_sell_overlay_render.rs`. Vertically the histogram keeps
    /// its own strip, so only x + width are pinned.
    #[test]
    #[allow(clippy::float_cmp)]
    fn chart_aligned_rect_matches_chart_inner_rect_horizontally() {
        use super::super::chart::{chart_inner_rect, chart_inner_rect_with_equity};
        for size in [
            Size::new(1280.0, 80.0),
            Size::new(1280.0, 720.0),
            Size::new(1920.0, 80.0),
            Size::new(3360.0, 80.0),
        ] {
            // No-equity chart shape ⇒ histogram with equity_axis = false.
            let chart_no_eq = chart_inner_rect(size);
            let his_no_eq =
                histogram_plot_rect(size, Alignment::ChartAligned { equity_axis: false });
            assert_eq!(
                (his_no_eq.x, his_no_eq.width),
                (chart_no_eq.x, chart_no_eq.width),
                "no-equity: histogram x+width must equal chart inner x+width at {size:?}"
            );

            // Equity overlay chart shape ⇒ histogram with equity_axis = true.
            let chart_eq = chart_inner_rect_with_equity(size);
            let his_eq = histogram_plot_rect(size, Alignment::ChartAligned { equity_axis: true });
            assert_eq!(
                (his_eq.x, his_eq.width),
                (chart_eq.x, chart_eq.width),
                "equity: histogram x+width must equal chart equity-inner x+width at {size:?}"
            );
        }
    }

    /// The STANDALONE alignment is unchanged from the widget's original shape
    /// (`canvas_chart::inner_rect`) — the gallery showcase cell must stay
    /// byte-identical. Pins x/y/width/height equality so the chart-alignment
    /// rework never silently shifts the isolated cell.
    #[test]
    #[allow(clippy::float_cmp)]
    fn standalone_rect_matches_base_inner_rect() {
        use super::super::canvas_chart::inner_rect;
        for size in [
            Size::new(1280.0, 80.0),
            Size::new(800.0, 200.0),
            Size::new(3360.0, 80.0),
        ] {
            let base = inner_rect(size);
            let standalone = histogram_plot_rect(size, Alignment::Standalone);
            assert_eq!(
                (base.x, base.y, base.width, base.height),
                (
                    standalone.x,
                    standalone.y,
                    standalone.width,
                    standalone.height
                ),
                "standalone plot rect must equal canvas_chart::inner_rect at {size:?}"
            );
        }
    }
}
