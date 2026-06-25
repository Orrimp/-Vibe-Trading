//! advisor-param-tuning (ADR-0069 T9) — render-layer proof of the POPULATED
//! gate-tied sweep RESULT GRID in the cockpit Tune editor.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the Tune grid draws. That trap
//! shipped multiple blind cockpit bugs (the Live-view saga, the trail 0-px
//! side-drawer, the Reports empty-curve). This guard renders the REAL
//! `screens::tune::view` HEADLESS with a POPULATED `SweepReportMirror` fixture
//! (a mix of Robust + Marginal + ≥1 FRAGILE cell) and asserts on the rendered
//! PIXELS that the result grid + the FRAGILE badge clay + the distribution
//! columns actually paint — with a NEGATIVE CONTROL (the `Empty` "set ranges and
//! press Run sweep" prompt paints NO grid).
//!
//! Guards (populated + negative control + the FRAGILE promotion lock + progress):
//!
//! 1. [`sweep_populated_paints_grid_and_fragile_badge`] — the populated fixture
//!    paints (a) the `DOWN_500` clay of the FRAGILE verdict badge + the FRAGILE
//!    row's Max-DD-p95 column (the load-bearing honesty pixel), and (b) a healthy
//!    amount of foreground text (proves the grid + distribution columns drew, not
//!    a blank pane). Writes `/tmp/param_sweep_populated_render.png`.
//! 2. [`sweep_empty_paints_no_grid`] — the negative control: the SAME harness
//!    with `PanelState::Empty` paints ~no clay in the GRID band (the prompt has no
//!    table). Proves guard (1) is not a tautology. Writes
//!    `/tmp/param_sweep_empty_render.png`.
//! 3. [`sweep_populated_paints_strictly_more_than_empty`] — the populated frame
//!    paints STRICTLY MORE clay + foreground in the grid band than the empty
//!    frame. Ties the two states so a regression that flattens both fails.
//! 4. [`sweep_fragile_promote_disabled_accent_discriminator`] — the populated
//!    grid (which has promotable Robust/Marginal rows with the enabled ACCENT
//!    "Use this config" pill) paints MORE accent in the grid band than a grid
//!    whose every cell is FRAGILE (all locked, no accent) — the promotion-lock
//!    discriminator (a Robust row's accent affordance vs a Fragile row's greyed
//!    lock). Writes both PNGs.
//! 5. [`sweep_progress_determinate_paints`] — mid-sweep `BakeoffProgress
//!    {done:3, total:12}` paints the determinate bar's `ACCENT_2` fill (model the
//!    leaderboard's `bakeoff_progress_render.rs`). Writes the PNG.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse
//! (presence/absence of a hue, not byte-exact), robust within macOS across
//! font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::tune_screen_program;

/// Render the bare Tune screen body at the `typical` 1920×1080 slot and return
/// the physical-pixel RGBA buffer + dimensions.
fn render_tune_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = tune_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

// ── Region band ────────────────────────────────────────────────────────────────
//
// The Tune screen stacks, top-to-bottom, at the 1920×1080 / scale-1.0 slot
// (measured from the saved PNGs — see the per-test comments). The form panel
// (header + family chips + two SMA axes + the readout) takes the upper portion;
// the RESULT GRID (the benchmark strip + the column header + the cell rows + the
// baseline row) is well below it. The GRID band scan starts at `GRID_TOP` so the
// form's controls (and the header's Run button) never confound the grid scans.
//
// The form is conservatively tall (a family chip row + two axis rows of inputs +
// preset chips + the readout). `GRID_TOP = 430` is below the form panel for the
// default SMA form; the populated grid's clay + foreground live at/below it.

/// Top of the RESULT-GRID band — below the form panel + the benchmark strip.
const GRID_TOP: u32 = 430;

/// `true` for a `DOWN_500`-clay (#C97B5E — R201 G123 B94) pixel — red dominant
/// over green, green over blue, red high. (The exact predicate the leaderboard
/// guard uses; the FRAGILE badge + the Max-DD column paint this clay.)
fn is_down_clay(r: i32, g: i32, b: i32) -> bool {
    r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130
}

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — green & blue
/// high and close, red clearly lower. (The enabled "Use this config" pill paints
/// this teal; the FRAGILE locked affordance does not.)
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// Count `DOWN_500`-clay pixels in the `[y0, y1)` row band.
fn down_clay_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_down_clay(r, g, b) {
                hits += 1;
            }
        }
    }
    hits
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
/// anything that crosses a luma floor the near-black `CANVAS`/`PANEL` tiers never
/// reach. Monotonic in how much content drew.
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

/// The clay in the GRID band — the FRAGILE verdict badge + the FRAGILE row's
/// Max-DD-p95 column + the negative cells. A populated grid with a fragile cell
/// necessarily paints clay; the empty prompt does not.
fn grid_clay_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    down_clay_in_band(w, rgba, GRID_TOP, h)
}

