//! advisor-signal-library-expansion (T12, ADR-0071 § D6) — render-layer proof
//! that the cockpit Leaderboard renders the FIXED 5-arm signal-library slate
//! HONESTLY at the full ~18-row field: each new arm (`v0.donchian_break`,
//! `v0.donchian_floor`, `v0.vol_breakout`, `v0.roc_momentum`, `v0.obv`) shows a
//! FRIENDLY label (NOT a raw `v0.donchian_break` id) + its KPIs + the (mostly
//! Fragile) badge.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the new rows / friendly labels
//! draw. The lesson from advisor-combination-search (repeated by the short slate)
//! is sharper still: the engine adds the new ids in `default_field()` but the
//! leaderboard `display_label` mapping must be extended UI-side or the rows show
//! RAW ids. This guard renders the REAL `screens::leaderboard::view` HEADLESS
//! with an 18-arm `BakeoffReportMirror` (the 13-arm field + the 5 new arms) and
//! asserts on the rendered PIXELS — with TWO negative controls (the 13-arm field
//! paints strictly less; a raw-id variant of the same 18-arm field paints
//! strictly less strategy-column text — proving the friendly labels, not the raw
//! ids, drew).
//!
//! Three guards (populated + two negative controls, per CLAUDE.md):
//!
//! 1. [`leaderboard_signal_library_paints_new_rows_and_friendly_labels`] — the
//!    18-arm fixture paints (a) MANY more foreground pixels than the 13-arm field
//!    (the 5 new rows drew), (b) foreground text in the NEW-ROWS band below where
//!    the 13-arm table ends (the new rows are physically there), and (c)
//!    `DOWN_500` Fragile-badge clay in that band (the new arms carry the honest
//!    Fragile flag). Writes the PNG to `/tmp/leaderboard_signal_library_render.png`.
//! 2. [`leaderboard_13_arm_is_the_negative_control_for_signal_library`] — the
//!    SAME harness with the 13-arm field paints STRICTLY LESS foreground + ~no
//!    text in the new-rows band. Proves guard (1) genuinely discriminates the
//!    wider field (it is not a tautology satisfied by the always-present chrome).
//! 3. [`friendly_labels_paint_more_strategy_text_than_raw_ids`] — the SAME 18-arm
//!    field, but with the 5 new arms' ids rewritten to NON-mapping forms (so
//!    `display_label` falls through to the raw id), paints STRICTLY LESS
//!    strategy-column foreground than the friendly-label field. This is the
//!    direct friendly-vs-raw discriminator: the friendly labels (e.g. "Donchian
//!    breakout (20-bar high)") are wider than the raw ids (e.g.
//!    "rawid.donchian_break") → more painted text in the strategy column.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_short_arms_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text rasterisation is per-OS). The file compiles to
//! nothing on Linux/Windows. Pixel thresholds are deliberately coarse
//! (presence/absence of a hue, relative magnitudes — never byte-exact).

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

/// Render the bare Leaderboard screen body at 1920×1600 / scale-1.0.
///
/// The viewport is 1600px tall (not the 1080 the canonical leaderboard guard
/// uses) because the 18-arm signal-library field is the LONGEST leaderboard
/// state: the recommendation block + 18 ranked rows + the persistent not-advice
/// disclaimer. `iced_test::screenshot` CLIPS to the viewport rectangle (content
/// beyond it is not captured — see `gallery_snapshots.rs` H-GAL-2), so a short
/// viewport would clip the bottom new rows. Verified by reading
/// `/tmp/leaderboard_signal_library_render.png`: at 1600px all 18 rows + the
/// disclaimer are captured with margin.
fn render_leaderboard_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1600), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

// ── Region bands ──────────────────────────────────────────────────────────────
//
// The bare body stacks (header / form / budget-context / recommendation / table
// / disclaimer). The 13-arm table ends well above the 18-arm table's last rows;
// the 5 NEW rows (ranked after the crown-eligible set) live in the LOWER table
// band. `NEW_ROWS_TOP` is a conservative y below where the 13-arm field's table
// ends, so it isolates the band only the wider field populates with table rows.

/// Top of the band that ONLY the 18-arm field fills with table rows (the 13-arm
/// field's table has ended above this y; only the recommendation + disclaimer —
/// not table rows — would otherwise appear here). Conservative; verified against
/// the saved PNG.
const NEW_ROWS_TOP: u32 = 980;

/// `true` for a `DOWN_500`-clay (#C97B5E) pixel — red dominant, green over blue.
/// The Fragile badges (+ the always-negative Max-DD column on losing new arms)
/// paint this hue. Same predicate as `leaderboard_short_arms_render.rs`.
fn is_down_clay(r: i32, g: i32, b: i32) -> bool {
    r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130
}

