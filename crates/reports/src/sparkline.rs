//! Sparkline encoding (R3.2 / Q4).
//!
//! Eight-level Unicode-block palette (`▁▂▃▄▅▆▇█`, U+2581..U+2588).
//! Decimal-only — no `f64` enters the encoder so the output is
//! byte-identical across platforms / locales.
//!
//! Determinism property: same input `&[Decimal]` → byte-identical
//! UTF-8 output.  Asserted in this module's `tests` submodule.

use rust_decimal::Decimal;

/// The eight bar characters, low → high.
const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Default sparkline width (60 cells), per the Design section.
pub const DEFAULT_WIDTH: usize = 60;

/// Encode `values` as a Unicode-block sparkline of exactly `width`
/// cells.
///
/// - Empty input renders as `width` ASCII spaces.
/// - Constant input (range = 0) renders as `width` repetitions of `▁`
///   (the lowest bar — operator default for "flat curve, flat line").
/// - Otherwise: each cell maps to one of the 8 bars by linear bucketing
///   over `[min, max]` of the downsampled cells.
///
/// The downsampling is a simple chunked-average: split `values` into
/// `width` contiguous chunks of size `ceil(N / width)`, average each
/// chunk in `Decimal`, and bucket the chunk averages.  When `N < width`
/// the input is up-sampled by repeating each value to fill the cells.
#[must_use]
pub fn encode(values: &[Decimal], width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if values.is_empty() {
        return " ".repeat(width);
    }

    let cells = downsample_avg(values, width);

    // Determine min, max across cells.  When max == min we short-circuit
    // to the lowest bar — avoids divide-by-zero and is also visually
    // honest (a flat curve is a flat line).
    let mut min = cells[0];
    let mut max = cells[0];
    for c in &cells {
        if *c < min {
            min = *c;
        }
        if *c > max {
            max = *c;
        }
    }
    let range = max - min;
    if range == Decimal::ZERO {
        return BARS[0].to_string().repeat(width);
    }

    // For each cell c, bucket = floor((c - min) / range * 8); clamp to
    // [0, 7].  All Decimal arithmetic — no f64.
    let eight = Decimal::from(8u32);
    let mut out = String::with_capacity(width * 4); // up to 3-byte UTF-8 per cell
    for c in cells {
        let scaled = ((c - min) * eight) / range;
        let idx_u = scaled.trunc().to_string();
        let idx: usize = idx_u.parse::<i64>().unwrap_or(0).clamp(0, 7) as usize;
        out.push(BARS[idx]);
    }
    out
}

/// Downsample (or up-sample) `values` to exactly `width` averaged cells.
///
/// When `N >= width` we walk `width` chunks of size `ceil(N / width)`.
/// When `N < width` we repeat each value `width / N` times (with the
/// final cell absorbing the remainder) so the output length is exactly
/// `width`.
fn downsample_avg(values: &[Decimal], width: usize) -> Vec<Decimal> {
    let n = values.len();
    if n >= width {
        let mut out = Vec::with_capacity(width);
        let chunk = n.div_ceil(width).max(1);
        for cell_idx in 0..width {
            let start = cell_idx * chunk;
            if start >= n {
                // All remaining cells inherit the last computed average.
                let last = *out.last().unwrap_or(&values[n - 1]);
                out.push(last);
                continue;
            }
            let end = (start + chunk).min(n);
            let mut sum = Decimal::ZERO;
            let mut count: u32 = 0;
            for v in &values[start..end] {
                sum += *v;
                count += 1;
            }
            if count == 0 {
                out.push(values[start]);
            } else {
                out.push(sum / Decimal::from(count));
            }
        }
        out
    } else {
        // Up-sample: each input value occupies `width / n` cells; the
        // last input absorbs any remainder.  No averaging needed.
        let mut out = Vec::with_capacity(width);
        for (i, v) in values.iter().enumerate() {
            let cells_for_this = if i + 1 == n {
                width - out.len()
            } else {
                width / n
            };
            for _ in 0..cells_for_this {
                out.push(*v);
            }
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t807_encode_empty_returns_spaces() {
        assert_eq!(encode(&[], 60), " ".repeat(60));
    }

    #[test]
    fn t807_encode_constant_returns_lowest_bar() {
        let xs = vec![dec!(1.0), dec!(1.0), dec!(1.0), dec!(1.0)];
        assert_eq!(encode(&xs, 4), "▁▁▁▁");
    }

    #[test]
    fn t807_encode_one_to_eight_eight_cells() {
        // Hand-computed: 1..8 with width 8 → each cell maps to its bar.
        let xs: Vec<Decimal> = (1..=8).map(Decimal::from).collect();
        let out = encode(&xs, 8);
        assert_eq!(out, "▁▂▃▄▅▆▇█");
    }

    #[test]
    fn t807_encode_default_width_renders_60_cells() {
        let xs: Vec<Decimal> = (1..=120).map(Decimal::from).collect();
        let out = encode(&xs, DEFAULT_WIDTH);
        // Cell count is the number of grapheme cells, not bytes.
        let cell_count = out.chars().count();
        assert_eq!(cell_count, DEFAULT_WIDTH);
    }

    #[test]
    fn t807_encode_deterministic_two_calls() {
        let xs: Vec<Decimal> = (0..1000).map(Decimal::from).collect();
        let a = encode(&xs, 60);
        let b = encode(&xs, 60);
        assert_eq!(a, b);
    }

    #[test]
    fn t807_encode_first_and_last_bars_at_extremes() {
        // Strictly increasing series → first cell is the lowest bar,
        // last cell is the highest.
        let xs: Vec<Decimal> = (1..=60).map(Decimal::from).collect();
        let out = encode(&xs, 60);
        let mut chars = out.chars();
        assert_eq!(chars.next().unwrap(), BARS[0]);
        assert_eq!(chars.last().unwrap(), BARS[7]);
    }

    #[test]
    fn t807_encode_short_input_upsamples() {
        // 2 inputs into 8 cells: each value occupies 4 cells.
        let xs = vec![dec!(0), dec!(7)];
        let out = encode(&xs, 8);
        assert_eq!(out, "▁▁▁▁████");
    }
}
