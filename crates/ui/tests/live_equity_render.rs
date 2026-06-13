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

// ── PHASE 2 — live-equity-history-durable: the HYDRATED-boot render gate ─────
//
// AC6 (THE gate, R5): a cockpit hydrated from a durable store tail (zero live
// snapshots) must rasterize a real ACCENT polyline. AC5: a live append AFTER
// the hydrate must still land and the curve still draw + grow — the A4 `as_of`
// delivery-guard reconciliation proven at the pixel layer. These use REALISTIC
// two-timestamp rows: 2023-era `bar_ts` (the plotted x-coord, an old data date)
// paired with PRIOR-SESSION `as_of` wallclock stamps (all in the past relative
// to a live `now()`) — exactly the guard-reconciliation scenario the architect
// flagged as the riskiest (a fresh `now()` live tick must out-rank the stale
// hydrated `as_of` max and append, not be dropped).

/// 2023-data-time base for the hydrate tail's `bar_ts` (the plotted x-coord):
/// 2023-01-15 12:30:00 UTC. The curve self-scales to its own x-range, so the
/// absolute epoch does not move the ACCENT bbox out of the crop band.
const HYDRATE_BAR_BASE: i64 = 1_673_789_400;

/// Prior-session wallclock base for the hydrate tail's `as_of` (the delivery
/// key): ~2025-04-30, comfortably in the past vs. a live `Timestamp::now()`.
const HYDRATE_AS_OF_BASE: i64 = 1_746_000_000;

/// Build one durable-tail row `(bar_ts, as_of, equity)` from a 2023 `bar_ts`
/// second and a prior-session `as_of` second — the exact tuple shape
/// `audit::query::equity_snapshot_tail` returns and `Message::PnlHydrated`
/// consumes.
fn hydrate_row(
    bar_secs: i64,
    as_of_secs: i64,
    equity: Decimal,
) -> (Timestamp, Timestamp, Money<Usdt>) {
    (
        Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(bar_secs)),
        Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(as_of_secs)),
        Money::<Usdt>::from_decimal(equity),
    )
}

/// A realistic ≥2-row hydrate tail mirroring `session_points`' equity SHAPE
/// (rises, dips for a non-zero drawdown, recovers — same vertical span) so the
/// rasterized ACCENT geometry is directly comparable to the live-path harness.
/// `bar_ts` and `as_of` advance one minute per row, in disjoint epoch ranges.
fn hydrate_tail() -> Vec<(Timestamp, Timestamp, Money<Usdt>)> {
    session_points()
        .into_iter()
        .enumerate()
        .map(|(i, (secs, eq))| {
            // `secs` (0..420) seeds the per-row offset for BOTH timestamps so the
            // tail is monotone in bar_ts and as_of, in their separate epochs.
            let _ = secs;
            let off = i as i64 * 60;
            hydrate_row(HYDRATE_BAR_BASE + off, HYDRATE_AS_OF_BASE + off, eq)
        })
        .collect()
}

/// **AC6 (THE gate).** A cockpit hydrated by ONE `Message::PnlHydrated` (a faked
/// ≥2-row durable tail) with **zero `PnlRefreshed`** must rasterize a real curve:
///   (i)   the buffer holds every hydrated row,
///   (ii)  the curve `PanelState` is `Ready`,
///   (iii) the rendered polyline paints ≥ `CURVE_DREW_MIN_ACCENT` ACCENT pixels
///         (the durable history is on-screen, not a blank "no graph" panel), and
///   (iv)  those pixels span a sane horizontal range (`≥ CURVE_X_SPAN_MIN`).
///
/// A model-Ready-but-blank-canvas regression fails here — this is the render
/// proof the restart-hydrated curve actually draws before the first new bar.
#[test]
fn hydrated_boot_curve_actually_renders() {
    let tail = hydrate_tail();
    let n = tail.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    // ONE batch hydrate through the production update path; NO live tick.
    update(&mut c, Message::PnlHydrated(tail));

    // (i) every hydrated row landed in the buffer.
    assert_eq!(
        c.live_equity_buffer.len(),
        n,
        "all {n} hydrated rows must seed the buffer (none dropped)"
    );
    // (ii) the model says Ready off the hydrate alone.
    assert_eq!(
        c.live_equity_curve.variant_name(),
        "ready",
        "curve must be Ready after a ≥2-row hydrate, before any live tick"
    );
    // (and the since-inception switch flipped — the caption is mode-correct).
    assert!(c.live_equity_hydrated);

    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);

    // (iii) the hydrated polyline rasterized — a real curve, not a blank panel.
    assert!(
        stats.count >= CURVE_DREW_MIN_ACCENT,
        "HYDRATED curve did NOT render: only {} ACCENT pixels in the curve band \
         (expected ≥ {CURVE_DREW_MIN_ACCENT}). The buffer is seeded + Ready but \
         the canvas painted no visible polyline — the restart-blank-curve bug.",
        stats.count,
    );
    // (iv) the curve spans a sane horizontal range (traverses the x/time axis).
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span >= CURVE_X_SPAN_MIN,
        "hydrated curve x-span is degenerate: ACCENT pixels span only {x_span}px \
         (expected ≥ {CURVE_X_SPAN_MIN}px). A real multi-row history traverses \
         the time axis.",
    );
}

