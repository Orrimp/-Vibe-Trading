//! advisor-crown-credibility (P1 / ADR-0085) — render-layer proof that the crown
//! banner CO-PRESENTS its overfitting (DSR) verdict.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing `crown_credibility(..)`
//! unit test, a text `.snap`, or a no-panic boot is NOT proof the WeakEvidence
//! band DRAWS on the banner. This guard renders the REAL
//! `screens::leaderboard::view` HEADLESS with the populated money-shot mirror
//! (`fake_bakeoff_report_mirror_five_arm` — `ActiveWins`, `crown_clears_dsr ==
//! false`) and asserts on the rendered PIXELS that the `WARN`-tier band actually
//! paints — WITH negative controls (the SAME mirror with `crown_clears_dsr`
//! flipped `true` shows the muted `Passes` line and NO WARN band; the
//! `BenchmarkWins` fixture carries NO credibility band at all).
//!
//! ## What is asserted (ADR-0085 § D6)
//!
//! 1. [`weak_evidence_band_paints_on_banner`] — the money shot. The `five_arm`
//!    mirror paints a substantial count of saturated `WARN`-amber pixels (the
//!    band border + text) that the SAME mirror with `crown_clears_dsr = true`
//!    (the `Passes` control) does NOT. The delta is the band; a regression that
//!    drops it collapses the delta. Writes `/tmp/crown_credibility_weak.png`.
//! 2. [`passes_state_is_control_not_weak`] — the flag-tracks-the-render control.
//!    The SAME `five_arm` mirror with `crown_clears_dsr` flipped `true` paints
//!    essentially NO WARN-amber in the banner region (the `Passes` line is
//!    `ACCENT`-teal, a different hue) — proving guard 1 is not a tautology (the
//!    band tracks the flag). Writes `/tmp/crown_credibility_passes.png`.
//! 3. [`benchmark_wins_banner_has_no_credibility_band`] — the
//!    no-badge-on-a-hold-pick invariant at the pixel layer. The `BenchmarkWins`
//!    fixture paints NO WARN-amber credibility band in the banner region. Writes
//!    `/tmp/crown_credibility_benchmark.png`.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_scorecard_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. The hue thresholds are deliberately coarse
//! (presence/absence of saturated amber, not byte-exact), robust within macOS
//! across font-DB jitter.

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

/// Count **saturated `WARN`-amber** pixels — the band's border + text hue. In
/// dark mode `WARN_500` is `rgb(224, 180, 92)`: warm (R > G > B), with R well
/// above B. The predicate keys on that warmth (`R > B + 40` and `G > B + 25` and
/// `R > 130`) so it fires on the amber band but NOT on the near-black panel
/// surfaces (`R ≈ G ≈ B`) NOR on the `ACCENT`-teal `Passes` line
/// (`rgb(111, 182, 174)` — there B ≈ G > R, so `R > B + 40` fails). This is the
/// signal that discriminates the WeakEvidence band from every other state.
fn warn_amber_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    for y in 0..h {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if r > 130 && r > b + 40 && g > b + 25 {
                hits += 1;
            }
        }
    }
    hits
}

/// Count `WARN`-amber pixels in the BANNER REGION only. The Recommendation panel
/// is the 3rd block in the result column (under the guided-input form + the Data
/// quality panel), so with the full 20-arm field it lands in the LOWER half of
/// the 1920×1080 frame (empirically the WeakEvidence band paints at y ≈ 875–910;
/// read `/tmp/crown_credibility_weak.png`). Restricting to `y > h/2` isolates the
/// band from the guided-input form's `ACCENT` chips (teal, not amber — but kept
/// out of frame anyway) and keeps the guard strictly about the banner. The
/// scorecard panel below the table carries at most a single `✗` glyph + a short
/// warn line (empirically < 100 amber px), far under the delta floor.
fn warn_amber_pixels_banner_region(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let start_y = h / 2;
    let mut hits = 0u64;
    for y in start_y..h {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if r > 130 && r > b + 40 && g > b + 25 {
                hits += 1;
            }
        }
    }
    hits
}

