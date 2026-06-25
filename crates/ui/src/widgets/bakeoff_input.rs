//! Guided bake-off input widget — advisor-bakeoff-ranking F3 + tuning knobs.
//!
//! The entry point to the single-coin investment-advisor journey (product §
//! journey step 1: "pick a coin (e.g. XRPUSD) and a budget (e.g. €200)" over a
//! configurable lookback "2 weeks → ~4 years"). Renders, above the leaderboard
//! table, a compact form with five control groups:
//!
//! ```text
//! ┌─ Plan your bake-off ─────────────────────────────────────────────┐
//! │ Coin                                                              │
//! │ [XRPUSDT] [ETHUSDT] [BTCUSDT*] [ADAUSDT] [AVAXUSDT] [BNBUSDT] …   │  ← chips
//! │ Budget          Lookback                                          │
//! │ [ 200 ]         [2 weeks] [1 month] [3 months] … [2024 H1*] …     │
//! │ €200 ≈ 200 USDT — FX not modelled.                                │  ← hint
//! │ Bar size (changes ranking)     Start capital (USDT)               │
//! │ [H1*] [H4] [D1]               [ 100000 ]                          │  ← tuning
//! │ Does not affect ranking …                                          │  ← honest note
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - **Coin picker** — Lumen chips over the corpus-covered coin universe
//!   (`leaderboard::BAKEOFF_COIN_UNIVERSE`), XRP-first; the active coin gets the
//!   `ACCENT` chip treatment (same discipline as `pair_chip`). Each dispatches
//!   `Message::BakeoffSelectCoin(Symbol)`.
//! - **Budget field** — a numeric `text_input` defaulting to `200`, dispatching
//!   `Message::BakeoffSetBudget(String)`. The bake-off RANKING is
//!   budget-independent; the budget carries forward to F4/F5 sizing and is shown
//!   in the leaderboard header for context. The €/USDT 1:1 assumption is stated
//!   under the field (product § D4 — "FX not modelled").
//! - **Lookback picker** — Lumen chips over `LeaderboardLookback::ALL`
//!   (2 weeks → 4 years + the 2024 presets); each dispatches
//!   `Message::BakeoffSelectLookback`.
//! - **Bar-size (timeframe) picker** — H1 / H4 / D1 chips; each dispatches
//!   `Message::BakeoffSelectTimeframe`. **Affects ranking**: different bar size
//!   can crown a different strategy.
//! - **Start-capital field** — a numeric `text_input` defaulting to `100000`,
//!   dispatching `Message::BakeoffSetStartCapital(String)`. Does NOT affect
//!   ranking (all arms run with the same capital); scales absolute equity + sizing.
//!
//! Built from existing widgets + tokens only — NO new theme token, NO new
//! widget primitive (the chip is a `Container` + `button`, exactly the
//! `pair_chip` shape). **Zero hex colours** — tokens via `crate::theme`.
//! **Zero string literals** — copy via `crate::strings`.

// The `space::* as u16` / `as f32` layout casts are bounded + safe (mirrors the
// per-module allow-pattern in `pair_chip.rs` / `screens/leaderboard.rs`).
#![allow(clippy::cast_possible_truncation)]

use iced::widget::{Column, Container, Row, Text, button, container, text_input};
use iced::{Border, Length};
use rust_decimal::Decimal;
use trading_core::{BudgetConversion, FxRate, Symbol};

