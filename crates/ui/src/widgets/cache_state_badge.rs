//! Cache-state badge widget — lab-yahoo-realdata Wave D-followup (T-D2).
//!
//! Three-state pill chip rendered next to the Source toggle when the
//! `YahooCache` data source is active. Surfaces the freshness of the
//! Yahoo parquet cache for the currently selected ticker:
//!
//! - **Fresh** — mtime ≤ 24 h (`UP_500` green).
//! - **Stale** — mtime > 24 h (`DOWN_500` warning).
//! - **Empty** — no parquet files for the ticker (`FG_3` muted).
//!
//! ## Design
//!
//! Mirrors `cadence_badge` shape: Lumen chip with `PANEL_RAISED` background,
//! `BORDER_1` outline, `R3` radius, `MICRO` text. The semantic foreground
//! color is the only state-conditional token.
//!
//! **Zero hex literals** — colors via `crate::theme`.
//! **Zero string literals** — copy via `crate::strings`.

use iced::widget::{Container, Text, container};

use crate::lab::cache_state::CacheState;
use crate::strings::{LAB_CACHE_STATE_EMPTY, LAB_CACHE_STATE_FRESH, LAB_CACHE_STATE_STALE};
use crate::theme::{ThemeMode, color, radius, space, text};

/// Render the cache-state badge.
///
/// Caller is responsible for conditional rendering (only when
/// `data_source == YahooCache`).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn view(state: CacheState, mode: ThemeMode) -> crate::Element<'static> {
    let (label, fg) = match state {
        CacheState::Fresh => (LAB_CACHE_STATE_FRESH, color::UP_500.current(mode)),
        CacheState::Stale => (LAB_CACHE_STATE_STALE, color::DOWN_500.current(mode)),
        CacheState::Empty => (LAB_CACHE_STATE_EMPTY, color::FG_3.current(mode)),
    };
    let bg = color::PANEL_RAISED.current(mode);
    let border_color = color::BORDER_1.current(mode);

    Container::new(Text::new(label).size(text::MICRO).color(fg))
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

    /// Each state maps to a unique label string.
    #[test]
    fn cache_state_badge_labels_distinct() {
        let labels = [
            LAB_CACHE_STATE_FRESH,
            LAB_CACHE_STATE_STALE,
            LAB_CACHE_STATE_EMPTY,
        ];
        for (i, a) in labels.iter().enumerate() {
            for (j, b) in labels.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "labels must be distinct: {a} == {b}");
                }
            }
            assert!(!a.is_empty(), "label must be non-empty: {a}");
        }
    }

    /// View does not panic for any state × mode combination.
    #[test]
    fn cache_state_badge_view_does_not_panic() {
        for state in [CacheState::Fresh, CacheState::Stale, CacheState::Empty] {
            let _ = view(state, ThemeMode::Dark);
            let _ = view(state, ThemeMode::Light);
        }
    }
}