/// **AC5 (the guard-reconciliation render proof).** Hydrate from a durable tail
/// whose `as_of` values are all in the PAST, THEN deliver ONE live
/// `PnlRefreshed(now())`. The fresh `now()` wallclock out-ranks the stale
/// hydrated `as_of` max, so the live point MUST append (not be dropped) — and
/// the curve must still draw AND grow. This is the A4 `as_of` delivery-guard
/// contract proven at the rasterized layer (the riskiest decision, the one the
/// model-layer AC5 also pins — here at the pixel layer the brief demands).
#[test]
fn live_append_after_hydrate_still_renders_and_grows() {
    let tail = hydrate_tail();
    let seeded = tail.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    update(&mut c, Message::PnlHydrated(tail));
    assert_eq!(c.live_equity_buffer.len(), seeded);

    // Baseline geometry of the hydrated-only curve. We compare the horizontal
    // EXTENT (x-span) rather than the raw ACCENT count, because appending a new
    // peak rescales the curve's Y-axis (the auto-fit range grows), which can
    // shift the polyline's pixel count slightly either way — a rescale is NOT a
    // regression. The x-span is the rescale-invariant "grew" signal: a forward
    // bar extends the curve along the time axis.
    let before_x_span = {
        let s = accent_pixel_stats(&render_live(c.clone()));
        s.max_x.saturating_sub(s.min_x)
    };

    // A live snapshot: fresh `now()` wallclock `as_of` (≥ every prior-session
    // hydrated `as_of`) + a `bar_ts` one minute AFTER the 2023 tail's last bar,
    // so the curve extends forward in DATA time too.
    let next_bar = HYDRATE_BAR_BASE + (seeded as i64) * 60;
    let live = PnlSnapshot {
        cash: Money::<Usdt>::from_decimal(dec!(104000)),
        unrealized: Money::<Usdt>::from_decimal(dec!(0)),
        realized: Money::<Usdt>::from_decimal(dec!(0)),
        total_equity: Money::<Usdt>::from_decimal(dec!(104000)),
        daily_return: Money::<Usdt>::from_decimal(dec!(0)),
        as_of: Timestamp::now(),
        bar_ts: Some(Timestamp::new(
            time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(next_bar),
        )),
    };
    update(&mut c, Message::PnlRefreshed(live));

    // Model proof: the live point landed (the guard did NOT drop it).
    assert_eq!(
        c.live_equity_buffer.len(),
        seeded + 1,
        "the first live snapshot after hydrate must append at the model layer"
    );

    // Render proof (i): the curve still draws a real polyline after the append.
    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);
    assert!(
        stats.count >= CURVE_DREW_MIN_ACCENT,
        "post-hydrate live-append curve did NOT render: only {} ACCENT pixels \
         (expected ≥ {CURVE_DREW_MIN_ACCENT}). The guard must not have dropped \
         the live point AND the curve must still rasterize.",
        stats.count,
    );
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    // Render proof (ii): the curve still spans the time axis (not degenerate)…
    assert!(
        x_span >= CURVE_X_SPAN_MIN,
        "post-append curve x-span is degenerate ({x_span}px < {CURVE_X_SPAN_MIN}px)",
    );
    // …and it GREW — the forward bar extended the curve's horizontal extent at
    // least as wide as the hydrated-only baseline (a dropped live point or a
    // collapsed curve would shrink it). Rescale-invariant, unlike a raw count.
    assert!(
        x_span >= before_x_span,
        "the curve must extend (or hold) its x-span after the forward live \
         append: {x_span}px now vs. {before_x_span}px hydrated-only — a shrink \
         would mean the live point was lost or the series collapsed"
    );
}

// ── PHASE 3 — ADR-0053 paper-mode-equity-wiring Y-variation gate (AC6) ────────
//
// The existing `count`/`x_span` assertions PASS on a flat equity series:
// `equity_curve.rs:178` (the flat-line guard) renders a degenerate series
// as a **centered, full-width horizontal line** that draws plenty of ACCENT
// pixels and spans the full x-axis. So `CURVE_DREW_MIN_ACCENT` and
// `CURVE_X_SPAN_MIN` do NOT distinguish a flat (flat `initial_capital` bug)
// from a real moving curve.
//
// The ONLY valid discriminator is the ACCENT bounding-box HEIGHT:
// `AccentStats` already tracks `min_y`/`max_y`. A moving curve has a tall
// bbox; the flat-line guard's centered horizontal line has a ~1-2px bbox.
// `CURVE_Y_VAR_MIN` is the new threshold that separates them.

/// Minimum ACCENT bounding-box HEIGHT (max_y − min_y) that a MOVING equity
/// curve must produce. A flat/degenerate series is rendered by the
/// `equity_curve.rs` flat-line guard as a centered horizontal line, producing
/// a bbox height of ~1–2 px (AA ramp), well below this threshold.
///
/// Calibration (via `diag_accent_bounding_box`): a healthy 8-point session
/// with rises/dips produces an ACCENT bbox spanning ~168 px vertically.
/// Setting the floor at 30 px is comfortably ABOVE the ~1–2 px flat-line
/// noise and comfortably BELOW the ~168 px healthy-curve height — the gap
/// is ~2 orders of magnitude, so this threshold is stable across themes and
/// minor layout shifts.
const CURVE_Y_VAR_MIN: u32 = 30;

