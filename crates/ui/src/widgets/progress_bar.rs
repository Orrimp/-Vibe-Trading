//! Determinate progress bar for the Lab run flow (lab-end-to-end-v2 T-AR-6 / R8).
//!
//! ## Visual contract (R8.2)
//!
//! - Height: `PROGRESS_BAR_HEIGHT` (8 px constant).
//! - Track color: `color::BG_3`.
//! - Fill color: `color::ACCENT_2`.
//! - Corner radius: `radius::R4`.
//! - Label: `Some(&str)` for "412 / 720 bars · 3.4s" rendered as small text
//!   next to the bar. `None` → bar only.
//!
//! ## Indeterminate variant (R8.3)
//!
//! When `progress == None`, the bar renders a static 30% partial fill to
//! signal "in progress but count unknown". A shimmer animation would require
//! iced canvas custom drawing; the static partial is the P1 deliverable.
//!
//! ## Q5=(a) — Spinner stays
//!
//! The progress bar is SIBLING to the `ThrottledSpinner`, not a replacement.
//! Lab top-bar renders: `[Spinner] [ProgressBar + label]` when inflight.
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — all dynamic strings from callers.

use iced::Length;
use iced::widget::{Row, Text, progress_bar};

use crate::theme::{ThemeMode, color, radius, space, text as theme_text};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Height of the progress bar track in logical pixels (R8.2).
pub const PROGRESS_BAR_HEIGHT: f32 = 8.0;

/// Width in logical pixels used when the bar is rendered in the Lab top-bar row.
pub const PROGRESS_BAR_WIDTH: f32 = 160.0;

/// Indeterminate sentinel — rendered as a static 30% partial fill.
const INDETERMINATE_FILL: f32 = 0.30;

// ── view ─────────────────────────────────────────────────────────────────────

/// Render the Lab progress bar.
///
/// - `progress` — `Some(f32)` in `[0.0, 1.0]` for determinate; `None` for
///   indeterminate (static 30% fill sentinel per R8.3 P1 deliverable).
/// - `label` — optional text rendered to the right of the bar (e.g.
///   `"412 / 720 bars · 3.4s"`). `None` → bar only.
/// - `mode` — active theme mode (Lumen Light / Dark).
///
/// Returns an `Element<'static>` so it can be pushed into a static `Row`.
#[must_use]
pub fn view<Message: 'static>(
    progress: Option<f32>,
    label: Option<&str>,
    mode: ThemeMode,
) -> iced::Element<'static, Message> {
    let fill_pct = progress.unwrap_or(INDETERMINATE_FILL).clamp(0.0, 1.0);

    let track_color = color::PANEL_SUNKEN.current(mode);
    let fill_color = color::ACCENT_2.current(mode);
    let radius_val: iced::border::Radius = radius::R4.into();

    let bar = progress_bar(0.0..=1.0, fill_pct)
        .girth(iced::Length::Fixed(PROGRESS_BAR_HEIGHT))
        .length(iced::Length::Fixed(PROGRESS_BAR_WIDTH))
        .style(move |_theme| iced::widget::progress_bar::Style {
            background: track_color.into(),
            bar: fill_color.into(),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: radius_val,
            },
        });

    if let Some(lbl) = label {
        let label_text: iced::Element<'static, Message> = Text::new(lbl.to_string())
            .size(theme_text::MICRO)
            .color(color::FG_3.current(mode))
            .into();

        Row::new()
            .spacing(space::S)
            .align_y(iced::Alignment::Center)
            .push(bar)
            .push(label_text)
            .width(Length::Shrink)
            .into()
    } else {
        bar.into()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    /// T-AR-6 — `view` constructs without panic at 0%.
    #[test]
    fn view_constructs_at_zero_pct() {
        let _el: iced::Element<'static, ()> = view(Some(0.0), None, ThemeMode::Dark);
    }

    /// T-AR-6 — `view` constructs without panic at 50%.
    #[test]
    fn view_constructs_at_50_pct() {
        let _el: iced::Element<'static, ()> =
            view(Some(0.5), Some("360 / 720 bars · 1.5s"), ThemeMode::Dark);
    }

    /// T-AR-6 — `view` constructs without panic at 100%.
    #[test]
    fn view_constructs_at_100_pct() {
        let _el: iced::Element<'static, ()> =
            view(Some(1.0), Some("720 / 720 bars · 3.0s"), ThemeMode::Light);
    }

    /// T-AR-6 — `view` constructs without panic in indeterminate mode.
    #[test]
    fn view_constructs_indeterminate() {
        let _el: iced::Element<'static, ()> = view(None, None, ThemeMode::Dark);
    }

    /// Progress bar renders correctly with a label in both modes.
    #[test]
    fn view_with_label_both_modes() {
        let _dark: iced::Element<'static, ()> =
            view(Some(0.42), Some("300 / 720 bars · 1.2s"), ThemeMode::Dark);
        let _light: iced::Element<'static, ()> =
            view(Some(0.42), Some("300 / 720 bars · 1.2s"), ThemeMode::Light);
    }
}
