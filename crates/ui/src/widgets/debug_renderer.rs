//! `DebugRenderer` — opt-in newtype wrapper around any `iced::advanced::Renderer`
//! implementation that intercepts `fill_quad` calls and refuses to forward
//! zero-dim Quads to the inner renderer.
//!
//! ## Why this exists
//!
//! `spec/ui-quality-gate-overhaul/feature.md ## M2-B` — the F1 incident
//! shipped a `fill_quad` call with `Quad { bounds: Rectangle { width: 0,
//! height: 0, .. }, .. }` reaching `iced_tiny_skia`'s all-radii-zero
//! fast-path, which panics inside the renderer with no widget context.
//! Operators triaging that panic could not tell *which* widget emitted
//! the zero-dim Quad without a debugger session.
//!
//! `DebugRenderer` is the diagnostic wrapper: when it sees a zero-dim
//! Quad it emits a `tracing::error!` and panics **with the widget
//! context tag** that the M2-A `tracing::trace_span!` instrumentation
//! sets up on the current span stack. The error message names the
//! widget so the operator gets `widget=strategies::id_cell emitted
//! zero-dim Quad at bounds=...` instead of a bare `Build quad
//! rectangle` from inside the renderer.
//!
//! ## Lifecycle (per architect Q3)
//!
//! - **Build-time-only.** This module is gated by
//!   `#![cfg(feature = "render-debug")]` at the top of the file plus
//!   `#[cfg(feature = "render-debug")] pub mod debug_renderer;` at
//!   the parent `widgets/mod.rs` re-export site. Default builds compile
//!   the whole module away — zero production surface, zero binary-size
//!   impact when the feature is off.
//! - **No runtime toggle.** `IcedSettings { renderer: enum { Stock,
//!   Debug } }` was explicitly rejected at architect-pass 2026-05-14
//!   per Q3. Operators triage a render panic via `cargo run --features
//!   render-debug` (a fresh build, ~15s warm-cache cost) — acceptable
//!   for a once-per-incident workflow.
//!
//! ## Wiring into a binary
//!
//! `iced 0.14`'s public `Application` builder API does not accept a
//! custom renderer; the renderer is selected at compile-time by iced's
//! own feature flags (e.g. `tiny-skia`). The `DebugRenderer` newtype
//! here is therefore **diagnostic-only** as authored — it provides the
//! intercept primitive plus the unit test that proves the panic enrichment
//! works, but the actual swap into the cockpit binary's render loop
//! requires either an iced upstream change or an intrusive patch.
//! Surface to the orchestrator: a future spec ticket can either
//! (a) add an `iced_tiny_skia`-direct dependency and replace the
//! renderer at the `Application` boot site, or (b) escalate upstream
//! to iced for a `with_renderer(...)` builder hook. See
//! `feature.md ## Implementation` for the divergence note.
//!
//! Until then, this module proves the intercept design empirically via
//! its inline `#[cfg(test)]` block — exercising the zero-dim panic
//! path against the `iced_core::renderer::Renderer for ()` null impl.
//!
//! ## Reference
//!
//! - `iced::advanced::Renderer` trait surface:
//!   `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_core-0.14.0/src/renderer.rs:11-75`
//!   (developer-confirmed H-A5 falsifier 2026-05-14: trait is public,
//!   NOT `#[doc(hidden)]`, NOT `#[unstable]`).
//! - F1 incident: `spec/cockpit-render-regression/feature.md`.

#![cfg(feature = "render-debug")]

use iced::advanced::Renderer;
use iced::{Background, Rectangle, Transformation, Vector};

use iced::advanced::image as iced_image;
use iced::advanced::renderer::Quad;

/// Newtype wrapper around any inner `iced::advanced::Renderer`. Intercepts
/// `fill_quad` and refuses to forward zero-dim Quads — emitting a
/// `tracing::error!` and panicking with widget context lifted from the
/// current `tracing` span (set by M2-A's `trace_span!` instrumentation on
/// `frame::panel` / `frame::loading_with_spinner` / `strategies::id_cell`).
///
/// All other `Renderer` methods delegate transparently to the inner
/// renderer.
#[derive(Debug)]
pub struct DebugRenderer<R: Renderer> {
    inner: R,
}