/// **AC6 — Y-variation gate: non-flat moving curve passes; flat initial_capital
/// series fails (the self-proving contrast pair, ADR-0053 A6 / D5).**
///
/// This is the render-layer half of the baseline-equity-divergence gate.
/// The existing `count`/`x_span` checks do NOT discriminate flat from moving
/// (the flat-line guard renders the flat series as a full-width centered
/// horizontal line that PASSES both); only the ACCENT bbox HEIGHT distinguishes.
///
/// **Non-flat half:** a moving paper equity series (with rises and dips) renders
/// a polyline whose ACCENT bbox height ≥ `CURVE_Y_VAR_MIN`. This is the "the
/// paper loop actually produced moving equity" proof at the pixel layer.
///
/// **Flat contrast half (the self-proving proof):** a flat `initial_capital`
/// series (all-equal values — the current paper-mode bug) renders a bbox height
/// < `CURVE_Y_VAR_MIN`. It still PASSES `count`/`x_span` (the flat-line guard
/// draws it as a full-width horizontal line) — proving Y-variation is the ONLY
/// valid discriminator for the "is equity moving?" question.
#[test]
fn y_variation_gate_moving_passes_flat_fails() {
    // ── Non-flat half ─────────────────────────────────────────────────────────
    // Use `session_points()` which has rises AND dips (same as the other tests),
    // producing a curve with real vertical extent.
    let mut moving_c = Cockpit::new();
    moving_c.current_screen = Screen::Live;
    for (secs, eq) in session_points() {
        update(&mut moving_c, Message::PnlRefreshed(snap(secs, eq)));
    }
    let moving_shot = render_live(moving_c);
    let moving_stats = accent_pixel_stats(&moving_shot);
    let moving_y_span = moving_stats.max_y.saturating_sub(moving_stats.min_y);

    // A moving curve must pass the Y-variation threshold.
    assert!(
        moving_y_span >= CURVE_Y_VAR_MIN,
        "Y-VARIATION GATE FAILED for MOVING curve: ACCENT bbox height = {}px < {}px (CURVE_Y_VAR_MIN). \
         A real paper equity session with rises and dips should have a tall polyline; \
         if this fails the curve is rendering as flat when it should not.",
        moving_y_span,
        CURVE_Y_VAR_MIN,
    );
    // Confirm count/x_span also pass (belt-and-braces).
    assert!(
        moving_stats.count >= CURVE_DREW_MIN_ACCENT,
        "moving curve count check: {} < {CURVE_DREW_MIN_ACCENT}",
        moving_stats.count,
    );
    let moving_x_span = moving_stats.max_x.saturating_sub(moving_stats.min_x);
    assert!(
        moving_x_span >= CURVE_X_SPAN_MIN,
        "moving curve x_span check: {moving_x_span} < {CURVE_X_SPAN_MIN}",
    );

    // ── Flat contrast half (self-proving proof) ───────────────────────────────
    // A flat series at constant `initial_capital` — the current paper-mode bug.
    // All points have the same equity value: 100_000 (never moves).
    let mut flat_c = Cockpit::new();
    flat_c.current_screen = Screen::Live;
    for secs in [0i64, 60, 120, 180, 240, 300, 360, 420] {
        update(
            &mut flat_c,
            Message::PnlRefreshed(snap(secs, dec!(100_000))),
        );
    }
    let flat_shot = render_live(flat_c);
    let flat_stats = accent_pixel_stats(&flat_shot);
    let flat_y_span = flat_stats.max_y.saturating_sub(flat_stats.min_y);

    // A flat curve FAILS the Y-variation threshold (the flat-line guard draws it
    // as a centered horizontal line — ~1-2 px bbox height).
    assert!(
        flat_y_span < CURVE_Y_VAR_MIN,
        "Y-VARIATION SELF-PROOF FAILED for FLAT curve: ACCENT bbox height = {}px >= {}px \
         (CURVE_Y_VAR_MIN). The flat-line guard should render the flat series as a centered \
         horizontal line (~1-2px bbox). If this fails the self-proving contrast pair is broken \
         and the Y-variation gate can no longer distinguish flat from moving.",
        flat_y_span,
        CURVE_Y_VAR_MIN,
    );
    // The flat curve STILL passes count/x_span — proving those are insufficient.
    // (The flat-line guard draws a full-width centered horizontal line.)
    assert!(
        flat_stats.count >= CURVE_DREW_MIN_ACCENT,
        "flat curve count check (must pass — flat-line guard draws a full-width line): \
         {} < {CURVE_DREW_MIN_ACCENT}",
        flat_stats.count,
    );
    let flat_x_span = flat_stats.max_x.saturating_sub(flat_stats.min_x);
    assert!(
        flat_x_span >= CURVE_X_SPAN_MIN,
        "flat curve x_span check (must pass — flat-line guard draws a full-width line): \
         {flat_x_span} < {CURVE_X_SPAN_MIN}",
    );

    // Final relational proof: the moving curve has a taller ACCENT bbox than the flat.
    assert!(
        moving_y_span > flat_y_span,
        "Y-variation discriminator broken: moving curve bbox height ({moving_y_span}px) \
         must be > flat curve bbox height ({flat_y_span}px)"
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

// ── PHASE 4 — lab-run-save-compare: Lab repaint + Compare overlay (T7) ────────
//
// The render-layer proof that the lab-run-save-compare feature's two new
// surfaces actually rasterize (project law — MEMORY.md "verify UI at the render
// layer"; model-Ready is necessary but NOT sufficient):
//
//   T7(a) — a Lab equity curve HYDRATED FROM A `lab-runs/` REPORT (loaded by the
//           two-root loader from a tempdir `lab-runs/` root) rasterizes a
//           non-empty `ACCENT` polyline on the real `chart::view` overlay widget
//           the Lab screen renders.
//   T7(b) — a Compare OVERLAY OF TWO RUNS (both loaded from `lab-runs/` reports)
//           rasterizes BOTH series — the primary `ACCENT` curve AND a second
//           `ACCENT_2` curve — proving two distinct curves drew on one chart
//           (R5: KPIs + equity overlay of both series).
//
// Both feed `ui::widgets::chart::view(bars, …, equity, compare, …)` via
// `ui::test_support::chart_overlay_program`, the EXACT overlay draw path
// (`ChartProgram::draw` → tiny_skia) the Lab/Compare screens paint.

use std::io::Write;

use ui::lab::equity_loader::{LabTuple, load_equity};
use ui::lab::state::{DateRange as LabDateRange, Preset};
use ui::test_support::chart_overlay_program;

/// Number of synthetic bars under the overlay. `synthetic_candles` stamps bar
/// `i` at `(FIXED_EPOCH_SECS + i*60)` seconds, so the bar window in
/// milliseconds is `[BAR_BASE_MS, BAR_BASE_MS + (RENDER_BARS-1)*60_000]`. The
/// `lab-runs/` fixture equity timestamps MUST fall INSIDE this window
/// (`bar_ms()` below) or `draw_equity_polyline` clamps them to one edge → a
/// degenerate vertical line; `compute_equity_range` would also drop them
/// (it windows to `[bars.first, bars.last]`).
const RENDER_BARS: usize = 60;

/// `FIXED_EPOCH_SECS` from `fixtures.rs` (Jan 2024) — the base second
/// `synthetic_candles` stamps bar 0 at. Mirrored here so the fixture equity
/// timestamps land inside the synthetic-bar window.
const FIXED_EPOCH_SECS: i64 = 1_705_320_000;
const BAR_BASE_MS: i64 = FIXED_EPOCH_SECS * 1000;

/// Equity-curve timestamp (ms) for the `n`-th fixture point — one bar-minute
/// apart, anchored at the bar window's base so the overlay traverses the
/// x-axis inside `[bars.first, bars.last]`.
fn bar_ms(n: i64) -> i64 {
    BAR_BASE_MS + n * 60_000
}

/// A `lab-runs/`-style report whose `## Equity curve` timestamps (milliseconds)
/// fall inside the `RENDER_BARS` synthetic-bar window `[0, 3_540_000]` ms.
/// `{symbol}` resolves the loader's `## Universe` symbol-match for the
/// requested tuple. `level` shifts the whole curve vertically so two runs can
/// be made to occupy DISTINCT y-bands (the two overlay polylines then paint
/// separate pixels — the proof that two distinct curves drew, not one line
/// overdrawn twice). Each curve still rises/dips/recovers (a tall traverse).
fn lab_runs_report_at(scenario: &str, symbol: &str, level: i64) -> String {
    // Six points: base + {0, +3000, +1000, -3000, +4000, +8000} offsets,
    // shifted by `level`. A run at level=100_000 lives ~97k–108k; a run at
    // level=60_000 lives ~57k–68k — disjoint y-bands.
    let p = |off: i64| level + off;
    format!(
        r#"---
scenario: {scenario}
seed: 0xC0FFEE
generated: 2026-06-01T12:00:00Z
wall_clock_s: 0.0
data_source: synthetic
---

# Backtest Report — {scenario}

## Summary

| Metric          | Value             |
|-----------------|-------------------|
| Scenario        | {scenario}        |
| Initial capital | ${} USDT          |
| Final equity    | ${} USDT          |
| Max drawdown    | 18.00%            |

## Universe

- {symbol}

## Equity curve

| Timestamp (ms) | Equity (USDT) |
|----------------|---------------|
| {}    | {}.00     |
| {}    | {}.00     |
| {}    | {}.00     |
| {}    | {}.00     |
| {}    | {}.00     |
| {}    | {}.00     |
"#,
        level,
        p(8000),
        bar_ms(0),
        p(0),
        bar_ms(10),
        p(3000),
        bar_ms(20),
        p(1000),
        bar_ms(30),
        p(-3000),
        bar_ms(40),
        p(4000),
        bar_ms(50),
        p(8000),
    )
}

/// Write a `lab-runs/<slug>/reports/<fname>` fixture report and return the
/// `lab-runs/` root path. Mirrors the developer's write-root shape exactly.
fn write_lab_run(lab_runs_root: &std::path::Path, slug: &str, fname: &str, content: &str) {
    let reports = lab_runs_root.join(slug).join("reports");
    std::fs::create_dir_all(&reports).expect("create lab-runs reports dir");
    let mut f = std::fs::File::create(reports.join(fname)).expect("create lab-runs report");
    f.write_all(content.as_bytes())
        .expect("write lab-runs report");
}

/// Synthetic bars for the overlay x-axis (same builder + seed the fixtures
/// cockpit uses, so the bar window is deterministic).
fn render_bars() -> Vec<trading_core::Bar> {
    let venue = trading_core::Venue::Binance;
    let symbol = trading_core::Symbol::new("XRPUSDT");
    let seed = ui::fixtures::seed_for(venue, &symbol);
    ui::fixtures::synthetic_candles(seed, venue, symbol, RENDER_BARS)
}

/// `(r,g,b)` 0-255 of an `ACCENT_2` (dark theme) pixel — the SECOND overlay
/// curve's color (the `accent_palette()[0]` compare line). ACCENT dark =
/// (111,182,174) teal-300; ACCENT_2 dark = (166,213,207) teal-200 (lighter).
/// They share a hue, so a loose per-channel box would let AA pixels count as
/// both — instead [`classify_curve`] below uses a NEAREST-of-two classifier so
/// each curve pixel is attributed to exactly ONE curve (the proof that two
/// DISTINCT curves drew).
fn accent2_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT_2.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

/// Squared Euclidean RGB distance.
fn dist2(p: (i32, i32, i32), q: (i32, i32, i32)) -> i32 {
    let dr = p.0 - q.0;
    let dg = p.1 - q.1;
    let db = p.2 - q.2;
    dr * dr + dg * dg + db * db
}

/// Which overlay curve (if any) a pixel belongs to. A pixel is attributed to
/// the NEAREST of {ACCENT, ACCENT_2} when it is within `CURVE_MATCH_R2` of that
/// color AND strictly closer to it than to the other (a clear margin). Pixels
/// near neither (background, gridlines, AA midpoints between the two) are
/// `Neither`. Because the classifier is winner-take-all, no pixel is ever
/// counted for BOTH curves — so two non-zero counts prove two distinct curves.
#[derive(PartialEq, Eq)]
enum Curve {
    Accent,
    Accent2,
    Neither,
}

/// Max squared distance for a pixel to be claimed by a curve color. The
/// ACCENT↔ACCENT_2 squared separation is 55²+31²+33² = 5075; a radius² of 900
/// (≈30/channel) is well inside half that separation, so the two acceptance
/// balls are disjoint and an AA pixel exactly between them is `Neither`.
const CURVE_MATCH_R2: i32 = 900;

fn classify_curve(r: u8, g: u8, b: u8) -> Curve {
    let p = (i32::from(r), i32::from(g), i32::from(b));
    let d_accent = dist2(p, accent_rgb());
    let d_accent2 = dist2(p, accent2_rgb());
    if d_accent <= CURVE_MATCH_R2 && d_accent < d_accent2 {
        Curve::Accent
    } else if d_accent2 <= CURVE_MATCH_R2 && d_accent2 < d_accent {
        Curve::Accent2
    } else {
        Curve::Neither
    }
}

/// Count `(accent, accent2)` curve pixels across the WHOLE frame via the
/// winner-take-all classifier. The bare overlay widget fills the frame, so no
/// sidebar/crop offset is needed (unlike the Live screen harness above).
fn count_curve_pixels(shot: &iced::window::Screenshot) -> (usize, usize) {
    let rgba: &[u8] = &shot.rgba;
    let (mut accent, mut accent2) = (0usize, 0usize);
    let mut i = 0;
    while i + 2 < rgba.len() {
        match classify_curve(rgba[i], rgba[i + 1], rgba[i + 2]) {
            Curve::Accent => accent += 1,
            Curve::Accent2 => accent2 += 1,
            Curve::Neither => {}
        }
        i += 4;
    }
    (accent, accent2)
}

/// Render the bare chart-overlay widget (bars + equity + compare) and return
/// its RGBA screenshot via the real `ChartProgram::draw` path.
fn render_overlay(
    bars: Vec<trading_core::Bar>,
    equity: Option<ui::lab::equity_loader::LabEquitySeries>,
    compare: Vec<ui::lab::equity_loader::LabEquitySeries>,
) -> iced::window::Screenshot {
    // SAFETY: test-only single-threaded env init before iced_test::screenshot,
    // mirroring `render_live` (UTC time-axis labels for determinism).
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };
    let program = chart_overlay_program(bars, equity, compare);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// Floor for "the overlay polyline actually drew". The overlay stroke is
/// thinner (1.5 px) than the Live curve's, and there is no fill under the
/// compare line, so the per-curve pixel budget is smaller; 120 is a
/// comfortable floor above AA noise yet well below a real traversing
/// polyline's count on a 1280-wide frame.
const OVERLAY_DREW_MIN: usize = 120;

/// Diagnostic (run with `--nocapture`): print the per-curve pixel counts for a
/// single-run overlay and a two-run overlay so `OVERLAY_DREW_MIN` is calibrated
/// empirically, not guessed. Mirrors `diag_accent_bounding_box` above.
#[test]
fn diag_overlay_curve_pixel_counts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene(&tmp);
    let (a, b) = load_two_run_scene(&lab_runs);

    let (bars_only_accent, bars_only_accent2) =
        count_curve_pixels(&render_overlay(render_bars(), None, vec![]));
    let (one_accent, one_accent2) =
        count_curve_pixels(&render_overlay(render_bars(), Some(a.clone()), vec![]));
    let (two_accent, two_accent2) =
        count_curve_pixels(&render_overlay(render_bars(), Some(a), vec![b]));
    eprintln!(
        "[diag] bars-only (no equity): ACCENT={bars_only_accent} ACCENT_2={bars_only_accent2}; \
         single-run overlay: ACCENT={one_accent} ACCENT_2={one_accent2}; \
         two-run overlay: ACCENT={two_accent} ACCENT_2={two_accent2}"
    );
}

/// Write a two-run `lab-runs/` scene: run A (momentum/XRPUSDT) at the ~100k
/// level + run B (sma/BTCUSDT) at the ~60k level — DISJOINT y-bands so the two
/// overlay polylines paint separate pixels. Returns the `lab-runs/` root.
fn write_two_run_scene(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    write_lab_run(
        &lab_runs,
        "v0-paper-sma",
        "backtest-20260601-130000-btc-2023-1m-sma-cross.md",
        &lab_runs_report_at("btc-2023-1m-sma-cross", "BTCUSDT", 60_000),
    );
    lab_runs
}

/// Load both runs of the two-run scene via the loader the production path uses.
fn load_two_run_scene(
    lab_runs: &std::path::Path,
) -> (
    ui::lab::equity_loader::LabEquitySeries,
    ui::lab::equity_loader::LabEquitySeries,
) {
    let tuple_a = LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
    };
    let tuple_b = LabTuple {
        strategy: smol_str::SmolStr::new("v0.sma"),
        symbol: smol_str::SmolStr::new("BTCUSDT"),
        range: LabDateRange::Preset(Preset::Last90d),
    };
    let a = load_equity(&tuple_a, lab_runs).expect("run A loads");
    let b = load_equity(&tuple_b, lab_runs).expect("run B loads");
    (a, b)
}