use crate::leaderboard::{BakeoffTimeframe, LeaderboardLookback};
use crate::state::Message;
use crate::strings::{
    LEADERBOARD_BUDGET_HINT_FMT, LEADERBOARD_BUDGET_LABEL, LEADERBOARD_BUDGET_PLACEHOLDER,
    LEADERBOARD_CAPITAL_HINT, LEADERBOARD_CAPITAL_LABEL, LEADERBOARD_CAPITAL_PLACEHOLDER,
    LEADERBOARD_COIN_LABEL, LEADERBOARD_LOOKBACK_LABEL, LEADERBOARD_TIMEFRAME_LABEL,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::frame;

/// Fixed width of the budget text field — wide enough for "€200" / "199.50"
/// at `BODY` size, not so wide it dwarfs the lookback chips beside it. A local
/// layout constant (the same kind as the Lab's SMA-input `Fixed(160.0)`), not a
/// design token.
const BUDGET_FIELD_WIDTH: f32 = 120.0;

/// Fixed width of the start-capital text field — wide enough for "100000" /
/// "50000.00" at `BODY` size. Same kind of local layout constant as
/// `BUDGET_FIELD_WIDTH`.
const CAPITAL_FIELD_WIDTH: f32 = 140.0;

/// Render the guided bake-off input form.
///
/// - `coin` — the currently-selected coin (drives the active-chip highlight).
/// - `budget_input` — the raw budget text (round-trips the operator's
///   keystrokes; rendered verbatim into the field).
/// - `lookback` — the currently-selected lookback (drives the active chip).
/// - `timeframe` — the currently-selected bar size (H1 / H4 / D1); drives the
///   active timeframe chip. **Affects ranking** — different bar size can crown
///   a different strategy.
/// - `start_capital_input` — the raw start-capital text (round-trips keystrokes).
///   Does NOT affect ranking; scales absolute equity + forward sizing.
/// - `eur_usd_rate` — the EUR/USD rate to use for the honest FX hint
///   (F7 / ADR-0065). Pass `trading_core::DEFAULT_EUR_USD_RATE` when no
///   operator override is configured.
/// - `mode` — active theme mode.
///
/// Returns the form as a titled `frame::panel` so it reads as one coherent
/// "plan your run" surface above the table.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    coin: &Symbol,
    budget_input: &str,
    lookback: LeaderboardLookback,
    timeframe: BakeoffTimeframe,
    start_capital_input: &str,
    eur_usd_rate: Decimal,
    mode: ThemeMode,
) -> crate::Element<'a> {
    // ── Coin row ──────────────────────────────────────────────────────────────
    let coin_block = Column::new()
        .spacing(space::XS)
        .push(field_label(LEADERBOARD_COIN_LABEL, mode))
        .push(coin_row(coin, mode));

    // ── Budget + lookback row (side by side) ──────────────────────────────────
    let budget_block = Column::new()
        .spacing(space::XS)
        .push(field_label(LEADERBOARD_BUDGET_LABEL, mode))
        .push(budget_field(budget_input, mode))
        .width(Length::Shrink);

    let lookback_block = Column::new()
        .spacing(space::XS)
        .push(field_label(LEADERBOARD_LOOKBACK_LABEL, mode))
        .push(lookback_row(lookback, mode))
        .width(Length::Fill);

    let budget_lookback_row = Row::new()
        .spacing(space::L)
        .align_y(iced::alignment::Vertical::Top)
        .push(budget_block)
        .push(lookback_block)
        .width(Length::Fill);

    // ── Budget hint (F7 / ADR-0065 — honest EUR→USDT FX note) ────────────────
    // Parse the budget_input to build a BudgetConversion; fall back to 200
    // when blank / unparseable. The conversion is used only for display here —
    // the engine uses the same BudgetConversion built at the seam in cockpit_live.rs.
    let hint_text: String = {
        use crate::widgets::num::{fmt_eur_plain, fmt_rate, fmt_usdt_plain};
        use rust_decimal_macros::dec;
        let eur: Decimal =
            crate::leaderboard::state::parse_budget(budget_input).unwrap_or(dec!(200));
        let fx = FxRate::config(eur_usd_rate);
        let conv = BudgetConversion::new(eur, fx);
        LEADERBOARD_BUDGET_HINT_FMT
            .replace("{eur}", &fmt_eur_plain(conv.eur()))
            .replace("{usdt}", &fmt_usdt_plain(conv.usdt().amount()))
            .replace("{rate}", &fmt_rate(conv.rate().rate()))
            .replace("{source}", conv.rate().source())
    };
    let hint = Text::new(hint_text)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    // ── Timeframe + start-capital row (side by side) ──────────────────────────
    let timeframe_block = Column::new()
        .spacing(space::XS)
        .push(field_label(LEADERBOARD_TIMEFRAME_LABEL, mode))
        .push(timeframe_row(timeframe, mode))
        .width(Length::Shrink);

    let capital_block = Column::new()
        .spacing(space::XS)
        .push(field_label(LEADERBOARD_CAPITAL_LABEL, mode))
        .push(capital_field(start_capital_input, mode))
        .width(Length::Shrink);

    let timeframe_capital_row = Row::new()
        .spacing(space::L)
        .align_y(iced::alignment::Vertical::Top)
        .push(timeframe_block)
        .push(capital_block)
        .width(Length::Fill);

    // Honest note under the capital field (does not affect ranking).
    let capital_hint = Text::new(LEADERBOARD_CAPITAL_HINT)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    let body = Column::new()
        .spacing(space::M)
        .push(coin_block)
        .push(budget_lookback_row)
        .push(hint)
        .push(timeframe_capital_row)
        .push(capital_hint)
        .width(Length::Fill);

    frame::panel(crate::strings::LEADERBOARD_PLAN_TITLE, body.into(), mode)
}

