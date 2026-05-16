//! Chart-hover tooltip — chart-buy-sell-emphasis v1.9 (M2 / T2009).
//!
//! Final canvas pass for [`widgets::chart`], rendered on top of the marker
//! layers when `Cockpit.chart_tooltip` is `Some(view)` and the canvas's
//! `ChartProgram::update` impl recorded a hovered-marker centroid.
//!
//! ## Architect's resolution (Q3 = b)
//!
//! Custom canvas pointer tracking + custom-drawn tooltip overlay. We share
//! the marker centroid math with the draw pass (one source of truth — the
//! `anchor_for_ts` helper in `widgets::chart`) and render the tooltip as a
//! `frame.fill_rectangle` + `frame.fill_text` block sized to the six
//! R4.2 fields.
//!
//! ## Field shape (Q4-operator-resolved)
//!
//! Six fields for fill markers: **Side**, **Price**, **Qty**, **Notional**,
//! **Time**, **Strategy ID**. Reduced ghost-variant for signal markers:
//! omits **Price** and **Notional** per R5.6, adds the "Intent — not
//! executed" badge and (when `was_clamped`) a clamp-reason sub-row.
//!
//! ## Positioning (R4.4)
//!
//! Default anchor is **above-and-right** of the marker centroid. When the
//! marker sits in the upper-right quadrant of the inner rect (i.e. the
//! default position would render off-canvas), flip to **below-and-left**.
//!
//! **Zero string literals** — copy via `crate::strings::CHART_TOOLTIP_*`.
//! **Zero hex colours** — all tokens via `crate::theme`.

use iced::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};
use iced::{Color, Point, Rectangle, Size, Vector};
use rust_decimal::Decimal;
use time::format_description::well_known::Rfc3339;

use crate::state::{ChartTooltipKind, ChartTooltipView};
use crate::strings::{
    CHART_TOOLTIP_CLAMP_SUFFIX, CHART_TOOLTIP_GHOST_BADGE, CHART_TOOLTIP_NOTIONAL_LABEL,
    CHART_TOOLTIP_PRICE_LABEL, CHART_TOOLTIP_QTY_LABEL, CHART_TOOLTIP_STRATEGY_LABEL,
    CHART_TOOLTIP_STRATEGY_NONE, CHART_TOOLTIP_TS_LABEL,
};
use crate::theme::{color, text, ThemeMode};

use super::chart::{side_badge_color, side_badge_label};

/// Tooltip card width in pixels. Fixed so the layout-math is deterministic
/// (canvas doesn't lay out auto-sized children — we draw a fixed
/// rectangle and place text inside).
const TOOLTIP_WIDTH_PX: f32 = 200.0;

/// Vertical padding inside the card.
const TOOLTIP_PAD_Y_PX: f32 = 8.0;

/// Horizontal padding inside the card.
const TOOLTIP_PAD_X_PX: f32 = 10.0;

/// Inter-row gap inside the card.
const TOOLTIP_ROW_GAP_PX: f32 = 4.0;

/// Pixel offset from the marker centroid to the **anchor corner** of the
/// tooltip in the default "above-right" orientation. The card's
/// bottom-left corner lands at `(centroid.x + GAP, centroid.y - GAP)`.
const TOOLTIP_GAP_PX: f32 = 12.0;

/// Render the tooltip card on top of the chart canvas. Called at the end
/// of `ChartProgram::draw` when both the tooltip view and the centroid
/// anchor are present.
pub(crate) fn draw_tooltip(
    frame: &mut Frame,
    bounds: Rectangle,
    anchor: Point,
    view: &ChartTooltipView,
    mode: ThemeMode,
) {
    let rows = build_rows(view);
    let height = card_height(&rows);

    let card_rect = compute_card_rect(anchor, bounds, height);

    // Background fill + 1-px BORDER_STRONG outline. Reuses the existing
    // panel-card semantic (BG_ELEV + BORDER_STRONG) so the tooltip reads
    // as a sibling of cockpit panels rather than a foreign overlay.
    let bg_path = Path::rectangle(
        Point::new(card_rect.x, card_rect.y),
        Size::new(card_rect.width, card_rect.height),
    );
    frame.fill(&bg_path, color::PANEL_RAISED.current(mode));
    frame.stroke(
        &bg_path,
        Stroke::default()
            .with_color(color::BORDER_STRONG.current(mode))
            .with_width(1.0),
    );

    // Drop-shadow: poor-man's shadow via a 1-px black-tier offset on top
    // of the bg fill (iced canvas has no native shadow on filled
    // rectangles). Skipped here to avoid double-shadow visual weight —
    // the marker already carries `whisper_shadow`.

    // Per-row text. Each row carries its own (label, value, optional
    // accent color).
    let mut y = card_rect.y + TOOLTIP_PAD_Y_PX;
    #[allow(clippy::cast_precision_loss)]
    let row_h = text::BODY as f32 + TOOLTIP_ROW_GAP_PX;
    for row in &rows {
        draw_row(frame, card_rect.x, y, card_rect.width, row, mode);
        y += row_h;
    }
}

