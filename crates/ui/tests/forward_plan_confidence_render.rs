//! P0-3 "Confidence check, not verdict" — render-layer proof of the CONFIDENCE
//! SUMMARY BLOCK in the cockpit Forward-plan screen (advisor-confidence-not-verdict,
//! ADR-0076).
//!
//! ## Why this file exists
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state,
//! a text `.snap`, or a no-panic boot is NOT proof the confidence block draws.
//! This guard renders the REAL `screens::forward_plan::view` HEADLESS with a
//! POPULATED `ForwardPlanView` carrying a `ConfidenceSummaryView` and asserts on
//! the rendered PIXELS that the four confidence facts + the section header actually
//! paint — with ONE negative control:
//!   - the SAME plan WITHOUT a confidence summary (the block must be ABSENT, i.e.
//!     the plan with confidence paints STRICTLY MORE foreground than without).
//!
//! Two guards:
//! - [`confidence_block_paints_more_foreground_than_without`] — the positive
//!   case: `fake_forward_plan_with_confidence()` paints more total foreground
//!   than `fake_forward_plan()` (same plan family, no confidence). This is the
//!   load-bearing discriminator: the confidence block is EXTRA content not
//!   present on the control.
//! - [`confidence_block_below_horizon_band`] — the block paints foreground in
//!   the LOWER portion of the frame (below the horizon block), meaning the four
//!   fact rows rendered and are not squashed into nothing. The same plan without
//!   confidence paints LESS in that lower band.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `forward_plan_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse (presence
//! vs absence of content, not byte-exact), robust within macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::forward_plan_screen_program;

/// Render the bare Forward-plan screen body at the `typical` 1920×1080 slot and
/// return the physical-pixel RGBA buffer + dimensions.
fn render_forward_plan_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = forward_plan_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

/// Count general foreground (text / marker) pixels in the `[y0, y1)` row band —
/// anything that crosses a luma floor the near-black `CANVAS`/`PANEL`/`PANEL_RAISED`
/// tiers never reach. Monotonic in how much content drew.
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

/// General foreground across the whole frame.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, 0, h)
}

// The confidence block appears AFTER the horizon block, which ends around y ≈ 600
// at the 1920×1080 / scale-1.0 slot. The lower band (y 600–900) is exclusively
// the confidence-summary block when present, and just the disclaimer footer when
// absent. The block adds four fact rows + a section header — meaningfully more
// foreground than just the footer.
//
// (Exact extents: header + framing ≈ 0–175; stance ≈ 175–300; rules ≈ 300–480;
// sizing ≈ 480–560; horizon ≈ 560–640; confidence ≈ 640–850; disclaimer ≈ 850+)
const CONFIDENCE_BAND_TOP: u32 = 600;
const CONFIDENCE_BAND_BOTTOM: u32 = 900;

/// Foreground in the confidence-block band — the four fact rows + section header.
/// High when the confidence block is present; low when absent (only the disclaimer).
fn confidence_band_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, CONFIDENCE_BAND_TOP, CONFIDENCE_BAND_BOTTOM)
}

/// **Positive case: confidence block renders more foreground than without.**
///
/// A populated `ForwardPlanView` WITH a `ConfidenceSummaryView` (n_candidates=18,
/// deflated_sharpe=0.87, crown_clears_dsr=false, min_btl_years=6.4) MUST paint
/// STRICTLY MORE total foreground than the SAME plan WITHOUT a confidence summary.
/// The delta is the four fact rows + section header of the confidence block.
///
/// Writes operator-facing PNGs to:
/// - `/tmp/forward_plan_confidence_render.png` (with confidence)
/// - `/tmp/forward_plan_no_confidence_render.png` (without — negative control)
#[test]
fn confidence_block_paints_more_foreground_than_without() {
    let with_confidence = ui::fixtures::fake_forward_plan_with_confidence();
    assert!(
        with_confidence.confidence.is_some(),
        "the fixture WITH confidence must carry a populated ConfidenceSummaryView"
    );

    let without_confidence = ui::fixtures::fake_forward_plan();
    assert!(
        without_confidence.confidence.is_none(),
        "the negative-control fixture must NOT carry a confidence summary"
    );

    let cockpit_with = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(with_confidence));
    let cockpit_without =
        ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(without_confidence));

    let (w_with, h_with, rgba_with) = render_forward_plan_rgba(cockpit_with);
    let (w_wo, h_wo, rgba_wo) = render_forward_plan_rgba(cockpit_without);

    // Operator-facing deliverables (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w_with, h_with, rgba_with.clone()) {
        let _ = img.save("/tmp/forward_plan_confidence_render.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(w_wo, h_wo, rgba_wo.clone()) {
        let _ = img.save("/tmp/forward_plan_no_confidence_render.png");
    }

    let fg_with = foreground_pixels(w_with, h_with, &rgba_with);
    let fg_without = foreground_pixels(w_wo, h_wo, &rgba_wo);

    // The plan with confidence must draw MORE total foreground (the block is extra
    // content — a section header + four fact rows that the control does not have).
    assert!(
        fg_with > fg_without + 500,
        "the plan WITH confidence must paint strictly more foreground than the \
         same plan WITHOUT (with {fg_with} vs without {fg_without}, delta must be \
         >500 px). If delta is low the confidence block did not render. \
         PNGs: /tmp/forward_plan_{{confidence,no_confidence}}_render.png"
    );
}

/// **Negative control: the confidence block is in the LOWER band.**
///
/// When the plan carries a confidence summary, the lower band (y 600–900, below
/// the horizon block) must show STRICTLY MORE foreground than the same plan without
/// confidence. The control plan (no confidence) has only the disclaimer footer in
/// that band; the test plan has the full confidence block (header + four facts) above
/// the footer. This is the SPATIAL discriminator: the block is where we expect.
///
/// Writes operator-facing PNGs to the same paths as the first guard (idempotent).
#[test]
fn confidence_block_below_horizon_band() {
    let with_confidence = ui::fixtures::fake_forward_plan_with_confidence();
    let without_confidence = ui::fixtures::fake_forward_plan();

    let cockpit_with = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(with_confidence));
    let cockpit_without =
        ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(without_confidence));

    let (w_with, _h_with, rgba_with) = render_forward_plan_rgba(cockpit_with);
    let (w_wo, _h_wo, rgba_wo) = render_forward_plan_rgba(cockpit_without);

    let conf_fg_with = confidence_band_foreground(w_with, &rgba_with);
    let conf_fg_wo = confidence_band_foreground(w_wo, &rgba_wo);

    // The confidence block (section header + four fact rows) paints far more
    // foreground in the lower band than the disclaimer-only control.
    assert!(
        conf_fg_with > conf_fg_wo + 300,
        "the plan WITH confidence must paint more foreground in the lower band \
         (y {CONFIDENCE_BAND_TOP}–{CONFIDENCE_BAND_BOTTOM}) than the plan without \
         (with {conf_fg_with} vs without {conf_fg_wo}, delta must be >300 px). \
         If delta is low the confidence block is missing from that region. \
         PNGs: /tmp/forward_plan_{{confidence,no_confidence}}_render.png"
    );
}
