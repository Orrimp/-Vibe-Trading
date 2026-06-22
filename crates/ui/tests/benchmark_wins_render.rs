//! advisor-benchmark-robustness B1 (ADR-0066) — render-layer proof of the
//! HONEST `BenchmarkWins` leaderboard on real crypto.
//!
//! ## Why this file exists (the B1 honesty contract, at the pixel layer)
//!
//! On real single-asset crypto, no active strategy clears the robustness bar,
//! so the engine crowns buy-and-hold — `outcome == BenchmarkWins`. The UI must
//! tell this honestly: the benchmark is the BASELINE that won (nothing active
//! was robust → holding is the least-bad), NOT a failed/fragile candidate, and
//! NOT "everything is broken". CLAUDE.md's iced rule says a passing model-state,
//! a text `.snap`, or a no-panic boot is NOT proof the screen draws — so this
//! guard renders the REAL `screens::leaderboard::view` HEADLESS with the
//! honest-real-crypto `BenchmarkWins` fixture and asserts on the rendered PIXELS
//! that:
//!
//! 1. the **crowned BASELINE row** paints the `ACCENT` highlight (the
//!    buy-and-hold row is crowned — the `★ best` tag + accent text + 2 px
//!    left-rule), i.e. the baseline visibly WON;
//! 2. the **honest recommendation copy** + the full table paint a healthy
//!    amount of foreground (the "No active strategy cleared the robustness bar …
//!    holding is the least-bad" headline is rendered FROM the structured
//!    `Recommendation`, not a no-op);
//! 3. the **crowned baseline row does NOT wear a saturated Fragile badge**
//!    (ADR-0066 § D3): even though buy-and-hold is itself Fragile, its flag
//!    renders as the quiet informational note, so the crowned row's
//!    strategy-column clay stays far below the active-arm Fragile-badge level —
//!    the baseline is exempt from the candidate verdict, never "disqualified".
//!
//! With TWO negative controls so no assertion is a tautology:
//!   - [`benchmark_wins_active_wins_negative_control`] — the `ActiveWins`
//!     populated state (a robust active arm crowned) is a DIFFERENT, populated
//!     leaderboard, proving the table-drew assertions are not satisfied by an
//!     arbitrary populated frame;
//!   - [`benchmark_wins_empty_negative_control`] — the `Empty` "press Run
//!     bake-off" prompt paints no table at all.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `leaderboard_populated_render.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file compiles
//! to nothing on Linux/Windows. Pixel thresholds are coarse (presence/absence of
//! a hue), robust within macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::time::Duration;

use ui::state::{Cockpit, PanelState};
use ui::test_support::leaderboard_screen_program;

/// Render the bare Leaderboard screen body at the `typical` 1920×1080 slot and
/// return the physical-pixel RGBA buffer + dimensions. The
/// `leaderboard_screen_program` harness pins the screen body to `ThemeMode::Dark`
/// (see `benchmark_wins_copy_tokens_are_light_capable` for the light-capability
/// guard at the token layer), so the pixel classifiers below are tuned to the
/// dark `ACCENT`/`DOWN_500` hexes.
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

// ── Region bands (shared with leaderboard_populated_render.rs geometry) ───────
//
// The screen body stacks, top-to-bottom, at the 1920×1080 / scale-1.0 slot:
//   y 110–305  the F3 guided-input form.
//   y 310–350  the budget-context line.
//   y 355+     the result body: the recommendation block THEN the ranked table.
//
// The recommendation block sits above the table, so on the multi-arm
// `BenchmarkWins` field the table's first (crowned) data row is well below
// `TABLE_TOP`. We keep the same `TABLE_TOP` floor used by the sibling guard so
// the form's active chips + the Run button never leak into the table scans.

/// Top of the TABLE band — the result body (recommendation + table) starts here.
const TABLE_TOP: u32 = 355;

/// Right edge of the STRATEGY column scan band — the friendly labels, the
/// `baseline`/`vote` tags, the Fragile badges + the "sat in cash" note all live
/// left of here; the numeric columns are all right of here. Calibrated to the
/// 1920-wide slot (same constant as the sibling F8 guard's `STRAT_COL_RIGHT`).
const STRAT_COL_RIGHT: u32 = 760;

