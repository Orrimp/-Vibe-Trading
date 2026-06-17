//! Design tokens — Lumen palette + tier system + whisper shadows + motion.
//!
//! **Every** colour, spacing value, border radius, font size, shadow, and
//! motion duration used in the cockpit flows from this module. Widgets
//! never inline hex codes or magic `Length::Units(N)` numbers. This is
//! the design-system contract.
//!
//! Phase 1 (Lumen foundation) ships:
//!
//! - **Palette** — the Lumen warm + cool neutral scales, accent ramp,
//!   and semantic up / down / warn / info ramps. All values come from
//!   [`spec/design/project/colors_and_type.css`][css]. Both light and
//!   dark mode are wired; the cold-start render is dark per
//!   [`ThemeMode::Dark`].
//! - **Tier surfaces** — `CANVAS / PANEL / PANEL_RAISED / PANEL_SUNKEN /
//!   OVERLAY`. The four-tier elevation language replaces the old flat
//!   `BG / BG_ELEV` split.
//! - **Whisper shadows** — `shadow::shadow_1 / shadow_2 / shadow_3` plus
//!   `shadow_inset`. Three soft elevation levels; dark mode uses darker
//!   alpha, not bigger blurs (Lumen rule).
//! - **Focus ring** — [`focus::ring`] returns the 3 px low-alpha accent
//!   ring used on every focusable widget.
//! - **Spacing ladder** — 13 steps `0 / 2 / 4 / 6 / 8 / 12 / 16 / 20 /
//!   24 / 32 / 40 / 48 / 64`. Naming `ZERO / TICK / XXS / XS / S / M /
//!   L / L_PLUS / XL / XXL / XXXL / HUGE / MASSIVE`.
//! - **Radii** — six steps `R1 / R2 / R3 / R4 / R5 / PILL`.
//! - **Typography** — seven sizes `MICRO / SMALL / BODY / H3 / H2 / H1 /
//!   DISPLAY` plus the `FONT_SANS` / `FONT_MONO` family strings.
//! - **Motion** — four durations `DUR_1..DUR_4` and two cubic-bezier
//!   easings `EASE_OUT / EASE_IN_OUT`. No bounces — trading desks don't
//!   want kinetic UI.
//!
//! Drift starts with "just one exception"; the whole scale stops being
//! useful the moment it has one. Prefer recombining existing tokens to
//! adding new ones.
//!
//! [css]: ../../../../../spec/design/project/colors_and_type.css

use iced::Color;

/// Iced-widget Catalog adapters — house style functions that route the
/// cockpit's design tokens into iced's per-widget `Catalog` trait
/// surface. See the submodule docs for the orphan-rule constraint that
/// makes this an adapter hub rather than a foreign trait impl
/// (Q3-sub refinement pass 2026-05-13).
pub mod iced_widget_catalogs;

/// Theme mode — `Dark` is the cold-start (Q6).
///
/// Both bins (`cockpit`, `cockpit_live`) cold-start in `Dark`. The light
/// palette is wired through every `ModeColor` so the runtime toggle
/// (downstream feature) lights up without a token rewrite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    /// Cool dark canvas — the default render.
    #[default]
    Dark,
    /// Warm paper-like canvas — wired but not yet runtime-toggled.
    Light,
}

/// Semantic colour tokens — the only colours that appear in widget code.
///
/// Raw `#rrggbb` literals anywhere outside this module are a bug and the
/// `consistency.rs` test will catch them. The set is large but every
/// constant is grounded in
/// [`spec/design/project/colors_and_type.css`][css] and the Phase 1
/// architect's token-mapping table — see `theme.rs::tests::*` for the
/// load-bearing dark + light hex pin tests.
///
/// [css]: ../../../../../spec/design/project/colors_and_type.css
pub mod color {
    use super::ThemeMode;
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

    /// Raw 8-bit RGBA → [`Color`].
    pub(super) const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    /// Dual-mode colour token. `dark` is the cold-start render; `light`
    /// is selected only when the runtime theme switches (downstream
    /// feature). Use [`ModeColor::current`] in style closures so a
    /// future toggle is a one-line change.
    #[derive(Debug, Clone, Copy)]
    pub struct ModeColor {
        /// Dark-mode value — used at cold start.
        pub dark: Color,
        /// Light-mode value — wired but not toggled in Phase 1.
        pub light: Color,
    }

    impl ModeColor {
        /// Resolve to the active theme.
        #[must_use]
        pub const fn current(&self, mode: ThemeMode) -> Color {
            match mode {
                ThemeMode::Dark => self.dark,
                ThemeMode::Light => self.light,
            }
        }
    }

    // ── Surface tier tokens ──────────────────────────────────────────────
    //
    // `colors_and_type.css:73, 114` — canvas
    // `colors_and_type.css:74, 115` — panel
    // `colors_and_type.css:75, 116` — panel-raised
    // `colors_and_type.css:76, 117` — panel-sunken
    // `colors_and_type.css:77, 118` — overlay (alpha)

    /// Tier 0 — app background. Top-level shell container only.
    pub const CANVAS: ModeColor = ModeColor {
        dark: rgb(0x13, 0x18, 0x20),  // cool-800
        light: rgb(0xF6, 0xF4, 0xEF), // warm-50
    };

    /// Tier 1 — default panel surface. Used by every panel widget.
    pub const PANEL: ModeColor = ModeColor {
        dark: rgb(0x1C, 0x21, 0x27),  // cool-700
        light: rgb(0xFB, 0xFA, 0xF7), // warm-25
    };

    /// Tier 2 — dialogs / popovers / dropdowns. Also the panel-header tint
    /// (Tier 1.h) inside a Tier 1 frame.
    pub const PANEL_RAISED: ModeColor = ModeColor {
        dark: rgb(0x2A, 0x30, 0x38),  // cool-600
        light: rgb(0xFF, 0xFF, 0xFF), // pure white
    };

    /// Sunken — input fields, table stripes.
    pub const PANEL_SUNKEN: ModeColor = ModeColor {
        dark: rgb(0x0B, 0x0F, 0x15),  // cool-900
        light: rgb(0xEF, 0xEB, 0xE3), // warm-100
    };

    /// Modal-dialog backdrop. Captures clicks outside the modal card.
    /// Lumen's overlay is alpha-blended; we materialise it as a flat
    /// pre-multiplied colour for iced's container API.
    pub const OVERLAY: ModeColor = ModeColor {
        dark: rgba(0x00, 0x00, 0x00, 0.55),
        light: rgba(0x14, 0x13, 0x0F, 0.45),
    };

    // ── Foreground (text) ────────────────────────────────────────────────
    //
    // `colors_and_type.css:79–83` — light fg ladder
    // `colors_and_type.css:120–125` — dark fg ladder

    /// Primary text — `cool-25-ish` dark / `warm-900` light.
    pub const FG_1: ModeColor = ModeColor {
        dark: rgb(0xE8, 0xEC, 0xF1),
        light: rgb(0x14, 0x13, 0x0F), // warm-900
    };

    /// Secondary text.
    pub const FG_2: ModeColor = ModeColor {
        dark: rgb(0xB7, 0xBF, 0xCB),
        light: rgb(0x34, 0x32, 0x2C), // warm-700
    };

    /// Tertiary / labels. The status bar baseline.
    pub const FG_3: ModeColor = ModeColor {
        dark: rgb(0x80, 0x89, 0x93),
        light: rgb(0x6F, 0x6A, 0x5E), // warm-500
    };

    /// Placeholder / disabled.
    pub const FG_4: ModeColor = ModeColor {
        dark: rgb(0x5C, 0x65, 0x71),
        light: rgb(0x9E, 0x97, 0x88), // warm-400
    };

    /// Text rendered on top of an `ACCENT` fill.
    pub const FG_ON_ACCENT: ModeColor = ModeColor {
        dark: rgb(0x0B, 0x0F, 0x15),  // cool-900
        light: rgb(0xFF, 0xFF, 0xFF), // pure white
    };

    // ── Accent ramp + soft ──────────────────────────────────────────────
    //
    // `colors_and_type.css:18–27` — accent-50 .. accent-900
    // `colors_and_type.css:89–92` — light surface accent + variants
    // `colors_and_type.css:131–134` — dark surface accent + variants

    /// Primary accent. Dark uses `accent-300` (lighter); light uses
    /// `accent-400` (darker) — both target the same perceived weight
    /// against their surface.
    pub const ACCENT: ModeColor = ModeColor {
        dark: rgb(0x6F, 0xB6, 0xAE),  // accent-300
        light: rgb(0x3F, 0x96, 0x8D), // accent-400
    };

    /// Hover variant of accent.
    pub const ACCENT_HOVER: ModeColor = ModeColor {
        dark: rgb(0xA6, 0xD5, 0xCF),  // accent-200
        light: rgb(0x2A, 0x7B, 0x73), // accent-500
    };

    /// Pressed variant of accent.
    pub const ACCENT_PRESS: ModeColor = ModeColor {
        dark: rgb(0x3F, 0x96, 0x8D),  // accent-400
        light: rgb(0x1F, 0x63, 0x5D), // accent-600
    };

