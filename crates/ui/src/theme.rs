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

    /// Border — panel outlines, separators.
    pub const BORDER: Color = rgb(0x2A, 0x31, 0x3F);
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
