//! backtest-equity-companion — render-layer proof of the POPULATED equity
//! curve in the cockpit **Reports** screen detail pane.
//!
//! ## Why this file exists
//!
//! The shipped `cockpit-reports-viewer` had a verification gap: every Reports
//! panel snapshot used a *no-companion* fixture (`equity: PanelState::Empty`,
//! see `panel_snapshots.rs::reports_state_ready`), so the **populated-curve**
//! render path — `(Some(_), PanelState::Ready(r))` with `r.equity: Ready` in
//! `screens::reports::detail_pane` — was UNVERIFIED at the render layer. The
//! loader unit tests proved `load_report` returns `Ready`; the offline
//! `viewer` bin proved no-panic; neither proved the **cockpit Reports screen
//! actually draws a curve** when equity is `Ready`.
//!
//! Two durable guards close that gap:
//!
//! 1. [`reports_demo_discover_load_is_ready`] — replicates the LIVE path:
//!    real `discover_reports()` → find the committed demo by `file_stem` →
//!    `load_report(&entry.path)` → assert `equity: Ready`. Catches any
//!    discovery/companion-resolver path mismatch a hand-built unit fixture
//!    would miss. Skip-if-absent so a checkout that prunes the demo artifact
//!    does not fail.
//!
//! 2. [`reports_populated_curve_draws_in_detail_pane`] — renders the REAL
//!    `screens::reports::view` (via the full cockpit shell) HEADLESS with a
//!    `Ready` equity selection, and asserts on the rendered PIXELS that the
//!    `ACCENT`-coloured polyline + `UP_500` fill actually paint in the detail
//!    pane. Uses a synthetic in-memory series (checkout-independent — no
//!    `spec/` dependency, no committed-PNG byte compare that would be brittle
//!    against cosmic-text font variance or demo pruning).
//!
//! 3. [`reports_empty_equity_draws_no_curve`] — the negative control: the
//!    SAME harness with `equity: Empty` must produce ~no curve-coloured
//!    pixels, proving guard (2) is not a tautology (it genuinely
//!    discriminates the populated curve from the "No equity data" empty
//!    state).
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like `render_snapshots.rs`, real-renderer pixel assertions are
//! macOS-canonical (cosmic-text font rasterisation is per-OS). The file
//! compiles to nothing on Linux/Windows. The pixel thresholds here are
//! deliberately coarse (presence/absence of curve hue, not byte-exact), so
//! they are robust within macOS across font-DB jitter.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]

use std::path::PathBuf;
use std::time::Duration;

use smol_str::SmolStr;
use trading_core::{BacktestMetrics, EquitySeries};
use ui::reports::state::newest_companion_index;
use ui::reports::{ReportEntry, ReportsScreenState, loader};
use ui::state::{Cockpit, PanelState, Screen};
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};
use ui::viewer::{ReportFrontMatter, ReportLoadResult};

/// The committed demo report (a non-anchored `btc-2024-h1-sma-cross` run
/// whose stem-matched companion CSV makes the curve populate).
const DEMO_STEM: &str = "backtest-20260617-180015-btc-2024-h1-sma-cross";

// ─── JOB 1: live discover → load round-trips to a Ready curve ────────────

/// Replicate the production path the cockpit Reports screen actually takes:
/// `discover_reports()` (the real all-slug scan) produces the `ReportEntry`,
/// and `load_report(&entry.path)` resolves its companion. Asserts the
/// discovered demo's `equity` is `Ready` — proving discovery hands the
/// companion resolver a path it can follow (a hand-built unit fixture cannot
/// catch a discovery/resolver path mismatch; this can).
///
/// Skip-if-absent (mirrors the loader's `load_equity_companion_real_demo_*`
/// guard) so a checkout that prunes the demo artifact does not fail.
#[test]
fn reports_demo_discover_load_is_ready() {
    let entries = loader::discover_reports();
    let Some(entry) = entries.iter().find(|e| e.file_stem.as_str() == DEMO_STEM) else {
        eprintln!("demo report `{DEMO_STEM}` not discovered — skipping (pruned checkout)");
        return;
    };
    // Guard the companion too: if the .md is committed but the artifacts/ CSV
    // was pruned, skip rather than fail (the curve can't be Ready without it).
    let has_companion = entry
        .path
        .parent()
        .map(|p| p.join("artifacts").join(DEMO_STEM))
        .is_some_and(|d| d.is_dir());
    if !has_companion {
        eprintln!("demo companion CSV pruned — skipping");
        return;
    }

    let loaded = loader::load_report(&entry.path).expect("load_report ok for the discovered demo");
    assert!(
        matches!(loaded.equity, PanelState::Ready(_)),
        "the discovered demo's equity must resolve to Ready via the LIVE \
         discover→load path (companion path mismatch if not); got {}",
        loaded.equity.variant_name()
    );
    if let PanelState::Ready(s) = &loaded.equity {
        assert!(
            !s.points.is_empty(),
            "a Ready demo curve must carry points (the populated-curve precondition)"
        );
    }
}

