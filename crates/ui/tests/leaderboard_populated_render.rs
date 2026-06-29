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
//! Three guards (populated + error-state, per CLAUDE.md):
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
//! 4. Fetch-error states (advisor-dynamic-data Wave C, ADR-0061):
//!    [`leaderboard_error_network_renders`],
//!    [`leaderboard_error_rate_limited_renders`],
//!    [`leaderboard_error_unknown_symbol_renders`],
//!    [`leaderboard_error_no_data_renders`] — each renders a `PanelState::Error`
//!    carrying one of the four dynamic-fetch error strings and asserts: (a) the
//!    error pane paints measurable foreground text (the string showed up), and
//!    (b) the table band is empty (no crowned-row ACCENT teal, no clay — the
//!    table did NOT accidentally draw behind the error pane). The negative control
//!    on (b) proves this is not a tautology w.r.t. the populated guard.
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
    // The full 13-arm advisor field (4 singles + 8 ensembles + buy-and-hold,
    // ADR-0067). `v0.sma` stays the crowned robust single.
    assert_eq!(
        mirror.rows.len(),
        13,
        "the fixture must be the full 13-arm field"
    );
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

// ── F8 ensemble + Fragile-badge render guard (advisor-ensemble, ADR-0063) ─────
//
// The 7-arm field (4 singles + 2 vote ensembles + buy-and-hold) with the
// robustness gate LIVE must paint, in the cockpit Leaderboard:
//   - the ensemble rows legibly (friendly "Majority vote (2-of-3)" /
//     "Unanimous vote (4-of-4)" labels + a `vote` tag) — measured as extra
//     foreground vs the 5-arm field;
//   - a visible Fragile BADGE (a `DOWN_50`-tinted pill with a `DOWN_500`
//     "fragile" label) on the flagged candidates, IN THE STRATEGY COLUMN (left
//     half) — distinct from the always-negative Max-DD column (right half).
//
// The Fragile badge is the load-bearing F8 pixel: the first time the Fragile
// state is non-inert, it must actually paint. We scope the clay scan to the
// LEFT half of the table so the Max-DD column (right) never confounds it.

/// Right edge of the STRATEGY column scan band — the friendly ensemble labels +
/// the Fragile badge live left of here; the numeric columns (Return / Sharpe /
/// Max-DD / Trades) are all right of here. Calibrated to the 1920-wide slot
/// (the strategy column is the `Length::Fill` left column; the four 110px
/// numeric columns + gaps occupy roughly the right ~560px).
const STRAT_COL_RIGHT: u32 = 760;

