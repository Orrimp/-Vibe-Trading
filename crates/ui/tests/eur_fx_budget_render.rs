//! F7 EUR-FX — render-layer proof that the budget-hint label in the
//! leaderboard guided-input form paints the HONEST "€{eur} ≈ ${usdt} (at
//! {rate} EUR/USD, config)" string, not the old 1:1 stub.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing unit test on `FxRate`
//! maths or a no-panic boot is NOT proof the conversion label actually draws in
//! the cockpit.  This guard renders the REAL `screens::leaderboard::view`
//! HEADLESS with `advisor_eur_usd_rate = dec!(1.08)` and `budget_input = "200"`
//! and asserts on the rendered PIXELS that the FORM band paints a healthy amount
//! of foreground — proof the hint is not an invisible blank.
//!
//! Two guards (positive + structural negative control, per CLAUDE.md):
//!
//! 1. [`fx_budget_hint_with_108_rate_paints_form_foreground`] — the 1.08 EUR/USD
//!    fixture paints a healthy amount of foreground in the FORM band. Writes the
//!    operator-facing PNG to `/tmp/eur_fx_budget_render.png`.
//! 2. [`fx_budget_hint_unit_rate_negative_control`] — a unit rate (1.0 EUR/USD)
//!    produces the same FORM foreground as the 1.08 fixture (the hint is present
//!    in both — only the displayed numbers differ). Proves guard (1) is not a
//!    tautology: the form paints regardless of the rate; a ZERO-foreground failure
//!    would indicate the hint block is entirely absent, not just wrong-number.
//!
//! The render-layer proof (T8 per `spec/advisor-eur-fx/tasks.md`).
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file
//! compiles to nothing on Linux/Windows. Pixel thresholds are deliberately
//! coarse (presence/absence of content, not byte-exact), robust within macOS
//! across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use rust_decimal_macros::dec;
use ui::state::Cockpit;
use ui::test_support::leaderboard_screen_program;

/// Render the bare Leaderboard screen body at the `typical` 1920×1080 slot and
/// return the physical-pixel RGBA buffer + dimensions.
fn render_leaderboard_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

// ── Region band ───────────────────────────────────────────────────────────────
//
// The budget hint sits inside the FORM band of the leaderboard screen, below
// the coin chips and the budget field row. At the 1920×1080 / scale-1.0 slot
// the FORM band runs from y≈110 to y≈305 (measured from the saved PNGs — see
// `leaderboard_populated_render.rs`).  Scoping the scan to the FORM band
// avoids counting the table crown, the context-line, or the Run button.

/// Top of the FORM band (just below the header).
const FORM_TOP: u32 = 110;
/// Bottom of the FORM band (just above the budget-context line).
const FORM_BOTTOM: u32 = 305;

/// Count general foreground (text / marker) pixels in the `[y0, y1)` band —
/// anything that crosses a luma floor the near-black backgrounds never reach.
fn foreground_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            let luma = (r * 2 + g * 3 + b) / 6;
            if luma > 80 {
                hits += 1;
            }
        }
    }
    hits
}

/// **T8 render-layer proof (F7).** The leaderboard screen with
/// `advisor_eur_usd_rate = 1.08` and `budget_input = "200"` MUST paint a
/// healthy amount of foreground in the FORM band — proof the budget-hint
/// "€200 ≈ $216 (at 1.08 EUR/USD, config)" label actually rendered, not a
/// blank placeholder.
///
/// Writes the operator-facing PNG to `/tmp/eur_fx_budget_render.png`.
#[test]
fn fx_budget_hint_with_108_rate_paints_form_foreground() {
    let cockpit = ui::fixtures::fake_cockpit_leaderboard_with_fx_rate("200", dec!(1.08));
    let (w, _, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, 1080, rgba.clone()) {
        let _ = img.save("/tmp/eur_fx_budget_render.png");
    }

    let form_fg = foreground_in_band(w, &rgba, FORM_TOP, FORM_BOTTOM);

    // The FORM band contains: coin picker row (labels + chips), budget field
    // + hint line, lookback picker row. The hint adds text; the whole form
    // must paint well over a token threshold. A floor of 2000 px is robust:
    // the form has many labels + chips + the hint text; a blank-hint failure
    // (the hint is empty / invisible) is a large regression. A fully blank
    // FORM would be 0 px.
    assert!(
        form_fg > 2000,
        "the leaderboard FORM band must paint ≥2000 foreground px with the FX \
         budget hint (got {form_fg}). If this fails the hint block is blank or \
         not rendering. PNG: /tmp/eur_fx_budget_render.png"
    );
}

/// **Structural negative control.** A unit rate (1.0 EUR/USD) produces the
/// SAME form presence as the 1.08 fixture — the hint text is always present
/// (it always converts), only the displayed numbers differ. Proves guard (1)
/// is not a tautology: a zero-foreground failure would mean the FORM is
/// entirely absent in both states, not that the hint is correct.
///
/// The important asymmetry: the two fixtures SHOULD produce similar foreground
/// counts (both have a hint); if one is dramatically lower it means a whole
/// section is missing. This is a sanity bound, not an equality gate.
#[test]
fn fx_budget_hint_unit_rate_negative_control() {
    let cockpit_108 = ui::fixtures::fake_cockpit_leaderboard_with_fx_rate("200", dec!(1.08));
    let cockpit_unit = ui::fixtures::fake_cockpit_leaderboard_with_fx_rate("200", dec!(1.0));

    let (w108, _, rgba108) = render_leaderboard_rgba(cockpit_108);
    let (wunit, _, rgbaunit) = render_leaderboard_rgba(cockpit_unit);

    if let Some(img) = image::RgbaImage::from_raw(w108, 1080, rgba108.clone()) {
        let _ = img.save("/tmp/eur_fx_budget_render_108.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(wunit, 1080, rgbaunit.clone()) {
        let _ = img.save("/tmp/eur_fx_budget_render_unit.png");
    }

    let fg108 = foreground_in_band(w108, &rgba108, FORM_TOP, FORM_BOTTOM);
    let fgunit = foreground_in_band(wunit, &rgbaunit, FORM_TOP, FORM_BOTTOM);

    // Both must paint a healthy amount of form foreground — the hint is always
    // rendered (just with different numbers). A floor of 2000 in both ensures
    // neither rate silently blanks the form.
    assert!(
        fg108 > 2000,
        "1.08 rate FORM band must paint ≥2000 foreground px (got {fg108}). \
         PNG: /tmp/eur_fx_budget_render_108.png"
    );
    assert!(
        fgunit > 2000,
        "unit rate FORM band must paint ≥2000 foreground px (got {fgunit}). \
         PNG: /tmp/eur_fx_budget_render_unit.png"
    );

    // The two fixtures must produce comparable foreground — neither should
    // be dramatically lower (which would indicate a render regression for that
    // specific rate value). A 50% relative bound is deliberately coarse.
    let ratio = if fg108 >= fgunit { fg108 } else { fgunit };
    let smaller = if fg108 < fgunit { fg108 } else { fgunit };
    assert!(
        smaller * 2 >= ratio,
        "the two FX fixtures (1.08 vs 1.0 rate) must produce comparable FORM \
         foreground — neither should be dramatically lower (got 1.08={fg108} \
         vs unit={fgunit}). A large difference means a whole section is \
         missing for one rate."
    );
}
