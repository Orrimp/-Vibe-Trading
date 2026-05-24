//! Cadence badge widget — lab-yahoo-realdata T-C3.3 / T-AR4 / R-UI-1.4.
//!
//! Renders a small chip showing the adaptive bar cadence (`1m` / `1h` / `1d`)
//! derived from `Interval::derive_from_range(start_ms, end_ms)`.
//!
//! The badge is only visible when `data_source == YahooCache` — callers
//! are responsible for conditional rendering.
//!
//! ## Design
//!
//! Lumen chip shape: `SURFACE_2` background, `TEXT_SECONDARY` foreground,
//! `MICRO` font size, `R3` border radius. Positioned to the right of the
//! date-range picker row in `screens/lab.rs`.
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::widget::{Container, Text, container};

use crate::strings::{LAB_CADENCE_1D, LAB_CADENCE_1H, LAB_CADENCE_1M};
use crate::theme::{ThemeMode, color, radius, space, text};

/// Bar-cadence selector for the Yahoo adaptive-cadence path (T-AR4 / Q4 = (c)).
///
/// Mirrors `data::yahoo::Interval` without depending on the `data` crate
/// from the pure widget layer. The `lab::runner` derives this value via
/// `Interval::derive_from_range` and stores it in the caller's local state
/// for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadenceLabel {
    /// 1-minute bars (range < 7 days).
    Minutes1,
    /// 1-hour bars (range 7..=60 days).
    Hours1,
    /// 1-day bars (range > 60 days).
    Days1,
}

impl CadenceLabel {
    /// Derive the cadence label from a `(start_ms, end_ms)` UTC epoch-millis pair.
    ///
    /// Decision boundaries mirror `Interval::derive_from_range` (operator-locked,
    /// Q4 = (c) / ADR-0040 § D6).
    #[must_use]
    pub fn derive_from_range(start_ms: i64, end_ms: i64) -> Self {
        const MS_PER_DAY: i64 = 86_400_000;
        let range_days = (end_ms - start_ms).max(0) / MS_PER_DAY;
        match range_days {
            d if d < 7 => CadenceLabel::Minutes1,
            d if d <= 60 => CadenceLabel::Hours1,
            _ => CadenceLabel::Days1,
        }
    }

    /// Short label string for the badge chip.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            CadenceLabel::Minutes1 => LAB_CADENCE_1M,
            CadenceLabel::Hours1 => LAB_CADENCE_1H,
            CadenceLabel::Days1 => LAB_CADENCE_1D,
        }
    }
}

/// Render the cadence badge chip.
///
/// Positioned right-aligned in the date-range picker row by the caller.
/// Only rendered when `data_source == YahooCache`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn view(cadence: CadenceLabel, mode: ThemeMode) -> crate::Element<'static> {
    let fg = color::FG_3.current(mode);
    let bg = color::PANEL_RAISED.current(mode);
    let border_color = color::BORDER_1.current(mode);

    Container::new(Text::new(cadence.as_label()).size(text::MICRO).color(fg))
        .padding([space::XXS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg.into()),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius::R3.into(),
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-C3.3 — derive_from_range boundary truth table (mirrors T-AR4).
    #[test]
    fn cadence_badge_derive_from_range_boundaries() {
        const MS: i64 = 86_400_000;
        // 0 days → Minutes1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 0),
            CadenceLabel::Minutes1
        );
        // 6 days → Minutes1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 6 * MS),
            CadenceLabel::Minutes1
        );
        // 7 days → Hours1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 7 * MS),
            CadenceLabel::Hours1
        );
        // 30 days → Hours1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 30 * MS),
            CadenceLabel::Hours1
        );
        // 60 days → Hours1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 60 * MS),
            CadenceLabel::Hours1
        );
        // 61 days → Days1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 61 * MS),
            CadenceLabel::Days1
        );
        // 90 days → Days1
        assert_eq!(
            CadenceLabel::derive_from_range(0, 90 * MS),
            CadenceLabel::Days1
        );
    }

    /// T-C3.3 — all cadence labels are non-empty.
    #[test]
    fn cadence_badge_labels_non_empty() {
        for c in [
            CadenceLabel::Minutes1,
            CadenceLabel::Hours1,
            CadenceLabel::Days1,
        ] {
            assert!(!c.as_label().is_empty(), "empty label for {c:?}");
        }
    }

    /// T-C3.3 — view does not panic for any cadence variant.
    #[test]
    fn cadence_badge_view_does_not_panic() {
        let _ = view(CadenceLabel::Minutes1, ThemeMode::Dark);
        let _ = view(CadenceLabel::Hours1, ThemeMode::Dark);
        let _ = view(CadenceLabel::Days1, ThemeMode::Dark);
    }
}
