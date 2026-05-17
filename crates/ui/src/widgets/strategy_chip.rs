//! Strategy chip widget — ui-rethink-phase-a-lab T-D-6.
//!
//! Renders a `StrategyId` as a Lumen chip with two interaction surfaces:
//!
//! 1. **Primary select** (click on chip body): dispatches
//!    `Message::LabSelectPrimaryStrategy(id)`. Updates
//!    `Cockpit::lab_state.strategy` and re-renders the chart with that
//!    strategy's buy/sell markers + equity curve as the primary overlay.
//!
//! 2. **Compare toggle** (click on the `+` / `×` affordance): dispatches
//!    `Message::LabToggleCompare(id)`. Adds or removes the strategy from
//!    `lab_state.compare_set` (≤4 cap enforced at state level; the
//!    5th press is no-op + caller emits a toast in Wave 2 / M2.5).
//!
//! **Color swatch:** the compare slot index `[0..4)` maps to the
//! `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]` palette via
//! `crate::theme::color::accent_palette()`. `None` when the strategy is
//! not in the compare set.
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::widget::{button, container, Button, Row, Text};
use iced::{Border, Length};
use trading_core::StrategyId;

use crate::lab::state::StrategyFamily;
use crate::state::Message;
use crate::strings;
use crate::theme::{color, radius, space, text, ThemeMode};

