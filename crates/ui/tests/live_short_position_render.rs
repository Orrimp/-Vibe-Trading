//! advisor-short-selling (T-U1 / T-U4, ADR-0068 § D8) — render-layer proof that
//! the cockpit Live view renders an open SHORT position HONESTLY: a SHORT badge,
//! a signed/negative `base_qty`, a NEGATIVE (not clamped) mark-to-market P&L
//! painted in `DOWN_500` clay, AND the load-bearing "a short can lose more than
//! your €200" unbounded-loss disclaimer.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the SHORT badge / negative P&L
//! draws. That trap shipped multiple blind cockpit bugs (the Live-view saga, the
//! trail 0-px side-drawer, the Reports empty-curve). This guard renders the REAL
//! Live screen (`shell::view` → `screens::live::view` → `widgets::positions`)
//! HEADLESS with a POPULATED short position and asserts on the rendered PIXELS
//! that the short surface paints — with a NEGATIVE CONTROL (a LONG-ONLY position
//! list paints ~no clay in the positions region, proving the populated guard is
//! not a tautology and that a long still renders correctly).
//!
//! Two guards (populated short + long-only negative control, per CLAUDE.md):
//!
//! 1. [`live_short_position_paints_badge_negative_pnl_and_disclaimer`] — the
//!    short fixture paints (a) `DOWN_500` clay in the POSITIONS band (the SHORT
//!    badge + the negative P&L + the negative P&L% — the down/clay treatment),
//!    (b) `WARN_500` amber in the FORWARD band (the unbounded-loss disclaimer),
//!    and (c) a healthy amount of foreground text. Writes the operator-facing
//!    PNG to `/tmp/live_short_position_render.png`.
//! 2. [`live_long_only_is_the_negative_control`] — the SAME harness with a
//!    LONG-ONLY position list paints STRICTLY LESS clay in the POSITIONS band
//!    (a long's P&L is positive → sage, the LONG badge is not clay) AND ~no
//!    `WARN_500` amber (no short → no unbounded-loss disclaimer). Proves the
//!    populated short guard genuinely discriminates a short from a long.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `live_forward_pnl_render.rs` / `leaderboard_populated_render.rs`, real-
//! renderer pixel assertions are macOS-canonical (cosmic-text rasterisation is
//! per-OS). The file compiles to nothing on Linux/Windows. Pixel thresholds are
//! deliberately coarse (presence/absence of a hue, not byte-exact).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::Cockpit;
use ui::test_support::program_from_cockpit;

const VIEW_W: u32 = 1280;
// Tall viewport so the WHOLE Live screen fits: the equity curve is greedy, so at
// 720px the positions panel (the SHORT badge) + the forward-P/L block's
// disclaimer are pushed off-screen. 1200px lets the full column render.
const VIEW_H: u32 = 1200;
const SCALE: f32 = 1.0;

/// Render the full cockpit shell (sidebar + Live screen body) and return the
/// physical-pixel RGBA buffer + dimensions.
fn render_live_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

// ── Region bands ──────────────────────────────────────────────────────────────
//
// The full shell renders a 180px sidebar on the left; every scan starts at
// x≥200 so the sidebar chrome never confounds the count. The Live screen stacks
// (header / health / equity / kpi / caption / forward-P/L block / bottom_row).
// The POSITIONS panel (where the SHORT badge + the signed qty + the P&L columns
// live) is in the bottom_row — the lower portion of the frame. The forward-P/L
// block (where the unbounded-loss disclaimer is carried) sits just above it.

/// Left edge of every scan — right of the 180px sidebar.
const CONTENT_X0: u32 = 200;

/// Right edge of the LEFT content half (the positions panel + the forward-P/L
/// value live here). The bottom_row's RIGHT half is the agent-activity tape,
/// whose SELL rows are ALWAYS `DOWN_500` clay (independent of long/short) — so
/// the clay scan is scoped to the left half to exclude the tape, keeping the
/// SHORT-badge / negative-P&L discriminator honest. The amber (disclaimer) scan
/// is full-width (the tape has no warn-amber).
const LEFT_CONTENT_X1: u32 = 660;

/// `true` for a `DOWN_500`-clay (#C97B5E — R201 G123 B94) pixel — red dominant
/// over green, green over blue, red high. The SHORT badge label, the negative
/// P&L, and the negative P&L% all paint this hue.
fn is_down_clay(r: i32, g: i32, b: i32) -> bool {
    r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130
}

/// `true` for a `WARN_500`-amber pixel (dark-mode #E0B45C — R224 G180 B92) — red
/// & green high and close, blue clearly lower (the warn-tint the unbounded-loss
/// disclaimer paints).
fn is_warn_amber(r: i32, g: i32, b: i32) -> bool {
    r > 170 && g > 140 && b < 140 && (r - g).abs() < 60 && (g - b) > 40
}

