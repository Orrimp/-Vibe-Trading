//! PROACTIVE pixel-layer render audit — Group A (equity / chart / data-table
//! surfaces): Live, Reports, Baseline, Compare.
//!
//! ## Why this file exists
//!
//! Not a reported bug — a sweep to catch latent "no graph / blank panel /
//! wrong-state / clipped" render bugs BEFORE the operator hits them, at the
//! ONLY layer that proves the screen draws: rasterized tiny-skia pixels (the
//! cardinal rule from `spec/dev-notes/iced-ui-render-verification.md`). The
//! Reports equity curve shipped blind THREE times on green proxies (text
//! `.snap`, no-panic boot, loader-Ready unit test); this file renders the
//! POPULATED / data-bearing state of each Group-A screen through the REAL
//! cockpit shell (`shell::view` via `program_from_cockpit`), writes the PNG to
//! `/tmp/ui-audit/group-a/`, and asserts on hue-pixels WITH a negative control
//! so "it drew the data" is provable by contrast, not assumed.
//!
//! ## Coverage map (what was already render-proven vs. the gap this closes)
//!
//! - **Live** — `live_equity_render.rs` already pins the populated ACCENT
//!   polyline + a degenerate negative control. This file adds a full-shell
//!   operator-facing PNG for the eyeball pass.
//! - **Reports** — `reports_populated_curve_render.rs` pins the populated
//!   curve + Empty negative control. Re-rendered here for the eyeball pass.
//! - **Compare** — `live_equity_render.rs` PHASE 4 pins the two-run overlay
//!   (ACCENT + ACCENT_2) + single-run negative control. Re-rendered here.
//! - **Baseline** — **NO pixel-render test existed.** `baseline_error_state.rs`
//!   only drives `Widget::layout` ("no-panic, non-zero root" — a layer-3 proxy,
//!   NOT a rasterized render) and only the Error/Loading states. The POPULATED
//!   Baseline curve — the realized buy-and-hold line the operator sees on the
//!   default 2024 screen — had NEVER been pixel-verified. THIS is the gap class
//!   that shipped Reports blind, and the primary reason this audit file lands.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Like every pixel harness in this crate, real-renderer assertions are
//! macOS-canonical (cosmic-text rasterization is per-OS); the file compiles to
//! nothing off macOS. Pixel thresholds are coarse presence/absence-of-hue (not
//! byte-exact), robust to font-DB jitter, and every positive is paired with a
//! negative control so it cannot be a tautology.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Money, PnlSnapshot, Timestamp, Usdt};

use ui::compare::state::{CachedCell, CompareScreenState, OverlaySlot};
use ui::lab::state::DateRange;
use ui::state::{BaselineYear, Cockpit, Message, Screen, update};
use ui::test_support::program_from_cockpit;

// ── Shared render plumbing ──────────────────────────────────────────────────

/// The audit output directory the orchestrator reads PNGs from.
const OUT_DIR: &str = "/tmp/ui-audit/group-a";

/// Operator-locked "typical" slot (1920×1080, scale 1.0) — the same the
/// Reports populated harness renders at, so the PNGs are directly comparable.
const VIEW_W: u32 = 1920;
const VIEW_H: u32 = 1080;
const SCALE: f32 = 1.0;

/// Render the FULL cockpit shell (sidebar + the routed screen body + status
/// bar) for `cockpit` and return `(w, h, rgba)`. This is exactly the path the
/// `cockpit_live` binary paints — `program_from_cockpit` → `shell::view`.
fn render_shell(cockpit: Cockpit) -> (u32, u32, Vec<u8>) {
    ui::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO);
    (shot.size.width, shot.size.height, shot.rgba.to_vec())
}

/// Save the RGBA buffer to `OUT_DIR/<name>.png` for the orchestrator's eyeball
/// pass (project law — verify UI at the render layer). Never panics on a save
/// failure (the directory is created best-effort).
fn save_png(name: &str, w: u32, h: u32, rgba: &[u8]) -> String {
    let _ = std::fs::create_dir_all(OUT_DIR);
    let path = format!("{OUT_DIR}/{name}.png");
    if let Some(img) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) {
        let _ = img.save(&path);
    }
    path
}

