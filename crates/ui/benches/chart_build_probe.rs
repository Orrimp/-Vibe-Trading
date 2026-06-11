//! `cockpit-chart-cache` Phase 1 MEASURE — geometry-build vs raster
//! split on a hover frame. The go/no-go number for `canvas::Cache`.
//!
//! ## The question
//!
//! `iced`'s `geometry::Cache` skips the **geometry-build** cost (the
//! `Path::new` + `Frame::stroke` / `fill` work inside each chart
//! `canvas::Program::draw`) on a cache hit, but NOT the **raster**
//! cost (tiny-skia still draws the cached geometry every frame). On a
//! HOVER frame — a redraw with no data change, the exact case the
//! cache optimises — the cache's theoretical ceiling is:
//!
//! ```text
//!   speedup_ceiling = build_ns / frame_ns
//! ```
//!
//! If `build_ns` is a small fraction of the frame, the cache buys
//! little and is not worth the stale-chart invalidation risk → STOP.
//!
//! ## How it measures (honest, exact)
//!
//! `widgets::chart_build_probe` (feature `chart-build-probe`) brackets
//! every chart `draw` body with a thread-local nanosecond timer. One
//! `Emulator::screenshot` runs the FULL production hover frame
//! (`view → update(RedrawRequested) → draw → tiny-skia readback` —
//! `iced_test::emulator::Emulator::screenshot`). We:
//!
//!   1. `chart_build_probe::reset()`            — zero the accumulator,
//!   2. time `emulator.screenshot(...)`         — the whole frame,
//!   3. read `chart_build_probe::accumulated()` — the build-only time.
//!
//! `accumulated / frame` == the fraction a cache hit would skip, timed
//! against the EXACT production rasteriser. No `Renderer`
//! reconstruction, no Phase-2 plumbing.
//!
//! ## What the criterion bench reports
//!
//! `bench_function` times the **whole hover frame** (the comparison
//! denominator). The build fraction is printed ONCE to stderr from a
//! warm-up pass before the criterion loop, so the operator reads the
//! split off the bench output without parsing criterion's own report.
//!
//! ## Run
//!
//! ```bash
//! cargo bench -p ui --bench chart_build_probe \
//!   --features chart-build-probe -- --profile-time 5
//! ```
//!
//! (NOTE: must be run WITH `--features chart-build-probe`. Without it
//! the bench target is not selected — see the `required-features` gate
//! in `crates/ui/Cargo.toml`.)

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};

use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::StreamExt;
use iced_test::futures::futures::channel::mpsc;
use iced_test::futures::futures::executor;

use ui::state::Screen;
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};
use ui::widgets::chart_build_probe;

const READY_DEADLINE_TICKS: usize = 10;

fn drive_to_ready<P>(emulator: &mut Emulator<P>, program: &P, rx: &mut mpsc::Receiver<Event<P>>)
where
    P: iced::Program + 'static,
{
    executor::block_on(async {
        for _ in 0..READY_DEADLINE_TICKS {
            match rx.next().await {
                Some(Event::Ready) => break,
                Some(Event::Action(action)) => emulator.perform(program, action),
                Some(Event::Failed(_)) | None => break,
            }
        }
    });
}

fn median(sorted: &[Duration]) -> Duration {
    sorted[sorted.len() / 2]
}

