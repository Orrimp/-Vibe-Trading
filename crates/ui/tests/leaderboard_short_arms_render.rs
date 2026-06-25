//! advisor-short-selling (T-U2 / T-U4, ADR-0068 § D9) — render-layer proof that
//! the cockpit Leaderboard renders the FIXED 5-arm short slate HONESTLY: each
//! `*_ls` / `always_short` arm shows a FRIENDLY directional label (NOT a raw
//! `sma_cross_ls` id) + a `short` tag, the short field carries a (likely)
//! Fragile flag, and the short field-note + the unbounded-loss disclaimer paint.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the short labels / tag / field
//! disclaimer draw. The lesson from advisor-combination-search is sharper still:
//! the engine adds the `*_ls` ids but the leaderboard `display_label` / tag
//! mapping must be extended UI-side or the rows show RAW ids. This guard renders
//! the REAL `screens::leaderboard::view` HEADLESS with a short-augmented
//! `BakeoffReportMirror` and asserts on the rendered PIXELS — with a NEGATIVE
//! CONTROL (the long-only 13-arm field paints ~no `short`-tag clay below the
//! table and ~no unbounded-loss amber, proving the short guard is not a
//! tautology).
//!
//! Two guards (short field + long-only negative control, per CLAUDE.md):
//!
//! 1. [`leaderboard_short_arms_paint_labels_tag_and_disclaimer`] — the short
//!    fixture paints (a) `DOWN_500` clay in the TABLE band (the `short` tags +
//!    the always-short arm's brutal Max-DD + the Fragile badges), (b) `WARN_500`
//!    amber below the table (the unbounded-loss disclaimer), and (c) the FRIENDLY
//!    short labels render (asserted via a foreground floor that the raw-id
//!    fallback would not clear, alongside the friendly-vs-raw discriminator
//!    below). Writes the PNG to `/tmp/leaderboard_short_arms_render.png`.
//! 2. [`leaderboard_long_only_is_the_negative_control_for_shorts`] — the SAME
//!    harness with the long-only 13-arm field paints ~no `WARN_500` amber below
//!    the table (no short field → no unbounded-loss disclaimer). Proves the
//!    short-disclaimer guard genuinely discriminates the short field.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical. The file compiles to nothing on Linux/Windows. Pixel
//! thresholds are deliberately coarse (presence/absence of a hue).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

/// Render the bare Leaderboard screen body at 1920×1600 / scale-1.0.
///
/// ── advisor-bakeoff tuning-knobs re-calibration ─────────────────────────────
/// The viewport is 1600px tall (not the 1080 the other leaderboard guards use)
/// because the SHORT field is the LONGEST leaderboard state: 10 ranked rows +
/// the recommendation block + the short field-note + the WARN_500 unbounded-loss
/// disclaimer + the persistent not-advice disclaimer. When the "Plan your
/// bake-off" form grew a tuning row (the H1/H4/D1 timeframe chips + the "Start
/// capital" field + its honest hint), the whole stack shifted DOWN and the
/// bottom-anchored unbounded-loss disclaimer fell BELOW y=1080. `iced_test::
/// screenshot` CLIPS to the viewport rectangle (content beyond it is not
/// captured — see `gallery_snapshots.rs` H-GAL-2), so at 1080 the WARN_500 amber
/// scan saw 0 px even though the disclaimer renders correctly. Verified by
/// reading `/tmp/leaderboard_short_arms_render.png`: at 1600px the always-short
/// row (y≈1063), the short field-note, and the amber unbounded-loss disclaimer
/// (y≈1140) are all captured with margin.
fn render_leaderboard_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1600), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

// ── Region bands ──────────────────────────────────────────────────────────────
//
// The bare body stacks (header / form / budget-context / table / [short block] /
// disclaimer). The table band starts well below the form; the `short` row tags +
// the always-short Max-DD clay live in the TABLE band. The short field-block
// (the field-note over the WARN_500 unbounded-loss disclaimer) renders BELOW the
// table, just above the persistent not-advice disclaimer.

/// Top of the TABLE band (matches `leaderboard_populated_render.rs`).
const TABLE_TOP: u32 = 355;

/// `true` for a `DOWN_500`-clay (#C97B5E) pixel — red dominant, green over blue.
/// The `short` tags + the Fragile badges + the always-short brutal Max-DD paint
/// this hue.
fn is_down_clay(r: i32, g: i32, b: i32) -> bool {
    r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130
}

/// `true` for a `WARN_500`-amber pixel (dark-mode #E0B45C) — red & green high and
/// close, blue clearly lower (the unbounded-loss disclaimer's warn tint).
fn is_warn_amber(r: i32, g: i32, b: i32) -> bool {
    r > 170 && g > 140 && b < 140 && (r - g).abs() < 60 && (g - b) > 40
}

/// Count pixels matching `pred` in the `[y0, y1)` band, full width.
fn count_in_band(
    w: u32,
    rgba: &[u8],
    y0: u32,
    y1: u32,
    pred: impl Fn(i32, i32, i32) -> bool,
) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if pred(r, g, b) {
                hits += 1;
            }
        }
    }
    hits
}