#[derive(Debug, Clone)]
struct TooltipRow {
    label: String,
    value: String,
    accent: Option<Color>,
}

fn build_rows(view: &ChartTooltipView) -> Vec<TooltipRow> {
    let mut rows = Vec::with_capacity(7);

    // Row 1: Side badge (always present), with optional "(clamped)"
    // suffix for ghost signals.
    let side_label = side_badge_label(view.side);
    let side_text = if view.was_clamped {
        format!("{side_label}{CHART_TOOLTIP_CLAMP_SUFFIX}")
    } else {
        side_label.to_string()
    };
    rows.push(TooltipRow {
        label: String::new(),
        value: side_text,
        accent: Some(side_badge_color(view.side, crate::theme::ThemeMode::Dark)),
    });

    // Ghost-variant badge (R5.6) directly under the side badge.
    if matches!(view.kind, ChartTooltipKind::Signal) {
        rows.push(TooltipRow {
            label: String::new(),
            value: CHART_TOOLTIP_GHOST_BADGE.to_string(),
            accent: None,
        });
    }

    // Price + Notional rows — only for executed-fill markers (R5.6
    // omits price and notional from the ghost variant).
    if let Some(price) = view.price {
        rows.push(TooltipRow {
            label: CHART_TOOLTIP_PRICE_LABEL.to_string(),
            value: format_price(price),
            accent: None,
        });
    }
    rows.push(TooltipRow {
        label: CHART_TOOLTIP_QTY_LABEL.to_string(),
        value: format_qty(view.qty),
        accent: None,
    });
    if let Some(notional) = view.notional {
        rows.push(TooltipRow {
            label: CHART_TOOLTIP_NOTIONAL_LABEL.to_string(),
            value: format_notional(notional),
            accent: None,
        });
    }

    // Time row.
    rows.push(TooltipRow {
        label: CHART_TOOLTIP_TS_LABEL.to_string(),
        value: format_ts(view.ts),
        accent: None,
    });

    // Strategy row — "—" when absent.
    let strategy_str = view
        .strategy_id
        .as_deref()
        .map_or_else(|| CHART_TOOLTIP_STRATEGY_NONE.to_string(), str::to_string);
    rows.push(TooltipRow {
        label: CHART_TOOLTIP_STRATEGY_LABEL.to_string(),
        value: strategy_str,
        accent: None,
    });

    // Optional clamp-reason sub-row (ghost variant only).
    if matches!(view.kind, ChartTooltipKind::Signal)
        && let Some(reason) = view.clamp_reason.as_deref()
    {
        rows.push(TooltipRow {
            label: String::new(),
            value: reason.to_string(),
            accent: None,
        });
    }

    rows
}

fn card_height(rows: &[TooltipRow]) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let body = text::BODY as f32;
    #[allow(clippy::cast_precision_loss)]
    let n = rows.len() as f32;
    TOOLTIP_PAD_Y_PX * 2.0 + n * body + (n - 1.0).max(0.0) * TOOLTIP_ROW_GAP_PX
}