/// Minimum EXTRA `ACCENT` pixels the hydrated equity overlay must add over the
/// bars-only (price-line) baseline. Calibration (`diag_overlay_curve_pixel_counts`):
/// bars-only ≈ 2610 ACCENT (the price line, also `ACCENT`); +equity overlay
/// ≈ 3894 (+1284). 400 is a comfortable floor well above paint jitter yet far
/// below the ~1284 a real overlay polyline adds. The CONTRAST (with-equity ≫
/// bars-only) is what isolates the equity curve from the same-color price line.
const OVERLAY_ACCENT_DELTA_MIN: usize = 400;

/// **T7(a) — Lab repaint-from-`lab-runs/` render proof.** A `LabEquitySeries`
/// LOADED by the two-root loader from a tempdir `lab-runs/` report rasterizes a
/// non-empty `ACCENT` polyline on the real Lab overlay widget (`chart::view`).
/// This is the AC4 "the curve survives a restart by repainting from the
/// persisted report" guarantee, proven at the pixel layer.
///
/// The Lab chart's PRICE line is also `ACCENT`, so a raw ACCENT count cannot
/// tell "the equity curve drew" from "the price line drew". The proof is a
/// CONTRAST: render the SAME bars WITHOUT the equity overlay (price line only)
/// and WITH the hydrated overlay, and assert the overlay adds a non-trivial
/// band of EXTRA `ACCENT` pixels — i.e. the loaded-from-disk equity polyline is
/// genuinely on screen, additive to the price line.
#[test]
fn lab_curve_hydrated_from_lab_runs_report_renders() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );

    // Load the series via the loader the production Lab cold path uses, pointed
    // at the `lab-runs/` root (the developer's write-root shape).
    let tuple = LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
    };
    let series = load_equity(&tuple, &lab_runs).expect("series loads from lab-runs report");
    assert!(
        series.samples.len() >= 2,
        "hydrated series must have ≥2 points to traverse (got {})",
        series.samples.len()
    );

    let bars = render_bars();
    let (baseline_accent, _) = count_curve_pixels(&render_overlay(bars.clone(), None, vec![]));
    let (with_equity_accent, _) = count_curve_pixels(&render_overlay(bars, Some(series), vec![]));

    let delta = with_equity_accent.saturating_sub(baseline_accent);
    assert!(
        delta >= OVERLAY_ACCENT_DELTA_MIN,
        "Lab curve hydrated from a lab-runs/ report did NOT render: the equity overlay added \
         only {delta} ACCENT pixels over the price-line baseline ({baseline_accent} → \
         {with_equity_accent}; expected +≥{OVERLAY_ACCENT_DELTA_MIN}). The series parsed from \
         disk but the overlay canvas painted no visible polyline — the repaint-from-disk \
         render bug."
    );
}