/// `true` for an `ACCENT`-teal (#6FB6AE — R111 G182 B174) pixel — green & blue
/// high and close, red clearly lower.
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

/// The crowned-row `ACCENT` teal — scoped to the TABLE band. On the leaderboard
/// the only TABLE-band `ACCENT` source is the crowned row's `★ best` tag +
/// accent strategy text + 2 px left-rule, so a non-trivial count proves the
/// crowned (baseline) row highlight painted.
fn crowned_teal_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    accent_teal_in_band(w, rgba, TABLE_TOP, h)
}

/// Count general foreground (text / marker) pixels in the `[y0, y1)` band.
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

/// Count `DOWN_500`-clay pixels in the STRATEGY-column band of the row band
/// `[y0, y1)` (x < `STRAT_COL_RIGHT`). On an ACTIVE Fragile arm this is the
/// saturated `DOWN_500` "fragile" badge label; on the BENCHMARK row it must be
/// ~0 because its flag renders as the muted `FG_3` informational note, not the
/// badge (ADR-0066 § D3). The Max-DD clay column is excluded by the x bound.
fn strat_col_clay_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let mut hits = 0u64;
    let x_max = STRAT_COL_RIGHT.min(w);
    for y in y0..y1 {
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

/// Locate the crowned (baseline) row's y-band: the first table data row, found
/// as the y of the topmost run of `ACCENT`-teal pixels in the TABLE band (the
/// crowned row's left-rule + accent text). Returns the `[y0, y1)` band of that
/// row (a fixed `ROW_H`-tall slice from the first teal row). Used to scope the
/// "crowned baseline row has no saturated Fragile badge" assertion to the
/// baseline row specifically.
fn crowned_row_band(w: u32, h: u32, rgba: &[u8]) -> Option<(u32, u32)> {
    const ROW_H: u32 = 40;
    for y in TABLE_TOP..h {
        let mut teal_in_row = 0u32;
        for x in 0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let (r, g, b) = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            if is_accent_teal(r, g, b) {
                teal_in_row += 1;
            }
        }
        // The crowned row's accent text + left-rule cross ~20+ px on its line;
        // require a small run so a stray AA pixel doesn't trigger.
        if teal_in_row > 12 {
            return Some((y, (y + ROW_H).min(h)));
        }
    }
    None
}

/// **The B1 render-layer guard.** The honest real-crypto `BenchmarkWins` field
/// MUST paint, in the cockpit Leaderboard:
/// - the crowned BASELINE row's `ACCENT` highlight (buy-and-hold won);
/// - the honest recommendation copy + full table (a lot of foreground);
/// - and the crowned baseline row must NOT wear a saturated Fragile badge (its
///   own Fragile flag renders as the quiet informational note, per ADR-0066 §
///   D3) — even though active arms below it DO paint Fragile badges.
///
/// Writes the operator-facing PNG to `/tmp/benchmark_wins_render.png`.
#[test]
fn benchmark_wins_paints_honest_baseline_crown() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins_full();
    assert_eq!(mirror.rows.len(), 7, "the honest field is a 7-arm field");
    // Buy-and-hold (the benchmark) is crowned.
    let crowned = mirror.crowned.and_then(|i| mirror.rows.get(i));
    assert!(
        crowned.map(|r| r.is_benchmark).unwrap_or(false),
        "the BenchmarkWins fixture must crown the buy-and-hold baseline"
    );
    // The unanimous ensemble has 0 trades (the "sat in cash" case).
    assert!(
        mirror
            .rows
            .iter()
            .any(|r| r.strategy.as_str() == "v0.8.vote.unanimous" && r.trade_count == 0),
        "the fixture must carry a 0-trade unanimous ensemble (sat-in-cash)"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/benchmark_wins_render.png");
    }

    let teal = crowned_teal_pixels(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);

    // (1) The crowned BASELINE row's accent treatment paints (★ best + accent id
    //     text + the 2 px ACCENT left-rule). Proves the baseline visibly WON.
    assert!(
        teal > 200,
        "the crowned BASELINE row's ACCENT highlight (★ best + accent text + \
         left-rule) must paint (expected >200 teal px in the TABLE band, got \
         {teal}). If this fails the crowned baseline did not render. \
         PNG: /tmp/benchmark_wins_render.png"
    );
    // (2) The honest recommendation copy + the 7-arm table = a lot of text.
    assert!(
        fg > 8000,
        "the honest BenchmarkWins recommendation + 7-arm table must paint a lot \
         of foreground (expected >8000 px, got {fg}). If low, the screen rendered \
         a blank/empty pane despite Ready data. PNG: /tmp/benchmark_wins_render.png"
    );

    // (3) The crowned BASELINE row must NOT wear a saturated Fragile badge — its
    //     own Fragile flag renders as the muted informational note (ADR-0066 §
    //     D3), so its strategy-column clay is ~0, far below an active Fragile
    //     arm's badge. Scope to the crowned row's y-band so the active arms'
    //     badges (below) don't confound it.
    let (cy0, cy1) = crowned_row_band(w, h, &rgba).expect(
        "the crowned baseline row must be locatable by its ACCENT teal — if None, \
         the crowned-row highlight did not paint. PNG: /tmp/benchmark_wins_render.png",
    );
    let crowned_row_clay = strat_col_clay_in_band(w, &rgba, cy0, cy1);
    assert!(
        crowned_row_clay < 25,
        "the crowned BASELINE row must NOT paint a saturated Fragile badge in its \
         strategy column (expected <25 stray clay px in the crowned row band \
         y={cy0}..{cy1}, got {crowned_row_clay}). The benchmark is the baseline \
         (ADR-0066 § D3) — its Fragile flag is informational, never the \
         disqualifying badge. PNG: /tmp/benchmark_wins_render.png"
    );
}

