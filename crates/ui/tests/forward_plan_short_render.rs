//! advisor-short-selling (T-U3 / T-U4, ADR-0068 § D8) — render-layer proof that
//! the cockpit Forward-plan screen describes a SHORT-CAPABLE arm's down-half
//! rules HONESTLY: the sell-to-open / cover IF/THEN lines (for the `_ls`
//! variants) or the standing "open a short and hold it" rule (for
//! `always_short`), the maintenance-margin liquidation floor, and the
//! load-bearing unbounded-loss disclaimer.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the short-rule copy / disclaimer
//! draws. This guard renders the REAL `screens::forward_plan::view` HEADLESS with
//! a POPULATED short-capable plan and asserts on the rendered PIXELS — with a
//! NEGATIVE CONTROL (the long-only `fake_forward_plan` paints ~no `WARN_500`
//! amber, proving the short surface is not a tautology and a long-only plan does
//! not carry the short disclaimer).
//!
//! Three guards (`_ls` variant + `always_short` control + long-only negative
//! control, per CLAUDE.md):
//!
//! 1. [`forward_plan_short_ls_paints_short_rules_and_disclaimer`] — the
//!    `sma_cross_ls` fixture paints (a) `WARN_500` amber (the liquidation floor
//!    line + the unbounded-loss disclaimer), (b) `ACCENT` teal in the rules area
//!    (the IF/THEN keywords — the long entry/exit lines AND the short
//!    sell-to-open / cover lines), and (c) a healthy amount of foreground text.
//!    Writes the PNG to `/tmp/forward_plan_short_ls_render.png`.
//! 2. [`forward_plan_always_short_paints_standing_short_rule`] — the
//!    `always_short` fixture paints the standing short rule + the disclaimer
//!    (amber) — the down-side mirror of buy-and-hold. Writes
//!    `/tmp/forward_plan_always_short_render.png`.
//! 3. [`forward_plan_long_only_is_the_negative_control_for_shorts`] — the SAME
//!    harness with the long-only `fake_forward_plan` paints STRICTLY LESS
//!    `WARN_500` amber (no short → no liquidation line, no unbounded-loss
//!    disclaimer). Proves the short guards genuinely discriminate.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `forward_plan_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical. The file compiles to nothing on Linux/Windows. Pixel
//! thresholds are deliberately coarse (presence/absence of a hue).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::forward_plan_screen_program;

/// Render the bare Forward-plan screen body at 1920×1080 / scale-1.0.
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

/// `true` for an `ACCENT`-teal (#6FB6AE) pixel — green & blue high and close, red
/// clearly lower (the IF/THEN keyword hue, shared with the other plan guards).
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// `true` for a `WARN_500`-amber pixel (dark-mode #E0B45C) — red & green high and
/// close, blue clearly lower (the liquidation line + the unbounded-loss
/// disclaimer's warn tint).
fn is_warn_amber(r: i32, g: i32, b: i32) -> bool {
    r > 170 && g > 140 && b < 140 && (r - g).abs() < 60 && (g - b) > 40
}

// The not-a-prediction framing banner at the TOP of EVERY plan is WARN_500-
// bordered (`screens::forward_plan::framing_banner`), so a whole-frame amber
// scan would count the banner on BOTH the short and the long-only plans. The
// SHORT-SPECIFIC amber (the liquidation line in the rules block + the
// unbounded-loss disclaimer at the foot) is BELOW the banner — so the amber scan
// is scoped to `y > BANNER_BOTTOM` to isolate it from the always-present banner.
const BANNER_BOTTOM: u32 = 200;