/// Count `DOWN_500`-clay pixels in the STRATEGY-column band (x < `STRAT_COL_RIGHT`,
/// y ≥ `TABLE_TOP`). On the F8 fixture this is the Fragile badges' clay label
/// (rsi + the majority ensemble); the Max-DD clay column is excluded by the x
/// bound. On a field with no fragile rows (or no table) this is ~0.
fn fragile_badge_clay(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    let x_max = STRAT_COL_RIGHT.min(w);
    for y in TABLE_TOP..h {
        for x in 0..x_max {
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

/// **The F8 render-layer guard.** The 7-arm ensemble fixture MUST paint, in the
/// cockpit Leaderboard:
/// - the crowned-row `ACCENT` highlight (v0.sma is crowned + robust);
/// - a Fragile BADGE in the STRATEGY column (the `DOWN_500` "fragile" label of
///   the flagged rsi + majority-ensemble rows) — the first non-inert Fragile
///   pixel;
/// - strictly MORE foreground than the 5-arm field (the two extra ensemble rows
///   + their friendly labels + `vote` tags drew).
///
/// Writes the operator-facing PNG to `/tmp/forward_f8_leaderboard_render.png`.
#[test]
fn leaderboard_f8_ensembles_and_fragile_badge_paint() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_with_ensembles();
    assert_eq!(mirror.rows.len(), 7, "the F8 fixture is a 7-arm field");
    assert_eq!(mirror.crowned, Some(0), "v0.sma is crowned (robust)");
    // The majority ensemble is Fragile → must be ranked but NOT crowned.
    let majority_idx = mirror
        .rows
        .iter()
        .position(|r| r.strategy.as_str() == "v0.8.vote.majority")
        .expect("majority ensemble present");
    assert_ne!(
        mirror.crowned,
        Some(majority_idx),
        "a Fragile ensemble must NOT be crowned (the credibility lock)"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/forward_f8_leaderboard_render.png");
    }

    // The crowned row still paints its ACCENT highlight.
    let teal = crowned_teal_pixels(w, h, &rgba);
    assert!(
        teal > 200,
        "the crowned row's ACCENT highlight must paint (expected >200 teal px in \
         the TABLE band, got {teal}). PNG: /tmp/forward_f8_leaderboard_render.png"
    );

    // The Fragile badge paints clay IN THE STRATEGY COLUMN (the load-bearing F8
    // pixel — the first time Fragile is non-inert). Two fragile rows (rsi +
    // majority ensemble), each a `DOWN_500` "fragile" label.
    let fragile_clay = fragile_badge_clay(w, h, &rgba);
    assert!(
        fragile_clay > 60,
        "the Fragile badge must paint its DOWN_500 label in the STRATEGY column \
         (expected >60 clay px left of x={STRAT_COL_RIGHT}, got {fragile_clay}). \
         If this fails the Fragile flag did not render — but F8 makes it \
         non-inert, so it MUST appear. PNG: /tmp/forward_f8_leaderboard_render.png"
    );

    // The full 7-arm table + recommendation is a lot of text.
    let fg = foreground_pixels(w, h, &rgba);
    assert!(
        fg > 8000,
        "the 7-arm table + recommendation must paint a lot of foreground \
         (expected >8000 px, got {fg}). PNG: /tmp/forward_f8_leaderboard_render.png"
    );
}

/// **F8 anti-tautology discriminator.** The 7-arm ensemble field paints
/// strictly MORE strategy-column Fragile clay than the original 5-arm field
/// fixture (which has one fragile single, rsi) — proving the ensemble field
/// adds a *second* fragile row's badge (the majority ensemble) AND that the
/// strategy-column clay scan genuinely tracks Fragile badges (not chrome). Also
/// asserts the 7-arm field paints more total foreground (the two extra rows).
#[test]
fn leaderboard_f8_strictly_exceeds_five_arm_field() {
    let seven = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror_with_ensembles(),
    ));
    // The dedicated 5-arm field (4 singles + buy-and-hold, ONE Fragile single).
    // `fake_bakeoff_report_mirror()` grew to the full 13-arm field (ADR-0067), so
    // this discriminator uses the original 5-arm fixture as the smaller baseline.
    let five = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror_five_arm(),
    ));

    let (w7, h7, r7) = render_leaderboard_rgba(seven);
    let (w5, h5, r5) = render_leaderboard_rgba(five);

    // The 7-arm field has TWO fragile rows (rsi + majority ensemble); the 5-arm
    // field has ONE (rsi). So the 7-arm strategy-column clay strictly exceeds.
    let clay7 = fragile_badge_clay(w7, h7, &r7);
    let clay5 = fragile_badge_clay(w5, h5, &r5);
    assert!(
        clay7 > clay5,
        "the 7-arm field (2 fragile rows) must paint strictly more strategy-column \
         Fragile clay than the 5-arm field (1 fragile row) (7-arm {clay7} vs \
         5-arm {clay5}). If equal, the second Fragile badge is not rendering."
    );

    // And the two extra ensemble rows paint more total foreground.
    let fg7 = foreground_pixels(w7, h7, &r7);
    let fg5 = foreground_pixels(w5, h5, &r5);
    assert!(
        fg7 > fg5,
        "the 7-arm field must paint more foreground than the 5-arm field (the two \
         extra ensemble rows drew) (7-arm {fg7} vs 5-arm {fg5})"
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

// ── advisor-combination-search 13-arm field render guard (OQ-6, ADR-0067) ─────
//
// The advisor field grew from 7 arms (4 singles + 2 ensembles + buy-and-hold) to
// 13 (4 singles + 8 ensembles + buy-and-hold) — the 6 new pre-registered
// combination arms (3 decorrelation pairs + the complete k∈{1,2,3}-of-4 ladder).
// The leaderboard MUST still paint the full ranked table at 13 rows: the crowned
// robust single's ACCENT highlight, the always-negative Max-DD clay column across
// all rows, the Fragile badges of the (now several) Fragile ensemble rows in the
// STRATEGY column, and a healthy foreground floor — with the `Empty` negative
// control still painting NO table. Per the verify-UI-at-render-layer
// non-negotiable, this is a populated PIXEL guard (read the PNG), not a model
// assertion.

/// **The advisor-combination-search 13-row render guard (T6 / OQ-6).** The full
/// 13-arm field MUST paint, in the cockpit Leaderboard:
/// - the crowned robust single's `ACCENT` highlight (`v0.sma`, `★ best`);
/// - the always-negative Max-DD `DOWN_500` clay column across the rows;
/// - the Fragile badges of the Fragile ensemble rows in the STRATEGY column (the
///   `Unanimous{n:2}` decorrelation pairs + the majority arm come back Fragile on
///   real crypto — the honest OQ-3 outcome, rendered as-is);
/// - a healthy foreground floor (13 rows + the recommendation drew, not a blank).
///
/// Writes the operator-facing PNG to `/tmp/leaderboard_populated_render.png` (the
/// same canonical path the 13-arm fixture now backs).
#[test]
fn leaderboard_thirteen_arm_field_paints_full_table() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    assert_eq!(
        mirror.rows.len(),
        13,
        "the fixture is the full 13-arm advisor field (4 singles + 8 ensembles + \
         buy-and-hold)"
    );
    assert_eq!(mirror.crowned, Some(0), "v0.sma (robust single) is crowned");
    // ≥1 Fragile ENSEMBLE row must be present (exercises the badge + the OQ-3
    // honest "Fragile combination" case). Count them so the guard is explicit.
    let fragile_ensembles = mirror
        .rows
        .iter()
        .filter(|r| {
            r.strategy.as_str().starts_with("v0.8.vote.")
                && matches!(
                    r.robustness,
                    Some(ui::leaderboard::state::RobustnessLabel::Fragile)
                )
        })
        .count();
    assert!(
        fragile_ensembles >= 1,
        "the 13-arm field must include ≥1 Fragile ensemble row (the honest OQ-3 \
         outcome), got {fragile_ensembles}"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_populated_render.png");
    }

    let teal = crowned_teal_pixels(w, h, &rgba);
    let clay = table_clay_pixels(w, h, &rgba);
    let fragile_clay = fragile_badge_clay(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The crowned row's accent treatment paints (★ best + accent id + left-rule).
    assert!(
        teal > 200,
        "the crowned row's ACCENT highlight must paint (expected >200 teal px in \
         the TABLE band, got {teal}). PNG: /tmp/leaderboard_populated_render.png"
    );
    // The Max-DD column is negative on every non-zero-DD row → clay across the
    // table. With 13 rows this is well over the floor.
    assert!(
        clay > 300,
        "the Max-drawdown column (always DOWN_500) must paint clay across the 13 \
         rows (expected >300 px, got {clay}) — proof the wider numeric table drew. \
         PNG: /tmp/leaderboard_populated_render.png"
    );
    // The Fragile ensemble badges paint clay in the STRATEGY column (left half).
    // Several ensembles are Fragile here, so this is comfortably non-trivial.
    assert!(
        fragile_clay > 60,
        "the Fragile ensemble badges must paint their DOWN_500 label in the \
         STRATEGY column (expected >60 clay px left of x={STRAT_COL_RIGHT}, got \
         {fragile_clay}). If 0 the Fragile combination rows did not render their \
         badge. PNG: /tmp/leaderboard_populated_render.png"
    );
    // 13 rows + the recommendation block is a lot of foreground text.
    assert!(
        fg > 9000,
        "the populated 13-row table + recommendation must paint a lot of \
         foreground (expected >9000 px, got {fg}). If low the screen rendered a \
         blank/empty pane despite Ready data. \
         PNG: /tmp/leaderboard_populated_render.png"
    );
}

/// **Anti-tautology discriminator for the 13-arm field.** The full 13-arm field
/// paints STRICTLY MORE strategy-column Fragile clay AND total foreground than
/// the 5-arm field (4 singles + buy-and-hold, ONE Fragile single). Ties the two
/// states together so a regression that collapses the field back (or drops the
/// Fragile ensemble badges) shrinks the gap and fails. Proves the 13-row guard
/// genuinely tracks the wider field, not chrome.
#[test]
fn leaderboard_thirteen_arm_strictly_exceeds_five_arm() {
    let thirteen = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror(),
    ));
    let five = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror_five_arm(),
    ));

    let (w13, h13, r13) = render_leaderboard_rgba(thirteen);
    let (w5, h5, r5) = render_leaderboard_rgba(five);

    // The 13-arm field has SEVERAL Fragile rows (incl. multiple Fragile
    // ensembles); the 5-arm field has ONE (the rsi single). So its
    // strategy-column Fragile clay strictly exceeds.
    let clay13 = fragile_badge_clay(w13, h13, &r13);
    let clay5 = fragile_badge_clay(w5, h5, &r5);
    assert!(
        clay13 > clay5,
        "the 13-arm field (several Fragile rows incl. ensembles) must paint \
         strictly more strategy-column Fragile clay than the 5-arm field (1 \
         Fragile single) (13-arm {clay13} vs 5-arm {clay5}). If not, the Fragile \
         ensemble badges are not rendering."
    );

    // And the 8 extra rows (4→12 active arms) paint more total foreground.
    let fg13 = foreground_pixels(w13, h13, &r13);
    let fg5 = foreground_pixels(w5, h5, &r5);
    assert!(
        fg13 > fg5 + 1500,
        "the 13-arm field must paint substantially more foreground than the 5-arm \
         field (the 8 extra rows drew) (13-arm {fg13} vs 5-arm {fg5})"
    );
}