/// The accent teal in the GRID band — the enabled "Use this config" pills on the
/// promotable (Robust/Marginal) rows. A grid with promotable rows paints this; a
/// grid where every cell is FRAGILE (all locked) paints far less.
fn grid_accent_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, GRID_TOP, h)
}

/// General foreground in the GRID band (the grid rows + distribution numbers).
fn grid_foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, GRID_TOP, h)
}

/// **The render-layer guard.** A populated `SweepReportMirror` MUST paint, in the
/// cockpit Tune screen:
/// - the `DOWN_500` clay of the FRAGILE verdict badge + the Max-DD-p95 column
///   (the load-bearing anti-overfit honesty pixel);
/// - a healthy amount of foreground text in the grid band (rows + distribution
///   columns drew).
///
/// Writes the operator-facing PNG to `/tmp/param_sweep_populated_render.png`.
#[test]
fn sweep_populated_paints_grid_and_fragile_badge() {
    let mirror = ui::fixtures::fake_sweep_report_mirror();
    // The fixture has a FRAGILE cell with a gaudy return — the anti-overfit case.
    assert!(
        mirror
            .cells
            .iter()
            .any(|c| matches!(c.verdict, ui::tune::SweepVerdictLabel::Fragile)),
        "the fixture must contain a FRAGILE cell"
    );

    let cockpit = ui::fixtures::fake_cockpit_tune(PanelState::Ready(mirror));
    let (w, h, rgba) = render_tune_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/param_sweep_populated_render.png");
    }

    let clay = grid_clay_pixels(w, h, &rgba);
    let fg = grid_foreground_pixels(w, h, &rgba);

    // The FRAGILE verdict badge (DOWN_50 backdrop + DOWN_500 "fragile" label) +
    // the FRAGILE row's Max-DD-p95 column paint clay in the grid band.
    assert!(
        clay > 80,
        "the FRAGILE badge + Max-DD column must paint DOWN_500 clay in the grid \
         band (expected >80 px, got {clay}). If this fails the FRAGILE flag / the \
         distribution columns did not render. PNG: /tmp/param_sweep_populated_render.png"
    );
    // The full grid (cells + distribution columns) is a lot of text.
    assert!(
        fg > 4000,
        "the populated result grid must paint a lot of foreground text in the \
         grid band (expected >4000 px, got {fg}). If this is low the screen \
         rendered a blank/empty pane despite Ready data. \
         PNG: /tmp/param_sweep_populated_render.png"
    );
}

/// **Negative control / discriminator.** The SAME harness with
/// `PanelState::Empty` renders the "set ranges and press Run sweep" prompt — a
/// single muted line, NO result grid. So there is ~no clay in the GRID band, and
/// far less grid-band foreground than the populated frame. Proves the populated
/// GRID guard genuinely discriminates (it is not satisfied by the form / the
/// header / the disclaimer).
#[test]
fn sweep_empty_paints_no_grid() {
    let cockpit = ui::fixtures::fake_cockpit_tune(PanelState::Empty);
    let (w, h, rgba) = render_tune_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/param_sweep_empty_render.png");
    }

    let clay = grid_clay_pixels(w, h, &rgba);
    // The empty prompt has no grid → no FRAGILE badge / Max-DD clay. A small
    // floor allows stray AA. (The form's controls are ABOVE the grid band; the
    // disclaimer is plain muted text with no clay.)
    assert!(
        clay < 60,
        "the Empty state must NOT paint result-grid clay (expected <60 stray clay \
         px, got {clay}). If high, the populated guard is a tautology. \
         PNG: /tmp/param_sweep_empty_render.png"
    );
}

/// Stronger discriminator — the populated frame paints STRICTLY MORE clay +
/// foreground in the grid band than the empty frame. Ties the two states
/// together in one assertion so a future regression that makes both look the
/// same fails.
#[test]
fn sweep_populated_paints_strictly_more_than_empty() {
    let populated = ui::fixtures::fake_cockpit_tune(PanelState::Ready(
        ui::fixtures::fake_sweep_report_mirror(),
    ));
    let empty = ui::fixtures::fake_cockpit_tune(PanelState::Empty);

    let (pw, ph, prgba) = render_tune_rgba(populated);
    let (ew, eh, ergba) = render_tune_rgba(empty);

    let pop_clay = grid_clay_pixels(pw, ph, &prgba);
    let emp_clay = grid_clay_pixels(ew, eh, &ergba);
    let pop_fg = grid_foreground_pixels(pw, ph, &prgba);
    let emp_fg = grid_foreground_pixels(ew, eh, &ergba);

    assert!(
        pop_clay > emp_clay + 50,
        "populated grid clay ({pop_clay}) must exceed empty ({emp_clay}) by a \
         clear margin — the result grid genuinely draws the FRAGILE/Max-DD clay"
    );
    assert!(
        pop_fg > emp_fg + 1500,
        "populated grid foreground ({pop_fg}) must far exceed empty ({emp_fg}) — \
         the result grid genuinely draws the rows + distribution columns"
    );
}

