//! advisor-llm-narration F9 (ADR-0064) — render-layer proof of the opt-in LLM
//! "why this one" NARRATION on the cockpit leaderboard recommendation block.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the narration draws. That trap
//! shipped multiple blind cockpit bugs (the Live-view saga, the trail 0-px
//! side-drawer, the Reports empty-curve). This guard renders the REAL
//! `screens::leaderboard::view` HEADLESS with a populated `BakeoffReportMirror`
//! in each of the four F9 `NarrationState`s and asserts on the rendered PIXELS
//! that:
//!
//! - the `Ready` state PAINTS the LLM prose card (the faithful fixture narration
//!   shows in a subtly-distinct AI-summary card — the `ACCENT` label/border +
//!   the long prose body), and
//! - the `FellBack` / `NotRequested` states are the NEGATIVE CONTROL — the
//!   templated reasons paint instead (far less foreground in the recommendation
//!   band, no AI-summary card accent), the honest floor, with the disclaimer
//!   present in every state.
//!
//! The narration crosses as the plain `ui`-owned `NarrationState` (String/enum
//! only — no `llm`/`agent` type through `view`); the render uses a canned `ui`
//! FIXTURE narration (`ui::fixtures::FAKE_NARRATION_READY_PROSE`) — NO agent, NO
//! `llm`, NO network (ADR-0064 § D5 fake-seam, the render-harness reservation).
//!
//! Operator-facing PNGs (read these):
//!   - `Ready`     → `/tmp/forward_f9_narration_ready_render.png`
//!   - `FellBack`  → `/tmp/forward_f9_narration_fallback_render.png`
//!   - `NotRequested` (the Explain control) → `/tmp/forward_f9_narration_not_requested_render.png`
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse
//! (presence/absence of a hue + a foreground delta, not byte-exact), robust
//! within macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::leaderboard::NarrationState;
use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

