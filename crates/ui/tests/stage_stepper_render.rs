//! advisor-calibrate-stage (R3-3a / ADR-0083) — render-layer proof of the
//! DATA → CALIBRATE → ANALYZE → SUGGEST spine stepper.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! CLAUDE.md non-negotiable + MEMORY.md "verify UI at the render layer": a
//! passing `stage_for` unit test, a text `.snap`, or a no-panic boot is NOT
//! proof the stepper band draws with the correct stage highlighted. That trap
//! shipped multiple blind cockpit bugs (the Live-view saga, the trail 0-px
//! side-drawer, the Reports empty-curve). This guard renders the REAL
//! `shell::view` HEADLESS (the full shell, via `program_from_cockpit`) and
//! asserts on the rendered PIXELS that:
//!
//! 1. [`stepper_highlights_current_stage`] — on `Screen::Tune` the CALIBRATE
//!    segment paints its SOLID `ACCENT` highlight in the stepper band (the 2nd
//!    of four segments — left-of-centre), and the band is present.
//! 2. [`stepper_highlight_moves_with_screen`] — the NEGATIVE CONTROL: on
//!    `Screen::ForwardPlan` the SUGGEST segment (the 4th / right-most) is
//!    highlighted, so the ACCENT teal's horizontal centre-of-mass shifts RIGHT
//!    versus the Tune frame. Proves guard 1 is not a tautology (the highlight
//!    genuinely tracks the screen, not a constant band).
//! 3. [`stepper_data_analyze_discriminator_moves_on_leaderboard`] — the
//!    DATA/ANALYZE discriminator: on `Screen::Leaderboard` with
//!    `PanelState::Empty` the highlight (DATA, 1st segment) sits LEFT of where
//!    it sits with a `Ready` result (ANALYZE, 3rd segment) — both on the SAME
//!    screen, driven only by the existing leaderboard substate.
//! 4. [`stepper_absent_off_journey`] — off the advisor journey (`Screen::Lab`)
//!    the band is elided: ~no stepper-band ACCENT teal.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file
//! compiles to nothing on Linux/Windows. Pixel thresholds are deliberately
//! coarse (presence/absence + a left-vs-right centre-of-mass shift, not
//! byte-exact), robust within macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState, Screen};
use ui::test_support::program_from_cockpit;

/// Render the FULL cockpit shell at the `typical` 1920×1080 slot and return the
/// physical-pixel RGBA buffer + dimensions. The shell puts the spine stepper at
/// the top of the centre column (right of the 180px sidebar), so the band lives
/// in the [`STEPPER_TOP`, `STEPPER_BOTTOM`) × [`CENTRE_LEFT`, w) region.
fn render_shell_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
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
// The stepper band is the FIRST child of the shell `centre` Column (above the
// screen body + status bar), so it occupies the top strip to the RIGHT of the
// sidebar. The band is one compact row (text::SMALL text + XS/S padding) → it
// fits comfortably inside the first ~60 px. Scoping the ACCENT scan to this
// strip keeps every screen's own ACCENT chrome (the leaderboard guided-input
// chips, the ForwardPlan stance badge, the Run buttons) OUT of the count —
// those all sit BELOW `STEPPER_BOTTOM`.