/// End-to-end render of the REAL committed demo through the **production
/// selection path** (`discover_reports` → `ReportsScreenState::load_selection`
/// → `screens::reports::view`). This is the closest possible replica of what
/// the operator sees when they click the demo row, and it regenerates the
/// operator-facing deliverable PNG at `/tmp/reports_demo_render.png`. Asserts
/// the rendered detail pane paints a populated curve. Skip-if-absent.
#[test]
fn reports_demo_real_selection_renders_curve() {
    let entries = loader::discover_reports();
    let Some(idx) = entries
        .iter()
        .position(|e| e.file_stem.as_str() == DEMO_STEM)
    else {
        eprintln!("demo `{DEMO_STEM}` not discovered — skipping (pruned checkout)");
        return;
    };
    let has_companion = entries[idx]
        .path
        .parent()
        .map(|p| p.join("artifacts").join(DEMO_STEM))
        .is_some_and(|d| d.is_dir());
    if !has_companion {
        eprintln!("demo companion CSV pruned — skipping");
        return;
    }

    // Production boot + selection path — NOT a hand-built loaded state.
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;
    ui::reports::load_into(&mut cockpit);
    let cockpit_idx = match &cockpit.reports_screen_state.discovered {
        PanelState::Ready(list) => list.iter().position(|e| e.file_stem.as_str() == DEMO_STEM),
        _ => None,
    }
    .expect("demo present in the cockpit's own discovered list");
    cockpit.reports_screen_state.selected = Some(cockpit_idx);
    cockpit.reports_screen_state.load_selection(cockpit_idx);

    let (w, h, rgba) = render_reports_rgba(cockpit);
    // Operator-facing deliverable (memory: verify UI at the render layer).
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/reports_demo_render.png");
    }
    let hits = curve_pixels(w, h, &rgba);
    assert!(
        hits > 1000,
        "the REAL demo report, selected via the production path, must paint a \
         populated curve (expected >1000 ACCENT/UP_500 px, got {hits}). \
         PNG: /tmp/reports_demo_render.png"
    );
}

// ─── JOB 2: render-layer proof + negative control ────────────────────────

/// Build a `Cockpit` routed to `Screen::Reports` with a single discovered
/// entry SELECTED and LOADED to a `Ready(ReportLoadResult)` whose `equity`
/// field is the supplied state. Checkout-independent (synthetic in-memory
/// data; the `path` is never read because `loaded` is pre-populated).
fn reports_cockpit_with_equity(equity: PanelState<EquitySeries>) -> Cockpit {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;

    let entry = ReportEntry {
        slug: SmolStr::new("v0-paper-sma"),
        file_stem: SmolStr::new(DEMO_STEM),
        path: PathBuf::from(format!("/fixture/v0-paper-sma/reports/{DEMO_STEM}.md")),
        has_companion: true,
    };
    let metrics = BacktestMetrics {
        total_return_pct: rust_decimal_macros::dec!(7.38),
        cagr_pct: rust_decimal_macros::dec!(3.60),
        cagr_present: true,
        sharpe: rust_decimal_macros::dec!(7.7975),
        sharpe_present: true,
        max_drawdown_pct: rust_decimal_macros::dec!(4.20),
        win_rate_pct: rust_decimal_macros::dec!(52.0),
        win_rate_present: true,
        trades: 441,
    };
    let loaded = ReportLoadResult {
        front_matter: ReportFrontMatter {
            scenario: SmolStr::new("btc-2024-h1-sma-cross"),
        },
        metrics: PanelState::Ready(metrics),
        equity,
        body_markdown: "# Backtest Report — btc-2024-h1-sma-cross\n\n## Summary\nrow\n".to_string(),
    };

    cockpit.reports_screen_state = ReportsScreenState {
        discovered: PanelState::Ready(vec![entry]),
        selected: Some(0),
        loaded: PanelState::Ready(loaded),
        // reports-picker-curve-filter — default curve-only filter.
        show_all_reports: false,
    };
    cockpit
}

/// Render the full cockpit shell (routing through the REAL
/// `screens::reports::view`) at the `typical` 1920×1080 slot and return the
/// physical-pixel RGBA buffer + dimensions.
fn render_reports_rgba(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let screenshot = iced_test::screenshot(&program, &theme, (1920, 1080), 1.0, Duration::ZERO);
    (
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
}

/// Count "curve-coloured" pixels in the detail pane (the right ~70% of the
/// frame; the left 320 px is the picker rail). The equity curve paints an
/// `ACCENT` (#6FB6AE, teal) polyline over a `UP_500` (#6E9B6A, sage) fill on
/// a `PANEL` (#1C2127, dark) background — both are saturated greens clearly
/// distinct from the dark chrome and neutral fg text. We count both hues.
fn curve_pixels(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let x0 = (w as f32 * 0.30) as u32; // skip the picker rail
    let mut hits = 0u64;
    for y in 0..h {
        for x in x0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            // Teal ACCENT polyline: green & blue high and close, red lower.
            let teal = g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25;
            // Sage UP_500 fill: green is clearly the dominant channel over a
            // mid-dark base (excludes the near-black panel + neutral text).
            let sage = g > 90 && g > r + 15 && g > b + 15 && (40..140).contains(&r);
            if teal || sage {
                hits += 1;
            }
        }
    }
    hits
}

