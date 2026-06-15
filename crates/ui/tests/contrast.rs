//! WCAG 2.1 contrast asserter for Lumen design-system tokens.
//!
//! # Purpose
//! Enumerates every `(fg, bg)` token pair derived from
//! `crates/ui/src/theme.rs` and asserts WCAG 2.1 contrast ratios per
//! `spec/ui-design-principles.md ## Accessibility minimums`:
//!   - **4.5:1 AA** — body text (`ContrastClass::Body`)
//!   - **7.0:1 AAA** — equity-critical text (`ContrastClass::Equity`)
//!
//! # Alpha handling
//! `iced::Color` carries `a: f32`. This asserter ignores alpha — the WCAG
//! 2.1 formula is defined on opaque colors. Tint-backdrop tokens (`UP_50`,
//! `DOWN_50`, `WARN_50`, `INFO_50`, `ACCENT_SOFT`, `OVERLAY`) are not
//! body-text tokens and do not appear in the PAIRS table at v0.1.0. Future
//! tint-as-text usage would require an `OptOut("alpha-tinted-decorative")`
//! entry.
//!
//! # Mode
//! Controlled by the `UI_CONTRAST_MODE` env var:
//!   - `gate` (default at v0.2.0): failures collect into a `Vec` and `panic!`
//!     at end of test. The 2-week Q-DUO-WARN observation window from v0.1.0
//!     has elapsed; gate is now enforcing for all non-opt-out'd pairs.
//!   - `warn`: failures emit `eprintln!` and the test exits PASS. Set
//!     `UI_CONTRAST_MODE=warn` as the explicit opt-out escape hatch (local
//!     dev, CI-pinning).
//!
//! # Opt-out marker
//! Pairs that physically cannot meet WCAG (disabled text tier, chart-line
//! strokes, decorative borders) carry `ContrastClass::OptOut(&'static str)`
//! with a mandatory reason string. The `OPT_OUTS` const table at the bottom
//! of this file records the 9 design-intent entries per D-CONT-4. The
//! in-PAIRS `OptOut` class entries (24 chart-line + comparison-stroke)
//! carry their reason directly in the `ContrastPair.class` field.
//!
//! **The `OPT_OUTS` table and the in-PAIRS `OptOut` entries are the
//! asserter's ONLY configuration surface. New opt-outs require touching
//! this one file (analyst → architect → developer review loop).**

use iced::Color;
use ui::theme::color;

// ── WCAG 2.1 hand-rolled formula ─────────────────────────────────────────────
//
// Spec refs:
//   https://www.w3.org/WAI/GL/wiki/Relative_luminance
//   https://www.w3.org/TR/WCAG21/#dfn-contrast-ratio