/// Top of the stepper band (the very top of the centre column).
const STEPPER_TOP: u32 = 0;
/// Bottom of the stepper band — one compact row + its padding fits under here.
const STEPPER_BOTTOM: u32 = 60;
/// Left edge of the centre column (just right of the 180 px sidebar). The
/// sidebar can carry its own ACCENT (an active row's left-rule), so the band
/// scan starts right of it.
const CENTRE_LEFT: u32 = 190;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — the exact
/// predicate the leaderboard + Reports render guards use. Green & blue high and
/// close, red clearly lower.
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// Count `ACCENT`-teal pixels in the stepper band (the top strip right of the
/// sidebar). On an advisor-journey screen the only band-region ACCENT source is
/// the active segment's SOLID fill.
fn stepper_teal(w: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    for y in STEPPER_TOP..STEPPER_BOTTOM {
        for x in CENTRE_LEFT..w {
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

/// The horizontal centre-of-mass (mean x, in physical pixels) of the
/// `ACCENT`-teal pixels in the stepper band. `None` if there is no measurable
/// teal (the band is elided / no highlight).
///
/// The four segments render left-to-right (DATA · CALIBRATE · ANALYZE ·
/// SUGGEST) as a compact left-aligned row, so the active segment's teal
/// centre-of-mass moves monotonically RIGHT (in absolute x) as the highlighted
/// stage advances. This absolute-pixel centroid is the anti-tautology signal:
/// the highlight genuinely tracks the screen (a constant / mislocated band
/// would not move). We compare centroids RELATIVELY between frames rather than
/// against a full-width assumption — the band hugs the left of the wide centre
/// column (measured: segments span x≈190–525 in the 1920-wide slot), so an
/// absolute-half threshold would be wrong; the left-to-right ORDER is the
/// invariant that matters.
fn stepper_teal_centroid_x(w: u32, rgba: &[u8]) -> Option<f64> {
    let mut sum_x = 0f64;
    let mut count = 0f64;
    for y in STEPPER_TOP..STEPPER_BOTTOM {
        for x in CENTRE_LEFT..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_accent_teal(r, g, b) {
                sum_x += f64::from(x);
                count += 1.0;
            }
        }
    }
    if count < 1.0 {
        return None;
    }
    Some(sum_x / count)
}

/// Save the operator-facing PNG (memory: verify UI at the render layer).
fn save_png(w: u32, h: u32, rgba: &[u8], path: &str) {
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
        let _ = img.save(path);
    }
}

/// **T11a — the positive guard.** On `Screen::Tune` the stepper band paints the
/// CALIBRATE segment's SOLID `ACCENT` highlight (the 2nd of four segments,
/// left-of-centre). Writes `/tmp/stage_stepper_calibrate.png`.
#[test]
fn stepper_highlights_current_stage() {
    // A real Tune state (SMA sweep at its default grid, no result yet).
    let cockpit = ui::fixtures::fake_cockpit_tune(PanelState::Empty);
    assert_eq!(cockpit.current_screen, Screen::Tune, "fixture is on Tune");

    let (w, h, rgba) = render_shell_rgba(cockpit);
    save_png(w, h, &rgba, "/tmp/stage_stepper_calibrate.png");

    let teal = stepper_teal(w, &rgba);
    assert!(
        teal > 200,
        "the CALIBRATE segment's SOLID ACCENT highlight must paint in the \
         stepper band (expected >200 teal px in the top strip, got {teal}). If \
         this fails the stepper band did not render its highlight. \
         PNG: /tmp/stage_stepper_calibrate.png"
    );

    // A highlighted band has a measurable teal centroid (the segment drew as a
    // solid fill, not a hairline). The horizontal position is proven to TRACK
    // the stage in `stepper_highlight_moves_with_screen` (the negative control)
    // and the DATA/ANALYZE discriminator test — here we just assert the
    // highlight painted.
    let _centroid =
        stepper_teal_centroid_x(w, &rgba).expect("a highlighted band has a teal centroid");
}

/// **T11b — the negative control.** The SAME harness on `Screen::ForwardPlan`
/// highlights the SUGGEST segment (the 4th / right-most), so the ACCENT teal's
/// centre-of-mass shifts RIGHT versus the Tune frame — and lands in the right
/// half. Proves T11a is not a tautology (a constant band would not move). Writes
/// `/tmp/stage_stepper_suggest.png`.
#[test]
fn stepper_highlight_moves_with_screen() {
    // Tune frame (CALIBRATE highlighted — 2nd segment).
    let tune = ui::fixtures::fake_cockpit_tune(PanelState::Empty);
    let (wt, ht, rt) = render_shell_rgba(tune);
    save_png(wt, ht, &rt, "/tmp/stage_stepper_calibrate.png");
    let calibrate_x =
        stepper_teal_centroid_x(wt, &rt).expect("Tune frame has a CALIBRATE highlight");

    // ForwardPlan frame (SUGGEST highlighted — 4th / right-most segment).
    let forward = ui::fixtures::fake_cockpit_forward_plan(PanelState::Empty);
    assert_eq!(
        forward.current_screen,
        Screen::ForwardPlan,
        "fixture is on ForwardPlan"
    );
    let (wf, hf, rf) = render_shell_rgba(forward);
    save_png(wf, hf, &rf, "/tmp/stage_stepper_suggest.png");

    let suggest_teal = stepper_teal(wf, &rf);
    assert!(
        suggest_teal > 200,
        "the SUGGEST segment's SOLID ACCENT highlight must paint in the stepper \
         band on ForwardPlan (expected >200 teal px, got {suggest_teal}). \
         PNG: /tmp/stage_stepper_suggest.png"
    );

    let suggest_x =
        stepper_teal_centroid_x(wf, &rf).expect("ForwardPlan frame has a SUGGEST highlight");

    // The load-bearing anti-tautology: SUGGEST (4th segment) is TWO segments to
    // the RIGHT of CALIBRATE (2nd segment), so its teal centroid must sit well
    // right of the CALIBRATE frame's. A constant / mislocated band fails here.
    // The margin (>80 px in the 1920-wide slot) is ~2 segment widths — coarse
    // enough to be robust to font jitter, strict enough that a non-moving band
    // fails.
    assert!(
        suggest_x > calibrate_x + 80.0,
        "the highlight must move RIGHT from CALIBRATE (Tune, 2nd segment) to \
         SUGGEST (ForwardPlan, 4th segment) — proof it tracks the screen, not a \
         constant band (CALIBRATE centroid x={calibrate_x:.1} vs SUGGEST \
         centroid x={suggest_x:.1}). PNGs: /tmp/stage_stepper_calibrate.png, \
         /tmp/stage_stepper_suggest.png"
    );
}

/// **T11c — the DATA/ANALYZE discriminator (the crux IA finding).** DATA and
/// ANALYZE share `Screen::Leaderboard`; the highlight is resolved by the
/// EXISTING leaderboard result substate. With `PanelState::Empty` the highlight
/// is DATA (1st segment, far left); with a `Ready` bake-off result it is ANALYZE
/// (3rd segment, right-of-centre) — SAME screen, highlight moves RIGHT. Writes
/// both PNGs.
#[test]
fn stepper_data_analyze_discriminator_moves_on_leaderboard() {
    // Leaderboard + Empty → DATA (far-left segment).
    let empty = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);
    assert_eq!(empty.current_screen, Screen::Leaderboard);
    let (we, he, re) = render_shell_rgba(empty);
    save_png(we, he, &re, "/tmp/stage_stepper_data.png");

    let data_teal = stepper_teal(we, &re);
    assert!(
        data_teal > 200,
        "the DATA segment's SOLID ACCENT highlight must paint in the stepper \
         band on Leaderboard+Empty (expected >200 teal px, got {data_teal}). \
         PNG: /tmp/stage_stepper_data.png"
    );
    let data_x = stepper_teal_centroid_x(we, &re).expect("Leaderboard+Empty has a DATA highlight");

    // Leaderboard + Ready → ANALYZE (3rd segment, two segments right of DATA).
    let ready = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror(),
    ));
    let (wr, hr, rr) = render_shell_rgba(ready);
    save_png(wr, hr, &rr, "/tmp/stage_stepper_analyze.png");

    let analyze_teal = stepper_teal(wr, &rr);
    assert!(
        analyze_teal > 200,
        "the ANALYZE segment's SOLID ACCENT highlight must paint in the stepper \
         band on Leaderboard+Ready (expected >200 teal px, got {analyze_teal}). \
         PNG: /tmp/stage_stepper_analyze.png"
    );
    let analyze_x =
        stepper_teal_centroid_x(wr, &rr).expect("Leaderboard+Ready has an ANALYZE highlight");

    // DATA (1st) is far-left; ANALYZE (3rd) is two segments to its RIGHT. The
    // highlight moves RIGHT purely from the substate flip — SAME screen, no new
    // state field, no navigation. This is the load-bearing D2 proof at the pixel
    // layer (the crux IA finding: DATA + ANALYZE share Screen::Leaderboard). The
    // >80 px margin is ~2 segment widths in the 1920-wide slot.
    assert!(
        analyze_x > data_x + 80.0,
        "the highlight must move RIGHT from DATA (Leaderboard+Empty) to ANALYZE \
         (Leaderboard+Ready) — driven ONLY by the existing result substate, on \
         the SAME screen (DATA centroid x={data_x:.1} vs ANALYZE centroid \
         x={analyze_x:.1}). PNGs: /tmp/stage_stepper_data.png, \
         /tmp/stage_stepper_analyze.png"
    );
}

/// **T11d — off-journey elision.** On a non-advisor screen (`Screen::Lab`) the
/// stepper band is elided (`stage_for` → `None`), so the top strip carries ~no
/// stepper ACCENT teal. Proves the band is pixel-silent where it should be — and
/// is the negative control that the band-region teal scan is not measuring
/// unrelated chrome. Writes `/tmp/stage_stepper_off_journey.png`.
#[test]
fn stepper_absent_off_journey() {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = Screen::Lab;
    let (w, h, rgba) = render_shell_rgba(cockpit);
    save_png(w, h, &rgba, "/tmp/stage_stepper_off_journey.png");

    let teal = stepper_teal(w, &rgba);
    assert!(
        teal < 150,
        "off the advisor journey (Lab) the stepper band must be elided — the top \
         strip must carry ~no stepper ACCENT teal (expected <150 stray px, got \
         {teal}). If high, the band did not elide OR the scan is catching Lab \
         chrome. PNG: /tmp/stage_stepper_off_journey.png"
    );
}
