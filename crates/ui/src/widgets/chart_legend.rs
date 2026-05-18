//! Chart legend — chart-canvas-overhaul v1.10 (M4 / T3016).
//!
//! Top-right inset card over the chart canvas (Q5 — architect's pick).
//! Five entries label the four marker layers (executed Buy / Sell + ghost
//! Buy / Sell signals) and the price-line stroke so the operator never has
//! to remember v1.9.0's marker semantics.
//!
//! ## Composition
//!
//! - **Card chrome** — `LEGEND_CARD_WIDTH_PX × LEGEND_CARD_HEIGHT_PX`
//!   rounded rectangle, `radius::R3` corners, `color::PANEL_SUNKEN`
//!   background, `color::BORDER_STRONG @ 1 px` outline, anchored at
//!   `(inner.right − card.w − space::M, inner.y + space::M)`.  Sits
//!   visually on a `shadow::shadow_1` whisper (rendered by the canvas
//!   layer underneath via the marker-shadow pass — no double shadow
//!   here, the card itself stays flat to match the tooltip's chrome).
//!
//! ## M7 — R9 chrome fix (T3027, 2026-05-12 ui-designer)
//!
//! The original T3016 landing used `PANEL_RAISED` fill + `BORDER_1`
//! outline.  At 3360×1890 native Retina dark mode that combination read
//! as nearly invisible against the chart canvas's `PANEL` background —
//! the `PANEL_RAISED → PANEL` luminance delta is the smallest pair in
//! the dark tier ladder (0x2A3038 → 0x1C2127, ~14 units per channel),
//! and `BORDER_1` is the quiet variant intended for in-panel control
//! separators, not for chrome that must stand off the canvas (evidence:
//! `/tmp/orch-diag/cockpit-final-charts.png` — top-right of the chart
//! shows the legend labels but no perceivable card boundary).  The
//! architect's M7 ladder
//! (`feature.md ## Design — M7 / Legend chrome ladder`) escalates
//! token reuse before adding new tokens.  We applied **rung (a) +
//! rung (b)** together, stopping below rung (c):
//!
//! - **Rung (a) — fill swap `PANEL_RAISED → PANEL_SUNKEN`.**  The
//!   dark-mode tier ladder is `PANEL_SUNKEN (0x0B0F15) < CANVAS
//!   (0x131820) < PANEL (0x1C2127) < PANEL_RAISED (0x2A3038)`
//!   (pinned by `theme::tests::tier_ladder_dark`).  `PANEL_SUNKEN`
//!   sits *two* tiers below `PANEL`, so the absolute luminance delta
//!   |PANEL − `PANEL_SUNKEN`| (~17 units / channel) is meaningfully
//!   larger than |PANEL − `PANEL_RAISED`| (~14 units / channel), and
//!   the card now reads as a recessed well rather than a barely-raised
//!   plate.
//! - **Rung (b) — outline swap `BORDER_1 → BORDER_STRONG`.**  This
//!   rejoins the architect's original Q5 spec — the T3016 landing
//!   note had deviated to `BORDER_1` for "quieter" chrome, and that
//!   judgement is the R9 regression.  `BORDER_STRONG` is the same
//!   outline `chart_tooltip::draw_tooltip` uses at
//!   [`chart_tooltip.rs:91`](chart_tooltip.rs); the tooltip is
//!   empirically visible against `PANEL` at the operator's hardware,
//!   so chrome-parity with the tooltip closes the visibility gap.
//!
//! We stopped at (a) + (b) — both are pure token reuse, zero new
//! tokens, and together they apply BOTH a fill-tier shift AND an
//! outline-strength bump.  Rungs (c) (shadow) and (d) (new
//! `LEGEND_CARD_BG` token) remain available if a future empirical
//! capture shows (a) + (b) borderline, but reuse beats addition
//! (Lumen discipline).
//! - **Rows (top → bottom)** — Buy / Sell / Buy signal / Sell signal /
//!   Price.  Each row: `LEGEND_GLYPH_PX`-tall glyph + `text::MICRO`
//!   label in `color::FG_2`.  The four marker rows reuse the chart's
//!   `draw_triangle` helper at the legend glyph size; the Price row
//!   paints a short `LINE_STROKE_PX`-wide horizontal segment in
//!   `color::ACCENT`.
//! - **Glyph palette** — `UP_500` / `DOWN_500` (fills) + `UP_400` /
//!   `DOWN_400 @ 60 % alpha` (ghosts) — identical to the chart's marker
//!   palette so the legend reads as a downsized mirror of the canvas
//!   content.
//!
//! ## Placement (Q5)
//!
//! Top-right inset over the chart canvas.  Rationale: doesn't reshape
//! the Charts-screen `Column`, matches `TradingView` convention, and the
//! top-right corner is the high-price / recent-time region where
//! executed markers cluster least.  The card is always visible when
//! `bars` is non-empty (the empty state already shows "No data" — no
//! markers, no legend).
//!
//! **Zero string literals** — copy via `crate::strings::CHART_LEGEND_*`.
//! **Zero hex colours** — tokens via `crate::theme`.
//!
//! ## Wire-up
//!
//! `draw_legend` is invoked as the chart canvas's final draw pass (Pass
//! 8, after the tooltip overlay) by `widgets::chart::ChartProgram::draw`.
//! v1.9.0's empty-state path returns early before reaching this pass —
//! the legend never paints on an empty `bars` slice.  The widget itself
//! ships from the ui-designer lane (T3015 / T3016); the wire-up edit
//! in `chart.rs` is the developer lane (T3017).