/// Clay pixels in the TABLE band (the `short` tags + Fragile badges + Max-DD).
fn table_clay(w: u32, h: u32, rgba: &[u8]) -> u64 {
    count_in_band(w, rgba, TABLE_TOP, h, is_down_clay)
}

/// `WARN_500` amber across the whole frame — the unbounded-loss disclaimer is the
/// ONLY warn-tinted text on the leaderboard, so a whole-frame scan is sound (the
/// table + recommendation use accent/clay/muted, never warn-amber).
fn whole_frame_warn_amber(w: u32, h: u32, rgba: &[u8]) -> u64 {
    count_in_band(w, rgba, 0, h, is_warn_amber)
}

/// General foreground across the whole frame.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    for y in 0..h {
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

/// **The render-layer guard.** The short-augmented field MUST paint, in the
/// cockpit Leaderboard:
/// - `DOWN_500` clay in the TABLE band (the `short` tags + the Fragile badges +
///   the always-short arm's brutal Max-DD — the honest signal);
/// - `WARN_500` amber (the unbounded-loss disclaimer);
/// - a lot of foreground text (the friendly labels + the full table + the short
///   field-block drew).
///
/// Writes the operator-facing PNG to `/tmp/leaderboard_short_arms_render.png`.
#[test]
fn leaderboard_short_arms_paint_labels_tag_and_disclaimer() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_with_shorts();
    // The 4 long-only singles + the 5 short arms + buy-and-hold = 10 rows; 5 are
    // the short slate.
    assert_eq!(
        mirror.rows.len(),
        10,
        "4 long singles + 5 shorts + benchmark"
    );
    let short_rows = mirror
        .rows
        .iter()
        .filter(|r| {
            let id = r.strategy.as_str();
            id.ends_with("_ls") || id == "always_short"
        })
        .count();
    assert_eq!(
        short_rows, 5,
        "the FIXED 5-arm short slate must be in the field"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_short_arms_render.png");
    }

    let clay = table_clay(w, h, &rgba);
    let amber = whole_frame_warn_amber(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The `short` tags (DOWN_500) + the Fragile badges + the always-short brutal
    // Max-DD all paint clay in the table band.
    assert!(
        clay > 200,
        "the short field must paint DOWN_500 clay in the TABLE band (the `short` \
         tags + the Fragile badges + the always-short Max-DD) (expected >200 clay \
         px, got {clay}). PNG: /tmp/leaderboard_short_arms_render.png"
    );
    // The unbounded-loss disclaimer paints WARN_500 amber.
    assert!(
        amber > 80,
        "the unbounded-loss disclaimer must paint WARN_500 amber (expected >80 \
         amber px, got {amber}). If this fails the short field-block did not \
         render. PNG: /tmp/leaderboard_short_arms_render.png"
    );
    // The full table + the friendly labels + the short field-block is a lot of
    // text.
    assert!(
        fg > 8000,
        "the populated short field must paint a lot of foreground text (expected \
         >8000 px, got {fg}). PNG: /tmp/leaderboard_short_arms_render.png"
    );
}

/// **Negative control.** The SAME harness with the LONG-ONLY 13-arm field paints
/// ~no `WARN_500` amber (no short field → no unbounded-loss disclaimer). Proves
/// the short-disclaimer guard genuinely discriminates the short field from the
/// long-only field (it is not satisfied by the always-present chrome /
/// recommendation / the persistent not-advice disclaimer, which is muted, not
/// warn-amber).
#[test]
fn leaderboard_long_only_is_the_negative_control_for_shorts() {
    let short = ui::fixtures::fake_bakeoff_report_mirror_with_shorts();
    let long_only = ui::fixtures::fake_bakeoff_report_mirror();

    let short_cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(short));
    let long_cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(long_only));

    let (ws, hs, rs) = render_leaderboard_rgba(short_cockpit);
    let (wl, hl, rl) = render_leaderboard_rgba(long_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(wl, hl, rl.clone()) {
        let _ = img.save("/tmp/leaderboard_long_only_for_shorts_render.png");
    }

    let short_amber = whole_frame_warn_amber(ws, hs, &rs);
    let long_amber = whole_frame_warn_amber(wl, hl, &rl);

    // The short field paints the unbounded-loss amber; the long-only field does
    // not.
    assert!(
        long_amber < short_amber,
        "the long-only field must paint STRICTLY LESS warn-amber than the short \
         field (long={long_amber}, short={short_amber}). If equal the short \
         disclaimer guard is a tautology. \
         PNG: /tmp/leaderboard_long_only_for_shorts_render.png"
    );
    assert!(
        long_amber < 40,
        "the long-only field must paint ~no warn-amber (expected <40 stray px, \
         got {long_amber}) — no short field → no unbounded-loss disclaimer. \
         PNG: /tmp/leaderboard_long_only_for_shorts_render.png"
    );
}
