//! advisor-overfitting-scorecard (P0-1 / ADR-0075) — render-layer proof of the
//! "How much to trust this" / "show your work" honesty block in the cockpit
//! leaderboard.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the scorecard block draws. This
//! guard renders the REAL `screens::leaderboard::view` HEADLESS with a populated
//! `BakeoffReportMirror` whose `scorecard` is `Some(..)` and asserts on the
//! rendered PIXELS that the credibility block actually paints — WITH a NEGATIVE
//! CONTROL (the SAME mirror with `scorecard = None` paints strictly less
//! foreground, i.e. no block).
//!
//! The fixtures used here are deliberately SHORT-TABLE (a 2-row buy-and-hold
//! field) so the scorecard block — which renders directly UNDER the ranked table
//! — lands inside the 1920×1080 screenshot viewport rather than scrolling off
//! below a tall 13-arm table. (The full-field render is exercised by
//! `leaderboard_populated_render.rs`; this file's job is the scorecard block.)
//!
//! Two guards (populated + negative control, per CLAUDE.md):
//!
//! 1. [`scorecard_block_paints_and_exceeds_no_scorecard`] — the populated
//!    fixture (scorecard present) paints STRICTLY MORE foreground than the SAME
//!    leaderboard with the scorecard removed. The four facts (Strategies tried /
//!    Deflated confidence / Minimum history / Beats holding?) + their plain-
//!    language glosses + the panel chrome are a substantial, measurable block.
//!    Writes the operator-facing PNG to `/tmp/leaderboard_scorecard_render.png`.
//! 2. [`scorecard_block_present_in_benchmark_wins_modal_case`] — the modal
//!    buy-and-hold-crowned case (`BenchmarkWins`, `crown_clears_dsr == false`)
//!    STILL carries + paints the scorecard block (it must read sensibly when
//!    holding wins), proving the block is not gated on an active win.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse
//! (presence/absence of foreground, not byte-exact), robust within macOS across
//! font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
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
/// `scorecard` is `Some(..)` MUST paint a substantial credibility block — proven
/// by a strict foreground delta against the SAME leaderboard with the scorecard
/// removed (the negative control). The delta is the four facts + their glosses +
/// the panel chrome; a regression that drops the block collapses the delta.
///
/// Uses the 2-row `benchmark_wins` fixture (short table) so the scorecard block
/// — which renders directly under the table — is inside the screenshot viewport.
///
/// Writes both PNGs (`/tmp/leaderboard_scorecard_render.png` = WITH the block,
/// `/tmp/leaderboard_no_scorecard_render.png` = WITHOUT) for the operator eyeball.
#[test]
fn scorecard_block_paints_and_exceeds_no_scorecard() {
    // WITH the scorecard — a short 2-row field so the under-table block is
    // in-frame. The fixture carries `Some(..)`.
    let with_sc = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    assert!(
        with_sc.scorecard.is_some(),
        "the fixture must carry a populated scorecard"
    );

    // WITHOUT the scorecard — the SAME mirror with the block removed. The ONLY
    // difference between the two frames is the scorecard block, so the
    // foreground delta is attributable to it alone (not a tautology).
    //
    // **Also remove the Risk story (`tail`) in BOTH baselines** so the delta is
    // strictly "scorecard added/removed" — without this, removing the
    // scorecard would shift the Risk story block UP into the 1080-px viewport
    // (the scorecard sits above the Risk story in the stack), inflating
    // `fg_without` and breaking the strict-positive-delta proof. Setting both
    // to `tail = None` keeps the test isolated to the scorecard contribution
    // (the post-`advisor-turnover-and-tail-metrics` discipline — each report-
    // only block has its own render guard; the scorecard test only proves the
    // scorecard renders).
    let mut with_sc_tail_off = with_sc.clone();
    with_sc_tail_off.tail = None;
    let mut without_sc = with_sc_tail_off.clone();
    without_sc.scorecard = None;

    let cockpit_with = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(with_sc_tail_off));
    let cockpit_without = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_sc));

    let (ww, wh, wr) = render_leaderboard_rgba(cockpit_with);
    let (nw, nh, nr) = render_leaderboard_rgba(cockpit_without);

    // Operator-facing deliverables (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(ww, wh, wr.clone()) {
        let _ = img.save("/tmp/leaderboard_scorecard_render.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(nw, nh, nr.clone()) {
        let _ = img.save("/tmp/leaderboard_no_scorecard_render.png");
    }

    let fg_with = foreground_pixels(ww, wh, &wr);
    let fg_without = foreground_pixels(nw, nh, &nr);

    // The four-fact scorecard panel (title + caption + four label/value/hint
    // stacks) is a lot of text. It must paint STRICTLY MORE foreground than the
    // same screen without it. The floor (>1200 px of delta) is well below the
    // measured block size but above font-DB jitter, so a regression that drops
    // the block (delta → ~0) fails loudly.
    assert!(
        fg_with > fg_without + 1200,
        "the scorecard 'show your work' block must paint substantially more \
         foreground than the same leaderboard without it (with={fg_with} vs \
         without={fg_without}, delta={}). If the delta is small the credibility \
         block did not render. PNG: /tmp/leaderboard_scorecard_render.png",
        fg_with as i64 - fg_without as i64
    );
}

