//! advisor-bakeoff tuning-knobs — render-layer proof that the two NEW bake-off
//! input knobs (the H1/H4/D1 **timeframe chip row** + the **start-capital text
//! field** with its honest "does not affect ranking" hint) actually DRAW in the
//! cockpit Leaderboard, with the SELECTED timeframe chip visibly distinct
//! (ACCENT) — and that the selection genuinely flows from state → pixels (drive
//! H4 through the real `Message::BakeoffSelectTimeframe` handler and prove the
//! active chip MOVED).
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, the `bakeoff_input` unit smoke (`view_constructs_*`), or a
//! no-panic boot is NOT proof the knobs paint. The knob view code can compile +
//! the unit smoke can pass while the chips/field never reach the screen (wrong
//! band, zero-dim node, clipped). This guard renders the REAL
//! `screens::leaderboard::view` HEADLESS and asserts on the rendered PIXELS that
//! the new tuning row drew, with a NEGATIVE CONTROL that distinguishes "knobs
//! drawn" from "blank panel": the three timeframe-chip slots are measured
//! independently and EXACTLY the selected slot is ACCENT-filled — and when the
//! selection moves H1 → H4 the filled slot moves with it.
//!
//! Four guards:
//!
//! 1. [`tuning_knobs_paint_timeframe_chips_and_capital_field`] — the default
//!    (H1) state paints (a) ACCENT teal in the timeframe-chip band (the active
//!    H1 chip), (b) a healthy foreground floor in the tuning band (three chips +
//!    the capital field + its value + the labels + the honest hint), and (c) the
//!    capital field's value ("100000") foreground. PNG:
//!    `/tmp/bakeoff_tuning_knobs_h1_render.png`.
//! 2. [`selected_timeframe_chip_is_the_only_accent_slot`] — the NEGATIVE CONTROL
//!    / discriminator: of the three chip slots, ONLY the selected (H1) slot is
//!    ACCENT-heavy; the H4 + D1 slots are ACCENT-light. Proves the highlight is
//!    a real per-chip selection treatment, not a band-wide teal wash (and not a
//!    blank panel — a blank panel paints ~0 teal in every slot).
//! 3. [`selecting_h4_moves_the_accent_chip`] — drive `BakeoffSelectTimeframe(
//!    FourHours)` through the REAL `ui::state::update` handler and prove the
//!    ACCENT-heavy slot MOVED from the H1 slot to the H4 slot (the H1 slot goes
//!    ACCENT-light, the H4 slot goes ACCENT-heavy). The load-bearing
//!    state-→-pixel proof. PNG: `/tmp/bakeoff_tuning_knobs_h4_render.png`.
//! 4. [`custom_start_capital_value_renders`] — drive `BakeoffSetStartCapital`
//!    with a NON-default value through the real handler and prove the field's
//!    value foreground changed (the typed value round-trips to pixels), distinct
//!    from the default so the guard is not satisfied by the placeholder alone.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are deliberately coarse
//! (presence/absence of a hue / a foreground delta), robust within macOS across
//! font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::leaderboard::BakeoffTimeframe;
use ui::state::{Cockpit, Message, PanelState};
use ui::test_support::leaderboard_screen_program;

/// Render the bare Leaderboard screen body at 1920×1080 / scale-1.0 and return
/// the physical-pixel RGBA buffer + dimensions (the same slot + harness the
/// other leaderboard render guards use).
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

/// A `Cockpit` on the Leaderboard screen in the cold `Empty` state, with the
/// guided-input form fully present (the form is the journey entry point — it
/// paints in `Empty`, per `leaderboard_empty_paints_no_table`). Empty keeps the
/// result-table out of the frame so the tuning-row band is uncontaminated by the
/// crowned-row ACCENT / Max-DD clay. The default timeframe is H1, default
/// capital "100000".
fn empty_leaderboard_cockpit() -> Cockpit {
    ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty)
}

// ── Region bands — the TUNING row (advisor-bakeoff tuning knobs) ──────────────
//
// The "Plan your bake-off" panel stacks, top-to-bottom: the panel title, the
// Coin chip row, the Budget field + Lookback chip row, the budget/FX hint, then
// the NEW tuning row — the "Bar size (changes ranking)" timeframe chips beside
// the "Start capital (USDT)" field — and finally the honest capital hint. The
// y-bands below were calibrated by reading the saved PNG
// (`/tmp/bakeoff_tuning_knobs_h1_render.png`); see the per-const measurements.

/// Top of the TUNING band — the "Bar size" label + the timeframe chip row begin
/// here (just below the budget/FX hint line). Calibrated from the saved PNG.
const TUNING_TOP: u32 = 300;
/// Bottom of the TUNING band — generously below the chip row + the capital field
/// + the honest hint, above where the budget-context line / result body begin.
const TUNING_BOTTOM: u32 = 420;

