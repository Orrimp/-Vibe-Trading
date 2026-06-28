//! Throttled spinner — local 10 fps replacement for `iced_aw::Spinner`.
//!
//! ## Why this exists (cockpit-performance-and-input-responsiveness M1 Candidate A)
//!
//! Upstream `iced_aw::Spinner` ships with `FRAMES_PER_SECOND = 60` baked
//! into its `update` impl
//! (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/src/widget/spinner.rs:176-201`).
//! Each `RedrawRequested` event the iced runtime delivers to a Spinner
//! instance triggers a `shell.request_redraw_at(now + 16ms)` re-schedule;
//! one visible Spinner is enough to pull the whole cockpit window into
//! 60 fps software-rasterized (`iced_tiny_skia`) repaint.
//!
//! Orchestrator-executed M0 profile (2026-05-15) confirmed the empirical
//! cost: idle cockpit at ~66.9 % CPU with
//! `iced_tiny_skia::Compositor::present` accounting for 45.5 % of
//! main-thread time. Per
//! [`feature.md ## M0 results`](../../../spec/v1/cockpit-performance-and-input-responsiveness/feature.md#m0-results-orchestrator-executed-2026-05-15)
//! the architect-ratified primary fix is to coarsen the spinner cadence
//! 60 fps → 10 fps so a future legitimate use of
//! `frame::loading_with_spinner` (data actually loading) does not eat a
//! full-cockpit-repaint budget.
//!
//! ## Strategy — wrap and throttle (A2 sub-candidate, architect-preferred)
//!
//! `iced_aw::Spinner` is MIT-licensed (cargo registry
//! `iced_aw-0.14.1/LICENSE`); the widget body is small (~150 LOC) and
//! we cannot patch the upstream `FRAMES_PER_SECOND` constant without a
//! fork (constraint envelope: no upstream `iced` / `iced_aw` fork — see
//! `trading_ui_library_constraints.md`). We mirror the upstream
//! `Widget` impl in this local module with `FRAMES_PER_SECOND = 10`
//! and `circle_radius = 2.0` to match the existing visual.
//!
//! Attribution: this widget is derived from the MIT-licensed
//! `iced_aw::Spinner` (Iced Audio / iced-aw contributors); see the
//! upstream source linked above. Behavioural drift from upstream is
//! intentional and limited to the FPS constant.
//!
//! ## Determinism (Brief B H-arch-9 carry-through)
//!
//! Upstream Spinner's `state()` uses `Instant::now()` to seed the
//! widget-local `last_update` field. Per the architect's H-arch-9
//! verdict on Brief B
//! ([`iced-aw-cherry-pick/feature.md#h-arch-9`](../../../spec/v1/iced-aw-cherry-pick/feature.md#h-arch-9--iced_awspinner-deterministic-render--resolved-pass-with-caveat)),
//! this is test-unreachable because `iced_test` snapshot paths render
//! at `t = 0.0` (no `RedrawRequested` events are delivered, so
//! `state.t` never advances). We preserve the same shape exactly — no
//! new wall-clock calls are introduced in this module, and
//! `scripts/check_no_clocks_in_ui_tests.sh` continues to PASS because
//! the tests in this file do not reference `Instant::now` /
//! `SystemTime::now`.

use iced::advanced::layout::{Limits, Node};
use iced::advanced::mouse::Cursor;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag};
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::time::{Duration, Instant};
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Vector, window};

/// A throttled spinner — same visual contract as `iced_aw::Spinner`,
/// but re-schedules redraws at 10 fps instead of 60 fps.
///
/// Use via [`crate::widgets::frame::loading_with_spinner`]; direct
/// construction is intentionally allowed for tests that want to assert
/// the cadence constant.
#[allow(missing_debug_implementations)]
pub struct ThrottledSpinner {
    /// The width of the spinner.
    width: Length,
    /// The height of the spinner.
    height: Length,
    /// The rate of one full revolution; matches upstream Spinner default
    /// (1 Hz). The cadence constant `FRAMES_PER_SECOND` is independent
    /// of this and controls only the redraw-request frequency.
    rate: Duration,
    /// The radius of the orbiting circle. Matches upstream
    /// `iced_aw::Spinner` default (2.0) so the rendered glyph is
    /// visually identical.
    circle_radius: f32,
}