/// **The render-layer guard.** With a `Ready` equity series, the cockpit
/// Reports screen detail pane MUST paint a populated curve — thousands of
/// ACCENT/UP_500 pixels. This is the proof the operator's "no equity data"
/// report is NOT a render bug: when equity is `Ready`, the curve draws.
///
/// Also writes the rendered PNG to `target/` for human inspection on demand.
#[test]
fn reports_populated_curve_draws_in_detail_pane() {
    let series = ui::fixtures::fake_equity_series_for_viewer();
    let n = series.points.len();
    assert!(n > 1, "the viewer fixture series must have >1 point");

    let cockpit = reports_cockpit_with_equity(PanelState::Ready(series));
    let (w, h, rgba) = render_reports_rgba(cockpit);

    // Persist for human eyeball (memory: verify UI at the render layer).
    let out = format!(
        "{}/../../target/reports_populated_curve.png",
        env!("CARGO_MANIFEST_DIR")
    );
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save(&out);
    }

    let hits = curve_pixels(w, h, &rgba);
    // Calibrated separation (macOS, typical slot): empty state ≈ 284 stray
    // chrome-green px; the smooth viewer fixture curve ≈ 1959; the choppier
    // committed demo curve ≈ 4384. A 1000-px floor sits ~3.5× above the empty
    // noise floor and ~2× below the smoothest populated curve — robust to
    // cosmic-text font-DB jitter without being a tautology (the negative
    // control below stays < 500).
    assert!(
        hits > 1000,
        "a Ready equity selection must paint a populated curve in the Reports \
         detail pane (expected >1000 ACCENT/UP_500 px, got {hits}). If this fails \
         the screen renders the empty state despite Ready equity — a render/wiring \
         bug in screens::reports::detail_pane. PNG: {out}"
    );
}

/// **Negative control / discriminator.** The SAME harness with `equity:
/// Empty` renders the "No equity data" empty state — near-zero curve-coloured
/// pixels. Proves the populated-curve assertion above genuinely discriminates
/// (it is not satisfied by sidebar/chrome greens). This is also the exact
/// state the no-companion sibling report (`backtest-20260527-…`, which sorts
/// FIRST) shows — the report the operator most likely had selected.
#[test]
fn reports_empty_equity_draws_no_curve() {
    let cockpit = reports_cockpit_with_equity(PanelState::Empty);
    let (w, h, rgba) = render_reports_rgba(cockpit);
    let hits = curve_pixels(w, h, &rgba);
    assert!(
        hits < 500,
        "the Empty equity state must NOT paint a curve (expected <500 stray \
         curve-hue px from chrome, got {hits}). If this is high the discriminator \
         is unreliable and the populated-curve guard is a tautology."
    );
}

// ─── JOB 3: discoverability render proof (marker + auto-select curve) ─────
//
// backtest-equity-companion UX follow-on. The operator could not find the one
// companion-bearing report (buried in a 112-row picker, with a no-companion
// near-duplicate sorting above it), so the Reports screen looked empty. Two
// changes fix discoverability: a "● curve" picker marker on companion rows,
// and a boot auto-select of the newest companion-bearing report. This render
// proof asserts BOTH paint at the render layer in ONE frame.

/// Count `ACCENT`-teal (#6FB6AE, the has-curve marker hue) pixels inside the
/// LEFT picker rail (x < 320 px) within the vertical band `[y0, y1)`. The
/// marker text "● curve" is the only `ACCENT` source on an **inactive** row
/// (an inactive row's label is `FG_3` muted + its border is `BORDER_1`), so a
/// band over an inactive companion row isolates the marker pixels — a clean,
/// non-tautological discriminator (the no-companion control band stays ~0).
fn marker_pixels_in_band(w: u32, rgba: &[u8], y0: u32, y1: u32) -> u64 {
    let x1 = 320u32.min(w); // the picker rail width (PICKER_WIDTH)
    let mut hits = 0u64;
    for y in y0..y1.min((rgba.len() as u32 / 4) / w) {
        for x in 0..x1 {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            // Teal ACCENT glyph: green & blue high and close, red clearly lower
            // (same predicate as the curve's teal polyline).
            if g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25 {
                hits += 1;
            }
        }
    }
    hits
}

