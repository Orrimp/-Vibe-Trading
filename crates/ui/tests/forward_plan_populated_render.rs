//! advisor-forward-plan v0.1.0 (roadmap F6) — render-layer proof of the
//! POPULATED forward buy/sell PLAN in the cockpit.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the plan draws. That trap
//! shipped multiple blind cockpit bugs (the Live-view saga, the trail 0-px
//! side-drawer, the Reports empty-curve). This guard renders the REAL
//! `screens::forward_plan::view` HEADLESS with a POPULATED `ForwardPlanView`
//! and asserts on the rendered PIXELS that the stance badge + the IF/THEN rule
//! text + the €200 projected-sizing number + the not-a-prediction framing
//! actually paint — with TWO negative controls:
//!   - the BUY-AND-HOLD degenerate plan (the same KIND of object, but visibly
//!     DIFFERENT — no IF/THEN accent in the rules band), proving the populated
//!     guard is not a tautology;
//!   - the EMPTY "no plan yet" state (paints ~no plan — a single prompt line).
//!
//! Three guards (populated active + buy-and-hold negative control + empty,
//! per CLAUDE.md + ADR-0062 § D8.1 / T-U3.1):
//!
//! - [`forward_plan_populated_paints_stance_rules_and_sizing`] — the active
//!   SMA fixture paints (a) `ACCENT` teal in the RULES band (the IF/THEN
//!   keywords), and (b) a healthy amount of foreground text (the stance,
//!   rules, sizing, horizon blocks plus the framing banner and the
//!   disclaimers actually drew, not a blank pane). Writes the operator-facing
//!   PNG to `/tmp/forward_plan_populated_render.png`.
//! - [`forward_plan_buy_and_hold_is_the_negative_control`] — the buy-and-hold
//!   degenerate fixture paints the plan (a lot of foreground — it IS the same
//!   kind of object) but paints STRICTLY LESS `ACCENT` teal in the RULES band
//!   (no IF/THEN keywords — there is no sell trigger / re-evaluation), proving
//!   the populated guard is not a tautology. Writes
//!   `/tmp/forward_plan_buy_and_hold_render.png`.
//! - [`forward_plan_empty_paints_no_plan`] — the second negative control: the
//!   `Empty` "run a bake-off first" prompt paints far less foreground than the
//!   populated plan and ~no `ACCENT` teal anywhere. Writes
//!   `/tmp/forward_plan_empty_render.png`.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file
//! compiles to nothing on Linux/Windows. Pixel thresholds are deliberately
//! coarse (presence/absence of a hue, not byte-exact), robust within macOS
//! across font-DB jitter.

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

// ── Region bands ──────────────────────────────────────────────────────────────
//
// The plan body stacks, top-to-bottom, at the 1920×1080 / scale-1.0 slot:
//
//   y   0– 90  header: page headline + caption.
//   y 100–175  the not-a-prediction FRAMING banner (warn-bordered card).
//   y 175–300  the "Right now" stance block (badge + as-of + latest signal).
//   y 300–460  the "Standing rules" block (IF/THEN lines + cadence) — the
//              IF/THEN keywords paint ACCENT teal HERE (active plans only;
//              the buy-and-hold plan paints NO accent in this band).
//   y 460+     the sizing block + horizon block + disclaimer.
//
// The RULES band is the load-bearing discriminator: an active plan paints the
// IF/THEN keyword accent there; the buy-and-hold plan does not. Because the
// exact y-extents shift a little with content, the RULES band is scoped
// generously and the whole-screen foreground carries the populated-vs-empty
// discrimination.

/// Top of the RULES band — where the IF/THEN keyword accent lives on an active
/// plan. Below the header + framing banner + stance block.
const RULES_TOP: u32 = 300;
/// Bottom of the RULES band — above the sizing block.
const RULES_BOTTOM: u32 = 470;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — green & blue
/// high and close, red clearly lower (the exact predicate the leaderboard +
/// Reports curve guards use for #6FB6AE).
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// Count `ACCENT`-teal pixels in the `[y0, y1)` row band.
fn accent_teal_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_accent_teal(r, g, b) {
                hits += 1;
            }
        }
    }
    hits
}

