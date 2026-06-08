//! cockpit-performance-and-input-responsiveness — interaction→render
//! micro-bench (Phase 1 MEASURE).
//!
//! The operator reported the cockpit lags **1–3 s per interaction** and
//! wants a ≥10× speedup. This bench objectively measures the
//! **interaction→render cost** — the full iced pipeline that runs on
//! every message / redraw:
//!
//! ```text
//!   program.view(&state)            // build the Element tree (work-bound)
//!     → UserInterface::build
//!     → ui.update(RedrawRequested)  // canvas::Program::update — rebuilds
//!                                    //   chart geometry every frame
//!     → ui.draw()                   // tiny-skia rasterization (render-bound)
//!     → renderer.screenshot(...)    // RGBA readback
//! ```
//!
//! `iced_test::emulator::Emulator::screenshot` runs **exactly** that
//! pipeline against the offscreen tiny-skia renderer the LIVE cockpit
//! uses (`crates/ui/Cargo.toml` features = ["tiny-skia", ...]). One call
//! == one rendered frame == the cost the operator pays per click.
//!
//! ## What the benches measure
//!
//! - `lab_screen_render_typical` — the **Lab/Charts** route at the
//!   operator-typical 1920×1080 viewport. This is the heavy screen:
//!   price-line `Canvas` + executed-fill markers + ghost signals +
//!   equity-overlay axis + volume histogram `Canvas` + position-curve
//!   `Canvas`. The worst-case interaction cost.
//! - `lab_screen_render_floor` — the same screen at the 1280×720 floor
//!   viewport (fewer pixels for tiny-skia to rasterize). The delta
//!   between this and `_typical` isolates the **render-bound** (pixel-
//!   count-proportional) component vs the **work-bound** (`view()` +
//!   geometry-rebuild, viewport-independent) component.
//! - `home_screen_render_typical` — a light, chart-free route as a
//!   baseline so we can attribute how much of the cost is the charts.
//!
//! ## Reading the debug-vs-release factor
//!
//! Run the SAME bench under both profiles and compare the reported
//! medians:
//!
//! ```bash
//! # dev (opt-level per [profile.dev] — operator's run profile)
//! cargo bench -p ui --bench cockpit_render -- --profile-time 5
//! # release (opt-level 3)
//! cargo bench -p ui --bench cockpit_render --release
//! ```
//!
//! No numeric thresholds are hard-coded — criterion compares against its
//! own saved baseline, and the human reads the dev-vs-release ratio off
//! the two runs.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::StreamExt;
use iced_test::futures::futures::channel::mpsc;
use iced_test::futures::futures::executor;

use ui::state::Screen;
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};

/// Bounded ticks to drain before we give up waiting for `Event::Ready`.
/// Fixtures boot is single-shot, so a healthy boot resolves in 1-3
/// events. Mirrors `tests/headless_emulator_smoke.rs`.
const READY_DEADLINE_TICKS: usize = 10;

/// Drain the emulator's boot events until `Ready` (or the deadline), so
/// the first timed `screenshot()` measures a steady-state frame rather
/// than a boot frame.
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

/// Time one full interaction→render frame: a fresh `Emulator` →
/// `view()` + `update(RedrawRequested)` (canvas geometry rebuild) +
/// `draw()` + tiny-skia RGBA readback — the exact cost an operator pays
/// per click in the live cockpit.
///
/// **Why a fresh `Emulator` per iteration:** `Emulator::screenshot`
/// (iced_test 0.14, emulator.rs:458) `take()`s its `UserInterface::Cache`
/// and never restores it, so a second `screenshot()` on the same emulator
/// panics on `Option::unwrap()`. Constructing a fresh emulator per frame
/// side-steps that. The fixed construction cost is identical across dev /
/// release and across screens, so it cancels in both the dev-vs-release
/// ratio and the heavy-minus-light (`lab` − `home`) chart-cost isolation.
///
/// The program + emulator are constructed inline (one scope) so their
/// `impl iced::Program` opaque types unify — passing them across a helper
/// boundary would mint two distinct opaque types iced's API rejects.
fn bench_render(c: &mut Criterion, name: &str, screen: Screen, viewport: iced::Size) {
    // Determinism for the chart's local-time axis (matches the snapshot
    // tests). Irrelevant to timing but keeps the rendered frame stable.
    // SAFETY: single-threaded bench setup before any render.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };

    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = screen;
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    c.bench_function(name, |b| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::channel(64);
            let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
            drive_to_ready(&mut emulator, &program, &mut rx);
            let shot = emulator.screenshot(&program, &theme, 1.0);
            black_box(shot);
        });
    });
}

/// Fixed-cost baseline: construct an `Emulator` + drive it to Ready, but
/// take **no** screenshot. `Emulator::new` builds the tiny-skia renderer
/// (font system, glyph atlas) — work that happens ONCE at app startup in
/// production, NOT per frame. Subtracting this from the `*_render_*`
/// benches isolates the true per-frame interaction→render cost:
///
/// ```text
///   per_frame_render ≈ <screen>_render_<vp>  −  emulator_construct_only
/// ```
fn emulator_construct_only(c: &mut Criterion) {
    // SAFETY: single-threaded bench setup before any render.
    unsafe { std::env::set_var(ui::strings::CHART_FORCE_UTC_ENV, "1") };
    let cockpit = charts_screen_cockpit();
    let program = program_from_cockpit(cockpit);
    let viewport = iced::Size::new(1920.0, 1080.0);

    c.bench_function("emulator_construct_only", |b| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::channel(64);
            let mut emulator = Emulator::new(tx, &program, Mode::Immediate, viewport);
            drive_to_ready(&mut emulator, &program, &mut rx);
            black_box(&emulator);
        });
    });
}

fn lab_screen_render_typical(c: &mut Criterion) {
    bench_render(
        c,
        "lab_screen_render_typical",
        Screen::Lab,
        iced::Size::new(1920.0, 1080.0),
    );
}

fn lab_screen_render_floor(c: &mut Criterion) {
    bench_render(
        c,
        "lab_screen_render_floor",
        Screen::Lab,
        iced::Size::new(1280.0, 720.0),
    );
}

fn home_screen_render_typical(c: &mut Criterion) {
    bench_render(
        c,
        "home_screen_render_typical",
        Screen::Live,
        iced::Size::new(1920.0, 1080.0),
    );
}

fn config() -> Criterion {
    // Render frames are slow (tens of ms in dev); shrink the sample/
    // measurement window so the suite finishes in a couple of minutes
    // even on the dev profile. Criterion still reports a stable median.
    Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(2))
}

criterion_group! {
    name = benches;
    config = config();
    targets =
        emulator_construct_only,
        lab_screen_render_typical,
        lab_screen_render_floor,
        home_screen_render_typical,
}
criterion_main!(benches);