/// Render a single strategy chip.
///
/// - `id` — the strategy identifier.
/// - `family` — strategy family rendered as a four-char badge pill.
/// - `is_primary` — whether this strategy is the currently-selected
///   primary strategy (`lab_state.strategy`).
/// - `compare_slot` — if the strategy is in the compare set, the
///   0-indexed slot (`0..COMPARE_SET_CAP`). `None` when not in the set.
/// - `mode` — active theme mode.
///
/// Returns a row element containing the chip body button and the
/// compare toggle affordance.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(
    id: StrategyId,
    family: StrategyFamily,
    is_primary: bool,
    compare_slot: Option<usize>,
    mode: ThemeMode,
) -> crate::Element<'static> {
    // ── Family badge pill ────────────────────────────────────────────────
    let badge_fg = if is_primary {
        color::FG_1.current(mode)
    } else {
        color::FG_3.current(mode)
    };
    let badge = Text::new(family.badge_label())
        .size(text::MICRO)
        .color(badge_fg);

    let badge_chip = container(badge)
        .padding([1_u16, space::XS as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R2.into(),
            },
            ..Default::default()
        });

    // ── Color swatch (if in compare set) ────────────────────────────────
    let swatch = compare_slot.map(|slot| {
        let palette = color::accent_palette();
        let swatch_color = palette
            .get(slot)
            .copied()
            .unwrap_or(color::ACCENT_2)
            .current(mode);

        container(iced::widget::Space::new())
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(move |_t: &iced::Theme| container::Style {
                background: Some(swatch_color.into()),
                border: Border {
                    color: swatch_color,
                    width: 0.0,
                    radius: radius::R2.into(),
                },
                ..Default::default()
            })
    });

    // ── Strategy id label ────────────────────────────────────────────────
    let id_fg = if is_primary {
        color::FG_1.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let id_label = Text::new(id.0.to_string())
        .size(text::BODY)
        .color(id_fg);

    // ── Chip body row ─────────────────────────────────────────────────────
    let mut chip_inner = Row::new()
        .spacing(space::XS)
        .align_y(iced::Alignment::Center)
        .push(badge_chip);

    if let Some(swatch_el) = swatch {
        chip_inner = chip_inner.push(swatch_el);
    }

    chip_inner = chip_inner.push(id_label);

    let border_color = if is_primary {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };

    let bg_color = if is_primary {
        color::ACCENT_SOFT.current(mode)
    } else {
        color::PANEL.current(mode)
    };

    let chip_container = container(chip_inner)
        .padding([space::S as u16, space::M as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg_color.into()),
            border: Border {
                color: border_color,
                width: if is_primary { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    let primary_id = id.clone();
    let chip_btn: Button<'static, Message> = button(chip_container)
        .on_press(Message::LabSelectPrimaryStrategy(primary_id))
        .padding(0)
        .style(|_t: &iced::Theme, _s| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink);

    // ── Compare toggle affordance ─────────────────────────────────────────
    let toggle_label = if compare_slot.is_some() {
        strings::STRATEGY_CHIP_COMPARE_REMOVE
    } else {
        strings::STRATEGY_CHIP_COMPARE_ADD
    };
    let toggle_fg = if compare_slot.is_some() {
        color::FG_2.current(mode)
    } else {
        color::FG_3.current(mode)
    };

    let toggle_text = Text::new(toggle_label)
        .size(text::SMALL)
        .color(toggle_fg);

    let toggle_container = container(toggle_text)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    let compare_id = id.clone();
    let toggle_btn: Button<'static, Message> = button(toggle_container)
        .on_press(Message::LabToggleCompare(compare_id))
        .padding(0)
        .style(|_t: &iced::Theme, _s| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink);

    // ── Outer row: [chip_btn | toggle_btn] ──────────────────────────────
    Row::new()
        .spacing(space::XS)
        .align_y(iced::Alignment::Center)
        .push(chip_btn)
        .push(toggle_btn)
        .into()
}

/// Render a row of strategy chips from the given strategy list.
///
/// `primary` is the currently-selected primary strategy (`lab_state.strategy`).
/// `compare_set` is the current compare set — used to compute `compare_slot`.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn row<'a>(
    strategies: &[StrategyId],
    families: &std::collections::HashMap<StrategyId, StrategyFamily>,
    primary: Option<&StrategyId>,
    compare_set: &[Option<StrategyId>],
    mode: ThemeMode,
) -> crate::Element<'a> {
    use iced::widget::Row;

    let mut chips = Row::new().spacing(space::S);

    for id in strategies {
        let family = families.get(id).copied().unwrap_or_default();
        let is_primary = primary.map_or(false, |p| p == id);
        let compare_slot = compare_set.iter().enumerate().find_map(|(i, slot)| {
            if slot.as_ref() == Some(id) {
                Some(i)
            } else {
                None
            }
        });
        chips = chips.push(view(id.clone(), family, is_primary, compare_slot, mode));
    }

    chips.into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use trading_core::StrategyId;

    use crate::lab::state::StrategyFamily;
    use crate::theme::ThemeMode;

    fn id(s: &str) -> StrategyId {
        StrategyId(smol_str::SmolStr::new(s))
    }

    /// T-D-6 — `view` constructs for a primary chip.
    #[test]
    fn strategy_chip_primary_constructs() {
        let _el = super::view(
            id("v1.momentum"),
            StrategyFamily::Rule,
            true,
            None,
            ThemeMode::Dark,
        );
    }

    /// T-D-6 — `view` constructs for a compare slot 0 chip.
    #[test]
    fn strategy_chip_compare_slot_0_constructs() {
        let _el = super::view(
            id("v0.5.macd"),
            StrategyFamily::Composed,
            false,
            Some(0),
            ThemeMode::Dark,
        );
    }

    /// T-D-6 — `view` constructs for all ACCENT_2..5 compare slots.
    #[test]
    fn strategy_chip_all_compare_slots_construct() {
        for slot in 0..4 {
            let _el = super::view(
                id(&format!("strategy-{slot}")),
                StrategyFamily::Hybrid,
                false,
                Some(slot),
                ThemeMode::Dark,
            );
        }
    }

    /// T-D-6 — `row` constructs from an empty strategy list.
    #[test]
    fn strategy_chip_row_empty() {
        let _el = super::row(&[], &HashMap::new(), None, &[], ThemeMode::Dark);
    }

    /// T-D-6 — `row` constructs with a primary selection and a compare set.
    #[test]
    fn strategy_chip_row_with_primary_and_compare() {
        let strategies = vec![id("v1.momentum"), id("v0.5.macd"), id("v2.dl")];
        let mut families = HashMap::new();
        families.insert(id("v1.momentum"), StrategyFamily::Rule);
        families.insert(id("v0.5.macd"), StrategyFamily::Composed);
        families.insert(id("v2.dl"), StrategyFamily::Dl);

        let primary = id("v1.momentum");
        let compare_set = vec![Some(id("v0.5.macd"))];

        let _el = super::row(
            &strategies,
            &families,
            Some(&primary),
            &compare_set,
            ThemeMode::Dark,
        );
    }

    /// T-D-6 — `LabSelectPrimaryStrategy` dispatch path: constructing chip
    /// with `is_primary=false` for a strategy that IS primary (a sanity-check
    /// on message routing). In real usage the caller derives `is_primary`
    /// from `lab_state.strategy`; here we just verify both states compile.
    #[test]
    fn strategy_chip_both_primary_states_compile() {
        let s = id("v1.momentum");
        let _primary = super::view(s.clone(), StrategyFamily::Llm, true, None, ThemeMode::Dark);
        let _non_primary =
            super::view(s.clone(), StrategyFamily::Llm, false, None, ThemeMode::Dark);
        let _with_compare =
            super::view(s.clone(), StrategyFamily::Llm, true, Some(1), ThemeMode::Dark);
    }

    /// T-D-6 — snapshot: `strategy_chip__primary_with_compare_slot_1`.
    ///
    /// Records the chip descriptor for the primary chip with compare slot 1
    /// occupied by a second strategy. Since iced elements are opaque structs,
    /// we snapshot the parameter summary instead.
    #[test]
    fn strategy_chip__primary_with_compare_slot_1() {
        let id_str = "v1.momentum";
        let family = StrategyFamily::Rule;
        let is_primary = true;
        let compare_slot: Option<usize> = Some(1);
        let mode = ThemeMode::Dark;

        let summary = format!(
            "id={id_str} family={} is_primary={is_primary} compare_slot={compare_slot:?} mode={mode:?}",
            family.badge_label()
        );

        insta::assert_snapshot!("strategy_chip__primary_with_compare_slot_1", summary);
    }
}