/// Render the whole PANE, not the fold.
///
/// These gates ask "did this block paint?", which is a question about the pane. Whether a
/// block clears the operator's 1080-px fold is asserted once, and only once, in
/// `leaderboard_scorecard_render` (bug-log #96 / ADR-0092). When ADR-0092 reordered the
/// pane, every band scan in a 1080-px frame started measuring an empty strip below the
/// clip — 13 gates failed the same day with "got 0". A frame taller than the pane cannot
/// clip, so these scans stay about content.
const PANE_HEIGHT: u32 = 2400;
/// Render the bare Leaderboard screen body at the `typical` 1920×1080 slot and
/// return the physical-pixel RGBA buffer + dimensions.
fn render_leaderboard_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot =
        iced_test::screenshot(&program, &theme, (1920, PANE_HEIGHT), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

// ── Region bands ──────────────────────────────────────────────────────────────
//
// The screen stacks header / guided-input FORM / budget context / result body.
// The result body's FIRST element is the RECOMMENDATION block (the `frame::panel`
// titled "Recommendation" holding the headline + clause + the F9 narration
// section), and the ranked TABLE is BELOW it.
//
// The F9 narration section lives INSIDE the recommendation block, near the top
// of the result body. The `Ready` state paints the AI-summary card there (an
// `ACCENT`-bordered, `ACCENT_SOFT`-tinted card with an `ACCENT` label + the long
// prose body); the `FellBack`/`NotRequested` states paint the two templated
// reason lines there instead (far less foreground, no AI-summary-card accent).
//
// We scope the RECOMMENDATION band to the top slice of the result body. The
// crowned-row table ACCENT + the Max-DD clay both live BELOW this band, so a
// foreground/accent delta scoped here tracks the narration section, not the
// table.
//
// ── advisor-bakeoff tuning-knobs re-calibration ─────────────────────────────
// The "Plan your bake-off" form grew a tuning row (the H1/H4/D1 timeframe chips
// + the "Start capital" field + its honest hint), which pushed the whole result
// body — and with it the recommendation block — DOWN. Re-measured from the saved
// PNGs (`/tmp/forward_f9_narration_*_render.png`): the AI-summary card's ACCENT
// top border now lands at y≈572, its ACCENT label at y≈589, and the table's
// crowned-row accent does not begin until y≈718. The band below brackets the
// recommendation card with margin and stops at 715 (above the table crown), so
// `Ready` paints heavy ACCENT here while `FellBack` paints ~none — the
// discriminator is preserved at the new position.

/// Top of the RECOMMENDATION band — where the recommendation `frame::panel` starts.
///
/// Re-measured 2026-09-24 after story 3-20 added three lines to the scorecard block
/// (ADR-0095). Two things changed and both are recorded here rather than left for the
/// next reader to rediscover:
///
/// 1. The block grew by 72 px (358 -> 430), so everything below it moved down.
/// 2. **The block's height is no longer constant across fixtures.** 3-20's arm-inventory
///    line lists the requested arms and WRAPS, so a wide field pushes the recommendation
///    further down than a narrow one. The previous doc comment's assumption — "everything
///    above the block is fixed-height in every fixture, so the band does not move with the
///    field size" — is false as of ADR-0095, and this band is sized for the SPREAD.
///
/// Re-measured again 2026-09-25, after story 4-13 added the cross-run check to the same
/// trust block: it grew a further 46 px (the fold gate's measured extent went 903 -> 949),
/// so everything below moved down by that much. The gates did NOT go red, but measuring
/// showed the old bottom bound had become a tripwire rather than a bracket — the
/// AI-summary card's bottom accent border (y = 1196) and the Explain button's (y = 1191)
/// had both fallen OUTSIDE a band that ends at 1190, and the Explain button's LABEL had
/// only 7 px of slack left. So the bottom moved 1190 -> 1230, on measurement: that is
/// below every feature the band is meant to bracket and still 23 px clear of the ranked
/// table's crowned-row accent (y >= 1253), whose clay the amber predicate also matches.
///
/// Measured by the `measure` test in each file, full-pane renders at 1920 x 2400:
///
/// | fixture | what paints | where |
/// |---|---|---|
/// | `five_arm` (5 arms) | WeakEvidence banner, 6186 amber px in band | y = 1023..1063 |
/// | narration `Ready` | AI-summary card, ACCENT (2579 px in band) | y = 1119..1196 |
/// | narration `NotRequested` | Explain ghost button, ACCENT (372 px in band) | y = 1173..1191 |
/// | narration `FellBack` | neither — 0 ACCENT in band (the control) | next teal at y = 1263 |
/// | `benchmark_wins` | stray amber only, 17 px in band, no dense run | — |
/// | any | ranked table's crowned-row accent | y >= 1253 |
///
/// The two control CEILINGS are what a bottom bound set too low would break by letting
/// the table in: `passes` measures 0 amber in band (229 in the whole frame) and
/// `benchmark_wins` 17 (138 whole-frame), both far under their `< 400` ceilings even if
/// every stray pixel fell inside.
///
/// So the band spans the recommendation region across every fixture and stops short of
/// the table, whose clay the colour predicates would otherwise match.
const REC_TOP: u32 = 950;
/// Bottom of the RECOMMENDATION band — below the AI-summary card, above the table.
const REC_BOTTOM: u32 = 1230;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — green & blue
/// high and close, red clearly lower (the exact predicate the leaderboard +
/// forward-plan curve guards use for #6FB6AE). The AI-summary card's `ACCENT`
/// label + 1 px border paint this hue.
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

/// Foreground in the RECOMMENDATION band — tracks how much the narration section
/// drew (the long LLM prose in `Ready` vs the two short templated reason lines
/// in `FellBack`/`NotRequested`).
fn recommendation_band_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, REC_TOP, REC_BOTTOM)
}

/// The AI-summary card's `ACCENT` teal — scoped to the RECOMMENDATION band. On
/// `Ready` this is the card's `ACCENT` label + 1 px border (non-trivial); on
/// `FellBack` (templated copy, no card) it is ~zero.
fn summary_card_accent(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, REC_TOP, REC_BOTTOM)
}

/// Render the populated leaderboard (5-arm field, `v0.sma` crowned, `ActiveWins`)
/// in the given F9 narration state and save the operator-facing PNG.
fn render_with_narration(narration: NarrationState, png_path: &str) -> (u32, u32, Vec<u8>) {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    let cockpit =
        ui::fixtures::fake_cockpit_leaderboard_with_narration(PanelState::Ready(mirror), narration);
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save(png_path);
    }
    (w, h, rgba)
}