/// Build a Reports cockpit whose discovered list contains BOTH a
/// companion-bearing entry and companion-less ones, driven through the REAL
/// auto-select decision (`newest_companion_index`). The selected entry's
/// `loaded` is a pre-populated `Ready(ReportLoadResult)` with `Ready` equity
/// (the `path`s are synthetic + never read, so the load is injected — exactly
/// as `reports_cockpit_with_equity` does), so the render is checkout-
/// independent. Returns the cockpit + the auto-selected index.
///
/// Row order (the picker's real shape): a no-companion near-duplicate
/// (`…20260527…`) sorts FIRST, the companion-bearing demo (`…20260617…`)
/// SECOND, an older no-companion report THIRD. The auto-select MUST land on
/// row 1 (the companion-bearing one), not row 0.
fn reports_cockpit_marker_and_autoselect() -> (Cockpit, usize) {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;

    let entries = vec![
        // Row 0 — the no-companion near-duplicate that sorts ABOVE the demo.
        ReportEntry {
            slug: SmolStr::new("v0-paper-sma"),
            file_stem: SmolStr::new("backtest-20260527-120000-btc-2024-h1-sma-cross"),
            path: PathBuf::from(
                "/fixture/v0-paper-sma/reports/backtest-20260527-120000-btc-2024-h1-sma-cross.md",
            ),
            has_companion: false,
        },
        // Row 1 — the companion-bearing demo (newest companion → auto-selected).
        ReportEntry {
            slug: SmolStr::new("v0-paper-sma"),
            file_stem: SmolStr::new(DEMO_STEM),
            path: PathBuf::from(format!("/fixture/v0-paper-sma/reports/{DEMO_STEM}.md")),
            has_companion: true,
        },
        // Row 2 — an older, no-companion report (the control band).
        ReportEntry {
            slug: SmolStr::new("v05-composed-strategies"),
            file_stem: SmolStr::new("backtest-20260101-090000-btc-rsi"),
            path: PathBuf::from(
                "/fixture/v05-composed-strategies/reports/backtest-20260101-090000-btc-rsi.md",
            ),
            has_companion: false,
        },
    ];

    // The production auto-select decision, exercised directly.
    let auto = newest_companion_index(&entries).expect("a companion-bearing row exists");
    assert_eq!(
        auto, 1,
        "auto-select must choose the companion-bearing demo (row 1), not the \
         higher-sorting no-companion near-duplicate (row 0)"
    );

    let metrics = BacktestMetrics {
        total_return_pct: rust_decimal_macros::dec!(7.38),
        cagr_pct: rust_decimal_macros::dec!(3.60),
        cagr_present: true,
        sharpe: rust_decimal_macros::dec!(7.7975),
        sharpe_present: true,
        max_drawdown_pct: rust_decimal_macros::dec!(4.20),
        win_rate_pct: rust_decimal_macros::dec!(52.0),
        win_rate_present: true,
        trades: 441,
    };
    let loaded = ReportLoadResult {
        front_matter: ReportFrontMatter {
            scenario: SmolStr::new("btc-2024-h1-sma-cross"),
        },
        metrics: PanelState::Ready(metrics),
        equity: PanelState::Ready(ui::fixtures::fake_equity_series_for_viewer()),
        body_markdown: "# Backtest Report — btc-2024-h1-sma-cross\n\n## Summary\nrow\n".to_string(),
    };

    cockpit.reports_screen_state = ReportsScreenState {
        discovered: PanelState::Ready(entries),
        selected: Some(auto),
        loaded: PanelState::Ready(loaded),
        // reports-picker-curve-filter — default curve-only filter.
        show_all_reports: false,
    };
    (cockpit, auto)
}

/// **The discoverability render guard.** Renders the Reports screen headless
/// with a discovered list holding BOTH a companion-bearing entry and
/// companion-less ones, after auto-select, and asserts BOTH fixes paint:
///
/// (a) the "● curve" marker shows ACCENT-teal pixels on the **companion** row
///     band, while a no-companion control row band has ~none (isolates the
///     marker — the companion row tested is INACTIVE, so its only ACCENT
///     source is the marker glyph), and
/// (b) the auto-selected report's detail pane paints a populated curve
///     (>1000 ACCENT/UP_500 px — the same guard as
///     `reports_populated_curve_draws_in_detail_pane`).
///
/// Saves the operator-facing PNG to `/tmp/reports_marker_render.png` (memory:
/// verify UI at the render layer).
#[test]
fn reports_marker_and_autoselect_render() {
    let (cockpit, _auto) = reports_cockpit_marker_and_autoselect();
    let (w, h, rgba) = render_reports_rgba(cockpit);

    // Operator-facing deliverable.
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/reports_marker_render.png");
    }

    // The three picker rows sit at the top of the rail under the title. Scan
    // generous vertical bands per row (the picker title + L padding push the
    // first row down ~64–96 px; each compact row ≈ 28–40 px tall). Row 1 (the
    // companion-bearing demo) carries the marker; rows 0 & 2 do not.
    //
    // Band 0 ≈ rows 0–1 top region (no-companion #0). Band 1 ≈ the companion
    // row #1. We scan a wide [60, 220) span covering all three rows for the
    // marker total, then a no-companion-only control span to prove isolation.
    let marker_all_rows = marker_pixels_in_band(w, &rgba, 60, 220);
    // The no-companion control: the FIRST row band only (row 0, the
    // `…20260527…` near-duplicate). Its label is muted + border hairline → no
    // ACCENT. A tight band over just row 0.
    let control_row0 = marker_pixels_in_band(w, &rgba, 60, 92);

    assert!(
        marker_all_rows > 30,
        "the '● curve' marker must paint ACCENT-teal pixels in the picker rail \
         on the companion row (expected >30 px across the row band, got \
         {marker_all_rows}). PNG: /tmp/reports_marker_render.png"
    );
    assert!(
        control_row0 < marker_all_rows,
        "the marker must be ISOLATED to the companion row: the no-companion row 0 \
         band ({control_row0} px) must hold fewer ACCENT px than the full \
         marker-bearing span ({marker_all_rows} px) — proving the marker is not \
         chrome/active-row bleed. PNG: /tmp/reports_marker_render.png"
    );

    // (b) the auto-selected detail pane paints a populated curve.
    let hits = curve_pixels(w, h, &rgba);
    assert!(
        hits > 1000,
        "auto-select must land on the companion-bearing report so its populated \
         curve renders on entry (expected >1000 ACCENT/UP_500 px in the detail \
         pane, got {hits}). PNG: /tmp/reports_marker_render.png"
    );
}