/// Count "equity-curve-coloured" pixels in the right ~70 % of the frame (skip
/// the left sidebar/picker rail, whose `ACCENT` active-item highlight would
/// otherwise leak in). The curves paint an `ACCENT` (#6FB6AE teal) polyline
/// over a `UP_500` (#6E9B6A sage) fill on a dark `PANEL`; we count both hues.
/// Same predicate family as `reports_populated_curve_render::curve_pixels`.
fn curve_pixels_right(w: u32, h: u32, rgba: &[u8]) -> u64 {
    let x0 = (w as f32 * 0.30) as u32;
    let mut hits = 0u64;
    for y in 0..h {
        for x in x0..w {
            let idx = ((y as usize * w as usize) + x as usize) * 4;
            if idx + 2 >= rgba.len() {
                continue;
            }
            let r = i32::from(rgba[idx]);
            let g = i32::from(rgba[idx + 1]);
            let b = i32::from(rgba[idx + 2]);
            let teal = g > 120 && b > 120 && (g - b).abs() < 40 && (g - r) > 25;
            let sage = g > 90 && g > r + 15 && g > b + 15 && (40..140).contains(&r);
            if teal || sage {
                hits += 1;
            }
        }
    }
    hits
}

// ════════════════════════════════════════════════════════════════════════════
// LIVE — full-shell operator-facing render (populated + cold negative control)
// ════════════════════════════════════════════════════════════════════════════

/// One realistic monotone wallclock-stamped P&L session (rises, dips for a
/// non-zero drawdown, recovers) — the shape the agent reconciler publishes.
fn live_session() -> Vec<(i64, Decimal)> {
    vec![
        (0, dec!(100000)),
        (60, dec!(100800)),
        (120, dec!(101500)),
        (180, dec!(101200)),
        (240, dec!(100400)),
        (300, dec!(101900)),
        (360, dec!(102600)),
        (420, dec!(103100)),
    ]
}

fn live_snap(secs: i64, equity: Decimal) -> PnlSnapshot {
    let as_of = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs));
    PnlSnapshot {
        cash: Money::<Usdt>::from_decimal(equity),
        unrealized: Money::<Usdt>::from_decimal(dec!(0)),
        realized: Money::<Usdt>::from_decimal(dec!(0)),
        total_equity: Money::<Usdt>::from_decimal(equity),
        daily_return: Money::<Usdt>::from_decimal(dec!(0)),
        as_of,
        bar_ts: None,
    }
}

/// **Live populated render.** Drive a realistic `PnlRefreshed` session through
/// the production `update` path, render the REAL Live screen via the full
/// shell, save the PNG, and assert the equity curve actually rasterized in the
/// detail area (many curve-hue px). The model preconditions (buffer len, Ready)
/// are asserted too so a model regression is distinguished from a render one.
#[test]
fn audit_live_populated_renders_curve() {
    let session = live_session();
    let n = session.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in &session {
        update(&mut c, Message::PnlRefreshed(live_snap(*secs, *eq)));
    }
    assert_eq!(c.live_equity_buffer.len(), n, "all live points must land");
    assert_eq!(
        c.live_equity_curve.variant_name(),
        "ready",
        "Live curve must be Ready after a multi-point session"
    );

    let (w, h, rgba) = render_shell(c);
    let path = save_png("live-populated", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits > 1000,
        "Live populated: the equity curve did NOT rasterize (expected >1000 \
         ACCENT/UP_500 px, got {hits}). PNG: {path}"
    );
}