/// Count pixels matching `pred` BELOW the framing banner (the short-specific
/// region). Excludes the always-present WARN_500 framing-banner border so the
/// scan isolates the short liquidation line + the unbounded-loss disclaimer.
fn count_below_banner(w: u32, h: u32, rgba: &[u8], pred: impl Fn(i32, i32, i32) -> bool) -> u64 {
    let mut hits = 0u64;
    for y in BANNER_BOTTOM..h {
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

/// Count pixels matching `pred` across the whole frame (used for teal + the
/// foreground floor, which are not confounded by the banner band).
fn count_whole_frame(w: u32, h: u32, rgba: &[u8], pred: impl Fn(i32, i32, i32) -> bool) -> u64 {
    let mut hits = 0u64;
    for y in 0..h {
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

/// General foreground across the whole frame.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    count_whole_frame(w, h, rgba, |r, g, b| (r * 2 + g * 3 + b) / 6 > 80)
}

/// **The render-layer guard (the `_ls` symmetric variant).** A populated
/// short-capable plan MUST paint, in the cockpit Forward-plan screen:
/// - `WARN_500` amber (the liquidation floor line + the unbounded-loss
///   disclaimer — the honest unbounded-loss framing);
/// - `ACCENT` teal (the IF/THEN keywords on the long entry/exit lines AND the
///   short sell-to-open / cover lines — the conditional structure drew);
/// - a healthy amount of foreground text (the whole plan + the short rules drew).
///
/// Writes the operator-facing PNG to `/tmp/forward_plan_short_ls_render.png`.
#[test]
fn forward_plan_short_ls_paints_short_rules_and_disclaimer() {
    let view = ui::fixtures::fake_forward_plan_short();
    assert!(
        view.is_short_capable(),
        "the fixture must be a short-capable arm"
    );
    assert!(
        !view.is_always_short(),
        "the `_ls` fixture is not always_short"
    );

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_short_ls_render.png");
    }

    // Amber BELOW the framing banner = the short-specific liquidation line + the
    // unbounded-loss disclaimer (the always-present WARN_500 banner border is
    // excluded by the band scope).
    let amber = count_below_banner(w, h, &rgba, is_warn_amber);
    let teal = count_whole_frame(w, h, &rgba, is_accent_teal);
    let fg = foreground_pixels(w, h, &rgba);

    // The liquidation floor line + the unbounded-loss disclaimer paint WARN_500
    // amber — the honest unbounded-loss framing.
    assert!(
        amber > 120,
        "the short liquidation line + the unbounded-loss disclaimer must paint \
         WARN_500 amber below the framing banner (expected >120 amber px, got \
         {amber}). If this fails the honest unbounded-loss framing did not \
         render. PNG: /tmp/forward_plan_short_ls_render.png"
    );
    // The IF/THEN keywords (long entry/exit + short sell-to-open/cover) paint
    // ACCENT teal.
    assert!(
        teal > 80,
        "the IF/THEN keywords (incl. the short sell-to-open / cover lines) must \
         paint ACCENT teal (expected >80 teal px, got {teal}). \
         PNG: /tmp/forward_plan_short_ls_render.png"
    );
    assert!(
        fg > 7000,
        "the populated short plan must paint a lot of foreground text (expected \
         >7000 px, got {fg}). PNG: /tmp/forward_plan_short_ls_render.png"
    );
}

/// **The always-short control guard.** The `always_short` fixture paints the
/// standing short rule + the liquidation line + the unbounded-loss disclaimer
/// (amber) — the down-side mirror of buy-and-hold. It renders as obviously the
/// same KIND of object as a plan, with the short surface present.
#[test]
fn forward_plan_always_short_paints_standing_short_rule() {
    let view = ui::fixtures::fake_forward_plan_always_short();
    assert!(
        view.is_always_short(),
        "the fixture must be the always_short control"
    );
    assert!(view.is_short_capable(), "always_short is short-capable");

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_always_short_render.png");
    }

    let amber = count_below_banner(w, h, &rgba, is_warn_amber);
    let fg = foreground_pixels(w, h, &rgba);

    assert!(
        amber > 120,
        "the always-short liquidation line + the unbounded-loss disclaimer must \
         paint WARN_500 amber below the framing banner (expected >120 amber px, \
         got {amber}). PNG: /tmp/forward_plan_always_short_render.png"
    );
    assert!(
        fg > 6000,
        "the always-short plan must paint a substantial plan (expected >6000 \
         foreground px, got {fg}) — it is the same KIND of object. \
         PNG: /tmp/forward_plan_always_short_render.png"
    );
}

/// **Negative control.** The SAME harness with the long-only `fake_forward_plan`
/// paints STRICTLY LESS `WARN_500` amber than the short plan (no short → no
/// liquidation line, no unbounded-loss disclaimer; the long fixture is not
/// `sizing_capped`, so it paints no warn at all). Proves the short guards
/// genuinely discriminate the short surface from a long-only plan.
#[test]
fn forward_plan_long_only_is_the_negative_control_for_shorts() {
    let short = ui::fixtures::fake_forward_plan_short();
    let long_only = ui::fixtures::fake_forward_plan();
    assert!(
        !long_only.is_short_capable(),
        "the control is a long-only plan"
    );

    let short_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(short));
    let long_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(long_only));

    let (ws, hs, rs) = render_forward_plan_rgba(short_cockpit);
    let (wl, hl, rl) = render_forward_plan_rgba(long_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(wl, hl, rl.clone()) {
        let _ = img.save("/tmp/forward_plan_long_only_for_shorts_render.png");
    }

    // Amber BELOW the framing banner: the short plan paints the liquidation line
    // + the unbounded-loss disclaimer there; the long-only plan paints NONE (its
    // only WARN_500 is the framing banner, which is ABOVE BANNER_BOTTOM and so
    // excluded). The always-present banner therefore cannot confound the control.
    let short_amber = count_below_banner(ws, hs, &rs, is_warn_amber);
    let long_amber = count_below_banner(wl, hl, &rl, is_warn_amber);

    assert!(
        long_amber < short_amber,
        "the long-only plan must paint STRICTLY LESS below-banner warn-amber than \
         the short plan (long={long_amber}, short={short_amber}). If equal the \
         short guard is a tautology. \
         PNG: /tmp/forward_plan_long_only_for_shorts_render.png"
    );
    assert!(
        long_amber < 40,
        "the long-only plan must paint ~no below-banner warn-amber (expected <40 \
         stray px, got {long_amber}) — no short → no liquidation line / \
         unbounded-loss disclaimer (the framing banner is excluded by the band). \
         PNG: /tmp/forward_plan_long_only_for_shorts_render.png"
    );
}