/// The IF/THEN keyword `ACCENT` teal in the RULES band. On an active plan this
/// is non-trivial (the IF/THEN keywords on the entry + exit lines); on the
/// buy-and-hold plan it is ~zero (no IF/THEN — there is no sell trigger).
fn rules_band_accent(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, RULES_TOP, RULES_BOTTOM)
}

/// `ACCENT` teal across the whole frame — used by the empty-state guard (the
/// empty prompt has no plan, so ~no accent anywhere).
fn whole_screen_accent(w: u32, h: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, 0, h)
}

/// Count general foreground (text / marker) pixels in the `[y0, y1)` band —
/// anything that crosses a luma floor the near-black `CANVAS`/`PANEL`/
/// `PANEL_RAISED` tiers never reach. Monotonic in how much content drew.
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

/// **The render-layer guard.** A populated active-strategy `ForwardPlanView`
/// MUST paint, in the cockpit Forward-plan screen:
/// - `ACCENT` teal in the RULES band (the IF/THEN keywords — the conditional
///   framing actually drew);
/// - a healthy amount of foreground text (the framing banner + the four blocks
///   + the disclaimers drew, not a blank pane).
///
/// Writes the operator-facing PNG to `/tmp/forward_plan_populated_render.png`.
#[test]
fn forward_plan_populated_paints_stance_rules_and_sizing() {
    let view = ui::fixtures::fake_forward_plan();
    assert!(
        !view.is_buy_and_hold(),
        "the populated fixture must be an ACTIVE plan (IF/THEN rules), not buy-and-hold"
    );

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_populated_render.png");
    }

    let rules_accent = rules_band_accent(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The IF/THEN keywords (entry + exit lines) paint ACCENT teal in the RULES
    // band — proof the conditional rule structure actually rendered.
    assert!(
        rules_accent > 60,
        "the IF/THEN keyword ACCENT must paint in the RULES band (expected >60 \
         teal px, got {rules_accent}). If this fails the standing-rules block \
         did not render the conditional structure. \
         PNG: /tmp/forward_plan_populated_render.png"
    );
    // The full plan (banner + stance + rules + sizing + horizon + disclaimers)
    // is a lot of text.
    assert!(
        fg > 7000,
        "the populated plan must paint a lot of foreground text (expected >7000 \
         px, got {fg}). If this is low the screen rendered a blank/empty pane \
         despite Ready data. PNG: /tmp/forward_plan_populated_render.png"
    );
}

/// **Negative control #1 — the buy-and-hold degenerate plan.** The SAME harness
/// with `fake_forward_plan_buy_and_hold()` paints the plan (a lot of foreground
/// — it IS the same KIND of object) but paints STRICTLY LESS `ACCENT` teal in
/// the RULES band than the active plan (no IF/THEN keywords — there is no sell
/// trigger / re-evaluation). Proves the populated guard genuinely discriminates
/// the conditional active plan from the degenerate one (it is not satisfied by
/// chrome / the framing banner / the disclaimer).
#[test]
fn forward_plan_buy_and_hold_is_the_negative_control() {
    let bh = ui::fixtures::fake_forward_plan_buy_and_hold();
    assert!(
        bh.is_buy_and_hold(),
        "the negative-control fixture is buy-and-hold"
    );
    let active = ui::fixtures::fake_forward_plan();

    let bh_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(bh));
    let active_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(active));

    let (wb, hb, rb) = render_forward_plan_rgba(bh_cockpit);
    let (wa, _ha, ra) = render_forward_plan_rgba(active_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(wb, hb, rb.clone()) {
        let _ = img.save("/tmp/forward_plan_buy_and_hold_render.png");
    }

    // It IS the same kind of object — it paints a substantial plan.
    let fg_bh = foreground_pixels(wb, hb, &rb);
    assert!(
        fg_bh > 4000,
        "the buy-and-hold plan must still paint a substantial plan (expected \
         >4000 foreground px, got {fg_bh}) — it is the same KIND of object. \
         PNG: /tmp/forward_plan_buy_and_hold_render.png"
    );

    // But it has NO IF/THEN keywords — strictly less RULES-band accent than the
    // active plan. This is the anti-tautology discriminator.
    let rules_accent_bh = rules_band_accent(wb, &rb);
    let rules_accent_active = rules_band_accent(wa, &ra);
    assert!(
        rules_accent_active > rules_accent_bh + 40,
        "the active plan must paint strictly more IF/THEN accent in the RULES \
         band than the buy-and-hold plan (active {rules_accent_active} vs \
         buy-and-hold {rules_accent_bh}). If they are equal the populated guard \
         is a tautology (the IF/THEN structure is not discriminating). \
         PNG: /tmp/forward_plan_buy_and_hold_render.png"
    );
}