/// The FRAGILE promotion-lock discriminator. The populated grid has promotable
/// Robust/Marginal rows whose enabled "Use this config" pill paints `ACCENT`
/// teal; an all-FRAGILE grid has every action locked + greyed (no accent). So the
/// populated grid paints STRICTLY MORE accent in the grid band than the all-
/// fragile grid — proving the enabled-vs-locked affordance genuinely renders
/// differently (the "Fragile cannot be crowned" honesty lock).
#[test]
fn sweep_fragile_promote_disabled_accent_discriminator() {
    use ui::tune::SweepVerdictLabel;

    // Populated mix (has promotable rows → enabled accent pills).
    let mixed = ui::fixtures::fake_sweep_report_mirror();
    // An all-fragile variant: flip every cell + the baseline to Fragile + lock.
    let mut all_fragile = ui::fixtures::fake_sweep_report_mirror();
    for cell in &mut all_fragile.cells {
        cell.verdict = SweepVerdictLabel::Fragile;
        cell.promotable = false;
    }
    all_fragile.baseline.verdict = SweepVerdictLabel::Fragile;
    all_fragile.baseline.promotable = false;

    let mixed_cockpit = ui::fixtures::fake_cockpit_tune(PanelState::Ready(mixed));
    let fragile_cockpit = ui::fixtures::fake_cockpit_tune(PanelState::Ready(all_fragile));

    let (mw, mh, mrgba) = render_tune_rgba(mixed_cockpit);
    let (fw, fh, frgba) = render_tune_rgba(fragile_cockpit);

    if let Some(img) = image::RgbaImage::from_raw(mw, mh, mrgba.clone()) {
        let _ = img.save("/tmp/param_sweep_mixed_promote_render.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(fw, fh, frgba.clone()) {
        let _ = img.save("/tmp/param_sweep_allfragile_promote_render.png");
    }

    let mixed_accent = grid_accent_pixels(mw, mh, &mrgba);
    let fragile_accent = grid_accent_pixels(fw, fh, &frgba);

    // The mixed grid's enabled "Use this config" accent pills paint teal; the
    // all-fragile grid's locked affordances do not.
    assert!(
        mixed_accent > 150,
        "the mixed grid's enabled 'Use this config' pills must paint ACCENT teal \
         (expected >150 px, got {mixed_accent}). If this fails the promotable \
         affordance did not render. PNG: /tmp/param_sweep_mixed_promote_render.png"
    );
    assert!(
        mixed_accent > fragile_accent + 100,
        "the mixed grid ({mixed_accent} accent px) must paint clearly MORE accent \
         than the all-fragile grid ({fragile_accent}) — proof the FRAGILE rows' \
         promotion affordance is DISABLED+greyed (no accent), the 'Fragile cannot \
         be crowned' lock. PNGs: /tmp/param_sweep_{{mixed,allfragile}}_promote_render.png"
    );
}

// ── Progress band (mirror bakeoff_progress_render.rs) ─────────────────────────
//
// The determinate sweep progress bar is inserted between the form panel and the
// result body. Measured from the saved PNG: the bar fill is an 8px-tall ACCENT_2
// track. We scan a generous band below the form for the ACCENT_2 fill.

/// `true` for an `ACCENT_2` progress-fill pixel — a teal/blue close pair (the
/// progress bar's fill hue). Reuse the accent-teal predicate (ACCENT_2 is in the
/// same teal family for the fill); a non-trivial count proves the bar filled.
fn is_progress_fill(r: i32, g: i32, b: i32) -> bool {
    is_accent_teal(r, g, b)
}

/// **Determinate progress guard.** A mid-sweep `BakeoffProgress { done:3,
/// total:12 }` paints a partially-filled determinate bar — its `ACCENT_2` fill
/// appears below the form. Models `bakeoff_progress_render.rs`.
#[test]
fn sweep_progress_determinate_paints() {
    let cockpit = ui::fixtures::fake_cockpit_tune_running_progress(3, 12, "fast=15, slow=40");
    let (w, h, rgba) = render_tune_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/param_sweep_progress_render.png");
    }

    // Scan the whole frame below the header for the progress fill (the bar sits
    // between the form and the result body; a generous band catches it across
    // form-height jitter).
    let mut fill = 0u64;
    for y in GRID_TOP..h {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_progress_fill(r, g, b) {
                fill += 1;
            }
        }
    }
    // Also scan the band just below the form where the bar most likely sits.
    let mut bar_band = 0u64;
    for y in 300..GRID_TOP {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_progress_fill(r, g, b) {
                bar_band += 1;
            }
        }
    }

    assert!(
        fill + bar_band > 200,
        "the determinate sweep progress bar must paint its ACCENT fill (expected \
         >200 fill px across the body, got {} below-form + {} bar-band). If this \
         fails the progress bar did not render mid-sweep. \
         PNG: /tmp/param_sweep_progress_render.png",
        fill,
        bar_band
    );
}