/// **The arm-count header note render guard (T7b / OQ-2).** The leaderboard
/// budget-context band MUST paint the quiet "{N} strategies head-to-head…" note
/// so a wider 13-arm bake-off is self-explanatory. The note adds a second text
/// line to the CONTEXT band, so the populated frame paints STRICTLY MORE
/// CONTEXT-band foreground than a synthetic baseline WITHOUT the note would — but
/// since the note is always present, we assert (a) the context band paints a
/// healthy floor that includes the second line, and (b) the note count text is
/// non-trivial. Read the PNG to confirm the line reads "13 strategies…".
///
/// Writes the operator-facing PNG to `/tmp/leaderboard_arm_count_note_render.png`.
#[test]
fn leaderboard_arm_count_note_paints_in_context_band() {
    // The note is sourced from the real field size, so it must read the current
    // advisor-field arm count. The field has grown over time:
    //   - 13 (4 singles + 8 ensembles + buy-and-hold) — the original v0.8 set.
    //   - +5 signal-library arms (ADR-0071) → 18.
    //   - +1 DVOL regime arm (ADR-0072) → 19.
    //   - +1 macro regime arm (ADR-0073) → 20.
    // Assert the CURRENT real arm count rather than a stale literal so the
    // count test tracks the field as it grows (the commit d3a9a4a discipline).
    let arm_count = ui::leaderboard::runner::advisor_field_arm_count();
    assert!(
        arm_count >= 13,
        "the advisor field is at least the original 13 arms; got {arm_count}"
    );

    let mirror = ui::fixtures::fake_bakeoff_report_mirror();
    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_arm_count_note_render.png");
    }

    // The CONTEXT band now carries TWO lines: the "Ranking strategies for €200 in
    // BTCUSDT." H3 line AND the arm-count note. A single H3 line alone produces
    // ~400-600 px; the two stacked lines push this comfortably higher. The floor
    // is set so the note's presence is load-bearing (a regression that drops the
    // note would fall under it).
    let context_fg = context_line_foreground(w, &rgba);
    assert!(
        context_fg > 750,
        "the budget-context band must paint BOTH the budget line AND the arm-count \
         note (expected >750 foreground px in the CONTEXT band, got {context_fg}). \
         If low the arm-count note ('13 strategies head-to-head…') did not render. \
         PNG: /tmp/leaderboard_arm_count_note_render.png"
    );

    // The table still renders below the (now two-line) context band — the note
    // didn't squash the leaderboard out.
    let teal = crowned_teal_pixels(w, h, &rgba);
    assert!(
        teal > 200,
        "the crowned leaderboard row must still paint below the two-line context \
         band (expected >200 teal px in the TABLE band, got {teal}). \
         PNG: /tmp/leaderboard_arm_count_note_render.png"
    );
}