/// Count pixels matching `pred` in the `[y0, y1)` band, full width.
fn count_in_band(
    w: u32,
    rgba: &[u8],
    y0: u32,
    y1: u32,
    pred: impl Fn(i32, i32, i32) -> bool,
) -> u64 {
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if pred(r, g, b) {
                hits += 1;
            }
        }
    }
    hits
}

/// General foreground (luma floor) in a `[y0, y1)` band — the painted text/chrome.
fn foreground_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    count_in_band(w, rgba, y0, y1, |r, g, b| (r * 2 + g * 3 + b) / 6 > 80)
}

/// General foreground across the whole frame.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, 0, h)
}

/// Foreground in the STRATEGY column band — the left ~640px of the table, where
/// the friendly label (or the raw id) is painted. Right of `W_STRAT_EST` are the
/// numeric columns (return / sharpe / max-dd / trades), which are IDENTICAL
/// between the friendly-label and raw-id fixtures — so scoping the scan to the
/// strategy column isolates the label-text difference.
///
/// `W_STRAT_EST` (≈ rank cell + the strategy fill column) is a conservative
/// left-of-the-numbers cut, verified against the saved PNG.
fn strategy_column_foreground(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    const W_STRAT_EST: u32 = 720;
    let x1 = W_STRAT_EST.min(w);
    let mut hits = 0u64;
    for y in y0..y1 {
        for x in 0..x1 {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if (r * 2 + g * 3 + b) / 6 > 80 {
                hits += 1;
            }
        }
    }
    hits
}

/// **The render-layer guard.** The 18-arm signal-library field MUST, in the
/// cockpit Leaderboard, paint MORE than the 13-arm field (the 5 new rows drew)
/// AND populate the new-rows band with text + Fragile-badge clay.
///
/// Writes the operator-facing PNG to `/tmp/leaderboard_signal_library_render.png`.
#[test]
fn leaderboard_signal_library_paints_new_rows_and_friendly_labels() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_with_signal_library();

    // Sanity: the fixture IS the 18-arm field with the 5 new arms present.
    assert_eq!(
        mirror.rows.len(),
        18,
        "13-arm field + 5 signal-library arms + (benchmark already in the 13)"
    );
    let new_ids = [
        "v0.donchian_break",
        "v0.donchian_floor",
        "v0.vol_breakout",
        "v0.roc_momentum",
        "v0.obv",
    ];
    for id in new_ids {
        assert!(
            mirror.rows.iter().any(|r| r.strategy.as_str() == id),
            "the signal-library fixture must carry the `{id}` arm"
        );
    }

    let mirror_13 = ui::fixtures::fake_bakeoff_report_mirror();

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let cockpit_13 = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror_13));

    let (w, h, rgba) = render_leaderboard_rgba(cockpit);
    let (_w13, _h13, rgba_13) = render_leaderboard_rgba(cockpit_13);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_signal_library_render.png");
    }

    let fg_18 = foreground_pixels(w, h, &rgba);
    let fg_13 = foreground_pixels(w, h, &rgba_13);
    let new_band_fg = foreground_in_band(w, &rgba, NEW_ROWS_TOP, h);
    let new_band_fg_13 = foreground_in_band(w, &rgba_13, NEW_ROWS_TOP, h);
    let new_band_clay = count_in_band(w, &rgba, NEW_ROWS_TOP, h, is_down_clay);

    // (a) The 18-arm field paints strictly MORE foreground than the 13-arm field
    //     — the 5 new rows drew. A coarse margin (each new row is ~hundreds of
    //     foreground px across the label + 4 numeric columns).
    assert!(
        fg_18 > fg_13 + 1500,
        "the 18-arm signal-library field must paint MORE foreground than the \
         13-arm field (the 5 new rows must draw) — got 18-arm={fg_18}, \
         13-arm={fg_13}. PNG: /tmp/leaderboard_signal_library_render.png"
    );
    // (b) The new-rows band carries real table-row text in the 18-arm field, and
    //     measurably MORE than the 13-arm field (whose table ended above).
    assert!(
        new_band_fg > 4000,
        "the new-rows band must paint the 5 new rows' text (expected >4000 fg px \
         below y={NEW_ROWS_TOP}, got {new_band_fg}). PNG: \
         /tmp/leaderboard_signal_library_render.png"
    );
    assert!(
        new_band_fg > new_band_fg_13 + 1500,
        "the new-rows band must paint STRICTLY MORE than the 13-arm field there \
         (18-arm={new_band_fg}, 13-arm={new_band_fg_13}) — proving the new rows, \
         not chrome, fill the band. PNG: /tmp/leaderboard_signal_library_render.png"
    );
    // (c) The new arms carry the honest Fragile badge → DOWN_500 clay in the band
    //     (4 of the 5 new arms are Fragile + their negative Max-DD column).
    assert!(
        new_band_clay > 120,
        "the new arms must paint DOWN_500 clay in the new-rows band (the Fragile \
         badges + the Max-DD column) (expected >120 clay px, got {new_band_clay}). \
         PNG: /tmp/leaderboard_signal_library_render.png"
    );
}

