//! cockpit-performance-and-input-responsiveness — dev-vs-release render
//! timing probe (Phase 1 MEASURE).
//!
//! `cargo bench` always compiles under the `bench` profile (opt-level 3),
//! so it can only report the *release* render cost. The operator,
//! however, was running the **dev** profile (`cargo run … --features
//! fixtures`, opt-level 0). To measure the dev-vs-release factor from one
//! source of truth, this probe times `Emulator::screenshot()` by hand and
//! prints the median frame time. The SAME binary compiles under both
//! profiles:
//!
//! ```bash
//! # dev (opt-level per [profile.dev] — the operator's run profile)
//! cargo test -p ui --test render_timing_probe -- --ignored --nocapture
//! # release (opt-level 3 — the canonical interactive-run profile)
//! cargo test -p ui --release --test render_timing_probe -- --ignored --nocapture
//! ```
//!
//! Marked `#[ignore]` so the default `cargo test -p ui` gate (428 tests)
//! never runs it — perf timing is opt-in and machine-dependent, not a
//! pass/fail invariant.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::StreamExt;
use iced_test::futures::futures::channel::mpsc;
use iced_test::futures::futures::executor;

use ui::state::Screen;
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};

const READY_DEADLINE_TICKS: usize = 10;

/// Render `frames` screenshots of `screen` at `viewport`, return the
/// per-frame durations (sorted, includes emulator construction) PLUS the
/// matching construction-only durations so the caller can subtract the
/// fixed renderer-build cost and report the true per-frame render.
///
/// `Emulator::screenshot` (iced_test 0.14, emulator.rs:458) leaks its
/// `UserInterface::Cache` and panics on a second call, so each frame uses
/// a fresh emulator. `Emulator::new` builds the tiny-skia font system —
/// a once-at-startup cost in production, not per frame — so we time it
/// separately and subtract.
struct RenderSamples {
    /// Construct + boot + one screenshot (full pipeline + setup).
    full: Vec<Duration>,
    /// Construct + boot only (no screenshot) — the fixed setup cost.
    setup: Vec<Duration>,
}

fn time_render(screen: Screen, viewport: iced::Size, frames: usize) -> RenderSamples {
    // SAFETY: single-threaded test setup before any render.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = screen;
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let boot = |emulator: &mut Emulator<_>, rx: &mut mpsc::Receiver<_>| {
        executor::block_on(async {
            for _ in 0..READY_DEADLINE_TICKS {
                match rx.next().await {
                    Some(Event::Ready) => break,
                    Some(Event::Action(action)) => emulator.perform(&program, action),
                    Some(Event::Failed(_)) | None => break,
                }
            }
        });
    };

    // Warm up the allocator / OS pages.
    for _ in 0..3 {
        let (tx, mut rx) = mpsc::channel(64);
        let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
        boot(&mut emulator, &mut rx);
        let _ = emulator.screenshot(&program, &theme, 1.0);
    }

    let mut full = Vec::with_capacity(frames);
    let mut setup = Vec::with_capacity(frames);
    for _ in 0..frames {
        // Full: construct + boot + screenshot.
        let t0 = Instant::now();
        let (tx, mut rx) = mpsc::channel(64);
        let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
        boot(&mut emulator, &mut rx);
        let shot = emulator.screenshot(&program, &theme, 1.0);
        full.push(t0.elapsed());
        std::hint::black_box(shot);

        // Setup-only: construct + boot, no screenshot.
        let t1 = Instant::now();
        let (tx2, mut rx2) = mpsc::channel(64);
        let mut emulator2 = Emulator::new(tx2, &program, Mode::Immediate, viewport);
        boot(&mut emulator2, &mut rx2);
        setup.push(t1.elapsed());
        std::hint::black_box(&emulator2);
    }
    full.sort_unstable();
    setup.sort_unstable();
    RenderSamples { full, setup }
}

fn median(sorted: &[Duration]) -> Duration {
    sorted[sorted.len() / 2]
}

fn report(label: &str, s: &RenderSamples) {
    let n = s.full.len();
    let full_med = median(&s.full);
    let setup_med = median(&s.setup);
    // Per-frame render = full pipeline − fixed renderer construction.
    let render_med = full_med.saturating_sub(setup_med);
    let render_p95 = {
        // Pair-wise subtract at the same rank for a P95 estimate.
        let i = (n * 95 / 100).min(n - 1);
        s.full[i].saturating_sub(s.setup[i / 2])
    };
    let profile = if cfg!(debug_assertions) {
        "dev (opt-level per [profile.dev])"
    } else {
        "release (opt-level 3)"
    };
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    eprintln!(
        "RENDER-TIMING [{profile}] {label}: \
         render_median={:.2}ms  render_p95~={:.2}ms  \
         (full_median={:.2}ms − setup_median={:.2}ms)  (n={n})",
        ms(render_med),
        ms(render_p95),
        ms(full_med),
        ms(setup_med),
    );
}

#[test]
#[ignore = "perf probe — run explicitly with --ignored --nocapture"]
fn probe_lab_render_typical() {
    let s = time_render(Screen::Lab, iced::Size::new(1920.0, 1080.0), 40);
    report("lab_typical_1920x1080", &s);
}

#[test]
#[ignore = "perf probe — run explicitly with --ignored --nocapture"]
fn probe_lab_render_floor() {
    let s = time_render(Screen::Lab, iced::Size::new(1280.0, 720.0), 40);
    report("lab_floor_1280x720", &s);
}

#[test]
#[ignore = "perf probe — run explicitly with --ignored --nocapture"]
fn probe_live_render_typical() {
    let s = time_render(Screen::Live, iced::Size::new(1920.0, 1080.0), 40);
    report("live_typical_1920x1080", &s);
}