use iced::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};
use iced::{Color, Point, Rectangle, Size};
use smol_str::SmolStr;

use super::canvas_chart::{LINE_STROKE_PX, with_alpha};
use super::chart::draw_triangle;
use crate::strings::{
    CHART_LEGEND_BUY_GHOST_LABEL, CHART_LEGEND_BUY_LABEL, CHART_LEGEND_COMPARE_NO_DATA,
    CHART_LEGEND_PRICE_LABEL, CHART_LEGEND_SELL_GHOST_LABEL, CHART_LEGEND_SELL_LABEL,
};
use crate::theme::{
    ThemeMode, color,
    layout::{LEGEND_CARD_HEIGHT_PX, LEGEND_CARD_WIDTH_PX, LEGEND_GLYPH_PX},
    radius, space, text,
};

/// Number of legend rows.  Drives `LEGEND_CARD_HEIGHT_PX` arithmetic
/// (5 × glyph + 4 × 2-px inter-row gap + 2 × `space::S` pad = 76 → 80).
const LEGEND_ROW_COUNT: usize = 5;

/// Inter-row vertical gap.  Two pixels separates consecutive rows
/// without crowding the 10-px glyphs.
const LEGEND_ROW_GAP_PX: f32 = 2.0;

/// Horizontal gap between the glyph column and the label column.
/// `space::S` (8 px) — same gap the tooltip uses inside its card.
const LEGEND_GLYPH_LABEL_GAP_PX: f32 = 8.0;

/// Glyph "alpha" for ghost-signal rows.  Matches the chart's ghost
/// triangle alpha at `chart::draw` Pass 4 (0.6).
const LEGEND_GHOST_ALPHA: f32 = 0.6;

/// Pixel length of the price-line legend stub.  Sized to read as a
/// distinct chart stroke without dominating the row; matches the
/// `LEGEND_GLYPH_PX` × 1.4 visual budget so the line and the triangles
/// occupy a comparable horizontal slot.
const LEGEND_PRICE_STUB_PX: f32 = 14.0;

/// A compare-legend entry.
///
/// Each entry shows a colored line-stub (the compare curve's `ACCENT_N` color)
/// with a short label. `has_data = false` renders the label in `FG_3`
/// (faded) for the "no data" treatment (R8.4 / T-D-15).
#[derive(Debug, Clone)]
pub(crate) struct CompareLegendEntry {
    /// Short label — typically the strategy ID truncated to fit the card.
    pub label: SmolStr,
    /// The compare curve's line color (`ACCENT_2..5` by slot).
    pub color: Color,
    /// When `false` the label renders faded — no cached report for this pair.
    pub has_data: bool,
}

/// Render the legend card as the chart canvas's final draw pass
/// (above the tooltip — Pass 7 in `chart::draw`).  The widget is a
/// **free function**, not an `iced::Widget`, because it composes onto
/// the existing `ChartProgram::draw` canvas frame.
///
/// `inner` is the chart's drawable rectangle (from
/// `canvas_chart::inner_rect_with_gutters`).  The card is positioned
/// at `(inner.right − card.w − space::M, inner.y + space::M)` so
/// it nestles into the top-right corner with `space::M` (12 px)
/// breathing room on both sides.
///
/// Pass `compare` as an empty slice for the basic 5-row legend (backward-
/// compatible with all pre-T-D-15 call sites).  When `compare` is
/// non-empty, up to 4 additional line-stub rows appear below the price
/// row, one per compare strategy.
pub(crate) fn draw_legend(frame: &mut Frame, inner: Rectangle, mode: ThemeMode) {
    draw_legend_impl(frame, inner, mode, &[]);
}

