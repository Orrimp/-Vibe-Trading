//! Render-verifiable harness for the Live equity curve — the regression
//! guard that exercises the **real render path** (cockpit-live-equity-render-guard,
//! 2026-06-11).
//!
//! ## Why this file exists (the lesson)
//!
//! Every prior fix to the Live equity curve was verified at the **agent /
//! unit-test layer** (publishing a `PnlSnapshot`, or asserting
//! `live_equity_buffer.len()`), which passed — while the **actually rendered
//! curve** was broken and the operator kept seeing "no graph". The
//! `state.rs` headless tests assert the model `PanelState` transitions; the
//! `panel_snapshots.rs` `live_screen_summary` builds a TEXT summary — neither
//! rasterizes the canvas. So a bug that drops the curve's points (the bar-time
//! monotone-guard regression) sails through all of them.
//!
//! This harness closes that gap: it drives a realistic sequence of
//! `Message::PnlRefreshed` snapshots into the cockpit state (the production
//! `update` path), renders the **real Live screen** via `iced_test::screenshot`
//! (which runs `view` → `EquityCurveProgram::draw` → tiny_skia rasterization,
//! the same path the live `cockpit_live` binary paints), and inspects the
//! curve region of the RGBA buffer for the polyline's `ACCENT`-colored pixels.
//!
//! An empty / Loading / degenerate curve draws **zero** `ACCENT` pixels in the
//! curve band (it shows the muted "No equity data" body on a `PANEL`
//! background; gridlines are `BORDER_1`, never `ACCENT`). A real Ready curve
//! draws a visible `ACCENT` polyline. So `ACCENT`-pixel-count > threshold
//! ⟺ the curve actually rendered. THIS is the signal the prior tests lacked.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Money, PnlSnapshot, Timestamp, Usdt};

use ui::state::{Cockpit, Message, Screen, update};
use ui::test_support::program_from_cockpit;
use ui::theme::{ThemeMode, color};

// Floor viewport — the same 1280×720 the headless-emulator smoke uses.
const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 720;
const SCALE: f32 = 1.0;

/// Curve-region crop window in physical pixels (scale 1.0 → logical == physical).
///
/// The Live screen layout (`screens/live.rs`) stacks
/// `headline → health_strip → equity_curve(240px) → kpi_row → caption →
/// bottom_row` inside the shell (sidebar LEFT = `SIDEBAR_WIDTH_PX` 180px). The
/// equity curve lands below the headline + health strip and is 240px tall.
///
/// We crop a generous band that (a) starts well RIGHT of the 180px sidebar (so
/// the sidebar's active-item `ACCENT` highlight can never leak into the count)
/// and (b) spans the vertical zone where the 240px curve canvas paints. The
/// `accent_pixel_stats` diagnostic below pins the actual `ACCENT` bounding box
/// so this window is verified empirically, not guessed.
const CROP_X: u32 = 220;
const CROP_W: u32 = 980; // 220..1200 — inside the 1280 frame, right of sidebar
const CROP_Y: u32 = 130;
const CROP_H: u32 = 290; // 130..420 — the equity-curve vertical zone (verified
// by diag_accent_bounding_box: a healthy curve's ACCENT bbox lands at
// y≈202..370, x≈271..1199, comfortably inside this window)

/// Per-channel tolerance for an `ACCENT` pixel match. tiny_skia anti-aliases
/// the polyline edges, so an exact byte match would only catch the line's
/// interior. A ±40 window over the 8-bit channels catches the AA ramp without
/// admitting unrelated colors (`PANEL` dark = (28,33,39) is ~150 away from
/// `ACCENT` dark = (111,182,174) on the green channel — nowhere near).
const CHANNEL_TOL: i32 = 40;

