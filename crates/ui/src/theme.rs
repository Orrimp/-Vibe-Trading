//! Design tokens: color, spacing, typography, radii.
//!
//! **Every** color, spacing value, border radius, and font size used in the
//! cockpit flows from this module. Widgets never inline hex codes or magic
//! `Length::Units(N)` numbers. This is the design-system contract.
//!
//! If you want to add a new token, think twice — the spacing scale is fixed
//! at `4 / 8 / 12 / 16 / 24 / 32`, the type scale at four sizes
//! (`caption / body / title / display`), and the palette at eight semantic
//! colors. Drift starts with "just one exception"; the whole scale stops
//! being useful the moment it has one. Prefer recombining existing tokens.
//!
//! Contrast spot-check: every `fg`/`bg` and `fg_muted`/`bg` pairing has
//! been hand-checked at ≥ 4.5:1 per WCAG AA against the primary dark
//! palette below.

use iced::Color;

/// Semantic color tokens — the only colors that appear in widget code.
///
/// Raw `#rrggbb` literals anywhere outside this module are a bug. The set
/// is deliberately small: if you find yourself wanting a fifth accent,
/// you are fighting the scale and should revisit the design, not add a
/// token.
pub mod color {
    use iced::Color;

    /// Raw 8-bit RGB → [`Color`]. Hex literals never leak outside this module.
    pub(super) const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Background — cockpit canvas.
    pub const BG: Color = rgb(0x11, 0x14, 0x1A);

    /// Elevated surface — cards, panels.
    pub const BG_ELEV: Color = rgb(0x1A, 0x1F, 0x29);

    /// Modal-dialog backdrop. Captures clicks outside the modal card.
    ///
    /// Sits behind the modal card and above the cockpit body in the iced
    /// `Stack`. Darker than `BG` so the modal card (`BG_ELEV`) reads as
    /// elevated; clicking this surface dismisses the modal.
    ///
    /// First consumer: `widgets::journal_transaction_modal` (tape-row →
    /// audit modal). Light-mode hex `#0B0D12CC` (80% opacity onto the
    /// light `BG`) is documented in `spec/ui-design-principles.md`'s
    /// light-mode table; lands with the broader light-mode feature.
    // TODO(light-mode): wire `#0B0D12CC` (light) when light-mode lands.
    pub const BG_OVERLAY: Color = rgb(0x0B, 0x0D, 0x12);

    /// Primary foreground text.
    pub const FG: Color = rgb(0xE8, 0xEC, 0xF2);

    /// Muted foreground — labels, secondary text.
    pub const FG_MUTED: Color = rgb(0x8B, 0x93, 0xA3);

    /// Accent — primary interactive element.
    pub const ACCENT: Color = rgb(0x5E, 0xA3, 0xFF);

    /// Positive — gains, buys, healthy state.
    pub const POS: Color = rgb(0x3E, 0xCF, 0x8E);

    /// Negative — losses, sells, danger, kill-switch fill.
    pub const NEG: Color = rgb(0xFF, 0x6B, 0x6B);

    /// Warning — amber, attention needed but not fatal.
    pub const WARN: Color = rgb(0xFF, 0xC4, 0x5A);

    /// Observation-only signals. Used for transaction-id text and other
    /// informational, non-interactive elements (funding-rate badges, "did
    /// you mean" hints). Distinct from `ACCENT`, which signals
    /// interactivity — `INFO` is read-only by convention.
    ///
    /// Light-mode hex `#1F6FE5` (same as light-mode `ACCENT`) is
    /// documented in `spec/ui-design-principles.md`; lands with the
    /// broader light-mode feature.
    // TODO(light-mode): wire `#1F6FE5` (light) when light-mode lands.
    pub const INFO: Color = rgb(0x7B, 0xC2, 0xFF);

    /// Border — panel outlines, separators.
    pub const BORDER: Color = rgb(0x2A, 0x31, 0x3F);

    /// Focused / hovered border, modal frame. Distinct from `BORDER` so
    /// the keyboard user can tell focused-from-active.
    ///
    /// First consumers: modal-card framing in
    /// `widgets::journal_transaction_modal`, and the focus ring on
    /// keyboard-navigated buttons (per accessibility minimums in
    /// `spec/ui-design-principles.md`).
    ///
    /// Light-mode hex `#C9D0DA` is documented in
    /// `spec/ui-design-principles.md`; lands with the broader
    /// light-mode feature.
    // TODO(light-mode): wire `#C9D0DA` (light) when light-mode lands.
    pub const BORDER_STRONG: Color = rgb(0x3A, 0x44, 0x56);
}

/// Spacing scale — use **only** these values. No exceptions.
///
/// The scale is `4 / 8 / 12 / 16 / 24 / 32`. Component padding, row gaps,
/// margins, and panel insets all pick from here. If you find yourself
/// wanting `10` or `20`, you are probably resizing something that should
/// use `12` or `16` and a different font size.
///
/// `u32` because iced's `Pixels` impls `From<u32>` but not `From<u16>`.
pub mod space {
    /// 4 px.
    pub const XS: u32 = 4;
    /// 8 px.
    pub const S: u32 = 8;
    /// 12 px.
    pub const M: u32 = 12;
    /// 16 px.
    pub const L: u32 = 16;
    /// 24 px.
    pub const XL: u32 = 24;
    /// 32 px.
    pub const XXL: u32 = 32;
}