/// **T7(b) — Compare two-run overlay render proof (the headline render gate for
/// R5).** TWO `LabEquitySeries`, BOTH loaded from `lab-runs/` reports, overlaid
/// on ONE chart (`equity` = run A drawn `ACCENT`; `compare[0]` = run B drawn
/// `ACCENT_2`) must rasterize BOTH curves. Asserts ACCENT pixels (run A) AND
/// ACCENT_2 pixels (run B) BOTH cross the "drew" floor — i.e. two DISTINCT
/// curves are on screen, not one. A single-curve regression (compare series
/// dropped, or both collapsed to the same color) fails here.
#[test]
fn compare_two_run_overlay_renders_both_series() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Run A (primary, ACCENT) at ~100k; run B (compare, ACCENT_2) at ~60k —
    // disjoint y-bands so the two polylines paint separate pixels.
    let lab_runs = write_two_run_scene(&tmp);
    let (series_a, series_b) = load_two_run_scene(&lab_runs);

    let shot = render_overlay(render_bars(), Some(series_a), vec![series_b]);

    let (accent, accent2) = count_curve_pixels(&shot);

    // Run A (primary, ACCENT) drew.
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "Compare overlay: the PRIMARY (ACCENT) run did NOT render: only {accent} ACCENT \
         pixels (expected ≥ {OVERLAY_DREW_MIN})."
    );
    // Run B (compare, ACCENT_2) drew — the proof TWO distinct curves are on
    // screen. This is the assertion that fails if the second series is dropped
    // or both render in the same color.
    assert!(
        accent2 >= OVERLAY_DREW_MIN,
        "Compare overlay: the SECOND (ACCENT_2) run did NOT render: only {accent2} ACCENT_2 \
         pixels (expected ≥ {OVERLAY_DREW_MIN}). The two-run overlay must rasterize BOTH \
         series as distinct curves — a dropped compare series or a same-color collapse lands \
         here."
    );
}