/// Count pixels matching `pred` in `x ∈ [x0, x1)`, full height. The greedy
/// equity curve shifts the positions panel + the forward block vertically, so a
/// whole-height scan (not a fragile y-band) keeps the discriminator robust; the
/// `x` bounds exclude the always-clay agent-activity tape where needed.
fn count_cols(w: u32, h: u32, rgba: &[u8], x0: u32, x1: u32, pred: impl Fn(i32, i32, i32) -> bool) -> u64 {
    let x_end = x1.min(w);
    let mut hits = 0u64;
    for y in 0..h {
        for x in x0..x_end {
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

/// Clay in the LEFT content half — the SHORT badge + the negative P&L (positions
/// panel) AND the negative forward P/L. Excludes the agent-activity tape (whose
/// SELL rows are always clay). On the long-only control the LONG badge is not
/// clay and the P&L is positive (sage), so this is ~0.
fn content_clay(w: u32, h: u32, rgba: &[u8]) -> u64 {
    count_cols(w, h, rgba, CONTENT_X0, LEFT_CONTENT_X1, is_down_clay)
}

/// Amber across the full content area — the unbounded-loss disclaimer. The Live
/// screen has NO framing banner and the tape has no warn-amber, so the
/// disclaimer is the only `WARN_500` source (the long-only control has none).
fn content_warn_amber(w: u32, h: u32, rgba: &[u8]) -> u64 {
    count_cols(w, h, rgba, CONTENT_X0, w, is_warn_amber)
}

/// General foreground (text) across the content area (right of the sidebar).
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    count_cols(w, h, rgba, CONTENT_X0, w, |r, g, b| (r * 2 + g * 3 + b) / 6 > 80)
}

/// **The render-layer guard.** A POPULATED short position MUST paint, in the
/// cockpit Live view:
/// - `DOWN_500` clay (the SHORT badge + the NEGATIVE, not-clamped P&L + P&L% in
///   the positions panel AND the negative forward P/L — the honest down/clay
///   treatment R-SS.4 mandates);
/// - `WARN_500` amber (the unbounded-loss disclaimer in the forward block);
/// - a healthy amount of foreground text (the screen drew, not a blank pane).
///
/// Writes the operator-facing PNG to `/tmp/live_short_position_render.png`.
#[test]
fn live_short_position_paints_badge_negative_pnl_and_disclaimer() {
    let cockpit = ui::fixtures::fake_cockpit_live_short();
    let (w, h, rgba) = render_live_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/live_short_position_render.png");
    }

    let clay = content_clay(w, h, &rgba);
    let amber = content_warn_amber(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The SHORT badge label + the negative P&L + the negative P&L% + the negative
    // forward P/L all paint DOWN_500 clay — proof the short rendered honestly
    // (the negative P&L is NOT clamped at 0).
    assert!(
        clay > 120,
        "the SHORT badge + the NEGATIVE P&L must paint DOWN_500 clay (expected \
         >120 clay px, got {clay}). If this fails the short position did not \
         render the honest down/clay treatment. \
         PNG: /tmp/live_short_position_render.png"
    );
    // The unbounded-loss disclaimer paints WARN_500 amber in the forward block.
    assert!(
        amber > 60,
        "the unbounded-loss disclaimer (\u{201c}a short can lose more than your \
         \u{20ac}200\u{201d}) must paint WARN_500 amber (expected >60 amber px, \
         got {amber}). If this fails the load-bearing disclaimer did not render. \
         PNG: /tmp/live_short_position_render.png"
    );
    // The Live screen is a lot of text.
    assert!(
        fg > 6000,
        "the populated Live view must paint a lot of foreground text (expected \
         >6000 px, got {fg}). If low the screen rendered a blank pane despite \
         Ready data. PNG: /tmp/live_short_position_render.png"
    );
}

/// **Negative control.** The SAME Live screen with a LONG-ONLY position list
/// paints STRICTLY LESS clay in the POSITIONS band (a long's P&L is positive →
/// sage `UP_500`, and the LONG badge is `ACCENT_SOFT`/`FG_2`, not clay) AND ~no
/// `WARN_500` amber (no short → no unbounded-loss disclaimer). Proves the
/// populated short guard genuinely discriminates a short from a long — the long
/// still renders, just without the short surface.
#[test]
fn live_long_only_is_the_negative_control() {
    let short = ui::fixtures::fake_cockpit_live_short();
    let long_only = ui::fixtures::fake_cockpit_live_long_only();

    let (ws, hs, rs) = render_live_rgba(short);
    let (wl, hl, rl) = render_live_rgba(long_only);

    if let Some(img) = image::RgbaImage::from_raw(wl, hl, rl.clone()) {
        let _ = img.save("/tmp/live_long_only_render.png");
    }

    let short_clay = content_clay(ws, hs, &rs);
    let long_clay = content_clay(wl, hl, &rl);
    let long_amber = content_warn_amber(wl, hl, &rl);

    // The long-only frame has no SHORT badge (the LONG badge is not clay) and a
    // positive (sage) P&L + forward P/L → far less clay than the short frame.
    assert!(
        long_clay < short_clay,
        "the long-only frame must paint STRICTLY LESS clay than the short frame \
         (long={long_clay}, short={short_clay}). If equal the short guard is a \
         tautology. PNG: /tmp/live_long_only_render.png"
    );
    assert!(
        long_clay < 60,
        "the long-only frame must paint ~no clay (expected <60 stray px, got \
         {long_clay}) — a long renders correctly WITHOUT the SHORT badge / \
         negative-P&L clay. PNG: /tmp/live_long_only_render.png"
    );
    // No short → no unbounded-loss disclaimer in the forward block.
    assert!(
        long_amber < 30,
        "the long-only forward run must NOT paint the unbounded-loss disclaimer \
         (expected <30 stray amber px, got {long_amber}). The short disclaimer is \
         short-specific. PNG: /tmp/live_long_only_render.png"
    );
}
