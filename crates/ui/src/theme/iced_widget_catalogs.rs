//! Iced-widget Catalog adapters — the hub for routing the cockpit's
//! design tokens into iced's per-widget `Catalog` trait surface.
//!
//! ## Background — Q3-sub (refinement pass 2026-05-13)
//!
//! The architect's design pass on `iced-native-widgets v0.1.0` (Brief A)
//! landed on **option (b)** for table styling: route the cockpit's
//! `BORDER_1` / `PANEL_SUNKEN` tokens into `iced::widget::table::Style`
//! via the Catalog system, in a NEW submodule that future Brief B
//! `iced_aw` adoptions can extend.
//!
//! ## Orphan-rule constraint
//!
//! `iced::widget::table::Catalog` is **already implemented upstream** for
//! `iced::Theme` (a re-export of `iced_widget::Theme` →
//! `iced_core::theme::Theme`) at `iced_widget-0.14.2/src/table.rs:704-714`.
//! Both the trait and the type are foreign to the `ui` crate, so a
//! second `impl Catalog for iced::Theme` here would violate Rust's
//! orphan rules and would conflict with the upstream impl regardless.
//!
//! ## What this module provides
//!
//! Rather than a foreign trait impl, this module exposes the cockpit's
//! **house style functions** that mint a `table::StyleFn` (the
//! `Box<dyn Fn(&Theme) -> Style + 'a>` shape the Catalog's
//! `Class<'a>` alias resolves to). The upstream Catalog impl's
//! `default()` returns iced's stock palette-derived separator color;
//! `cockpit_table_style_fn` returns our cockpit's `BORDER_1` token
//! instead so the visual feel matches the rest of the panel chrome.
//!
//! Native `iced::widget::table::Table::new(...)` v0.14 does **not**
//! expose a `.style(...)` builder — the upstream impl pre-bakes
//! `Theme::default()` at construction time. Until iced ships a
//! `Table::style(StyleFn)` setter (tracked upstream; not in 0.14), the
//! style-fn defined here is consumed by:
//!
//! 1. **Brief B `iced_aw` adopters** — any future widget that accepts a
//!    `StyleFn<Theme>` builder for table-like surfaces.
//! 2. **Test scaffolding / docs** — call sites that need to render
//!    explicit `Style` snapshots for visual regression.
//! 3. **Themer overrides** — wrapping a `Table` in `iced::widget::Themer`
//!    with this style-fn substitutes the table's class for a
//!    cockpit-tinted variant without modifying upstream.
//!
//! The function returns a static `Style` — no per-status branching is
//! needed at this level (Table's `Style` only carries `separator_x` /
//! `separator_y`; the selected-row 2 px ACCENT left-rule lives in the
//! per-cell `Element` border helpers, not in the table-level style).
//!
//! ## Future-proofing — Brief B
//!
//! When `iced_aw` is adopted in Brief B, additional Catalog adapters
//! (e.g. for `iced_aw::table`, `iced_aw::number_input`, etc.) land in
//! this module as sibling functions. Keeping them centralized prevents
//! drift between the iced-native and iced_aw chrome.

use iced::widget::table::{Style, StyleFn};
use iced::{Background, Color, Theme};

use super::color::{self};
use super::{radius, ThemeMode};

/// Domain-level "what does this status say" classification for status
/// pills. Independent of `iced_aw::style::Status` (which is interaction
/// state — hovered/pressed/disabled). The caller maps its own enum
/// (e.g. `state::StrategyStatus`) into `BadgeIntent` at the use site so
/// `theme` does not depend on `state` and so future surfaces (risk
/// thresholds, latency bands, fill-health pills) can route the same
/// three-band palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeIntent {
    /// "All good." Sage / `UP_*` ramp. Strategy `Ready`, latency `OK`,
    /// risk-band `Healthy`.
    Positive,
    /// "Working on it / nothing to report." Muted accent tint. Strategy
    /// `Loading`, latency `WARN` lower-end, empty-with-pending states.
    Neutral,
    /// "Something is wrong." Clay / `DOWN_*` ramp. Strategy `Error`,
    /// latency `HIGH` / `HALTED`, risk-band `Tripped`.
    Negative,
}

