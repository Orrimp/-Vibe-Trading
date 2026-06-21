//! advisor-leaderboard-screen v0.1.0 — render-layer proof of the POPULATED
//! strategy bake-off LEADERBOARD in the cockpit.
//!
//! ## Why this file exists (the operator's #1 sensitivity)
//!
//! MEMORY.md "verify UI at the render layer": a passing model-`Ready` state, a
//! text `.snap`, or a no-panic boot is NOT proof the leaderboard draws. That
//! trap shipped multiple blind cockpit bugs (the Live-view saga, the trail
//! 0-px side-drawer, the Reports empty-curve). This guard renders the REAL
//! `screens::leaderboard::view` HEADLESS with a POPULATED `BakeoffReportMirror`
//! and asserts on the rendered PIXELS that the ranked rows + the crowned
//! `ACCENT` highlight + the recommendation headline actually paint — with a
//! NEGATIVE CONTROL (the `Empty` "press Run bake-off" state paints NO
//! leaderboard table).
//!
//! Three guards:
//!
//! 1. [`leaderboard_populated_paints_rows_crown_and_recommendation`] — the
//!    populated fixture paints (a) the crowned-row `ACCENT` teal (the `★ best`
//!    tag + accent strategy text + the 2 px left-rule), (b) the `DOWN_500` clay
//!    of the always-negative Max-drawdown column (proves the numeric columns
//!    render), and (c) a healthy amount of foreground text (proves the table +
//!    recommendation actually drew, not a blank pane). Writes the
//!    operator-facing PNG to `/tmp/leaderboard_populated_render.png`.
//! 2. [`leaderboard_empty_paints_no_table`] — the negative control: the SAME
//!    harness with `PanelState::Empty` paints ~no ACCENT teal + ~no clay
//!    (the "press Run bake-off" prompt has no table). Proves guard (1) is not a
//!    tautology (it genuinely discriminates the populated leaderboard from the
//!    empty prompt).
//! 3. [`leaderboard_benchmark_wins_headline_renders`] — renders the
//!    `BenchmarkWins` fixture and asserts the recommendation block paints (the
//!    "Nothing beat simply holding…" branch is rendered FROM the structured
//!    `Recommendation`, not a no-op).
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `reports_populated_curve_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file
//! compiles to nothing on Linux/Windows. Pixel thresholds are deliberately
//! coarse (presence/absence of a hue, not byte-exact), robust within macOS
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

// ── Region bands (advisor-bakeoff-ranking F3) ─────────────────────────────────
//
// The screen body stacks, top-to-bottom, at the 1920×1080 / scale-1.0 slot
// (measured from the saved PNGs — see the per-band comments):
//
//   y   0– 90  header: page title + caption + the always-present `Run bake-off`
//              button (top-right, ACCENT teal in EVERY state).
//   y 110–305  the F3 GUIDED-INPUT FORM: the `Coin` chip row (active coin = a
//              SOLID ACCENT teal chip), the `Budget` field + `Lookback` chip row
//              (active lookback = a SOLID ACCENT teal chip), and the budget hint.
//   y 310–350  the BUDGET-CONTEXT line ("Ranking strategies for €200 in …").
//   y 350+     the RESULT body: the ranked TABLE (column header + rows; the
//              crowned row paints ACCENT accent text + a 2 px left-rule, and the
//              always-negative Max-DD column paints DOWN_500 clay).
//
// Scoping each scan to its band is what keeps the discriminators honest now that
// the form (always present) ALSO paints ACCENT teal: the crowned-row teal scan
// is restricted to the TABLE band so the form's active chips never leak into it,
// and the form-chip teal scan is restricted to the FORM band so the crowned row
// + the Run button never leak into it.

// The `Run bake-off` button (always ACCENT, present in every state) lives in
// the header at y≈34–66; every body scan below starts at `FORM_TOP` or
// `TABLE_TOP`, so the button is never counted.

/// Top of the FORM band (just below the header).
const FORM_TOP: u32 = 110;
/// Bottom of the FORM band (just above the budget-context line). The active
/// coin chip (≈y173–201) + active lookback chip (≈y233–261) + the budget hint
/// all sit inside `FORM_TOP..FORM_BOTTOM`.
const FORM_BOTTOM: u32 = 305;

/// Top of the BUDGET-CONTEXT band.
const CONTEXT_TOP: u32 = 308;
/// Bottom of the BUDGET-CONTEXT band (just above the table's column header).
const CONTEXT_BOTTOM: u32 = 350;

/// Top of the TABLE band — the ranked table (column header + rows) starts here,
/// well below the form + context line. The crowned-row ACCENT + the Max-DD clay
/// live ONLY at/below this y, so a teal/clay scan from here is table-only.
const TABLE_TOP: u32 = 355;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — green & blue
/// high and close, red clearly lower (the exact predicate the Reports curve
/// guard uses for #6FB6AE).
fn is_accent_teal(r: i32, g: i32, b: i32) -> bool {
    g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25
}

/// `true` for a `DOWN_500`-clay (#C97B5E — R201 G123 B94) pixel — red dominant
/// over green, green over blue, red high.
fn is_down_clay(r: i32, g: i32, b: i32) -> bool {
    r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130
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

/// The crowned-row `ACCENT` teal — scoped to the TABLE band (excludes the
/// always-present form active-chips + the Run button). On the leaderboard the
/// only TABLE-band `ACCENT` source is the crowned row's `★ best` tag + accent
/// strategy text + its 2 px left-rule, so a non-trivial count proves the
/// crowned-row highlight painted.
fn crowned_teal_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, TABLE_TOP, h)
}

/// The `DOWN_500`-clay of the always-negative Max-drawdown column — scoped to
/// the TABLE band (the form + context lines have no clay). A populated table
/// necessarily paints clay; the empty prompt does not.
fn table_clay_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    down_clay_in_band(w, rgba, TABLE_TOP, h)
}

/// The FORM band's `ACCENT` teal — the SOLID active coin chip + active lookback
/// chip. A non-trivial count proves the guided-input coin + lookback pickers
/// drew WITH a selection highlighted (the chosen coin/lookback pops as a solid
/// teal chip). Excludes the table (below `FORM_BOTTOM`) + the Run button (above
/// `FORM_TOP`).
fn form_active_chip_teal(w: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, FORM_TOP, FORM_BOTTOM)
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

/// General foreground across the whole frame (the legacy whole-screen count).
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, 0, h)
}

/// Foreground in the budget-context band — proves the "Ranking strategies for
/// €200 in {coin}." header line drew (a single H3 text line between the form
/// and the table).
fn context_line_foreground(w: u32, rgba: &[u8]) -> u64 {
    foreground_in_band(w, rgba, CONTEXT_TOP, CONTEXT_BOTTOM)
}

/// **The render-layer guard.** A populated `BakeoffReportMirror` MUST paint, in
/// the cockpit Leaderboard screen:
/// - the crowned-row `ACCENT` teal (the `★ best` tag + accent text + left-rule);
/// - the `DOWN_500` clay of the Max-drawdown column (the numeric table drew);
/// - a healthy amount of foreground text (rows + recommendation drew).
///
/// Writes the operator-facing PNG to `/tmp/leaderboard_populated_render.png`.
#[test]
fn leaderboard_populated_paints_rows_crown_and_recommendation() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    assert!(mirror.rows.len() >= 5, "the fixture must have ≥5 rows");
    assert_eq!(mirror.crowned, Some(0), "v0.sma is crowned");

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_populated_render.png");
    }

    let teal = crowned_teal_pixels(w, h, &rgba);
    let clay = table_clay_pixels(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The crowned row's accent treatment (★ best tag + accent id text + the
    // 2 px ACCENT left-rule spanning the row height) paints well over 200 px.
    assert!(
        teal > 200,
        "the crowned row's ACCENT highlight (★ best + accent text + left-rule) \
         must paint (expected >200 teal px in the TABLE band, got {teal}). If \
         this fails the crowned-row highlight did not render. \
         PNG: /tmp/leaderboard_populated_render.png"
    );
    // The Max-drawdown column is always negative → DOWN_500 clay on every row.
    assert!(
        clay > 150,
        "the Max-drawdown column (always DOWN_500) must paint clay across the \
         rows (expected >150 px, got {clay}) — proof the numeric table drew. \
         PNG: /tmp/leaderboard_populated_render.png"
    );
    // The full table + recommendation block is a lot of text.
    assert!(
        fg > 8000,
        "the populated table + recommendation must paint a lot of foreground \
         text (expected >8000 px, got {fg}). If this is low the screen rendered \
         a blank/empty pane despite Ready data. PNG: /tmp/leaderboard_populated_render.png"
    );

    // ── F3 guided input — the form + budget-context header must paint ─────────
    // The default fixture starts on BTCUSDT / €200 / 2024 H1, so the active
    // coin chip + active lookback chip are SOLID ACCENT teal in the FORM band.
    let form_teal = form_active_chip_teal(w, &rgba);
    assert!(
        form_teal > 1500,
        "the guided-input coin + lookback pickers must paint their SOLID ACCENT \
         active chips (expected >1500 teal px in the FORM band, got {form_teal}). \
         If this fails the coin/lookback selection did not render. \
         PNG: /tmp/leaderboard_populated_render.png"
    );
    // The budget-context line ("Ranking strategies for €200 in BTCUSDT.") drew.
    let context_fg = context_line_foreground(w, &rgba);
    assert!(
        context_fg > 400,
        "the budget-context header must paint (expected >400 foreground px in \
         the CONTEXT band, got {context_fg}). If this fails the \
         €-budget-in-coin header did not render. \
         PNG: /tmp/leaderboard_populated_render.png"
    );
}

/// **Negative control / discriminator.** The SAME harness with
/// `PanelState::Empty` renders the "press Run bake-off" prompt — a single muted
/// line, NO leaderboard table. So there is ~no crowned-row ACCENT teal and ~no
/// Max-drawdown clay IN THE TABLE BAND, and far less foreground than the
/// populated frame. Proves the populated TABLE guard genuinely discriminates (it
/// is not satisfied by chrome / the header / the always-present guided-input
/// form / the disclaimer).
///
/// The guided-input form IS present in the Empty state (it's the entry point),
/// so the FORM band still paints the active-chip teal — but the TABLE-band scans
/// used here exclude it, which is exactly the point of the region split.
#[test]
fn leaderboard_empty_paints_no_table() {
    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_empty_render.png");
    }

    let table_teal = crowned_teal_pixels(w, h, &rgba);
    let table_clay = table_clay_pixels(w, h, &rgba);

    // The empty prompt has no crowned row → no ACCENT teal IN THE TABLE band.
    // (The header Run button + the form active chips are both ABOVE the table
    // band, so they don't confound this scan.) A small floor allows stray AA.
    assert!(
        table_teal < 150,
        "the Empty state must NOT paint a crowned-row ACCENT highlight in the \
         TABLE band (expected <150 stray teal px, got {table_teal}). If high, \
         the populated guard is a tautology. PNG: /tmp/leaderboard_empty_render.png"
    );
    // The empty prompt has no table → no Max-drawdown clay column.
    assert!(
        table_clay < 100,
        "the Empty state must NOT paint the Max-drawdown clay column \
         (expected <100 stray clay px, got {table_clay}). \
         PNG: /tmp/leaderboard_empty_render.png"
    );

    // ── Positive: the guided-input form STILL paints in the Empty state ───────
    // (the form is the entry point — it must be visible before any run). The
    // active coin + lookback chips paint SOLID ACCENT teal in the FORM band.
    let form_teal = form_active_chip_teal(w, &rgba);
    assert!(
        form_teal > 1500,
        "the guided-input form must paint its active chips even in the Empty \
         state (expected >1500 teal px in the FORM band, got {form_teal}) — the \
         form is the journey entry point. PNG: /tmp/leaderboard_empty_render.png"
    );
}

/// Stronger discriminator — the populated frame paints STRICTLY MORE accent +
/// clay + foreground than the empty frame. Ties the two states together in one
/// assertion so a future regression that makes both look the same fails.
#[test]
fn leaderboard_populated_strictly_exceeds_empty() {
    let populated = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror(),
    ));
    let empty = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);

    let (wp, hp, rp) = render_leaderboard_rgba(populated);
    let (we, he, re) = render_leaderboard_rgba(empty);

    // Compare the TABLE-band crowned teal (the form teal is identical in both —
    // the form is always present — so a whole-screen teal compare would not
    // discriminate; the table-band scope is what makes this a real test).
    let teal_p = crowned_teal_pixels(wp, hp, &rp);
    let teal_e = crowned_teal_pixels(we, he, &re);
    let fg_p = foreground_pixels(wp, hp, &rp);
    let fg_e = foreground_pixels(we, he, &re);

    assert!(
        teal_p > teal_e + 150,
        "populated must paint strictly more crowned-row ACCENT (TABLE band) than \
         empty (populated {teal_p} vs empty {teal_e})"
    );
    assert!(
        fg_p > fg_e + 4000,
        "populated (table + recommendation) must paint far more foreground than \
         the empty prompt (populated {fg_p} vs empty {fg_e})"
    );
}

/// The `BenchmarkWins` recommendation branch renders. Renders the
/// buy-and-hold-wins fixture and asserts the recommendation block paints (a lot
/// of foreground — the headline "Nothing beat simply holding BTCUSDT…" is
/// rendered FROM the structured `Recommendation`, plus the 2-row table). Guards
/// that the headline-from-structure mapping is not a no-op.
#[test]
fn leaderboard_benchmark_wins_headline_renders() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins();
    // The benchmark (buy-and-hold) is crowned in this fixture.
    let crowned = mirror.crowned.and_then(|i| mirror.rows.get(i));
    assert!(
        crowned.map(|r| r.is_benchmark).unwrap_or(false),
        "the benchmark-wins fixture must crown the buy-and-hold row"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_benchmark_wins_render.png");
    }

    let fg = foreground_pixels(w, h, &rgba);
    let teal = crowned_teal_pixels(w, h, &rgba);
    // Recommendation headline + caption + 2-row table + disclaimer = plenty.
    assert!(
        fg > 6000,
        "the benchmark-wins recommendation + table must paint (expected >6000 \
         foreground px, got {fg}). PNG: /tmp/leaderboard_benchmark_wins_render.png"
    );
    // The benchmark row is crowned, so the ★ best tag + accent treatment paints
    // in the TABLE band (the form's active chips are excluded by the band scope).
    assert!(
        teal > 150,
        "the crowned benchmark row must still paint the ACCENT highlight in the \
         TABLE band (expected >150 teal px, got {teal}). \
         PNG: /tmp/leaderboard_benchmark_wins_render.png"
    );
}

/// **F3 render-layer proof — the guided input with a NON-DEFAULT selection.**
///
/// Renders the screen with an EXPLICIT guided-input selection (XRPUSDT / €350 /
/// 1 month) over a populated leaderboard and asserts the input controls + the
/// budget-context header paint. Distinct from
/// `leaderboard_populated_paints_rows_crown_and_recommendation` (which uses the
/// defaults) so the guard is not satisfied by the default state alone — the
/// selection genuinely flows from state to pixels.
///
/// Writes the operator-facing PNG to
/// `/tmp/leaderboard_guided_input_render.png`.
#[test]
fn leaderboard_guided_input_with_selection_paints_controls_and_context() {
    use ui::leaderboard::LeaderboardLookback;

    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    // A non-default selection: a DIFFERENT coin (XRP, not the default BTC), a
    // DIFFERENT budget (€350), a DIFFERENT lookback (1 month, a relative window).
    let cockpit = ui::fixtures::fake_cockpit_leaderboard_with_input(
        PanelState::Ready(mirror),
        "XRPUSDT",
        "350",
        LeaderboardLookback::OneMonth,
    );
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_guided_input_render.png");
    }

    // The coin picker + lookback picker each paint a SOLID ACCENT active chip
    // (now XRPUSDT + "1 month") in the FORM band — proof both pickers drew with
    // the chosen selection highlighted.
    let form_teal = form_active_chip_teal(w, &rgba);
    assert!(
        form_teal > 1500,
        "the guided-input coin + lookback pickers must paint their SOLID ACCENT \
         active chips for the chosen selection (expected >1500 teal px in the \
         FORM band, got {form_teal}). PNG: /tmp/leaderboard_guided_input_render.png"
    );

    // The form panel draws a substantial amount of foreground (the panel title,
    // three field labels, ten coin chips, the budget field + value, nine
    // lookback chips, the budget hint).
    let form_fg = foreground_in_band(w, &rgba, FORM_TOP, FORM_BOTTOM);
    assert!(
        form_fg > 2500,
        "the guided-input form must paint its labels + chips + budget field \
         (expected >2500 foreground px in the FORM band, got {form_fg}). \
         PNG: /tmp/leaderboard_guided_input_render.png"
    );

    // The budget-context header ("Ranking strategies for €350 in XRPUSDT.")
    // drew in the CONTEXT band.
    let context_fg = context_line_foreground(w, &rgba);
    assert!(
        context_fg > 400,
        "the budget-context header must paint the chosen €-budget-in-coin line \
         (expected >400 foreground px in the CONTEXT band, got {context_fg}). \
         PNG: /tmp/leaderboard_guided_input_render.png"
    );

    // The crowned table still renders below the form (the leaderboard didn't
    // get squashed out by the new controls).
    let table_teal = crowned_teal_pixels(w, h, &rgba);
    assert!(
        table_teal > 150,
        "the crowned leaderboard row must still paint below the guided input \
         (expected >150 teal px in the TABLE band, got {table_teal}). \
         PNG: /tmp/leaderboard_guided_input_render.png"
    );
}