/// **T7(b) contrast self-proof.** Render the SAME overlay scene WITHOUT the
/// compare series and assert NO `ACCENT_2` pixels appear — proving the
/// `ACCENT_2` count in `compare_two_run_overlay_renders_both_series` genuinely
/// comes from the second run, not from chrome/AA bleed. Belt-and-braces so the
/// two-curve discriminator can never silently degrade into "ACCENT_2 is always
/// present".
#[test]
fn single_run_overlay_draws_no_accent2() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    let tuple = LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
    };
    let series = load_equity(&tuple, &lab_runs).expect("series loads");

    // No compare series → ACCENT_2 must be absent.
    let shot = render_overlay(render_bars(), Some(series), vec![]);
    let (_accent, accent2) = count_curve_pixels(&shot);
    assert!(
        accent2 < OVERLAY_DREW_MIN,
        "single-run overlay must draw NO second (ACCENT_2) curve, but saw {accent2} ACCENT_2 \
         pixels (≥ {OVERLAY_DREW_MIN}). If this fails the two-curve discriminator is reading \
         chrome/AA bleed, not the actual compare series — fix the detector before trusting it."
    );
}

// ── PHASE 5 — lab-compare-equity-overlay: REAL Compare-screen overlay (T3) ────
//
// The headline gate for THIS feature (R3 / AC1-AC2). PHASE 4 above proved the
// bare `chart::view` overlay widget rasterizes two series; here we prove the
// **real Compare-screen production path** does the same when driven by two
// persisted runs:
//
//   1. Write two `lab-runs/` reports + their COMPANION equity CSVs.
//   2. Scan them via `compare::cache::scan_report_roots` (the production
//      cold-boot path) — building `CachedCell`s whose new `equity_series_ts`
//      field is hydrated from the companion CSVs (the T1 wiring).
//   3. Install those cells in a `Cockpit`'s `compare_screen_state.cache`, select
//      both via `Message::CompareToggleOverlay` (the T2 ring), and render the
//      REAL `screens::compare::view` (its `overlay_panel` → `chart::view`).
//   4. Assert BOTH curves rasterize (ACCENT + ACCENT_2) in the chart band, and
//      a single-run contrast draws NO ACCENT_2.
//
// This exercises the EXACT screen code the operator sees — not a synthetic
// series, not the bare widget. The `CachedCell.equity_series_ts → LabEquitySeries
// → chart::view` thread is what this feature added; this test is its proof.

use reports::csv_artifacts::{EquitySample, write_equity_csv};
use ui::compare::state::{CachedCell, OverlaySlot};
use ui::lab::state::DateRange as CmpDateRange;
use ui::state::{StrategiesConfig, StrategyConfigEntry};
use ui::test_support::compare_screen_program;

