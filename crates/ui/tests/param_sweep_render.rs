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
//! 6. [`sweep_macd_form_paints_third_axis`] + [`sweep_sma_form_has_no_third_axis`]
//!    (T7b) — selecting MACD renders the MACD axis form with a THIRD axis (Signal
//!    period) in a band where the SMA form (2 axes) paints ~nothing. Together
//!    they prove the picker swaps in the composed family's OWN axes (NOT the SMA
//!    axes, NOT the retired pending note). One screenshot per test (macOS font-
//!    mutex hazard). Writes both PNGs.
//! 7. [`sweep_macd_populated_paints_grid_and_fragile_badge`] (T7b) — a populated
//!    MACD `SweepReportMirror` (composed `macd(...)` labels + a FRAGILE cell)
//!    paints the gate-tied FRAGILE clay + the result grid. Writes the PNG.
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

// The MACD form has a THIRD axis row (Signal period) that the SMA form (two
// axes) does not. Measured from the saved PNGs at the 1920×1080 / scale-1.0 slot:
// the SMA form's last content is its grid readout at y≈369, so SMA paints ~nothing
// below y≈378; the MACD form's third "Signal period" axis (label + min/max/step
// inputs + preset chips) sits at y≈367-420 and its readout at y≈443. So a band
// JUST BELOW the SMA form, `[MACD_AXIS3_TOP, MACD_AXIS3_BOTTOM)`, captures MACD's
// THIRD axis but is blank for SMA — the clean discriminator that the MACD picker
// swapped in the MACD axes (NOT the 2-axis SMA form, NOT the retired pending
// note, which painted a single muted line WAY above this band).

/// Top of the MACD-third-axis band — below the SMA form's readout (y≈369).
const MACD_AXIS3_TOP: u32 = 385;

/// Bottom of the MACD-third-axis band — above the result-grid band.
const MACD_AXIS3_BOTTOM: u32 = 428;

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

/// Foreground in the MACD-third-axis band — the region just below the SMA form's
/// readout. The MACD form's THIRD axis (Signal period: label + min/max/step
/// inputs + preset chips) paints here; the SMA form (two axes) paints ~nothing
/// (its content ends at the readout, above this band). So a high count here ==
/// "a third axis row rendered" == the MACD form (not SMA, not the retired
/// pending note) drew.
fn macd_third_axis_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, MACD_AXIS3_TOP, MACD_AXIS3_BOTTOM)
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

/// **advisor-param-promotion (ADR-0070 § T12 Proof 1) — the WIRED button.** The
/// "Use this config" affordance on a promotable row is now an ENABLED accent
/// BUTTON (it carries `on_press(Message::PromoteSweptConfig(..))`); the FRAGILE
/// row keeps its greyed locked LABEL (no press). This proof pins the
/// discriminator at the pixel layer: the populated mix (promotable rows) paints
/// the enabled accent affordance in the USE column; an all-fragile grid (every
/// row locked) paints ~none. It is the SAME mechanism the
/// `sweep_fragile_promote_disabled_accent_discriminator` test guards — restated
/// here as the explicit promotion-wiring proof so a regression that reverts the
/// button to a no-op label (or weakens the FRAGILE lock) fails LOUDLY.
///
/// FAIL-before: with the affordance still a non-pressable visual `Container`
/// pill, this test's accent assertion holds AT THE PIXEL LAYER (the pill looked
/// identical) — so the pixel guard is NOT, alone, proof of the wiring. The
/// load-bearing wiring proof is the `state.rs` pure-state test
/// (`promote_swept_config` sets `pending_forward_promotion` + navigates) + the
/// Proof-2 promoted-plan render; this guard proves the AFFORDANCE renders as the
/// enabled, eligible state vs the locked one. Writes both PNGs.
#[test]
fn sweep_promotable_use_config_is_enabled_accent_button() {
    use ui::tune::SweepVerdictLabel;

    // Populated mix → promotable rows carry the enabled accent button.
    let mixed = ui::fixtures::fake_sweep_report_mirror();
    assert!(
        mixed.cells.iter().any(|c| c.promotable),
        "the mix must contain at least one promotable row (the enabled button)"
    );
    // All-fragile control → every affordance is the locked label (no accent).
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
        let _ = img.save("/tmp/param_sweep_promote_button_mixed.png");
    }
    if let Some(img) = image::RgbaImage::from_raw(fw, fh, frgba.clone()) {
        let _ = img.save("/tmp/param_sweep_promote_button_allfragile.png");
    }

    let mixed_accent = grid_accent_pixels(mw, mh, &mrgba);
    let fragile_accent = grid_accent_pixels(fw, fh, &frgba);

    assert!(
        mixed_accent > 150,
        "the promotable rows' ENABLED 'Use this config' BUTTON must paint ACCENT \
         teal (expected >150 px, got {mixed_accent}). If this fails the wired \
         affordance did not render as the enabled accent state. \
         PNG: /tmp/param_sweep_promote_button_mixed.png"
    );
    assert!(
        mixed_accent > fragile_accent + 100,
        "the promotable button accent ({mixed_accent}) must clearly exceed the \
         all-fragile grid ({fragile_accent}) — proof the FRAGILE lock stays a \
         greyed label (no on_press, no accent) while the promotable row is the \
         enabled accent button. The lock is SACRED — a fragile config can never \
         promote. PNGs: /tmp/param_sweep_promote_button_{{mixed,allfragile}}.png"
    );
}