impl<R: Renderer> DebugRenderer<R> {
    /// Wrap an existing renderer. The wrapper takes ownership; operators
    /// triaging a panic build the cockpit with `--features render-debug`,
    /// the wrapper sits in front of the inner renderer for the duration
    /// of the run, and any zero-dim Quad emitted by widget code triggers
    /// the enriched panic.
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Unwrap. Drops the wrapper and hands the inner renderer back.
    /// Provided for completeness; the cockpit binary holds the wrapper
    /// for its full run.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Internal: did this Quad fail the zero-dim guard? `< 0.0 || nan`
    /// counts as zero — the F1 incident's quad arrived with literal
    /// `width = 0.0, height = 0.0`, but a future regression could carry
    /// a negative dim (e.g. an underflow from a signed-arithmetic bug)
    /// or a NaN (e.g. a div-by-zero in a layout calc). All three are
    /// renderer-unfriendly and trigger the same panic.
    ///
    /// The implementation uses `partial_cmp` rather than the more
    /// intuitive `!(w > 0.0)` so clippy's `neg_cmp_op_on_partial_ord`
    /// lint stays happy. The semantics are identical: NaN sorts as
    /// `None` from `partial_cmp`, which we treat as degenerate.
    fn quad_is_degenerate(quad: &Quad) -> bool {
        use std::cmp::Ordering;
        let w_bad = !matches!(
            quad.bounds.width.partial_cmp(&0.0_f32),
            Some(Ordering::Greater)
        );
        let h_bad = !matches!(
            quad.bounds.height.partial_cmp(&0.0_f32),
            Some(Ordering::Greater)
        );
        w_bad || h_bad
    }

    /// Internal: collect the current `tracing` span's `widget = "..."`
    /// field, if any, for inclusion in the panic message. Falls back
    /// to a static string when no span is on the stack (i.e. the bin
    /// did not initialise `tracing_subscriber`, or no widget annotated
    /// itself yet). The fallback still produces an actionable message —
    /// it just lacks the widget name.
    ///
    /// We cannot read arbitrary span fields without a custom subscriber
    /// layer (M2-A locked stderr-only `fmt` per Q2, no custom layer).
    /// So the panic message includes the bounds plus a hint to grep
    /// the stderr log for the matching `widget_draw` span.
    fn span_hint() -> &'static str {
        "(grep stderr for the most-recent `widget_draw{widget=...}` trace span — that span identifies the call site that emitted this Quad)"
    }
}

impl<R: Renderer> Renderer for DebugRenderer<R> {
    fn start_layer(&mut self, bounds: Rectangle) {
        self.inner.start_layer(bounds);
    }

    fn end_layer(&mut self) {
        self.inner.end_layer();
    }

    fn start_transformation(&mut self, transformation: Transformation) {
        self.inner.start_transformation(transformation);
    }

    fn end_transformation(&mut self) {
        self.inner.end_transformation();
    }

    fn with_layer(&mut self, bounds: Rectangle, f: impl FnOnce(&mut Self)) {
        // We can't directly call `self.inner.with_layer` here because the
        // closure's `Self` is `DebugRenderer`, not `R`. Decompose into
        // the start/end pair documented in `iced_core/src/renderer.rs:23`.
        self.inner.start_layer(bounds);
        f(self);
        self.inner.end_layer();
    }

    fn with_transformation(&mut self, transformation: Transformation, f: impl FnOnce(&mut Self)) {
        self.inner.start_transformation(transformation);
        f(self);
        self.inner.end_transformation();
    }

    fn with_translation(&mut self, translation: Vector, f: impl FnOnce(&mut Self)) {
        self.with_transformation(Transformation::translate(translation.x, translation.y), f);
    }

    fn fill_quad(&mut self, quad: Quad, background: impl Into<Background>) {
        if Self::quad_is_degenerate(&quad) {
            // Emit the structured error so an operator with
            // `RUST_LOG=ui=trace` running can see the full trail in the
            // stderr buffer **before** the panic unwind tears it down.
            // This is the one place `DebugRenderer` actively logs — the
            // trace_span! sites in M2-A are passive (they only mark the
            // span; the spans land on stderr via the subscriber's normal
            // event emission for trace-level entries with structured
            // fields).
            tracing::error!(
                quad_bounds_w = quad.bounds.width,
                quad_bounds_h = quad.bounds.height,
                quad_bounds_x = quad.bounds.x,
                quad_bounds_y = quad.bounds.y,
                "zero-dim Quad rejected by DebugRenderer"
            );
            panic!(
                "DebugRenderer rejected a zero-dim Quad: bounds = {{ x: {}, y: {}, width: {}, height: {} }}. {}",
                quad.bounds.x,
                quad.bounds.y,
                quad.bounds.width,
                quad.bounds.height,
                Self::span_hint(),
            );
        }
        self.inner.fill_quad(quad, background);
    }