/// Returns the cockpit's house style for `iced::widget::table::Table`.
///
/// The function shape matches `iced::widget::table::default` so it
/// drops in as a `Class<'a>` payload via `cockpit_table_style_fn`.
///
/// Tokens routed:
/// - `separator_x` / `separator_y` → `color::BORDER_1` (the hairline
///   separator used between every panel chrome boundary; see
///   [`crate::theme::color::BORDER_1`]).
///
/// `ThemeMode::Dark` is hard-pinned per the cockpit cold-start contract
/// (every other widget style closure does the same — see
/// `widgets::frame::active_row` at `frame.rs:127`). When a runtime
/// theme toggle lands, this function flips to `current(mode)` against
/// the `Theme` argument.
#[must_use]
pub fn cockpit_table_style(_theme: &Theme) -> Style {
    let separator: Background = color::BORDER_1.current(ThemeMode::Dark).into();
    Style {
        separator_x: separator,
        separator_y: separator,
    }
}

/// Returns a boxed [`StyleFn`] wrapping [`cockpit_table_style`].
///
/// The `'a` lifetime is unconstrained because the underlying function
/// is `fn`-pointer-callable (no captured borrows). The boxed shape
/// matches `iced::widget::table::Catalog::Class<'a>` so call sites can
/// substitute this for the stock `Theme::default()` Class when wrapping
/// a `Table` in `iced::widget::Themer`.
#[must_use]
pub fn cockpit_table_style_fn<'a>() -> StyleFn<'a, Theme> {
    Box::new(cockpit_table_style)
}

// ── Brief B (`iced-aw-cherry-pick`, T-M3-1) — badge Catalog adapter ──
//
// `iced_aw::Badge` v0.14 exposes a `.style(impl Fn(&Theme, Status) ->
// badge::Style + 'a)` builder (badge.rs:115-119). The `Status` enum is
// **interaction state** (Active / Hovered / Pressed / Disabled /
// Focused / Selected — see iced_aw `style/status.rs`), NOT domain
// status. The Strategies STATUS column (`crates/ui/src/widgets/
// strategies.rs:113-129`) needs different colours per
// `StrategyStatus::{Ready, Loading, Error}` row, so the closure has to
// capture the domain band by value.
//
// Shape: a factory `cockpit_badge_style_fn(intent)` returns the boxed
// closure that iced_aw consumes; the closure body picks a base palette
// from the captured `BadgeIntent` (this file's enum — see top of
// module) and layers interaction-state modifiers on top of that base
// (alpha-scale on `Disabled`, base on every other variant).
//
// Palette choices (cited in the body comments below) follow the
// Lumen "soft tint backdrop + saturated foreground label" pattern
// from `spec/ui-design-principles.md ## Status pill colors`. The
// 50-step tint surfaces (`UP_50`, `DOWN_50`, `ACCENT_SOFT`) give a
// muted backdrop so a column of all-Ready strategies reads as a calm
// row of pills instead of a wall of saturated sage; the 500-step
// foreground keeps the **label** as the high-contrast signal — same
// semantic weight the pre-Brief-B `colored_cell(label, UP_500)`
// pattern carried. Visual continuity at the snapshot byte level: the
// label colour is unchanged across the upgrade, the new bytes are
// only the backdrop fill + radius.

