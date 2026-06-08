//! Phase-1 MEASURE instrumentation — chart geometry-build timer.
//!
//! `cockpit-chart-cache` Phase 1. Answers the question the operator's
//! brief poses before any `canvas::Cache` work ships:
//!
//! > On a **hover frame** (a redraw with NO data change — the exact
//! > case the cache optimises), what fraction of the frame is spent
//! > **building** the chart geometry (the `Path::new` + `Frame::stroke`
//! > / `fill` work each `canvas::Program::draw` does) versus
//! > **rasterising** the built geometry to pixels (tiny-skia)?
//!
//! `iced`'s `geometry::Cache` eliminates the *build* cost on a cache
//! hit (the closure is skipped, the stored geometry is returned) but
//! NOT the *raster* cost (the cached geometry is still drawn each
//! frame). So the cache's ceiling = build / (build + raster). If that
//! fraction is small, the cache is not worth the stale-chart
//! invalidation risk and we STOP.
//!
//! ## How it measures (no renderer reconstruction)
//!
//! Each chart `Program::draw` brackets its geometry-build body with
//! [`BuildTimer::start`]. The guard's `Drop` adds the elapsed wall-time
//! to a thread-local nanosecond accumulator. A Phase-1 bench / probe
//! then, on a single screenshot frame (which runs the full
//! `view → update → draw → tiny-skia-readback` pipeline — see
//! `iced_test::emulator::Emulator::screenshot`), records BOTH:
//!
//! - the accumulator (== total time inside every chart `draw`'s build
//!   body == the work a cache hit would skip), and
//! - the whole screenshot wall-time (== the full hover frame).
//!
//! `build_ns / frame_ns` is the cache's theoretical ceiling. This times
//! the **exact** code the cache skips through the **exact** production
//! rasteriser, so it needs no `Renderer` reconstruction and no Phase-2
//! plumbing to exist first.
//!
//! ## Zero production cost
//!
//! The whole module is gated behind the `chart-build-probe` cargo
//! feature. With the feature off (every production build —
//! `cockpit`, `cockpit_live`, `viewer`), [`BuildTimer::start`] is an
//! `#[inline]` no-op returning a zero-sized guard with an empty `Drop`,
//! and the accumulator / reader functions do not exist. The probe is
//! enabled only by the Phase-1 bench (`cargo bench -p ui --bench
//! chart_build_probe --features chart-build-probe`).

/// RAII guard returned by [`BuildTimer::start`]. While the feature is
/// on, records `Instant::now()` at construction and, on `Drop`, adds
/// the elapsed nanoseconds to the thread-local accumulator. While the
/// feature is off, this is a zero-sized type with a no-op `Drop`.
#[cfg(feature = "chart-build-probe")]
pub(crate) struct BuildTimer {
    start: std::time::Instant,
}

#[cfg(feature = "chart-build-probe")]
mod imp {
    use std::cell::Cell;
    use std::time::Duration;

    thread_local! {
        /// Accumulated geometry-build time on this thread since the
        /// last [`reset`]. Single-threaded by construction — every
        /// chart `draw` runs on iced's UI thread, which is the same
        /// thread the Phase-1 probe drives the emulator on.
        static BUILD_ACCUM: Cell<Duration> = const { Cell::new(Duration::ZERO) };
    }

    /// Add `elapsed` to this thread's build accumulator.
    pub(super) fn add(elapsed: Duration) {
        BUILD_ACCUM.with(|c| c.set(c.get() + elapsed));
    }

    /// Reset the accumulator to zero. Call before the timed frame.
    pub fn reset() {
        BUILD_ACCUM.with(|c| c.set(Duration::ZERO));
    }

    /// Read the accumulated build time without resetting.
    #[must_use]
    pub fn accumulated() -> Duration {
        BUILD_ACCUM.with(Cell::get)
    }
}

#[cfg(feature = "chart-build-probe")]
pub use imp::{accumulated, reset};

#[cfg(feature = "chart-build-probe")]
impl BuildTimer {
    /// Start timing a geometry-build body. Hold the returned guard for
    /// the duration of the path-construction work; its `Drop` records
    /// the elapsed time into the thread-local accumulator.
    #[inline]
    #[must_use]
    pub(crate) fn start() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "chart-build-probe")]
impl Drop for BuildTimer {
    #[inline]
    fn drop(&mut self) {
        imp::add(self.start.elapsed());
    }
}

// ---- Feature-off shim: zero-sized, zero-cost. ----

/// Zero-sized no-op guard when the `chart-build-probe` feature is off.
/// Identical call shape (`let _t = BuildTimer::start();`) so the chart
/// `draw` impls carry one unconditional line with no `#[cfg]` clutter.
#[cfg(not(feature = "chart-build-probe"))]
pub(crate) struct BuildTimer;

#[cfg(not(feature = "chart-build-probe"))]
impl BuildTimer {
    #[inline]
    #[must_use]
    pub(crate) fn start() -> Self {
        Self
    }
}