/// One realistic Live session: a monotone-non-decreasing wallclock-stamped
/// sequence of `(seconds_since_epoch, equity)` points, the shape the agent's
/// reconciler publishes once per bar. Rises, dips (so Max-DD is non-zero),
/// recovers — a curve with visible vertical movement.
fn session_points() -> Vec<(i64, Decimal)> {
    vec![
        (0, dec!(100000)),
        (60, dec!(100800)),
        (120, dec!(101500)),
        (180, dec!(101200)),
        (240, dec!(100400)), // dip → drawdown
        (300, dec!(101900)),
        (360, dec!(102600)),
        (420, dec!(103100)),
    ]
}

/// Build a `PnlSnapshot` with `as_of` = the given wallclock second and the
/// given total equity. Mirrors the production reconciler publish shape.
fn snap(secs: i64, equity: Decimal) -> PnlSnapshot {
    let as_of = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs));
    PnlSnapshot {
        cash: Money::<Usdt>::from_decimal(equity),
        unrealized: Money::<Usdt>::from_decimal(dec!(0)),
        realized: Money::<Usdt>::from_decimal(dec!(0)),
        total_equity: Money::<Usdt>::from_decimal(equity),
        daily_return: Money::<Usdt>::from_decimal(dec!(0)),
        as_of,
        // This harness exercises the curve's RASTERIZATION over a monotone
        // wallclock sequence; `None` makes the x-coord fall back to `as_of`,
        // preserving the calibrated ACCENT bbox above. The bar_ts→x-axis
        // mapping is asserted at the unit level by
        // `live_equity_curve_plots_bar_ts_not_wallclock` (state.rs).
        bar_ts: None,
    }
}

