//! Story 3-21 AC4 / bug-log #102 — render-layer proof that the Lab screen
//! TELLS the operator whether their saved session came back.
//!
//! ## Why this file exists
//!
//! Until #102 the cockpit restored nothing and saved nothing: every launch was
//! a cold start, and a cold start is pixel-identical to "your saved selection,
//! faithfully restored". The notice this guards is the only thing that makes
//! the difference visible — so proving it at the model layer would prove
//! exactly the wrong half. MEMORY.md "verify UI at the render layer".
//!
//! Three states, rendered through the REAL shell → `screens::lab::view`:
//!
//! | outcome | what must be on screen |
//! |---|---|
//! | `Fresh` | "Fresh session" — an answer, not a silence |
//! | `Restored` | the same line replaced by a timestamp |
//! | `Failed` | the failure AND where the unreadable file was kept, in `DOWN_500` |
//!
//! The negative control is the one that matters here: rendering the SAME
//! outcome twice must be byte-identical, so a difference between two DIFFERENT
//! outcomes is the notice and not renderer jitter. Without it, "the frames
//! differ" proves nothing — that is the bug-log #96 failure mode, where a gate
//! passed on a scrollbar thumb.
//!
//! ## macOS gate (ADR-0057 § D2)
//!
//! Real-renderer pixel assertions are macOS-canonical (cosmic-text rasterises
//! per-OS). The file compiles to nothing elsewhere.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Duration;

use ui::lab::persistence::{FailureReason, RestoreFailure, RestoreOutcome, SCHEMA_VERSION};
use ui::state::{Cockpit, Screen};
use ui::test_support::program_from_cockpit;

const W: u32 = 1600;
const H: u32 = 900;

/// The notice draws in the Lab body's first row, and NOWHERE else. Measured on
/// 2026-09-24 at 1600x900 by the `measure` test below, which prints the per-band
/// deltas the three outcomes actually produce:
///
/// ```text
/// restored: y    0..60   diff=1120     down=0
/// restored: y   60..900  diff=0        down=1594
/// failed:   y    0..60   diff=4632     down=570
/// failed:   y   60..900  diff=0        down=1594
/// fresh:    y    0..60                 down=0
/// ```
///
/// The `down=1594` below the band is pre-existing Lab chrome — it is why the
/// colour assertion is banded rather than whole-frame, and why the band is
/// stated with the render it came from (ADR-0092's rule after bug-log #96).
const BAND_TOP: u32 = 0;
const BAND_BOTTOM: u32 = 60;
/// Everything below the notice must be byte-identical across outcomes: the line
/// is a notice, not a layout change.
const BELOW_BAND: u32 = BAND_BOTTOM;

fn lab_cockpit(outcome: RestoreOutcome) -> Cockpit {
    let mut c = ui::test_support::charts_screen_cockpit();
    c.current_screen = Screen::Lab;
    c.lab_restore = outcome;
    // The notice only renders when persistence is ARMED for this cockpit —
    // a fixture cockpit persists nothing and must stay silent about it. The
    // screenshot path renders `view` only (never `update`), so nothing is
    // written to this path; `persistence_off_renders_nothing` is the other
    // half of that contract.
    c.lab_state_path = Some(PathBuf::from("/tmp/lab-session-restore-render.json"));
    c
}

fn render(outcome: RestoreOutcome) -> Vec<u8> {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(lab_cockpit(outcome));
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (W, H), 1.0, Duration::ZERO)
        .rgba
        .to_vec()
}

/// Pixels differing by more than tiny-skia's AA noise, within a y-band.
fn diff_in_band(a: &[u8], b: &[u8], top: u32, bottom: u32) -> u64 {
    let mut n = 0u64;
    for y in top..bottom.min(H) {
        for x in 0..W {
            let i = ((y as usize * W as usize) + x as usize) * 4;
            let d = (0..3)
                .map(|k| i32::from(a[i + k]).abs_diff(i32::from(b[i + k])))
                .max()
                .unwrap_or(0);
            if d > 2 {
                n += 1;
            }
        }
    }
    n
}

/// `DOWN_500` pixels — the failure colour — within a y-band.
fn down_pixels(rgba: &[u8], top: u32, bottom: u32) -> u64 {
    let c = ui::theme::color::DOWN_500.current(ui::theme::ThemeMode::Dark);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let target = [
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    ];
    let mut n = 0u64;
    for y in top..bottom.min(H) {
        for x in 0..W {
            let i = ((y as usize * W as usize) + x as usize) * 4;
            if (0..3).all(|k| (i32::from(rgba[i + k]) - target[k]).abs() <= 24) {
                n += 1;
            }
        }
    }
    n
}

fn failed() -> RestoreOutcome {
    RestoreOutcome::Failed(RestoreFailure {
        reason: FailureReason::UnsupportedVersion {
            found: 99,
            supported: SCHEMA_VERSION,
        },
        preserved_at: Some(PathBuf::from(
            "/tmp/cockpit-lab-state.json.unreadable-1758700000",
        )),
    })
}