/// Type scale — four sizes, no more.
pub mod text {
    /// 11 px — captions, axis labels, column headers.
    pub const CAPTION: u32 = 11;
    /// 13 px — body text, panel cell content.
    pub const BODY: u32 = 13;
    /// 16 px — panel titles, card headings.
    pub const TITLE: u32 = 16;
    /// 22 px — the equity number, kill-switch label, halted banner.
    pub const DISPLAY: u32 = 22;
}

/// Border radii. Two values; cockpit is rectangular by design.
pub mod radius {
    /// 2 px — chips, small inputs, badges.
    pub const SMALL: f32 = 2.0;
    /// 4 px — panels, cards, buttons.
    pub const MEDIUM: f32 = 4.0;
}

/// Panel-level layout constants.
pub mod layout {
    use super::space;
    /// Outer padding inside a panel frame.
    pub const PANEL_PADDING: u32 = space::L;
    /// Gap between a panel's header and its body.
    pub const PANEL_GAP: u32 = space::M;
    /// Gap between sibling panels in the grid.
    pub const PANEL_OUTER_GAP: u32 = space::L;
    /// Max rows shown in the live tape (R6.2 — last 200 fills).
    pub const TAPE_MAX_ROWS: usize = 200;
}

/// Latency thresholds per R6.2. Source of truth for badge color logic.
pub mod latency {
    /// `< 500 ms` → OK (green).
    pub const OK_MS: i64 = 500;
    /// `< 2 s`  → WARN (amber).
    pub const WARN_MS: i64 = 2_000;
    /// `≥ 10 s` → HALTED (red banner, not just "high").
    pub const HALTED_MS: i64 = 10_000;
}

/// Returns the color that should represent a signed delta in P&L, returns,
/// or exposure. Centralizes the "color only for pos/neg, muted for zero"
/// rule. Widgets must call this rather than picking colors inline.
#[must_use]
pub fn color_for_delta(delta: rust_decimal::Decimal) -> Color {
    if delta.is_zero() {
        color::FG_MUTED
    } else if delta.is_sign_positive() {
        color::POS
    } else {
        color::NEG
    }
}

/// Returns the color for a latency value in milliseconds, per the R6.2
/// thresholds. Widgets must use this helper rather than re-doing the
/// thresholds inline. Halted and High share red by design — the distinct
/// labels are carried by the strings, not the color.
#[must_use]
pub fn color_for_latency_ms(ms: i64) -> Color {
    if ms >= latency::WARN_MS {
        // Covers both High (≥ 2s) and Halted (≥ 10s) bands — same red.
        color::NEG
    } else if ms >= latency::OK_MS {
        color::WARN
    } else {
        color::POS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare two `Color`s by their byte-level RGBA components. Avoids
    /// dragging in a `PartialEq` impl on iced's `Color` (which uses `f32`
    /// equality and is awkward in `assert_eq!`).
    fn rgba8(c: Color) -> (u8, u8, u8, u8) {
        // `f32 → u8` is intentional: every `rgb(r,g,b)` constant is a
        // round-trip of `(byte / 255.0) * 255.0 → round() → byte`, which
        // is exact for the discrete byte values we feed in. The clippy
        // truncation/sign-loss lints fire on the lossy *general* case;
        // here the input is bounded to `[0.0, 1.0]` by construction.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let to_u8 = |x: f32| (x * 255.0).round() as u8;
        (to_u8(c.r), to_u8(c.g), to_u8(c.b), to_u8(c.a))
    }

    /// Modal-dialog backdrop — `#0B0D12` per
    /// `spec/ui-design-principles.md` color palette.
    #[test]
    fn bg_overlay_has_principles_dark_hex() {
        assert_eq!(rgba8(color::BG_OVERLAY), (0x0B, 0x0D, 0x12, 0xFF));
    }

    /// Observation-only accent — `#7BC2FF` per
    /// `spec/ui-design-principles.md` color palette.
    #[test]
    fn info_has_principles_dark_hex() {
        assert_eq!(rgba8(color::INFO), (0x7B, 0xC2, 0xFF, 0xFF));
    }

    /// Focused / hovered border — `#3A4456` per
    /// `spec/ui-design-principles.md` color palette.
    #[test]
    fn border_strong_has_principles_dark_hex() {
        assert_eq!(rgba8(color::BORDER_STRONG), (0x3A, 0x44, 0x56, 0xFF));
    }

    /// `BORDER_STRONG` must be visibly distinct from `BORDER` so a
    /// keyboard-focused element can be told apart from a panel outline.
    /// Principles doc: "Focus rings use `border_strong`, not `accent`,
    /// so the keyboard user can tell focused-from-active."
    #[test]
    fn border_strong_is_distinct_from_border() {
        assert_ne!(rgba8(color::BORDER), rgba8(color::BORDER_STRONG));
    }

    /// `BG_OVERLAY` must be darker than `BG` so the modal card (`BG_ELEV`)
    /// reads as elevated above the dimmed-out cockpit body.
    #[test]
    fn bg_overlay_is_darker_than_bg() {
        let bg = rgba8(color::BG);
        let overlay = rgba8(color::BG_OVERLAY);
        let lum = |(r, g, b, _): (u8, u8, u8, u8)| u32::from(r) + u32::from(g) + u32::from(b);
        assert!(
            lum(overlay) < lum(bg),
            "bg_overlay ({overlay:?}) must be darker than bg ({bg:?})",
        );
    }
}