/// Render the Live screen of the given cockpit and return its RGBA screenshot.
fn render_live(cockpit: Cockpit) -> iced::window::Screenshot {
    // Snapshot-determinism: render the bottom time-axis labels at UTC so this
    // harness is machine-independent (matches the visual-snapshot contract).
    // SAFETY: test-only single-threaded env init before iced_test::screenshot.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// `(r, g, b)` 0-255 of an `ACCENT` (dark theme) pixel — the polyline color.
fn accent_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

/// Does `(r,g,b)` match `ACCENT` within `CHANNEL_TOL` on every channel?
fn is_accent(r: u8, g: u8, b: u8) -> bool {
    let (ar, ag, ab) = accent_rgb();
    (i32::from(r) - ar).abs() <= CHANNEL_TOL
        && (i32::from(g) - ag).abs() <= CHANNEL_TOL
        && (i32::from(b) - ab).abs() <= CHANNEL_TOL
}

/// Count `ACCENT`-colored pixels inside the curve crop window, and report the
/// bounding box of those pixels (for the diagnostic). The bounding box lets a
/// human verify the crop window actually frames the curve.
struct AccentStats {
    count: usize,
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
}

fn accent_pixel_stats(shot: &iced::window::Screenshot) -> AccentStats {
    let w = shot.size.width;
    let h = shot.size.height;
    let rgba: &[u8] = &shot.rgba;
    let mut count = 0usize;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (u32::MAX, 0u32, u32::MAX, 0u32);

    let x0 = CROP_X.min(w);
    let x1 = (CROP_X + CROP_W).min(w);
    let y0 = CROP_Y.min(h);
    let y1 = (CROP_Y + CROP_H).min(h);

    for y in y0..y1 {
        for x in x0..x1 {
            let idx = ((y * w + x) * 4) as usize;
            // Bounds-safety: the buffer is w*h*4; idx+2 is always < len here.
            if idx + 2 >= rgba.len() {
                continue;
            }
            if is_accent(rgba[idx], rgba[idx + 1], rgba[idx + 2]) {
                count += 1;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    AccentStats {
        count,
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

// ── Diagnostic: pin the ACCENT bounding box (run with --nocapture) ──────────

/// Diagnostic that renders a healthy Ready curve and prints the `ACCENT`
/// pixel bounding box + count, so the crop window above can be verified
/// empirically rather than guessed. Not an assertion gate — `--nocapture`
/// surfaces the numbers; kept as a `#[test]` so it compiles + runs in CI and
/// documents the observed geometry in the log.
#[test]
fn diag_accent_bounding_box() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in session_points() {
        update(&mut c, Message::PnlRefreshed(snap(secs, eq)));
    }
    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);
    eprintln!(
        "[diag] ACCENT pixels in crop ({CROP_X}..{},{CROP_Y}..{}): count={} bbox=({}..{}, {}..{})",
        CROP_X + CROP_W,
        CROP_Y + CROP_H,
        stats.count,
        stats.min_x,
        stats.max_x,
        stats.min_y,
        stats.max_y,
    );
    eprintln!(
        "[diag] frame size = {}x{}, ACCENT rgb (dark) = {:?}",
        shot.size.width,
        shot.size.height,
        accent_rgb()
    );
}

// ── PHASE 1 — the render-verifiable assertion (the regression guard) ────────

/// Minimum `ACCENT` pixels that constitute a "the curve actually drew" signal.
/// A healthy 8-point session draws ~1200 ACCENT pixels (see
/// `diag_accent_bounding_box`); a degenerate / empty curve draws 0. 200 is a
/// comfortable floor that no AA-noise or single-dot artifact can reach, while
/// staying far below the ~1200 a real polyline produces.
const CURVE_DREW_MIN_ACCENT: usize = 200;

/// Minimum horizontal extent (in px) of the `ACCENT` pixels — a real curve's
/// polyline spans most of the panel width (the healthy case spans ~947px). A
/// single-point dot or a vertical-only artifact would fail this. This is the
/// "sane x-span" assertion: the curve must traverse the time axis, not collapse
/// to one x.
const CURVE_X_SPAN_MIN: u32 = 400;

/// **AC1 (render proof).** Drive a realistic monotone wallclock-stamped
/// `PnlRefreshed` sequence through the production `update` path, render the
/// REAL Live screen, and assert the equity curve actually drew:
///   (i)   the model buffer holds every point (no drops),
///   (ii)  the curve `PanelState` is `Ready`,
///   (iii) the rendered polyline paints a non-trivial number of `ACCENT`
///         pixels (the curve is on-screen, not a blank "no graph" panel), and
///   (iv)  those pixels span a sane horizontal range (the curve traverses the
///         time axis — not a degenerate single-x dot).
///
/// This is the test that would have caught every "the unit test passed but the
/// operator sees no graph" miss this session: it asserts on rasterized pixels,
/// not on `PnlSnapshot` publishes or model state alone.
#[test]
fn live_equity_curve_actually_renders() {
    let points = session_points();
    let n = points.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in &points {
        update(&mut c, Message::PnlRefreshed(snap(*secs, *eq)));
    }

    // (i) every monotone point landed in the buffer — none dropped.
    assert_eq!(
        c.live_equity_buffer.len(),
        n,
        "all {n} monotone points must be retained (none dropped by the guard)"
    );
    // (ii) the model says Ready.
    assert_eq!(
        c.live_equity_curve.variant_name(),
        "ready",
        "curve must be Ready after a multi-point session"
    );

    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);

    // (iii) the polyline rasterized — a real curve, not a blank panel.
    assert!(
        stats.count >= CURVE_DREW_MIN_ACCENT,
        "equity curve did NOT render: only {} ACCENT pixels in the curve band \
         (expected ≥ {CURVE_DREW_MIN_ACCENT}). This is the 'no graph' bug — the \
         model is Ready but the canvas painted no visible polyline.",
        stats.count,
    );
    // (iv) the curve spans a sane horizontal range (traverses the x/time axis).
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span >= CURVE_X_SPAN_MIN,
        "equity curve x-span is degenerate: ACCENT pixels span only {x_span}px \
         (expected ≥ {CURVE_X_SPAN_MIN}px). A real curve traverses the time axis; \
         a single-point dot or collapsed series would land here.",
    );
}

// ── PHASE 1 — PROVE THE HARNESS CATCHES THE BUG ─────────────────────────────

/// **The regression-guard proof (TODO §1 failing case).** Reconstruct the
/// reverted-I1 bar-time / monotone-guard scenario and assert the harness SEES
/// the empty/degenerate curve.
///
/// ## The bug being reproduced
///
/// I1 (reverted in `40f5de9`) stamped `PnlSnapshot.as_of = bar.close_ts` (the
/// 2023 data time) instead of `Timestamp::now()`. During a FAST replay the
/// agent first publishes a snapshot, then the next bar's `close_ts` can be
/// EARLIER on the wall axis than what the buffer already holds — and
/// `push_live_equity_point`'s monotone guard (`as_of < back.ts` ⇒ drop) drops
/// every such out-of-order point. The buffer never grows past its first point,
/// `EquitySeries::from_points` builds a 1-point series, and the rendered curve
/// collapses to a single dot → the operator's "no graph, no movement".
///
/// Here we feed exactly that pathology: a first point, then a run of snapshots
/// whose `as_of` goes strictly BACKWARDS (descending) — the signature of
/// bar-time stamping after a wallclock-anchored open during fast replay. The
/// guard drops them all.
///
/// ## What this proves
///
/// If a future change re-introduces the bar-time regression (or any change
/// that lets the curve's points get dropped / collapse), THIS test goes red
/// because the rendered curve loses its `ACCENT` polyline. The prior
/// model-only / text-summary tests would NOT catch it — that is the entire
/// reason this harness exists.
#[test]
fn harness_catches_dropped_points_empty_curve() {
    // The failing case: one good open point, then 6 snapshots whose `as_of`
    // marches BACKWARDS (descending), as bar-time stamping produces during a
    // fast replay against a wallclock-anchored first point.
    let dropped_seq: Vec<(i64, Decimal)> = vec![
        (1_000_000, dec!(100000)), // first point lands
        (700, dec!(100800)),       // as_of < back → dropped
        (650, dec!(101500)),       // dropped
        (600, dec!(101200)),       // dropped
        (550, dec!(100400)),       // dropped
        (500, dec!(101900)),       // dropped
        (450, dec!(102600)),       // dropped
    ];

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in &dropped_seq {
        update(&mut c, Message::PnlRefreshed(snap(*secs, *eq)));
    }

    // Model-level proof of the drop: only the first point survived the guard.
    assert_eq!(
        c.live_equity_buffer.len(),
        1,
        "the monotone guard must drop every backwards-`as_of` point, leaving \
         only the first (this is the bar-time regression's mechanism)"
    );

    // The render-level proof — THE POINT OF THIS HARNESS. A 1-point series is a
    // degenerate curve: the canvas paints (at most) a single dot, NOT a
    // traversing polyline. The harness sees the curve is effectively absent.
    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);

    assert!(
        stats.count < CURVE_DREW_MIN_ACCENT,
        "REGRESSION-GUARD SELF-CHECK FAILED: the dropped-points scenario should \
         render an EMPTY/degenerate curve, but the harness saw {} ACCENT pixels \
         (≥ {CURVE_DREW_MIN_ACCENT}). If this assertion ever fails it means the \
         harness can no longer distinguish a broken curve from a healthy one — \
         fix the harness before trusting it.",
        stats.count,
    );

    // And the x-span proof: with a single retained point there is no horizontal
    // traverse — the curve cannot span the time axis.
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span < CURVE_X_SPAN_MIN,
        "the degenerate 1-point curve must not span the time axis, but ACCENT \
         pixels spanned {x_span}px (≥ {CURVE_X_SPAN_MIN}px expected only for a \
         real multi-point curve)",
    );
}

