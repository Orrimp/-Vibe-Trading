//! Story 4-13 AC3 (gap-analysis B7) — render-layer proof that the CROSS-RUN check
//! paints beside the per-run scorecard, and that its states do not look alike.
//!
//! ## Why this file exists
//!
//! The per-run scorecard already tells the operator how many strategies were tried in
//! THIS bake-off. It cannot see the other axis: someone who re-runs the bake-off on a
//! new coin, a new window, or simply a later date is running a SEQUENCE of tests, and
//! the chance that one of them says "an active strategy beats holding" grows with the
//! length of that sequence even when nothing ever does. Story 4-13 computes that; AC3
//! is that it reaches the screen.
//!
//! A presence-only test would not be enough here, and the reason is specific rather
//! than procedural. This surface has FOUR states and three of them are ways of saying
//! "we cannot conclude anything yet". If "22 runs, within chance" and "the ledger is
//! damaged and this line reads better than the truth" painted the same pixels, the
//! screen's honesty would again depend on nothing having gone wrong — which is
//! precisely the defect story 3-20 removed from the search-completeness line and which
//! `leaderboard_search_completeness_render.rs` exists to keep removed.
//!
//! Six gates:
//!
//! 1. [`the_cross_run_check_paints`] — presence, against the SAME pane with
//!    `fdr_annex = None`, which must render NOTHING: a run that kept no ledger has no
//!    sequence to describe, not even an empty one.
//! 2. [`the_none_state_takes_no_height_at_all`] — `None` renders nothing, proven at
//!    the pane's bottom edge, because a foreground count cannot distinguish "nothing"
//!    from "an empty box that still takes height".
//! 3. [`insufficient_history_does_not_look_like_a_finding`] — **the negative control.**
//!    The sufficient state and the no-history state must differ on screen.
//! 4. [`a_damaged_ledger_paints_a_warning_the_healthy_one_does_not`] — the damage case
//!    is the one where something is WRONG rather than merely absent (unreadable rows
//!    under-count the sequence, which biases the check optimistic), and it carries a
//!    `WARN_500` tint the other three states do not.
//! 5. [`the_three_insufficiency_reasons_read_differently`] — AC5's literal
//!    requirement: the three reasons are three statements, not one shrug.
//! 6. [`the_numbers_come_from_the_annex_not_a_constant`] — change the recorded run
//!    count and the pixels move, so the line is reading the mirrored annex.
//!
//! ## The measurement every threshold below is set from
//!
//! Full-pane renders at 1920 x 2400, Dark, measured 2026-09-25 by [`measure`]
//! (`cargo test -p ui --test leaderboard_cross_run_annex_render -- --ignored
//! --nocapture measure`). `fg` is content foreground left of the scrollbar gutter;
//! `warn` is `WARN_500` within a +/-24 per-channel tolerance.
//!
//! | state | fg | warn |
//! |---|---|---|
//! | sufficient (22 runs, within chance) | 77917 | 0 |
//! | `fdr_annex = None` | 71792 | 0 |
//! | insufficient: no history | 76300 | 0 |
//! | insufficient: too few runs | 76381 | 0 |
//! | insufficient: damaged ledger | 77414 | **629** |
//! | sufficient, renumbered to 137 runs | 77932 | 0 |
//!
//! | frame pair | differing px |
//! |---|---|
//! | sufficient vs `None` | 1012949 (everything below the block shifts) |
//! | sufficient vs no-history | 6432 |
//! | no-history vs too-few | 3834 |
//! | too-few vs damaged | 5733 |
//! | no-history vs damaged | 5713 |
//! | sufficient vs renumbered | 6764 |
//!
//! And the block's own height, from the pane's bottom edge: 2230 with it, 2184
//! without — 46 px, independently corroborated by `leaderboard_scorecard_render`'s
//! fold measurement (the trust block's extent went 903 -> 949 when this story landed,
//! leaving 131 px of the 1080-px fold still unspent).
//!
//! Every floor is set at roughly half its measured value: far below what the change
//! actually paints, far above font-DB jitter. Re-run [`measure`] and re-anchor the
//! table after any copy or layout change — never fit a number until it passes.
//!
//! ## macOS gate (ADR-0057 § D2)
//!
//! Real-renderer pixel assertions are macOS-canonical (cosmic-text rasterises
//! per-OS). Thresholds are presence/absence floors, not byte-exact.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use ui::leaderboard::{BakeoffReportMirror, FdrAnnexView, FdrInsufficiency};
use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