/// **Negative-control discriminator (modal case).** The buy-and-hold-crowned
/// `BenchmarkWins` fixture STILL carries + paints the scorecard block — it must
/// read sensibly when holding wins (the honest modal crypto outcome), NOT
/// vanish. Asserts the fixture's scorecard is the honest "doesn't clear DSR"
/// shape AND that the block paints (strict foreground delta vs the same fixture
/// with the scorecard removed).
///
/// This is the SAME fixture as guard (1) — kept as a distinct test so the
/// `crown_clears_dsr == false` modal-case invariant is named and asserted
/// independently of the foreground-delta proof.
#[test]
fn scorecard_block_present_in_benchmark_wins_modal_case() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    let sc = mirror
        .scorecard
        .expect("benchmark-wins fixture carries a scorecard");

    // The modal case: holding is crowned, so the scorecard's "beats holding
    // after the search?" check is honestly false — the block must still render.
    assert!(
        !sc.crown_clears_dsr,
        "the modal benchmark-wins scorecard must read 'does not clear DSR' \
         (the honest 'holding is the call' case)"
    );
    // The mirrored fields are the real (non-degenerate) values, not zeros.
    assert!(sc.n_candidates >= 2, "real candidate count");
    assert!(
        sc.deflated_sharpe > 0.0 && sc.deflated_sharpe < 1.0,
        "DSR is a real probability in (0, 1)"
    );

    // Same Risk-story-isolation discipline as the first guard — see comment
    // there. Strip BOTH baselines' `tail` so the delta is strictly "scorecard
    // present/absent" (the only block that should move with this test's
    // intent).
    let mut with_sc_tail_off = mirror;
    with_sc_tail_off.tail = None;
    let mut without_sc = with_sc_tail_off.clone();
    without_sc.scorecard = None;

    let cockpit_with = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(with_sc_tail_off));
    let cockpit_without = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_sc));

    let (ww, wh, wr) = render_leaderboard_rgba(cockpit_with);
    let (nw, nh, nr) = render_leaderboard_rgba(cockpit_without);

    if let Some(img) = image::RgbaImage::from_raw(ww, wh, wr.clone()) {
        let _ = img.save("/tmp/leaderboard_scorecard_benchmark_wins_render.png");
    }

    let fg_with = foreground_pixels(ww, wh, &wr);
    let fg_without = foreground_pixels(nw, nh, &nr);

    assert!(
        fg_with > fg_without + 1200,
        "the scorecard block must paint in the buy-and-hold-crowned modal case \
         too (with={fg_with} vs without={fg_without}, delta={}). If the delta is \
         small the block did not render when holding won. \
         PNG: /tmp/leaderboard_scorecard_benchmark_wins_render.png",
        fg_with as i64 - fg_without as i64
    );
}
