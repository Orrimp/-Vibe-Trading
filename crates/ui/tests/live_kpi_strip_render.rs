//! Render-verifiable harness for the Live **KPI strip** — the missing half of
//! story 2-15's proof (2-15 review H1, 2026-08-12).
//!
//! ## Why this file exists (the gap it closes)
//!
//! Story 2-15 wired two panels on the Live screen: the equity curve and the
//! KPI strip. The curve half has had a binding rasterized guard since
//! `live_equity_render.rs`. The strip half had **none** — reverting
//! `screens/live.rs`'s `let kpi_state = &model.live_kpi;` back to the
//! pre-story hard-wired `&PanelState::Loading` (i.e. deleting the entire
//! feature) left the whole `-p ui` suite green:
//!
//! * `tests/panel_snapshots.rs::kpi_strip_lines` reads `c.live_kpi` **directly
//!   and never calls `screens::live::view`** — a mirror of the model, not of
//!   the screen (and it is macOS-gated, so two of three CI legs compile it to
//!   zero tests);
//! * `tests/live_equity_render.rs` crops to the CURVE band (`CROP_Y 130..420`),
//!   so the strip below it is never sampled;
//! * the visual baselines render the fresh-boot state, where `live_kpi` is
//!   `Loading` anyway.
//!
//! So this harness renders the **real Live screen** through
//! `iced_test::screenshot` (`view` → tiny-skia rasterization, the same path
//! `cockpit_live` paints) and inspects the KPI strip's own band for the
//! sentiment-coloured value glyphs that only the POPULATED strip draws.
//!
//! ## The discriminator
//!
//! `widgets::kpi_strip` renders two visually distinct bodies:
//!
//! * the **populated** strip — Total-return in `UP_500`/`DOWN_500`/`FG_1` and
//!   Max-DD **always** in `DOWN_500` (`widgets::num::format_pct_max_dd`), at
//!   `text::H1` (24 px);
//! * the **unavailable** strip — six `—` placeholders, every one of them
//!   `FG_3`, plus an `FG_3` advisory line.
//!
//! `FG_3` dark (128,137,147) is nowhere near `DOWN_500` dark (201,123,94) or
//! `UP_500` dark (110,155,106), so *sentiment-coloured pixels inside the strip
//! band* ⟺ *the populated strip drew*.
//!
//! Measured (`diag_kpi_band_sentiment_bbox`): the populated strip paints **550**
//! sentiment pixels in the band; the six-dash body paints **54** (anti-aliasing
//! fringe on surrounding chrome — the dashes themselves contribute none). The
//! threshold sits at 120, an order of magnitude clear of both.
//!
//! ## Not OS-gated (2-15 review L13)
//!
//! `panel_snapshots.rs` is `#![cfg(target_os = "macos")]` per ADR-0057 D2
//! (text layout differs off-macOS), which left the Linux/Windows legs with
//! zero KPI-strip coverage. This file has **no `cfg(target_os)` gate** —
//! matching `live_equity_render.rs` and `headless_emulator_smoke.rs`. It is
//! safe to run everywhere because it asserts on *colour populations*, never on
//! glyph metrics or a byte-compared PNG: font fallback changes where the
//! digits land, not what colour they are.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Money, PnlSnapshot, Timestamp, Usdt};

use ui::state::{Cockpit, LiveEquityWindow, Message, PanelState, Screen, update};
use ui::test_support::program_from_cockpit;
use ui::theme::{ThemeMode, color};

// Floor viewport — the same 1280×720 the other render harnesses use.
const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 720;
const SCALE: f32 = 1.0;

static INIT_UTC: std::sync::Once = std::sync::Once::new();
fn force_utc_once() {
    INIT_UTC.call_once(ui::force_chart_utc_for_tests);
}

/// KPI-strip crop window in physical pixels (scale 1.0 → logical == physical).
///
/// The Live column stacks `headline → health_strip → equity_curve(240 px) →
/// kpi_row → caption(s) → bottom_row`, inside the shell's 180 px sidebar. The
/// curve's ACCENT bbox is empirically y≈202..370 (`live_equity_render.rs`'s
/// `diag_accent_bounding_box`), so the strip lands below ~435. `X` starts well
/// right of the sidebar so its active-item highlight can never leak in.
/// Verified empirically by `diag_kpi_band_sentiment_bbox` below, not guessed.
const KPI_X: u32 = 220;
const KPI_W: u32 = 980; // 220..1200
const KPI_Y: u32 = 430;
const KPI_H: u32 = 130; // 430..560 — the kpi_row band