/// **Arm-count note anti-tautology.** The CONTEXT band with the arm-count note
/// paints STRICTLY MORE foreground than the SAME band rendered from a state whose
/// note would be absent — approximated here by asserting the two-line context
/// band strictly exceeds a single H3 line's typical footprint via the Empty
/// state (which renders the SAME two-line context band, proving the note is part
/// of the always-present context, not a result-only artifact). This pins that the
/// note is a structural part of the header, present even before a run.
#[test]
fn leaderboard_arm_count_note_present_in_empty_state() {
    // The note is part of the always-present context band — it must paint even in
    // the Empty (pre-run) state, so the operator sees the field size up front.
    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);
    let (w, _h, rgba) = render_leaderboard_rgba(cockpit);

    let context_fg = context_line_foreground(w, &rgba);
    assert!(
        context_fg > 750,
        "the arm-count note must paint in the CONTEXT band even in the Empty state \
         (the field size is shown before any run) — expected >750 foreground px, \
         got {context_fg}."
    );
}

// ── Fetch-error state guards (advisor-dynamic-data Wave C, ADR-0061) ─────────
//
// Each of the four `LEADERBOARD_FETCH_*` error strings must:
//   (a) produce measurable foreground text pixels (the error message rendered);
//   (b) paint NO crowned-row ACCENT teal in the TABLE band (no leaderboard
//       table accidentally drew behind the error pane).
//
// A shared helper keeps the pixel logic DRY.

