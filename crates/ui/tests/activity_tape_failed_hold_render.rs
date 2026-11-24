//! bug-log `#217` — the status bar's failed activity row, at the rendered-PIXEL
//! layer (AD-10): red inside its 3 s hold, GONE after it, and still drawn when
//! no tick purges it.
//!
//! ## What this file drives
//!
//! The full cockpit shell (`ui::test_support::program_from_cockpit`, so
//! `shell::view` with `widgets::status_bar` and its `widgets::activity_tape`)
//! through `iced_test::screenshot` at 1920×1080, scale 1.0, a fresh screenshot
//! per frame. The purge runs through the production arm, `ui::state::update`
//! with `Message::ToastTick(now)`: the message the always-on `ToastDismiss`
//! unit sends every 500 ms (`subscription_batch_inclusion.rs`'s
//! `batched_toast_tick_purges_the_activity_tape` pins that the batch sends it).
//!
//! **The seed.** One `YahooPreload` row, started at `base = Instant::now() + 1 h`
//! and failed at `base + 10 s`, by `ActivityTape::apply_at` (ADR-0046 K5). Its
//! hold ends at `base + 13 s` (the failure + the spec literal 3 s, R2.5).
//! Because `base` is in the future, the widget's `now.saturating_duration_since(
//! started_at)` is zero and the row reads "Loading data <1s" in every frame, so
//! the frames are deterministic (`widgets/activity_tape.rs`'s CLOCK-OK block
//! names this file as the one populated render of the tape).
//!
//! **The frames.**
//! - A: the seed, no tick (the shipped cockpit before `#217`: nothing purges);
//! - B: the seed after `ToastTick(base + 12.999 s)`, the last instant inside the hold;
//! - C: the seed after `ToastTick(base + 13 s)`, the deadline;
//! - E: `Cockpit::default()`, the empty tape.
//!
//! **The band.** The status bar's 24 px content row (T1508), rows `[h − 24, h)`,
//! from the end of the 180 px sidebar (`theme::layout::SIDEBAR_WIDTH_PX`) to the
//! right edge: the bar and nothing else. Pixels are classed as `DOWN_500` clay
//! (dark #C97B5E; the classifier of `benchmark_wins_render.rs`) and as
//! foreground (integer luma `(2r + 5g + b) / 8 > 70`; the `PANEL` background
//! is about 33, `FG_3` text about 136).
//!
//! ## Tests
//!
//! | test | what it pins |
//! |---|---|
//! | `failed_row_is_red_inside_the_hold` (T1) | clay(B) ≥ [`CLAY_FLOOR`] AND band(B) == band(A) byte for byte: a tick inside the hold changes nothing drawn |
//! | `failed_row_is_gone_after_the_hold` (T2) | band(C) == band(E) byte for byte (the row is GONE, not recoloured) AND clay(C) ≤ [`CLAY_STRAY`] |
//! | `without_a_tick_the_failed_row_stays_red` (T3) | the NEGATIVE CONTROL, the pre-`#217` behaviour: clay(A) ≥ [`CLAY_FLOOR`] AND band(A) != band(E) |
//! | `empty_status_bar_has_no_clay` (T4) | the baseline and scope check (bug-log `#188`): clay(E) ≤ [`CLAY_STRAY`] AND foreground(E) ≥ [`CHROME_FLOOR`], so the band covers the bar's chrome text |
//!
//! ## Thresholds: fixed literals, measured once (2026-11-24, macOS dev box)
//!
//! clay(A) = clay(B) = 156, clay(C) = clay(E) = 0; foreground(A) = 980,
//! foreground(E) = 724; A and E differ in 2456 band pixels.
//! [`CLAY_FLOOR`] = clay(A) / 2 = 78; [`CLAY_STRAY`] = clay(E) + 5 = 5;
//! [`CHROME_FLOOR`] = foreground(E) / 2 = 362. None is recomputed at run time.
//! Red first: S0 (these tests, no fix) still drew the row in C (clay(C) = 156,
//! T2 red); S1 (the purge line alone, the hold still counted from the start)
//! had purged it before B (clay(B) = 0, T1 red); S2 (the hold from the
//! failure) is green.
//!
//! ## T-T4 probes (one edit each, MEASURED 2026-11-24)
//!
//! | probe | one edit | red here |
//! |---|---|---|
//! | T217-P1 | delete `model.activity_tape.purge(now);` from the `ToastTick` arm (`state.rs`) | T2 |
//! | T217-P2 | `now + Duration::from_secs(ACTIVITY_FAILED_HOLD_SECS)` → `state.started_at + Duration::from_secs(ACTIVITY_FAILED_HOLD_SECS)` (`lab/activity.rs`, the `Failed` arm) | T1 |
//! | T217-P3 | `ACTIVITY_FAILED_HOLD_SECS: u64 = 3` → `2` | T1 |
//! | T217-P4 | `ACTIVITY_FAILED_HOLD_SECS: u64 = 3` → `4` | T2 |
//! | T217-P5 | `s.red_hold_until.is_some_and(\|d\| now < d)` → `now <= d` in `purge` | T2 |
//! | T217-P7 | `purge(now)` → `purge(now + std::time::Duration::from_secs(7200))` in the arm | T1 |
//! | T217-P8 | `purge(now)` → `purge(std::time::Instant::now())` in the arm (the K5 violation) | T2 |
//! | T217-P9 | the widget's `let is_failed = …` → `let is_failed = false;` | T1, T3 |
//! | T217-P10 | `.push(tape_widget)` removed from `status_bar.rs` | T1, T3 |
//! | T217-P12 | `.push(bar)` removed from `shell.rs`'s centre column | T1, T3, T4 |
//!
//! P6, P11 and P13 have no red here by design (they are the lib tests' and
//! the relay's; P13, `apply`'s pre-#217 body, is red only at the lib test
//! `apply_counts_the_hold_from_the_failure_it_applies`, because this file
//! seeds through `apply_at`). The full table is bug-log `#217`'s.
//!
//! ## macOS gate (ADR-0057 D2)
//!
//! Real-renderer pixel assertions are macOS-canonical; this file compiles to
//! nothing on Windows. `scripts/ci_test_target_counts.py` declares it at 4 tests
//! on macOS and 0 on Windows, so a silent zero on the canonical leg is a CI
//! failure, not a pass (bug-log `#190`).
//!
//! PNGs for a human read (render-verification rule 2):
//! `/tmp/activity_tape_hold_{a,b,c,e}.png`.