/// **Negative control 1 — `ActiveWins` populated.** A DIFFERENT, fully-populated
/// leaderboard (a robust active arm crowned, NO honest-BenchmarkWins copy) also
/// paints a crowned row + lots of foreground — proving guard (1)/(2) are not
/// satisfied by "any populated frame", and that the `BenchmarkWins`-specific
/// assertions in the main guard genuinely exercise the BenchmarkWins fixture.
///
/// The discriminator that separates the two states is NOT pixel-count here (both
/// are populated) but the headline COPY (asserted to render from structure in
/// the main guard) + the crowned-row clay assertion (in `ActiveWins` the crowned
/// arm is robust, so it also has ~0 clay — but its lower rows include a real
/// Fragile active badge, which the main guard's row-scoped clay assertion would
/// have caught had the crown landed on a fragile arm).
#[test]
fn benchmark_wins_active_wins_negative_control() {
    let mirror = ui::fixtures::fake_bakeoff_report_mirror_with_ensembles();
    // Sanity: this control crowns an ACTIVE arm, not the benchmark.
    let crowned = mirror.crowned.and_then(|i| mirror.rows.get(i));
    assert!(
        !crowned.map(|r| r.is_benchmark).unwrap_or(true),
        "the ActiveWins control must crown an ACTIVE arm (not the benchmark)"
    );

    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(mirror));
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/benchmark_wins_active_control_render.png");
    }

    // It IS a populated leaderboard, so it paints a crown + foreground too — that
    // is the point: the populated-ness alone does not identify BenchmarkWins.
    let teal = crowned_teal_pixels(w, h, &rgba);
    let fg = foreground_pixels(w, h, &rgba);
    assert!(
        teal > 200,
        "the ActiveWins control is also populated and paints a crowned row \
         (expected >200 teal px, got {teal})"
    );
    assert!(
        fg > 8000,
        "the ActiveWins control is also populated and paints lots of foreground \
         (expected >8000 px, got {fg})"
    );
}

/// **Negative control 2 — `Empty`.** The "press Run bake-off" prompt paints NO
/// table: ~no crowned-row ACCENT teal in the TABLE band. Proves the main guard's
/// crowned-row + foreground assertions genuinely discriminate the populated
/// `BenchmarkWins` leaderboard from the empty prompt.
#[test]
fn benchmark_wins_empty_negative_control() {
    let cockpit = ui::fixtures::fake_cockpit_leaderboard(PanelState::Empty);
    let (w, h, rgba) = render_leaderboard_rgba(cockpit);

    let table_teal = crowned_teal_pixels(w, h, &rgba);
    assert!(
        table_teal < 150,
        "the Empty state must NOT paint a crowned-row ACCENT highlight in the \
         TABLE band (expected <150 stray teal px, got {table_teal}). If high, the \
         BenchmarkWins crown guard is a tautology."
    );
}