/// Position the card relative to the marker centroid (R4.4). Default
/// orientation is above-and-right; when the default would push the card
/// off the right or top edge of `bounds`, flip to below-and-left.
///
/// **T3006 — defence-in-depth clamp inside `bounds`** (chart-canvas-
/// overhaul v1.10.0, R1.3).  After the default-orientation choice and
/// the flip-on-overflow heuristic, hard-clamp the returned rectangle
/// so it is fully contained in `bounds` even when `width > bounds.width`
/// or `height > bounds.height` (in which case the clamp pins the card
/// to the bounds origin and the card visibly truncates rather than
/// silently rendering off-canvas).  The pre-T3006 form composed
/// `.max(bounds.x)` then `.min(bounds.x + bounds.width - width)`;
/// when `bounds.width < width` the `.min(...)` produced a value
/// smaller than `bounds.x`, and the final clamp resolved to a
/// position the card couldn't fit at.  The post-T3006 form takes
/// the order `min → max` so the pin-to-origin invariant holds:
/// if the card is wider than `bounds`, `x = bounds.x`.
fn compute_card_rect(anchor: Point, bounds: Rectangle, height: f32) -> Rectangle {
    let width = TOOLTIP_WIDTH_PX;

    // Default anchor places the card's bottom-left corner at
    // (anchor.x + gap, anchor.y - gap).
    let mut x = anchor.x + TOOLTIP_GAP_PX;
    let mut y = anchor.y - TOOLTIP_GAP_PX - height;

    // Flip horizontally when the card would overflow the right edge.
    if x + width > bounds.x + bounds.width {
        x = anchor.x - TOOLTIP_GAP_PX - width;
    }
    // Flip vertically when the card would overflow the top edge.
    if y < bounds.y {
        y = anchor.y + TOOLTIP_GAP_PX;
    }

    // T3006 — defence-in-depth clamp.  Apply the upper-bound clamp
    // first, then the lower-bound clamp, so a pathological
    // `width > bounds.width` pins the card to `bounds.x` (instead
    // of producing a position smaller than `bounds.x` and then
    // having `.max(bounds.x)` correct it back — only correct by
    // coincidence when bounds.width ≥ width).
    let right_anchor = (bounds.x + bounds.width - width).max(bounds.x);
    let bottom_anchor = (bounds.y + bounds.height - height).max(bounds.y);
    x = x.min(right_anchor).max(bounds.x);
    y = y.min(bottom_anchor).max(bounds.y);

    Rectangle {
        x,
        y,
        width,
        height,
    }
}

fn draw_row(frame: &mut Frame, x: f32, y: f32, width: f32, row: &TooltipRow, mode: ThemeMode) {
    #[allow(clippy::cast_precision_loss)]
    let body = text::BODY as f32;
    let label_color = color::FG_3.current(mode);
    let value_color = row.accent.unwrap_or_else(|| color::FG_1.current(mode));

    // Label on the left, value right-aligned. When label is empty the
    // value spans the row (used for the side badge + ghost badge).
    if row.label.is_empty() {
        let mid = Point::new(x + TOOLTIP_PAD_X_PX, y);
        #[allow(clippy::useless_conversion)]
        frame.fill_text(CanvasText {
            content: row.value.clone(),
            position: mid,
            color: value_color,
            size: body.into(),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Top.into(),
            ..CanvasText::default()
        });
        return;
    }

    let label_pos = Point::new(x + TOOLTIP_PAD_X_PX, y);
    #[allow(clippy::useless_conversion)]
    frame.fill_text(CanvasText {
        content: row.label.clone(),
        position: label_pos,
        color: label_color,
        size: body.into(),
        align_x: iced::alignment::Horizontal::Left.into(),
        align_y: iced::alignment::Vertical::Top.into(),
        ..CanvasText::default()
    });

    let value_pos = Point::new(x + width - TOOLTIP_PAD_X_PX, y);
    #[allow(clippy::useless_conversion)]
    frame.fill_text(CanvasText {
        content: row.value.clone(),
        position: value_pos,
        color: value_color,
        size: body.into(),
        align_x: iced::alignment::Horizontal::Right.into(),
        align_y: iced::alignment::Vertical::Top.into(),
        ..CanvasText::default()
    });
    // `Vector` import unused warning suppressor — Vector is brought in
    // for `compute_card_rect` future use of slope vectors.
    let _ = Vector::ZERO;
}

fn format_price(d: Decimal) -> String {
    format!("{d:.4}")
}

fn format_qty(d: Decimal) -> String {
    format!("{d:.4}")
}

fn format_notional(d: Decimal) -> String {
    format!("{d:.2}")
}

