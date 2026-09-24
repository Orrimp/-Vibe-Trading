//! advisor-overfitting-scorecard (P0-1 / ADR-0075, placement ADR-0092) — render-layer
//! proof that the "How much to trust this" / "show your work" honesty block in the
//! cockpit leaderboard paints, and that it paints ABOVE THE FOLD.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the scorecard block draws. This
//! guard renders the REAL `screens::leaderboard::view` HEADLESS with a populated
//! `BakeoffReportMirror` whose `scorecard` is `Some(..)` and asserts on the
//! rendered PIXELS, against a NEGATIVE CONTROL (the SAME mirror with
//! `scorecard = None`).
//!
//! ## What bug-log #96 changed here
//!
//! Until 2026-09-15 this gate compared whole-frame foreground at 1920×1080 with and
//! without the block. Once the Data quality panel was stacked above the table, the
//! block rendered entirely below the fold, and the ONLY pixels that differed between
//! the two frames were the scrollbar thumb — a 10-px strip at x≈1894. The gate was
//! measuring chrome, with the opposite sign (`delta=-1980`). Two rules now keep that
//! failure from repeating silently:
//!
//! 1. No measurement reads the scrollbar gutter ([`GUTTER_PX`]).
//! 2. "It paints" and "it is visible without scrolling" are separate assertions.
//!    Presence is measured in a frame tall enough that nothing scrolls or clips
//!    (asserted, not assumed), so the with/without delta is exactly the block.
//!    Visibility is asserted from the block's measured extent: it must end above
//!    y = 1080, and the 1080-px frame must paint those rows exactly as the unclipped
//!    frame does.
//!
//! Guards:
//!
//! 1. [`scorecard_block_paints_and_exceeds_no_scorecard`] — presence, unclipped.
//! 2. [`scorecard_block_is_entirely_above_the_fold`] — the #96 product requirement.
//! 3. [`scorecard_block_present_in_benchmark_wins_modal_case`] — the modal
//!    buy-and-hold case (`BenchmarkWins`, `crown_clears_dsr == false`) still carries
//!    and paints the block: it is not gated on an active win.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical. The file compiles to nothing on Linux/Windows. Thresholds are
//! presence/absence floors, not byte-exact.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

/// The operator's `typical` viewport width.
const WIDTH: u32 = 1920;
/// The fold #96 is about: the `typical` 1920×1080 viewport's height.
const FOLD: u32 = 1080;
/// Tall enough that the whole benchmark-wins pane fits without scrolling. Asserted by
/// [`assert_unclipped`], so a future block that outgrows it fails loudly instead of
/// silently clipping the measurement.
const TALL: u32 = 2400;
/// Right-edge strip where the pane's vertical scrollbar draws (x = 1894..=1904 of
/// 1920 in #96). Every content measurement stops short of it.
const GUTTER_PX: u32 = 32;

struct Frame {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

impl Frame {
    /// Text / marker pixel: crosses a luma floor the near-black `CANVAS` / `PANEL` /
    /// `PANEL_RAISED` tiers never reach.
    fn fg(&self, x: u32, y: u32) -> bool {
        let i = ((y as usize * self.w as usize) + x as usize) * 4;
        let (r, g, b) = (
            i32::from(self.rgba[i]),
            i32::from(self.rgba[i + 1]),
            i32::from(self.rgba[i + 2]),
        );
        (r * 2 + g * 3 + b) / 6 > 80
    }

    fn content_cols(&self) -> u32 {
        self.w - GUTTER_PX
    }

    /// Foreground pixels left of the scrollbar gutter.
    fn content_foreground(&self) -> u64 {
        let mut n = 0u64;
        for y in 0..self.h {
            for x in 0..self.content_cols() {
                if self.fg(x, y) {
                    n += 1;
                }
            }
        }
        n
    }

    /// Is content row `y` of `self` pixel-identical to content row `y_other` of `other`?
    fn row_eq(&self, y: u32, other: &Frame, y_other: u32) -> bool {
        let span = self.content_cols() as usize * 4;
        let a = (y as usize * self.w as usize) * 4;
        let b = (y_other as usize * other.w as usize) * 4;
        self.rgba[a..a + span] == other.rgba[b..b + span]
    }

    /// Same as [`Frame::row_eq`], but tolerant of anti-aliasing.
    ///
    /// Measured 2026-09-16 between the 1080-px and 2400-px frames of the same pane: ONE
    /// row of 358 differed, by 3 pixels at x = 21..23, each channel off by 1/255 — the
    /// anti-aliased edge of a panel's rounded corner, which the two frame heights
    /// rasterise a hair differently. A clip, a scrollbar overlay or a layout shift moves
    /// whole glyphs and blows past this by two orders of magnitude.
    fn row_matches(&self, y: u32, other: &Frame, y_other: u32) -> bool {
        const AA: i32 = 2;
        let span = self.content_cols() as usize * 4;
        let a = (y as usize * self.w as usize) * 4;
        let b = (y_other as usize * other.w as usize) * 4;
        self.rgba[a..a + span]
            .iter()
            .zip(&other.rgba[b..b + span])
            .all(|(p, q)| (i32::from(*p) - i32::from(*q)).abs() <= AA)
    }