#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::{Duration, Instant};

use trading_core::{ActivityEvent, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase};
use ui::state::{Cockpit, Message, update};
use ui::test_support::program_from_cockpit;

const VIEW_W: u32 = 1920;
const VIEW_H: u32 = 1080;
const SCALE: f32 = 1.0;

/// The status bar's content row height (T1508; `status_bar.rs`'s `BAR_HEIGHT`).
const BAR_H: u32 = 24;

/// clay(A) / 2, with clay(A) = 156 measured (see the module doc).
const CLAY_FLOOR: usize = 78;
/// clay(E) + 5, with clay(E) = 0 measured.
const CLAY_STRAY: usize = 5;
/// foreground(E) / 2, with foreground(E) = 724 measured.
const CHROME_FLOOR: usize = 362;

static INIT_UTC: std::sync::Once = std::sync::Once::new();

fn render(cockpit: Cockpit) -> iced::window::Screenshot {
    INIT_UTC.call_once(ui::force_chart_utc_for_tests);
    let program = program_from_cockpit(cockpit);
    iced_test::screenshot(
        &program,
        &iced::Theme::Dark,
        (VIEW_W, VIEW_H),
        SCALE,
        Duration::ZERO,
    )
}

fn dump_png(shot: &iced::window::Screenshot, path: &str) {
    if let Some(img) =
        image::RgbaImage::from_raw(shot.size.width, shot.size.height, shot.rgba.to_vec())
    {
        let _ = img.save(path);
    }
}