// ─── JOB 4: switch-regression — the operator's "switch → empty" report ────
//
// 2026-06-19: operator reported "open a report → I see a graph; switch to
// another report → the graph is empty." Investigation (orchestrator) found the
// selection/render path is CORRECT — `ReportsScreenState::load_selection`
// reloads `equity` on every `ReportsSelect`, and `equity_curve::view` rebuilds
// its canvas program per frame (no stale cache). The empties were a DATA fact:
// only some reports ship a stem-matched companion CSV, so switching to a
// no-companion report legitimately shows the empty state. The fix expanded the
// companion-bearing set (6 → 14). These guards lock the real behaviour in at
// the PIXEL layer: switching between two distinct companion reports repaints a
// DIFFERENT populated curve each time, and switching to a no-companion report
// shows the honest empty state. Skip-if-absent (pruned checkout).

/// Index of the newest discovered companion report whose `file_stem` contains
/// `needle` (the live `discover_reports` list, exactly as the cockpit sees it).
fn companion_idx_for(cockpit: &Cockpit, needle: &str) -> Option<usize> {
    match &cockpit.reports_screen_state.discovered {
        PanelState::Ready(list) => list
            .iter()
            .position(|e| e.has_companion && e.file_stem.as_str().contains(needle)),
        _ => None,
    }
}

/// **Switch-regression guard (the operator's bug).** In ONE session, select a
/// companion report, render (curve A), then SWITCH to a second distinct
/// companion report and render (curve B). Both must paint a populated curve —
/// refuting "switch → empty" — and the two frames must DIFFER, refuting a
/// hypothetical stale-cache "switch → keeps showing curve A". Drives the real
/// production path (`load_into` → `load_selection`) over the live corpus, so it
/// only passes when the on-disk companion CSVs actually resolve + render.
#[test]
fn reports_switch_between_companions_repaints_distinct_curve() {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;
    ui::reports::load_into(&mut cockpit);

    // Two distinct curve-bearing reports from the 2026-06-19 companion batch.
    let (Some(a), Some(b)) = (
        companion_idx_for(&cockpit, "avax-2024-sma-cross"),
        companion_idx_for(&cockpit, "dot-2024-sma-cross"),
    ) else {
        eprintln!("avax/dot companions not discovered — skipping (pruned checkout)");
        return;
    };

    // Select report A → render.
    cockpit.reports_screen_state.selected = Some(a);
    cockpit.reports_screen_state.load_selection(a);
    let (wa, ha, rgba_a) = render_reports_rgba(cockpit.clone());
    if let Some(img) = image::RgbaImage::from_raw(wa, ha, rgba_a.clone()) {
        let _ = img.save("/tmp/reports_switch_a_avax.png");
    }
    let hits_a = curve_pixels(wa, ha, &rgba_a);
    assert!(
        hits_a > 1000,
        "companion report A (avax) must paint a populated curve (got {hits_a}). \
         PNG: /tmp/reports_switch_a_avax.png"
    );

    // SWITCH to report B → render. This is the operator's exact action.
    cockpit.reports_screen_state.selected = Some(b);
    cockpit.reports_screen_state.load_selection(b);
    let (wb, hb, rgba_b) = render_reports_rgba(cockpit);
    if let Some(img) = image::RgbaImage::from_raw(wb, hb, rgba_b.clone()) {
        let _ = img.save("/tmp/reports_switch_b_dot.png");
    }
    let hits_b = curve_pixels(wb, hb, &rgba_b);
    assert!(
        hits_b > 1000,
        "after switching A->B the curve must STILL paint — this is the operator's \
         'switch -> empty' report; empty here = regression (got {hits_b}). \
         PNG: /tmp/reports_switch_b_dot.png"
    );
    assert!(
        rgba_a != rgba_b,
        "switching avax->dot must REPAINT a different frame (identical = stale-cache \
         bug: the switch did not reload the new report's equity)"
    );
}

/// **Negative control — switch to a no-companion report shows empty.** This is
/// the state the operator saw and (correctly) read as "empty": a report with no
/// companion CSV has no curve to draw. Proves the empties are honest data, not
/// a render/selection bug — and that the populated guard above discriminates.
#[test]
fn reports_switch_to_no_companion_is_empty() {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;
    ui::reports::load_into(&mut cockpit);

    let Some(a) = companion_idx_for(&cockpit, "avax-2024-sma-cross") else {
        eprintln!("avax companion not discovered — skipping (pruned checkout)");
        return;
    };
    let no_comp = match &cockpit.reports_screen_state.discovered {
        PanelState::Ready(list) => list.iter().position(|e| !e.has_companion),
        _ => None,
    };
    let Some(no_comp) = no_comp else {
        eprintln!("no no-companion report in corpus — skipping");
        return;
    };

    // Start on the companion report → curve paints.
    cockpit.reports_screen_state.selected = Some(a);
    cockpit.reports_screen_state.load_selection(a);
    let (wa, ha, rgba_a) = render_reports_rgba(cockpit.clone());
    assert!(
        curve_pixels(wa, ha, &rgba_a) > 1000,
        "precondition: the companion report must paint a curve first"
    );

    // Switch to a no-companion report → honest empty state (CORRECT behaviour).
    cockpit.reports_screen_state.selected = Some(no_comp);
    cockpit.reports_screen_state.load_selection(no_comp);
    let (we, he, rgba_e) = render_reports_rgba(cockpit);
    let hits_e = curve_pixels(we, he, &rgba_e);
    assert!(
        hits_e < 500,
        "a no-companion report must show the empty state (no curve); got {hits_e} \
         curve px — if high, the screen is painting a stale/previous curve"
    );
}