const WIDTH: u32 = 1920;
/// Tall enough that the benchmark-wins pane never clips — same rationale and value as
/// `leaderboard_scorecard_render`'s `TALL`.
const TALL: u32 = 2400;
/// The scrollbar gutter, excluded from every measurement (bug-log #96).
const GUTTER_PX: u32 = 32;

struct Frame {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

impl Frame {
    fn content_cols(&self) -> u32 {
        self.w - GUTTER_PX
    }

    fn px(&self, x: u32, y: u32) -> (i32, i32, i32) {
        let i = ((y as usize * self.w as usize) + x as usize) * 4;
        (
            i32::from(self.rgba[i]),
            i32::from(self.rgba[i + 1]),
            i32::from(self.rgba[i + 2]),
        )
    }

    fn content_foreground(&self) -> u64 {
        let mut n = 0u64;
        for y in 0..self.h {
            for x in 0..self.content_cols() {
                let (r, g, b) = self.px(x, y);
                if (r * 2 + g * 3 + b) / 6 > 80 {
                    n += 1;
                }
            }
        }
        n
    }

    /// `WARN_500` pixels — the tint the damaged-ledger line draws in.
    ///
    /// Matched within a tolerance around the theme colour rather than by a hue
    /// predicate: the leaderboard's amber `r > 130 && r > b + 40 && g > b + 25`
    /// predicate also matches `DOWN_500` drawdown clay (see
    /// `crown_credibility_render`'s `BANNER_BOTTOM` note), and this scan covers the
    /// whole pane including the ranked table.
    fn warn_pixels(&self) -> u64 {
        let c = ui::theme::color::WARN_500.current(ui::theme::ThemeMode::Dark);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let t = [
            (c.r * 255.0).round() as i32,
            (c.g * 255.0).round() as i32,
            (c.b * 255.0).round() as i32,
        ];
        let mut n = 0u64;
        for y in 0..self.h {
            for x in 0..self.content_cols() {
                let (r, g, b) = self.px(x, y);
                if (r - t[0]).abs() <= 24 && (g - t[1]).abs() <= 24 && (b - t[2]).abs() <= 24 {
                    n += 1;
                }
            }
        }
        n
    }

    /// The last row carrying any content foreground — the pane's bottom edge.
    ///
    /// The `None` gate below needs this: a foreground delta alone cannot tell "the
    /// block rendered nothing" from "the block rendered an empty box with spacing",
    /// because an empty box paints no pixels and still takes height.
    fn content_bottom(&self) -> u32 {
        (0..self.h)
            .rev()
            .find(|&y| {
                (0..self.content_cols()).any(|x| {
                    let (r, g, b) = self.px(x, y);
                    (r * 2 + g * 3 + b) / 6 > 80
                })
            })
            .unwrap_or(0)
    }

    fn differing_pixels(&self, other: &Frame) -> u64 {
        let mut n = 0u64;
        for y in 0..self.h.min(other.h) {
            for x in 0..self.content_cols() {
                let (r, g, b) = self.px(x, y);
                let (r2, g2, b2) = other.px(x, y);
                if (r - r2).abs() > 2 || (g - g2).abs() > 2 || (b - b2).abs() > 2 {
                    n += 1;
                }
            }
        }
        n
    }