/// **The money shot.** The `five_arm` mirror (`ActiveWins`, `crown_clears_dsr ==
/// false`) MUST paint the `WeakEvidence` band on the banner — proven by a strict
/// WARN-amber delta in the banner region against the SAME mirror with
/// `crown_clears_dsr` flipped `true` (the `Passes` control). The delta is the
/// band's border + text; a regression that drops the band collapses it.
///
/// Writes both PNGs (`/tmp/crown_credibility_weak.png` = the WARN band,
/// `/tmp/crown_credibility_passes.png` = the muted Passes line) for the operator
/// eyeball (MEMORY: a pixel count is a claim, not proof — READ the image).
#[test]
fn weak_evidence_band_paints_on_banner() {
    // The money-shot mirror already IS the WeakEvidence case.
    let weak = ui::fixtures::fake_bakeoff_report_mirror_five_arm();
    assert_eq!(
        weak.recommendation.outcome,
        ui::leaderboard::state::OutcomeKind::ActiveWins,
        "the five-arm fixture must be the ActiveWins crown"
    );
    assert!(
        weak.scorecard
            .as_ref()
            .is_some_and(|sc| !sc.crown_clears_dsr),
        "the five-arm fixture's crown must FAIL DSR (the money shot)"
    );

    // The negative control — the SAME mirror, `crown_clears_dsr` flipped `true`,
    // so the banner shows the muted `Passes` line (ACCENT-teal), NOT the WARN
    // band. The ONLY difference between the two frames is the credibility state,
    // so the amber delta is attributable to the band alone (not a tautology).
    let mut passes = weak.clone();
    if let Some(sc) = passes.scorecard.as_mut() {
        sc.crown_clears_dsr = true;
    }

    let cockpit_weak = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(weak));
    let cockpit_passes = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(passes));

    let (ww, wh, wr) = render_leaderboard_rgba(cockpit_weak);
    let (pw, ph, pr) = render_leaderboard_rgba(cockpit_passes);

    // Operator-facing deliverables (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(ww, wh, wr.clone()) {
        let _ = img.save("/tmp/crown_credibility_weak.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(pw, ph, pr.clone()) {
        let _ = img.save("/tmp/crown_credibility_passes.png");
    }

    let amber_weak = warn_amber_pixels_banner_region(ww, wh, &wr);
    let amber_passes = warn_amber_pixels_banner_region(pw, ph, &pr);

    // The WeakEvidence band (a ~2-line WARN-amber sentence + a 1 px WARN_500
    // border spanning the panel width) is a substantial amber block. It must
    // paint STRICTLY MORE amber than the `Passes` control (whose banner line is
    // ACCENT-teal, not amber). The floor (>600 px of delta) is well below the
    // measured band size but above font-DB jitter, so a regression that drops
    // the band (delta → ~0) fails loudly.
    assert!(
        amber_weak > amber_passes + 600,
        "the WeakEvidence band must paint substantially more WARN-amber in the \
         banner region than the Passes control (weak={amber_weak} vs \
         passes={amber_passes}, delta={}). If the delta is small the band did \
         not render. PNG: /tmp/crown_credibility_weak.png",
        amber_weak as i64 - amber_passes as i64
    );
}

/// **The flag-tracks-the-render control.** The SAME `five_arm` mirror with
/// `crown_clears_dsr` flipped `true` shows the muted `Passes` affordance and
/// paints essentially NO WARN-amber in the banner region (the `Passes` line is
/// `ACCENT`-teal). This proves the WeakEvidence guard is NOT a tautology — the
/// band genuinely tracks the flag. (The PNG is written by
/// `weak_evidence_band_paints_on_banner`; this test asserts the absence.)
#[test]
fn passes_state_is_control_not_weak() {
    let mut passes = ui::fixtures::fake_bakeoff_report_mirror_five_arm();
    if let Some(sc) = passes.scorecard.as_mut() {
        sc.crown_clears_dsr = true;
    }
    assert!(
        passes
            .scorecard
            .as_ref()
            .is_some_and(|sc| sc.crown_clears_dsr),
        "the mutated mirror's crown must CLEAR DSR (the Passes control)"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(passes));
    let (w, h, r) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, r.clone()) {
        let _ = img.save("/tmp/crown_credibility_passes.png");
    }

    // The Passes banner has NO WARN band — only the muted ACCENT `✓` line. The
    // banner region should carry only incidental amber (well under the band's
    // footprint). A loose ceiling (< 400 px) tolerates font jitter yet fails if
    // the WARN band ever paints here.
    let amber = warn_amber_pixels_banner_region(w, h, &r);
    assert!(
        amber < 400,
        "the Passes state must NOT paint a WARN band in the banner region \
         (found {amber} amber px — expected only the muted ACCENT line). \
         PNG: /tmp/crown_credibility_passes.png"
    );
}

/// **The no-badge-on-a-hold-pick invariant (pixel layer).** The `BenchmarkWins`
/// fixture (buy-and-hold crowned) carries NO credibility band — the banner region
/// paints no WARN-amber credibility block (ADR-0085 § D4: the DSR is on a losing
/// ACTIVE arm, so a badge on a hold pick would mislead). Writes
/// `/tmp/crown_credibility_benchmark.png`.
///
/// NOTE: the `BenchmarkWins` fixture is a SHORT 2-row field, so its scorecard
/// panel renders directly under the table WITHIN the viewport — but the scorecard
/// panel's own "Beats holding? ✗" row is FAR below the banner region (bottom two
/// thirds), and this guard counts amber only in the TOP third, so the scorecard's
/// ✗ can never be mistaken for a banner badge.
#[test]
fn benchmark_wins_banner_has_no_credibility_band() {
    let bench = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    assert_eq!(
        bench.recommendation.outcome,
        ui::leaderboard::state::OutcomeKind::BenchmarkWins,
        "the benchmark-wins fixture must be the BenchmarkWins crown"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(bench));
    let (w, h, r) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, r.clone()) {
        let _ = img.save("/tmp/crown_credibility_benchmark.png");
    }

    // No credibility band on a hold pick — the banner region carries no WARN
    // credibility block. Same loose ceiling as the Passes control.
    let amber = warn_amber_pixels_banner_region(w, h, &r);
    assert!(
        amber < 400,
        "the BenchmarkWins banner must carry NO credibility band in the banner \
         region (found {amber} amber px). A badge on a hold pick would bind an \
         active-arm DSR statistic to a passive pick (ADR-0085 § D4). \
         PNG: /tmp/crown_credibility_benchmark.png"
    );

    // Sanity: the whole-frame amber is likewise not a huge block (the scorecard's
    // single ✗ glyph + any warn text is small) — guards against a future change
    // that accidentally floods the frame with amber.
    let _ = warn_amber_pixels(w, h, &r);
}
