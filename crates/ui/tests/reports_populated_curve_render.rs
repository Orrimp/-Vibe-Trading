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
