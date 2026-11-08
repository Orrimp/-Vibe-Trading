//! Story 3-19 (ADR-0101) — RENDERED-PIXEL proof that the live-rate provenance
//! reaches the operator's screen (AD-10).
//!
//! ## Why a pixel test and not a unit test
//!
//! The change this story makes to the UI is a change to a LABEL: the FX note
//! now reads "€200 ≈ $223.30 (at 1.1165 EUR/USD, live:ecb as of 2026-11-06)"
//! when a live rate is in force, and keeps reading "… 1.08 EUR/USD, config)"
//! when it is not. A unit test on the composed string (that is
//! `advisor_live_fx_rate.rs`) cannot tell whether the line ever painted — the
//! defect class that shipped the empty equity curve twice. So this file renders
//! the real screens headless and asserts on pixels.
//!
//! ## The discriminator
//!
//! The live note is STRICTLY LONGER than the config note (a four-decimal rate
//! plus "live:ecb as of 2026-11-06" versus "config"), so it must paint strictly
//! more foreground in the band that holds it. That is a real two-sided
//! discriminator rather than a presence check: both states paint a note, and
//! they must paint DIFFERENT amounts of it. Each guard therefore ships with its
//! negative control:
//!
//! - [`forward_plan_live_fx_note_paints_more_than_the_config_note`] — the
//!   launched-run note on the Forward-plan sizing block (positive = live,
//!   control = config). PNGs: `/tmp/advisor_live_fx_plan_{live,config}.png`.
//! - [`forward_plan_without_an_fx_note_still_paints_the_static_line`] — the
//!   second control: `forward_fx = None` (the legacy soak path) still paints a
//!   budget line, so the guard above is measuring the note's CONTENT and not
//!   its existence.
//! - [`leaderboard_hint_paints_more_with_a_live_rate`] — the pre-launch budget
//!   hint, which is also where the "Refresh rate" affordance lives (ADR-0101
//!   D7). PNGs: `/tmp/advisor_live_fx_hint_{live,config}.png`.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like every other pixel harness here, real-renderer assertions are
//! macOS-canonical (cosmic-text rasterisation is per-OS); the file compiles to
//! nothing elsewhere. Thresholds are coarse and relative, robust to font-DB
//! jitter within macOS.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::FxRate;
use ui::state::{Cockpit, PanelState};
use ui::test_support::{forward_plan_screen_program, leaderboard_screen_program};

/// The ECB rate measured 2026-11-08 and the publication date it carried.
const LIVE_RATE: Decimal = dec!(1.1165);
const LIVE_AS_OF: &str = "2026-11-06";

