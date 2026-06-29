//! advisor-turnover-and-tail-metrics (P1-2) — render-layer proof of the
//! "Risk story" tail/median honesty block in the cockpit leaderboard.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the Risk story block draws.
//! This guard renders the REAL `screens::leaderboard::view` HEADLESS with a
//! populated `BakeoffReportMirror` whose `tail` is `Some(..)` and asserts on the
//! rendered PIXELS that the tail/median block actually paints — WITH a NEGATIVE
//! CONTROL (the SAME mirror with `tail = None` paints strictly less foreground,
//! i.e. no block).
//!
//! The fixtures used here are deliberately SHORT-TABLE (a 2-row buy-and-hold
//! field) so the Risk story block — which renders directly UNDER the scorecard
//! block (the two honesty layers pair: trust + risk) — lands inside the
//! 1920×1080 screenshot viewport rather than scrolling off below a tall 13-arm
//! table. (The full-field render is exercised by `leaderboard_populated_render.rs`;
//! this file's job is the Risk story block.)
//!
//! Two guards (populated + negative control, per CLAUDE.md):
//!
//! 1. [`risk_story_block_paints_and_exceeds_no_tail`] — the populated fixture
//!    (tail present) paints STRICTLY MORE foreground than the SAME leaderboard
//!    with the tail removed. The six facts (Median / CVaR_95 / CVaR_99 / Skew /
//!    Sortino / Calmar) + their plain-language glosses + the panel chrome + the
//!    informational note are a substantial, measurable block. Writes the
//!    operator-facing PNG to `/tmp/leaderboard_risk_story_render.png`.
//! 2. [`risk_story_block_present_in_benchmark_wins_modal_case`] — the modal
//!    buy-and-hold-crowned case still carries + paints the Risk story block (it
//!    must read sensibly when holding wins), proving the block is not gated on
//!    an active win.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_scorecard_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse
//! (presence/absence of foreground, not byte-exact), robust within macOS across
//! font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

/// Render the bare Leaderboard screen body at a TALL 1920×2400 slot and return
/// the physical-pixel RGBA buffer + dimensions.
///
/// **Why 2400 px tall (vs the `leaderboard_scorecard_render.rs` 1080 px):** the
/// Risk story block sits BELOW the scorecard in the result-pane stack
/// (recommendation → table → scorecard → Risk story → disclaimer), and even on
/// the 2-row `benchmark_wins` fixture the scorecard alone reaches the bottom of
/// a 1080-px viewport. A 1080-px shot would render the Risk story off-screen
/// (the scrollable area extends below), defeating the pixel-delta proof. 2400 px
/// is comfortably above the full stack height — the Risk story block lands
/// inside the viewport regardless of the (`benchmark_wins` 2-row vs canonical
/// 13-row) fixture.
fn render_leaderboard_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 2400), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

/// Count general foreground (text / marker) pixels across the whole frame —
/// anything that crosses a luma floor the near-black `CANVAS`/`PANEL`/
/// `PANEL_RAISED` tiers never reach. Monotonic in how much content drew.
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

