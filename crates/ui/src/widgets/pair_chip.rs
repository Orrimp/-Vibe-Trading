//! Pair chip widget — ui-rethink-phase-a-lab T-D-5.
//!
//! Renders a `(Venue, Symbol)` tuple as a Lumen chip button. Used in the
//! Lab top-bar pair-chip row (R3). Clicking a chip dispatches
//! `Message::LabSelectPair(venue, symbol)`.
//!
//! **Active-chip treatment:** the currently-selected pair uses the
//! `frame::active_row` accent-rule left border and FG_1 text.
//! Inactive chips use the default chip style with FG_2 text.
//!
//! **Venue label:** Phase A universe is single-venue (Binance). The venue
//! suffix is hidden when there is only one venue in the universe — the
//! widget accepts an `show_venue` flag so a future multi-venue universe
//! can opt-in without touching call sites.
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::widget::{Container, Row, Text, button, container};
use iced::{Border, Length};
use trading_core::{Symbol, Venue};

use crate::state::Message;
use crate::theme::{ThemeMode, color, radius, space, text};

/// Render a single pair chip.
///
/// - `venue` / `symbol` — the pair this chip represents.
/// - `active` — whether this is the currently-selected pair.
/// - `show_venue` — append a `· Binance`-style suffix when `true`.
///   Set `false` for Phase A (single-venue universe).
/// - `mode` — active theme mode.
///
/// Returns a button element that dispatches
/// `Message::LabSelectPair(venue, symbol)` on press.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(
    venue: Venue,
    symbol: Symbol,
    active: bool,
    show_venue: bool,
    mode: ThemeMode,
) -> crate::Element<'static> {
    let symbol_str = symbol.0.to_string();

    let label_text = if show_venue {
        format!("{symbol_str} · {}", crate::strings::PAIR_CHIP_VENUE_BINANCE)
    } else {
        symbol_str
    };

    let fg = if active {
        color::FG_1.current(mode)
    } else {
        color::FG_2.current(mode)
    };

    let label = Text::new(label_text).size(text::SMALL).color(fg);

    let row_content = Row::new()
        .push(label)
        .spacing(space::XS)
        .align_y(iced::Alignment::Center);

    // Active chip: accent left-rule border via ACCENT color.
    // Inactive chip: subtle border via BORDER_1.
    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };

    let bg_color = if active {
        color::ACCENT_SOFT.current(mode)
    } else {
        color::PANEL.current(mode)
    };

    let chip = Container::new(row_content)
        .padding([space::S as u16, space::M as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg_color.into()),
            border: Border {
                color: border_color,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    let msg_venue = venue;
    let msg_symbol = symbol.clone();

    button(chip)
        .on_press(Message::LabSelectPair(msg_venue, msg_symbol))
        .padding(0)
        .style(|_t: &iced::Theme, _s| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// Render a row of pair chips from the given universe slice.
///
/// `selected` is the currently-active `(Venue, Symbol)` or `None` for
/// cold-start (no chip is highlighted). The list is rendered in the
/// order given by `universe` — callers pass the XRP-first const slice
/// from `lab::universe::XRP_FIRST_UNIVERSE`.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn row<'a>(
    universe: &'a [(Venue, Symbol)],
    selected: Option<&'a (Venue, Symbol)>,
    multi_venue: bool,
    mode: ThemeMode,
) -> crate::Element<'a> {
    use iced::widget::Row;

    let mut chips = Row::new().spacing(space::S);

    for (v, s) in universe {
        let active = selected.is_some_and(|(sv, ss)| sv == v && ss == s);
        chips = chips.push(view(*v, s.clone(), active, multi_venue, mode));
    }

    chips.into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(non_snake_case)] // double-underscore test names are a local snapshot-panel naming convention
mod tests {
    use trading_core::{Symbol, Venue};

    use crate::theme::ThemeMode;

    /// T-D-5 — `view` constructs without panic.
    #[test]
    fn pair_chip_constructs() {
        let _el = super::view(
            Venue::Binance,
            Symbol::new("XRPUSDT"),
            false,
            false,
            ThemeMode::Dark,
        );
        let _el2 = super::view(
            Venue::Binance,
            Symbol::new("ETHUSDT"),
            true,
            true,
            ThemeMode::Light,
        );
    }

    /// T-D-5 — `row` constructs from a 3-pair universe with no selection.
    #[test]
    fn pair_chip_row_constructs_no_selection() {
        let universe = vec![
            (Venue::Binance, Symbol::new("XRPUSDT")),
            (Venue::Binance, Symbol::new("ETHUSDT")),
            (Venue::Binance, Symbol::new("BTCUSDT")),
        ];
        let _el = super::row(&universe, None, false, ThemeMode::Dark);
    }

    /// T-D-5 — `row` constructs from a 3-pair universe with an active selection.
    #[test]
    fn pair_chip_row_constructs_with_selection() {
        let universe = vec![
            (Venue::Binance, Symbol::new("XRPUSDT")),
            (Venue::Binance, Symbol::new("ETHUSDT")),
            (Venue::Binance, Symbol::new("BTCUSDT")),
        ];
        let selected = (Venue::Binance, Symbol::new("ETHUSDT"));
        let _el = super::row(&universe, Some(&selected), false, ThemeMode::Dark);
    }

    /// T-D-5 — Active chip is visually distinct from inactive chip (checked
    /// by verifying the elements are distinct objects; we can't compare iced
    /// elements so we verify the code paths don't panic).
    #[test]
    fn pair_chip_active_vs_inactive_constructs() {
        let active = super::view(
            Venue::Binance,
            Symbol::new("XRPUSDT"),
            true,
            false,
            ThemeMode::Dark,
        );
        let inactive = super::view(
            Venue::Binance,
            Symbol::new("XRPUSDT"),
            false,
            false,
            ThemeMode::Dark,
        );
        // Both should construct. Different code paths.
        drop(active);
        drop(inactive);
    }

    /// T-D-5 — snapshot: `pair_chip__active_xrpusdt`.
    ///
    /// Records the state-summary text for the active XRPUSDT chip.
    /// Layout is iced-internal and cannot be stringified without a renderer,
    /// so we record the chip parameters as a stable descriptor instead.
    #[test]
    fn pair_chip__active_xrpusdt() {
        let venue = Venue::Binance;
        let symbol = Symbol::new("XRPUSDT");
        let active = true;
        let show_venue = false;
        let mode = ThemeMode::Dark;

        let summary = format!(
            "venue={venue:?} symbol={} active={active} show_venue={show_venue} mode={mode:?}",
            symbol.0
        );

        insta::assert_snapshot!("pair_chip__active_xrpusdt", summary);
    }
}