/// Render the bare Forward-plan screen body at the 1920×1080 slot.
fn render_forward_plan(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = forward_plan_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

/// Render the bare Leaderboard screen body at the 1920×1080 slot.
fn render_leaderboard(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = leaderboard_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

/// Count foreground (text) pixels in the `[y0, y1)` band — anything crossing a
/// luma floor the near-black panel tiers never reach. Monotonic in how much
/// text drew, which is what makes a longer label measurable.
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

fn save(name: &str, w: u32, h: u32, rgba: &[u8]) {
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
        let _ = img.save(format!("/tmp/{name}.png"));
    }
}

/// The live FX rate as the fetcher would return it.
fn live_fx() -> FxRate {
    FxRate::new(LIVE_RATE, "live:ecb", LIVE_AS_OF).expect("a positive rate")
}

// ── Forward plan — the launched run's note ───────────────────────────────────
//
// The sizing block sits below the header, framing banner, stance block and
// standing-rules block. The BUDGET LINE — the one line this story changes — is
// the second line of that block. Measured per-row from the saved PNGs at the
// 1920×1080 / scale-1.0 slot (`measured_bands_for_the_record` prints the scan):
// rows 586-612 hold the budget line and nothing else — the sizing sentence above
// it ends by row 580 and the rows below it are empty. In that band the config
// note paints ~1232 px and the live note ~1482 px, so the ~250 px difference IS
// the extra copy, not a shift in the blocks above.

/// Top of the BUDGET-LINE band.
const SIZING_TOP: u32 = 586;
/// Bottom of the BUDGET-LINE band.
const SIZING_BOTTOM: u32 = 612;

/// **The guard.** A plan launched at the LIVE rate paints strictly more
/// foreground in the sizing band than the same plan launched at the configured
/// rate — because the live note carries four rate decimals plus "live:ecb as of
/// 2026-11-06" where the config note carries "config".
#[test]
fn forward_plan_live_fx_note_paints_more_than_the_config_note() {
    let live = ui::fixtures::fake_cockpit_forward_plan_with_fx(live_fx());
    let config = ui::fixtures::fake_cockpit_forward_plan_with_fx(FxRate::config(dec!(1.08)));

    let (wl, hl, rl) = render_forward_plan(live);
    let (wc, hc, rc) = render_forward_plan(config);
    save("advisor_live_fx_plan_live", wl, hl, &rl);
    save("advisor_live_fx_plan_config", wc, hc, &rc);

    let live_band = foreground_in_band(wl, &rl, SIZING_TOP, SIZING_BOTTOM);
    let config_band = foreground_in_band(wc, &rc, SIZING_TOP, SIZING_BOTTOM);

    // Both must paint a real sizing block — neither state is blank.
    assert!(
        config_band > 800,
        "the CONFIG budget line must paint (measured ~1232 px, got {config_band}). \
         If this is low the line did not render at all and the comparison below \
         would be meaningless. PNG: /tmp/advisor_live_fx_plan_config.png"
    );
    assert!(
        live_band > 800,
        "the LIVE budget line must paint (measured ~1482 px, got {live_band}). \
         PNG: /tmp/advisor_live_fx_plan_live.png"
    );

    // And the live note must be visibly LONGER — the provenance + the extra
    // rate decimals actually reached the screen.
    assert!(
        live_band > config_band + 150,
        "the live note must paint clearly more foreground than the config note \
         (live {live_band} vs config {config_band}). If they are equal the \
         provenance never reached the rendered label — which is exactly the \
         defect a text-level test cannot see. \
         PNGs: /tmp/advisor_live_fx_plan_{{live,config}}.png"
    );
}

/// **Negative control #2 — no FX note at all.** The legacy path (`forward_fx =
/// None`, a soak/research run) still paints a budget line from the default
/// rate, and it paints LESS than the live note. Proves the guard above measures
/// the note's content rather than the mere presence of a sizing block.
#[test]
fn forward_plan_without_an_fx_note_still_paints_the_static_line() {
    let mut none = ui::fixtures::fake_cockpit_forward_plan(PanelState::Ready(
        ui::fixtures::fake_forward_plan(),
    ));
    none.forward_fx = None;
    let live = ui::fixtures::fake_cockpit_forward_plan_with_fx(live_fx());

    let (wn, hn, rn) = render_forward_plan(none);
    let (wl, _hl, rl) = render_forward_plan(live);
    save("advisor_live_fx_plan_no_note", wn, hn, &rn);

    let none_band = foreground_in_band(wn, &rn, SIZING_TOP, SIZING_BOTTOM);
    let live_band = foreground_in_band(wl, &rl, SIZING_TOP, SIZING_BOTTOM);

    assert!(
        none_band > 800,
        "even without an FxNote the sizing block paints its static budget line \
         (got {none_band} px) — the F7 fallback is unchanged. \
         PNG: /tmp/advisor_live_fx_plan_no_note.png"
    );
    assert!(
        live_band > none_band + 150,
        "the live note must paint clearly more than the no-note fallback \
         (live {live_band} vs none {none_band})."
    );
}

// ── Leaderboard — the pre-launch hint + the refresh affordance ───────────────
//
// The budget hint sits inside the FORM band, below the coin chips and the
// budget field row (the band `eur_fx_budget_render.rs` established).

/// Top of the FORM band (just below the header).
const FORM_TOP: u32 = 110;
/// Bottom of the FORM band (just above the budget-context line).
const FORM_BOTTOM: u32 = 305;

/// **The pre-launch guard.** With a live rate cached, the budget hint paints
/// strictly more foreground in the FORM band than with only the configured
/// rate — the live provenance and the "Refresh rate" affordance are both on
/// screen, above the fold, before anything is launched.
#[test]
fn leaderboard_hint_paints_more_with_a_live_rate() {
    let live =
        ui::fixtures::fake_cockpit_leaderboard_with_live_fx_rate("200", LIVE_RATE, LIVE_AS_OF);
    let config = ui::fixtures::fake_cockpit_leaderboard_with_fx_rate("200", dec!(1.08));

    let (wl, hl, rl) = render_leaderboard(live);
    let (wc, hc, rc) = render_leaderboard(config);
    save("advisor_live_fx_hint_live", wl, hl, &rl);
    save("advisor_live_fx_hint_config", wc, hc, &rc);

    let live_band = foreground_in_band(wl, &rl, FORM_TOP, FORM_BOTTOM);
    let config_band = foreground_in_band(wc, &rc, FORM_TOP, FORM_BOTTOM);

    assert!(
        config_band > 2000,
        "the FORM band must paint the whole form plus its hint (got \
         {config_band} px). PNG: /tmp/advisor_live_fx_hint_config.png"
    );
    assert!(
        live_band > config_band + 100,
        "the live-rate hint must paint more foreground than the config hint \
         (live {live_band} vs config {config_band}) — the live provenance is \
         longer copy and it must be on screen. \
         PNGs: /tmp/advisor_live_fx_hint_{{live,config}}.png"
    );
}

/// **The band's own guard.** The two renders must differ ONLY inside the
/// budget-line band — every other row of the screen is pixel-identical.
///
/// This is what makes the comparison above a measurement of the LABEL rather
/// than of some layout shift the longer string caused elsewhere, and it is the
/// test that fails (with the offending rows named) if a future layout change
/// moves the budget line out of `[SIZING_TOP, SIZING_BOTTOM)` — telling the
/// next reader to re-measure instead of silently weakening the guard.
#[test]
fn only_the_budget_line_band_differs_between_the_two_renders() {
    let (wl, hl, rl) =
        render_forward_plan(ui::fixtures::fake_cockpit_forward_plan_with_fx(live_fx()));
    let (wc, hc, rc) = render_forward_plan(ui::fixtures::fake_cockpit_forward_plan_with_fx(
        FxRate::config(dec!(1.08)),
    ));
    assert_eq!((wl, hl), (wc, hc), "both renders are the same size");

    let mut differing_rows: Vec<u32> = Vec::new();
    for y in 0..hl {
        let live_row = foreground_in_band(wl, &rl, y, y + 1);
        let config_row = foreground_in_band(wc, &rc, y, y + 1);
        if live_row != config_row {
            differing_rows.push(y);
        }
    }

    assert!(
        !differing_rows.is_empty(),
        "the two renders must differ somewhere — if they are identical the live \
         provenance never reached the screen at all"
    );
    let strays: Vec<u32> = differing_rows
        .iter()
        .copied()
        .filter(|y| *y < SIZING_TOP || *y >= SIZING_BOTTOM)
        .collect();
    assert!(
        strays.is_empty(),
        "the live and config renders must differ ONLY in the budget-line band \
         [{SIZING_TOP}, {SIZING_BOTTOM}); rows outside it also changed: {strays:?}. \
         Either the layout moved (re-measure the band) or the provenance leaked \
         into another surface."
    );
}