/// sRGB channel [0,1] → linearized luminance per WCAG 2.1.
fn linearize(c: f32) -> f64 {
    let c = c as f64;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance per W3C "Relative luminance" definition.
fn relative_luminance(color: Color) -> f64 {
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

/// Contrast ratio per WCAG 2.1: `(L_lighter + 0.05) / (L_darker + 0.05)`.
/// Returns a value in `[1.0, 21.0]` (1.0 = identical; 21.0 = white on black).
fn contrast_ratio(fg: Color, bg: Color) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// WCAG threshold class per `spec/ui-design-principles.md ## Accessibility minimums`.
#[derive(Debug, Clone, Copy)]
enum ContrastClass {
    /// WCAG 2.1 AA body text — assert ratio ≥ 4.5.
    Body,
    /// WCAG 2.1 AAA equity-critical text — assert ratio ≥ 7.0.
    Equity,
    /// Skip with mandatory reason — logged at runtime for audit.
    OptOut(&'static str),
}

/// A single (fg, bg) pair to contrast-check.
#[derive(Debug, Clone, Copy)]
struct ContrastPair {
    pair_id: &'static str,
    fg: Color,
    bg: Color,
    class: ContrastClass,
}

// ── Mode selector ─────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Warn,
    Gate,
}

/// Returns `Mode::Gate` unless `UI_CONTRAST_MODE=warn`.
///
/// v0.2.0: gate is now the default. Set `UI_CONTRAST_MODE=warn` to opt out
/// (local dev, CI-pinning). The v0.1.0 default arm was `_ => Mode::Warn`.
fn current_mode() -> Mode {
    match std::env::var("UI_CONTRAST_MODE").as_deref() {
        Ok("warn") => Mode::Warn,
        _ => Mode::Gate,
    }
}

// ── MIN_PAIRS floor ───────────────────────────────────────────────────────────

/// Floor for the PAIRS table count. Defends against K2 — a future
/// theme.rs refactor that re-shapes token storage and silently breaks
/// PAIRS enumeration. The v0.1.0 PAIRS table contains 83 entries; 60 is a
/// comfortable floor that allows normal palette evolution (token removal /
/// class downgrade) without bumping the floor in tasks.md, while still
/// catching a catastrophic enumeration-shape break that drops the count to ~0.
const MIN_PAIRS: usize = 60;

// ── PAIRS table ───────────────────────────────────────────────────────────────
//
// 83 entries organized per D-CONT-3 architect audit table (2026-05-29).
// Groups:
//   A. FG_1..4 × {CANVAS, PANEL, PANEL_RAISED, PANEL_SUNKEN} × {Dark, Light}   = 32
//   B. FG_1 × {CANVAS, PANEL} × {Dark, Light} (Equity class duplicates)         = 4
//   C. FG_ON_ACCENT × {ACCENT, ACCENT_HOVER, ACCENT_PRESS} × {Dark, Light}      = 6
//   D. {UP_500, DOWN_500, WARN_500, INFO_500} × {CANVAS, PANEL} × {Dark, Light} = 16
//   E. {UP_400, DOWN_400} × {CANVAS, PANEL} × {Dark, Light}                     = 8  (OptOut)
//   F. {ACCENT_2..5} × {CANVAS, PANEL} × {Dark, Light}                          = 16 (OptOut)
//   G. BORDER_STRONG × CANVAS × Dark                                             = 1  (OptOut)
// Total: 83

const PAIRS: &[ContrastPair] = &[
    // ── Group A: FG_* × surface tiers — DARK mode ─────────────────────────

    // FG_1 dark
    ContrastPair {
        pair_id: "fg_1_on_canvas_dark",
        fg: color::FG_1.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_1_on_panel_dark",
        fg: color::FG_1.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_1_on_panel_raised_dark",
        fg: color::FG_1.dark,
        bg: color::PANEL_RAISED.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_1_on_panel_sunken_dark",
        fg: color::FG_1.dark,
        bg: color::PANEL_SUNKEN.dark,
        class: ContrastClass::Body,
    },
    // FG_2 dark
    ContrastPair {
        pair_id: "fg_2_on_canvas_dark",
        fg: color::FG_2.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_2_on_panel_dark",
        fg: color::FG_2.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_2_on_panel_raised_dark",
        fg: color::FG_2.dark,
        bg: color::PANEL_RAISED.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_2_on_panel_sunken_dark",
        fg: color::FG_2.dark,
        bg: color::PANEL_SUNKEN.dark,
        class: ContrastClass::Body,
    },
    // FG_3 dark
    ContrastPair {
        pair_id: "fg_3_on_canvas_dark",
        fg: color::FG_3.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_3_on_panel_dark",
        fg: color::FG_3.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_3_on_panel_raised_dark",
        fg: color::FG_3.dark,
        bg: color::PANEL_RAISED.dark,
        class: ContrastClass::OptOut(
            "sub-AA dark-mode pair ratified as v0.2.0 opt-out debt; \
             candidate for a future dedicated palette-tune (4 are trivially darkenable \
             — see analyst per-pair table)",
        ),
    },
    ContrastPair {
        pair_id: "fg_3_on_panel_sunken_dark",
        fg: color::FG_3.dark,
        bg: color::PANEL_SUNKEN.dark,
        class: ContrastClass::Body,
    },
    // FG_4 dark — OptOut (disabled-text-tier per WCAG 2.1 § 1.4.3 inactive UI exception)
    ContrastPair {
        pair_id: "fg_4_on_canvas_dark",
        fg: color::FG_4.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    ContrastPair {
        pair_id: "fg_4_on_panel_dark",
        fg: color::FG_4.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    ContrastPair {
        pair_id: "fg_4_on_panel_raised_dark",
        fg: color::FG_4.dark,
        bg: color::PANEL_RAISED.dark,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    ContrastPair {
        pair_id: "fg_4_on_panel_sunken_dark",
        fg: color::FG_4.dark,
        bg: color::PANEL_SUNKEN.dark,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    // ── Group A: FG_* × surface tiers — LIGHT mode ────────────────────────

    // FG_1 light
    ContrastPair {
        pair_id: "fg_1_on_canvas_light",
        fg: color::FG_1.light,
        bg: color::CANVAS.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_1_on_panel_light",
        fg: color::FG_1.light,
        bg: color::PANEL.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_1_on_panel_raised_light",
        fg: color::FG_1.light,
        bg: color::PANEL_RAISED.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_1_on_panel_sunken_light",
        fg: color::FG_1.light,
        bg: color::PANEL_SUNKEN.light,
        class: ContrastClass::Body,
    },
    // FG_2 light
    ContrastPair {
        pair_id: "fg_2_on_canvas_light",
        fg: color::FG_2.light,
        bg: color::CANVAS.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_2_on_panel_light",
        fg: color::FG_2.light,
        bg: color::PANEL.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_2_on_panel_raised_light",
        fg: color::FG_2.light,
        bg: color::PANEL_RAISED.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_2_on_panel_sunken_light",
        fg: color::FG_2.light,
        bg: color::PANEL_SUNKEN.light,
        class: ContrastClass::Body,
    },
    // FG_3 light
    ContrastPair {
        pair_id: "fg_3_on_canvas_light",
        fg: color::FG_3.light,
        bg: color::CANVAS.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_3_on_panel_light",
        fg: color::FG_3.light,
        bg: color::PANEL.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_3_on_panel_raised_light",
        fg: color::FG_3.light,
        bg: color::PANEL_RAISED.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_3_on_panel_sunken_light",
        fg: color::FG_3.light,
        bg: color::PANEL_SUNKEN.light,
        class: ContrastClass::Body,
    },
    // FG_4 light — OptOut (disabled-text-tier)
    ContrastPair {
        pair_id: "fg_4_on_canvas_light",
        fg: color::FG_4.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    ContrastPair {
        pair_id: "fg_4_on_panel_light",
        fg: color::FG_4.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    ContrastPair {
        pair_id: "fg_4_on_panel_raised_light",
        fg: color::FG_4.light,
        bg: color::PANEL_RAISED.light,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    ContrastPair {
        pair_id: "fg_4_on_panel_sunken_light",
        fg: color::FG_4.light,
        bg: color::PANEL_SUNKEN.light,
        class: ContrastClass::OptOut("disabled-text-tier"),
    },
    // ── Group B: FG_1 Equity-class duplicates ─────────────────────────────
    //
    // Per D-CONT-3: FG_1 × {CANVAS, PANEL} × {Dark, Light} as AAA Equity class.
    // These are additional entries (the Body entries above are separate rows).
    ContrastPair {
        pair_id: "fg_1_equity_on_canvas_dark",
        fg: color::FG_1.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Equity,
    },
    ContrastPair {
        pair_id: "fg_1_equity_on_panel_dark",
        fg: color::FG_1.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Equity,
    },
    ContrastPair {
        pair_id: "fg_1_equity_on_canvas_light",
        fg: color::FG_1.light,
        bg: color::CANVAS.light,
        class: ContrastClass::Equity,
    },
    ContrastPair {
        pair_id: "fg_1_equity_on_panel_light",
        fg: color::FG_1.light,
        bg: color::PANEL.light,
        class: ContrastClass::Equity,
    },
    // ── Group C: FG_ON_ACCENT × accent fills × {Dark, Light} ─────────────
    ContrastPair {
        pair_id: "fg_on_accent_on_accent_dark",
        fg: color::FG_ON_ACCENT.dark,
        bg: color::ACCENT.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_on_accent_on_accent_hover_dark",
        fg: color::FG_ON_ACCENT.dark,
        bg: color::ACCENT_HOVER.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_on_accent_on_accent_press_dark",
        fg: color::FG_ON_ACCENT.dark,
        bg: color::ACCENT_PRESS.dark,
        class: ContrastClass::Body,
    },
    // Light mode accent — genuine WCAG-AA defect surfaced by M-T1 dry-run:
    //   FG_ON_ACCENT.light (pure white) on ACCENT.light (#3F968D) = 3.52:1
    //   Ratified as v0.2.0 opt-out debt per operator path (A) decision.
    ContrastPair {
        pair_id: "fg_on_accent_on_accent_light",
        fg: color::FG_ON_ACCENT.light,
        bg: color::ACCENT.light,
        class: ContrastClass::OptOut(
            "sub-AA light-mode pair ratified as v0.2.0 opt-out debt; \
             candidate for a future dedicated palette-tune (4 are trivially darkenable \
             — see analyst per-pair table)",
        ),
    },
    ContrastPair {
        pair_id: "fg_on_accent_on_accent_hover_light",
        fg: color::FG_ON_ACCENT.light,
        bg: color::ACCENT_HOVER.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "fg_on_accent_on_accent_press_light",
        fg: color::FG_ON_ACCENT.light,
        bg: color::ACCENT_PRESS.light,
        class: ContrastClass::Body,
    },
    // ── Group D: Semantic ramp {UP_500, DOWN_500, WARN_500, INFO_500} × {CANVAS, PANEL} × {Dark, Light} ──

    // UP_500
    ContrastPair {
        pair_id: "up_500_on_canvas_dark",
        fg: color::UP_500.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "up_500_on_panel_dark",
        fg: color::UP_500.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    // Light — marginal sub-AA: up_500_on_canvas_light = ~4.46 per M-T1 dry-run
    // Ratified as v0.2.0 opt-out debt per operator path (A) decision.
    ContrastPair {
        pair_id: "up_500_on_canvas_light",
        fg: color::UP_500.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut(
            "sub-AA light-mode pair ratified as v0.2.0 opt-out debt; \
             candidate for a future dedicated palette-tune (4 are trivially darkenable \
             — see analyst per-pair table)",
        ),
    },
    ContrastPair {
        pair_id: "up_500_on_panel_light",
        fg: color::UP_500.light,
        bg: color::PANEL.light,
        class: ContrastClass::Body,
    },
    // DOWN_500
    ContrastPair {
        pair_id: "down_500_on_canvas_dark",
        fg: color::DOWN_500.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "down_500_on_panel_dark",
        fg: color::DOWN_500.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    // Light — marginal sub-AA: down_500_on_canvas_light = ~4.33 per M-T1 dry-run
    // Ratified as v0.2.0 opt-out debt per operator path (A) decision.
    ContrastPair {
        pair_id: "down_500_on_canvas_light",
        fg: color::DOWN_500.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut(
            "sub-AA light-mode pair ratified as v0.2.0 opt-out debt; \
             candidate for a future dedicated palette-tune (4 are trivially darkenable \
             — see analyst per-pair table)",
        ),
    },
    ContrastPair {
        pair_id: "down_500_on_panel_light",
        fg: color::DOWN_500.light,
        bg: color::PANEL.light,
        class: ContrastClass::Body,
    },
    // WARN_500
    ContrastPair {
        pair_id: "warn_500_on_canvas_dark",
        fg: color::WARN_500.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "warn_500_on_panel_dark",
        fg: color::WARN_500.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    // Light — genuine WCAG-AA defects per M-T1 dry-run:
    //   warn_500_on_canvas_light = 2.96:1
    //   warn_500_on_panel_light  = 3.11:1
    //   Ratified as v0.2.0 opt-out debt per operator path (A) decision:
    //   amber-on-light cannot reach 4.5:1 AA without abandoning the amber semantic.
    ContrastPair {
        pair_id: "warn_500_on_canvas_light",
        fg: color::WARN_500.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut(
            "amber-on-light cannot reach 4.5:1 AA without abandoning the amber semantic \
             — ratified light-mode debt",
        ),
    },
    ContrastPair {
        pair_id: "warn_500_on_panel_light",
        fg: color::WARN_500.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut(
            "amber-on-light cannot reach 4.5:1 AA without abandoning the amber semantic \
             — ratified light-mode debt",
        ),
    },
    // INFO_500
    ContrastPair {
        pair_id: "info_500_on_canvas_dark",
        fg: color::INFO_500.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "info_500_on_panel_dark",
        fg: color::INFO_500.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "info_500_on_canvas_light",
        fg: color::INFO_500.light,
        bg: color::CANVAS.light,
        class: ContrastClass::Body,
    },
    ContrastPair {
        pair_id: "info_500_on_panel_light",
        fg: color::INFO_500.light,
        bg: color::PANEL.light,
        class: ContrastClass::Body,
    },
    // ── Group E: {UP_400, DOWN_400} × {CANVAS, PANEL} × {Dark, Light} — OptOut ──
    //
    // Chart-line strokes; non-text by design per ui-design-principles.
    // Kept in PAIRS for audit-of-exclusions logging per R4.3.
    ContrastPair {
        pair_id: "up_400_on_canvas_dark",
        fg: color::UP_400.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "up_400_on_panel_dark",
        fg: color::UP_400.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "up_400_on_canvas_light",
        fg: color::UP_400.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "up_400_on_panel_light",
        fg: color::UP_400.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "down_400_on_canvas_dark",
        fg: color::DOWN_400.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "down_400_on_panel_dark",
        fg: color::DOWN_400.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "down_400_on_canvas_light",
        fg: color::DOWN_400.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "down_400_on_panel_light",
        fg: color::DOWN_400.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("chart-line-stroke-not-text"),
    },
    // ── Group F: {ACCENT_2..5} × {CANVAS, PANEL} × {Dark, Light} — OptOut ──
    //
    // Comparison-overlay strokes per `accent_palette()` doc comment.
    // Non-text by design. Kept in PAIRS for audit-of-exclusions per R4.3.

    // ACCENT_2
    ContrastPair {
        pair_id: "accent_2_on_canvas_dark",
        fg: color::ACCENT_2.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_2_on_panel_dark",
        fg: color::ACCENT_2.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_2_on_canvas_light",
        fg: color::ACCENT_2.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_2_on_panel_light",
        fg: color::ACCENT_2.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    // ACCENT_3
    ContrastPair {
        pair_id: "accent_3_on_canvas_dark",
        fg: color::ACCENT_3.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_3_on_panel_dark",
        fg: color::ACCENT_3.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_3_on_canvas_light",
        fg: color::ACCENT_3.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_3_on_panel_light",
        fg: color::ACCENT_3.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    // ACCENT_4
    ContrastPair {
        pair_id: "accent_4_on_canvas_dark",
        fg: color::ACCENT_4.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_4_on_panel_dark",
        fg: color::ACCENT_4.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_4_on_canvas_light",
        fg: color::ACCENT_4.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_4_on_panel_light",
        fg: color::ACCENT_4.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    // ACCENT_5
    ContrastPair {
        pair_id: "accent_5_on_canvas_dark",
        fg: color::ACCENT_5.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_5_on_panel_dark",
        fg: color::ACCENT_5.dark,
        bg: color::PANEL.dark,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_5_on_canvas_light",
        fg: color::ACCENT_5.light,
        bg: color::CANVAS.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    ContrastPair {
        pair_id: "accent_5_on_panel_light",
        fg: color::ACCENT_5.light,
        bg: color::PANEL.light,
        class: ContrastClass::OptOut("chart-comparison-stroke-not-text"),
    },
    // ── Group G: BORDER_STRONG × CANVAS × Dark — OptOut ──────────────────
    //
    // Hairline border decoration per ui-design-principles ## Tier elevation model.
    ContrastPair {
        pair_id: "border_strong_on_canvas_dark",
        fg: color::BORDER_STRONG.dark,
        bg: color::CANVAS.dark,
        class: ContrastClass::OptOut("border-not-text"),
    },
];

// ── Design-intent opt-out table (D-CONT-4) ────────────────────────────────────
//
// 15 entries (v0.2.0): 8× FG_4 disabled-tier + 1× BORDER_STRONG decorative
// + 6× v0.2.0 sub-AA pairs ratified as opt-out debt (operator path A).
// These are the same pair_ids as the OPT_OUT-classed entries in PAIRS above.
// This table serves as the compile-time manifest for code-review auditability
// — reviewers can diff this table to track opt-out growth.

#[derive(Debug)]
struct OptOutEntry {
    pair_id: &'static str,
    reason: &'static str,
}

const OPT_OUTS: &[OptOutEntry] = &[
    // FG_4 placeholder/disabled tier — sub-AA by design per
    // ui-design-principles ## Color palette "FG_4 — placeholder / disabled".
    // Disabled text MAY be sub-AA per WCAG 2.1 § 1.4.3
    // ("inactive UI components" exception).
    OptOutEntry {
        pair_id: "fg_4_on_canvas_dark",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_panel_dark",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_panel_raised_dark",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_panel_sunken_dark",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_canvas_light",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_panel_light",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_panel_raised_light",
        reason: "disabled-text-tier",
    },
    OptOutEntry {
        pair_id: "fg_4_on_panel_sunken_light",
        reason: "disabled-text-tier",
    },
    // Border decoration — non-text hairline divider per
    // ui-design-principles ## Tier elevation model.
    OptOutEntry {
        pair_id: "border_strong_on_canvas_dark",
        reason: "border-not-text",
    },
    // v0.2.0 sub-AA pairs ratified as documented opt-out debt (operator path A).
    // 4 trivially-darkenable pairs — future dedicated palette-tune feature expected
    // to retire these entries and re-class them Body.
    OptOutEntry {
        pair_id: "up_500_on_canvas_light",
        reason: "sub-AA light-mode pair ratified as v0.2.0 opt-out debt; \
                 candidate for a future dedicated palette-tune (4 are trivially darkenable \
                 — see analyst per-pair table)",
    },
    OptOutEntry {
        pair_id: "down_500_on_canvas_light",
        reason: "sub-AA light-mode pair ratified as v0.2.0 opt-out debt; \
                 candidate for a future dedicated palette-tune (4 are trivially darkenable \
                 — see analyst per-pair table)",
    },
    OptOutEntry {
        pair_id: "fg_3_on_panel_raised_dark",
        reason: "sub-AA dark-mode pair ratified as v0.2.0 opt-out debt; \
                 candidate for a future dedicated palette-tune (4 are trivially darkenable \
                 — see analyst per-pair table)",
    },
    OptOutEntry {
        pair_id: "fg_on_accent_on_accent_light",
        reason: "sub-AA light-mode pair ratified as v0.2.0 opt-out debt; \
                 candidate for a future dedicated palette-tune (4 are trivially darkenable \
                 — see analyst per-pair table)",
    },
    // 2 hard amber-on-light pairs — cannot reach 4.5:1 AA without abandoning the
    // amber semantic; ratified as permanent light-mode debt.
    OptOutEntry {
        pair_id: "warn_500_on_panel_light",
        reason: "amber-on-light cannot reach 4.5:1 AA without abandoning the amber semantic \
                 — ratified light-mode debt",
    },
    OptOutEntry {
        pair_id: "warn_500_on_canvas_light",
        reason: "amber-on-light cannot reach 4.5:1 AA without abandoning the amber semantic \
                 — ratified light-mode debt",
    },
];

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Verify that `OPT_OUTS` is non-empty and each entry has a non-empty reason.
/// This is a compile-time safety net — the `OptOut(&str)` variant mandates
/// the argument but we also check the manifest table.
#[test]
fn opt_outs_all_have_reasons() {
    for entry in OPT_OUTS {
        assert!(
            !entry.reason.is_empty(),
            "OPT_OUTS entry '{}' has an empty reason string",
            entry.pair_id
        );
    }
}

/// Reference vector — WHITE on BLACK = 21.00 (WCAG 2.1 maximum).
#[test]
fn ref_vector_white_on_black_is_21() {
    let ratio = contrast_ratio(Color::WHITE, Color::BLACK);
    assert!(
        (ratio - 21.0).abs() < 0.01,
        "expected WHITE on BLACK = 21.0, got {ratio:.4}"
    );
}

/// Reference vector — BLACK on WHITE = 21.00 (symmetric).
#[test]
fn ref_vector_black_on_white_is_21() {
    let ratio = contrast_ratio(Color::BLACK, Color::WHITE);
    assert!(
        (ratio - 21.0).abs() < 0.01,
        "expected BLACK on WHITE = 21.0, got {ratio:.4}"
    );
}

/// Reference vector — #777777 on #FFFFFF = 4.4781 per WCAG 2.1 reference.
#[test]
fn ref_vector_777_on_fff_is_4_48() {
    let fg = Color::from_rgb(
        0x77 as f32 / 255.0,
        0x77 as f32 / 255.0,
        0x77 as f32 / 255.0,
    );
    let bg = Color::WHITE;
    let ratio = contrast_ratio(fg, bg);
    assert!(
        (ratio - 4.4781).abs() < 0.01,
        "expected #777 on #FFF = 4.4781, got {ratio:.4}"
    );
}

/// Reference vector — #888888 on #000000 = 5.9240 per WCAG 2.1 reference.
#[test]
fn ref_vector_888_on_000_is_5_92() {
    let fg = Color::from_rgb(
        0x88 as f32 / 255.0,
        0x88 as f32 / 255.0,
        0x88 as f32 / 255.0,
    );
    let bg = Color::BLACK;
    let ratio = contrast_ratio(fg, bg);
    assert!(
        (ratio - 5.9240).abs() < 0.01,
        "expected #888 on #000 = 5.9240, got {ratio:.4}"
    );
}

/// Floor assertion — defends against K2 (silent enumeration break).
///
/// If a future theme.rs refactor re-shapes token storage and breaks
/// PAIRS enumeration, the count drops toward 0 and this test catches it.
#[test]
fn pairs_table_meets_minimum_count() {
    assert!(
        PAIRS.len() >= MIN_PAIRS,
        "theme token enumeration detected only {} pairs; \
         refactor likely broke enumeration (MIN_PAIRS = {})",
        PAIRS.len(),
        MIN_PAIRS,
    );
}

/// Main asserter — iterates all PAIRS and asserts per-class WCAG thresholds.
///
/// In gate mode (default at v0.2.0): failures panic at end of test.
/// In WARN mode (`UI_CONTRAST_MODE=warn`): failures emit `eprintln!` and the
/// test PASSES. Set `UI_CONTRAST_MODE=warn` as the explicit opt-out escape hatch.
///
/// At v0.2.0 the 6 known sub-AA pairs are ratified as `OptOut` entries (operator
/// path A). The gate ENFORCES for all other `Body`/`Equity` pairs. Expect opt-out
/// audit lines (NOT failures) for:
///   - `fg_3_on_panel_raised_dark`    (3.75 — dark-mode debt, trivially darkenable)
///   - `fg_on_accent_on_accent_light` (3.52 — light-mode debt, trivially darkenable)
///   - `up_500_on_canvas_light`       (4.46 — light-mode debt, trivially darkenable)
///   - `down_500_on_canvas_light`     (4.33 — light-mode debt, trivially darkenable)
///   - `warn_500_on_canvas_light`     (2.96 — amber-on-light ratified debt)
///   - `warn_500_on_panel_light`      (3.11 — amber-on-light ratified debt)
#[test]
fn all_theme_pairs_meet_wcag() {
    let mode = current_mode();
    let mut violations: Vec<String> = Vec::new();

    for pair in PAIRS {
        let ratio = contrast_ratio(pair.fg, pair.bg);

        match pair.class {
            ContrastClass::Body => {
                let threshold = 4.5_f64;
                if ratio < threshold {
                    let msg = format!(
                        "  {} = {:.2} < threshold {:.1}",
                        pair.pair_id, ratio, threshold
                    );
                    if mode == Mode::Gate {
                        violations.push(msg);
                    } else {
                        eprintln!(
                            "WARN: contrast pair {} = {:.2} < threshold {:.1}",
                            pair.pair_id, ratio, threshold
                        );
                    }
                }
            }
            ContrastClass::Equity => {
                let threshold = 7.0_f64;
                if ratio < threshold {
                    let msg = format!(
                        "  {} = {:.2} < threshold {:.1}",
                        pair.pair_id, ratio, threshold
                    );
                    if mode == Mode::Gate {
                        violations.push(msg);
                    } else {
                        eprintln!(
                            "WARN: contrast pair {} = {:.2} < threshold {:.1}",
                            pair.pair_id, ratio, threshold
                        );
                    }
                }
            }
            ContrastClass::OptOut(reason) => {
                // Always log opt-outs for audit-of-exclusions per R4.3.
                eprintln!(
                    "opt-out: {}; reason: {}; ratio: {:.2}",
                    pair.pair_id, reason, ratio
                );
            }
        }
    }

    if !violations.is_empty() {
        panic!("contrast assertion failed:\n{}", violations.join("\n"));
    }
}

// ── Falsification probes (P-CONT-1) ──────────────────────────────────────────
//
// Run these with `cargo test -p ui --test contrast -- --nocapture` to observe
// the expected behavior. Both are marked `#[ignore]` — they require manual
// insertion of the probe entry + running in gate mode to see the panic.
//
// P-CONT-1.A — Deliberately-low-contrast pair probe.
//   Expected gate-mode stderr: panic "contrast assertion failed:\n  probe_low_contrast_white_on_pale_grey = 1.07 < threshold 4.5"
//   Expected warn-mode stderr: "WARN: contrast pair probe_low_contrast_white_on_pale_grey = 1.07 < threshold 4.5"
//   After observing: revert the probe insertion and rerun.
//
// P-CONT-1.B — MIN_PAIRS floor probe.
//   Comment out 25 PAIRS entries; run `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast`.
//   Expected: panic "theme token enumeration detected only 58 pairs; refactor likely broke enumeration (MIN_PAIRS = 60)"
//   After observing: revert.

/// Falsification probe P-CONT-1.A — verifies gate mode rejects a
/// deliberately-low-contrast pair.
///
/// To run: temporarily prepend the probe entry to PAIRS, then:
///   `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast -- --nocapture --include-ignored probe_low_contrast_rejects_in_gate_mode`
///
/// Expected: panic with "probe_low_contrast_white_on_pale_grey = 1.07 < threshold 4.5"
///
/// After verifying, revert the probe entry and rerun `cargo test -p ui --test contrast`.
#[test]
#[ignore]
fn probe_low_contrast_rejects_in_gate_mode() {
    // This test is a documentation probe — the actual check is inside
    // `all_theme_pairs_meet_wcag`. Running it in gate mode WITH the probe
    // entry temporarily inserted into PAIRS causes `all_theme_pairs_meet_wcag`
    // to panic. This test merely documents the recipe.
    //
    // Inline demonstration: compute ratio of the probe pair to confirm formula.
    let fg = Color::WHITE;
    let bg = Color::from_rgb(0.9, 0.9, 0.9);
    let ratio = contrast_ratio(fg, bg);
    // Ratio should be ~1.07 (very low contrast — same luminance family).
    assert!(
        ratio < 2.0,
        "probe ratio {ratio:.2} is unexpectedly high — formula may be wrong"
    );
    eprintln!("probe: WHITE on pale-grey (#E6E6E6) = {ratio:.2} (expected ~1.07)",);
}

/// Falsification probe P-CONT-1.B — verifies the MIN_PAIRS floor fires.
///
/// To run: comment out 25 PAIRS entries, then:
///   `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast -- --nocapture`
///
/// Expected: panic "theme token enumeration detected only 58 pairs;
///           refactor likely broke enumeration (MIN_PAIRS = 60)".
///
/// After verifying, revert.
#[test]
#[ignore]
fn probe_min_pairs_floor_fires_when_pairs_truncated() {
    // Documentation probe only. The real check is `pairs_table_meets_minimum_count`.
    // Manually comment 25 PAIRS entries before running `pairs_table_meets_minimum_count`
    // in gate mode to observe the panic.
    let current = PAIRS.len();
    eprintln!("current PAIRS count: {current} (MIN_PAIRS = {MIN_PAIRS})");
    assert!(
        current >= MIN_PAIRS,
        "pairs table too small: {current} < {MIN_PAIRS}"
    );
}