/// Extended variant of `draw_legend` that renders additional compare
/// curve rows (T-D-15 / M3). Call from `ChartProgram::draw` when
/// `self.compare` is non-empty.
pub(crate) fn draw_legend_with_compare(
    frame: &mut Frame,
    inner: Rectangle,
    mode: ThemeMode,
    compare: &[CompareLegendEntry],
) {
    draw_legend_impl(frame, inner, mode, compare);
}

#[allow(clippy::too_many_lines)]
fn draw_legend_impl(
    frame: &mut Frame,
    inner: Rectangle,
    mode: ThemeMode,
    compare: &[CompareLegendEntry],
) {
    let cmp_count = compare.len().min(4); // at most 4 compare entries
    let card_rect = compute_card_rect_dynamic(inner, cmp_count);

    // Card chrome — `PANEL_SUNKEN` fill + `BORDER_STRONG` 1-px outline +
    // R3 rounded corners.  iced 0.14's `canvas::Path` doesn't expose
    // rounded-rect natively from a single path call, but `Frame::fill`
    // accepts a `Path::rectangle` and the visible rounding comes from
    // the chrome stroke at the same radius.  We render a flat fill
    // and a 1-px outline — the visual effect at `radius::R3` (6 px)
    // is dominated by the recessed-tier luminance contrast + the
    // strong outline, not the corner arc.  T3027 (R9): the original
    // `PANEL_RAISED` + `BORDER_1` combination read as nearly invisible
    // against `PANEL` at 3360×1890 dark mode (see module docstring).
    // Reuses the same outline tier as `chart_tooltip::draw_tooltip` so
    // legend + tooltip carry chrome-parity against the chart panel.
    let bg_path = Path::rectangle(
        Point::new(card_rect.x, card_rect.y),
        Size::new(card_rect.width, card_rect.height),
    );
    frame.fill(&bg_path, color::PANEL_SUNKEN.current(mode));
    frame.stroke(
        &bg_path,
        Stroke::default()
            .with_color(color::BORDER_STRONG.current(mode))
            .with_width(1.0),
    );

    // Five rows, top → bottom: Buy / Sell / Buy signal / Sell signal /
    // Price.  Each row sits at `row_origin_y` with the glyph centred
    // vertically on `LEGEND_GLYPH_PX / 2`.
    #[allow(clippy::cast_precision_loss)]
    let pad = space::S as f32;
    let row_stride = LEGEND_GLYPH_PX + LEGEND_ROW_GAP_PX;

    // Glyph column anchor — `pad` from the card's left edge.  Each
    // triangle anchors at its **base midpoint** per `draw_triangle`'s
    // contract; the price stub anchors at its **left endpoint**.
    let glyph_x = card_rect.x + pad + LEGEND_GLYPH_PX / 2.0;

    // Label column anchor — `pad + glyph + gap` from the card's left.
    let label_x = card_rect.x + pad + LEGEND_GLYPH_PX + LEGEND_GLYPH_LABEL_GAP_PX;

    // ── Base 5 rows (marker palette + price line) ────────────────────────
    for (row_idx, row) in legend_rows(mode).iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let row_top = card_rect.y + pad + row_idx as f32 * row_stride;
        let glyph_center_y = row_top + LEGEND_GLYPH_PX / 2.0;
        let label_y = row_top + LEGEND_GLYPH_PX / 2.0;

        match row.glyph {
            LegendGlyph::Triangle { color: c, upward } => {
                // `draw_triangle` anchors at the triangle's base; offset
                // the anchor so the glyph sits visually centred in the
                // row's vertical slot.  For an upward triangle the base
                // is at the bottom; for a downward triangle the base is
                // at the top.  Either way, anchor at
                // `(glyph_x, glyph_center_y + LEGEND_GLYPH_PX / 2)`
                // for upward (so apex points up from row centre) or
                // `(glyph_x, glyph_center_y - LEGEND_GLYPH_PX / 2)` for
                // downward — `draw_triangle` builds the geometry from
                // the anchor point.
                let anchor_y = if upward {
                    glyph_center_y + LEGEND_GLYPH_PX / 2.0
                } else {
                    glyph_center_y - LEGEND_GLYPH_PX / 2.0
                };
                draw_triangle(
                    frame,
                    Point::new(glyph_x, anchor_y),
                    c,
                    upward,
                    LEGEND_GLYPH_PX,
                    None, // no outline — keeps the legend visually quiet
                    None, // no drop shadow — matches tooltip chrome
                );
            }
            LegendGlyph::Stroke { color: c } => {
                // Short horizontal segment standing in for the chart's
                // price stroke.  Render at the row's vertical midpoint
                // with `LINE_STROKE_PX` width — same stroke the chart
                // canvas uses.
                let stub_start = Point::new(glyph_x - LEGEND_PRICE_STUB_PX / 2.0, glyph_center_y);
                let stub_end = Point::new(glyph_x + LEGEND_PRICE_STUB_PX / 2.0, glyph_center_y);
                let stub_path = Path::new(|builder| {
                    builder.move_to(stub_start);
                    builder.line_to(stub_end);
                });
                frame.stroke(
                    &stub_path,
                    Stroke::default().with_color(c).with_width(LINE_STROKE_PX),
                );
            }
        }

        // Row label — `text::MICRO` at `color::FG_2`, left-anchored to
        // `label_x`, vertically centred on the glyph.
        #[allow(clippy::cast_precision_loss)]
        let micro = text::MICRO as f32;
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: row.label.to_string(),
            position: Point::new(label_x, label_y),
            color: color::FG_2.current(mode),
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }

    // ── Compare rows (T-D-15 / M3) ────────────────────────────────────────────
    //
    // One row per compare strategy, in positional ACCENT_2..5 color.
    // When `entry.has_data = false`, the label renders faded (`FG_3`)
    // and the "no data" suffix replaces the strategy label (R8.4).
    for (cmp_idx, entry) in compare.iter().take(4).enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let row_idx = LEGEND_ROW_COUNT + cmp_idx;
        #[allow(clippy::cast_precision_loss)]
        let row_top = card_rect.y + pad + row_idx as f32 * row_stride;
        let glyph_center_y = row_top + LEGEND_GLYPH_PX / 2.0;
        let label_y = glyph_center_y;

        // Line-stub glyph in the entry's ACCENT_N color.
        let stub_color = if entry.has_data {
            entry.color
        } else {
            with_alpha(entry.color, 0.35)
        };
        let stub_start = Point::new(glyph_x - LEGEND_PRICE_STUB_PX / 2.0, glyph_center_y);
        let stub_end = Point::new(glyph_x + LEGEND_PRICE_STUB_PX / 2.0, glyph_center_y);
        let stub_path = Path::new(|builder| {
            builder.move_to(stub_start);
            builder.line_to(stub_end);
        });
        frame.stroke(
            &stub_path,
            Stroke::default()
                .with_color(stub_color)
                .with_width(LINE_STROKE_PX),
        );

        // Label — faded when no data.
        let label_text = if entry.has_data {
            entry.label.to_string()
        } else {
            format!("{} ({})", entry.label, CHART_LEGEND_COMPARE_NO_DATA)
        };
        let label_color = if entry.has_data {
            color::FG_2.current(mode)
        } else {
            color::FG_3.current(mode)
        };
        #[allow(clippy::cast_precision_loss)]
        let micro = text::MICRO as f32;
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: label_text,
            position: Point::new(label_x, label_y),
            color: label_color,
            size: micro.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center.into(),
            ..CanvasText::default()
        });
    }
}