impl Default for ThrottledSpinner {
    fn default() -> Self {
        Self {
            width: Length::Fixed(20.0),
            height: Length::Fixed(20.0),
            rate: Duration::from_secs_f32(1.0),
            circle_radius: 2.0,
        }
    }
}

impl ThrottledSpinner {
    /// Target redraw cadence — 10 fps, vs upstream `iced_aw::Spinner`'s
    /// 60 fps. The choice is architect-ratified per
    /// [`feature.md Q3 resolution`](../../../spec/v1/cockpit-performance-and-input-responsiveness/feature.md#q3-resolution--m2-perf-budget-floor-is-fps_p50--30-hardware-uniform-no-coefficient):
    /// 10 fps stays above the operator's 30-fps perf-budget floor
    /// (the cockpit-smoke gate budget) while cutting per-frame
    /// repaint cost ~6×.
    pub const FRAMES_PER_SECOND: u64 = 10;

    /// Creates a new throttled spinner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the width of the spinner.
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the spinner.
    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the circle radius of the orbiting circle.
    #[must_use]
    pub fn circle_radius(mut self, radius: f32) -> Self {
        self.circle_radius = radius;
        self
    }
}

/// Internal widget state — animation phase + last redraw timestamp.
/// Shape preserved verbatim from upstream `iced_aw::Spinner` so the
/// H-arch-9 caveat about `Instant::now()` reachability stays accurate.
struct SpinnerState {
    last_update: Instant,
    t: f32,
}

fn is_visible(bounds: &Rectangle) -> bool {
    bounds.width > 0.0 && bounds.height > 0.0
}

fn fill_circle(
    renderer: &mut impl renderer::Renderer,
    position: Vector,
    radius: f32,
    color: Color,
) {
    if radius > 0.0 {
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: position.x,
                    y: position.y,
                    width: radius * 2.0,
                    height: radius * 2.0,
                },
                border: Border {
                    radius: radius.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            },
            color,
        );
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ThrottledSpinner
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        Node::new(limits.width(self.width).height(self.height).resolve(
            self.width,
            self.height,
            Size::new(f32::INFINITY, f32::INFINITY),
        ))
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !is_visible(&bounds) {
            return;
        }

        let size = if bounds.width < bounds.height {
            bounds.width
        } else {
            bounds.height
        } / 2.0;
        let state = state.state.downcast_ref::<SpinnerState>();
        let center = bounds.center();
        let distance_from_center = size - self.circle_radius;
        let (y, x) = (state.t * std::f32::consts::PI * 2.0).sin_cos();
        let position = Vector::new(
            center.x + x * distance_from_center - self.circle_radius,
            center.y + y * distance_from_center - self.circle_radius,
        );

        fill_circle(renderer, position, self.circle_radius, style.text_color);
    }

    fn tag(&self) -> Tag {
        Tag::of::<SpinnerState>()
    }

    fn state(&self) -> State {
        // SpinnerState seed mirrors upstream `iced_aw::Spinner::state`
        // exactly so the H-arch-9 "Instant::now is test-unreachable"
        // analysis carries through. iced_test snapshot paths never
        // deliver `RedrawRequested`, so `state.t` and `state.last_update`
        // stay at their seed values throughout snapshot rendering.
        State::new(SpinnerState {
            last_update: Instant::now(),
            t: 0.0,
        })
    }

    fn update(
        &mut self,
        state: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        // Edition 2021 — upstream `iced_aw::Spinner` uses an if-let-chain
        // (edition 2024); we de-sugar into a nested form so the workspace
        // edition stays unbumped.
        let now = match event {
            Event::Window(window::Event::RedrawRequested(now)) if is_visible(&bounds) => now,
            _ => return,
        };

        {
            let state = state.state.downcast_mut::<SpinnerState>();
            let duration = (*now - state.last_update).as_secs_f32();
            let increment = if self.rate == Duration::ZERO {
                0.0
            } else {
                duration * 1.0 / self.rate.as_secs_f32()
            };

            state.t += increment;

            if state.t > 1.0 {
                state.t -= 1.0;
            }

            // M1 Candidate A — coarsen redraw cadence to 10 fps.
            // 1000 / FRAMES_PER_SECOND = 100 ms between redraws.
            // Upstream `iced_aw::Spinner` schedules at 60 fps (~16 ms);
            // dropping to 10 fps cuts the per-second repaint count ~6×
            // and removes the dominant continuous-redraw trigger the M0
            // profile flagged at 45.5 % `Compositor::present` self-time.
            shell.request_redraw_at(window::RedrawRequest::At(
                *now + Duration::from_millis(1000 / Self::FRAMES_PER_SECOND),
            ));
            state.last_update = *now;
        }
    }
}