// ─── JOB 5: curve-only picker filter (reports-picker-curve-filter) ────────
//
// The operator kept landing on companion-less "no equity data" reports (only
// 14 of 117 ship a curve), so the picker now DEFAULTS to a curve-only filter
// with an "All" toggle. These render guards prove, at the PIXEL layer:
//   (a) default (curve-only) shows ONLY companion rows;
//   (b) toggled (show_all) shows STRICTLY MORE rows (the full corpus);
//   (c) index-safety — selecting a filtered companion row loads the CORRECT
//       report (its own curve paints), proving the enumerate-and-skip keeps
//       each visible row's TRUE full-discovered-list index.

/// The picker rail's absolute X span in the rendered frame: it sits AFTER the
/// left sidebar (`SIDEBAR_WIDTH_PX` ≈ 180 px) and the body's `space::L` left
/// padding, and is `PICKER_WIDTH` (320 px) wide. We scan a conservative span
/// that excludes the sidebar nav labels on the left and the detail pane on the
/// right, so only the picker rows are measured.
const RAIL_X0: u32 = 185;
const RAIL_X1: u32 = 520;

/// Whether a scanline carries picker-rail "row content" — i.e. enough
/// foreground (label text / marker / active-row fill) pixels in the rail x-span
/// to be a row rather than inter-row gap. Background tiers (CANVAS / PANEL /
/// PANEL_RAISED) are near-black (< ~45 per channel); text + markers cross a
/// luma floor the dark tiers never reach.
fn rail_scanline_has_content(w: u32, rgba: &[u8], y: u32) -> bool {
    let x1 = RAIL_X1.min(w);
    let mut fg = 0u32;
    for x in RAIL_X0..x1 {
        let idx = ((y as usize * w as usize) + x as usize) * 4;
        let r = i32::from(rgba[idx]);
        let g = i32::from(rgba[idx + 1]);
        let b = i32::from(rgba[idx + 2]);
        let luma = (r * 2 + g * 3 + b) / 6;
        if luma > 70 {
            fg += 1;
        }
    }
    fg >= 8
}

/// Bottom margin to exclude from rail scans: the full-width status bar
/// ("Disconnected · Latency …") sits in the bottom ~30 px and crosses the rail
/// x-span, so it would otherwise register as a spurious picker row. Picker rows
/// never reach this low (the synthetic corpus is ≤ 4 short rows at the top).
const RAIL_Y_BOTTOM_MARGIN: u32 = 40;

/// Count visible picker ROWS in the rail within `[y_start, h - bottom_margin)`
/// (skipping the title + filter-toggle band above and the status bar below).
/// Collapses content scanlines into bands, MERGING runs closer than `MERGE_GAP`
/// px so a single row's label-text run and its active-row border run (a 1-2 px
/// line ~8 px below the text) count as ONE row, while distinct rows (≥ ~14 px
/// apart center-to-center, with the next row's top border well past the merge
/// window) stay separate. Counts ROWS (coarse bands), robust to cosmic-text
/// glyph jitter.
fn picker_row_bands(w: u32, h: u32, rgba: &[u8], y_start: u32) -> u32 {
    const MERGE_GAP: u32 = 10;
    let y_end = h.saturating_sub(RAIL_Y_BOTTOM_MARGIN);
    let mut bands = 0u32;
    let mut in_band = false;
    let mut gap = 0u32;
    for y in y_start..y_end {
        if rail_scanline_has_content(w, rgba, y) {
            if !in_band {
                bands += 1;
                in_band = true;
            }
            gap = 0;
        } else if in_band {
            gap += 1;
            if gap >= MERGE_GAP {
                in_band = false;
            }
        }
    }
    bands
}

/// Total foreground (text / marker / active-fill) pixel count in the picker
/// rail within `[y_start, h - bottom_margin)`. Monotonic in the number of
/// visible rows: more rows → more text → more foreground pixels. A band-free,
/// calibration-light signal used to corroborate the row-band count
/// (curve-only < show_all). Excludes the bottom status bar.
fn rail_foreground_pixels(w: u32, h: u32, rgba: &[u8], y_start: u32) -> u64 {
    let x1 = RAIL_X1.min(w);
    let y_end = h.saturating_sub(RAIL_Y_BOTTOM_MARGIN);
    let mut hits = 0u64;
    for y in y_start..y_end {
        for x in RAIL_X0..x1 {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            let luma = (r * 2 + g * 3 + b) / 6;
            if luma > 70 {
                hits += 1;
            }
        }
    }
    hits
}