/// The timeframe-chip row's own y-band (the chips alone — excludes the "Bar
/// size" label above and the hint below). Calibrated from the saved PNG (the
/// active chip's ACCENT fill spans y≈321–348) so a teal scan here is the chip
/// FILL, not the label text; the band stops at 350, above the muted hint line.
const CHIP_ROW_TOP: u32 = 320;
const CHIP_ROW_BOTTOM: u32 = 350;

/// The timeframe chip row starts at the panel's left content edge. The three
/// chips (H1 / H4 / D1) flow left-to-right with `space::S` gaps; each chip is a
/// short 2-char label in `SMALL` text padded `[XS, S]`. The slot boundaries
/// below were measured from the saved PNGs — the H1 chip's ACCENT fill spans
/// x≈32–61 (when H1 is selected) and the H4 chip's spans x≈70–100 (when H4 is
/// selected). The boundaries fall in the inter-chip GAPS (62–69, ~101–107) so
/// each slot brackets exactly one chip and they are mutually exclusive — a
/// teal-heavy slot is unambiguous.
const CHIP_AREA_LEFT: u32 = 28;
/// Right edge of the H1 slot (slot 0) — in the gap after the H1 chip (ends ~61),
/// before the H4 chip (starts ~70).
const SLOT0_RIGHT: u32 = 66;
/// Right edge of the H4 slot (slot 1) — in the gap after the H4 chip (ends ~100),
/// before the D1 chip.
const SLOT1_RIGHT: u32 = 105;
/// Right edge of the D1 slot (slot 2). Past this the "Start capital" field +
/// label begin — well clear of the chips (the row uses a `space::L` gap before
/// the capital block; the field value starts at x≈178).
const SLOT2_RIGHT: u32 = 145;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — the exact
/// predicate the leaderboard crowned-row + Reports curve guards use: green & blue
/// high and close, red clearly lower. The SELECTED timeframe chip is a SOLID
/// ACCENT fill, so it paints this hue densely.
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// Count `ACCENT`-teal pixels in the rectangle `[x0,x1) × [y0,y1)`.
fn accent_teal_in_rect(w: u32, rgba: &[u8], x0: u32, x1: u32, y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    let x1 = x1.min(w);
    for y in y0..y1 {
        for x in x0..x1 {
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

/// ACCENT teal in the timeframe chip row, full width of the chip area.
fn chip_row_teal(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_rect(
        w,
        rgba,
        CHIP_AREA_LEFT,
        SLOT2_RIGHT,
        CHIP_ROW_TOP,
        CHIP_ROW_BOTTOM,
    )
}

/// ACCENT teal in the three per-chip slots: `(H1, H4, D1)`. Exactly one is the
/// selected (ACCENT-filled) slot at a time.
fn per_slot_teal(w: u32, rgba: &[u8]) -> (u64, u64, u64) {
    let h1 = accent_teal_in_rect(
        w,
        rgba,
        CHIP_AREA_LEFT,
        SLOT0_RIGHT,
        CHIP_ROW_TOP,
        CHIP_ROW_BOTTOM,
    );
    let h4 = accent_teal_in_rect(
        w,
        rgba,
        SLOT0_RIGHT,
        SLOT1_RIGHT,
        CHIP_ROW_TOP,
        CHIP_ROW_BOTTOM,
    );
    let d1 = accent_teal_in_rect(
        w,
        rgba,
        SLOT1_RIGHT,
        SLOT2_RIGHT,
        CHIP_ROW_TOP,
        CHIP_ROW_BOTTOM,
    );
    (h1, h4, d1)
}

/// Count general foreground (text / chip-fill) pixels in `[x0,x1) × [y0,y1)` —
/// anything crossing a luma floor the near-black `CANVAS`/`PANEL` tiers never
/// reach. Monotonic in how much content drew.
fn foreground_in_rect(w: u32, rgba: &[u8], x0: u32, x1: u32, y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    let x1 = x1.min(w);
    for y in y0..y1 {
        for x in x0..x1 {
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

/// Foreground across the whole tuning band (labels + chips + capital field +
/// value + honest hint).
fn tuning_band_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_rect(w, rgba, 0, w, TUNING_TOP, TUNING_BOTTOM)
}

/// The capital-FIELD value band — the "Start capital (USDT)" `text_input` sits to
/// the RIGHT of the timeframe chips on the same row. Its rendered VALUE (e.g.
/// "100000") is foreground in this rect. Calibrated from the saved PNG: the
/// value glyphs run x≈178–225 on the chip row. The band is scoped tightly to the
/// field value (NOT down into the muted hint line below) and starts well right
/// of the D1 chip (x>128), so it tracks the field value, not the chips/labels.
const CAPITAL_FIELD_LEFT: u32 = 170;
const CAPITAL_FIELD_RIGHT: u32 = 235;

fn capital_field_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_rect(
        w,
        rgba,
        CAPITAL_FIELD_LEFT,
        CAPITAL_FIELD_RIGHT,
        CHIP_ROW_TOP,
        CHIP_ROW_BOTTOM,
    )
}

/// **Guard 1.** The default (H1) leaderboard input MUST paint the new tuning
/// knobs: ACCENT teal in the timeframe chip row (the active H1 chip), a healthy
/// foreground floor across the whole tuning band (three chips + the "Bar size" /
/// "Start capital" labels + the capital field + its value + the honest hint),
/// and a non-trivial capital-field value foreground.
///
/// Writes the operator-facing PNG to `/tmp/bakeoff_tuning_knobs_h1_render.png`.
#[test]
fn tuning_knobs_paint_timeframe_chips_and_capital_field() {
    let (w, h, rgba) = render_leaderboard_rgba(empty_leaderboard_cockpit());

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/bakeoff_tuning_knobs_h1_render.png");
    }

    // (a) The active H1 chip paints a SOLID ACCENT fill in the chip row.
    let chip_teal = chip_row_teal(w, &rgba);
    assert!(
        chip_teal > 250,
        "the active (H1) timeframe chip must paint a SOLID ACCENT fill in the \
         chip row (expected >250 teal px, got {chip_teal}). If 0 the timeframe \
         chip row did not render. PNG: /tmp/bakeoff_tuning_knobs_h1_render.png"
    );

    // (b) The tuning band as a whole paints a healthy foreground floor — the
    //     two field labels, three chips, the capital field + value, the hint.
    let band_fg = tuning_band_foreground(w, &rgba);
    assert!(
        band_fg > 2000,
        "the tuning band (timeframe label + 3 chips + capital label + field + \
         value + honest hint) must paint a healthy foreground floor (expected \
         >2000 px, got {band_fg}). If low the tuning row did not render. \
         PNG: /tmp/bakeoff_tuning_knobs_h1_render.png"
    );

    // (c) The capital field's value ("100000") paints foreground in the field.
    let cap_fg = capital_field_foreground(w, &rgba);
    assert!(
        cap_fg > 80,
        "the start-capital field must paint its value (expected >80 foreground \
         px in the field rect, got {cap_fg}). If 0 the capital field did not \
         render its value. PNG: /tmp/bakeoff_tuning_knobs_h1_render.png"
    );
}

/// **Guard 2 — the NEGATIVE CONTROL / discriminator.** Of the three timeframe
/// chip slots, ONLY the selected (default H1) slot is ACCENT-heavy; the H4 + D1
/// slots are ACCENT-light. This proves the highlight is a real per-chip
/// SELECTION treatment, not a band-wide teal wash — and not a blank panel (a
/// blank panel paints ~0 teal in EVERY slot, failing the H1-heavy assertion).
#[test]
fn selected_timeframe_chip_is_the_only_accent_slot() {
    let (w, h, rgba) = render_leaderboard_rgba(empty_leaderboard_cockpit());

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/bakeoff_tuning_knobs_h1_render.png");
    }

    let (h1, h4, d1) = per_slot_teal(w, &rgba);

    // The selected H1 slot is a SOLID ACCENT fill → heavy teal.
    assert!(
        h1 > 250,
        "the selected H1 chip slot must be ACCENT-heavy (expected >250 teal px, \
         got {h1}). PNG: /tmp/bakeoff_tuning_knobs_h1_render.png"
    );
    // The unselected H4 + D1 slots are PANEL fill + a hairline border → light.
    assert!(
        h4 < 120,
        "the unselected H4 chip slot must be ACCENT-light (expected <120 stray \
         teal px, got {h4}) — only the SELECTED chip is filled. If high, the \
         highlight is a band-wide wash, not a per-chip selection. \
         PNG: /tmp/bakeoff_tuning_knobs_h1_render.png"
    );
    assert!(
        d1 < 120,
        "the unselected D1 chip slot must be ACCENT-light (expected <120 stray \
         teal px, got {d1}). PNG: /tmp/bakeoff_tuning_knobs_h1_render.png"
    );
    // And the selected slot strictly dominates each unselected slot.
    assert!(
        h1 > h4 + 150 && h1 > d1 + 150,
        "the selected H1 slot must paint strictly more ACCENT than the H4/D1 \
         slots (H1={h1}, H4={h4}, D1={d1}). If close, the selection highlight is \
         not discriminating between chips."
    );
}

/// **Guard 3 — the load-bearing state-→-pixel proof.** Drive
/// `Message::BakeoffSelectTimeframe(FourHours)` through the REAL
/// `ui::state::update` handler and prove the ACCENT-heavy slot MOVED from the H1
/// slot to the H4 slot: the H1 slot goes ACCENT-light and the H4 slot goes
/// ACCENT-heavy. This proves the chip selection genuinely flows from the message
/// handler → state → pixels (not just that the default happens to draw).
///
/// Writes the operator-facing PNG to `/tmp/bakeoff_tuning_knobs_h4_render.png`.
#[test]
fn selecting_h4_moves_the_accent_chip() {
    // Baseline: default H1 selection.
    let (w0, _h0, rgba0) = render_leaderboard_rgba(empty_leaderboard_cockpit());
    let (h1_before, h4_before, _d1_before) = per_slot_teal(w0, &rgba0);

    // Drive H4 through the production message handler.
    let mut cockpit = empty_leaderboard_cockpit();
    ui::state::update(
        &mut cockpit,
        Message::BakeoffSelectTimeframe(BakeoffTimeframe::FourHours),
    );
    assert_eq!(
        cockpit.leaderboard_screen_state.timeframe,
        BakeoffTimeframe::FourHours,
        "the update handler must store the H4 selection"
    );

    let (w, h, rgba) = render_leaderboard_rgba(cockpit);
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/bakeoff_tuning_knobs_h4_render.png");
    }

    let (h1_after, h4_after, _d1_after) = per_slot_teal(w, &rgba);

    // Before: H1 was the filled slot. After: H4 is the filled slot.
    assert!(
        h1_before > 250,
        "sanity: H1 must be the filled slot BEFORE the switch (got {h1_before})"
    );
    assert!(
        h4_after > 250,
        "after selecting H4 the H4 chip slot must be ACCENT-heavy (expected >250 \
         teal px, got {h4_after}). If 0 the chip selection did not flow from the \
         message handler to the pixels. PNG: /tmp/bakeoff_tuning_knobs_h4_render.png"
    );
    assert!(
        h1_after < 120,
        "after selecting H4 the (now unselected) H1 chip slot must go \
         ACCENT-light (expected <120 stray teal px, got {h1_after}). If still \
         heavy, the OLD selection is still painted — the highlight did not move. \
         PNG: /tmp/bakeoff_tuning_knobs_h4_render.png"
    );
    // The filled slot strictly moved: H4 gained what H1 lost.
    assert!(
        h4_after > h4_before + 150 && h1_after + 150 < h1_before,
        "the ACCENT fill must MOVE from the H1 slot to the H4 slot (H1 \
         {h1_before}→{h1_after}, H4 {h4_before}→{h4_after}). If the slots did not \
         swap, the selection is not driving the highlight."
    );
}

/// **Guard 4 — the capital value round-trips state → pixels.** Drive
/// `Message::BakeoffSetStartCapital` with a NON-default value through the real
/// handler and prove the field's value foreground CHANGED vs the default — so the
/// typed value genuinely reaches the rendered field (not just the placeholder).
///
/// Uses a value with a very different glyph count ("100000" → "7") so the
/// foreground delta is unambiguous and robust to font jitter.
#[test]
fn custom_start_capital_value_renders() {
    // Default capital "100000" → its value foreground.
    let (w0, _h0, rgba0) = render_leaderboard_rgba(empty_leaderboard_cockpit());
    let cap_default = capital_field_foreground(w0, &rgba0);
    assert!(
        cap_default > 80,
        "sanity: the default '100000' value must paint in the field (got \
         {cap_default})"
    );

    // Drive a single-digit value through the production handler.
    let mut cockpit = empty_leaderboard_cockpit();
    ui::state::update(
        &mut cockpit,
        Message::BakeoffSetStartCapital("7".to_string()),
    );
    assert_eq!(
        cockpit.leaderboard_screen_state.start_capital_input, "7",
        "the update handler must store the typed capital verbatim"
    );

    let (w, h, rgba) = render_leaderboard_rgba(cockpit);
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/bakeoff_tuning_knobs_capital7_render.png");
    }
    let cap_custom = capital_field_foreground(w, &rgba);

    // A single "7" is far less ink than "100000" → the field value foreground
    // strictly dropped. Proves the typed value (not the placeholder) is painted.
    assert!(
        cap_custom + 100 < cap_default,
        "the field value foreground must change when the typed capital changes \
         ('100000' {cap_default} px → '7' {cap_custom} px). If unchanged, the \
         field is painting the placeholder/default, not the typed value. \
         PNG: /tmp/bakeoff_tuning_knobs_capital7_render.png"
    );
    // But the field still painted SOMETHING (the "7" + the field chrome).
    assert!(
        cap_custom > 0,
        "the field must still paint the single '7' digit (got {cap_custom})"
    );
}
