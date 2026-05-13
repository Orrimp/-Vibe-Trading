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
use iced::{Background, Theme};

use super::color::{self};
use super::ThemeMode;

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
}