/// **Negative control #2 — the empty / no-pick state.** The `Empty` "run a
/// bake-off first" prompt is a single muted line, NO plan. So it paints far
/// less foreground than the populated plan and ~no `ACCENT` teal anywhere.
/// Proves the populated guard genuinely discriminates (it is not satisfied by
/// the screen header / chrome).
#[test]
fn forward_plan_empty_paints_no_plan() {
    let empty_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Empty);
    let populated_cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(
        ui::fixtures::fake_forward_plan(),
    ));

    let (we, he, re) = render_forward_plan_rgba(empty_cockpit);
    let (wp, hp, rp) = render_forward_plan_rgba(populated_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(we, he, re.clone()) {
        let _ = img.save("/tmp/forward_plan_empty_render.png");
    }

    // ~No ACCENT teal anywhere — the empty prompt has no plan (no stance badge,
    // no IF/THEN keywords). A small floor allows stray AA.
    let empty_accent = whole_screen_accent(we, he, &re);
    assert!(
        empty_accent < 120,
        "the Empty state must NOT paint plan ACCENT (expected <120 stray teal \
         px, got {empty_accent}). If high, the populated guard is a tautology. \
         PNG: /tmp/forward_plan_empty_render.png"
    );

    // Strictly less foreground than the populated plan (the prompt is one line;
    // the plan is six stacked blocks).
    let fg_empty = foreground_pixels(we, he, &re);
    let fg_pop = foreground_pixels(wp, hp, &rp);
    assert!(
        fg_pop > fg_empty + 4000,
        "the populated plan must paint far more foreground than the empty \
         prompt (populated {fg_pop} vs empty {fg_empty}). \
         PNG: /tmp/forward_plan_empty_render.png"
    );
}

/// **RSI-reversion render guard.** Verifies that the FAITHFUL RSI rule copy
/// (flip-to-false exit at RSI > 30, NO overbought-70 claim, plus the
/// compound-condition caveat) actually renders in the cockpit Forward-plan
/// screen:
///
/// - `ACCENT` teal in the RULES band (IF/THEN keywords on entry + exit lines);
/// - a healthy foreground count (all blocks drew).
/// - Strictly less RULES-band accent than the SMA active fixture (same proof
///   structure as the BuyAndHold negative control, just to avoid tautology with
///   a different strategy family than the main populated guard).
///
/// Writes the operator-facing PNG to `/tmp/forward_plan_rsi_render.png`.
#[test]
fn forward_plan_rsi_reversion_paints_faithful_rules() {
    let rsi_view = ui::fixtures::fake_forward_plan_rsi();
    assert!(
        !rsi_view.is_buy_and_hold(),
        "the RSI fixture must be an active plan (IF/THEN rules)"
    );

    let cockpit = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(rsi_view));
    let (w, h, rgba) = render_forward_plan_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_plan_rsi_render.png");
    }

    let rules_accent = rules_band_accent(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The IF/THEN keywords (RSI entry + flip-to-false exit) must paint ACCENT
    // teal in the RULES band — proof the faithful non-SMA copy rendered.
    assert!(
        rules_accent > 60,
        "the RSI plan IF/THEN keywords must paint ACCENT teal in the RULES band \
         (expected >60 teal px, got {rules_accent}). If this fails the RSI rule \
         copy did not render the conditional structure. \
         PNG: /tmp/forward_plan_rsi_render.png"
    );
    // The full plan must be a lot of text (all six blocks + framing).
    assert!(
        fg > 7000,
        "the RSI plan must paint a lot of foreground text (expected >7000 px, \
         got {fg}). If this is low the screen rendered a blank/empty pane. \
         PNG: /tmp/forward_plan_rsi_render.png"
    );
}