/// Write a companion equity CSV (`<stem>-equity.csv`) beside a `lab-runs/`
/// report via the PRODUCTION `csv_artifacts::write_equity_csv` schema, carrying
/// a 6-point series that rises/dips/recovers shifted to `level` (so two runs
/// occupy disjoint y-bands → separate overlay pixels). Mirrors the offsets of
/// `lab_runs_report_at` so the two series have a comparable shape.
fn write_companion_equity_csv(
    lab_runs_root: &std::path::Path,
    slug: &str,
    md_fname: &str,
    level: i64,
) {
    let reports = lab_runs_root.join(slug).join("reports");
    std::fs::create_dir_all(&reports).expect("create lab-runs reports dir");
    let stem = md_fname.strip_suffix(".md").expect("md suffix");
    let csv_path = reports.join(format!("{stem}-equity.csv"));

    let p = |off: i64| -> Decimal { Decimal::from(level + off) };
    let offsets = [0i64, 3000, 1000, -3000, 4000, 8000];
    let samples: Vec<EquitySample> = offsets
        .iter()
        .enumerate()
        .map(|(i, off)| EquitySample {
            ts: Timestamp::new(
                time::OffsetDateTime::UNIX_EPOCH
                    + time::Duration::milliseconds(bar_ms(i as i64 * 10)),
            ),
            equity_total: p(*off),
            realized_pnl: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            cash_balance: p(*off),
        })
        .collect();
    write_equity_csv(&csv_path, &samples).expect("write companion equity csv");
}

/// Build a Compare-screen `Cockpit` whose cache is hydrated by the PRODUCTION
/// `scan_report_roots` over a two-run `lab-runs/` scene (reports + companion
/// CSVs), with a populated `strategies_config` so the real matrix renders.
fn compare_cockpit_from_two_run_scene(lab_runs: &std::path::Path) -> Cockpit {
    let cache = ui::compare::cache::scan_report_roots(&[lab_runs.to_path_buf()]);

    let mut c = Cockpit::new();
    c.current_screen = Screen::Compare;
    c.strategies_config = Some(StrategiesConfig {
        strategies: vec![
            StrategyConfigEntry {
                id: trading_core::StrategyId::new("top10_momentum_h1"),
                source_path: smol_str::SmolStr::new("config/strategies/top10_momentum_h1.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: trading_core::StrategyId::new("btc_sma_cross"),
                source_path: smol_str::SmolStr::new("config/strategies/btc_sma.toml"),
                params: vec![],
            },
        ],
    });
    c.compare_screen_state.cache = cache;
    c
}

/// The two overlay slots for the scene — run A (momentum/XRPUSDT, ACCENT) and
/// run B (sma/BTCUSDT, ACCENT_2). Keys match `scan_report_roots`' cell keys
/// (`strategy.id` from frontmatter, `DateRange::default()`).
fn scene_slots() -> (OverlaySlot, OverlaySlot) {
    let range = CmpDateRange::default();
    (
        (
            smol_str::SmolStr::new("top10_momentum_h1"),
            trading_core::Symbol::new("XRPUSDT"),
            range.clone(),
        ),
        (
            smol_str::SmolStr::new("btc_sma_cross"),
            trading_core::Symbol::new("BTCUSDT"),
            range,
        ),
    )
}

/// A `lab-runs/`-style report WITH a `strategy:` frontmatter block + a
/// `## Summary` KPI table — the shape `compare::cache::scan_report_roots`
/// requires (it skips reports without `strategy.id`). The per-bar series for
/// the overlay lives in the companion CSV, not the `.md` (the `.md`'s equity
/// section is a sparkline only). `{strategy_id}` keys the scanned `CachedCell`.
fn scan_report_with_strategy(
    scenario: &str,
    strategy_id: &str,
    strategy_kind: &str,
    level: i64,
) -> String {
    format!(
        r#"---
scenario: {scenario}
seed: 0xC0FFEE
generated: 2026-06-01T12:00:00Z
wall_clock_s: 0.0
data_source: synthetic
strategy:
  id: {strategy_id}
  kind: {strategy_kind}
  source: config/strategies/{strategy_id}.toml
---

# Backtest Report — {scenario}

## Summary

| Metric          | Value             |
|-----------------|-------------------|
| Scenario        | {scenario}        |
| Initial capital | ${level}.00 USDT  |
| Final equity    | ${level}.00 USDT  |
| Sharpe ratio    | **0.94**          |
| Total return    | **12.3 %**        |
| Max drawdown    | **-5.6 %**        |
| Trade count     | **42**            |
"#
    )
}

/// Write the full two-run scene: `.md` reports WITH strategy blocks (for the
/// production `scan_report_roots` KPI scan) + companion equity CSVs (for the
/// timestamped overlay series). Returns the `lab-runs/` root. Run A at the
/// ~100k level, run B at the ~60k level (disjoint y-bands).
fn write_two_run_scene_with_csvs(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let lab_runs = tmp.path().join("lab-runs");
    // Run A — momentum/XRPUSDT @ 100k.
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &scan_report_with_strategy(
            "top10-2024-h1-momentum",
            "top10_momentum_h1",
            "cross_sectional_momentum",
            100_000,
        ),
    );
    write_companion_equity_csv(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        100_000,
    );
    // Run B — sma/BTCUSDT @ 60k.
    write_lab_run(
        &lab_runs,
        "v0-paper-sma",
        "backtest-20260601-130000-btc-2023-1m-sma-cross.md",
        &scan_report_with_strategy(
            "btc-2023-1m-sma-cross",
            "btc_sma_cross",
            "sma_crossover",
            60_000,
        ),
    );
    write_companion_equity_csv(
        &lab_runs,
        "v0-paper-sma",
        "backtest-20260601-130000-btc-2023-1m-sma-cross.md",
        60_000,
    );
    lab_runs
}

/// Crop band for the overlay CHART within the Compare screen body. The screen
/// stacks `toolbar → matrix(Fill) → overlay-panel(title + legend + 240px
/// chart)`; with `matrix = Length::Fill` the overlay panel sits at the BOTTOM.
/// We crop the lower ~250px where the 240px chart paints, excluding the matrix
/// cells' `+`/✓ chips above — so the curve-pixel classifier sees only the two
/// overlay polylines, never a selected cell's ACCENT/ACCENT_2 chip.
const CMP_CROP_Y0: u32 = VIEW_H.saturating_sub(250);