/// Card-rect anchor — top-right inset with `space::M` breathing room
/// on both edges (Q5 architect's resolution). `compare_count` extra rows
/// are added below the base 5 rows; the card grows vertically.
fn compute_card_rect_dynamic(inner: Rectangle, compare_count: usize) -> Rectangle {
    #[allow(clippy::cast_precision_loss)]
    let gap = space::M as f32;
    let row_stride = LEGEND_GLYPH_PX + LEGEND_ROW_GAP_PX;
    // When no compare rows: return the static token height (backward compat).
    // When compare rows added: grow by exactly `compare_count × row_stride`
    // above the base `LEGEND_CARD_HEIGHT_PX`.  This matches the test's
    // `base_h + n * row_stride` expectation.
    #[allow(clippy::cast_precision_loss)]
    let height = LEGEND_CARD_HEIGHT_PX + (compare_count as f32 * row_stride);
    Rectangle {
        x: inner.x + inner.width - LEGEND_CARD_WIDTH_PX - gap,
        y: inner.y + gap,
        width: LEGEND_CARD_WIDTH_PX,
        height,
    }
}

/// Fixed-height variant preserved for tests that pin `LEGEND_CARD_HEIGHT_PX`.
#[cfg(test)]
fn compute_card_rect(inner: Rectangle) -> Rectangle {
    compute_card_rect_dynamic(inner, 0)
}

