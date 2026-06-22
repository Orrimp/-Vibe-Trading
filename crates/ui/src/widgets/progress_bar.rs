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
use iced::widget::{Column, Row, Text, progress_bar};

use crate::theme::{ThemeMode, color, radius, space, text as theme_text};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Height of the progress bar track in logical pixels (R8.2).
pub const PROGRESS_BAR_HEIGHT: f32 = 8.0;

/// Width in logical pixels used when the bar is rendered in the Lab top-bar row.
pub const PROGRESS_BAR_WIDTH: f32 = 160.0;

/// Indeterminate sentinel — rendered as a static 30% partial fill.
const INDETERMINATE_FILL: f32 = 0.30;

/// Build the styled `progress_bar` primitive at the given `length`, with the
/// shared track / fill / radius tokens. One source of truth for the bar's look,
/// reused by both [`view`] (fixed-width Lab top-bar) and [`view_block`]
/// (full-width labelled block). `fill_pct` is pre-clamped by the callers.
fn styled_bar(
    fill_pct: f32,
    length: Length,
    mode: ThemeMode,
) -> iced::widget::ProgressBar<'static, iced::Theme> {
    let track_color = color::PANEL_SUNKEN.current(mode);
    let fill_color = color::ACCENT_2.current(mode);
    let radius_val: iced::border::Radius = radius::R4.into();
    progress_bar(0.0..=1.0, fill_pct)
        .girth(iced::Length::Fixed(PROGRESS_BAR_HEIGHT))
        .length(length)
        .style(move |_theme| iced::widget::progress_bar::Style {
            background: track_color.into(),
            bar: fill_color.into(),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: radius_val,
            },
        })
}

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
    let bar = styled_bar(fill_pct, Length::Fixed(PROGRESS_BAR_WIDTH), mode);

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

/// Render a FULL-WIDTH determinate progress bar with the label ABOVE it — a
/// labelled block suited to sitting beneath a panel at the panel's width (the
/// bake-off progress placement: "same width as the Plan-your-bake-off panel,
/// beneath it").
///
/// - `progress` — `Some(f32)` in `[0.0, 1.0]` for determinate; `None` → the
///   indeterminate 30% sentinel (the cold pre-first-event fallback).
/// - `label` — the line shown above the bar (e.g. "Running v0.5.bbands — 4 of
///   7"). Always present here (a block without a label is just [`view`]).
/// - `mode` — active theme mode.
///
/// Reuses [`styled_bar`] (same track / fill / radius tokens as the Lab bar) so
/// there is ONE progress-bar look in the design system — only the width + label
/// placement differ. Returns `Element<'static>`.
#[must_use]
pub fn view_block<Message: 'static>(
    progress: Option<f32>,
    label: &str,
    mode: ThemeMode,
) -> iced::Element<'static, Message> {
    let fill_pct = progress.unwrap_or(INDETERMINATE_FILL).clamp(0.0, 1.0);
    let bar = styled_bar(fill_pct, Length::Fill, mode);

    let label_text: iced::Element<'static, Message> = Text::new(label.to_string())
        .size(theme_text::MICRO)
        .color(color::FG_3.current(mode))
        .width(Length::Fill)
        .into();

    Column::new()
        .spacing(space::XS)
        .width(Length::Fill)
        .push(label_text)
        .push(bar)
        .into()
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

    /// `view_block` (the full-width labelled block — the bake-off progress bar)
    /// constructs without panic in BOTH themes, determinate + indeterminate.
    /// (The new screen must render in `--theme dark` AND `--theme light`.)
    #[test]
    fn view_block_constructs_both_modes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let _det: iced::Element<'static, ()> =
                view_block(Some(0.42), "Running v0.5.bbands — 4 of 7", mode);
            let _indet: iced::Element<'static, ()> = view_block(None, "Starting…", mode);
            let _full: iced::Element<'static, ()> =
                view_block(Some(1.0), "Running v0.buyhold — 7 of 7", mode);
        }
    }

    /// `view_block` always wraps the label + bar in a `Column` (2 children) —
    /// the label is ALWAYS present (it is the labelled-block variant), unlike
    /// `view` whose label is optional. Pins the structural contract.
    #[test]
    fn view_block_has_label_and_bar() {
        use iced::advanced::widget::Tree;
        let el: iced::Element<'static, ()> = view_block(Some(0.5), "Running x — 4 of 7", ThemeMode::Dark);
        let tree = Tree::new(el.as_widget());
        assert_eq!(
            tree.children.len(),
            2,
            "view_block must be a Column with 2 children (label Text + ProgressBar); got {}",
            tree.children.len()
        );
    }
}