/// Caption band — the muted `FG_3` scope caption rendered directly under the
/// strip. Located empirically (a per-row muted-pixel delta between the two
/// caption states put the glyph rows at y 542..551), then padded either side:
/// 535..560. Used by the caption-honesty pixel test.
const CAP_Y: u32 = 535;
const CAP_H: u32 = 25; // 535..560

/// Per-channel tolerance for a colour match. tiny-skia anti-aliases glyph
/// edges; ±30 catches the solid interior of a 24 px digit without admitting
/// `FG_3` (128,137,147), `PANEL` (28,33,39) or `ACCENT` (111,182,174).
const CHANNEL_TOL: i32 = 30;

fn rgb_of(c: iced::Color) -> (i32, i32, i32) {
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

fn near(px: (u8, u8, u8), target: (i32, i32, i32)) -> bool {
    (i32::from(px.0) - target.0).abs() <= CHANNEL_TOL
        && (i32::from(px.1) - target.1).abs() <= CHANNEL_TOL
        && (i32::from(px.2) - target.2).abs() <= CHANNEL_TOL
}

struct BandStats {
    /// Pixels coloured `UP_500` or `DOWN_500` — the populated strip's value
    /// glyphs. Zero for the six-dash body.
    sentiment: usize,
    /// Pixels coloured `FG_3` — labels, dashes, captions.
    muted: usize,
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
}

fn band_stats(shot: &iced::window::Screenshot, y0: u32, h: u32) -> BandStats {
    let w = shot.size.width;
    let rgba: &[u8] = &shot.rgba;
    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    let muted_c = rgb_of(color::FG_3.current(ThemeMode::Dark));

    let mut stats = BandStats {
        sentiment: 0,
        muted: 0,
        min_x: u32::MAX,
        max_x: 0,
        min_y: u32::MAX,
        max_y: 0,
    };
    let x1 = (KPI_X + KPI_W).min(w);
    let y1 = (y0 + h).min(shot.size.height);
    for y in y0.min(shot.size.height)..y1 {
        for x in KPI_X.min(w)..x1 {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= rgba.len() {
                continue;
            }
            let px = (rgba[idx], rgba[idx + 1], rgba[idx + 2]);
            if near(px, up) || near(px, down) {
                stats.sentiment += 1;
                stats.min_x = stats.min_x.min(x);
                stats.max_x = stats.max_x.max(x);
                stats.min_y = stats.min_y.min(y);
                stats.max_y = stats.max_y.max(y);
            } else if near(px, muted_c) {
                stats.muted += 1;
            }
        }
    }
    stats
}

fn kpi_stats(shot: &iced::window::Screenshot) -> BandStats {
    band_stats(shot, KPI_Y, KPI_H)
}

fn render_live(cockpit: Cockpit) -> iced::window::Screenshot {
    force_utc_once();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

fn snap(secs: i64, equity: Decimal) -> PnlSnapshot {
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

/// A Live cockpit driven through the production `update` path by a realistic
/// session: rises, dips (non-zero Max-DD), recovers. Both panels end `Ready`.
fn moving_session_cockpit() -> Cockpit {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in [
        (0i64, dec!(100000)),
        (60, dec!(100800)),
        (120, dec!(101500)),
        (180, dec!(101200)),
        (240, dec!(100400)),
        (300, dec!(101900)),
        (360, dec!(102600)),
        (420, dec!(103100)),
    ] {
        update(&mut c, Message::PnlRefreshed(snap(secs, eq)));
    }
    c
}

// ── Calibration diagnostic (run with --nocapture) ───────────────────────────

/// Pins the sentiment-pixel bounding box + counts so the crop window above is
/// verified empirically. Mirrors `live_equity_render.rs::diag_accent_bounding_box`.
#[test]
fn diag_kpi_band_sentiment_bbox() {
    let ready = kpi_stats(&render_live(moving_session_cockpit()));
    let mut loading_c = moving_session_cockpit();
    loading_c.live_kpi = PanelState::Loading;
    let loading = kpi_stats(&render_live(loading_c));
    eprintln!(
        "[diag] KPI band ({KPI_X}..{}, {KPI_Y}..{}): READY sentiment={} muted={} bbox=({}..{}, {}..{}) | LOADING sentiment={} muted={}",
        KPI_X + KPI_W,
        KPI_Y + KPI_H,
        ready.sentiment,
        ready.muted,
        ready.min_x,
        ready.max_x,
        ready.min_y,
        ready.max_y,
        loading.sentiment,
        loading.muted,
    );

    let cap_untrunc = band_stats(&render_live(moving_session_cockpit()), CAP_Y, CAP_H).muted;
    let mut trunc_c = moving_session_cockpit();
    trunc_c.live_equity_window = LiveEquityWindow::Rolling;
    let cap_trunc = band_stats(&render_live(trunc_c), CAP_Y, CAP_H).muted;
    eprintln!(
        "[diag] caption band ({CAP_Y}..{}): session-scoped={cap_untrunc} muted px, \
         rolling-window={cap_trunc} muted px",
        CAP_Y + CAP_H,
    );
}

/// Minimum sentiment-coloured pixels that constitute "the populated strip
/// actually drew". Calibrated by `diag_kpi_band_sentiment_bbox`: a healthy
/// Ready strip paints ~550 (its Total-return and Max-DD values at `text::H1`,
/// 24 px), the six-dash body ~54 (AA fringe only). 120 sits ~2× above the
/// dashes' floor and ~4.5× below the populated strip.
const STRIP_DREW_MIN_SENTIMENT: usize = 120;

// ── THE gate: the populated strip rasterizes ────────────────────────────────

/// **2-15 review H1 — the binding render proof for the KPI-strip half.**
///
/// Drive a realistic live session through the production `update` path, render
/// the REAL Live screen, and assert the populated strip drew:
///   (i)   the model says `Ready` (necessary, never sufficient),
///   (ii)  the strip band carries a non-trivial population of sentiment-
///         coloured value pixels, and
///   (iii) they span a real horizontal range (the six cards are laid out
///         across the strip, not collapsed into one cell).
///
/// **This test is the one that goes RED when the wiring is reverted.** Set
/// `screens/live.rs`'s `kpi_state` back to `&PanelState::Loading` — the
/// pre-story hard-wire this story exists to remove — and (ii) fails: the screen
/// paints six `FG_3` dashes and the count drops 550 → 54 (verified 2026-08-12;
/// `live_equity_render.rs`, `panel_snapshots.rs` and `headless_emulator_smoke.rs`
/// all stayed green under that same mutation, which is why this file exists).
#[test]
fn live_kpi_strip_actually_renders() {
    let c = moving_session_cockpit();
    // (i) model-level precondition.
    assert_eq!(
        c.live_kpi.variant_name(),
        "ready",
        "KPI strip must be Ready after a multi-point session"
    );

    let stats = kpi_stats(&render_live(c));

    // (ii) the value glyphs rasterized.
    assert!(
        stats.sentiment >= STRIP_DREW_MIN_SENTIMENT,
        "KPI strip did NOT render its values: only {} sentiment-coloured pixels \
         in the strip band (expected ≥ {STRIP_DREW_MIN_SENTIMENT}). The model is \
         Ready but the screen is painting the six-dash 'metrics unavailable' \
         body — i.e. the Live screen is not reading `model.live_kpi`.",
        stats.sentiment,
    );

    // (iii) the cards span the strip (six columns, not one).
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span >= 200,
        "the strip's value pixels span only {x_span}px — the six-card grid \
         should spread them across the panel width",
    );
}

/// **The negative control — and the exact mutation this file guards.**
///
/// `PanelState::Loading` for the KPI strip is *precisely* the state
/// `screens/live.rs` hard-wired before story 2-15 (and the state the revert
/// mutation restores). The SAME screen, the SAME session data behind it, only
/// the strip's panel state differs — and the band must go quiet. Without this
/// half, a "sentiment pixels exist" assertion could be satisfied by any
/// coloured pixel anywhere in the crop.
#[test]
fn loading_kpi_strip_draws_no_sentiment_pixels() {
    let mut c = moving_session_cockpit();
    c.live_kpi = PanelState::Loading;

    let stats = kpi_stats(&render_live(c));
    assert!(
        stats.sentiment < STRIP_DREW_MIN_SENTIMENT,
        "NEGATIVE CONTROL FAILED: a Loading KPI strip renders six FG_3 dashes \
         and must paint (essentially) no sentiment-coloured pixels, but the \
         harness saw {} (≥ {STRIP_DREW_MIN_SENTIMENT}). If this fails the crop \
         window is sampling something other than the strip — fix the harness \
         before trusting it.",
        stats.sentiment,
    );
    // …and the dashes ARE drawn (the band is not simply empty/off-screen).
    assert!(
        stats.muted > 0,
        "the unavailable strip must still paint its FG_3 dashes in this band; \
         zero muted pixels means the crop window misses the strip entirely"
    );
}

/// The relational proof (belt-and-braces): the SAME screen, populated vs.
/// hard-wired-Loading, must differ by a wide margin — so the guard can never
/// silently degrade into "both look the same".
#[test]
fn populated_strip_draws_far_more_than_loading_strip() {
    let ready = kpi_stats(&render_live(moving_session_cockpit())).sentiment;
    let mut loading_c = moving_session_cockpit();
    loading_c.live_kpi = PanelState::Loading;
    let loading = kpi_stats(&render_live(loading_c)).sentiment;
    assert!(
        ready > loading * 4 + 100,
        "the harness must strongly distinguish a populated strip ({ready} \
         sentiment px) from the Loading strip ({loading} px)"
    );
}

// ── 2-15 review H2 — a healthy FLAT feed must not read "unavailable" ─────────

/// **H2 at the pixel layer.** A freshly booted `cockpit_live` in the default
/// `ExecutionMode::Observe` places no orders ⇒ no fills ⇒ flat equity, so its
/// KPI values are `0.00 % / 0.00 % / 0`. That payload used to trip
/// `kpi_strip`'s old `is_all_absent` guard and render six dashes claiming "Backtest metrics
/// unavailable" — about a feed that was working perfectly, on precisely the
/// product's default first-run screen.
///
/// The honest render is the real (zero) values. `format_pct_max_dd` paints
/// Max-DD in `DOWN_500` **always**, including at zero, so the flat-but-Ready
/// strip is pixel-distinguishable from the dashes it used to be swallowed by.
#[test]
fn flat_ready_strip_renders_values_not_dashes() {
    let mut flat = Cockpit::new();
    flat.current_screen = Screen::Live;
    for secs in [0i64, 60, 120, 180, 240] {
        update(&mut flat, Message::PnlRefreshed(snap(secs, dec!(200))));
    }
    assert_eq!(
        flat.live_kpi.variant_name(),
        "ready",
        "a ≥2-point flat feed is Ready — nothing is missing, it just has not moved"
    );

    let stats = kpi_stats(&render_live(flat));
    assert!(
        stats.sentiment >= STRIP_DREW_MIN_SENTIMENT,
        "a HEALTHY FLAT feed rendered no value pixels ({} < \
         {STRIP_DREW_MIN_SENTIMENT}) — the screen is showing 'Backtest metrics \
         unavailable' for data that is fine and flat (2-15 review H2). 'No \
         data' and 'flat data' must not look identical.",
        stats.sentiment,
    );
}

// ── 2-15 review H3 — the caption must describe the window it covers ──────────

/// **H3 at the pixel layer.** Once the bounded ring evicts its head,
/// `live_equity_buffer[0]` is no longer the session open, so "Total return ·
/// Session to date" is a false claim about the number beside it (and a
/// drawdown whose peak was evicted has silently vanished from Max-DD).
///
/// The rendered caption must change. `LIVE_ROLLING_WINDOW_CAPTION` is ~5×
/// longer than `LIVE_SESSION_RETURN_CAPTION`, so it paints substantially more
/// muted (`FG_3`) glyph pixels in the caption band — a rasterized difference,
/// not a model assertion. The two cockpits are identical apart from the
/// truncation latch, so the extra pixels can only be the caption itself.
#[test]
fn truncated_window_renders_a_different_caption() {
    let untruncated = band_stats(&render_live(moving_session_cockpit()), CAP_Y, CAP_H).muted;

    let mut truncated_c = moving_session_cockpit();
    truncated_c.live_equity_window = LiveEquityWindow::Rolling;
    let truncated = band_stats(&render_live(truncated_c), CAP_Y, CAP_H).muted;

    // Calibration guard: the band is positioned for the POPULATED strip's
    // height (the unavailable strip carries an extra advisory line and pushes
    // the caption down out of it). Fail with a legible message rather than a
    // confusing "equal counts" if the layout ever moves.
    assert!(
        untruncated > 0,
        "caption-band calibration lost: the session-scoped caption paints no \
         muted pixels in {CAP_Y}..{} — re-locate the band (see the row-delta \
         method described above)",
        CAP_Y + CAP_H,
    );

    // Relational, not absolute: `LIVE_ROLLING_WINDOW_CAPTION` is 78 characters
    // against `LIVE_SESSION_RETURN_CAPTION`'s 15, so it paints several times
    // the glyph area on ANY font — which is what keeps this assertion valid
    // off-macOS, where metrics differ but the ratio does not.
    assert!(
        truncated > untruncated * 2 + 100,
        "the rolling-window caption must visibly replace the session-scoped one \
         once the ring has evicted: caption band painted {truncated} muted px \
         truncated vs {untruncated} px untruncated. Comparable counts mean the \
         screen is still claiming 'Session to date' for a window that has slid."
    );
}