/// **The F9 `Ready` render-layer guard.** A faithful fixture narration in
/// `NarrationState::Ready` MUST paint, in the cockpit Leaderboard recommendation
/// block:
/// - the AI-summary card's `ACCENT` teal (the label + the 1 px accent border) in
///   the RECOMMENDATION band — proof the labelled prose card actually drew;
/// - a healthy amount of foreground text in the RECOMMENDATION band (the long
///   plain-language prose, far more than the two short templated reason lines);
/// - the disclaimer + the rest of the screen (a lot of whole-screen foreground).
///
/// Writes the operator-facing PNG to `/tmp/forward_f9_narration_ready_render.png`.
#[test]
fn narration_ready_paints_llm_prose_card() {
    // The fixture prose is the canned faithful narration — names v0.sma, states
    // ActiveWins, uses only KPIs in the table, trips no banned phrase (the
    // ADR § D5 FaithfulFakeProvider analogue, but `ui`-side, no agent/network).
    let prose = ui::fixtures::FAKE_NARRATION_READY_PROSE;
    assert!(
        prose.len() > 200,
        "the fixture narration must be a substantial paragraph (the whole point \
         of F9 is fluent prose) — got {} chars",
        prose.len()
    );

    let (w, h, rgba) = render_with_narration(
        NarrationState::Ready(smol_str::SmolStr::new(prose)),
        "/tmp/forward_f9_narration_ready_render.png",
    );

    let card_accent = summary_card_accent(w, &rgba);
    let rec_fg = recommendation_band_foreground(w, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The AI-summary card's ACCENT label + 1 px border paint teal in the
    // recommendation band — proof the labelled prose card rendered (not bare
    // text, not the templated reasons).
    assert!(
        card_accent > 120,
        "the AI-summary card's ACCENT label + border must paint in the \
         RECOMMENDATION band (expected >120 teal px, got {card_accent}). If this \
         fails the labelled LLM-prose card did not render. \
         PNG: /tmp/forward_f9_narration_ready_render.png"
    );
    // The long prose is a lot of foreground in the recommendation band.
    assert!(
        rec_fg > 2500,
        "the LLM prose must paint a lot of foreground in the RECOMMENDATION band \
         (expected >2500 px, got {rec_fg}). If low, the prose did not render. \
         PNG: /tmp/forward_f9_narration_ready_render.png"
    );
    // The whole screen (recommendation + table + disclaimer) is a lot of text.
    assert!(
        fg > 8000,
        "the populated leaderboard + the narration must paint a lot of \
         foreground (expected >8000 px, got {fg}). \
         PNG: /tmp/forward_f9_narration_ready_render.png"
    );
}

/// **The F9 `FellBack` negative control.** The SAME populated leaderboard in
/// `NarrationState::FellBack` paints the TEMPLATED reasons (the honest floor) —
/// NOT the LLM prose. So the RECOMMENDATION band paints:
/// - ~no AI-summary-card `ACCENT` teal (there is no card), and
/// - far LESS foreground than the `Ready` state (two short reason lines + a
///   quiet one-line note, vs the long prose paragraph).
///
/// Proves the `Ready` guard is not a tautology (it genuinely discriminates the
/// painted prose from the templated fallback). Writes the operator-facing PNG to
/// `/tmp/forward_f9_narration_fallback_render.png`.
#[test]
fn narration_fallback_paints_templated_copy_not_prose() {
    let (w, _h, rgba) = render_with_narration(
        NarrationState::FellBack,
        "/tmp/forward_f9_narration_fallback_render.png",
    );

    let card_accent = summary_card_accent(w, &rgba);
    let rec_fg = recommendation_band_foreground(w, &rgba);

    // No AI-summary card in the fallback → ~no card ACCENT in the rec band.
    // (The crowned-row table accent is BELOW REC_BOTTOM, so it does not leak.)
    assert!(
        card_accent < 90,
        "the FellBack state must NOT paint an AI-summary-card ACCENT in the \
         RECOMMENDATION band (expected <90 stray teal px, got {card_accent}). If \
         high, the prose card leaked into the honest fallback. \
         PNG: /tmp/forward_f9_narration_fallback_render.png"
    );
    // The templated reasons + the quiet note are far less text than the prose,
    // but the band is NOT blank — the headline + reasons are the honest floor.
    assert!(
        rec_fg > 200,
        "the FellBack state must still paint the templated headline + reasons \
         (the honest floor — never blank; expected >200 foreground px, got \
         {rec_fg}). PNG: /tmp/forward_f9_narration_fallback_render.png"
    );
}

/// **The F9 `Ready`-vs-`FellBack` discriminator (anti-tautology).** The `Ready`
/// state paints STRICTLY MORE recommendation-band foreground + AI-summary-card
/// accent than the `FellBack` state. Ties the two render states together in one
/// assertion so a future regression that makes the prose card and the templated
/// fallback look the same FAILS — the load-bearing proof that the LLM prose is a
/// distinct, richer surface and the fallback is the honest floor.
#[test]
fn narration_ready_strictly_exceeds_fallback() {
    let ready = ui::fixtures::fake_cockpit_leaderboard_with_narration(
        PanelState::Ready(ui::fixtures::fake_bakeoff_report_mirror()),
        NarrationState::Ready(smol_str::SmolStr::new(
            ui::fixtures::FAKE_NARRATION_READY_PROSE,
        )),
    );
    let fellback = ui::fixtures::fake_cockpit_leaderboard_with_narration(
        PanelState::Ready(ui::fixtures::fake_bakeoff_report_mirror()),
        NarrationState::FellBack,
    );

    let (wr, _hr, rr) = render_leaderboard_rgba(ready);
    let (wf, _hf, rf) = render_leaderboard_rgba(fellback);

    let rec_fg_ready = recommendation_band_foreground(wr, &rr);
    let rec_fg_fell = recommendation_band_foreground(wf, &rf);
    let accent_ready = summary_card_accent(wr, &rr);
    let accent_fell = summary_card_accent(wf, &rf);

    // The prose paragraph is much more text than the two templated reason lines.
    assert!(
        rec_fg_ready > rec_fg_fell + 1500,
        "the Ready prose must paint strictly more recommendation-band foreground \
         than the FellBack templated copy (Ready {rec_fg_ready} vs FellBack \
         {rec_fg_fell}). If they are close the prose card is not discriminating."
    );
    // The AI-summary card adds its ACCENT label + border that the fallback lacks.
    assert!(
        accent_ready > accent_fell + 80,
        "the Ready AI-summary card must paint strictly more recommendation-band \
         ACCENT than the FellBack fallback (Ready {accent_ready} vs FellBack \
         {accent_fell})."
    );
}

/// **The F9 `NotRequested` state — the Explain control paints.** The default
/// `NarrationState::NotRequested` paints the templated reasons + the opt-in
/// Explain control (an `ACCENT`-bordered ghost button). So the RECOMMENDATION
/// band paints SOME `ACCENT` teal (the Explain button's accent border + label)
/// — proof the opt-in trigger rendered — but FAR LESS recommendation-band
/// foreground than the `Ready` prose (no prose paragraph). Writes the
/// operator-facing PNG to `/tmp/forward_f9_narration_not_requested_render.png`.
#[test]
fn narration_not_requested_paints_explain_control() {
    let (w, _h, rgba) = render_with_narration(
        NarrationState::NotRequested,
        "/tmp/forward_f9_narration_not_requested_render.png",
    );

    // The Explain ghost button paints its ACCENT border + label in the rec band.
    let button_accent = summary_card_accent(w, &rgba);
    assert!(
        button_accent > 80,
        "the Explain control (an ACCENT-bordered ghost button) must paint in the \
         RECOMMENDATION band (expected >80 teal px, got {button_accent}). If this \
         fails the opt-in Explain trigger did not render. \
         PNG: /tmp/forward_f9_narration_not_requested_render.png"
    );
    // The templated reasons + the control are present (the honest floor — never
    // blank).
    let rec_fg = recommendation_band_foreground(w, &rgba);
    assert!(
        rec_fg > 200,
        "the NotRequested state must paint the templated headline + reasons + the \
         Explain control (expected >200 foreground px, got {rec_fg}). \
         PNG: /tmp/forward_f9_narration_not_requested_render.png"
    );
}

/// Prints the geometry table in [`REC_TOP`]'s doc comment — and the identical one in
/// `crown_credibility_render`'s `BANNER_TOP`. Run it after ANY change to the pane
/// above the recommendation block:
///
/// ```text
/// cargo test -p ui --test leaderboard_narration_render -- --ignored --nocapture measure
/// ```
///
/// It exists because both bands are anchored to MEASURED positions, and the honest way
/// to move one is to re-measure, never to widen it until the test passes (the #77
/// failure both files are written against). It reports, per fixture, the contiguous
/// row-runs in which the discriminating colour paints, so the band can be checked to
/// still bracket the recommendation region and still stop short of the ranked table —
/// whose `DOWN_500` clay the amber predicate also matches.
#[test]
#[ignore = "measurement, not a gate"]
fn measure() {
    /// Merge rows carrying `>= 8` hits into contiguous runs, tolerating 3-row gaps
    /// (glyph interiors), and report runs of at least 20 px.
    fn runs(w: u32, h: u32, rgba: &[u8], hit: fn(i32, i32, i32) -> bool) -> Vec<(u32, u32, u64)> {
        let per_row: Vec<u64> = (0..h)
            .map(|y| {
                (0..w)
                    .filter(|&x| {
                        let i = ((y as usize * w as usize) + x as usize) * 4;
                        hit(
                            i32::from(rgba[i]),
                            i32::from(rgba[i + 1]),
                            i32::from(rgba[i + 2]),
                        )
                    })
                    .count() as u64
            })
            .collect();
        let mut out: Vec<(u32, u32, u64)> = Vec::new();
        let mut open: Option<(u32, u32, u64)> = None;
        for (y, &n) in per_row.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let y = y as u32;
            if n >= 8 {
                open = match open {
                    Some((a, _, t)) => Some((a, y, t + n)),
                    None => Some((y, y, n)),
                };
            } else if let Some((a, b, t)) = open
                && y > b + 3
            {
                out.push((a, b, t));
                open = None;
            }
        }
        if let Some(r) = open {
            out.push(r);
        }
        out.retain(|&(_, _, t)| t >= 20);
        out
    }

    fn amber(r: i32, g: i32, b: i32) -> bool {
        r > 130 && r > b + 40 && g > b + 25
    }
    fn teal(r: i32, g: i32, b: i32) -> bool {
        is_accent_teal(r, g, b)
    }

    let five_arm = render_leaderboard_rgba(ui::fixtures::fake_cockpit_leaderboard(
        PanelState::Ready(ui::fixtures::fake_bakeoff_report_mirror_five_arm()),
    ));
    let bench = render_leaderboard_rgba(ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins(),
    )));
    let ready = render_with_narration(
        NarrationState::Ready(smol_str::SmolStr::new(
            ui::fixtures::FAKE_NARRATION_READY_PROSE,
        )),
        "/tmp/forward_f9_narration_ready_render.png",
    );
    let not_req = render_with_narration(
        NarrationState::NotRequested,
        "/tmp/forward_f9_narration_not_requested_render.png",
    );
    let fellback = render_with_narration(
        NarrationState::FellBack,
        "/tmp/forward_f9_narration_fallback_render.png",
    );

    for (label, (w, h, rgba), hit) in [
        (
            "five_arm         amber",
            &five_arm,
            amber as fn(i32, i32, i32) -> bool,
        ),
        ("five_arm          teal", &five_arm, teal),
        ("benchmark_wins   amber", &bench, amber),
        ("narration Ready   teal", &ready, teal),
        ("narration NotReq  teal", &not_req, teal),
        ("narration FellBck teal", &fellback, teal),
    ] {
        let rs = runs(*w, *h, rgba, hit);
        let shown: Vec<String> = rs
            .iter()
            .map(|(a, b, t)| format!("y={a}..{b} ({t} px)"))
            .collect();
        println!("{label}: {}", shown.join("  "));
    }
    println!("band under test: REC_TOP={REC_TOP} REC_BOTTOM={REC_BOTTOM}");
    for (label, (w, _h, rgba)) in [
        ("Ready  ", &ready),
        ("NotReq ", &not_req),
        ("FellBck", &fellback),
    ] {
        println!(
            "{label} teal in band = {}",
            accent_teal_in_band(*w, rgba, REC_TOP, REC_BOTTOM)
        );
    }
}