impl<'a, Message, Theme, Renderer> From<ThrottledSpinner> for Element<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'a,
    Theme: 'a,
    Message: 'a,
{
    fn from(spinner: ThrottledSpinner) -> Self {
        Self::new(spinner)
    }
}

/// Convenience `view` function — returns a 20×20 `ThrottledSpinner` as
/// an `iced::Element`.
///
/// The `_mode` parameter is accepted for API uniformity with other widget
/// `view` helpers (theme adaption is handled via `renderer::Style::text_color`
/// inherited from the ambient iced theme).
#[must_use]
pub fn view<Message: 'static, Theme: 'static, Renderer>(
    _mode: crate::theme::ThemeMode,
) -> iced::Element<'static, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'static,
{
    ThrottledSpinner::new().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M1 Candidate A acceptance — the cadence constant is 10 fps,
    /// not the upstream 60 fps. This is the single load-bearing
    /// difference vs `iced_aw::Spinner` and the only invariant the
    /// orchestrator's post-fix cockpit-smoke run depends on.
    #[test]
    fn frames_per_second_is_ten() {
        assert_eq!(ThrottledSpinner::FRAMES_PER_SECOND, 10);
    }

    /// Defensive regression — if a future refactor accidentally
    /// bumps the cadence back to 60, this test FAILs with a clear
    /// message instead of silently regressing the perf budget.
    #[test]
    fn frames_per_second_is_not_sixty() {
        assert_ne!(
            ThrottledSpinner::FRAMES_PER_SECOND,
            60,
            "ThrottledSpinner regressed to 60 fps — the whole point of \
             this widget is the 10 fps cadence (cockpit-perf M1 Candidate A). \
             See crates/ui/src/widgets/throttled_spinner.rs for context."
        );
    }

    /// Visual-parity smoke — default `circle_radius` matches the
    /// upstream `iced_aw::Spinner` default (2.0). The
    /// `loading_with_spinner` helper used to construct a 16x16 spinner
    /// with the default 2.0 radius; preserving that shape means the
    /// rendered glyph is byte-identical to the legacy `iced_aw::Spinner`
    /// glyph at the `t = 0.0` snapshot baseline.
    #[test]
    fn default_circle_radius_matches_upstream() {
        let s = ThrottledSpinner::new();
        assert!((s.circle_radius - 2.0).abs() < f32::EPSILON);
    }

    /// Builder smoke — `width` / `height` / `circle_radius` builders
    /// thread the values through. Stays in sync with the upstream API
    /// surface so the `frame::loading_with_spinner` migration is a
    /// straight find-and-replace.
    #[test]
    fn builders_thread_values() {
        let s = ThrottledSpinner::new()
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .circle_radius(3.0);
        assert_eq!(s.width, Length::Fixed(16.0));
        assert_eq!(s.height, Length::Fixed(16.0));
        assert!((s.circle_radius - 3.0).abs() < f32::EPSILON);
    }

    /// Default size matches upstream `iced_aw::Spinner::default`
    /// (20×20). Callers that want the legacy 16×16 cockpit size
    /// thread it through the `width`/`height` builders explicitly,
    /// same as before.
    #[test]
    fn default_size_matches_upstream() {
        let s = ThrottledSpinner::new();
        assert_eq!(s.width, Length::Fixed(20.0));
        assert_eq!(s.height, Length::Fixed(20.0));
    }
}