/// **The contrast pair (belt-and-braces).** Render the SAME screen twice — once
/// healthy, once in the dropped-points failure mode — and assert the healthy
/// curve paints strictly MORE `ACCENT` than the broken one by a wide margin.
/// This pins the harness's discriminating power as a single relational fact
/// (healthy ≫ broken), independent of the absolute thresholds above, so the
/// guard can never silently degrade into "both look the same".
#[test]
fn healthy_curve_draws_far_more_than_broken() {
    // Healthy.
    let mut healthy = Cockpit::new();
    healthy.current_screen = Screen::Live;
    for (secs, eq) in session_points() {
        update(&mut healthy, Message::PnlRefreshed(snap(secs, eq)));
    }
    let healthy_shot = render_live(healthy);
    let healthy_accent = accent_pixel_stats(&healthy_shot).count;

    // Broken (the dropped-points pathology).
    let mut broken = Cockpit::new();
    broken.current_screen = Screen::Live;
    let backwards: Vec<(i64, Decimal)> = vec![
        (1_000_000, dec!(100000)),
        (700, dec!(100800)),
        (650, dec!(101500)),
        (600, dec!(101200)),
    ];
    for (secs, eq) in backwards {
        update(&mut broken, Message::PnlRefreshed(snap(secs, eq)));
    }
    let broken_shot = render_live(broken);
    let broken_accent = accent_pixel_stats(&broken_shot).count;

    assert!(
        healthy_accent > broken_accent * 4 + 100,
        "harness must strongly distinguish healthy ({healthy_accent} ACCENT px) \
         from broken ({broken_accent} ACCENT px); the healthy curve should draw \
         a polyline an order of magnitude larger than the broken curve's noise"
    );
}