/// Drive `frames` hover-frame screenshots, returning, per frame, the
/// (total frame, build-only) split read off the `chart_build_probe`
/// accumulator. A FRESH emulator per frame because
/// `Emulator::screenshot` (iced_test 0.14, emulator.rs:458) leaks its
/// `UserInterface::Cache` and panics on a second call — the same
/// constraint `benches/cockpit_render.rs` documents.
fn measure_split(
    screen: Screen,
    viewport: iced::Size,
    frames: usize,
) -> (Vec<Duration>, Vec<Duration>) {
    // SAFETY: single-threaded bench setup before any render.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = screen;
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let mut totals = Vec::with_capacity(frames);
    let mut builds = Vec::with_capacity(frames);

    // Warm the allocator / glyph atlas so the timed frames are steady-
    // state (the first paint pays one-time font-shaping costs that a
    // live hover frame never re-pays).
    for _ in 0..5 {
        let (tx, mut rx) = mpsc::channel(64);
        let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
        drive_to_ready(&mut emulator, &program, &mut rx);
        let _ = emulator.screenshot(&program, &theme, 1.0);
    }

    for _ in 0..frames {
        let (tx, mut rx) = mpsc::channel(64);
        let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
        drive_to_ready(&mut emulator, &program, &mut rx);

        chart_build_probe::reset();
        let t0 = Instant::now();
        let shot = emulator.screenshot(&program, &theme, 1.0);
        let frame_total = t0.elapsed();
        let build = chart_build_probe::accumulated();
        black_box(shot);

        totals.push(frame_total);
        builds.push(build);
    }
    totals.sort_unstable();
    builds.sort_unstable();
    (totals, builds)
}

/// Print the build-vs-raster split to stderr — the go/no-go number.
fn report_split(label: &str, totals: &[Duration], builds: &[Duration]) {
    let n = totals.len();
    let total_med = median(totals);
    let build_med = median(builds);
    // Raster (+view+update+readback) = whatever is NOT the chart build.
    let raster_med = total_med.saturating_sub(build_med);
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let frac = if total_med.as_nanos() == 0 {
        0.0
    } else {
        build_med.as_secs_f64() / total_med.as_secs_f64() * 100.0
    };
    let profile = if cfg!(debug_assertions) {
        "dev (opt-level per [profile.dev])"
    } else {
        "release/bench (opt-level 3)"
    };
    eprintln!(
        "CHART-BUILD-SPLIT [{profile}] {label}: \
         frame_median={:.3}ms  build_median={:.3}ms  raster+rest_median={:.3}ms  \
         build_fraction={frac:.1}%  (cache speedup ceiling = build_fraction)  (n={n})",
        ms(total_med),
        ms(build_med),
        ms(raster_med),
    );
}

fn config() -> Criterion {
    Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(2))
}

/// Lab/Charts at 1920×1080 — the heaviest chart route (the case the
/// cache helps MOST). If the build fraction is small HERE, it is
/// smaller everywhere else.
fn lab_hover_frame_typical(c: &mut Criterion) {
    let viewport = iced::Size::new(1920.0, 1080.0);
    // One-shot split report to stderr before the criterion timing loop.
    let (totals, builds) = measure_split(Screen::Lab, viewport, 40);
    report_split("lab_typical_1920x1080", &totals, &builds);

    // Criterion times the whole hover frame (the comparison baseline).
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Lab;
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    c.bench_function("lab_hover_frame_typical", |b| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::channel(64);
            let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
            drive_to_ready(&mut emulator, &program, &mut rx);
            let shot = emulator.screenshot(&program, &theme, 1.0);
            black_box(shot);
        });
    });
}

/// Lab/Charts at 1280×720 — the floor viewport. Fewer pixels for
/// tiny-skia, so the raster share shrinks and the build share grows;
/// the build-fraction here is the MOST generous case for the cache.
fn lab_hover_frame_floor(c: &mut Criterion) {
    let viewport = iced::Size::new(1280.0, 720.0);
    let (totals, builds) = measure_split(Screen::Lab, viewport, 40);
    report_split("lab_floor_1280x720", &totals, &builds);

    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Lab;
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    c.bench_function("lab_hover_frame_floor", |b| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::channel(64);
            let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
            drive_to_ready(&mut emulator, &program, &mut rx);
            let shot = emulator.screenshot(&program, &theme, 1.0);
            black_box(shot);
        });
    });
}

criterion_group! {
    name = benches;
    config = config();
    targets = lab_hover_frame_typical, lab_hover_frame_floor,
}
criterion_main!(benches);