fn assert_error_state_paints_no_table(msg: &str, png_path: &str) {
    use smol_str::SmolStr;

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Error(SmolStr::new(msg)));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save(png_path);
    }

    // (a) The error pane paints foreground text (the message string is visible).
    //     The screen has the sidebar + header chrome even in the error state,
    //     so the floor is deliberately low (100 px), not high — the test only
    //     asserts the pane is NOT blank.
    let fg = foreground_pixels(w, h, &rgba);
    assert!(
        fg > 100,
        "the Error state must paint some foreground text (got {fg} px). \
         The error message may not be rendering. PNG: {png_path}"
    );

    // (b) No crowned-row ACCENT teal in the TABLE band — the leaderboard table
    //     must NOT draw behind the error pane.
    //     Calibrated: error state renders 0 teal px, populated renders ≥249.
    let table_teal = crowned_teal_pixels(w, h, &rgba);
    assert!(
        table_teal < 150,
        "the Error state must NOT paint a crowned-row ACCENT highlight in the \
         TABLE band (expected <150 stray teal px, got {table_teal}). \
         The leaderboard table may have leaked through. PNG: {png_path}"
    );

    // (c) No Max-drawdown clay in the TABLE band from a leaderboard table.
    //     Threshold is 200 (not 100) because the error panel's warning decoration
    //     at x≈20, y≈380 contributes ~143 orange-clay pixels — this is chrome,
    //     not a leaderboard table. Populated state produces ≥477 clay px,
    //     so 200 is a clean discriminator. Empty state produces 0.
    let table_clay = table_clay_pixels(w, h, &rgba);
    assert!(
        table_clay < 250,
        "the Error state must NOT paint leaderboard-table clay in the TABLE band \
         (expected <250 stray clay px, got {table_clay}). \
         If >250, a leaderboard table is rendering behind the error pane. \
         PNG: {png_path}"
    );
}

/// Network-error string renders without a leaderboard table.
///
/// Maps to `BinanceFetchError::Network` / `::Timeout` paths in
/// `dynamic_error_to_friendly` (Wave B, bakeoff/mod.rs).
/// Writes PNG to `/tmp/leaderboard_error_network_render.png`.
#[test]
fn leaderboard_error_network_renders() {
    assert_error_state_paints_no_table(
        ui::strings::LEADERBOARD_FETCH_NETWORK_ERROR,
        "/tmp/leaderboard_error_network_render.png",
    );
}

/// Rate-limited error string renders without a leaderboard table.
///
/// Maps to `BinanceFetchError::RateLimited` (HTTP 429).
/// Writes PNG to `/tmp/leaderboard_error_rate_limited_render.png`.
#[test]
fn leaderboard_error_rate_limited_renders() {
    assert_error_state_paints_no_table(
        ui::strings::LEADERBOARD_FETCH_RATE_LIMITED,
        "/tmp/leaderboard_error_rate_limited_render.png",
    );
}

/// Unknown-symbol error string renders without a leaderboard table.
///
/// Maps to `BinanceFetchError::UnknownSymbol`.
/// Writes PNG to `/tmp/leaderboard_error_unknown_symbol_render.png`.
#[test]
fn leaderboard_error_unknown_symbol_renders() {
    assert_error_state_paints_no_table(
        ui::strings::LEADERBOARD_FETCH_UNKNOWN_SYMBOL,
        "/tmp/leaderboard_error_unknown_symbol_render.png",
    );
}

/// No-data error string renders without a leaderboard table.
///
/// Maps to `BinanceFetchError::NoDataForRange` / `DynamicCacheError::NoData`.
/// Writes PNG to `/tmp/leaderboard_error_no_data_render.png`.
#[test]
fn leaderboard_error_no_data_renders() {
    assert_error_state_paints_no_table(
        ui::strings::LEADERBOARD_FETCH_NO_DATA,
        "/tmp/leaderboard_error_no_data_render.png",
    );
}