    fn save(&self, path: &str) {
        if let Some(img) = image::RgbaImage::from_raw(self.w, self.h, self.rgba.clone()) {
            let _ = img.save(path);
        }
    }
}

fn render(mirror: BakeoffReportMirror) -> Frame {
    ui::force_chart_utc_for_tests();
    let cockpit: Cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let program = leaderboard_screen_program(cockpit);
    let shot = iced_test::screenshot(
        &program,
        &iced::Theme::Dark,
        (WIDTH, TALL),
        1.0,
        Duration::ZERO,
    );
    Frame {
        w: shot.size.width,
        h: shot.size.height,
        rgba: shot.rgba.to_vec(),
    }
}

/// The healthy, SUFFICIENT state: a recorded sequence long enough to speak about.
/// The fixture is within chance (22 runs, 1 beats-hold, 1.1 expected) — the honest
/// headline the story exists to surface.
fn sufficient() -> BakeoffReportMirror {
    let m = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    let a = m
        .fdr_annex
        .expect("the fixture must carry a cross-run annex, or every gate here is vacuous");
    assert!(
        a.insufficient.is_none(),
        "the baseline fixture must be the SUFFICIENT state"
    );
    assert_eq!(
        a.beats_hold_within_chance(),
        Some(true),
        "the baseline fixture must be the within-chance reading"
    );
    m
}

/// The same pane with the annex replaced by one of the insufficiency states.
fn with_insufficiency(
    why: FdrInsufficiency,
    runs: usize,
    unreadable: usize,
) -> BakeoffReportMirror {
    let mut m = sufficient();
    #[allow(clippy::cast_precision_loss)]
    let expected = runs as f64 * 0.05;
    m.fdr_annex = Some(FdrAnnexView {
        runs,
        unreadable_rows: unreadable,
        beats_hold: 0,
        expected_false_beats_hold: expected,
        insufficient: Some(why),
    });
    m
}

/// The same SUFFICIENT state with a different recorded sequence — 137 runs against
/// 6.9 expected, still within chance. Only the two numbers in the sentence change, so
/// a line that were a fixed string would render identically.
fn renumbered_mirror() -> BakeoffReportMirror {
    let mut m = sufficient();
    let a = m.fdr_annex.expect("the fixture carries an annex");
    m.fdr_annex = Some(FdrAnnexView {
        runs: 137,
        expected_false_beats_hold: 6.9,
        ..a
    });
    m
}

/// **Presence (AC3).** The cross-run check paints. The control is the SAME pane with
/// `fdr_annex = None` — a run that was never asked to record itself, which must render
/// NOTHING: there is no sequence there, not even an empty one, and printing "N = 0"
/// would present a bookkeeping choice as a finding.
///
/// Measured: with `fg=77917`, without `fg=71792` — delta 6125, the statement plus its
/// caption, both full-width sentences. Floor at 3000.
#[test]
fn the_cross_run_check_paints() {
    let with = render(sufficient());
    let mut none_mirror = sufficient();
    none_mirror.fdr_annex = None;
    let without = render(none_mirror);
    with.save("/tmp/leaderboard_cross_run_sufficient.png");
    without.save("/tmp/leaderboard_cross_run_absent.png");

    let (a, b) = (with.content_foreground(), without.content_foreground());
    assert!(
        a > b + 3000,
        "the cross-run check must paint: with={a} vs without(None)={b}, delta={}. \
         PNG: /tmp/leaderboard_cross_run_sufficient.png",
        a as i64 - b as i64
    );
}

/// **`None` renders NOTHING — not an empty box.** The presence gate above compares
/// foreground, and foreground alone cannot tell "the helper returned `None`" from "the
/// helper returned an empty container": an empty box paints no pixels and still takes
/// height. So this measures the pane's bottom edge, which moves by exactly the height
/// the block occupies.
///
/// Measured 2026-09-25: pane bottom 2230 with the block, 2184 without — 46 px, which
/// is independently corroborated by `leaderboard_scorecard_render`'s fold measurement
/// (the trust block's extent went 903 -> 949 when this story landed).
///
/// The window is tight on both sides and each side catches something:
///
/// - **below 40** — the `None` state is taking height it should not (a phantom gap),
///   or the block shrank because a line stopped rendering.
/// - **above 52** — the copy now wraps onto more lines than it did when measured.
///   That is a fold-budget event (`leaderboard_scorecard_render` asserts the trust
///   block ends above y = 1080, and it ends at 949), and catching it here says WHY
///   rather than leaving the fold gate to report a number.
#[test]
fn the_none_state_takes_no_height_at_all() {
    let with = render(sufficient());
    let mut none_mirror = sufficient();
    none_mirror.fdr_annex = None;
    let without = render(none_mirror);

    let (a, b) = (with.content_bottom(), without.content_bottom());
    let height = a as i64 - b as i64;
    assert!(
        (40..=52).contains(&height),
        "the cross-run block must occupy ~46 px and the `None` state exactly 0: pane          bottom {a} with it vs {b} without, i.e. {height} px. Below 40, the `None`          state is taking height it should not (or a line stopped rendering); above 52,          the copy has started wrapping and the trust block's fold budget          (`leaderboard_scorecard_render`, block ends at y = 949 of 1080) is being spent          here. Re-run `measure` and re-anchor on the new number if the change is          intended."
    );
}

/// **The negative control (AC3/AC5), and the reason this file exists.** A run with no
/// usable cross-run history must not render like one that has a sequence and found it
/// unremarkable. Both are honest; they are not the same statement, and a surface that
/// painted them alike would let "we cannot say" pass for "we checked".
///
/// Two-sided so neither half can be satisfied by a block that renders unconditionally:
/// the frames must differ substantially, AND the no-history frame must carry strictly
/// LESS foreground (its line is one short sentence against the sufficient state's
/// arithmetic) — which also rules out the trivial pass where the insufficient branch
/// accidentally renders the sufficient string.
#[test]
fn insufficient_history_does_not_look_like_a_finding() {
    let found = render(sufficient());
    let empty = render(with_insufficiency(FdrInsufficiency::NoHistory, 0, 0));
    empty.save("/tmp/leaderboard_cross_run_no_history.png");

    // Measured: 6432 differing px, and fg 76300 (no-history) vs 77917 (sufficient).
    let delta = found.differing_pixels(&empty);
    assert!(
        delta > 3000,
        "the sufficient and no-history states must differ on screen (delta={delta}). \
         PNG: /tmp/leaderboard_cross_run_no_history.png"
    );
    let (f, e) = (found.content_foreground(), empty.content_foreground());
    assert!(
        e < f,
        "the no-history line is one short sentence and must paint LESS than the \
         sufficient state's arithmetic: sufficient={f} vs no-history={e}"
    );
}

/// **Damage is not absence.** Unreadable ledger rows mean the sequence is LONGER than
/// the part that can be counted, which shrinks the chance-alone expectation, which
/// biases the whole check optimistic — the one direction an honesty surface must never
/// fail in quietly. So this state alone is tinted `WARN_500`, on top of saying
/// "damaged" in words (colour is never the only signal).
///
/// The gate is not "is there any amber", which the pane's other chrome could satisfy;
/// it is that the damaged frame carries substantially MORE `WARN_500` than the healthy
/// one, while the healthy one stays at its baseline.
#[test]
fn a_damaged_ledger_paints_a_warning_the_healthy_one_does_not() {
    let healthy = render(sufficient());
    let damaged = render(with_insufficiency(
        FdrInsufficiency::PartiallyUnreadable,
        40,
        3,
    ));
    damaged.save("/tmp/leaderboard_cross_run_damaged.png");

    // Measured: damaged 629 WARN_500 px, healthy 0 — the pane carries no other
    // WARN_500 at this tolerance, so the discriminator is clean rather than a margin
    // over chrome.
    let (h, d) = (healthy.warn_pixels(), damaged.warn_pixels());
    assert!(
        d > h + 300,
        "the damaged-ledger line must paint in WARN_500: healthy={h} vs damaged={d}, \
         delta={}. PNG: /tmp/leaderboard_cross_run_damaged.png",
        d as i64 - h as i64
    );
    assert!(
        h < 150,
        "the healthy state must stay at the pane's WARN_500 chrome baseline ({h}) — if \
         it drifted up, the delta above stops isolating the damage warning"
    );
}

/// **AC5 — the three reasons are three statements.** No history, one run, and a
/// damaged ledger are different things to tell an operator, and the middle one is
/// actionable in a way the first is not ("run it again" vs "there is nothing to do
/// yet"). Each pair must differ on screen.
#[test]
fn the_three_insufficiency_reasons_read_differently() {
    let none = render(with_insufficiency(FdrInsufficiency::NoHistory, 0, 0));
    let few = render(with_insufficiency(FdrInsufficiency::TooFewRuns, 1, 0));
    let bad = render(with_insufficiency(
        FdrInsufficiency::PartiallyUnreadable,
        40,
        3,
    ));
    few.save("/tmp/leaderboard_cross_run_too_few.png");

    // Measured: 3834 / 5733 / 5713 differing px for the three pairs. Floor at 1500.
    for (a, b, what) in [
        (&none, &few, "no-history vs too-few-runs"),
        (&few, &bad, "too-few-runs vs damaged"),
        (&none, &bad, "no-history vs damaged"),
    ] {
        let delta = a.differing_pixels(b);
        assert!(
            delta > 1500,
            "{what} must render differently (delta={delta}); an operator cannot act on \
             a reason the screen does not distinguish"
        );
    }
}

/// **The numbers come from the annex, not a constant.** Change the recorded run count
/// and the pixels must move. If the line were a fixed string, this frame would be
/// identical — and the check would be decoration.
#[test]
fn the_numbers_come_from_the_annex_not_a_constant() {
    let base = render(sufficient());
    let after = render(renumbered_mirror());

    // Measured: 6764 differing px — "22" -> "137" and "1.1" -> "6.9" reflow the rest
    // of a long sentence, so nearly the whole line moves. Floor at 2000.
    let delta = base.differing_pixels(&after);
    assert!(
        delta > 2000,
        "changing the recorded run count must change the rendered line (delta={delta}); \
         if it does not, the line is not reading the annex"
    );
}

/// Prints the numbers the thresholds above are set from. Run with
/// `-- --ignored --nocapture` after a layout or copy change.
#[test]
#[ignore = "measurement, not a gate"]
fn measure() {
    let s = render(sufficient());
    let mut none_mirror = sufficient();
    none_mirror.fdr_annex = None;
    let n = render(none_mirror);
    let no_hist = render(with_insufficiency(FdrInsufficiency::NoHistory, 0, 0));
    let few = render(with_insufficiency(FdrInsufficiency::TooFewRuns, 1, 0));
    let bad = render(with_insufficiency(
        FdrInsufficiency::PartiallyUnreadable,
        40,
        3,
    ));
    let renumbered = render(renumbered_mirror());

    for (label, f) in [
        ("sufficient ", &s),
        ("annex=None ", &n),
        ("no-history ", &no_hist),
        ("too-few    ", &few),
        ("damaged    ", &bad),
        ("renumbered ", &renumbered),
    ] {
        println!(
            "{label} fg={:<7} warn={}",
            f.content_foreground(),
            f.warn_pixels()
        );
    }
    println!("sufficient vs None       diff={}", s.differing_pixels(&n));
    println!(
        "sufficient vs no-history diff={}",
        s.differing_pixels(&no_hist)
    );
    println!(
        "no-history vs too-few    diff={}",
        no_hist.differing_pixels(&few)
    );
    println!(
        "too-few vs damaged       diff={}",
        few.differing_pixels(&bad)
    );
    println!(
        "no-history vs damaged    diff={}",
        no_hist.differing_pixels(&bad)
    );
    println!(
        "sufficient vs renumbered diff={}",
        s.differing_pixels(&renumbered)
    );
    println!(
        "pane bottom: sufficient={} annex=None={} (difference = the block's height)",
        s.content_bottom(),
        n.content_bottom()
    );
}
