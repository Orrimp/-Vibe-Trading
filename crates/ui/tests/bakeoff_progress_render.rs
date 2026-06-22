//! advisor-bakeoff-progress — render-layer proof of the DETERMINATE bake-off
//! progress bar in the cockpit Leaderboard (the operator's headline ask).
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model state, a text
//! `.snap`, or a no-panic boot is NOT proof the bar draws. This guard renders
//! the REAL `screens::leaderboard::view` HEADLESS with a bake-off IN FLIGHT and
//! a candidate-level `BakeoffProgress { done: 3, total: 7, current_id:
//! "v0.5.bbands" }` set, and asserts on the rendered PIXELS that the determinate
//! progress bar (its `ACCENT_2` fill + the "Running … — 4 of 7" label) actually
//! paints BENEATH the input panel — with a NEGATIVE CONTROL (the not-running
//! state paints NO progress bar there).
//!
//! The channel→state path (the "last mile" wiring) is proved separately by
//! `bakeoff_progress_relay.rs`; THIS file proves the populated state draws.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are coarse (presence/absence of
//! a hue, not byte-exact), robust within macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

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

// ── Region band ────────────────────────────────────────────────────────────────
//
// The progress strip is inserted between the "Plan your bake-off" input panel
// and the budget-context line. Measured from the saved PNG: the form panel ends
// ~y305, the strip's LABEL paints ~y327, and the BAR FILL paints ~y341–348 (an
// 8px-tall track). The STRIP_FILL band is scoped tightly to the bar rows so the
// budget-context text below (~y369 in the running frame) never confounds the
// ACCENT_2 fill scan. A separate, wider LABEL band carries the label foreground.

/// Top of the BAR-FILL band — the 8px determinate track.
const FILL_TOP: u32 = 338;
/// Bottom of the BAR-FILL band.
const FILL_BOTTOM: u32 = 352;

/// Top of the LABEL band — the "Running … — 4 of 7" line above the bar.
const LABEL_TOP: u32 = 320;
/// Bottom of the LABEL band (just above the bar fill).
const LABEL_BOTTOM: u32 = 338;

/// `true` for an `ACCENT_2`-fill (#A6D5CF — R166 G213 B207) pixel: the lighter
/// TEAL of the progress-bar FILL. The teal-ness gates `(g - r) > 20` AND
/// `(b - r) > 12` so near-white TEXT (where r ≈ g ≈ b — the title, the
/// budget-context line) is EXCLUDED, and the `r > 130` floor excludes the form
/// chips' darker `ACCENT` (#6FB6AE — R111). So this scan tracks the bar's fill
/// specifically, not chrome or text.
fn is_accent2_fill(r: i32, g: i32, b: i32) -> bool {
    r > 130 && g > 150 && b > 150 && (g - r) > 20 && (b - r) > 12 && (g - b).abs() < 30
}

/// Count `ACCENT_2`-fill pixels in the `[y0, y1)` row band.
fn accent2_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_accent2_fill(r, g, b) {
                hits += 1;
            }
        }
    }
    hits
}

/// Count general foreground (text) pixels in the `[y0, y1)` band — anything that
/// crosses a luma floor the near-black background tiers never reach.
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

/// The progress-bar `ACCENT_2` fill in the BAR-FILL band.
fn strip_fill_pixels(w: u32, rgba: &[u8]) -> u64 {
    accent2_in_band(w, rgba, FILL_TOP, FILL_BOTTOM)
}

/// Foreground (the "Running … — 4 of 7" label) in the LABEL band.
fn strip_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, LABEL_TOP, LABEL_BOTTOM)
}

/// **The render-layer guard.** A bake-off IN FLIGHT with a candidate-level
/// `BakeoffProgress { done: 3, total: 7, current_id: "v0.5.bbands" }` MUST paint,
/// in the cockpit Leaderboard, BENEATH the input panel:
/// - the determinate bar's `ACCENT_2` fill (the bar drew with a real fraction —
///   ~3/7 of the full-width track);
/// - foreground text (the "Running v0.5.bbands — 4 of 7" label).
///
/// Writes the operator-facing PNG to `/tmp/bakeoff_progress_render.png`.
#[test]
fn bakeoff_progress_bar_paints_beneath_input_panel() {
    let cockpit = ui::fixtures::fake_cockpit_leaderboard_running_progress(3, 7, "v0.5.bbands");
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/bakeoff_progress_render.png");
    }

    let fill = strip_fill_pixels(w, &rgba);
    let fg = strip_foreground(w, &rgba);

    // The determinate bar fills ~3/7 of a full-width (≈1850px) track at 8px tall
    // → a few thousand ACCENT_2 px. A robust floor well above stray AA.
    assert!(
        fill > 1500,
        "the determinate progress bar's ACCENT_2 fill must paint beneath the \
         input panel (expected >1500 fill px in the STRIP band, got {fill}). If \
         this fails the bar did not render with a real fraction. \
         PNG: /tmp/bakeoff_progress_render.png"
    );
    // The "Running v0.5.bbands — 4 of 7" label is a line of text above the bar.
    assert!(
        fg > 300,
        "the progress label ('Running … — 4 of 7') must paint in the STRIP band \
         (expected >300 foreground px, got {fg}). \
         PNG: /tmp/bakeoff_progress_render.png"
    );
}

/// **Negative control / discriminator.** The SAME screen NOT running (the cold
/// Empty state) paints NO progress bar beneath the input panel — so ~no
/// `ACCENT_2` fill + far less foreground IN THE STRIP band. Proves the populated
/// guard genuinely discriminates (the strip is gated on `running`, not chrome).
#[test]
fn not_running_paints_no_progress_strip() {
    use ui::state::PanelState;

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/bakeoff_progress_empty_render.png");
    }

    let fill = strip_fill_pixels(w, &rgba);
    assert!(
        fill < 150,
        "the not-running state must NOT paint a progress-bar ACCENT_2 fill in the \
         STRIP band (expected <150 stray px, got {fill}). If high, the populated \
         guard is a tautology. PNG: /tmp/bakeoff_progress_empty_render.png"
    );
}

/// **Anti-tautology discriminator.** The running-with-progress frame paints
/// strictly MORE `ACCENT_2` strip fill than the not-running frame. Ties the two
/// states together so a regression that makes both look the same fails.
#[test]
fn running_strictly_exceeds_not_running_strip_fill() {
    use ui::state::PanelState;

    let running = ui::fixtures::fake_cockpit_leaderboard_running_progress(3, 7, "v0.5.bbands");
    let idle = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);

    let (wr, _hr, rr) = render_leaderboard_rgba(running);
    let (wi, _hi, ri) = render_leaderboard_rgba(idle);

    let fill_r = strip_fill_pixels(wr, &rr);
    let fill_i = strip_fill_pixels(wi, &ri);
    assert!(
        fill_r > fill_i + 1000,
        "running-with-progress must paint strictly more strip ACCENT_2 fill than \
         idle (running {fill_r} vs idle {fill_i})"
    );
}
