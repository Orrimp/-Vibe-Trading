//! Story 3-20 AC1/AC2 — render-layer proof that the verdict surface says WHAT IT
//! SEARCHED, and that a search which silently lost arms does not look like an
//! honest null.
//!
//! ## Why this file exists
//!
//! Product review 2026-08-04, finding 1: the success state and the silent-failure
//! state were visually identical. "No strategy beat holding" and "we meant to run
//! twelve, something broke, and here is what came back" rendered the same pixels, so
//! the screen's honesty depended entirely on nothing having gone wrong. AC2 requires
//! at least one negative control proving the two differ ON SCREEN — a model-layer
//! assertion would prove exactly the wrong half.
//!
//! Three gates:
//!
//! 1. [`search_statement_paints`] — presence, against the same pane with the
//!    requested set cleared (which must render nothing, because a mirror that does
//!    not know what was asked for cannot claim completeness).
//! 2. [`an_incomplete_search_does_not_look_like_an_honest_null`] — **the negative
//!    control.** An arm that was requested and did not come back turns the statement
//!    into a `DOWN_500` warning, and the honest-null frame paints none of that colour
//!    in the same band.
//! 3. [`the_arm_names_come_from_the_request_not_the_rows`] — renaming a requested arm
//!    changes the pixels, so the inventory line is reading the live request rather
//!    than a hardcoded list (AC1).
//!
//! ## macOS gate (ADR-0057 § D2)
//!
//! Real-renderer pixel assertions are macOS-canonical.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

use ui::leaderboard::BakeoffReportMirror;
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

    /// `DOWN_500` pixels — the failure colour the incomplete warning draws in.
    fn down_pixels(&self) -> u64 {
        let c = ui::theme::color::DOWN_500.current(ui::theme::ThemeMode::Dark);
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

/// The honest null: buy-and-hold wins, every requested arm came back.
fn complete() -> BakeoffReportMirror {
    let m = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    assert!(
        !m.requested_arms.is_empty(),
        "the fixture must carry the requested set"
    );
    assert_eq!(
        m.requested_arms.len(),
        m.rows.iter().filter(|r| !r.is_benchmark).count(),
        "the fixture must model a COMPLETE search, or gate 2's control is vacuous"
    );
    m
}

/// The same run, with one requested arm silently absent from the results.
fn incomplete() -> BakeoffReportMirror {
    let mut m = complete();
    let victim = m
        .rows
        .iter()
        .position(|r| !r.is_benchmark)
        .expect("an active arm to drop");
    m.rows.remove(victim);
    m.ranked.retain(|&i| i != victim);
    for i in &mut m.ranked {
        if *i > victim {
            *i -= 1;
        }
    }
    m.crowned = m.crowned.and_then(|c| match c.cmp(&victim) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(c - 1),
        std::cmp::Ordering::Less => Some(c),
    });
    m
}

/// **Presence (AC1).** The statement paints. The control is the SAME pane with the
/// requested set cleared, which must render nothing at all — a mirror that does not
/// know what was asked for cannot substantiate a completeness claim, and inventing
/// one would be the defect this story exists to remove.
///
/// Measured 2026-09-24: complete `fg=61559`, cleared `fg=60277` (delta 1282 — the
/// headline plus the arm-inventory line). The floor sits well below that and far
/// above font-DB jitter.
#[test]
fn search_statement_paints() {
    let with = render(complete());
    let mut cleared_mirror = complete();
    cleared_mirror.requested_arms.clear();
    let without = render(cleared_mirror);
    with.save("/tmp/leaderboard_search_complete.png");

    let (a, b) = (with.content_foreground(), without.content_foreground());
    assert!(
        a > b + 700,
        "the search-completeness statement must paint: with={a} vs without={b}, \
         delta={}. PNG: /tmp/leaderboard_search_complete.png",
        a as i64 - b as i64
    );
}