fn restored() -> RestoreOutcome {
    RestoreOutcome::Restored {
        saved_at: Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_758_700_000)),
    }
}

/// **The negative control, and it comes first.** Two renders of the SAME
/// outcome must be byte-identical in the band. Everything below asserts that
/// two frames DIFFER; without this, renderer jitter would satisfy all of it —
/// which is exactly how bug-log #96's gate passed on a scrollbar thumb.
#[test]
fn identical_outcomes_render_identically() {
    let a = render(RestoreOutcome::Fresh);
    let b = render(RestoreOutcome::Fresh);
    assert_eq!(
        diff_in_band(&a, &b, BAND_TOP, BAND_BOTTOM),
        0,
        "the same outcome must render the same pixels, or every diff below is noise"
    );
}

/// AC4 — "Fresh session" and "Restored from <when>" are DIFFERENT pixels.
/// Before bug-log #102 was fixed these two states were indistinguishable on
/// screen, because the cockpit restored nothing and every launch was fresh.
#[test]
fn restored_reads_differently_from_fresh() {
    let fresh = render(RestoreOutcome::Fresh);
    let restored = render(restored());
    assert!(
        diff_in_band(&fresh, &restored, BAND_TOP, BAND_BOTTOM) > 600,
        "a restored session must say so; measured 1120 differing pixels"
    );
    assert_eq!(
        diff_in_band(&fresh, &restored, BELOW_BAND, H),
        0,
        "the notice must not move the rest of the screen"
    );
}

/// AC3/AC4 — a session that could not be read says so, in the failure colour,
/// and names where the file was kept. The colour is asserted because the text
/// alone would also be satisfied by a muted line the operator scrolls past.
#[test]
fn a_failed_restore_is_loud_and_says_where_the_file_went() {
    let fresh = render(RestoreOutcome::Fresh);
    let failed_shot = render(failed());

    assert!(
        diff_in_band(&fresh, &failed_shot, BAND_TOP, BAND_BOTTOM) > 2500,
        "the failure line is long — reason plus the kept-aside path; measured 4632"
    );
    assert!(
        down_pixels(&failed_shot, BAND_TOP, BAND_BOTTOM) > 300,
        "the failure must draw in DOWN_500; measured 570 pixels"
    );
    assert_eq!(
        down_pixels(&fresh, BAND_TOP, BAND_BOTTOM),
        0,
        "and a healthy session must draw none of it — otherwise the colour proves nothing"
    );
    assert_eq!(
        down_pixels(&render(restored()), BAND_TOP, BAND_BOTTOM),
        0,
        "nor may a successful restore"
    );
    assert_eq!(
        diff_in_band(&fresh, &failed_shot, BELOW_BAND, H),
        0,
        "the notice must not move the rest of the screen"
    );
}

/// The other half of D3: a cockpit with persistence OFF says nothing at all.
/// Telling a fixture or gallery viewer "Fresh session" would claim a durability
/// they do not have — and it moved 8 byte-exact `render_snapshots` baselines
/// before this gate existed.
#[test]
fn persistence_off_renders_nothing() {
    ui::force_chart_utc_for_tests();
    let mut c = ui::test_support::charts_screen_cockpit();
    c.current_screen = Screen::Lab;
    c.lab_restore = RestoreOutcome::Fresh;
    assert!(
        c.lab_state_path.is_none(),
        "a fixture cockpit must not be armed for persistence"
    );
    let unarmed = iced_test::screenshot(
        &program_from_cockpit(c),
        &iced::Theme::Dark,
        (W, H),
        1.0,
        Duration::ZERO,
    )
    .rgba
    .to_vec();

    let armed = render(RestoreOutcome::Fresh);
    // 250, not 600: "Fresh session" alone is 387 differing pixels (measured
    // 2026-09-24). The 1120 in `restored_reads_differently_from_fresh` is the
    // UNION of two strings' pixels, which is a different quantity — reusing
    // that number here would have been a threshold fitted to the wrong render.
    let delta = diff_in_band(&unarmed, &armed, BAND_TOP, BAND_BOTTOM);
    assert!(
        delta > 250,
        "an armed cockpit says 'Fresh session'; an unarmed one must not (delta={delta})"
    );
}

/// Prints what the three outcomes actually do to the frame. Run with
/// `-- --ignored --nocapture` when re-siting the band after a layout change.
#[test]
#[ignore = "measurement, not a gate"]
fn measure() {
    let fresh = render(RestoreOutcome::Fresh);
    for (name, other) in [("restored", restored()), ("failed", failed())] {
        let shot = render(other);
        for band in [(0, 60), (60, 140), (140, 300), (300, H)] {
            println!(
                "{name}: y {:>4}..{:<4} diff={:<8} down={}",
                band.0,
                band.1,
                diff_in_band(&fresh, &shot, band.0, band.1),
                down_pixels(&shot, band.0, band.1),
            );
        }
    }
    println!(
        "fresh down in band = {}",
        down_pixels(&fresh, BAND_TOP, BAND_BOTTOM)
    );
}