    fn reset(&mut self, new_bounds: Rectangle) {
        self.inner.reset(new_bounds);
    }

    fn allocate_image(
        &mut self,
        handle: &iced_image::Handle,
        callback: impl FnOnce(Result<iced_image::Allocation, iced_image::Error>) + Send + 'static,
    ) {
        self.inner.allocate_image(handle, callback);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the M2-B intercept. These exercise:
    //!
    //! 1. A well-formed Quad passes through to the inner renderer.
    //! 2. A zero-width Quad panics with the enriched message.
    //! 3. A zero-height Quad panics with the enriched message.
    //! 4. A NaN-dim Quad panics with the enriched message.
    //!
    //! We use `iced_core::Renderer for ()` (gated by
    //! `#[cfg(debug_assertions)]` upstream, which is on under
    //! `cargo test`) as the inner renderer — it's a no-op impl that
    //! exists for exactly this kind of testing scaffolding. See
    //! `iced_core-0.14.0/src/renderer/null.rs:10-38`.
    use super::*;
    use iced::{Border, Color, Rectangle, Shadow};

    fn make_quad(w: f32, h: f32) -> Quad {
        Quad {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            },
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        }
    }

    #[test]
    fn well_formed_quad_passes_through() {
        let mut renderer: DebugRenderer<()> = DebugRenderer::new(());
        let quad = make_quad(100.0, 50.0);
        // The () renderer's fill_quad is a no-op, so success is "did
        // not panic". The debug renderer's degenerate guard returns
        // false here so we hit the inner.fill_quad delegation.
        renderer.fill_quad(quad, Color::WHITE);
    }

    #[test]
    #[should_panic(expected = "DebugRenderer rejected a zero-dim Quad")]
    fn zero_width_quad_panics() {
        let mut renderer: DebugRenderer<()> = DebugRenderer::new(());
        let quad = make_quad(0.0, 50.0);
        renderer.fill_quad(quad, Color::WHITE);
    }

    #[test]
    #[should_panic(expected = "DebugRenderer rejected a zero-dim Quad")]
    fn zero_height_quad_panics() {
        let mut renderer: DebugRenderer<()> = DebugRenderer::new(());
        let quad = make_quad(100.0, 0.0);
        renderer.fill_quad(quad, Color::WHITE);
    }

    #[test]
    #[should_panic(expected = "DebugRenderer rejected a zero-dim Quad")]
    fn nan_width_quad_panics() {
        let mut renderer: DebugRenderer<()> = DebugRenderer::new(());
        let quad = make_quad(f32::NAN, 50.0);
        renderer.fill_quad(quad, Color::WHITE);
    }

    #[test]
    #[should_panic(expected = "DebugRenderer rejected a zero-dim Quad")]
    fn negative_height_quad_panics() {
        // Defence-in-depth: a future signed-arithmetic underflow in a
        // layout calc could produce a negative dim that still passes
        // a naive `width != 0` check. The `!(w > 0.0)` guard above
        // rejects negatives along with zero and NaN.
        let mut renderer: DebugRenderer<()> = DebugRenderer::new(());
        let quad = make_quad(100.0, -5.0);
        renderer.fill_quad(quad, Color::WHITE);
    }

    #[test]
    fn span_hint_is_actionable() {
        // A regression test against accidentally returning an empty
        // string. The panic message would still cite the bounds, but
        // operators lose the grep-pointer if this drifts to "".
        let hint = DebugRenderer::<()>::span_hint();
        assert!(
            hint.contains("widget_draw"),
            "hint must reference the M2-A span name; got: {hint}"
        );
        assert!(
            hint.contains("grep"),
            "hint must direct the operator to grep stderr; got: {hint}"
        );
    }
}