    /// Soft accent fill — chips, gentle highlights.
    /// Lumen specifies `rgba(111, 182, 174, 0.12)` for dark and the
    /// `accent-50` opaque colour for light.
    pub const ACCENT_SOFT: ModeColor = ModeColor {
        dark: rgba(0x6F, 0xB6, 0xAE, 0.12),
        light: rgb(0xEC, 0xF6, 0xF5), // accent-50
    };

    // ── Comparison-overlay accent ramp (ACCENT_2..5) ────────────────────
    //
    // Four new tokens added for the multi-strategy comparison overlay
    // (ui-rethink-phase-a-lab T-D-9 / Design § 7).  Hex values are
    // verbatim from
    // [`spec/dev-notes/lumen-accent-palette-extension-2026-05-17.md`].
    // These tokens are the ONLY callers that use the new hues; all chart
    // code and strategy_chip code must reference these constants so the
    // Lumen Phase 1 hex-audit (`grep '#' …`) stays clean.

    /// Comparison slot 0 — desaturated teal.
    /// Dark: `accent-200` (#A6D5CF). Light: `accent-500` (#2A7B73).
    pub const ACCENT_2: ModeColor = ModeColor {
        dark: rgb(0xA6, 0xD5, 0xCF),
        light: rgb(0x2A, 0x7B, 0x73),
    };

    /// Comparison slot 1 — cool blue.
    /// Dark: #82AEDC. Light: #3D6BA8.
    pub const ACCENT_3: ModeColor = ModeColor {
        dark: rgb(0x82, 0xAE, 0xDC),
        light: rgb(0x3D, 0x6B, 0xA8),
    };

    /// Comparison slot 2 — muted purple.
    /// Dark: #B79BD4. Light: #6E4F9C.
    pub const ACCENT_4: ModeColor = ModeColor {
        dark: rgb(0xB7, 0x9B, 0xD4),
        light: rgb(0x6E, 0x4F, 0x9C),
    };

    /// Comparison slot 3 — amber.
    /// Dark: `warn-400` (#E0B45C). Light: `warn-600` (#A8842F).
    pub const ACCENT_5: ModeColor = ModeColor {
        dark: rgb(0xE0, 0xB4, 0x5C),
        light: rgb(0xA8, 0x84, 0x2F),
    };

    /// Returns the four comparison-slot accent tokens in positional order
    /// `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]`. Used by `strategy_chip`
    /// and the chart comparison-overlay draw pass.
    ///
    /// The unit test `accent_palette_slot_order_is_stable` pins this
    /// ordering so any reorder shows up as a deliberate test edit.
    #[must_use]
    pub const fn accent_palette() -> [ModeColor; 4] {
        [ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]
    }

    // ── Semantic ramps — sage / clay / warn / info ───────────────────────
    //
    // `colors_and_type.css:57–70` — light values
    // `colors_and_type.css:136–148` — dark values

    /// Sage gain — soft tint backdrop (`up-50`).
    pub const UP_50: ModeColor = ModeColor {
        dark: rgba(0x88, 0xB3, 0x83, 0.12),
        light: rgb(0xE9, 0xF0, 0xE7),
    };

    /// Sage gain — `up-400` brighter shade.
    pub const UP_400: ModeColor = ModeColor {
        dark: rgb(0x88, 0xB3, 0x83),
        light: rgb(0x6E, 0x9B, 0x6A),
    };

    /// Sage gain — `up-500` deeper shade. Public default for "positive".
    pub const UP_500: ModeColor = ModeColor {
        dark: rgb(0x6E, 0x9B, 0x6A),
        light: rgb(0x54, 0x7A, 0x52),
    };

    /// Clay loss — soft tint backdrop (`down-50`).
    pub const DOWN_50: ModeColor = ModeColor {
        dark: rgba(0xDD, 0x8E, 0x70, 0.12),
        light: rgb(0xF5, 0xE5, 0xDD),
    };

    /// Clay loss — `down-400` brighter shade.
    pub const DOWN_400: ModeColor = ModeColor {
        dark: rgb(0xDD, 0x8E, 0x70),
        light: rgb(0xC9, 0x7B, 0x5E),
    };

    /// Clay loss — `down-500` deeper shade. Public default for "negative".
    pub const DOWN_500: ModeColor = ModeColor {
        dark: rgb(0xC9, 0x7B, 0x5E),
        light: rgb(0xA9, 0x5F, 0x46),
    };

    /// Warn — soft tint backdrop (`warn-50`).
    pub const WARN_50: ModeColor = ModeColor {
        dark: rgba(0xE0, 0xB4, 0x5C, 0.12),
        light: rgb(0xF6, 0xEC, 0xD3),
    };

    /// Warn — `warn-400` brighter shade.
    pub const WARN_400: ModeColor = ModeColor {
        dark: rgb(0xE0, 0xB4, 0x5C),
        light: rgb(0xD4, 0xA2, 0x4A),
    };

    /// Warn — `warn-500` deeper shade. Public default for "warning".
    /// Lumen's CSS doesn't expose a dark `--warn-500` explicitly; the
    /// dark surface inherits `warn-400` for legibility, so we shadow the
    /// 500 step with the 400 hex in dark mode (matches the published
    /// Lumen brand kit's dark-mode behaviour).
    pub const WARN_500: ModeColor = ModeColor {
        dark: rgb(0xE0, 0xB4, 0x5C),
        light: rgb(0xB7, 0x86, 0x2F),
    };

    /// Info — soft tint backdrop (`info-50`).
    pub const INFO_50: ModeColor = ModeColor {
        dark: rgba(0x84, 0xA6, 0xD0, 0.12),
        light: rgb(0xE4, 0xEC, 0xF5),
    };

    /// Info — `info-400` brighter shade.
    pub const INFO_400: ModeColor = ModeColor {
        dark: rgb(0x84, 0xA6, 0xD0),
        light: rgb(0x5E, 0x84, 0xB4),
    };

    /// Info — `info-500` deeper shade. Public default for "informational".
    /// Same dark-mode caveat as `WARN_500`: Lumen shadows the 500 step
    /// with `info-400` on cool surfaces.
    pub const INFO_500: ModeColor = ModeColor {
        dark: rgb(0x84, 0xA6, 0xD0),
        light: rgb(0x43, 0x6A, 0x9A),
    };

    // ── Borders ──────────────────────────────────────────────────────────
    //
    // `colors_and_type.css:85–87` — light border ladder
    // `colors_and_type.css:127–129` — dark border ladder

    /// Hairline between panels.
    pub const BORDER_1: ModeColor = ModeColor {
        dark: rgb(0x23, 0x2A, 0x33),
        light: rgb(0xE2, 0xDD, 0xD2), // warm-200
    };

    /// Stronger divider — sunken-input border, table-stripe edge.
    pub const BORDER_2: ModeColor = ModeColor {
        dark: rgb(0x2E, 0x36, 0x40),
        light: rgb(0xC9, 0xC2, 0xB3), // warm-300
    };

    /// Hover / active border. Distinct from `BORDER_1` so the keyboard
    /// user can tell active-from-resting; the focus ring layers
    /// `focus::ring` on top.
    pub const BORDER_STRONG: ModeColor = ModeColor {
        dark: rgb(0x40, 0x49, 0x54),
        light: rgb(0x9E, 0x97, 0x88), // warm-400
    };

    // ── Loading / progress indicators ────────────────────────────────────
    //
    // Brief B (`iced-aw-cherry-pick`, T-M2-1) adds an `iced_aw::Spinner`
    // alongside the existing `muted_body` loading-text helper. The
    // spinner is paired in a `Row` with that text — pinning its tint to
    // the same `FG_3` step keeps the indicator visually quiet and
    // matches the "loading text fades into chrome, content is the
    // signal" pattern from `spec/ui-design-principles.md ## Empty,
    // loading, error states`. Using `ACCENT` would shout an "active"
    // signal at a moment when the operator should be waiting; `UP_500`
    // would falsely imply a positive result. `FG_3` is the only step
    // that says "we know, we're waiting too" without lying.

    /// Loading-indicator tint. Pinned to `FG_3` so `iced_aw::Spinner`
    /// renders with the same muted weight as the `muted_body`
    /// `Loading…` text it pairs with — see
    /// [`crate::widgets::frame::muted_body`] which already routes
    /// `FG_3` for body text inside loading panels.
    pub const SPINNER_TINT: ModeColor = FG_3;
}

/// Shadow tokens — the whisper-shadow ladder.
///
/// Lumen layers two box-shadows per level (`0 1px 1px ..., 0 1px 2px
/// ...`); iced takes one shadow per `container::Style`, so we collapse
/// to the dominant outer layer. The inner hair-shadow is inherited from
/// the 1 px hairline border every panel already draws, which lands in
/// the same colour budget. See Q3 / Phase 1 design notes for the
/// rendering verification.
pub mod shadow {
    use super::{ThemeMode, color::rgba};
    use iced::{Color, Shadow, Vector};