// ── Composed-family form + result grid (T7b) ──────────────────────────────────
//
// The T7b flip makes MACD / RSI / Bollinger runnable: selecting a composed
// family renders ITS axis form (MACD = 3 axes; RSI = 2; Bollinger = 1 axis + a
// `k` multi-select) — NOT the SMA axes, and NOT the (retired) "not sweepable
// yet" pending note. These guards prove, at the pixel layer:
//   (a) selecting MACD paints MORE form-band foreground than SMA (3 axes > 2) —
//       the chosen family's OWN, more-numerous axes drew (the pending note would
//       have drawn FAR LESS than even SMA's 2 axes: this is the fail-before
//       discriminator, the old behaviour is strictly below SMA, the new strictly
//       above);
//   (b) a populated MACD result grid paints the FRAGILE clay + heavy foreground
//       (the result grid + the gate-tied FRAGILE treatment are family-agnostic).

// NOTE (macOS render-harness hazard): each test below renders EXACTLY ONE
// screenshot. Rendering two screenshots inside one `#[test]` reliably wedges the
// cosmic-text/CoreText font mutex on macOS (spec/dev-notes/iced-ui-render-
// verification.md). So the MACD-vs-SMA discriminator is split into two
// single-screenshot tests that share the `MACD_AXIS3_*` band: MACD paints a real
// third-axis row there; SMA (two axes) paints ~nothing there.

/// **Composed-family form proof (T7b), MACD half.** Selecting MACD renders the
/// MACD axis form — three `{min, max, step}` axis rows (fast / slow / signal) +
/// preset chips. The THIRD axis (Signal period) sits in the `MACD_AXIS3_*` band
/// (just below where the SMA form ends), so a healthy foreground count there
/// proves the MACD form rendered a third axis (NOT the SMA 2-axis form, NOT the
/// retired one-line pending note which painted far above this band).
///
/// Writes `/tmp/param_sweep_macd_form_render.png`.
#[test]
fn sweep_macd_form_paints_third_axis() {
    use ui::tune::TuneFamily;

    let macd = ui::fixtures::fake_cockpit_tune_family(TuneFamily::Macd, PanelState::Empty);
    let (mw, mh, mrgba) = render_tune_rgba(macd);

    if let Some(img) = image::RgbaImage::from_raw(mw, mh, mrgba.clone()) {
        let _ = img.save("/tmp/param_sweep_macd_form_render.png");
    }

    let macd_axis3 = macd_third_axis_foreground(mw, &mrgba);
    assert!(
        macd_axis3 > 500,
        "the MACD form must paint its THIRD axis row (Signal period: label + 3 \
         inputs + preset chips) in the third-axis band (expected >500 fg px, got \
         {macd_axis3}). If low, the MACD picker did not swap in the MACD axes. \
         PNG: /tmp/param_sweep_macd_form_render.png"
    );
}