/// Build a Reports cockpit with a deterministic, synthetic discovered list:
/// ONE companion-bearing report (the avax demo stem) + THREE companion-less
/// reports. Curve-only shows 1 row; show_all shows 4. The selection + loaded
/// state are injected (synthetic paths never read), so the render is checkout-
/// independent. `show_all_reports` is set by the caller.
///
/// The companion row's loaded `equity` is `Ready` so its detail-pane curve
/// paints (used by the index-safety guard). Returns the cockpit + the TRUE
/// full-list index of the companion row.
fn reports_cockpit_filter_scene(show_all: bool) -> (Cockpit, usize) {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;

    // Row order: a no-companion report FIRST (so the companion is NOT row 0 —
    // this is what makes the index-safety test meaningful: the visible
    // companion row's true index is 1, not 0).
    //
    // SHORT slugs + stems so each row label fits on ONE line in the 320-px rail
    // (no 2-line wrap), keeping the row-band probe exact: 1 row = 1 band. The
    // real corpus stems wrap, but row COUNT is what we assert; single-line rows
    // make the count unambiguous. (The live-corpus avax/dot rows are exercised
    // by JOB 4 + the index-safety guard below.)
    let entries = vec![
        ReportEntry {
            slug: SmolStr::new("sma"),
            file_stem: SmolStr::new("bt-btc-1"),
            path: PathBuf::from("/fixture/sma/reports/bt-btc-1.md"),
            has_companion: false,
        },
        // The ONE companion-bearing row (true full-list index = 1).
        ReportEntry {
            slug: SmolStr::new("sma"),
            file_stem: SmolStr::new("bt-avax-2"),
            path: PathBuf::from("/fixture/sma/reports/bt-avax-2.md"),
            has_companion: true,
        },
        ReportEntry {
            slug: SmolStr::new("rsi"),
            file_stem: SmolStr::new("bt-eth-3"),
            path: PathBuf::from("/fixture/rsi/reports/bt-eth-3.md"),
            has_companion: false,
        },
        ReportEntry {
            slug: SmolStr::new("macd"),
            file_stem: SmolStr::new("bt-sol-4"),
            path: PathBuf::from("/fixture/macd/reports/bt-sol-4.md"),
            has_companion: false,
        },
    ];
    let companion_idx = 1usize;

    let metrics = BacktestMetrics {
        total_return_pct: rust_decimal_macros::dec!(7.38),
        cagr_pct: rust_decimal_macros::dec!(3.60),
        cagr_present: true,
        sharpe: rust_decimal_macros::dec!(7.7975),
        sharpe_present: true,
        max_drawdown_pct: rust_decimal_macros::dec!(4.20),
        win_rate_pct: rust_decimal_macros::dec!(52.0),
        win_rate_present: true,
        trades: 441,
    };
    let loaded = ReportLoadResult {
        front_matter: ReportFrontMatter {
            scenario: SmolStr::new("avax-2024-sma-cross"),
        },
        metrics: PanelState::Ready(metrics),
        equity: PanelState::Ready(ui::fixtures::fake_equity_series_for_viewer()),
        body_markdown: "# Backtest Report — avax-2024-sma-cross\n\n## Summary\nrow\n".to_string(),
    };

    cockpit.reports_screen_state = ReportsScreenState {
        discovered: PanelState::Ready(entries),
        selected: Some(companion_idx),
        loaded: PanelState::Ready(loaded),
        show_all_reports: show_all,
    };
    (cockpit, companion_idx)
}

/// Default (curve-only) shows ONLY companion rows. With the default filter
/// (`show_all_reports == false`) and a synthetic corpus of one companion row
/// and three companion-less rows, the picker rail must show exactly ONE row
/// band. This proves the three "no equity data" reports are hidden by default
/// — the exact operator pain this fixes. Saves + (caller Reads) the PNG.
#[test]
fn reports_filter_curve_only_shows_only_companion_rows() {
    let (cockpit, _companion_idx) = reports_cockpit_filter_scene(false);
    let (w, h, rgba) = render_reports_rgba(cockpit);
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/reports_filter_curve_only.png");
    }
    // Rows start below the title + filter-toggle band (~80 px); the status bar
    // at the bottom is excluded inside `picker_row_bands`.
    let bands = picker_row_bands(w, h, &rgba, 80);
    assert_eq!(
        bands, 1,
        "curve-only (default) must show exactly the 1 companion row; the 3 \
         companion-less reports must be hidden (got {bands} row bands). \
         PNG: /tmp/reports_filter_curve_only.png"
    );
}

/// **(b) Toggled (show_all) shows STRICTLY MORE rows.** The SAME corpus with
/// `show_all_reports == true` must show all 4 rows — strictly more than the
/// curve-only view's 1. Proves the "All" toggle reveals the full corpus.
/// Saves + (caller Reads) the PNG.
#[test]
fn reports_filter_show_all_reveals_more_rows() {
    let (cockpit_curve, _) = reports_cockpit_filter_scene(false);
    let (wc, hc, rgba_c) = render_reports_rgba(cockpit_curve);
    let bands_curve = picker_row_bands(wc, hc, &rgba_c, 80);
    let fg_curve = rail_foreground_pixels(wc, hc, &rgba_c, 80);

    let (cockpit_all, _) = reports_cockpit_filter_scene(true);
    let (wa, ha, rgba_a) = render_reports_rgba(cockpit_all);
    if let Some(img) = image::RgbaImage::from_raw(wa, ha, rgba_a.clone()) {
        let _ = img.save("/tmp/reports_filter_show_all.png");
    }
    let bands_all = picker_row_bands(wa, ha, &rgba_a, 80);
    let fg_all = rail_foreground_pixels(wa, ha, &rgba_a, 80);

    assert_eq!(
        bands_all, 4,
        "show_all must reveal all 4 discovered rows (got {bands_all}). \
         PNG: /tmp/reports_filter_show_all.png"
    );
    assert!(
        bands_all > bands_curve,
        "show_all ({bands_all} rows) must show STRICTLY MORE than curve-only \
         ({bands_curve} rows). PNG: /tmp/reports_filter_show_all.png"
    );
    // Corroborating, band-free signal: more rows → more rail foreground px.
    assert!(
        fg_all > fg_curve,
        "show_all rail foreground px ({fg_all}) must exceed curve-only \
         ({fg_curve}) — strictly more row text is visible. PNG: \
         /tmp/reports_filter_show_all.png"
    );
}