/// Returns the cockpit's house style for `iced_aw::Badge`, given a
/// domain-level [`BadgeIntent`].
///
/// `ThemeMode::Dark` is hard-pinned per the cockpit cold-start
/// contract (matches `cockpit_table_style` above and every widget
/// style closure in the crate — see `widgets::frame::active_row` at
/// `frame.rs:127`). When a runtime theme toggle lands, the
/// `current(mode)` calls below flip to a `mode: ThemeMode`
/// parameter threaded from the caller.
///
/// The `iced_aw::style::Status` argument is interaction state — the
/// domain colour is fixed by `intent`, so this function only branches
/// `Status` to dim on `Disabled` (matching `iced_aw`'s stock
/// `disabled()` helper at `style/badge.rs:200-206`). Hover / pressed
/// / focused / selected fall through to the base palette: a status
/// pill is informational, not interactive, and a mouse hover should
/// not falsely imply a clickable target.
#[must_use]
pub fn cockpit_badge_style(
    _theme: &Theme,
    status: iced_aw::style::Status,
    intent: BadgeIntent,
) -> iced_aw::style::badge::Style {
    let mode = ThemeMode::Dark;
    let (background, text_color): (Color, Color) = match intent {
        // Positive — sage tint backdrop, saturated sage label. Matches
        // P&L `UP_500` rule (`ui-design-principles.md ## P&L coloring`)
        // and preserves the pre-badge `colored_cell(label, UP_500)`
        // foreground so the snapshot label byte stays identical.
        BadgeIntent::Positive => (color::UP_50.current(mode), color::UP_500.current(mode)),
        // Neutral — accent-soft chip backdrop (the canonical chip fill
        // per `ui-design-principles.md ## Color tokens`), muted `FG_3`
        // label. Same `FG_3` the pre-badge Loading variant rendered
        // at the text-colour level, kept here so the upgrade is purely
        // additive (backdrop + radius) rather than a colour swap.
        BadgeIntent::Neutral => (color::ACCENT_SOFT.current(mode), color::FG_3.current(mode)),
        // Negative — clay tint backdrop, saturated clay label.
        // Mirrors the Positive case across the P&L pair so
        // operators read the column as the same green/red mental
        // model the rest of the cockpit trains.
        BadgeIntent::Negative => (color::DOWN_50.current(mode), color::DOWN_500.current(mode)),
    };

    let base = iced_aw::style::badge::Style {
        background: Background::Color(background),
        // PILL radius — status pills follow the "tag" pattern
        // (`ui-design-principles.md ## Radii`: `PILL=999px` for tags,
        // toggle thumbs, status-bar dots) rather than the `R3=6px`
        // chip-button shape. A status indicator is a category label,
        // not a clickable control.
        border_radius: Some(radius::PILL),
        // Backdrop carries the visual edge; no separate stroke. iced_aw
        // stock badges default to a 1 px border (`style/badge.rs:43`);
        // we override to 0 because the soft-tint backdrop already
        // separates from the panel chrome at the alpha we picked.
        border_width: 0.0,
        border_color: None,
        text_color,
    };

    match status {
        // Disabled — alpha-scale 0.5 on both axes, mirroring iced_aw's
        // stock `disabled()` helper at `style/badge.rs:200-206`. The
        // strategy-status pill never actually goes through a Disabled
        // event in v0.1 (badges are informational, not pressable), but
        // matching the upstream contract keeps future surfaces (e.g.
        // a disabled-strategies filter pill) drop-in compatible.
        iced_aw::style::Status::Disabled => iced_aw::style::badge::Style {
            background: base.background.scale_alpha(0.5),
            text_color: base.text_color.scale_alpha(0.5),
            ..base
        },
        // Active / Hovered / Pressed / Focused / Selected → base. Status
        // pills are informational, not interactive; a mouse hover or
        // keyboard focus should not change the colour and falsely imply
        // a clickable target.
        iced_aw::style::Status::Active
        | iced_aw::style::Status::Hovered
        | iced_aw::style::Status::Pressed
        | iced_aw::style::Status::Focused
        | iced_aw::style::Status::Selected => base,
    }
}