/// **Live cold negative control.** A fresh cockpit with ZERO snapshots shows
/// the Loading "no equity data" body — near-zero curve-hue px. Proves the
/// populated assertion above genuinely discriminates the drawn curve from
/// chrome.
#[test]
fn audit_live_cold_draws_no_curve() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    let (w, h, rgba) = render_shell(c);
    let path = save_png("live-empty", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits < 600,
        "Live cold state must NOT paint a curve (expected <600 stray chrome px, \
         got {hits}). If high, the curve-hue discriminator is unreliable. PNG: {path}"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// BASELINE — the gap: first-ever pixel render of the populated BH curve
// ════════════════════════════════════════════════════════════════════════════

/// **Baseline populated render — the audit's primary new guard.** Boot-load
/// the committed realized buy-and-hold curves via the PRODUCTION
/// `baseline::load_into` path (reads the committed
/// `spec/runbooks/artifacts/passive-baseline-2026-06-08/bh-equity-curve-*.csv`),
/// render the REAL Baseline screen for BOTH years through the full shell, save
/// each PNG, and assert the realized BH polyline actually rasterized.
///
/// Skip-if-absent: a minimal checkout that prunes the runbook CSVs leaves the
/// curve in `Error` (the `baseline_error_state.rs` gate covers that path);
/// this guard targets the populated path that nothing else pixel-verified.
#[test]
fn audit_baseline_populated_renders_curve() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Baseline;
    ui::baseline::load_into(&mut c);

    // Precondition: at least the 2024 (default) curve loaded Ready. If the
    // committed CSVs were pruned, skip (Error path is covered elsewhere).
    let ready_2024 = matches!(
        c.baseline_screen_state.curve_2024,
        ui::state::PanelState::Ready(_)
    );
    if !ready_2024 {
        eprintln!("baseline 2024 CSV pruned/absent — skipping populated render guard");
        return;
    }

    // Default year (2024) — the cold-start screen.
    c.baseline_screen_state.active_year = BaselineYear::Y2024;
    let (w, h, rgba) = render_shell(c.clone());
    let path_2024 = save_png("baseline-2024-populated", w, h, &rgba);
    let hits_2024 = curve_pixels_right(w, h, &rgba);
    assert!(
        hits_2024 > 1000,
        "Baseline 2024 populated: the realized BH curve did NOT rasterize \
         (expected >1000 ACCENT/UP_500 px, got {hits_2024}). The screen is Ready \
         but the canvas painted no visible polyline — a render/wiring bug in \
         screens::baseline. PNG: {path_2024}"
    );

    // Toggle to 2023 (the other data-bearing curve) — proves the year switch
    // re-renders a populated curve, not a stale/blank one.
    if matches!(
        c.baseline_screen_state.curve_2023,
        ui::state::PanelState::Ready(_)
    ) {
        c.baseline_screen_state.active_year = BaselineYear::Y2023;
        let (w, h, rgba) = render_shell(c);
        let path_2023 = save_png("baseline-2023-populated", w, h, &rgba);
        let hits_2023 = curve_pixels_right(w, h, &rgba);
        assert!(
            hits_2023 > 1000,
            "Baseline 2023 populated: the realized BH curve did NOT rasterize \
             after the year toggle (expected >1000 px, got {hits_2023}). PNG: {path_2023}"
        );
    }
}