/// A field label — `MICRO` muted, the same convention the table column headers
/// use (a quiet caption above each control).
fn field_label(label: &'static str, mode: ThemeMode) -> crate::Element<'static> {
    Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(mode))
        .into()
}

/// The coin-chip row over the corpus-covered universe, XRP-first. The
/// `selected` coin gets the `ACCENT` chip treatment. Wrapped in a `Wrap`-style
/// `Row` (chips flow; the universe is 10 wide).
fn coin_row<'a>(selected: &Symbol, mode: ThemeMode) -> crate::Element<'a> {
    let mut row = Row::new().spacing(space::S);
    for &sym_str in crate::leaderboard::BAKEOFF_COIN_UNIVERSE {
        let active = selected.0.as_str() == sym_str;
        row = row.push(coin_chip(sym_str, active, mode));
    }
    row.width(Length::Fill).into()
}

/// A single coin chip — `Container` + transparent `button`, exactly the
/// `pair_chip` shape (active = `ACCENT_SOFT` fill + `ACCENT` 1.5 px border +
/// `FG_1`; inactive = `PANEL` fill + `BORDER_1` + `FG_2`). Dispatches
/// `Message::BakeoffSelectCoin(Symbol)`.
fn coin_chip<'a>(sym_str: &str, active: bool, mode: ThemeMode) -> crate::Element<'a> {
    // Active chip = SOLID `ACCENT` fill + `FG_ON_ACCENT` text (the same strong,
    // accessible "selected" treatment as the source-toggle chip + the Run
    // button) so the chosen coin/lookback pops at a glance and reads beyond
    // colour. Inactive = `PANEL` fill + `BORDER_1` hairline + `FG_2` text.
    let fg = if active {
        color::FG_ON_ACCENT.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };
    let bg_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::PANEL.current(mode)
    };

    let label = Text::new(sym_str.to_string()).size(text::SMALL).color(fg);

    let chip = Container::new(label)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg_color.into()),
            border: Border {
                color: border_color,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    let symbol = Symbol::new(sym_str);
    button(chip)
        .on_press(Message::BakeoffSelectCoin(symbol))
        .padding(0)
        .style(|_t: &iced::Theme, _s: button::Status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// The budget `text_input` — a fixed-width numeric field at `BODY` size,
/// dispatching `Message::BakeoffSetBudget(String)` on every keystroke. The
/// placeholder shows the default (`200`) so the field never looks broken when
/// empty.
fn budget_field<'a>(budget_input: &str, mode: ThemeMode) -> crate::Element<'a> {
    text_input(LEADERBOARD_BUDGET_PLACEHOLDER, budget_input)
        .on_input(Message::BakeoffSetBudget)
        .size(text::BODY)
        .padding([space::XS as u16, space::S as u16])
        .width(Length::Fixed(BUDGET_FIELD_WIDTH))
        .style(
            move |_t: &iced::Theme, _s: text_input::Status| text_input::Style {
                background: color::PANEL.current(mode).into(),
                border: Border {
                    color: color::BORDER_1.current(mode),
                    width: 1.0,
                    radius: radius::R4.into(),
                },
                icon: color::FG_3.current(mode),
                placeholder: color::FG_3.current(mode),
                value: color::FG_1.current(mode),
                selection: color::ACCENT_SOFT.current(mode),
            },
        )
        .into()
}

/// The lookback-chip row over `LeaderboardLookback::ALL` (2 weeks → 4 years +
/// the 2024 presets). The active lookback gets the `ACCENT` chip treatment.
fn lookback_row<'a>(selected: LeaderboardLookback, mode: ThemeMode) -> crate::Element<'a> {
    let mut row = Row::new().spacing(space::S);
    for &lb in LeaderboardLookback::ALL {
        row = row.push(lookback_chip(lb, lb == selected, mode));
    }
    row.width(Length::Fill).into()
}

/// A single lookback chip — same chip shape as the coin chip. Dispatches
/// `Message::BakeoffSelectLookback(LeaderboardLookback)`.
fn lookback_chip<'a>(
    lookback: LeaderboardLookback,
    active: bool,
    mode: ThemeMode,
) -> crate::Element<'a> {
    // Active chip = SOLID `ACCENT` fill + `FG_ON_ACCENT` text (the same strong,
    // accessible "selected" treatment as the source-toggle chip + the Run
    // button) so the chosen coin/lookback pops at a glance and reads beyond
    // colour. Inactive = `PANEL` fill + `BORDER_1` hairline + `FG_2` text.
    let fg = if active {
        color::FG_ON_ACCENT.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };
    let bg_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::PANEL.current(mode)
    };

    let label = Text::new(lookback_label(lookback))
        .size(text::SMALL)
        .color(fg);

    let chip = Container::new(label)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg_color.into()),
            border: Border {
                color: border_color,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    button(chip)
        .on_press(Message::BakeoffSelectLookback(lookback))
        .padding(0)
        .style(|_t: &iced::Theme, _s: button::Status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// Map a `LeaderboardLookback` to its chip copy (all in `strings`).
fn lookback_label(lookback: LeaderboardLookback) -> &'static str {
    use crate::strings::{
        LEADERBOARD_LOOKBACK_1M, LEADERBOARD_LOOKBACK_1Y, LEADERBOARD_LOOKBACK_2W,
        LEADERBOARD_LOOKBACK_2Y, LEADERBOARD_LOOKBACK_3M, LEADERBOARD_LOOKBACK_4Y,
        LEADERBOARD_LOOKBACK_6M, LEADERBOARD_LOOKBACK_H1_2024, LEADERBOARD_LOOKBACK_H2_2024,
    };
    match lookback {
        LeaderboardLookback::TwoWeeks => LEADERBOARD_LOOKBACK_2W,
        LeaderboardLookback::OneMonth => LEADERBOARD_LOOKBACK_1M,
        LeaderboardLookback::ThreeMonths => LEADERBOARD_LOOKBACK_3M,
        LeaderboardLookback::SixMonths => LEADERBOARD_LOOKBACK_6M,
        LeaderboardLookback::OneYear => LEADERBOARD_LOOKBACK_1Y,
        LeaderboardLookback::TwoYears => LEADERBOARD_LOOKBACK_2Y,
        LeaderboardLookback::FourYears => LEADERBOARD_LOOKBACK_4Y,
        LeaderboardLookback::H1_2024 => LEADERBOARD_LOOKBACK_H1_2024,
        LeaderboardLookback::H2_2024 => LEADERBOARD_LOOKBACK_H2_2024,
    }
}

/// Public lookback→label mapping so the screen header can reuse the SAME copy
/// the chips use (single source of truth for the human window name).
#[must_use]
pub fn lookback_copy(lookback: LeaderboardLookback) -> &'static str {
    lookback_label(lookback)
}

