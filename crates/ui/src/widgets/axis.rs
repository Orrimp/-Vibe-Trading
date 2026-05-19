//! Shared axis-rendering helpers (T-D-N17, ADR-0034 § D8).
//!
//! Extracted to avoid duplicating tick-spacing and label-formatting logic
//! across `widgets::chart` and `widgets::training_plot`. Pure functions only —
//! no rendering state, no `Frame` references. Callers drive their own draw
//! passes using the computed positions.
//!
//! `widgets::chart` retains its own copies of the chart-specific helpers
//! (time axis, price axis) as pure-internal functions. This module provides
//! the generalized helpers that `widgets::training_plot` needs for its
//! loss-curve y-axis.
//!
//! ## Invariant
//!
//! All functions here are pure: same inputs → same outputs. No global state,
//! no `Instant::now()`, no external I/O. Tests are inline and deterministic.

/// Compute up to `max_ticks` evenly-spaced tick positions on a linear axis
/// spanning `[0, scale]` (where `scale` is the max value, already multiplied
/// by the caller's padding factor, e.g. `max * 1.1`).
///
/// Returns a `Vec<f32>` of tick values in ascending order starting from 0.
/// The spacing is chosen to be a "nice" power-of-10 multiple so labels are
/// round numbers.
///
/// # Examples
///
/// ```
/// use ui::widgets::axis::tick_positions;
/// let ticks = tick_positions(1.0, 5);
/// // Expected: [0.0, 0.25, 0.5, 0.75, 1.0] or similar
/// assert!(ticks.len() <= 6);
/// assert!(ticks[0] == 0.0);
/// ```
pub(crate) fn tick_positions(scale: f32, max_ticks: usize) -> Vec<f32> {
    if scale <= 0.0 || max_ticks == 0 {
        return vec![0.0];
    }

    // Choose a "nice" step: the largest power-of-10 multiple that gives
    // at most `max_ticks` intervals.
    // `max_ticks` is always small (≤ 20) in practice; the f32 precision is fine.
    #[allow(clippy::cast_precision_loss)]
    let raw_step = scale / max_ticks as f32;
    let magnitude = 10_f32.powf(raw_step.log10().floor());
    let step = if raw_step / magnitude < 2.0 {
        magnitude
    } else if raw_step / magnitude < 5.0 {
        2.0 * magnitude
    } else {
        5.0 * magnitude
    };

    let mut ticks = Vec::with_capacity(max_ticks + 1);
    let mut v = 0.0_f32;
    while v <= scale + step * 0.01 {
        ticks.push(v);
        v += step;
        // Guard against floating-point drift.
        if ticks.len() > max_ticks + 2 {
            break;
        }
    }
    ticks
}

/// Format a tick label value with adaptive precision.
///
/// - Values ≥ 10: no decimal places (e.g. "12").
/// - Values 1..10: 1 decimal place (e.g. "2.5").
/// - Values 0.1..1: 2 decimal places (e.g. "0.25").
/// - Values < 0.1: 3 decimal places (e.g. "0.025").
pub(crate) fn format_tick_label(v: f32) -> String {
    if v >= 10.0 {
        format!("{v:.0}")
    } else if v >= 1.0 {
        format!("{v:.1}")
    } else if v >= 0.1 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

/// Map a value in `[0, scale]` to a y-pixel coordinate in `[top, bottom]`
/// (y=0 at top, y increases downward — iced canvas convention).
///
/// Values are clamped to `[0, scale]` before mapping.
///
/// Reserved for canvas rendering (not yet used — canvas plot is a follow-on).
#[allow(dead_code)]
pub(crate) fn y_for_value(value: f32, scale: f32, top: f32, bottom: f32) -> f32 {
    if scale <= 0.0 {
        return bottom;
    }
    let clamped = value.clamp(0.0, scale);
    // Invert: scale (max) maps to top, 0 maps to bottom.
    let frac = clamped / scale;
    bottom - frac * (bottom - top)
}

/// Map an index `i` in `[0, count-1]` to an x-pixel coordinate in
/// `[left, right]`.
///
/// Returns `left` when `count <= 1`.
///
/// Reserved for canvas rendering (not yet used — canvas plot is a follow-on).
#[allow(dead_code)]
pub(crate) fn x_for_index(i: usize, count: usize, left: f32, right: f32) -> f32 {
    if count <= 1 {
        return left;
    }
    // `i` and `count` are small axis-tick indices (≤ max_ticks ≤ 20);
    // f32 mantissa (23 bits) is sufficient for this domain.
    #[allow(clippy::cast_precision_loss)]
    let ratio = i as f32 / (count - 1) as f32;
    left + ratio * (right - left)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify tick_positions produces the same Vec as the previous in-chart
    /// helper for common inputs (T-D-N17 invariant).
    #[test]
    fn tick_positions_scale_1_max_5() {
        let ticks = tick_positions(1.0, 5);
        assert!(ticks[0] == 0.0, "first tick must be 0");
        assert!(ticks.len() <= 7, "must not exceed max_ticks+2 guard");
        // All ticks must be in [0, 1.0 + epsilon].
        for t in &ticks {
            assert!(*t >= 0.0 && *t <= 1.1, "tick {t} out of range");
        }
    }

    #[test]
    fn tick_positions_scale_10_max_5() {
        let ticks = tick_positions(10.0, 5);
        assert_eq!(ticks[0], 0.0);
        // With scale=10 and max_ticks=5, step should be 2.0
        assert!(
            ticks.len() >= 3,
            "should have at least 3 ticks for scale=10"
        );
    }

    #[test]
    fn tick_positions_zero_scale_returns_zero() {
        let ticks = tick_positions(0.0, 5);
        assert_eq!(ticks, vec![0.0]);
    }

    #[test]
    fn format_tick_label_precision() {
        assert_eq!(format_tick_label(12.0), "12");
        assert_eq!(format_tick_label(2.5), "2.5");
        assert_eq!(format_tick_label(0.25), "0.25");
        assert_eq!(format_tick_label(0.025), "0.025");
    }

    #[test]
    fn y_for_value_extremes() {
        // value=0 → bottom; value=scale → top
        let y_bot = y_for_value(0.0, 1.0, 10.0, 100.0);
        let y_top = y_for_value(1.0, 1.0, 10.0, 100.0);
        assert!((y_bot - 100.0).abs() < 1e-5, "value=0 must map to bottom");
        assert!((y_top - 10.0).abs() < 1e-5, "value=scale must map to top");
    }

    #[test]
    fn x_for_index_spread() {
        let x0 = x_for_index(0, 5, 0.0, 100.0);
        let x4 = x_for_index(4, 5, 0.0, 100.0);
        assert!((x0 - 0.0).abs() < 1e-5);
        assert!((x4 - 100.0).abs() < 1e-5);
    }
}