/// The status bar's content row, as RGBA bytes: rows `[h − 24, h)`, columns
/// from the end of the sidebar to the right edge.
fn band(shot: &iced::window::Screenshot) -> Vec<u8> {
    let (w, h) = (shot.size.width, shot.size.height);
    assert!(
        w == VIEW_W && h == VIEW_H,
        "frame is {w}×{h}, expected {VIEW_W}×{VIEW_H} at scale 1.0"
    );
    let x0 = ui::theme::layout::SIDEBAR_WIDTH_PX as u32;
    let mut out = Vec::with_capacity(((w - x0) * BAR_H * 4) as usize);
    for y in (h - BAR_H)..h {
        let row = (y * w) as usize * 4;
        out.extend_from_slice(&shot.rgba[row + x0 as usize * 4..row + w as usize * 4]);
    }
    out
}

/// `DOWN_500` clay (dark #C97B5E = 201, 123, 94): red high and dominant over
/// green, green over blue (`benchmark_wins_render.rs`'s `is_down_clay`).
fn is_down_clay(r: i32, g: i32, b: i32) -> bool {
    r > 150 && (r - g) > 40 && (g - b) > 12 && b < 130
}

fn pixels(band: &[u8]) -> impl Iterator<Item = (i32, i32, i32)> + '_ {
    band.chunks_exact(4)
        .map(|p| (i32::from(p[0]), i32::from(p[1]), i32::from(p[2])))
}

fn clay(band: &[u8]) -> usize {
    pixels(band)
        .filter(|&(r, g, b)| is_down_clay(r, g, b))
        .count()
}

fn foreground(band: &[u8]) -> usize {
    pixels(band)
        .filter(|&(r, g, b)| (r * 2 + g * 5 + b) / 8 > 70)
        .count()
}

/// Number of pixels that differ between two bands of the same size.
fn differing(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len(), "bands of different sizes");
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(p, q)| p != q)
        .count()
}

fn event(phase: ActivityPhase) -> ActivityEvent {
    ActivityEvent {
        id: ActivityId(217),
        kind: ActivityKind::YahooPreload,
        label: "t217 preload".to_owned(),
        phase,
        ts_ms: 0,
    }
}

/// The seed (frame A) and its `base`: one preload started at `base` and failed
/// at `base + 10 s`, with `base` an hour ahead so the drawn label is `<1s`.
fn seeded() -> (Cockpit, Instant) {
    // CLOCK-OK: the seed is an hour AHEAD of the clock, so the widget's
    // `now.saturating_duration_since(started_at)` is zero in every frame and
    // the label is always `<1s` (`widgets/activity_tape.rs`'s marker).
    let base = Instant::now() + Duration::from_secs(3600);
    let mut cockpit = Cockpit::default();
    cockpit
        .activity_tape
        .apply_at(event(ActivityPhase::Start { total_units: None }), base);
    cockpit.activity_tape.apply_at(
        event(ActivityPhase::End(ActivityOutcome::Failed(
            "t217 failure".to_owned(),
        ))),
        base + Duration::from_secs(10),
    );
    assert_eq!(
        cockpit.activity_tape.visible().len(),
        1,
        "precondition: the seed puts one failed row on the tape"
    );
    (cockpit, base)
}

/// The seed after one production `ToastTick(base + offset)`.
fn seeded_after_tick(offset: Duration) -> Cockpit {
    let (mut cockpit, base) = seeded();
    update(&mut cockpit, Message::ToastTick(base + offset));
    cockpit
}