/// The timeframe-chip row over `BakeoffTimeframe::ALL` (H1 / H4 / D1). The
/// active timeframe gets the `ACCENT` chip treatment.
fn timeframe_row<'a>(selected: BakeoffTimeframe, mode: ThemeMode) -> crate::Element<'a> {
    let mut row = Row::new().spacing(space::S);
    for &tf in BakeoffTimeframe::ALL {
        row = row.push(timeframe_chip(tf, tf == selected, mode));
    }
    row.width(Length::Shrink).into()
}

/// A single timeframe chip — same chip shape as coin / lookback chips.
/// Dispatches `Message::BakeoffSelectTimeframe(BakeoffTimeframe)`.
fn timeframe_chip<'a>(tf: BakeoffTimeframe, active: bool, mode: ThemeMode) -> crate::Element<'a> {
    let fg = if active {
        color::FG_ON_ACCENT.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };
    let bg_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::PANEL.current(mode)
    };

    let label = Text::new(tf.chip_label()).size(text::SMALL).color(fg);

    let chip = Container::new(label)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg_color.into()),
            border: Border {
                color: border_color,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    button(chip)
        .on_press(Message::BakeoffSelectTimeframe(tf))
        .padding(0)
        .style(|_t: &iced::Theme, _s: button::Status| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// The start-capital `text_input` — a fixed-width numeric field at `BODY` size,
/// dispatching `Message::BakeoffSetStartCapital(String)` on every keystroke.
/// The placeholder shows the legacy default (`100000`) so the field never
/// looks broken when empty.
fn capital_field<'a>(start_capital_input: &str, mode: ThemeMode) -> crate::Element<'a> {
    text_input(LEADERBOARD_CAPITAL_PLACEHOLDER, start_capital_input)
        .on_input(Message::BakeoffSetStartCapital)
        .size(text::BODY)
        .padding([space::XS as u16, space::S as u16])
        .width(Length::Fixed(CAPITAL_FIELD_WIDTH))
        .style(
            move |_t: &iced::Theme, _s: text_input::Status| text_input::Style {
                background: color::PANEL.current(mode).into(),
                border: Border {
                    color: color::BORDER_1.current(mode),
                    width: 1.0,
                    radius: radius::R4.into(),
                },
                icon: color::FG_3.current(mode),
                placeholder: color::FG_3.current(mode),
                value: color::FG_1.current(mode),
                selection: color::ACCENT_SOFT.current(mode),
            },
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leaderboard::BakeoffTimeframe;

    /// Every lookback maps to a non-empty chip label (no panic, no blank chip).
    #[test]
    fn every_lookback_has_a_label() {
        for &lb in LeaderboardLookback::ALL {
            assert!(
                !lookback_label(lb).is_empty(),
                "lookback {lb:?} must have a chip label"
            );
        }
    }

    /// Every timeframe maps to a non-empty chip label (no panic, no blank chip).
    #[test]
    fn every_timeframe_has_a_chip_label() {
        for &tf in BakeoffTimeframe::ALL {
            assert!(
                !tf.chip_label().is_empty(),
                "timeframe {tf:?} must have a chip label"
            );
        }
    }

    /// `view` constructs without panic at both theme modes for a representative
    /// selection (smoke — the render-layer proof is the screenshot harness).
    #[test]
    fn view_constructs_both_modes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let _ = view(
                &Symbol::new("XRPUSDT"),
                "200",
                LeaderboardLookback::OneMonth,
                BakeoffTimeframe::OneHour,
                "100000",
                trading_core::DEFAULT_EUR_USD_RATE,
                mode,
            );
        }
    }

    /// `view` constructs with all three timeframe variants (smoke).
    #[test]
    fn view_constructs_all_timeframes() {
        for &tf in BakeoffTimeframe::ALL {
            let _ = view(
                &Symbol::new("BTCUSDT"),
                "200",
                LeaderboardLookback::H1_2024,
                tf,
                "50000",
                trading_core::DEFAULT_EUR_USD_RATE,
                ThemeMode::Dark,
            );
        }
    }

    /// `lookback_copy` is the same string the chip uses (single source).
    #[test]
    fn lookback_copy_matches_chip_label() {
        for &lb in LeaderboardLookback::ALL {
            assert_eq!(lookback_copy(lb), lookback_label(lb));
        }
    }
}