/// **Baseline negative control.** Force both curves into `Error` (loader on a
/// bogus path — the fixtures-only-checkout shape) and render. The curve + band
/// show their muted Error body → near-zero curve-hue px, while the KPI strip
/// stays populated from the const. Proves the populated assertion above is not
/// a tautology (the const-sourced KPI strip + chrome don't trip the curve-hue
/// discriminator).
#[test]
fn audit_baseline_error_draws_no_curve() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Baseline;
    let bogus = std::path::Path::new("/__definitely_missing__/bh.csv");
    c.baseline_screen_state.curve_2023 = ui::baseline::load_baseline_curve(bogus);
    c.baseline_screen_state.curve_2024 = ui::baseline::load_baseline_curve(bogus);
    assert!(matches!(
        c.baseline_screen_state.curve_2024,
        ui::state::PanelState::Error(_)
    ));

    let (w, h, rgba) = render_shell(c);
    let path = save_png("baseline-error", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits < 600,
        "Baseline Error state must NOT paint a curve (expected <600 stray chrome \
         px, got {hits}). If high, the curve-hue discriminator is reading the KPI \
         strip / chrome and the populated guard is a tautology. PNG: {path}"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// COMPARE — full-shell two-run overlay render (populated + empty control)
// ════════════════════════════════════════════════════════════════════════════

/// Build a `CachedCell` carrying a hydrated 6-point timestamped equity series
/// shifted to `level` (so two runs occupy disjoint y-bands → separate overlay
/// pixels). Mirrors the shape `compare::cache::scan_report_roots` produces from
/// a companion CSV — but injected directly so the render is checkout-independent.
fn compare_cell(level: i64, multi: bool) -> CachedCell {
    // Six points one bar-minute apart, anchored at a fixed epoch base.
    let base_ms: i64 = 1_705_320_000 * 1000; // FIXED_EPOCH_SECS (Jan 2024) * 1000
    let offsets = [0i64, 3000, 1000, -3000, 4000, 8000];
    let series: Vec<(i64, Decimal)> = offsets
        .iter()
        .enumerate()
        .map(|(i, off)| (base_ms + (i as i64) * 60_000, Decimal::from(level + off)))
        .collect();
    CachedCell {
        sharpe: 0.94,
        total_return_pct: 12.3,
        max_drawdown_pct: -5.6,
        trade_count: 42,
        equity_curve_tail: offsets.iter().map(|o| (level + o) as f64).collect(),
        equity_series_ts: series,
        source_report_path: SmolStr::new("/fixture/report.md"),
        generated_at: SmolStr::new("2026-06-01T12:00:00Z"),
        is_multi_symbol: multi,
    }
}

/// Two overlay slots keyed exactly as the cache stores them.
fn compare_slots() -> (OverlaySlot, OverlaySlot) {
    let range = DateRange::default();
    (
        (
            SmolStr::new("top10_momentum_h1"),
            trading_core::Symbol::new("XRPUSDT"),
            range.clone(),
        ),
        (
            SmolStr::new("btc_sma_cross"),
            trading_core::Symbol::new("BTCUSDT"),
            range,
        ),
    )
}

/// Build a Compare cockpit with two hydrated cells installed in the cache.
fn compare_cockpit_two_cells() -> (Cockpit, OverlaySlot, OverlaySlot) {
    let (slot_a, slot_b) = compare_slots();
    let mut cache: BTreeMap<OverlaySlot, CachedCell> = BTreeMap::new();
    cache.insert(slot_a.clone(), compare_cell(100_000, true));
    cache.insert(slot_b.clone(), compare_cell(60_000, false));

    let mut c = Cockpit::new();
    c.current_screen = Screen::Compare;
    c.compare_screen_state = CompareScreenState {
        cache,
        ..CompareScreenState::default()
    };
    (c, slot_a, slot_b)
}

/// **Compare populated render.** Install two hydrated cells, select BOTH
/// through the production `CompareToggleOverlay` ring, render the REAL Compare
/// screen via the full shell, save the PNG, and assert the overlay chart band
/// paints both curves (counted as right-pane curve hue — the overlay sits at
/// the bottom of the body, well right of the sidebar).
#[test]
fn audit_compare_overlay_populated_renders() {
    let (mut c, slot_a, slot_b) = compare_cockpit_two_cells();
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));
    assert_eq!(
        c.compare_screen_state.overlay_selection.len(),
        2,
        "both runs must be in the overlay ring"
    );

    let (w, h, rgba) = render_shell(c);
    let path = save_png("compare-overlay-populated", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits > 500,
        "Compare overlay populated: the two-run overlay did NOT rasterize \
         (expected >500 ACCENT/ACCENT_2 px in the body, got {hits}). The cells' \
         equity_series_ts did not reach chart::view through overlay_panel. PNG: {path}"
    );
}