/// Index-safety — the filtered companion row loads the CORRECT report. This is
/// the exact bug class the feature must NOT introduce: when curve-only hides
/// rows, the visible companion row's `Message::ReportsSelect(idx)` must carry
/// its TRUE full-discovered-list index, so `load_selection(idx)` resolves the
/// RIGHT report — NEVER a filtered-subset position that loads a different
/// report.
///
/// The guard drives the REAL production path over the LIVE corpus (so the
/// on-disk companion CSV actually resolves + renders, exactly like JOB 4).
/// First it `load_into`s the real discovered list (the cockpit's own), then
/// finds a companion row's TRUE full-list index via `companion_idx_for` (avax)
/// and asserts there is a no-companion report at a LOWER index — that lower row
/// is hidden under curve-only, so a buggy "position in the filtered subset"
/// would resolve to a DIFFERENT (smaller) index and load the wrong report.
/// Then it drives `update(Message::ReportsSelect(true_idx))` through the
/// production handler (the same message the curve-only picker row emits) and
/// asserts the detail pane paints avax's OWN curve AND that the loaded
/// front-matter scenario is avax's — proving the index resolved to the intended
/// report, not a neighbour. Skip-if-absent (pruned checkout).
#[test]
fn reports_filter_curve_only_selects_correct_report_by_true_index() {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Reports;
    // Curve-only is the default; this is the operator's default surface.
    cockpit.reports_screen_state.show_all_reports = false;
    ui::reports::load_into(&mut cockpit);

    let Some(true_idx) = companion_idx_for(&cockpit, "avax-2024-sma-cross") else {
        eprintln!("avax companion not discovered — skipping (pruned checkout)");
        return;
    };

    // Precondition that makes re-indexing a real, detectable bug: there is at
    // least one HIDDEN (no-companion) report at an index BELOW the avax row. If
    // the picker wrongly re-indexed against the curve-only filtered subset,
    // avax (whose subset position is smaller than its true index) would resolve
    // to a different, lower full-list index → the wrong report.
    let lower_no_companion = match &cockpit.reports_screen_state.discovered {
        PanelState::Ready(list) => list
            .iter()
            .enumerate()
            .take(true_idx)
            .any(|(_, e)| !e.has_companion),
        _ => false,
    };
    assert!(
        lower_no_companion,
        "test precondition: a hidden no-companion report must sit below the avax \
         companion (true idx {true_idx}) so a filtered-subset re-index would \
         resolve to the WRONG report — otherwise this guard cannot catch the bug"
    );

    // Capture the avax stem at its TRUE index (what the picker row's
    // ReportsSelect(true_idx) must resolve to).
    let expected_stem: String = match &cockpit.reports_screen_state.discovered {
        PanelState::Ready(list) => list[true_idx].file_stem.to_string(),
        _ => unreachable!(),
    };

    // Drive the EXACT message the curve-only picker row emits, through the REAL
    // update handler (not a hand-set field) — proving the wired path is index-
    // safe end to end.
    ui::state::update(&mut cockpit, ui::state::Message::ReportsSelect(true_idx));

    // The selection + load must resolve to the avax report at its TRUE index.
    assert_eq!(
        cockpit.reports_screen_state.selected,
        Some(true_idx),
        "the handler must select the TRUE full-list index"
    );
    let loaded_scenario = match &cockpit.reports_screen_state.loaded {
        PanelState::Ready(r) => r.front_matter.scenario.to_string(),
        other => panic!("expected the avax report to load Ready, got {other:?}"),
    };
    assert!(
        loaded_scenario.contains("avax"),
        "ReportsSelect(true_idx={true_idx}) must load the avax report (stem \
         {expected_stem}); got scenario {loaded_scenario:?} — a wrong index \
         loaded a different report (the killed bug class)"
    );

    // Render the detail pane and prove avax's OWN curve paints (>1000 px). A
    // re-index to a no-companion neighbour would show the empty state instead.
    let (w, h, rgba) = render_reports_rgba(cockpit);
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.clone()) {
        let _ = img.save("/tmp/reports_filter_index_safety.png");
    }
    let hits = curve_pixels(w, h, &rgba);
    assert!(
        hits > 1000,
        "selecting the filtered companion row by its TRUE index ({true_idx}) must \
         load THAT report and paint its curve (>1000 px, got {hits}). PNG: \
         /tmp/reports_filter_index_safety.png"
    );
}
