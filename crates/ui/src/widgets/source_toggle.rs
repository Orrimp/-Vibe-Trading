//! Source toggle widget — lab-yahoo-realdata T-C3.2 / R-UI-1.1.
//!
//! Chip toggle between `Synthetic` (GBM), `YahooCache` (real Yahoo bars), and
//! — when the `binance` feature is enabled — `BinanceCache` (the pinned
//! Binance hourly corpus). Dispatches `Message::LabSelectDataSource(
//! LabDataSource)` on each chip press.
//!
//! ## Design
//!
//! Chips side-by-side in a `Row`. The active chip uses the Lumen `ACCENT`
//! token; inactive chips use the `PANEL_RAISED` token. No new Lumen tokens.
//!
//! ## Feature gating (simple-strategies-realdata T-B5 / AC8)
//!
//! The Binance chip is `#[cfg(feature = "binance")]`. In a `--no-default-
//! features --features fixtures` build (no `binance`) the toggle renders TWO
//! chips, byte-identical to the pre-feature widget — the fixtures cockpit is
//! unchanged. The everyday cockpit (`default = [.., "binance"]`) renders THREE.
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::Length;
use iced::widget::{Row, button};

use crate::lab::state::LabDataSource;
use crate::state::Message;
#[cfg(feature = "binance")]
use crate::strings::LAB_SOURCE_BINANCE;
use crate::strings::{LAB_SOURCE_SYNTHETIC, LAB_SOURCE_YAHOO};
use crate::theme::{ThemeMode, color, radius, space, text};

/// Render the Source toggle — `Synthetic` + `YahooCache` chips, plus a
/// `BinanceCache` chip when the `binance` feature is enabled.
///
/// The active variant is visually distinguished by the accent background.
/// Pressing an already-active chip is a no-op (the iced button still fires
/// `LabSelectDataSource` but the handler is idempotent).
#[must_use]
pub fn view(current: LabDataSource, mode: ThemeMode) -> crate::Element<'static> {
    let synthetic_active = current == LabDataSource::Synthetic;
    let yahoo_active = current == LabDataSource::YahooCache;

    let synthetic_btn = chip_button(
        LAB_SOURCE_SYNTHETIC,
        synthetic_active,
        Message::LabSelectDataSource(LabDataSource::Synthetic),
        mode,
    );
    let yahoo_btn = chip_button(
        LAB_SOURCE_YAHOO,
        yahoo_active,
        Message::LabSelectDataSource(LabDataSource::YahooCache),
        mode,
    );

    let row = Row::new()
        .spacing(space::XXS)
        .push(synthetic_btn)
        .push(yahoo_btn);

    // simple-strategies-realdata T-B1 — the third chip is feature-gated so the
    // fixtures cockpit (no `binance`) stays a two-chip, byte-unchanged toggle.
    #[cfg(feature = "binance")]
    let row = {
        let binance_active = current == LabDataSource::BinanceCache;
        let binance_btn = chip_button(
            LAB_SOURCE_BINANCE,
            binance_active,
            Message::LabSelectDataSource(LabDataSource::BinanceCache),
            mode,
        );
        row.push(binance_btn)
    };

    row.width(Length::Shrink).into()
}

/// Build a single chip button for the source toggle.
#[allow(clippy::cast_possible_truncation)]
fn chip_button(
    label: &'static str,
    active: bool,
    msg: Message,
    mode: ThemeMode,
) -> crate::Element<'static> {
    let fg = if active {
        color::FG_ON_ACCENT.current(mode)
    } else {
        color::FG_3.current(mode)
    };
    let bg = if active {
        color::ACCENT.current(mode)
    } else {
        color::PANEL_RAISED.current(mode)
    };

    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };
    button(iced::widget::Text::new(label).size(text::SMALL).color(fg))
        .on_press(msg)
        .padding([space::XXS as u16, space::S as u16])
        .style(move |_t: &iced::Theme, _s| button::Style {
            background: Some(bg.into()),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius::R3.into(),
            },
            text_color: fg,
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-C3.2 — view does not panic for any source variant (incl. light mode).
    #[test]
    fn source_toggle_view_does_not_panic() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let _ = view(LabDataSource::Synthetic, mode);
            let _ = view(LabDataSource::YahooCache, mode);
            let _ = view(LabDataSource::BinanceCache, mode);
        }
    }

    /// T-C3.2 — active chip differs by `data_source` selection.
    #[test]
    fn source_toggle_active_selection() {
        // Just verify the enum-level logic — no render runtime needed.
        assert_eq!(LabDataSource::default(), LabDataSource::Synthetic);
        assert_ne!(LabDataSource::Synthetic, LabDataSource::YahooCache);
        assert_ne!(LabDataSource::YahooCache, LabDataSource::BinanceCache);
        assert_ne!(LabDataSource::Synthetic, LabDataSource::BinanceCache);
    }
}