/// **Compare empty negative control.** No selection → the overlay shows its
/// "pick runs to compare" prompt; near-zero curve-hue px. Proves the populated
/// assertion above genuinely discriminates the drawn overlay from chrome.
#[test]
fn audit_compare_empty_draws_no_curve() {
    let (c, _a, _b) = compare_cockpit_two_cells();
    // No CompareToggleOverlay → overlay_selection stays empty.
    let (w, h, rgba) = render_shell(c);
    let path = save_png("compare-empty", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits < 600,
        "Compare empty state must NOT paint an overlay curve (expected <600 \
         stray chrome px, got {hits}). PNG: {path}"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// REPORTS — full-shell populated render (eyeball pass; deep coverage lives in
// reports_populated_curve_render.rs). Synthetic in-memory series → checkout-
// independent.
// ════════════════════════════════════════════════════════════════════════════

/// Build a Reports cockpit with one entry selected + loaded to a `Ready`
/// report whose equity is the supplied state (synthetic; the `path` is never
/// read because `loaded` is pre-populated).
fn reports_cockpit(equity: ui::state::PanelState<trading_core::EquitySeries>) -> Cockpit {
    use ui::reports::{ReportEntry, ReportsScreenState};
    use ui::viewer::{ReportFrontMatter, ReportLoadResult};

    let mut c = Cockpit::new();
    c.current_screen = Screen::Reports;
    let entry = ReportEntry {
        slug: SmolStr::new("v0-paper-sma"),
        file_stem: SmolStr::new("backtest-20260617-180015-btc-2024-h1-sma-cross"),
        path: PathBuf::from("/fixture/report.md"),
        has_companion: true,
    };
    let metrics = trading_core::BacktestMetrics {
        total_return_pct: dec!(7.38),
        cagr_pct: dec!(3.60),
        cagr_present: true,
        sharpe: dec!(7.7975),
        sharpe_present: true,
        max_drawdown_pct: dec!(4.20),
        win_rate_pct: dec!(52.0),
        win_rate_present: true,
        trades: 441,
    };
    let loaded = ReportLoadResult {
        front_matter: ReportFrontMatter {
            scenario: SmolStr::new("btc-2024-h1-sma-cross"),
        },
        metrics: ui::state::PanelState::Ready(metrics),
        equity,
        body_markdown: "# Backtest Report — btc-2024-h1-sma-cross\n\n## Summary\nrow\n".to_string(),
    };
    c.reports_screen_state = ReportsScreenState {
        discovered: ui::state::PanelState::Ready(vec![entry]),
        selected: Some(0),
        loaded: ui::state::PanelState::Ready(loaded),
        // reports-picker-curve-filter — default curve-only filter.
        show_all_reports: false,
    };
    c
}

/// **Reports populated render (eyeball pass).** Renders the real Reports screen
/// with a `Ready` equity selection through the full shell, saves the PNG, and
/// asserts the curve drew. The authoritative discriminator + marker/auto-select
/// guards live in `reports_populated_curve_render.rs`; this is the operator
/// eyeball deliverable for the Group-A sweep.
#[test]
fn audit_reports_populated_renders_curve() {
    let series = ui::fixtures::fake_equity_series_for_viewer();
    let c = reports_cockpit(ui::state::PanelState::Ready(series));
    let (w, h, rgba) = render_shell(c);
    let path = save_png("reports-audit-populated", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits > 1000,
        "Reports populated: the equity curve did NOT rasterize (expected >1000 \
         ACCENT/UP_500 px, got {hits}). PNG: {path}"
    );
}

/// **Reports empty negative control.** `equity: Empty` → "No equity data" body;
/// near-zero curve-hue px.
#[test]
fn audit_reports_empty_draws_no_curve() {
    let c = reports_cockpit(ui::state::PanelState::Empty);
    let (w, h, rgba) = render_shell(c);
    let path = save_png("reports-audit-empty", w, h, &rgba);
    let hits = curve_pixels_right(w, h, &rgba);
    assert!(
        hits < 600,
        "Reports Empty state must NOT paint a curve (expected <600 stray chrome \
         px, got {hits}). PNG: {path}"
    );
}