/// Count `(accent, accent2)` curve pixels inside the overlay chart band only
/// (rows `≥ CMP_CROP_Y0`) via the same winner-take-all classifier.
fn count_curve_pixels_chart_band(shot: &iced::window::Screenshot) -> (usize, usize) {
    let w = shot.size.width;
    let h = shot.size.height;
    let rgba: &[u8] = &shot.rgba;
    let (mut accent, mut accent2) = (0usize, 0usize);
    let y0 = CMP_CROP_Y0.min(h);
    for y in y0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= rgba.len() {
                continue;
            }
            match classify_curve(rgba[idx], rgba[idx + 1], rgba[idx + 2]) {
                Curve::Accent => accent += 1,
                Curve::Accent2 => accent2 += 1,
                Curve::Neither => {}
            }
        }
    }
    (accent, accent2)
}

/// Render the real Compare screen body of `cockpit` and return its screenshot.
fn render_compare_screen(cockpit: Cockpit) -> iced::window::Screenshot {
    // SAFETY: test-only single-threaded env init before iced_test::screenshot
    // (UTC time-axis labels for determinism), mirroring `render_live`.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };
    let program = compare_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// Diagnostic (run with `--nocapture`): per-curve pixel counts in the overlay
/// chart band for the real Compare screen with two runs selected. Calibrates
/// the `OVERLAY_DREW_MIN` floor empirically against the cropped band.
#[test]
fn diag_compare_screen_overlay_pixel_counts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene_with_csvs(&tmp);
    let mut c = compare_cockpit_from_two_run_scene(&lab_runs);
    let (slot_a, slot_b) = scene_slots();

    // Sanity: the cells exist AND carry hydrated timestamped series.
    let cell_a: &CachedCell = c
        .compare_screen_state
        .cache
        .get(&slot_a)
        .expect("run A cell present in scanned cache");
    let cell_b: &CachedCell = c
        .compare_screen_state
        .cache
        .get(&slot_b)
        .expect("run B cell present in scanned cache");
    eprintln!(
        "[diag] cell A series len={} ; cell B series len={}",
        cell_a.equity_series_ts.len(),
        cell_b.equity_series_ts.len()
    );

    // Select both through the production message path.
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));

    let shot = render_compare_screen(c);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);
    eprintln!(
        "[diag] Compare-screen overlay chart band (y≥{CMP_CROP_Y0}): ACCENT={accent} ACCENT_2={accent2}"
    );
}

/// **T3 / AC1-AC2 (THE gate).** Two persisted runs selected in the REAL Compare
/// screen overlay BOTH rasterize — run A as the `ACCENT` primary curve, run B
/// as the `ACCENT_2` compare curve — when hydrated from companion-CSV-backed
/// `CachedCell`s through `screens::compare::view`'s `overlay_panel` → the
/// render-proven `chart::view` overlay. The `equity_series_ts` field (T1) +
/// the selection ring (T2) + the screen wiring are proven end-to-end here.
#[test]
fn compare_screen_two_run_overlay_renders_both_series() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene_with_csvs(&tmp);
    let mut c = compare_cockpit_from_two_run_scene(&lab_runs);
    let (slot_a, slot_b) = scene_slots();

    // Both cells carry a non-empty timestamped series (companion CSV hydrated).
    assert!(
        c.compare_screen_state
            .cache
            .get(&slot_a)
            .is_some_and(|cell| cell.equity_series_ts.len() >= 2),
        "run A cell must hydrate a ≥2-point timestamped series from its companion CSV"
    );
    assert!(
        c.compare_screen_state
            .cache
            .get(&slot_b)
            .is_some_and(|cell| cell.equity_series_ts.len() >= 2),
        "run B cell must hydrate a ≥2-point timestamped series from its companion CSV"
    );

    // Select both runs through the production update path (the T2 ring).
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));
    assert_eq!(
        c.compare_screen_state.overlay_selection.len(),
        2,
        "both runs must be in the overlay selection ring"
    );

    let shot = render_compare_screen(c);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);

    // Run A (primary, ACCENT) drew on the real screen.
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "Compare SCREEN overlay: the PRIMARY (ACCENT) run did NOT render: only {accent} ACCENT \
         pixels in the chart band (expected ≥ {OVERLAY_DREW_MIN}). The cell's hydrated \
         equity_series_ts did not reach chart::view through overlay_panel."
    );
    // Run B (compare, ACCENT_2) drew — two DISTINCT curves on one chart.
    assert!(
        accent2 >= OVERLAY_DREW_MIN,
        "Compare SCREEN overlay: the SECOND (ACCENT_2) run did NOT render: only {accent2} \
         ACCENT_2 pixels in the chart band (expected ≥ {OVERLAY_DREW_MIN}). The second selected \
         run's series was dropped or both curves collapsed to one color."
    );
}

/// **T3 contrast self-proof.** The SAME real Compare screen with only ONE run
/// selected draws the `ACCENT` primary curve but NO `ACCENT_2` — proving the
/// `ACCENT_2` count above genuinely comes from the second selected run, not
/// from the screen's chrome / matrix chips / AA bleed inside the chart band.
#[test]
fn compare_screen_single_run_overlay_draws_no_accent2() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene_with_csvs(&tmp);
    let mut c = compare_cockpit_from_two_run_scene(&lab_runs);
    let (slot_a, _slot_b) = scene_slots();

    // Select ONLY run A.
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    assert_eq!(c.compare_screen_state.overlay_selection.len(), 1);

    let shot = render_compare_screen(c);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);

    // The primary curve drew…
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "single-run Compare screen: the primary (ACCENT) curve must still render ({accent} \
         ACCENT pixels < {OVERLAY_DREW_MIN})"
    );
    // …and NO second curve.
    assert!(
        accent2 < OVERLAY_DREW_MIN,
        "single-run Compare screen overlay must draw NO second (ACCENT_2) curve, but saw \
         {accent2} ACCENT_2 pixels in the chart band (≥ {OVERLAY_DREW_MIN}). The two-curve \
         discriminator is reading chrome/chip/AA bleed — not the actual second run."
    );
}