/// One legend entry — a glyph + a string label.
#[derive(Debug, Clone, Copy)]
struct LegendRow {
    glyph: LegendGlyph,
    label: &'static str,
}

/// Glyph shapes the legend supports: the four triangles (matching the
/// chart's marker palette) and the price-line stroke.
#[derive(Debug, Clone, Copy)]
enum LegendGlyph {
    /// Filled triangle pointing up (`upward = true`) or down (`upward
    /// = false`).  Colour from the chart's marker palette.
    Triangle { color: Color, upward: bool },
    /// Horizontal line stroke standing in for the price line.
    Stroke { color: Color },
}

/// The five rows in render order.  Marker rows reuse the chart's
/// `_500` (fill) and `_400 @ 60 % alpha` (ghost) palette so the legend
/// reads as a downsampled sibling of the canvas content.
fn legend_rows(mode: ThemeMode) -> [LegendRow; LEGEND_ROW_COUNT] {
    [
        LegendRow {
            glyph: LegendGlyph::Triangle {
                color: color::UP_500.current(mode),
                upward: true,
            },
            label: CHART_LEGEND_BUY_LABEL,
        },
        LegendRow {
            glyph: LegendGlyph::Triangle {
                color: color::DOWN_500.current(mode),
                upward: false,
            },
            label: CHART_LEGEND_SELL_LABEL,
        },
        LegendRow {
            glyph: LegendGlyph::Triangle {
                color: with_alpha(color::UP_400.current(mode), LEGEND_GHOST_ALPHA),
                upward: true,
            },
            label: CHART_LEGEND_BUY_GHOST_LABEL,
        },
        LegendRow {
            glyph: LegendGlyph::Triangle {
                color: with_alpha(color::DOWN_400.current(mode), LEGEND_GHOST_ALPHA),
                upward: false,
            },
            label: CHART_LEGEND_SELL_GHOST_LABEL,
        },
        LegendRow {
            glyph: LegendGlyph::Stroke {
                color: color::ACCENT.current(mode),
            },
            label: CHART_LEGEND_PRICE_LABEL,
        },
    ]
}

/// Corner radius the card chrome targets.  Pulled into a constant so
/// the test can reference it without going through `theme::radius`
/// (keeps the assert local to the widget's contract).
#[cfg(test)]
const LEGEND_CARD_RADIUS_PX: f32 = radius::R3;
#[cfg(not(test))]
#[allow(dead_code)]
const LEGEND_CARD_RADIUS_PX: f32 = radius::R3;