/// **The render-layer guard.** A populated `BakeoffReportMirror` whose
/// `tail` is `Some(..)` MUST paint a substantial Risk story block — proven by a
/// strict foreground delta against the SAME leaderboard with the tail removed
/// (the negative control). The delta is the six facts + their glosses + the
/// panel chrome + the informational footer; a regression that drops the block
/// collapses the delta.
///
/// Uses the 2-row `benchmark_wins` fixture (short table) so the Risk story block
/// — which renders directly under the scorecard — is inside the screenshot
/// viewport.
///
/// Writes both PNGs (`/tmp/leaderboard_risk_story_render.png` = WITH the block,
/// `/tmp/leaderboard_no_risk_story_render.png` = WITHOUT) for the operator
/// eyeball.
#[test]
fn risk_story_block_paints_and_exceeds_no_tail() {
    // WITH the Risk story — a short 2-row field so the under-scorecard block is
    // in-frame. The fixture carries `Some(..)`.
    let with_tail = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    assert!(
        with_tail.tail.is_some(),
        "the fixture must carry a populated tail summary"
    );

    // WITHOUT the Risk story — the SAME mirror with the tail removed. The ONLY
    // difference between the two frames is the Risk story block, so the
    // foreground delta is attributable to it alone (not a tautology).
    let mut without_tail = with_tail.clone();
    without_tail.tail = None;

    let cockpit_with = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(with_tail));
    let cockpit_without = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_tail));

    let (ww, wh, wr) = render_leaderboard_rgba(cockpit_with);
    let (nw, nh, nr) = render_leaderboard_rgba(cockpit_without);

    // Operator-facing deliverables (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(ww, wh, wr.clone()) {
        let _ = img.save("/tmp/leaderboard_risk_story_render.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(nw, nh, nr.clone()) {
        let _ = img.save("/tmp/leaderboard_no_risk_story_render.png");
    }

    let fg_with = foreground_pixels(ww, wh, &wr);
    let fg_without = foreground_pixels(nw, nh, &nr);

    // The six-fact Risk story panel (title + caption + six label/value/hint
    // stacks + the informational footer) is a lot of text. It must paint
    // STRICTLY MORE foreground than the same screen without it. The floor
    // (>1500 px of delta) is well below the measured block size but above
    // font-DB jitter, so a regression that drops the block (delta → ~0) fails
    // loudly. The threshold is slightly higher than the scorecard's (>1200)
    // because the Risk story has SIX facts to the scorecard's four.
    assert!(
        fg_with > fg_without + 1500,
        "the Risk story 'tail/median honesty' block must paint substantially \
         more foreground than the same leaderboard without it (with={fg_with} \
         vs without={fg_without}, delta={}). If the delta is small the block \
         did not render. PNG: /tmp/leaderboard_risk_story_render.png",
        fg_with as i64 - fg_without as i64
    );
}

/// **Negative-control discriminator (modal case).** The buy-and-hold-crowned
/// `BenchmarkWins` fixture STILL carries + paints the Risk story block — it
/// must read sensibly when holding wins (the honest modal crypto outcome), NOT
/// vanish. Asserts the fixture's tail is the honest "wider-tail / negative-skew"
/// shape AND that the block paints (strict foreground delta vs the same fixture
/// with the tail removed).
///
/// This is the SAME fixture as guard (1) — kept as a distinct test so the
/// "tail reads sensibly when holding wins" invariant is named and asserted
/// independently of the foreground-delta proof.
#[test]
fn risk_story_block_present_in_benchmark_wins_modal_case() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    let tail = mirror
        .tail
        .expect("benchmark-wins fixture carries a tail summary");

    // The modal case: holding is crowned, so the tail reads as the honest
    // single-asset hold (wider negative tail, mildly negative skew). The block
    // must still render with sensible plain-language copy.
    assert!(
        tail.cvar_95 < 0.0,
        "the modal benchmark-wins CVaR_95 must read as a real loss (the honest \
         path-dependent single-asset hold)"
    );
    assert!(
        tail.cvar_99 < tail.cvar_95,
        "CVaR_99 must be deeper than CVaR_95 (the extreme-tail complement)"
    );
    assert!(
        tail.median_terminal_wealth > 0.0,
        "median terminal wealth is a positive USDT figure (a real path's \
         middle outcome)"
    );

    let mut without_tail = mirror.clone();
    without_tail.tail = None;

    let cockpit_with = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let cockpit_without = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_tail));

    let (ww, wh, wr) = render_leaderboard_rgba(cockpit_with);
    let (nw, nh, nr) = render_leaderboard_rgba(cockpit_without);

    if let Some(img) = image::RgbaImage::from_raw(ww, wh, wr.clone()) {
        let _ = img.save("/tmp/leaderboard_risk_story_benchmark_wins_render.png");
    }

    let fg_with = foreground_pixels(ww, wh, &wr);
    let fg_without = foreground_pixels(nw, nh, &nr);

    assert!(
        fg_with > fg_without + 1500,
        "the Risk story block must paint in the buy-and-hold-crowned modal \
         case too (with={fg_with} vs without={fg_without}, delta={}). If the \
         delta is small the block did not render when holding won. \
         PNG: /tmp/leaderboard_risk_story_benchmark_wins_render.png",
        fg_with as i64 - fg_without as i64
    );
}