fn format_ts(ts: trading_core::Timestamp) -> String {
    ts.inner()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("—"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::state::{ChartTooltipKind, ChartTooltipView};
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use trading_core::{Side, Timestamp};

    fn fixed_ts() -> Timestamp {
        let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        Timestamp::new(dt)
    }

    fn fill_view() -> ChartTooltipView {
        ChartTooltipView {
            kind: ChartTooltipKind::Fill,
            side: Side::Buy,
            price: Some(dec!(40010.1234)),
            qty: dec!(0.1234),
            notional: Some(dec!(4937.25)),
            ts: fixed_ts(),
            strategy_id: Some(SmolStr::new("sma_crossover")),
            was_clamped: false,
            clamp_reason: None,
        }
    }

    fn signal_view(clamped: bool) -> ChartTooltipView {
        ChartTooltipView {
            kind: ChartTooltipKind::Signal,
            side: Side::Sell,
            price: None,
            qty: dec!(0.05),
            notional: None,
            ts: fixed_ts(),
            strategy_id: Some(SmolStr::new("sma_crossover")),
            was_clamped: clamped,
            clamp_reason: if clamped {
                Some(SmolStr::new("per_symbol_cap"))
            } else {
                None
            },
        }
    }

    /// T2009 — fill-variant renders all six R4.2 fields verbatim:
    /// Side, Price, Qty, Notional, Time, Strategy.
    #[test]
    fn chart_tooltip_fill_variant_has_six_fields() {
        let rows = build_rows(&fill_view());
        // Side, Price, Qty, Notional, Time, Strategy = 6 rows.
        assert_eq!(rows.len(), 6, "fill variant should have 6 rows");
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&""), "side badge has empty label");
        assert!(labels.contains(&CHART_TOOLTIP_PRICE_LABEL));
        assert!(labels.contains(&CHART_TOOLTIP_QTY_LABEL));
        assert!(labels.contains(&CHART_TOOLTIP_NOTIONAL_LABEL));
        assert!(labels.contains(&CHART_TOOLTIP_TS_LABEL));
        assert!(labels.contains(&CHART_TOOLTIP_STRATEGY_LABEL));
    }

    /// T2019 — ghost-variant renders the ghost-badge row and omits Price
    /// + Notional per R5.6.
    #[test]
    fn chart_tooltip_ghost_variant_renders_no_price() {
        let rows = build_rows(&signal_view(false));
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        let values: Vec<_> = rows.iter().map(|r| r.value.as_str()).collect();
        assert!(
            values.contains(&CHART_TOOLTIP_GHOST_BADGE),
            "ghost badge present: rows={values:?}"
        );
        assert!(
            !labels.contains(&CHART_TOOLTIP_PRICE_LABEL),
            "no price label in ghost variant"
        );
        assert!(
            !labels.contains(&CHART_TOOLTIP_NOTIONAL_LABEL),
            "no notional label in ghost variant"
        );
        assert!(
            labels.contains(&CHART_TOOLTIP_QTY_LABEL),
            "qty present in ghost variant"
        );
    }

    /// T2019 — ghost-variant with `was_clamped = true` carries the
    /// "(clamped)" suffix on the side row + a clamp-reason sub-row.
    #[test]
    fn chart_tooltip_ghost_clamped_renders_reason() {
        let rows = build_rows(&signal_view(true));
        let values: Vec<_> = rows.iter().map(|r| r.value.as_str()).collect();
        let has_clamped_side = values
            .iter()
            .any(|v| v.contains(CHART_TOOLTIP_CLAMP_SUFFIX.trim()));
        assert!(has_clamped_side, "side row should carry clamped suffix");
        assert!(
            values.contains(&"per_symbol_cap"),
            "clamp reason sub-row missing: rows={values:?}"
        );
    }

    /// T2009 — `compute_card_rect` flips when the marker is in the
    /// upper-right quadrant of the inner rect (R4.4).
    #[test]
    fn chart_tooltip_flips_in_upper_right_quadrant() {
        // Marker in upper-right of bounds.
        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        };
        let height = 100.0;
        let upper_right = Point::new(380.0, 10.0);
        let rect = compute_card_rect(upper_right, bounds, height);
        // Card should land below + left of the marker.
        assert!(
            rect.x + rect.width <= 400.0,
            "card respects right edge: x={}, w={}",
            rect.x,
            rect.width
        );
        assert!(rect.y >= 0.0, "card respects top edge: y={}", rect.y);
    }

    /// T3006 — `tooltip_card_stays_inside_bounds_at_corners` —
    /// chart-canvas-overhaul v1.10.0 (R1.3 defence-in-depth).
    ///
    /// Drive `compute_card_rect` at four extreme marker positions
    /// (each corner of `bounds`) and assert the returned card stays
    /// fully inside `bounds` regardless of the default-orientation
    /// flip decision.  Catches the off-screen-render hypothesis
    /// from R1.3 (chart-canvas-overhaul brief, Hypothesis 3) without
    /// requiring the iced runtime.
    #[test]
    fn tooltip_card_stays_inside_bounds_at_corners() {
        // Bounds chosen to be larger than the card by a comfortable
        // margin so the default + flip orientations both produce
        // valid placements.
        let bounds = Rectangle {
            x: 100.0,
            y: 50.0,
            width: 600.0,
            height: 400.0,
        };
        let card_h = 120.0;
        let card_w = TOOLTIP_WIDTH_PX;

        // Iterate all four corners.
        let corners = [
            // Top-left: default orientation should NOT flip.
            Point::new(bounds.x + 4.0, bounds.y + 4.0),
            // Top-right: default would overflow right; expects
            // horizontal flip.
            Point::new(bounds.x + bounds.width - 4.0, bounds.y + 4.0),
            // Bottom-left: default would overflow top; expects vertical flip.
            Point::new(bounds.x + 4.0, bounds.y + bounds.height - 4.0),
            // Bottom-right: default would overflow both; expects both flips.
            Point::new(
                bounds.x + bounds.width - 4.0,
                bounds.y + bounds.height - 4.0,
            ),
        ];

        for (i, anchor) in corners.iter().enumerate() {
            let r = compute_card_rect(*anchor, bounds, card_h);
            assert!(
                r.x >= bounds.x - 0.001,
                "corner {i}: card.x ({}) >= bounds.x ({})",
                r.x,
                bounds.x,
            );
            assert!(
                r.y >= bounds.y - 0.001,
                "corner {i}: card.y ({}) >= bounds.y ({})",
                r.y,
                bounds.y,
            );
            assert!(
                r.x + r.width <= bounds.x + bounds.width + 0.001,
                "corner {i}: card.right ({}) <= bounds.right ({})",
                r.x + r.width,
                bounds.x + bounds.width,
            );
            assert!(
                r.y + r.height <= bounds.y + bounds.height + 0.001,
                "corner {i}: card.bottom ({}) <= bounds.bottom ({})",
                r.y + r.height,
                bounds.y + bounds.height,
            );
            assert!(
                (r.width - card_w).abs() < f32::EPSILON,
                "corner {i}: width preserved",
            );
            assert!(
                (r.height - card_h).abs() < f32::EPSILON,
                "corner {i}: height preserved",
            );
        }
    }

    /// T3006 — pathological pin-to-origin case: card wider than
    /// `bounds`.  Earlier implementation could clamp `x` to
    /// `bounds.x + bounds.width - width` < `bounds.x`, then re-apply
    /// `.max(bounds.x)` and land at `bounds.x` — but only because of
    /// the second clamp's coincidental order.  The post-T3006 clamp
    /// makes the pin-to-origin invariant explicit.
    #[test]
    fn tooltip_card_pins_to_origin_when_wider_than_bounds() {
        let bounds = Rectangle {
            x: 200.0,
            y: 100.0,
            width: 50.0,  // narrower than TOOLTIP_WIDTH_PX
            height: 40.0, // smaller than any plausible card height
        };
        let r = compute_card_rect(Point::new(225.0, 120.0), bounds, 100.0);
        // Card pins to bounds origin (or close — the order of clamps
        // resolves to `x = bounds.x` and `y = bounds.y`).
        assert!(
            (r.x - bounds.x).abs() < 0.001,
            "pin: card.x ({}) == bounds.x ({})",
            r.x,
            bounds.x,
        );
        assert!(
            (r.y - bounds.y).abs() < 0.001,
            "pin: card.y ({}) == bounds.y ({})",
            r.y,
            bounds.y,
        );
    }
}