/// Returns a boxed style-fn for `iced_aw::Badge::style(...)`, baking
/// the domain [`BadgeIntent`] into the closure.
///
/// The return type uses `iced_aw::style::StyleFn` (which is itself
/// `Box<dyn Fn(&Theme, Status) -> Style + 'a>` — see
/// `iced_aw/src/style/status.rs:21`). Call sites pass the returned
/// value straight to
/// `Badge::new(label).style(cockpit_badge_style_fn(intent))` without
/// an adapter shim — the `Badge::style(impl Fn(...) -> Style)`
/// builder at `iced_aw/src/widget/badge.rs:115-119` accepts both
/// shapes via the `Theme::Class<'a>: From<StyleFn<...>>` bound.
#[must_use]
pub fn cockpit_badge_style_fn<'a>(
    intent: BadgeIntent,
) -> iced_aw::style::StyleFn<'a, Theme, iced_aw::style::badge::Style> {
    Box::new(move |theme, status| cockpit_badge_style(theme, status, intent))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time guarantee that the style-fn signature matches the
    /// upstream Catalog's `Class<'a>` alias. If iced's `StyleFn`
    /// signature ever changes, this test refuses to compile and the
    /// adapter migrates in lockstep.
    #[test]
    fn cockpit_table_style_fn_is_a_valid_style_fn() {
        let _: StyleFn<'_, Theme> = cockpit_table_style_fn();
    }

    /// The house style uses `BORDER_1` for both axes — same hairline
    /// the rest of the panel chrome draws.
    #[test]
    fn cockpit_table_style_separators_match_border_1() {
        let style = cockpit_table_style(&Theme::Dark);
        let expected: Background = color::BORDER_1.current(ThemeMode::Dark).into();
        assert_eq!(format!("{:?}", style.separator_x), format!("{expected:?}"));
        assert_eq!(format!("{:?}", style.separator_y), format!("{expected:?}"));
    }

    // ── Brief B (T-M3-1) — badge Catalog adapter tests ─────────────

    /// Compile-time guarantee that the badge style-fn signature matches
    /// `iced_aw`'s `StyleFn` alias. If iced_aw bumps the closure shape
    /// in a future minor, this test refuses to compile and the adapter
    /// migrates in lockstep — same gate the table adapter uses above.
    #[test]
    fn cockpit_badge_style_fn_is_a_valid_style_fn() {
        let _: iced_aw::style::StyleFn<'_, Theme, iced_aw::style::badge::Style> =
            cockpit_badge_style_fn(BadgeIntent::Positive);
    }

    /// Each `BadgeIntent` routes to the documented Lumen palette pair.
    /// Asserted via debug-formatted equality so a token rename in
    /// `theme::color` surfaces here rather than at the snapshot byte
    /// level — same pattern as the table adapter test above.
    #[test]
    fn cockpit_badge_style_routes_lumen_tokens_for_each_intent() {
        let theme = Theme::Dark;
        let mode = ThemeMode::Dark;

        let positive = cockpit_badge_style(
            &theme,
            iced_aw::style::Status::Active,
            BadgeIntent::Positive,
        );
        let positive_bg: Background = color::UP_50.current(mode).into();
        let positive_fg = color::UP_500.current(mode);
        assert_eq!(
            format!("{:?}", positive.background),
            format!("{positive_bg:?}"),
            "Positive intent → UP_50 backdrop",
        );
        assert_eq!(
            format!("{:?}", positive.text_color),
            format!("{positive_fg:?}"),
            "Positive intent → UP_500 label",
        );

        let neutral =
            cockpit_badge_style(&theme, iced_aw::style::Status::Active, BadgeIntent::Neutral);
        let neutral_bg: Background = color::ACCENT_SOFT.current(mode).into();
        let neutral_fg = color::FG_3.current(mode);
        assert_eq!(
            format!("{:?}", neutral.background),
            format!("{neutral_bg:?}"),
            "Neutral intent → ACCENT_SOFT backdrop",
        );
        assert_eq!(
            format!("{:?}", neutral.text_color),
            format!("{neutral_fg:?}"),
            "Neutral intent → FG_3 label",
        );

        let negative = cockpit_badge_style(
            &theme,
            iced_aw::style::Status::Active,
            BadgeIntent::Negative,
        );
        let negative_bg: Background = color::DOWN_50.current(mode).into();
        let negative_fg = color::DOWN_500.current(mode);
        assert_eq!(
            format!("{:?}", negative.background),
            format!("{negative_bg:?}"),
            "Negative intent → DOWN_50 backdrop",
        );
        assert_eq!(
            format!("{:?}", negative.text_color),
            format!("{negative_fg:?}"),
            "Negative intent → DOWN_500 label",
        );
    }

    /// Pill radius + zero border are the structural invariants every
    /// intent shares — the backdrop alpha already carries the visual
    /// edge, no separate stroke. If `radius::PILL` ever changes or a
    /// border re-appears, snapshot bytes shift and this catches the
    /// upstream-token drift before the snapshot tests do.
    #[test]
    fn cockpit_badge_style_uses_pill_radius_and_no_border() {
        let theme = Theme::Dark;
        for intent in [
            BadgeIntent::Positive,
            BadgeIntent::Neutral,
            BadgeIntent::Negative,
        ] {
            let style = cockpit_badge_style(&theme, iced_aw::style::Status::Active, intent);
            assert_eq!(
                style.border_radius,
                Some(radius::PILL),
                "intent {intent:?} → PILL radius",
            );
            assert_eq!(style.border_width, 0.0, "intent {intent:?} → zero border");
            assert!(
                style.border_color.is_none(),
                "intent {intent:?} → no border color"
            );
        }
    }

    /// `Status::Disabled` alpha-scales both the backdrop and the label
    /// to 0.5 — same shape iced_aw's stock `disabled()` helper at
    /// `style/badge.rs:200-206` uses. Status pills don't fire Disabled
    /// in v0.1 (informational, not pressable), but a future "disabled
    /// strategies" filter pill should drop in without re-litigation.
    #[test]
    fn cockpit_badge_style_disabled_scales_alpha() {
        let theme = Theme::Dark;
        let base = cockpit_badge_style(
            &theme,
            iced_aw::style::Status::Active,
            BadgeIntent::Positive,
        );
        let disabled = cockpit_badge_style(
            &theme,
            iced_aw::style::Status::Disabled,
            BadgeIntent::Positive,
        );
        assert_eq!(
            format!("{:?}", disabled.background),
            format!("{:?}", base.background.scale_alpha(0.5)),
            "Disabled → 0.5 alpha on backdrop",
        );
        assert_eq!(
            format!("{:?}", disabled.text_color),
            format!("{:?}", base.text_color.scale_alpha(0.5)),
            "Disabled → 0.5 alpha on label",
        );
    }

    /// Hover / Pressed / Focused / Selected must equal Active byte-for-
    /// byte — status pills don't change colour on interaction. If a
    /// future change adds a hover state, this test will scream and the
    /// principles update lands first.
    #[test]
    fn cockpit_badge_style_non_disabled_states_match_active() {
        let theme = Theme::Dark;
        let active = cockpit_badge_style(
            &theme,
            iced_aw::style::Status::Active,
            BadgeIntent::Positive,
        );
        for s in [
            iced_aw::style::Status::Hovered,
            iced_aw::style::Status::Pressed,
            iced_aw::style::Status::Focused,
            iced_aw::style::Status::Selected,
        ] {
            let style = cockpit_badge_style(&theme, s, BadgeIntent::Positive);
            assert_eq!(
                format!("{:?}", style.background),
                format!("{:?}", active.background),
                "status {s:?} → same backdrop as Active",
            );
            assert_eq!(
                format!("{:?}", style.text_color),
                format!("{:?}", active.text_color),
                "status {s:?} → same label as Active",
            );
        }
    }
}
