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

/// The header band height to EXCLUDE from the table-region scans. The `Run
/// bake-off` button (always `ACCENT`-teal, present in EVERY state — Empty too)
/// lives in the top header at y≈40–70; the recommendation + table start at
/// y≈100. Cropping below the header isolates the crowned-row teal from the
/// always-present button so the discriminator is clean.
const HEADER_BAND_H: u32 = 100;

/// Count `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixels in the BODY region
/// (below the header band). On the leaderboard the only body `ACCENT` source is
/// the crowned row's `★ best` tag + accent strategy text + its 2 px left-rule —
/// so a non-trivial count proves the crowned-row highlight painted. (The bare
/// body has no sidebar, and the teal Run button is excluded by the crop, so
/// there is no other `ACCENT` to confound.)
fn accent_teal_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    for y in HEADER_BAND_H..h {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            // Teal ACCENT: green & blue high and close, red clearly lower (the
            // exact predicate the Reports curve guard uses for #6FB6AE).
            if g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25 {
                hits += 1;
            }
        }
    }
    hits
}

/// Count `DOWN_500`-clay (#C97B5E — R201 G123 B94) pixels across the frame. The
/// Max-drawdown column is ALWAYS negative (`format_pct_max_dd` always paints
/// `DOWN_500`), so a populated table necessarily paints clay — a clean,
/// table-only signal (the empty prompt has no clay). Red is the dominant
/// channel, green mid, blue lowest — clearly distinct from the teal accent and
/// the neutral light text.
fn down_clay_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    for y in HEADER_BAND_H..h {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            // Clay: red clearly dominant over green, green over blue, red high.
            if r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130 {
                hits += 1;
            }
        }
    }
    hits
}

/// Count general foreground (text / marker) pixels — anything that crosses a
/// luma floor the near-black `CANVAS`/`PANEL`/`PANEL_RAISED` tiers never reach.
/// Monotonic in how much content drew: a populated table + recommendation has
/// far more foreground than the single-line empty prompt.
fn foreground_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let mut hits = 0u64;
    for y in 0..h {
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            let luma = (r * 2 + g * 3 + b) / 6;
            if luma > 80 {
                hits += 1;
            }
        }
    }
    hits
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

    let teal = accent_teal_pixels(w, h, &rgba);
    let clay = down_clay_pixels(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // The crowned row's accent treatment (★ best tag + accent id text + the
    // 2 px ACCENT left-rule spanning the row height) paints well over 200 px.
    assert!(
        teal > 200,
        "the crowned row's ACCENT highlight (★ best + accent text + left-rule) \
         must paint (expected >200 teal px, got {teal}). If this fails the \
         crowned-row highlight did not render. PNG: /tmp/leaderboard_populated_render.png"
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
}

/// **Negative control / discriminator.** The SAME harness with
/// `PanelState::Empty` renders the "press Run bake-off" prompt — a single muted
/// line, NO leaderboard table. So there is ~no crowned-row ACCENT teal and ~no
/// Max-drawdown clay, and far less foreground than the populated frame. Proves
/// the populated guard above genuinely discriminates (it is not satisfied by
/// chrome / the header / the disclaimer).
#[test]
fn leaderboard_empty_paints_no_table() {
    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/leaderboard_empty_render.png");
    }

    let teal = accent_teal_pixels(w, h, &rgba);
    let clay = down_clay_pixels(w, h, &rgba);

    // The empty prompt has no crowned row → no ACCENT teal. (The header + the
    // disclaimer are FG_1 / FG_3 / WARN — none teal.) A tiny floor allows for
    // stray antialiasing on the header text.
    assert!(
        teal < 100,
        "the Empty state must NOT paint the crowned-row ACCENT highlight \
         (expected <100 stray teal px, got {teal}). If high, the populated \
         guard is a tautology. PNG: /tmp/leaderboard_empty_render.png"
    );
    // The empty prompt has no table → no Max-drawdown clay column.
    assert!(
        clay < 100,
        "the Empty state must NOT paint the Max-drawdown clay column \
         (expected <100 stray clay px, got {clay}). PNG: /tmp/leaderboard_empty_render.png"
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

    let teal_p = accent_teal_pixels(wp, hp, &rp);
    let teal_e = accent_teal_pixels(we, he, &re);
    let fg_p = foreground_pixels(wp, hp, &rp);
    let fg_e = foreground_pixels(we, he, &re);

    assert!(
        teal_p > teal_e + 150,
        "populated must paint strictly more crowned-row ACCENT than empty \
         (populated {teal_p} vs empty {teal_e})"
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
    let teal = accent_teal_pixels(w, h, &rgba);
    // Recommendation headline + caption + 2-row table + disclaimer = plenty.
    assert!(
        fg > 6000,
        "the benchmark-wins recommendation + table must paint (expected >6000 \
         foreground px, got {fg}). PNG: /tmp/leaderboard_benchmark_wins_render.png"
    );
    // The benchmark row is crowned, so the ★ best tag + accent treatment paints.
    assert!(
        teal > 150,
        "the crowned benchmark row must still paint the ACCENT highlight \
         (expected >150 teal px, got {teal}). PNG: /tmp/leaderboard_benchmark_wins_render.png"
    );
}