/// **The negative control (AC2), and the reason this file exists.** A run that lost
/// an arm must not render like the honest null.
///
/// The honest-null pane already carries some `DOWN_500` — drawdown percentages in the
/// ranked table (measured: 190 px). So the gate is not "is there any red", which
/// would pass on the table alone; it is that the incomplete pane carries
/// substantially MORE (measured: 612 px, i.e. +422 for the warning line), while the
/// complete pane stays at its table-chrome baseline.
#[test]
fn an_incomplete_search_does_not_look_like_an_honest_null() {
    let honest = render(complete());
    let broken = render(incomplete());
    broken.save("/tmp/leaderboard_search_incomplete.png");

    let (h, i) = (honest.down_pixels(), broken.down_pixels());
    assert!(
        i > h + 250,
        "an incomplete search must paint the warning in DOWN_500: honest={h} vs \
         incomplete={i}, delta={}. PNG: /tmp/leaderboard_search_incomplete.png",
        i as i64 - h as i64
    );
    assert!(
        h < 400,
        "the honest null must stay at the table's DOWN_500 chrome baseline ({h}) — if \
         it drifted up, the delta above stops isolating the warning"
    );
    assert!(
        honest.differing_pixels(&broken) > 1000,
        "the two states must differ on screen at all (AC2's literal requirement)"
    );
}

/// **AC1 — the names come from the live request, not a hardcoded list.** Rename a
/// requested arm and the pixels must move. If the inventory line were a constant,
/// this frame would be identical.
#[test]
fn the_arm_names_come_from_the_request_not_the_rows() {
    let base = render(complete());
    let mut renamed = complete();
    renamed.requested_arms[0] = "v9.zzz_renamed_for_this_test".into();
    let after = render(renamed);

    let delta = base.differing_pixels(&after);
    assert!(
        delta > 200,
        "renaming a requested arm must change the rendered inventory line (delta={delta}); \
         if it does not, the line is not reading the request"
    );
}

/// **AC5 — the next step exists after "hold", and ONLY then.**
///
/// The negative control is the active-wins fixture: when a strategy actually won there
/// is nothing to explain, and a "what would have to change" panel there would be
/// filler. So the gate is two-sided — the panel must paint on the hold outcome and
/// must NOT paint on the win outcome. A one-sided presence check would pass on a panel
/// that renders unconditionally, which is the thing being ruled out.
#[test]
fn the_next_step_appears_after_hold_and_not_after_a_win() {
    let hold = render(complete());
    let win = ui::fixtures::fake_bakeoff_report_mirror();
    assert!(
        matches!(
            win.recommendation.outcome,
            ui::leaderboard::OutcomeKind::ActiveWins
        ),
        "the control fixture must be an active WIN, or this gate proves nothing"
    );
    let win_frame = render(win);

    // Same pane, minus the next-step panel: the hold fixture with the outcome flipped
    // to ActiveWins renders everything else identically and the panel not at all.
    let mut suppressed = complete();
    suppressed.recommendation.outcome = ui::leaderboard::OutcomeKind::ActiveWins;
    let without = render(suppressed);

    let (with_fg, without_fg) = (hold.content_foreground(), without.content_foreground());
    assert!(
        with_fg > without_fg + 400,
        "the next-step panel must paint after a hold verdict: with={with_fg} vs \
         without={without_fg}, delta={}",
        with_fg as i64 - without_fg as i64
    );
    assert!(
        win_frame.content_foreground() > 0,
        "the win fixture must render at all (guards against a vacuous comparison)"
    );
}

/// Prints the numbers the thresholds above are set from. Run with
/// `-- --ignored --nocapture` after a layout change.
#[test]
#[ignore = "measurement, not a gate"]
fn measure() {
    let c = render(complete());
    let mut cleared = complete();
    cleared.requested_arms.clear();
    let n = render(cleared);
    let i = render(incomplete());
    c.save("/tmp/search_complete.png");
    i.save("/tmp/search_incomplete.png");
    println!(
        "complete   fg={} down={}",
        c.content_foreground(),
        c.down_pixels()
    );
    println!(
        "cleared    fg={} down={}",
        n.content_foreground(),
        n.down_pixels()
    );
    println!(
        "incomplete fg={} down={}",
        i.content_foreground(),
        i.down_pixels()
    );
    println!("complete vs cleared    diff={}", c.differing_pixels(&n));
    println!("complete vs incomplete diff={}", c.differing_pixels(&i));
}