    fn row_has_fg(&self, y: u32) -> bool {
        (0..self.content_cols()).any(|x| self.fg(x, y))
    }

    /// Operator-facing deliverable (memory: verify UI at the render layer).
    fn save(&self, path: &str) {
        if let Some(img) = image::RgbaImage::from_raw(self.w, self.h, self.rgba.clone()) {
            let _ = img.save(path);
        }
    }
}

/// Render the bare Leaderboard screen body, `WIDTH` wide and `height` tall.
fn render(cockpit: Cockpit, height: u32) -> Frame {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (WIDTH, height), 1.0, Duration::ZERO);
    Frame {
        w: shot.size.width,
        h: shot.size.height,
        rgba: shot.rgba.to_vec(),
    }
}

/// The pane must END inside a tall frame — its bottom band carries no content.
/// Otherwise the with/without delta would include whatever the shorter frame
/// un-clips, and would no longer be the block alone.
fn assert_unclipped(f: &Frame, what: &str) {
    const BAND: u32 = 48;
    let clipped = (f.h - BAND..f.h).any(|y| f.row_has_fg(y));
    assert!(
        !clipped,
        "{what}: content reaches the bottom {BAND} px of the {}-px frame, so the pane is \
         clipped and the with/without delta is not the block alone — raise TALL",
        f.h
    );
}

/// **Presence.** A populated `BakeoffReportMirror` whose `scorecard` is `Some(..)`
/// MUST paint a substantial credibility block: strictly more content-area foreground
/// than the SAME leaderboard without it. Both frames are unclipped, so blocks that move
/// when the scorecard is removed are still counted in full, and the delta is the four
/// facts + their glosses + the panel chrome (the Risk story no longer has to be
/// switched off to isolate it, as the 1080-px version of this test had to).
///
/// Writes `/tmp/leaderboard_scorecard_render_tall.png` (WITH) and
/// `/tmp/leaderboard_no_scorecard_render_tall.png` (WITHOUT).
#[test]
fn scorecard_block_paints_and_exceeds_no_scorecard() {
    let with_sc = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    assert!(
        with_sc.scorecard.is_some(),
        "the fixture must carry a populated scorecard"
    );
    let mut without_sc = with_sc.clone();
    without_sc.scorecard = None;

    let with = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(with_sc)),
        TALL,
    );
    let without = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_sc)),
        TALL,
    );
    with.save("/tmp/leaderboard_scorecard_render_tall.png");
    without.save("/tmp/leaderboard_no_scorecard_render_tall.png");
    // `without` is shorter by the block, so `with` fitting implies both fit.
    assert_unclipped(&with, "with the scorecard");

    let fg_with = with.content_foreground();
    let fg_without = without.content_foreground();
    // Title + caption + four label/value/hint stacks is a lot of text; the floor sits
    // far below the measured block and far above font-DB jitter, so a regression that
    // drops the block (delta → ~0) fails loudly.
    assert!(
        fg_with > fg_without + 1200,
        "the scorecard 'show your work' block must paint substantially more foreground \
         than the same leaderboard without it (with={fg_with} vs without={fg_without}, \
         delta={}). PNG: /tmp/leaderboard_scorecard_render_tall.png",
        fg_with as i64 - fg_without as i64
    );
}