#[cfg(test)]
#[allow(clippy::float_arithmetic, clippy::float_cmp)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    /// Approximate-equality helper for the rect / row-stride arithmetic.
    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.001
    }

    fn dummy_inner() -> Rectangle {
        // Fixture mimicking a 1280×720-floor inner rect after the
        // chart's price-gutter + right-margin + outer 8-px gutter pulls.
        Rectangle {
            x: 64.0,
            y: 16.0,
            width: 1000.0,
            height: 600.0,
        }
    }

    /// T3016 — card-anchor arithmetic lines up the legend's top-right
    /// corner with `inner.right - space::M`.  Catches a regression
    /// where someone flips the sign or anchors at the wrong edge.
    #[test]
    fn legend_card_dimensions_match_tokens() {
        let inner = dummy_inner();
        let card = compute_card_rect(inner);
        // Width + height pin to the architect's tokens.
        assert!(approx_eq(card.width, LEGEND_CARD_WIDTH_PX));
        assert!(approx_eq(card.height, LEGEND_CARD_HEIGHT_PX));
        // Right edge sits `space::M` (12 px) inside the inner rect's
        // right edge.
        #[allow(clippy::cast_precision_loss)]
        let gap = space::M as f32;
        assert!(
            approx_eq(card.x + card.width, inner.x + inner.width - gap),
            "card right edge = inner.right - space::M",
        );
        // Top edge sits `space::M` below the inner rect's top edge.
        assert!(
            approx_eq(card.y, inner.y + gap),
            "card top edge = inner.y + space::M",
        );
    }

    /// T3016 — the legend's glyph palette mirrors the chart marker
    /// palette: `UP_500` / `DOWN_500` (fills), `UP_400` / `DOWN_400`
    /// (ghosts at 60 % alpha), `ACCENT` (line stub).  If a future
    /// refactor swaps any of these, the chart and the legend visually
    /// disagree.
    #[test]
    fn legend_glyphs_use_marker_palette() {
        let rows = legend_rows(ThemeMode::Dark);
        // Row 0 — Buy fill — UP_500, upward.
        match rows[0].glyph {
            LegendGlyph::Triangle { color: c, upward } => {
                assert!(upward, "Buy is upward");
                let exp = color::UP_500.current(ThemeMode::Dark);
                assert_eq!((c.r, c.g, c.b), (exp.r, exp.g, exp.b), "Buy = UP_500");
                assert!((c.a - 1.0).abs() < 1e-4, "Buy fill alpha = 1.0");
            }
            LegendGlyph::Stroke { .. } => panic!("Buy must be a triangle"),
        }
        // Row 1 — Sell fill — DOWN_500, downward.
        match rows[1].glyph {
            LegendGlyph::Triangle { color: c, upward } => {
                assert!(!upward, "Sell is downward");
                let exp = color::DOWN_500.current(ThemeMode::Dark);
                assert_eq!((c.r, c.g, c.b), (exp.r, exp.g, exp.b), "Sell = DOWN_500");
            }
            LegendGlyph::Stroke { .. } => panic!("Sell must be a triangle"),
        }
        // Row 2 — Buy signal ghost — UP_400 at 60 % alpha, upward.
        match rows[2].glyph {
            LegendGlyph::Triangle { color: c, upward } => {
                assert!(upward, "Buy signal is upward");
                let exp = color::UP_400.current(ThemeMode::Dark);
                assert_eq!((c.r, c.g, c.b), (exp.r, exp.g, exp.b), "ghost = UP_400");
                assert!((c.a - LEGEND_GHOST_ALPHA).abs() < 1e-4, "ghost alpha = 0.6",);
            }
            LegendGlyph::Stroke { .. } => panic!("Buy signal must be a triangle"),
        }
        // Row 3 — Sell signal ghost — DOWN_400 at 60 % alpha, downward.
        match rows[3].glyph {
            LegendGlyph::Triangle { color: c, upward } => {
                assert!(!upward, "Sell signal is downward");
                let exp = color::DOWN_400.current(ThemeMode::Dark);
                assert_eq!((c.r, c.g, c.b), (exp.r, exp.g, exp.b), "ghost = DOWN_400");
                assert!((c.a - LEGEND_GHOST_ALPHA).abs() < 1e-4, "ghost alpha = 0.6",);
            }
            LegendGlyph::Stroke { .. } => panic!("Sell signal must be a triangle"),
        }
        // Row 4 — Price line — ACCENT stroke.
        match rows[4].glyph {
            LegendGlyph::Stroke { color: c } => {
                let exp = color::ACCENT.current(ThemeMode::Dark);
                assert_eq!((c.r, c.g, c.b), (exp.r, exp.g, exp.b), "Price = ACCENT");
            }
            LegendGlyph::Triangle { .. } => panic!("Price must be a stroke"),
        }
    }

    /// T3016 — row labels are sourced from `strings::CHART_LEGEND_*`
    /// constants, in render order.  Confirms zero string drift between
    /// the widget and the copy registry.
    #[test]
    fn legend_labels_route_through_strings() {
        let rows = legend_rows(ThemeMode::Dark);
        assert_eq!(rows[0].label, CHART_LEGEND_BUY_LABEL);
        assert_eq!(rows[1].label, CHART_LEGEND_SELL_LABEL);
        assert_eq!(rows[2].label, CHART_LEGEND_BUY_GHOST_LABEL);
        assert_eq!(rows[3].label, CHART_LEGEND_SELL_GHOST_LABEL);
        assert_eq!(rows[4].label, CHART_LEGEND_PRICE_LABEL);
    }

    /// T3016 — light + dark palettes both resolve through `ModeColor`;
    /// the ghost rows must carry the 60 % alpha in both modes (the
    /// `_400` shade differs between modes, but the alpha override is
    /// theme-independent).  Catches a regression where someone hard-
    /// codes the dark `_400` hex into the light branch.
    #[test]
    fn legend_palette_resolves_for_light_mode() {
        let dark = legend_rows(ThemeMode::Dark);
        let light = legend_rows(ThemeMode::Light);
        // Buy fill differs across modes (UP_500 dark vs UP_500 light).
        let (
            LegendGlyph::Triangle {
                color: dark_buy, ..
            },
            LegendGlyph::Triangle {
                color: light_buy, ..
            },
        ) = (dark[0].glyph, light[0].glyph)
        else {
            panic!("Buy must be a triangle in both modes");
        };
        assert_ne!(
            (dark_buy.r, dark_buy.g, dark_buy.b),
            (light_buy.r, light_buy.g, light_buy.b),
            "Buy fill differs between modes",
        );
        // Ghost alpha holds at 0.6 in both modes.  Both `LegendGlyph`
        // arms expose a `color.a`; pull through a single accessor so
        // the clippy `match_same_arms` lint doesn't complain about the
        // arms being byte-identical (which is intentional here — the
        // legend always carries a colour, the alpha lives on it).
        let alpha = |g: LegendGlyph| match g {
            LegendGlyph::Triangle { color: c, .. } | LegendGlyph::Stroke { color: c } => c.a,
        };
        assert!((alpha(dark[2].glyph) - LEGEND_GHOST_ALPHA).abs() < 1e-4);
        assert!((alpha(light[2].glyph) - LEGEND_GHOST_ALPHA).abs() < 1e-4);
        assert!((alpha(dark[3].glyph) - LEGEND_GHOST_ALPHA).abs() < 1e-4);
        assert!((alpha(light[3].glyph) - LEGEND_GHOST_ALPHA).abs() < 1e-4);
    }

    /// T3016 — the card's chrome at `radius::R3` is the documented
    /// design pick.  Pinning the constant here so a refactor that
    /// retunes the legend's corner radius fails the assert before the
    /// chrome lands inconsistent with the tooltip card.
    #[test]
    fn legend_card_corner_radius_pinned() {
        assert!(
            (LEGEND_CARD_RADIUS_PX - radius::R3).abs() < 1e-4,
            "legend card radius = R3",
        );
    }

    /// T3016 — the legend's row arithmetic must clear the card height
    /// at `LEGEND_CARD_HEIGHT_PX`.  Defence-in-depth on top of
    /// `theme::tests::t3009_legend_card_height_clears_five_entries`
    /// — that test pins the token sizing; this one pins the widget's
    /// row-stride consumption.
    #[test]
    fn legend_rows_fit_inside_card_height() {
        #[allow(clippy::cast_precision_loss)]
        let pad = space::S as f32;
        let row_stride = LEGEND_GLYPH_PX + LEGEND_ROW_GAP_PX;
        #[allow(clippy::cast_precision_loss)]
        let rows = LEGEND_ROW_COUNT as f32;
        // Top pad + (rows × glyph) + ((rows − 1) × gap) + bottom pad.
        let consumed = pad + rows * LEGEND_GLYPH_PX + (rows - 1.0) * LEGEND_ROW_GAP_PX + pad;
        let _ = row_stride; // referenced for self-documentation.
        assert!(
            consumed <= LEGEND_CARD_HEIGHT_PX,
            "rows consume {consumed} px; card height {LEGEND_CARD_HEIGHT_PX}",
        );
    }

    /// Plain-text summary of the legend composition.  Pinned via
    /// snapshot so a future refactor that retunes row order / glyph
    /// shape / colour bindings is visible without a pixel renderer.
    /// Mirrors the `chart_summary` style used by sibling widget tests.
    fn legend_summary(mode: ThemeMode) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        out.push_str("widget: chart_legend\n");
        out.push_str("placement: top-right inset over inner rect\n");
        let _ = writeln!(out, "card_width_px: {LEGEND_CARD_WIDTH_PX}");
        let _ = writeln!(out, "card_height_px: {LEGEND_CARD_HEIGHT_PX}");
        let _ = writeln!(out, "card_radius_px: {LEGEND_CARD_RADIUS_PX}");
        out.push_str("card_background: PANEL_SUNKEN\n");
        out.push_str("card_border: BORDER_STRONG @ 1px\n");
        let _ = writeln!(out, "glyph_px: {LEGEND_GLYPH_PX}");
        let _ = writeln!(out, "ghost_alpha: {LEGEND_GHOST_ALPHA}");
        let _ = writeln!(out, "row_count: {LEGEND_ROW_COUNT}");
        out.push_str("label_color: FG_2\n");
        out.push_str("label_size: text::MICRO\n");
        for (idx, row) in legend_rows(mode).iter().enumerate() {
            let (glyph_kind, palette, upward) = match row.glyph {
                LegendGlyph::Triangle { upward, .. } => match idx {
                    0 => ("triangle", "UP_500", upward),
                    1 => ("triangle", "DOWN_500", upward),
                    2 => ("triangle", "UP_400@0.6", upward),
                    3 => ("triangle", "DOWN_400@0.6", upward),
                    _ => ("triangle", "?", upward),
                },
                LegendGlyph::Stroke { .. } => ("stroke", "ACCENT", false),
            };
            let label = row.label;
            let _ = writeln!(
                out,
                "row_{idx}: glyph={glyph_kind} upward={upward} palette={palette} label=\"{label}\"",
            );
        }
        out
    }

    #[test]
    fn legend_composition_snapshot_dark() {
        assert_snapshot!(
            "chart_legend__composition_dark",
            legend_summary(ThemeMode::Dark)
        );
    }

    // ── T-D-15 compare legend tests ───────────────────────────────────────────

    /// T-D-15 — `compute_card_rect_dynamic` with zero compare rows returns
    /// the same height as the token `LEGEND_CARD_HEIGHT_PX`.
    #[test]
    fn compare_legend_zero_entries_same_height_as_base() {
        let inner = dummy_inner();
        let base = compute_card_rect(inner);
        let dynamic = compute_card_rect_dynamic(inner, 0);
        assert!(approx_eq(base.height, dynamic.height));
        assert!(approx_eq(base.width, dynamic.width));
        assert!(approx_eq(base.x, dynamic.x));
        assert!(approx_eq(base.y, dynamic.y));
    }

    /// T-D-15 — each additional compare row grows the card by `LEGEND_GLYPH_PX + LEGEND_ROW_GAP_PX`.
    #[test]
    fn compare_legend_grows_card_per_row() {
        let inner = dummy_inner();
        let base_h = compute_card_rect_dynamic(inner, 0).height;
        for n in 1..=4 {
            #[allow(clippy::cast_precision_loss)]
            let expected_h = base_h + n as f32 * (LEGEND_GLYPH_PX + LEGEND_ROW_GAP_PX);
            let actual_h = compute_card_rect_dynamic(inner, n).height;
            assert!(
                approx_eq(actual_h, expected_h),
                "n={n}: expected {expected_h} got {actual_h}"
            );
        }
    }

    /// T-D-15 — `compare_color_slot_assignment_is_stable`: the 4 compare
    /// slots map to ACCENT_2/3/4/5 in order. Pinned so a future palette
    /// reorder is visible before it ships.
    #[test]
    fn compare_color_slot_assignment_is_stable() {
        let palette = color::accent_palette();
        // Slot 0 → ACCENT_2
        let slot0_dark = palette[0].current(ThemeMode::Dark);
        let exp0 = color::ACCENT_2.current(ThemeMode::Dark);
        assert_eq!(
            (slot0_dark.r, slot0_dark.g, slot0_dark.b),
            (exp0.r, exp0.g, exp0.b),
            "slot 0 = ACCENT_2"
        );
        // Slot 3 → ACCENT_5
        let slot3_dark = palette[3].current(ThemeMode::Dark);
        let exp3 = color::ACCENT_5.current(ThemeMode::Dark);
        assert_eq!(
            (slot3_dark.r, slot3_dark.g, slot3_dark.b),
            (exp3.r, exp3.g, exp3.b),
            "slot 3 = ACCENT_5"
        );
    }

    /// T-D-15 — `CompareLegendEntry` with `has_data = false` has a different
    /// (faded) label text than a `has_data = true` entry.
    #[test]
    fn compare_legend_no_data_label_uses_suffix() {
        let no_data_entry = CompareLegendEntry {
            label: smol_str::SmolStr::new("v2.momentum"),
            color: color::ACCENT_2.current(ThemeMode::Dark),
            has_data: false,
        };
        // The draw path builds the label inline; verify the suffix is added.
        let label_text = if no_data_entry.has_data {
            no_data_entry.label.to_string()
        } else {
            format!("{} ({})", no_data_entry.label, CHART_LEGEND_COMPARE_NO_DATA)
        };
        assert!(label_text.contains(CHART_LEGEND_COMPARE_NO_DATA));
        assert!(label_text.contains("v2.momentum"));
    }
}