/// **Composed-family form proof (T7b), SMA control half / negative control.** The
/// SMA form (two axes) ends at its grid readout, ABOVE the `MACD_AXIS3_*` band —
/// so SMA paints ~nothing there. Paired with [`sweep_macd_form_paints_third_axis`]
/// this proves the band genuinely discriminates "has a third axis" (MACD) from
/// "has only two axes" (SMA) — the picker swaps the form, it is not a tautology.
///
/// Writes `/tmp/param_sweep_sma_form_render.png`.
#[test]
fn sweep_sma_form_has_no_third_axis() {
    use ui::tune::TuneFamily;

    let sma = ui::fixtures::fake_cockpit_tune_family(TuneFamily::Sma, PanelState::Empty);
    let (sw, sh, srgba) = render_tune_rgba(sma);

    if let Some(img) = image::RgbaImage::from_raw(sw, sh, srgba.clone()) {
        let _ = img.save("/tmp/param_sweep_sma_form_render.png");
    }

    let sma_axis3 = macd_third_axis_foreground(sw, &srgba);
    assert!(
        sma_axis3 < 200,
        "the SMA form (two axes) must paint ~nothing in the third-axis band \
         (expected <200 stray fg px, got {sma_axis3}) — its content ends at the \
         readout above the band. If high, the band is not a clean MACD-vs-SMA \
         discriminator. PNG: /tmp/param_sweep_sma_form_render.png"
    );
}

/// **Composed-family result-grid proof (T7b).** A populated MACD
/// `SweepReportMirror` (with `macd(f,s,sig)` param labels and a FRAGILE cell)
/// paints, in the cockpit Tune screen with MACD selected:
/// - the `DOWN_500` clay of the FRAGILE verdict badge + the Max-DD-p95 column
///   (the gate-tied honesty treatment is family-agnostic — a composed config
///   that overfits still renders FRAGILE + promotion-locked);
/// - a healthy amount of grid-band foreground (the rows + distribution columns).
///
/// Writes `/tmp/param_sweep_macd_populated_render.png`.
#[test]
fn sweep_macd_populated_paints_grid_and_fragile_badge() {
    use ui::tune::{SweepVerdictLabel, TuneFamily};

    let mirror = ui::fixtures::fake_sweep_report_mirror_macd();
    // The MACD fixture must carry a FRAGILE cell (the anti-overfit case) and the
    // composed `macd(...)` label shape.
    assert!(
        mirror
            .cells
            .iter()
            .any(|c| matches!(c.verdict, SweepVerdictLabel::Fragile)),
        "the MACD fixture must contain a FRAGILE cell"
    );
    assert!(
        mirror
            .cells
            .iter()
            .any(|c| c.params_label.contains("macd(")),
        "the MACD fixture must carry the composed `macd(...)` param labels"
    );

    let cockpit =
        ui::fixtures::fake_cockpit_tune_family(TuneFamily::Macd, PanelState::Ready(mirror));
    let (w, h, rgba) = render_tune_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/param_sweep_macd_populated_render.png");
    }

    let clay = grid_clay_pixels(w, h, &rgba);
    let fg = grid_foreground_pixels(w, h, &rgba);

    assert!(
        clay > 80,
        "the MACD FRAGILE badge + Max-DD column must paint DOWN_500 clay in the \
         grid band (expected >80 px, got {clay}). The gate-tied FRAGILE treatment \
         must cover composed cells. PNG: /tmp/param_sweep_macd_populated_render.png"
    );
    assert!(
        fg > 4000,
        "the populated MACD result grid must paint a lot of grid-band foreground \
         (expected >4000 px, got {fg}). PNG: /tmp/param_sweep_macd_populated_render.png"
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