/// **Negative control #1.** The SAME harness with the 13-arm field paints
/// STRICTLY LESS foreground overall + ~no table-row text in the new-rows band.
/// Proves the populated guard genuinely discriminates the wider 18-arm field
/// (it is not satisfied by the always-present header / form / recommendation /
/// disclaimer chrome).
#[test]
fn leaderboard_13_arm_is_the_negative_control_for_signal_library() {
    let mirror_18 = ui::fixtures::fake_bakeoff_report_mirror_with_signal_library();
    let mirror_13 = ui::fixtures::fake_bakeoff_report_mirror();

    let cockpit_18 = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror_18));
    let cockpit_13 = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror_13));

    let (w18, h18, r18) = render_leaderboard_rgba(cockpit_18);
    let (w13, h13, r13) = render_leaderboard_rgba(cockpit_13);

    if let Some(img) = image::RgbaImage::from_raw(w13, h13, r13.clone()) {
        let _ = img.save("/tmp/leaderboard_signal_library_13arm_control.png");
    }

    let band_18 = foreground_in_band(w18, &r18, NEW_ROWS_TOP, h18);
    let band_13 = foreground_in_band(w13, &r13, NEW_ROWS_TOP, h13);

    assert!(
        band_13 < band_18,
        "the 13-arm field must paint STRICTLY LESS in the new-rows band than the \
         18-arm field (13-arm={band_13}, 18-arm={band_18}). If equal the wider- \
         field guard is a tautology. PNG: \
         /tmp/leaderboard_signal_library_13arm_control.png"
    );
}

/// **Negative control #2 — the direct friendly-vs-raw discriminator.** The SAME
/// 18-arm field, but with the 5 new arms' ids rewritten to NON-mapping forms
/// (so `display_label` falls through and renders the raw id), paints STRICTLY
/// LESS strategy-column foreground than the friendly-label field. Proves the
/// FRIENDLY labels (wider than the raw ids) actually drew — not the raw ids.
///
/// This is the exact advisor-combination-search regression made render-visible:
/// if `display_label` did NOT map the new ids, the friendly field would paint
/// the same (raw) strategy text as this control and the assertion would fail.
#[test]
fn friendly_labels_paint_more_strategy_text_than_raw_ids() {
    use smol_str::SmolStr;

    let friendly = ui::fixtures::fake_bakeoff_report_mirror_with_signal_library();

    // Build the raw-id control: clone the friendly field and rewrite ONLY the 5
    // new arms' ids to forms `display_label` does not map (a `rawid.` prefix is
    // not in any slate), so they fall through to the raw id. Everything else
    // (numbers, ranks, robustness, the other 13 rows) is byte-identical, so the
    // ONLY rendered difference is the new arms' strategy-column TEXT.
    let mut raw = friendly.clone();
    for row in &mut raw.rows {
        let mapped = match row.strategy.as_str() {
            "v0.donchian_break" => Some("rawid.donchian_break"),
            "v0.donchian_floor" => Some("rawid.donchian_floor"),
            "v0.vol_breakout" => Some("rawid.vol_breakout"),
            "v0.roc_momentum" => Some("rawid.roc_momentum"),
            "v0.obv" => Some("rawid.obv"),
            _ => None,
        };
        if let Some(new_id) = mapped {
            row.strategy = SmolStr::new(new_id);
        }
    }

    let cockpit_friendly = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(friendly));
    let cockpit_raw = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(raw));

    let (wf, hf, rf) = render_leaderboard_rgba(cockpit_friendly);
    let (wr, _hr, rr) = render_leaderboard_rgba(cockpit_raw);

    if let Some(img) = image::RgbaImage::from_raw(wr, _hr, rr.clone()) {
        let _ = img.save("/tmp/leaderboard_signal_library_rawid_control.png");
    }

    // Scope to the strategy column of the new-rows band — the numeric columns are
    // identical between the two fixtures, so any difference there is label text.
    let friendly_text = strategy_column_foreground(wf, &rf, NEW_ROWS_TOP, hf);
    let raw_text = strategy_column_foreground(wr, &rr, NEW_ROWS_TOP, hf);

    assert!(
        friendly_text > raw_text,
        "the FRIENDLY labels must paint MORE strategy-column text than the raw \
         ids (friendly={friendly_text}, raw={raw_text}) — the friendly labels \
         (e.g. \"Donchian breakout (20-bar high)\") are wider than the raw ids \
         (e.g. \"rawid.donchian_break\"). If equal/less the display_label mapping \
         is not applied. PNG: /tmp/leaderboard_signal_library_rawid_control.png"
    );
}