// ── Render-crash regression (a bonus bug this harness caught) ────────────────

/// **Render-crash regression — flat / single-point series.**
/// cockpit-live-equity-render-guard (2026-06-11): building this harness
/// surfaced a real production crash. A `Ready` equity curve holding ONE point
/// (the first live bar after boot — the design renders the curve from ≥1
/// point), or any all-equal run, made the Y-range collapse so the rasterizer
/// got a NaN `Point.y` and panicked inside `lyon_path`
/// (`assertion failed: p.y.is_finite()`). That is the cockpit crashing the
/// instant the agent publishes its first P&L snapshot.
///
/// The fix (`widgets/equity_curve.rs`) guards the Y denominator and centers a
/// flat line. This test pins it: rendering a 1-point curve AND a multi-point
/// FLAT (all-equal) curve must NOT panic. Before the fix both panicked; the
/// model-level `state.rs` tests never exercised the rasterizer so they sailed
/// past it — the exact gap this render harness exists to close.
#[test]
fn flat_and_single_point_curves_render_without_panic() {
    // (a) Single point — Ready from one bar (the first-bar-after-boot case).
    let mut one = Cockpit::new();
    one.current_screen = Screen::Live;
    update(&mut one, Message::PnlRefreshed(snap(0, dec!(100000))));
    assert_eq!(one.live_equity_buffer.len(), 1);
    assert_eq!(one.live_equity_curve.variant_name(), "ready");
    // The assertion under test is simply: this does not panic.
    let _ = render_live(one);

    // (b) Multi-point FLAT line — equity never moves (a paused / no-fill
    // session). All-equal → the same Y-range collapse, different point count.
    let mut flat = Cockpit::new();
    flat.current_screen = Screen::Live;
    for secs in [0i64, 60, 120, 180, 240] {
        update(&mut flat, Message::PnlRefreshed(snap(secs, dec!(100000))));
    }
    assert_eq!(flat.live_equity_buffer.len(), 5);
    assert_eq!(flat.live_equity_curve.variant_name(), "ready");
    // A flat multi-point line renders a finite horizontal traverse — and must
    // not panic. (It draws ACCENT, but we only gate "no panic" here; the
    // value-rich assertions live in `live_equity_curve_actually_renders`.)
    let _ = render_live(flat);
}