/// T1 — a tick inside the hold leaves the failed row drawn, red, unchanged.
#[test]
fn failed_row_is_red_inside_the_hold() {
    let a = render(seeded().0);
    let b = render(seeded_after_tick(Duration::from_millis(12_999)));
    dump_png(&b, "/tmp/activity_tape_hold_b.png");
    let (band_a, band_b) = (band(&a), band(&b));
    let clay_b = clay(&band_b);
    eprintln!(
        "T1: clay(A) = {}, clay(B) = {clay_b}, foreground(B) = {}",
        clay(&band_a),
        foreground(&band_b)
    );
    assert!(
        clay_b >= CLAY_FLOOR,
        "T1: inside the hold (ToastTick at base + 12.999 s) the failed row draws red: \
         clay(B) = {clay_b}, floor {CLAY_FLOOR}. PNG: /tmp/activity_tape_hold_b.png"
    );
    assert_eq!(
        differing(&band_a, &band_b),
        0,
        "T1: a tick inside the hold must change nothing in the status bar (B vs A)"
    );
}

/// T2 — a tick at the deadline removes the row: the bar draws as if the tape
/// were empty.
#[test]
fn failed_row_is_gone_after_the_hold() {
    let c = render(seeded_after_tick(Duration::from_secs(13)));
    let e = render(Cockpit::default());
    dump_png(&c, "/tmp/activity_tape_hold_c.png");
    let (band_c, band_e) = (band(&c), band(&e));
    let clay_c = clay(&band_c);
    eprintln!("T2: clay(C) = {clay_c}, clay(E) = {}", clay(&band_e));
    assert_eq!(
        differing(&band_c, &band_e),
        0,
        "T2: after the hold (ToastTick at base + 13 s) the status bar equals the empty-tape \
         bar byte for byte (C vs E). PNG: /tmp/activity_tape_hold_c.png"
    );
    assert!(
        clay_c <= CLAY_STRAY,
        "T2: no clay after the hold: clay(C) = {clay_c}, stray bound {CLAY_STRAY}"
    );
}

/// T3 — the negative control: with no tick nothing purges, and the failed
/// row stays red (the shipped cockpit before bug-log `#217`).
#[test]
fn without_a_tick_the_failed_row_stays_red() {
    let a = render(seeded().0);
    let e = render(Cockpit::default());
    dump_png(&a, "/tmp/activity_tape_hold_a.png");
    let (band_a, band_e) = (band(&a), band(&e));
    let clay_a = clay(&band_a);
    eprintln!(
        "T3: clay(A) = {clay_a}, foreground(A) = {}, differing(A, E) = {}",
        foreground(&band_a),
        differing(&band_a, &band_e)
    );
    assert!(
        clay_a >= CLAY_FLOOR,
        "T3: with no tick the failed row is drawn red: clay(A) = {clay_a}, floor {CLAY_FLOOR}. \
         PNG: /tmp/activity_tape_hold_a.png"
    );
    assert_ne!(
        differing(&band_a, &band_e),
        0,
        "T3: with no tick the status bar must differ from the empty-tape bar (A vs E)"
    );
}

/// T4 — the baseline and scope check: the empty bar has no clay, and the band
/// holds the bar's chrome text (so a band that missed the bar cannot pass T2).
#[test]
fn empty_status_bar_has_no_clay() {
    let e = render(Cockpit::default());
    dump_png(&e, "/tmp/activity_tape_hold_e.png");
    let band_e = band(&e);
    let (clay_e, fg_e) = (clay(&band_e), foreground(&band_e));
    eprintln!("T4: clay(E) = {clay_e}, foreground(E) = {fg_e}");
    assert!(
        clay_e <= CLAY_STRAY,
        "T4: the empty status bar draws no clay: clay(E) = {clay_e}, stray bound {CLAY_STRAY}"
    );
    assert!(
        fg_e >= CHROME_FLOOR,
        "T4: the band holds the status bar's chrome text: foreground(E) = {fg_e}, \
         floor {CHROME_FLOOR}. PNG: /tmp/activity_tape_hold_e.png"
    );
}