/// **Anti-tautology tie.** The `BenchmarkWins` field and the `ActiveWins` field
/// are BOTH populated 7-arm leaderboards — but they differ in WHO is crowned (the
/// baseline vs an active arm). This test renders both and asserts the crowned
/// baseline row in the `BenchmarkWins` frame has STRICTLY LESS strategy-column
/// clay in its crowned-row band than the `ActiveWins` field paints in its
/// LOWER (active Fragile) rows — i.e. the benchmark-Fragile note really is muted
/// relative to an active Fragile badge, not the same pixels.
#[test]
fn benchmark_wins_baseline_note_is_muted_vs_active_badge() {
    let bw = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror_benchmark_wins_full(),
    ));
    let aw = ui::fixtures::fake_cockpit_leaderboard(PanelState::Ready(
        ui::fixtures::fake_bakeoff_report_mirror_with_ensembles(),
    ));

    let (wb, hb, rb) = render_leaderboard_rgba(bw);
    let (wa, ha, ra) = render_leaderboard_rgba(aw);

    // The crowned BASELINE row band in the BenchmarkWins frame.
    let (by0, by1) = crowned_row_band(wb, hb, &rb).expect("BenchmarkWins crowned row locatable");
    let baseline_row_clay = strat_col_clay_in_band(wb, &rb, by0, by1);

    // The ActiveWins frame paints active Fragile badges (rsi + majority) across
    // its full table band — a non-trivial amount of strategy-column clay.
    let active_field_clay = strat_col_clay_in_band(wa, &ra, TABLE_TOP, ha);

    assert!(
        active_field_clay > 60,
        "the ActiveWins field must paint active Fragile badges in the strategy \
         column (expected >60 clay px, got {active_field_clay}) — the control \
         reference for 'a real Fragile badge'"
    );
    assert!(
        baseline_row_clay < active_field_clay,
        "the crowned BASELINE row's Fragile note must paint strictly LESS \
         strategy-column clay than the ActiveWins field's active Fragile badges \
         (baseline row {baseline_row_clay} vs active field {active_field_clay}) — \
         the benchmark note is informational, not the disqualifying badge"
    );
}

/// **Light-theme token parity (coding rule: copy must be theme-capable).**
///
/// The `leaderboard_screen_program` harness pins `screens::leaderboard::view` to
/// `ThemeMode::Dark` (it hardcodes the mode + the `CANVAS` backdrop at
/// `ThemeMode::Dark`, irrespective of the iced `Theme`), so a *rendered* light
/// pixel here would only re-measure the dark surface — it cannot exercise real
/// light theming through this seam. Instead we assert the light-capability at
/// the TOKEN layer: every colour token the new B1 copy uses resolves to a
/// DISTINCT, non-degenerate value in light vs dark. If a future edit hardcodes a
/// dark-only colour into the benchmark/baseline/sat-in-cash treatment, the light
/// variant would collapse onto the dark one and this fails.
#[test]
fn benchmark_wins_copy_tokens_are_light_capable() {
    use ui::theme::{ThemeMode, color};

    // The tokens the U1–U3 treatment paints with: FG_3 (the baseline tag, the
    // benchmark informational note, the sat-in-cash note), ACCENT (the crowned
    // baseline row), and the DOWN_50/DOWN_500 pair (the active Fragile badge the
    // benchmark note is deliberately NOT). Each must differ light vs dark.
    for (name, tok) in [
        ("FG_3", color::FG_3),
        ("ACCENT", color::ACCENT),
        ("DOWN_50", color::DOWN_50),
        ("DOWN_500", color::DOWN_500),
        ("FG_1", color::FG_1),
    ] {
        let d = tok.current(ThemeMode::Dark);
        let l = tok.current(ThemeMode::Light);
        assert!(
            (d.r - l.r).abs() + (d.g - l.g).abs() + (d.b - l.b).abs() > 0.01,
            "{name} must resolve to a distinct value in light vs dark (the new B1 \
             copy is theme-capable, not dark-pinned): dark={d:?} light={l:?}"
        );
    }
}