    /// Tier 1 — panel chrome. `(offset_y, blur, alpha)` per design table.
    #[must_use]
    pub fn shadow_1(mode: ThemeMode) -> Shadow {
        match mode {
            ThemeMode::Dark => Shadow {
                color: rgba(0x00, 0x00, 0x00, 0.30),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 2.0,
            },
            ThemeMode::Light => Shadow {
                color: rgba(0x14, 0x13, 0x0F, 0.04),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 2.0,
            },
        }
    }

    /// Tier 2 — dialogs / popovers.
    #[must_use]
    pub fn shadow_2(mode: ThemeMode) -> Shadow {
        match mode {
            ThemeMode::Dark => Shadow {
                color: rgba(0x00, 0x00, 0x00, 0.35),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
            ThemeMode::Light => Shadow {
                color: rgba(0x14, 0x13, 0x0F, 0.06),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
        }
    }

    /// Tier 3 — modals.
    #[must_use]
    pub fn shadow_3(mode: ThemeMode) -> Shadow {
        match mode {
            ThemeMode::Dark => Shadow {
                color: rgba(0x00, 0x00, 0x00, 0.50),
                offset: Vector::new(0.0, 12.0),
                blur_radius: 28.0,
            },
            ThemeMode::Light => Shadow {
                color: rgba(0x14, 0x13, 0x0F, 0.08),
                offset: Vector::new(0.0, 12.0),
                blur_radius: 24.0,
            },
        }
    }

    /// Sunken inset — iced's `Shadow` is outer-only, so the visual
    /// equivalent of CSS `inset 0 1px 0 rgba(...)` is rendered as a
    /// 1 px hairline `Container` along the input's top edge. This
    /// function returns the colour for that hairline.
    #[must_use]
    pub fn shadow_inset(mode: ThemeMode) -> Color {
        match mode {
            ThemeMode::Dark => rgba(0xFF, 0xFF, 0xFF, 0.03),
            ThemeMode::Light => rgba(0x14, 0x13, 0x0F, 0.04),
        }
    }
}

/// Focus ring — Lumen's 3 px low-alpha accent halo.
///
/// Rendered as an outer iced `Shadow` with offset `(0, 0)` and a 3 px
/// blur. iced doesn't natively support box-shadow `spread`; the blur
/// produces the same perceived halo at the alpha values Lumen specifies.
pub mod focus {
    use super::{ThemeMode, color::rgba};
    use iced::{Shadow, Vector};

    /// 3 px low-alpha accent ring. Layered on top of `BORDER_STRONG`
    /// borders for keyboard-focused interactive elements.
    ///
    /// Light: `rgba(63, 150, 141, 0.28)` (accent-400).
    /// Dark: `rgba(166, 213, 207, 0.30)` (accent-200).
    #[must_use]
    pub fn ring(mode: ThemeMode) -> Shadow {
        let color = match mode {
            ThemeMode::Dark => rgba(0xA6, 0xD5, 0xCF, 0.30),
            ThemeMode::Light => rgba(0x3F, 0x96, 0x8D, 0.28),
        };
        Shadow {
            color,
            offset: Vector::ZERO,
            blur_radius: 3.0,
        }
    }
}

/// Spacing scale — the Lumen 13-step ladder.
///
/// Use **only** these values. No exceptions. Pixel values, top-to-bottom:
/// `0 / 2 / 4 / 6 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64`.
///
/// `u32` because iced's `Pixels` impls `From<u32>` but not `From<u16>`.
pub mod space {
    /// 0 px — collapse a slot without removing it.
    pub const ZERO: u32 = 0;
    /// 2 px — hairline gap, separator spacing.
    pub const TICK: u32 = 2;
    /// 4 px — chip padding, dense table cell padding.
    pub const XXS: u32 = 4;
    /// 6 px — small button padding, inline icon gap.
    pub const XS: u32 = 6;
    /// 8 px — control padding, badge padding.
    pub const S: u32 = 8;
    /// 12 px — section padding, panel header gap.
    pub const M: u32 = 12;
    /// 16 px — panel padding, list-row gap.
    pub const L: u32 = 16;
    /// 20 px — block separator.
    pub const L_PLUS: u32 = 20;
    /// 24 px — panel outer gap, dialog padding.
    pub const XL: u32 = 24;
    /// 32 px — top-of-screen breathing room.
    pub const XXL: u32 = 32;
    /// 40 px — section divider spacing.
    pub const XXXL: u32 = 40;
    /// 48 px — page-level header padding.
    pub const HUGE: u32 = 48;
    /// 64 px — empty-state vertical anchor.
    pub const MASSIVE: u32 = 64;
}

/// Type scale — Lumen's 7-step typography ladder.
///
/// Sizes in CSS pixels (desktop-app convention; UI is fixed-zoom).
pub mod text {
    /// 11 px — table column headers, timestamps, status-bar text.
    pub const MICRO: u32 = 11;
    /// 12 px — small labels.
    pub const SMALL: u32 = 12;
    /// 13 px — default desktop body / UI text.
    pub const BODY: u32 = 13;
    /// 15 px — sub-section headers.
    pub const H3: u32 = 15;
    /// 18 px — panel titles, card headings.
    pub const H2: u32 = 18;
    /// 24 px — page-level headings, equity number.
    pub const H1: u32 = 24;
    /// 32 px — hero numbers, marketing-style display.
    pub const DISPLAY: u32 = 32;
}

/// Font family stacks. `Inter` for UI, `JetBrains Mono` for numerics.
///
/// The runtime does not bundle `Inter` or `JetBrains Mono` TTFs —
/// operator-locked: every kilobyte of font is a kilobyte not spent on
/// faster bar rendering. iced falls through the stack to a platform
/// default. The constants exist so widget code can still cite the
/// canonical Lumen stack when the runtime gains font loading.
pub mod font {
    /// UI sans-serif stack: `Inter` → platform default.
    pub const FONT_SANS: &str = "Inter, -apple-system, BlinkMacSystemFont, \"Segoe UI\", \"Helvetica Neue\", Arial, sans-serif";
    /// Numerics monospace stack: `JetBrains Mono` → platform default.
    pub const FONT_MONO: &str = "\"JetBrains Mono\", ui-monospace, \"SF Mono\", Menlo, \"Cascadia Mono\", Consolas, monospace";
}

/// Border radii — the Lumen 6-step radii ladder.
///
/// `f32` because iced's `border::Radius` impls `From<f32>`.
pub mod radius {
    /// 2 px — dense table inputs.
    pub const R1: f32 = 2.0;
    /// 4 px — default control radius.
    pub const R2: f32 = 4.0;
    /// 6 px — buttons, chips.
    pub const R3: f32 = 6.0;
    /// 8 px — cards, panels.
    pub const R4: f32 = 8.0;
    /// 12 px — modals, sheets.
    pub const R5: f32 = 12.0;
    /// Pill / fully-rounded — tags, toggle thumbs, status-bar dots.
    pub const PILL: f32 = 999.0;
}

/// Motion tokens — durations + easings.
///
/// Trading desks don't want kinetic UI. No bounces, no spring physics.
/// Four short durations and two cubic-bezier curves; that's all.
pub mod motion {
    use std::time::Duration;

    /// 80 ms — tap feedback.
    pub const DUR_1: Duration = Duration::from_millis(80);
    /// 140 ms — hover, focus.
    pub const DUR_2: Duration = Duration::from_millis(140);
    /// 220 ms — panel reveal.
    pub const DUR_3: Duration = Duration::from_millis(220);
    /// 320 ms — modal enter.
    pub const DUR_4: Duration = Duration::from_millis(320);

    /// Cubic-bezier easing — `ease-out` flavoured (no overshoot).
    /// Control points: `(0.22, 0.61, 0.36, 1.0)`.
    pub const EASE_OUT: [f32; 4] = [0.22, 0.61, 0.36, 1.0];
    /// Cubic-bezier easing — symmetric `ease-in-out`.
    /// Control points: `(0.4, 0.0, 0.2, 1.0)`.
    pub const EASE_IN_OUT: [f32; 4] = [0.4, 0.0, 0.2, 1.0];
}

/// Panel-level layout constants. Tier-1 default geometry.
pub mod layout {
    use super::space;
    use crate::state::Screen;
    /// Outer padding inside a panel frame.
    pub const PANEL_PADDING: u32 = space::L;
    /// Gap between a panel's header and its body.
    pub const PANEL_GAP: u32 = space::M;
    /// Gap between sibling panels in the grid.
    pub const PANEL_OUTER_GAP: u32 = space::L;
    /// Max rows shown in the live tape (R6.2 — last 200 fills).
    pub const TAPE_MAX_ROWS: usize = 200;

    /// Phase 2 — fixed sidebar-nav column width in logical pixels.
    /// Sized so single-word labels (`Home / Debug / Charts`) sit comfortably
    /// with `space::M` left padding + `space::L` right padding.
    pub const SIDEBAR_WIDTH_PX: f32 = 180.0;

    /// Phase 2 — right-rail Phase 6 Assistant slot reservation. The shell
    /// renders this column with `Length::Fixed(0.0)` until the v2-LLM
    /// Assistant ships in Phase 6. (Phase 2 Q7 — structural-now.)
    ///
    /// **K6 Option A (Phase F):** this constant is PRESERVED at `0.0` to
    /// keep `shell_grid.rs:14-16` hard invariant + Phase D `trail_drawer`
    /// body byte-identical. A NEW constant `RIGHT_RAIL_OPEN_WIDTH_PX` is
    /// introduced for the open-state width. The shell picks one of the two
    /// constants based on `assistant_state.is_open`.
    pub const RIGHT_RAIL_WIDTH_PX: f32 = 0.0;

    /// Phase F — right-rail width when the Assistant slot is OPEN (K6 Option A).
    ///
    /// `RIGHT_RAIL_WIDTH_PX = 0.0` stays as the CLOSED-state default (Phase 2
    /// Q7 ratification; preserved by K6 Option A for byte-identical Phase D
    /// `trail_drawer` body + the `shell_grid.rs:14-16` hard invariant).
    ///
    /// At Phase F, `shell::view` picks one of the two constants based on
    /// `assistant_state.is_open`:
    ///
    /// ```rust,ignore
    /// let right_rail_width = if model.assistant_state.is_open {
    ///     RIGHT_RAIL_OPEN_WIDTH_PX
    /// } else {
    ///     RIGHT_RAIL_WIDTH_PX  // == 0.0
    /// };
    /// ```
    ///
    /// 320 px is the Lumen Phase 6 sketch width; also the width used by the
    /// Memory drawer (Q5=(b)) so the operator's mental model of "right-side
    /// panels are 320 px wide" is consistent.
    pub const RIGHT_RAIL_OPEN_WIDTH_PX: f32 = 320.0;

    /// Strategies-table column-1 active-row rule height in logical pixels.
    ///
    /// **Why a fixed value (and not `Length::Fill`):** inside an
    /// `iced::widget::table::Table` cell, the layout pass briefly resolves
    /// a `Length::Fill` height to `0.0` during the first frame's two-pass
    /// measurement. A child Container styled with a non-`None` background
    /// then emits a `fill_quad` with zero-height bounds, which the
    /// `iced_tiny_skia` renderer's `rounded_rectangle` all-radii-zero
    /// fast-path (`tiny_skia::Rect::from_xywh(x, y, w, 0.0)`) refuses with
    /// `panic!("Build quad rectangle")`. Pinning the rule's height to a
    /// concrete pixel value bypasses the zero-height transient.
    ///
    /// 24 px matches the cockpit's Table body row geometry
    /// (`text::BODY = 14 px` + `space::S = 4 px` top + 4 px bottom + ~2 px
    /// inter-cell breathing). Tune in 4 px steps if a future Lumen update
    /// shifts row metrics.
    ///
    /// See [`spec/cockpit-render-regression/feature.md`](../../../spec/cockpit-render-regression/feature.md)
    /// `## M0 results` for the bisect that pinned this trigger to the
    /// rule Container inside [`widgets::strategies::id_cell`](crate::widgets::strategies),
    /// and `## M0-FIX` for the F1 falsifier that confirmed the fix.
    pub const STRATEGY_RULE_HEIGHT_PX: f32 = 24.0;

    /// Phase 3 — sidebar entry list — six entries in master-roadmap scan
    /// order (Q8). Inserts `Strategies / Risk / Audit` between `Debug`
    /// and `Charts` per the analyst's ratified insertion point. Phase 2's
    /// `SIDEBAR_ENTRIES_PHASE_2` was removed atomically on Phase 3 ship —
    /// no forward-compat need.
    ///
    /// Uses deprecated `Screen` variants; kept for test-snapshot regression
    /// baseline. Phase A shell uses `SIDEBAR_ENTRIES_PHASE_A`.
    #[allow(deprecated)]
    pub const SIDEBAR_ENTRIES_PHASE_3: &[Screen] = &[
        Screen::Home,
        Screen::Debug,
        Screen::Strategies,
        Screen::Risk,
        Screen::Audit,
        Screen::Charts,
    ];

    /// Phase 5 (Q1) — adds `Screen::Control` (`HumanControl` panel) as
    /// the 7th sidebar entry, appended to the end so the existing 6
    /// positions are preserved. The Phase 2 R1.6 sidebar widget API is
    /// parameterised — additive only.
    ///
    /// **Deprecated at Phase A** — use `SIDEBAR_ENTRIES_PHASE_A`.
    /// Kept for one cycle for any call sites that reference it directly.
    #[allow(deprecated)]
    pub const SIDEBAR_ENTRIES_PHASE_5: &[Screen] = &[
        Screen::Home,
        Screen::Debug,
        Screen::Strategies,
        Screen::Risk,
        Screen::Audit,
        Screen::Charts,
        Screen::Control,
    ];

    /// Phase A (ui-rethink-phase-a-lab T-D-3) — Phase A workflow-group
    /// sidebar shape per Design § 6 / R9.1. New three-group structure:
    ///
    /// ```text
    /// Lab        ← default route (R1.2)
    /// Live       ← renamed from Home
    /// Compare    ← placeholder (Phase E)
    /// ─────
    /// Strategies ← unchanged
    /// Memory     ← placeholder (Phase F)
    /// Models     ← placeholder (Phase F)
    /// Trail      ← renamed from Audit (Phase D body)
    /// ─────
    /// Settings   ← placeholder (Phase C rollup)
    /// ```
    pub const SIDEBAR_ENTRIES_PHASE_A: &[Screen] = &[
        Screen::Lab,
        Screen::Live,
        Screen::Compare,
        // cockpit-baseline-panel v0.1.0 (R6 / D2) — navigable, after Compare.
        // Must stay lock-step with `SIDEBAR_GROUPS_PHASE_C` Work group below
        // (the flatten-invariant test is the guard, AC6).
        Screen::Baseline,
        Screen::Strategies,
        Screen::Memory,
        Screen::Models,
        // cockpit-reports-viewer v0.1.0 (R6 / D4) — Library group, after
        // Models (browse-a-corpus shape, same as Models). Must stay
        // lock-step with `SIDEBAR_GROUPS_PHASE_C` library group below (the
        // flatten-invariant test is the guard, AC6).
        Screen::Reports,
        Screen::Trail,
        Screen::Settings,
    ];

    /// Phase C — Three-group sidebar IA.
    ///
    /// Groups: `work` (Lab · Live · Compare · Baseline) / `library`
    /// (Strategies · Memory · Models · Reports · Trail) / `chrome`
    /// (Settings). A 1-px `BORDER_1` hairline divider is rendered between
    /// each group in `widgets::sidebar_nav::view` (Design § A1/A2).
    ///
    /// Invariant: `SIDEBAR_GROUPS_PHASE_C.iter().flat_map(|g| g.iter())
    /// .copied().collect::<Vec<_>>()` must equal
    /// `SIDEBAR_ENTRIES_PHASE_A.to_vec()` — verified by
    /// `theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a`.
    pub const SIDEBAR_GROUPS_PHASE_C: &[&[Screen]] = &[
        // work — cockpit-baseline-panel v0.1.0 (R6) inserts `Baseline`
        // after `Compare`; mirrors `SIDEBAR_ENTRIES_PHASE_A` in lock-step.
        &[Screen::Lab, Screen::Live, Screen::Compare, Screen::Baseline],
        &[
            Screen::Strategies,
            Screen::Memory,
            Screen::Models,
            // cockpit-reports-viewer v0.1.0 (R6 / D4) — after Models.
            Screen::Reports,
            Screen::Trail,
        ], // library
        &[Screen::Settings], // chrome
    ];

    /// Phase 3 — Audit-screen pagination size (Q4 — fixed 250 rows / page).
    /// Bins use this constant when issuing `recent_journal_filtered`
    /// (`LIMIT 250 OFFSET page * 250`); the screen pagination header
    /// renders "Showing N–M of T" using the same constant.
    pub const AUDIT_PAGE_SIZE: u32 = 250;

    /// Phase 4 — equity-history sparkline point cap for the cockpit
    /// Strategies-detail screen (Q9). The fetched `EquitySeries` is
    /// `downsample(SPARKLINE_POINT_CAP)`-d before landing on
    /// `Cockpit::strategy_equity`.
    pub const SPARKLINE_POINT_CAP: usize = 120;

    /// cockpit-live-dashboard-wiring v0.1.0 (D-buffer) — bounded ring cap
    /// for the session-scoped live equity buffer
    /// (`Cockpit::live_equity_buffer`). `2_880` = 48 h of 1-min bars at full
    /// resolution before any eviction; a longer session quietly slides a
    /// 48 h window. This governs **retention/memory** only
    /// (`2_880` × ~48 B ≈ 140 KB worst case) — the chart consumer still
    /// `downsample`s to `SPARKLINE_POINT_CAP` for pixels.
    pub const LIVE_EQUITY_BUFFER_CAP: usize = 2_880;

    // ── Chart canvas overhaul (v1.10.0) — axis gutters + legend chrome ───
    //
    // Six tokens introduced by `chart-canvas-overhaul` v1.10.0 to host the
    // price axis (left gutter), time axis (bottom gutter), an optional
    // right margin (TradingView-style centring per Q1), and the top-right
    // legend inset card (Q5). All values are in **logical pixels**; iced's
    // HiDPI pipeline scales them on Retina.
    //
    // See `spec/chart-canvas-overhaul/feature.md ## Design` for the per-
    // token derivation. Naming is the architect's pick — the suffix
    // `_PX` makes them grep-distinct from spacing tokens (`space::*`) and
    // signals "absolute pixel value, not a step on the spacing ladder."

    /// Left price-axis gutter width (M2 / R4.1). Sized for a 5-digit
    /// price label (`102.05`) at `text::MICRO` (11 px) with a
    /// `space::S` (8 px) pad on each side. Derivation:
    /// `5 digits × 6.5 px/digit + 2 × 8 px pad = 48.5 → 48`.
    pub const AXIS_GUTTER_PRICE_PX: f32 = 48.0;

    /// Right canvas margin (Q1 — TradingView-style centring without a
    /// right-side label column in v1.10). Empty band so the most-recent
    /// close-price marker breathes; v1.11 may repurpose it for a
    /// right-side current-price tag.
    pub const AXIS_GUTTER_RIGHT_PX: f32 = 16.0;

    /// Bottom time-axis gutter (M3 / R4.2). Stack: `text::MICRO`
    /// baseline (11 px) + 4-px tick + `space::XXS` (4 px) gap +
    /// `space::XXS` (4 px) bottom pad = 23 → round to 24.
    pub const AXIS_GUTTER_TIME_PX: f32 = 24.0;

    /// Legend card chrome width (M4 / Q5). Fits "Buy signal" /
    /// "Sell signal" — the longest entry — at `text::MICRO` with the
    /// 10-px glyph and `space::S` inter-column gap.
    pub const LEGEND_CARD_WIDTH_PX: f32 = 140.0;

    /// Legend card chrome height — 5 entries × (10 px glyph + 2 px
    /// inter-row gap) + 2 × `space::S` interior pad = 76 → round to 80.
    pub const LEGEND_CARD_HEIGHT_PX: f32 = 80.0;

    /// Legend triangle glyph height — `≈ ¾ × MARKER_SIZE_PX` (13 px).
    /// Sized to read as a downsampled sibling of the chart's executed-
    /// fill markers at `text::MICRO` label height.
    pub const LEGEND_GLYPH_PX: f32 = 10.0;
}

/// Latency thresholds per R6.2. Source of truth for badge colour logic.
pub mod latency {
    /// `< 500 ms` → OK (green / `UP_500`).
    pub const OK_MS: i64 = 500;
    /// `< 2 s` → WARN (amber / `WARN_500`).
    pub const WARN_MS: i64 = 2_000;
    /// `≥ 10 s` → HALTED (red banner / `DOWN_500`, not just "high").
    pub const HALTED_MS: i64 = 10_000;
}

/// Returns the colour that should represent a signed delta in P&L,
/// returns, or exposure. Centralizes the "colour only for pos/neg, muted
/// for zero" rule. Widgets must call this rather than picking colours
/// inline.
///
/// Phase 1: returns `Color` resolved against the cold-start
/// [`ThemeMode::Dark`]. Signature preserved across the Lumen rewrite
/// (see `lumen-phase-1-foundation.md` R9.3).
#[must_use]
pub fn color_for_delta(delta: rust_decimal::Decimal) -> Color {
    if delta.is_zero() {
        color::FG_3.current(ThemeMode::Dark)
    } else if delta.is_sign_positive() {
        color::UP_500.current(ThemeMode::Dark)
    } else {
        color::DOWN_500.current(ThemeMode::Dark)
    }
}

/// Returns the colour for a latency value in milliseconds, per the R6.2
/// thresholds. Halted and High share red by design — the distinct
/// labels are carried by the strings, not the colour.
///
/// Phase 1: returns `Color` resolved against the cold-start
/// [`ThemeMode::Dark`]. Signature preserved (R9.4).
#[must_use]
pub fn color_for_latency_ms(ms: i64) -> Color {
    if ms >= latency::WARN_MS {
        // Covers both High (≥ 2 s) and Halted (≥ 10 s) bands — same red.
        color::DOWN_500.current(ThemeMode::Dark)
    } else if ms >= latency::OK_MS {
        color::WARN_500.current(ThemeMode::Dark)
    } else {
        color::UP_500.current(ThemeMode::Dark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare two `Color`s by their byte-level RGB components. We
    /// deliberately drop alpha for the hex-pin tests because most tokens
    /// are opaque (`a = 1.0`); the alpha-blended overlay / soft tokens
    /// have dedicated tests below that probe alpha explicitly.
    fn rgb8(c: Color) -> (u8, u8, u8) {
        // `f32 → u8` is intentional: every `rgb(r, g, b)` constant is a
        // round-trip of `(byte / 255.0) * 255.0 → round() → byte`, which
        // is exact for the discrete byte values we feed in. The clippy
        // truncation/sign-loss lints fire on the lossy *general* case;
        // here the input is bounded to `[0.0, 1.0]` by construction.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let to_u8 = |x: f32| (x * 255.0).round() as u8;
        (to_u8(c.r), to_u8(c.g), to_u8(c.b))
    }

    /// T1501 — pin the load-bearing dark-mode hex values byte-for-byte
    /// against `spec/design/project/colors_and_type.css`. If any
    /// constant drifts, this test fails before a snapshot diff or a
    /// presentation screenshot ever lands. Eight tokens cover the
    /// full surface ladder + the four semantic ramps + the accent.
    #[test]
    fn t1501_palette_dark_hex_pinned() {
        // Surfaces (cool ladder).
        assert_eq!(
            rgb8(color::CANVAS.current(ThemeMode::Dark)),
            (0x13, 0x18, 0x20),
            "CANVAS dark = cool-800 #131820",
        );
        assert_eq!(
            rgb8(color::PANEL.current(ThemeMode::Dark)),
            (0x1C, 0x21, 0x27),
            "PANEL dark = cool-700 #1C2127",
        );
        assert_eq!(
            rgb8(color::PANEL_RAISED.current(ThemeMode::Dark)),
            (0x2A, 0x30, 0x38),
            "PANEL_RAISED dark = cool-600 #2A3038",
        );
        assert_eq!(
            rgb8(color::PANEL_SUNKEN.current(ThemeMode::Dark)),
            (0x0B, 0x0F, 0x15),
            "PANEL_SUNKEN dark = cool-900 #0B0F15",
        );

        // Foreground (text).
        assert_eq!(
            rgb8(color::FG_1.current(ThemeMode::Dark)),
            (0xE8, 0xEC, 0xF1),
            "FG_1 dark = #E8ECF1",
        );

        // Accent (teal) replaces the old blue.
        assert_eq!(
            rgb8(color::ACCENT.current(ThemeMode::Dark)),
            (0x6F, 0xB6, 0xAE),
            "ACCENT dark = accent-300 #6FB6AE",
        );

        // Semantic ramps (sage / clay).
        assert_eq!(
            rgb8(color::UP_500.current(ThemeMode::Dark)),
            (0x6E, 0x9B, 0x6A),
            "UP_500 dark = sage #6E9B6A",
        );
        assert_eq!(
            rgb8(color::DOWN_500.current(ThemeMode::Dark)),
            (0xC9, 0x7B, 0x5E),
            "DOWN_500 dark = clay #C97B5E",
        );
    }

    /// T1501 — pin the load-bearing light-mode hex values. Light mode is
    /// wired but not yet runtime-toggled (Q6); pinning the values here
    /// guarantees V8 (light-palette parity) lights up the day a toggle
    /// lands.
    #[test]
    fn t1501_palette_light_hex_pinned() {
        // Surfaces (warm ladder).
        assert_eq!(
            rgb8(color::CANVAS.current(ThemeMode::Light)),
            (0xF6, 0xF4, 0xEF),
            "CANVAS light = warm-50 #F6F4EF",
        );
        assert_eq!(
            rgb8(color::PANEL.current(ThemeMode::Light)),
            (0xFB, 0xFA, 0xF7),
            "PANEL light = warm-25 #FBFAF7",
        );
        assert_eq!(
            rgb8(color::PANEL_RAISED.current(ThemeMode::Light)),
            (0xFF, 0xFF, 0xFF),
            "PANEL_RAISED light = pure white",
        );
        assert_eq!(
            rgb8(color::PANEL_SUNKEN.current(ThemeMode::Light)),
            (0xEF, 0xEB, 0xE3),
            "PANEL_SUNKEN light = warm-100 #EFEBE3",
        );

        // Foreground.
        assert_eq!(
            rgb8(color::FG_1.current(ThemeMode::Light)),
            (0x14, 0x13, 0x0F),
            "FG_1 light = warm-900 #14130F",
        );

        // Accent (teal) at the darker accent-400 step on light surfaces.
        assert_eq!(
            rgb8(color::ACCENT.current(ThemeMode::Light)),
            (0x3F, 0x96, 0x8D),
            "ACCENT light = accent-400 #3F968D",
        );

        // Semantic ramps.
        assert_eq!(
            rgb8(color::UP_500.current(ThemeMode::Light)),
            (0x54, 0x7A, 0x52),
            "UP_500 light = #547A52",
        );
        assert_eq!(
            rgb8(color::DOWN_500.current(ThemeMode::Light)),
            (0xA9, 0x5F, 0x46),
            "DOWN_500 light = #A95F46",
        );
    }

    /// T1501 — every step on the spacing ladder is non-zero except
    /// `ZERO`, and the ladder is monotonically increasing. Catches a
    /// mistyped constant before any pixel lands on screen.
    #[test]
    fn t1501_spacing_ladder_complete() {
        assert_eq!(space::ZERO, 0);
        let ladder = [
            space::TICK,
            space::XXS,
            space::XS,
            space::S,
            space::M,
            space::L,
            space::L_PLUS,
            space::XL,
            space::XXL,
            space::XXXL,
            space::HUGE,
            space::MASSIVE,
        ];
        // 12 non-zero entries (13 total including ZERO).
        assert_eq!(ladder.len(), 12, "12 non-zero spacing steps");
        for (i, v) in ladder.iter().enumerate() {
            assert!(*v > 0, "spacing[{i}] = {v} must be non-zero");
        }
        // Strictly increasing.
        for i in 1..ladder.len() {
            assert!(
                ladder[i] > ladder[i - 1],
                "spacing must strictly increase: {} → {}",
                ladder[i - 1],
                ladder[i],
            );
        }
        // Pin the canonical Lumen pixel values.
        assert_eq!(ladder, [2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 64]);
    }

    /// T1501 — motion durations match Lumen's `--dur-1..4` (80, 140,
    /// 220, 320 ms).
    #[test]
    fn t1501_motion_durations_pinned() {
        assert_eq!(motion::DUR_1.as_millis(), 80, "DUR_1 = 80 ms");
        assert_eq!(motion::DUR_2.as_millis(), 140, "DUR_2 = 140 ms");
        assert_eq!(motion::DUR_3.as_millis(), 220, "DUR_3 = 220 ms");
        assert_eq!(motion::DUR_4.as_millis(), 320, "DUR_4 = 320 ms");
    }

    /// T1501 — radii ladder pins to Lumen's `--radius-1..5 + pill`.
    /// Each `radius::*` constant is a literal `f32` from a CSS pixel
    /// integer; the equality is exact by construction (no arithmetic),
    /// so the float-comparison clippy lint is suppressed at the helper.
    #[allow(clippy::float_cmp)]
    fn pin_f32(actual: f32, expected: f32, label: &str) {
        assert_eq!(actual, expected, "{label}");
    }

    #[test]
    fn t1501_radii_ladder_pinned() {
        pin_f32(radius::R1, 2.0, "R1 = 2px");
        pin_f32(radius::R2, 4.0, "R2 = 4px");
        pin_f32(radius::R3, 6.0, "R3 = 6px");
        pin_f32(radius::R4, 8.0, "R4 = 8px");
        pin_f32(radius::R5, 12.0, "R5 = 12px");
        pin_f32(radius::PILL, 999.0, "PILL = 999px");
    }

    /// T1501 — typography scale pins to Lumen's `--fs-*`.
    #[test]
    fn t1501_text_ladder_pinned() {
        assert_eq!(text::MICRO, 11);
        assert_eq!(text::SMALL, 12);
        assert_eq!(text::BODY, 13);
        assert_eq!(text::H3, 15);
        assert_eq!(text::H2, 18);
        assert_eq!(text::H1, 24);
        assert_eq!(text::DISPLAY, 32);
    }

    /// Luminance proxy — sum of integer RGB byte values. Bounded to
    /// `[0.0, 1.0]` by construction, so the lossy `f32 → u8` cast is
    /// exact for our discrete inputs.
    fn lum(c: Color) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let to_u8 = |x: f32| (x * 255.0).round() as u8;
        u32::from(to_u8(c.r)) + u32::from(to_u8(c.g)) + u32::from(to_u8(c.b))
    }

    /// R2.5 acceptance — the four surface tiers form a strict luminance
    /// ladder in dark mode (`PANEL_SUNKEN < CANVAS < PANEL <
    /// PANEL_RAISED`) so each tier reads as a distinct elevation.
    #[test]
    fn tier_token_presence_test() {
        let sunken = lum(color::PANEL_SUNKEN.current(ThemeMode::Dark));
        let canvas = lum(color::CANVAS.current(ThemeMode::Dark));
        let panel = lum(color::PANEL.current(ThemeMode::Dark));
        let raised = lum(color::PANEL_RAISED.current(ThemeMode::Dark));
        assert!(sunken < canvas, "sunken {sunken} < canvas {canvas}");
        assert!(canvas < panel, "canvas {canvas} < panel {panel}");
        assert!(panel < raised, "panel {panel} < raised {raised}");
    }

    /// R3.3 acceptance — Lumen's "shadows in dark mode are darker, not
    /// bigger" rule. The dark `shadow_1` colour has more alpha-weighted
    /// blackness than the light `shadow_1`; blur radius is identical.
    #[test]
    fn shadow_dark_is_more_black_than_light() {
        let dark = shadow::shadow_1(ThemeMode::Dark);
        let light = shadow::shadow_1(ThemeMode::Light);
        // Same blur — darker, not bigger. Both are literal `2.0` from
        // `shadow_1`, no arithmetic, so the equality is exact.
        pin_f32(
            dark.blur_radius,
            light.blur_radius,
            "shadow_1 blur same across modes",
        );
        // Dark shadow alpha (0.30) > light shadow alpha (0.04).
        assert!(
            dark.color.a > light.color.a,
            "dark alpha {} > light alpha {}",
            dark.color.a,
            light.color.a,
        );
    }

    /// T1503 — pin the three-level shadow ladder `(offset_y, blur, alpha)`
    /// against the Design table for both `ThemeMode::Dark` and
    /// `ThemeMode::Light`. Uses approx-equality (1e-4 tolerance) for `f32`
    /// comparisons to guard against rounding in future refactors.
    ///
    /// Design table (spec/tasks/lumen-phase-1-foundation.md lines 154–156):
    /// - Dark:  (1,2,0.30) / (4,10,0.35) / (12,28,0.50)
    /// - Light: (1,2,0.04) / (4,10,0.06) / (12,24,0.08)
    #[test]
    fn t1503_shadow_ladder_dark() {
        let s1 = shadow::shadow_1(ThemeMode::Dark);
        assert!(
            (s1.offset.y - 1.0).abs() < 1e-4,
            "shadow_1 dark offset_y: expected 1.0, got {}",
            s1.offset.y,
        );
        assert!(
            (s1.blur_radius - 2.0).abs() < 1e-4,
            "shadow_1 dark blur: expected 2.0, got {}",
            s1.blur_radius,
        );
        assert!(
            (s1.color.a - 0.30).abs() < 1e-4,
            "shadow_1 dark alpha: expected 0.30, got {}",
            s1.color.a,
        );

        let s2 = shadow::shadow_2(ThemeMode::Dark);
        assert!(
            (s2.offset.y - 4.0).abs() < 1e-4,
            "shadow_2 dark offset_y: expected 4.0, got {}",
            s2.offset.y,
        );
        assert!(
            (s2.blur_radius - 10.0).abs() < 1e-4,
            "shadow_2 dark blur: expected 10.0, got {}",
            s2.blur_radius,
        );
        assert!(
            (s2.color.a - 0.35).abs() < 1e-4,
            "shadow_2 dark alpha: expected 0.35, got {}",
            s2.color.a,
        );

        let s3 = shadow::shadow_3(ThemeMode::Dark);
        assert!(
            (s3.offset.y - 12.0).abs() < 1e-4,
            "shadow_3 dark offset_y: expected 12.0, got {}",
            s3.offset.y,
        );
        assert!(
            (s3.blur_radius - 28.0).abs() < 1e-4,
            "shadow_3 dark blur: expected 28.0, got {}",
            s3.blur_radius,
        );
        assert!(
            (s3.color.a - 0.50).abs() < 1e-4,
            "shadow_3 dark alpha: expected 0.50, got {}",
            s3.color.a,
        );
    }

    /// T1503 — pin shadow ladder values for `ThemeMode::Light`.
    #[test]
    fn t1503_shadow_ladder_light() {
        let s1 = shadow::shadow_1(ThemeMode::Light);
        assert!(
            (s1.offset.y - 1.0).abs() < 1e-4,
            "shadow_1 light offset_y: expected 1.0, got {}",
            s1.offset.y,
        );
        assert!(
            (s1.blur_radius - 2.0).abs() < 1e-4,
            "shadow_1 light blur: expected 2.0, got {}",
            s1.blur_radius,
        );
        assert!(
            (s1.color.a - 0.04).abs() < 1e-4,
            "shadow_1 light alpha: expected 0.04, got {}",
            s1.color.a,
        );

        let s2 = shadow::shadow_2(ThemeMode::Light);
        assert!(
            (s2.offset.y - 4.0).abs() < 1e-4,
            "shadow_2 light offset_y: expected 4.0, got {}",
            s2.offset.y,
        );
        assert!(
            (s2.blur_radius - 10.0).abs() < 1e-4,
            "shadow_2 light blur: expected 10.0, got {}",
            s2.blur_radius,
        );
        assert!(
            (s2.color.a - 0.06).abs() < 1e-4,
            "shadow_2 light alpha: expected 0.06, got {}",
            s2.color.a,
        );

        let s3 = shadow::shadow_3(ThemeMode::Light);
        assert!(
            (s3.offset.y - 12.0).abs() < 1e-4,
            "shadow_3 light offset_y: expected 12.0, got {}",
            s3.offset.y,
        );
        assert!(
            (s3.blur_radius - 24.0).abs() < 1e-4,
            "shadow_3 light blur: expected 24.0, got {}",
            s3.blur_radius,
        );
        assert!(
            (s3.color.a - 0.08).abs() < 1e-4,
            "shadow_3 light alpha: expected 0.08, got {}",
            s3.color.a,
        );
    }

    /// T1503 — `shadow_inset` returns `Color` (not `Shadow`), and the
    /// dark variant is a brighter/lighter-alpha value than the light variant
    /// (dark inset: white 3% → luminance near-max; light inset: near-black
    /// 4% → near-zero luminance).
    #[test]
    fn t1503_shadow_inset_returns_color_and_modes_distinct() {
        let dark_inset = shadow::shadow_inset(ThemeMode::Dark);
        let light_inset = shadow::shadow_inset(ThemeMode::Light);

        // Dark inset is white (rgb 255,255,255) — luminance is max.
        // Light inset is near-black (rgb 20,19,15) — luminance is low.
        // The two colours must be visually distinct.
        assert_ne!(
            rgb8(dark_inset),
            rgb8(light_inset),
            "shadow_inset must differ across modes",
        );

        // Dark inset: white at 3% alpha — R channel is 255.
        let (r_dark, g_dark, b_dark) = rgb8(dark_inset);
        assert_eq!(
            (r_dark, g_dark, b_dark),
            (0xFF, 0xFF, 0xFF),
            "dark inset colour is white (0xFF,0xFF,0xFF)"
        );
        assert!(
            (dark_inset.a - 0.03).abs() < 1e-4,
            "dark inset alpha: expected 0.03, got {}",
            dark_inset.a,
        );

        // Light inset: warm near-black at 4% alpha — barely visible.
        let (r_light, g_light, b_light) = rgb8(light_inset);
        assert_eq!(
            (r_light, g_light, b_light),
            (0x14, 0x13, 0x0F),
            "light inset colour is warm-900 (0x14,0x13,0x0F)"
        );
        assert!(
            (light_inset.a - 0.04).abs() < 1e-4,
            "light inset alpha: expected 0.04, got {}",
            light_inset.a,
        );
    }

    /// R1.2 acceptance — the light palette is wired (not just dark).
    /// The `CANVAS` / `PANEL` / `FG_1` light values must differ from the
    /// dark values, proving the dual-mode struct isn't a stub.
    #[test]
    fn light_palette_present() {
        let canvas_dark = rgb8(color::CANVAS.current(ThemeMode::Dark));
        let canvas_light = rgb8(color::CANVAS.current(ThemeMode::Light));
        assert_ne!(canvas_dark, canvas_light, "CANVAS differs across modes");

        let panel_dark = rgb8(color::PANEL.current(ThemeMode::Dark));
        let panel_light = rgb8(color::PANEL.current(ThemeMode::Light));
        assert_ne!(panel_dark, panel_light, "PANEL differs across modes");

        let fg_dark = rgb8(color::FG_1.current(ThemeMode::Dark));
        let fg_light = rgb8(color::FG_1.current(ThemeMode::Light));
        assert_ne!(fg_dark, fg_light, "FG_1 differs across modes");
    }

    /// `BORDER_STRONG` must be visibly distinct from `BORDER_1` so a
    /// keyboard-focused element can be told apart from a panel outline.
    #[test]
    fn border_strong_is_distinct_from_border() {
        assert_ne!(
            rgb8(color::BORDER_1.current(ThemeMode::Dark)),
            rgb8(color::BORDER_STRONG.current(ThemeMode::Dark)),
        );
    }

    /// `OVERLAY` must read as darker than `CANVAS` so the modal card
    /// (`PANEL_RAISED`) reads as elevated above the dimmed-out cockpit
    /// body. Overlay carries alpha; we compare its RGB luminance.
    #[test]
    fn overlay_is_darker_than_canvas() {
        let canvas = lum(color::CANVAS.current(ThemeMode::Dark));
        let overlay = lum(color::OVERLAY.current(ThemeMode::Dark));
        assert!(
            overlay < canvas,
            "OVERLAY ({overlay}) must be darker than CANVAS ({canvas})",
        );
    }

    /// `color_for_delta` returns sage / clay / muted rather than the
    /// old neon green / red / muted (R9.3 — Lumen rename).
    #[test]
    fn color_for_delta_uses_lumen_ramp() {
        use rust_decimal_macros::dec;
        let pos = color_for_delta(dec!(1));
        let neg = color_for_delta(dec!(-1));
        let zero = color_for_delta(dec!(0));
        assert_eq!(rgb8(pos), (0x6E, 0x9B, 0x6A), "positive = sage UP_500");
        assert_eq!(rgb8(neg), (0xC9, 0x7B, 0x5E), "negative = clay DOWN_500");
        assert_eq!(rgb8(zero), (0x80, 0x89, 0x93), "zero = muted FG_3");
    }

    /// `color_for_latency_ms` band reconcile (Q8 / R9.4). Halted shares
    /// the red band with High by design — the distinct labels carry the
    /// distinction, not the colour.
    #[test]
    fn color_for_latency_ms_uses_lumen_ramp() {
        let ok = color_for_latency_ms(100);
        let warn = color_for_latency_ms(1_000);
        let high = color_for_latency_ms(5_000);
        let halted = color_for_latency_ms(15_000);
        assert_eq!(rgb8(ok), (0x6E, 0x9B, 0x6A), "OK = sage UP_500");
        assert_eq!(rgb8(warn), (0xE0, 0xB4, 0x5C), "WARN = WARN_500 (dark)");
        assert_eq!(rgb8(high), (0xC9, 0x7B, 0x5E), "HIGH = clay DOWN_500");
        assert_eq!(rgb8(halted), (0xC9, 0x7B, 0x5E), "HALTED = clay DOWN_500");
    }

    /// `focus::ring` returns a 3 px blur, zero offset, accent-tinted
    /// shadow — the iced-idiomatic equivalent of Lumen's CSS
    /// `box-shadow: 0 0 0 3px rgba(...)`. The exact-equality on the
    /// shape fields is sound: offsets and blur are literal `f32` values
    /// constructed inside `focus::ring`, no arithmetic.
    #[test]
    fn focus_ring_shape() {
        let dark = focus::ring(ThemeMode::Dark);
        pin_f32(dark.offset.x, 0.0, "dark offset.x");
        pin_f32(dark.offset.y, 0.0, "dark offset.y");
        pin_f32(dark.blur_radius, 3.0, "dark blur_radius");
        // Accent-200 dark, 30% alpha.
        let (r, g, b) = rgb8(dark.color);
        assert_eq!((r, g, b), (0xA6, 0xD5, 0xCF));
        assert!((dark.color.a - 0.30).abs() < 1e-4);

        let light = focus::ring(ThemeMode::Light);
        pin_f32(light.offset.x, 0.0, "light offset.x");
        pin_f32(light.offset.y, 0.0, "light offset.y");
        pin_f32(light.blur_radius, 3.0, "light blur_radius");
        let (r, g, b) = rgb8(light.color);
        assert_eq!((r, g, b), (0x3F, 0x96, 0x8D));
        assert!((light.color.a - 0.28).abs() < 1e-4);
    }

    /// T3009 — pin the six chart-canvas-overhaul layout tokens to the
    /// architect's design table in
    /// `spec/chart-canvas-overhaul/feature.md ## Design`.  These tokens
    /// drive the price axis (M2), time axis (M3), and legend (M4) draw
    /// passes added by v1.10.0; drift on any one of them desyncs the
    /// chart canvas from `inner_rect_with_gutters` arithmetic and the
    /// legend card chrome.  Pinned at developer-pass time so an
    /// accidental rename / retune fails this test before it lands a
    /// snapshot diff.
    #[test]
    fn t3009_chart_canvas_overhaul_tokens_pinned() {
        pin_f32(
            layout::AXIS_GUTTER_PRICE_PX,
            48.0,
            "AXIS_GUTTER_PRICE_PX = 48 px",
        );
        pin_f32(
            layout::AXIS_GUTTER_RIGHT_PX,
            16.0,
            "AXIS_GUTTER_RIGHT_PX = 16 px",
        );
        pin_f32(
            layout::AXIS_GUTTER_TIME_PX,
            24.0,
            "AXIS_GUTTER_TIME_PX = 24 px",
        );
        pin_f32(
            layout::LEGEND_CARD_WIDTH_PX,
            140.0,
            "LEGEND_CARD_WIDTH_PX = 140 px",
        );
        pin_f32(
            layout::LEGEND_CARD_HEIGHT_PX,
            80.0,
            "LEGEND_CARD_HEIGHT_PX = 80 px",
        );
        pin_f32(layout::LEGEND_GLYPH_PX, 10.0, "LEGEND_GLYPH_PX = 10 px");
    }

    /// T3009 — the legend card must fit inside the chart canvas at the
    /// 1280×720 floor.  Inner-rect width at the floor (after the price
    /// gutter on the left, the right gutter, and the outer 8-px gutter
    /// on each side from `canvas_chart::inner_rect`) must comfortably
    /// host the legend's `LEGEND_CARD_WIDTH_PX + 2 × space::M` budget.
    /// Catches a token-set regression where someone bumps the gutters
    /// without checking the legend still fits.
    #[test]
    fn t3009_legend_card_fits_at_1280_floor() {
        // Cockpit chart body roughly subtracts a 180-px sidebar and a
        // `space::L` (16 px) of outer padding on each side. Be
        // conservative: the chart's outer canvas allocation at the
        // 1280-px floor is at least 1280 − 180 − 32 = 1068 px wide.
        // The legend card needs LEGEND_CARD_WIDTH_PX + the `space::M`
        // (12 px) padding on each side to read; assert the inequality.
        #[allow(clippy::cast_precision_loss)]
        let space_m = super::space::M as f32;
        #[allow(clippy::cast_precision_loss)]
        let space_s = super::space::S as f32;
        let card_budget = layout::LEGEND_CARD_WIDTH_PX + 2.0 * space_m;
        let canvas_min =
            1068.0 - layout::AXIS_GUTTER_PRICE_PX - layout::AXIS_GUTTER_RIGHT_PX - 2.0 * space_s;
        assert!(
            card_budget < canvas_min,
            "legend card_budget ({card_budget}) must fit inside floor-canvas inner-rect ({canvas_min}) at 1280×720",
        );
    }

    /// T3009 — the legend card's height accounting must clear five
    /// entries at `LEGEND_GLYPH_PX` plus inter-row breathing room and
    /// `space::S` interior padding on the top and bottom.  Catches a
    /// regression where someone shrinks `LEGEND_CARD_HEIGHT_PX` below
    /// the row arithmetic the widget assumes.
    #[test]
    fn t3009_legend_card_height_clears_five_entries() {
        const ENTRIES: f32 = 5.0;
        #[allow(clippy::cast_precision_loss)]
        let pad = super::space::S as f32;
        // Row stride = glyph height + 2-px gap; final row drops the gap.
        let rows = ENTRIES * layout::LEGEND_GLYPH_PX + (ENTRIES - 1.0) * 2.0;
        let needed = rows + 2.0 * pad;
        assert!(
            layout::LEGEND_CARD_HEIGHT_PX >= needed,
            "LEGEND_CARD_HEIGHT_PX ({}) must clear 5 entries ({needed} px)",
            layout::LEGEND_CARD_HEIGHT_PX,
        );
    }

    // ── T-D-9 — Lumen ACCENT_2..5 tokens ────────────────────────────────

    /// T-D-9 — pin `ACCENT_2..5` dark hex values verbatim from the
    /// `lumen-accent-palette-extension-2026-05-17` dev-note. Any change
    /// to these constants requires updating the dev-note and this test
    /// simultaneously so the change is deliberate.
    #[test]
    fn accent_2_to_5_dark_hex_pinned() {
        assert_eq!(
            rgb8(color::ACCENT_2.current(ThemeMode::Dark)),
            (0xA6, 0xD5, 0xCF),
            "ACCENT_2 dark = accent-200 #A6D5CF"
        );
        assert_eq!(
            rgb8(color::ACCENT_3.current(ThemeMode::Dark)),
            (0x82, 0xAE, 0xDC),
            "ACCENT_3 dark = cool-blue #82AEDC"
        );
        assert_eq!(
            rgb8(color::ACCENT_4.current(ThemeMode::Dark)),
            (0xB7, 0x9B, 0xD4),
            "ACCENT_4 dark = muted-purple #B79BD4"
        );
        assert_eq!(
            rgb8(color::ACCENT_5.current(ThemeMode::Dark)),
            (0xE0, 0xB4, 0x5C),
            "ACCENT_5 dark = amber #E0B45C"
        );
    }

    /// T-D-9 — pin `ACCENT_2..5` light hex values verbatim from the
    /// `lumen-accent-palette-extension-2026-05-17` dev-note.
    #[test]
    fn accent_2_to_5_light_hex_pinned() {
        assert_eq!(
            rgb8(color::ACCENT_2.current(ThemeMode::Light)),
            (0x2A, 0x7B, 0x73),
            "ACCENT_2 light = accent-500 #2A7B73"
        );
        assert_eq!(
            rgb8(color::ACCENT_3.current(ThemeMode::Light)),
            (0x3D, 0x6B, 0xA8),
            "ACCENT_3 light = cool-blue #3D6BA8"
        );
        assert_eq!(
            rgb8(color::ACCENT_4.current(ThemeMode::Light)),
            (0x6E, 0x4F, 0x9C),
            "ACCENT_4 light = muted-purple #6E4F9C"
        );
        assert_eq!(
            rgb8(color::ACCENT_5.current(ThemeMode::Light)),
            (0xA8, 0x84, 0x2F),
            "ACCENT_5 light = amber #A8842F"
        );
    }

    /// T-D-9 — `color::accent_palette()` returns the four tokens in
    /// positional order `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]`.
    /// Reordering the slots requires a deliberate test edit.
    #[test]
    fn accent_palette_slot_order_is_stable() {
        let palette = color::accent_palette();
        assert_eq!(palette.len(), 4);
        // Slot 0 — desaturated teal (ACCENT_2).
        assert_eq!(
            rgb8(palette[0].current(ThemeMode::Dark)),
            (0xA6, 0xD5, 0xCF),
            "slot 0 = ACCENT_2 dark"
        );
        // Slot 1 — cool blue (ACCENT_3).
        assert_eq!(
            rgb8(palette[1].current(ThemeMode::Dark)),
            (0x82, 0xAE, 0xDC),
            "slot 1 = ACCENT_3 dark"
        );
        // Slot 2 — muted purple (ACCENT_4).
        assert_eq!(
            rgb8(palette[2].current(ThemeMode::Dark)),
            (0xB7, 0x9B, 0xD4),
            "slot 2 = ACCENT_4 dark"
        );
        // Slot 3 — amber (ACCENT_5).
        assert_eq!(
            rgb8(palette[3].current(ThemeMode::Dark)),
            (0xE0, 0xB4, 0x5C),
            "slot 3 = ACCENT_5 dark"
        );
    }

    /// T-D-9 — all four accent tokens differ across modes (both modes
    /// are wired, per the `ModeColor` contract).
    #[test]
    fn accent_palette_modes_differ() {
        for (i, token) in color::accent_palette().iter().enumerate() {
            assert_ne!(
                rgb8(token.current(ThemeMode::Dark)),
                rgb8(token.current(ThemeMode::Light)),
                "ACCENT_{} dark and light must differ",
                i + 2,
            );
        }
    }

    // ── Phase C — Sidebar IA grouping (T-D-N07) ──────────────────────────────

    /// T-D-N07 — `SIDEBAR_GROUPS_PHASE_C` flattened must equal
    /// `SIDEBAR_ENTRIES_PHASE_A` element-for-element. Validates the
    /// group-composition invariant documented in Design § A2.
    #[test]
    #[allow(non_snake_case)]
    fn sidebar_groups_phase_c__flatten_matches_phase_a() {
        use crate::state::Screen;
        use layout::{SIDEBAR_ENTRIES_PHASE_A, SIDEBAR_GROUPS_PHASE_C};
        let flat: Vec<crate::state::Screen> = SIDEBAR_GROUPS_PHASE_C
            .iter()
            .flat_map(|g| g.iter())
            .copied()
            .collect();
        assert_eq!(
            flat,
            SIDEBAR_ENTRIES_PHASE_A.to_vec(),
            "SIDEBAR_GROUPS_PHASE_C flattened must equal SIDEBAR_ENTRIES_PHASE_A"
        );

        // cockpit-reports-viewer v0.1.0 (R6 / D4 / AC6) — `Reports` is in the
        // Library group, immediately between `Models` and `Trail`, in BOTH
        // consts (the lock-step the flatten check above guards). Using
        // `position(...).unwrap_or(usize::MAX)` keeps the assert panic-free
        // (no `expect`): a missing entry sorts to `MAX` and trips the ordering
        // assertion with a clear message rather than an unwrap panic.
        let models_idx = flat
            .iter()
            .position(|s| *s == Screen::Models)
            .unwrap_or(usize::MAX);
        let reports_idx = flat
            .iter()
            .position(|s| *s == Screen::Reports)
            .unwrap_or(usize::MAX);
        let trail_idx = flat
            .iter()
            .position(|s| *s == Screen::Trail)
            .unwrap_or(usize::MAX);
        assert!(
            models_idx < reports_idx && reports_idx < trail_idx,
            "Reports must sit between Models and Trail (Library group, D4); \
             got Models@{models_idx} Reports@{reports_idx} Trail@{trail_idx}"
        );
    }
}