/// **Visibility — the #96 product requirement.** On the operator's 1920×1080 viewport
/// the whole "How much to trust this" block is on screen without scrolling.
///
/// The block's extent is measured, not assumed, in the two unclipped frames: it starts
/// at the first content row where WITH and WITHOUT diverge, and it is exactly as tall
/// as the distance everything after it moves down. That extent must end above the
/// fold, and the 1080-px frame must paint every row of it exactly as the unclipped
/// frame does — not a scrollbar, not a clip.
///
/// Writes `/tmp/leaderboard_scorecard_render.png` (the 1080-px frame, the one the
/// operator sees).
#[test]
fn scorecard_block_is_entirely_above_the_fold() {
    let with_sc = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    assert!(
        with_sc.scorecard.is_some(),
        "the fixture must carry a scorecard"
    );
    let mut without_sc = with_sc.clone();
    without_sc.scorecard = None;

    let tall_with = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(with_sc.clone())),
        TALL,
    );
    let tall_without = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_sc)),
        TALL,
    );
    assert_unclipped(&tall_with, "with the scorecard");

    // Where the block starts: the first content row where the two frames diverge.
    let y0 = (0..TALL)
        .find(|&y| !tall_with.row_eq(y, &tall_without, y))
        .expect("the frames never diverge left of the gutter: the scorecard block did not render");

    // How tall it is: the block that follows it moves down by exactly its height (plus
    // the stack spacing). Take the first text-bearing rows of that block as drawn
    // WITHOUT the scorecard — text, so blank rows cannot match an arbitrary shift — and
    // find them in the frame WITH it.
    //
    // The shift is the offset at which MOST of that window matches byte-for-byte, not
    // all of it. Measured 2026-09-24, after 3-20 added three lines to the block: at the
    // true shift (430 px) 44 of the 48 rows are byte-identical and 4 differ across
    // nearly the full content width (x = 17..1887) — a 1-px full-width hairline whose
    // container now lands on a fractional y and rasterises one row off. Requiring all
    // 48 made the measurement, not the property, the thing that failed: the fold
    // assertion below was comfortably satisfied (the block ends at 903 of 1080) while
    // the test reported "the block after the scorecard was not found".
    //
    // So: take the offset that maximises exact row matches, and require that it is a
    // DOMINANT match. That still proves what the extent measurement needs — the pane
    // below is the same content, translated — and refuses a coincidental alignment.
    // It does not touch the fold assertion, which is the actual product requirement
    // (ADR-0092 D3); loosening THAT is the #77 failure this file exists to avoid.
    const WINDOW: u32 = 48;
    const MIN_MATCHING_ROWS: usize = 40;
    let y1 = (y0..TALL)
        .find(|&y| tall_without.row_has_fg(y))
        .expect("the pane has content after the scorecard's position");
    let (shift, matched) = (1..TALL.saturating_sub(y1 + WINDOW))
        .map(|d| {
            (
                d,
                (y1..y1 + WINDOW)
                    .filter(|&y| tall_without.row_eq(y, &tall_with, y + d))
                    .count(),
            )
        })
        .max_by_key(|&(_, n)| n)
        .expect("a non-empty shift range");
    assert!(
        matched >= MIN_MATCHING_ROWS,
        "the block after the scorecard was not found, translated, in the frame that has \
         it: the best offset ({shift} px) reproduces only {matched} of {WINDOW} rows \
         byte-for-byte. Below {MIN_MATCHING_ROWS} the pane is not merely shifted — \
         something re-laid out, and the extent measured from it would be fiction."
    );
    // Conservative bottom: from the first diverging row, so it can only over-state.
    let bottom = y0 + shift;

    let fold = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(with_sc)),
        FOLD,
    );
    fold.save("/tmp/leaderboard_scorecard_render.png");
    eprintln!(
        "scorecard block: rows {y0}..{bottom} (height incl. spacing {shift} px), fold at {FOLD}"
    );

    assert!(
        bottom <= FOLD,
        "the 'How much to trust this' block must be entirely visible on a {WIDTH}x{FOLD} \
         viewport without scrolling (bug-log #96), but it ends at row {bottom}. \
         PNG: /tmp/leaderboard_scorecard_render.png"
    );
    let diverging: Vec<u32> = (y0..bottom)
        .filter(|&y| !fold.row_matches(y, &tall_with, y))
        .collect();
    assert!(
        diverging.is_empty(),
        "the 1080-px frame must paint the scorecard rows as the unclipped frame does; {} of \
         rows {y0}..{bottom} differ by more than anti-aliasing (first: {:?}). \
         PNG: /tmp/leaderboard_scorecard_render.png",
        diverging.len(),
        diverging.first()
    );
}

/// **Negative-control discriminator (modal case).** The buy-and-hold-crowned
/// `BenchmarkWins` fixture STILL carries + paints the scorecard block — it must read
/// sensibly when holding wins (the honest modal crypto outcome), NOT vanish. Asserts
/// the fixture's scorecard is the honest "doesn't clear DSR" shape AND that the block
/// paints (the same unclipped delta as guard 1).
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

    // The modal case: holding is crowned, so the scorecard's "beats holding after the
    // search?" check is honestly false — the block must still render.
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

    let mut without_sc = mirror.clone();
    without_sc.scorecard = None;
    let with = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror)),
        TALL,
    );
    let without = render(
        ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(without_sc)),
        TALL,
    );
    with.save("/tmp/leaderboard_scorecard_benchmark_wins_render.png");
    assert_unclipped(&with, "with the scorecard (modal case)");

    let fg_with = with.content_foreground();
    let fg_without = without.content_foreground();
    assert!(
        fg_with > fg_without + 1200,
        "the scorecard block must paint in the buy-and-hold-crowned modal case too \
         (with={fg_with} vs without={fg_without}, delta={}). \
         PNG: /tmp/leaderboard_scorecard_benchmark_wins_render.png",
        fg_with as i64 - fg_without as i64
    );
}
