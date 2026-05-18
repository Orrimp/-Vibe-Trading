//! T2030 — Chart tooltip **hover-event-detection** integration test
//! (chart-buy-sell-emphasis v1.9 M6 follow-up).
//!
//! This is the test the previous-pass tooltip work needed but never wrote.
//! The shipped `crates/ui/tests/chart_tooltip_integration.rs` exercises
//! **render-given-hover-state** — i.e. `ui::state::update` correctly
//! synthesises a `ChartTooltipView` when handed a `Message::ChartMarkerHovered`.
//! It does NOT exercise the path that actually generates that message in the
//! running cockpit: the chart canvas's `Program::update` impl translating a
//! `mouse::Event::CursorMoved` into the hover message.
//!
//! Operator feedback 2026-05-11 (commit `ff96ce4`): hovering chart triangles
//! produced no tooltip despite T2018–T2020 having shipped `[x]`. The gap was
//! exactly the missing layer — the canvas pointer-event plumbing — that this
//! test now pins.
//!
//! ## What this test exercises
//!
//! 1. Construct a `ChartProgram` with a known bar series + one fill marker.
//! 2. Inject a synthetic `canvas::Event::Mouse(mouse::Event::CursorMoved {
//!    position: <absolute_screen_position_at_marker_centroid> })`.
//! 3. Assert `Program::update` returns `(Some(Message::ChartMarkerHovered(Fill(0))),
//!    Status::Captured)` AND the canvas-state `ChartHoverState` flips its
//!    `hovered_marker_centroid` to the marker's canvas-local position.
//! 4. Inject a second `CursorMoved` away from any marker; assert the program
//!    publishes `Message::ChartMarkerHoverEnded` (or `None` if state was
//!    already cleared) and the state's hover flag clears.
//!
//! All assertions ride on the public test-helper
//! `ui::widgets::chart::dispatch_canvas_event_for_test` so the integration
//! test does not depend on `pub(crate)` types.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iced::widget::canvas;
use iced::{Point, Rectangle, event::Status, mouse};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{
    Bar, FeeTier, FillView, Money, Price, Quantity, Side, SignalView, StrategyId, Symbol,
    Timeframe, Timestamp, Venue,
};

use ui::state::{ChartMarkerIndex, Message};
use ui::widgets::chart::{ChartHoverState, dispatch_canvas_event_for_test};

// ── Fixture builders ───────────────────────────────────────────────────────

fn fixed_ts(offset_secs: i64) -> Timestamp {
    let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000 + offset_secs)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    Timestamp::new(dt)
}

fn make_bar(offset_secs: i64, close: Decimal) -> Bar {
    let p = |d: Decimal| Price::new(d).unwrap();
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneMinute,
        open_ts: fixed_ts(offset_secs),
        close_ts: fixed_ts(offset_secs),
        open: p(close - dec!(50)),
        high: p(close + dec!(80)),
        low: p(close - dec!(80)),
        close: p(close),
        volume: Quantity::new(dec!(12.5)).unwrap(),
        trade_count: 100,
        local_recv_ts: fixed_ts(offset_secs),
        venue: Venue::Binance,
    }
}

fn make_fill(offset_secs: i64, side: Side, price: Decimal) -> FillView {
    FillView {
        symbol: Symbol::new("BTCUSDT"),
        side,
        price: Price::new(price).unwrap(),
        qty: Quantity::new(dec!(0.1)).unwrap(),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: fixed_ts(offset_secs),
        transaction_id: SmolStr::new(format!("tx-{offset_secs}")),
    }
}

fn make_signal(offset_secs: i64, side: Side) -> SignalView {
    SignalView {
        signal_id: SmolStr::new(format!("sig-{offset_secs}")),
        symbol: Symbol::new("BTCUSDT"),
        side,
        intended_qty: Quantity::new(dec!(0.05)).unwrap(),
        signal_ts: fixed_ts(offset_secs),
        strategy_id: StrategyId::new("sma_crossover"),
        was_clamped: false,
        clamp_reason: None,
    }
}

// Canvas bounds for every test — placed at (100, 50) on the screen so we
// can verify the `position_in(bounds)` math (it should subtract bounds.x +
// bounds.y from absolute screen coords).
const BOUNDS: Rectangle = Rectangle {
    x: 100.0,
    y: 50.0,
    width: 800.0,
    height: 600.0,
};

/// `theme::space::S` — the base decorative gutter applied on every
/// side by `widgets::canvas_chart::inner_rect_with_gutters`.
const GUTTER_PX: f32 = 8.0;

/// chart-canvas-overhaul v1.10.0 (T3010 / R4.1) — additional left
/// gutter consumed by the price-axis labels.  The chart's drawable
/// inner rect now starts at `gutter + AXIS_GUTTER_PRICE_PX` rather
/// than just `gutter`.  Mirrored here so the expected-marker-x math
/// stays explicit.
const AXIS_GUTTER_PRICE_PX: f32 = 48.0;

/// Right margin consumed by the brief's R4 layout.  Doesn't move the
/// marker's leftmost x, but does reduce `inner.width`.  Documented
/// here for symmetry with the production tokens; not currently used
/// because the test fixture's only marker sits at the LEFTMOST bar
/// (x_frac = 0), where the right margin doesn't enter the math.
#[allow(dead_code)]
const AXIS_GUTTER_RIGHT_PX: f32 = 16.0;

/// Bottom gutter consumed by the time-axis labels.  Reduces
/// `inner.height` (so the y-coord math against `inner` shrinks).
const AXIS_GUTTER_TIME_PX: f32 = 24.0;

/// `widgets::canvas_chart::RANGE_PAD_FRACTION` — the 5 % pad applied to
/// the price range so the line+markers don't graze the gutter. Mirrored
/// here so the expected-y math stays self-contained.
const RANGE_PAD_FRACTION: f32 = 0.05;

/// Build the canonical 3-bar series spanning 0s / 60s / 120s.  The single
/// fill (at 0s) lands at the **left edge** of the chart's inner rect:
/// `x = inner.x = gutter (8)` in canvas-local space.
///
/// Returns `(bars, fills, expected_marker_local)` — the third tuple
/// element is the marker's expected canvas-local `(x, y)` position so
/// the test can place the cursor accurately on the marker centroid.
fn three_bar_series_with_left_fill() -> (Vec<Bar>, Vec<FillView>, Point) {
    let bars = vec![
        make_bar(0, dec!(40_000)),
        make_bar(60, dec!(40_100)),
        make_bar(120, dec!(40_050)),
    ];
    let fills = vec![make_fill(0, Side::Buy, dec!(40_000))];
    let marker_local = expected_marker_local(&bars, dec!(40_000));
    (bars, fills, marker_local)
}

/// Mirror of the production `anchor_for_ts` math for the *first* bar /
/// *first* fill — used by the test to position the synthetic cursor on
/// the actual marker centroid rather than guessing.
///
/// **Updated for chart-canvas-overhaul v1.10.0 (T3010):** the
/// production `chart_inner_rect` now applies a four-sided gutter
/// (left price-axis = 48, right margin = 16, top = 0, bottom
/// time-axis = 24) on top of the base 8-px decorative gutter on
/// each side.  The marker's canvas-local x at the first bar
/// (`x_frac = 0`) is therefore `gutter + AXIS_GUTTER_PRICE_PX`,
/// and `inner.height` shrinks by `AXIS_GUTTER_TIME_PX` at the
/// bottom.
///
/// For the fixture above (3 bars, fill at the leftmost ts):
/// - `x = gutter + AXIS_GUTTER_PRICE_PX = 8 + 48 = 56`.
/// - `y` derives from `y_for_price(close, range_with_pad)` against
///   the inner rectangle of `(BOUNDS.height − 2·gutter −
///   AXIS_GUTTER_TIME_PX)`.
fn expected_marker_local(bars: &[Bar], fill_price: Decimal) -> Point {
    let lows: Vec<f32> = bars
        .iter()
        .map(|b| b.low.get().to_string().parse::<f32>().unwrap_or(0.0))
        .collect();
    let highs: Vec<f32> = bars
        .iter()
        .map(|b| b.high.get().to_string().parse::<f32>().unwrap_or(0.0))
        .collect();
    let min_low = lows.iter().copied().fold(f32::INFINITY, f32::min);
    let max_high = highs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (max_high - min_low).max(1.0);
    let pad = span * RANGE_PAD_FRACTION;
    let (range_lo, range_hi) = (min_low - pad, max_high + pad);

    // chart-canvas-overhaul v1.10.0 — inner rect = base gutter + R4
    // axis gutters.  Top stays 0 (legend is inside `inner`).
    let inner_y = GUTTER_PX; // top gutter = 0
    let inner_h = BOUNDS.height - 2.0 * GUTTER_PX - AXIS_GUTTER_TIME_PX;
    let inner_x = GUTTER_PX + AXIS_GUTTER_PRICE_PX;
    let price: f32 = fill_price.to_string().parse().unwrap_or(0.0);
    let frac = (price - range_lo) / (range_hi - range_lo);
    let y = inner_y + (1.0 - frac) * inner_h;
    let x = inner_x; // ts == bars[0].close_ts → x_frac = 0 → x = inner.x
    Point::new(x, y)
}

/// Build the absolute-screen cursor `Point` for a canvas-local marker
/// centroid by adding `BOUNDS.x` + `BOUNDS.y`. This is the inverse of
/// `Cursor::position_in(bounds)`.
fn screen_point_for_marker(local: Point) -> Point {
    Point::new(BOUNDS.x + local.x, BOUNDS.y + local.y)
}

// ── Tests ──────────────────────────────────────────────────────────────────

/// T2030 — `CursorMoved` directly over the first fill marker's centroid
/// publishes `Message::ChartMarkerHovered(Fill(0))` with
/// `Status::Captured` AND records the marker centroid in the
/// `ChartHoverState`.
///
/// This is the load-bearing assertion: it proves the canvas
/// `Program::update` impl actually interprets a synthetic mouse-move at
/// the marker's screen position as a hover. The previous-pass test
/// suite never made this assertion.
#[test]
fn cursor_moved_over_marker_publishes_hover_message() {
    let (bars, fills, marker_local) = three_bar_series_with_left_fill();
    let mut state = ChartHoverState::default();

    // Cursor at the marker's absolute screen position. The marker is
    // pinned at canvas-local `(gutter, y_for_price(40_000))`; we
    // convert to absolute-screen via `screen_point_for_marker` which
    // adds `BOUNDS.x + BOUNDS.y`. The 28-px hit rect gives us slack
    // even if the production `anchor_for_ts` math drifts by a few
    // pixels.
    let position = screen_point_for_marker(marker_local);
    let event = canvas::Event::Mouse(mouse::Event::CursorMoved { position });

    let (msg, status) =
        dispatch_canvas_event_for_test(bars, fills, vec![], &mut state, event, BOUNDS, position);

    // The published message MUST be a Hovered(Fill(0)) AND the event
    // MUST be captured (so the chart owns the pointer interaction —
    // prevents event-bubbling to siblings under the chart).
    assert!(
        matches!(
            msg,
            Some(Message::ChartMarkerHovered(ChartMarkerIndex::Fill(0)))
        ),
        "CursorMoved over marker 0 must publish ChartMarkerHovered(Fill(0)); \
         got {msg:?}"
    );
    assert_eq!(
        status,
        Status::Captured,
        "hover event must be Captured to suppress bubbling"
    );

    // The canvas state must also have flipped — the draw pass reads
    // `hovered_marker_centroid` to position the tooltip card.
    assert!(
        state.is_hovering(),
        "ChartHoverState must remember the hovered marker"
    );
    let centroid = state
        .hovered_marker_centroid()
        .expect("centroid recorded for the hovered marker");
    // Centroid is canvas-local (bounds-relative). x should be at the
    // gutter (~8 px); y is in the chart drawable area (> 0, < height).
    assert!(
        centroid.x.is_finite() && centroid.x >= 0.0 && centroid.x <= BOUNDS.width,
        "centroid x in canvas-local bounds: {}",
        centroid.x
    );
    assert!(
        centroid.y.is_finite() && centroid.y >= 0.0 && centroid.y <= BOUNDS.height,
        "centroid y in canvas-local bounds: {}",
        centroid.y
    );
}

/// T2030 — `CursorMoved` to a position OUTSIDE any marker's hit-rect
/// must NOT publish a Hovered message. (It may publish
/// `ChartMarkerHoverEnded` if the state had been hovering, but on a
/// fresh state with no prior hover it returns `None` to suppress
/// spurious churn.)
#[test]
fn cursor_moved_off_marker_does_not_publish_hover() {
    let (bars, fills, _marker_local) = three_bar_series_with_left_fill();
    let mut state = ChartHoverState::default();

    // Cursor far from the only marker — middle of the chart, where
    // no fill sits. (Marker is at left edge; middle of canvas is
    // bounds.x + 400 = 500 in absolute screen.)
    let position = Point::new(500.0, 350.0);
    let event = canvas::Event::Mouse(mouse::Event::CursorMoved { position });

    let (msg, _status) =
        dispatch_canvas_event_for_test(bars, fills, vec![], &mut state, event, BOUNDS, position);

    assert!(
        msg.is_none() || matches!(msg, Some(Message::ChartMarkerHoverEnded)),
        "cursor off all markers must NOT publish Hovered; got {msg:?}"
    );
    assert!(
        !state.is_hovering(),
        "ChartHoverState must not record a hovered marker after off-marker move"
    );
}

/// T2030 — Hover-then-leave sequence: cursor enters a marker (publishes
/// Hovered), then leaves the marker (publishes HoverEnded). The state
/// transitions match.
#[test]
fn cursor_moved_then_leaving_publishes_hover_ended() {
    let (bars, fills, marker_local) = three_bar_series_with_left_fill();
    let mut state = ChartHoverState::default();

    // Step 1: cursor lands on the marker.
    let on_marker = screen_point_for_marker(marker_local);
    let enter = canvas::Event::Mouse(mouse::Event::CursorMoved {
        position: on_marker,
    });
    let (msg, _status) = dispatch_canvas_event_for_test(
        bars.clone(),
        fills.clone(),
        vec![],
        &mut state,
        enter,
        BOUNDS,
        on_marker,
    );
    assert!(
        matches!(
            msg,
            Some(Message::ChartMarkerHovered(ChartMarkerIndex::Fill(0)))
        ),
        "enter publishes Hovered; got {msg:?}"
    );
    assert!(state.is_hovering(), "state remembers hover");

    // Step 2: cursor moves off the marker (still on canvas, just not
    // on any hit-rect).
    let off_marker = Point::new(500.0, 350.0);
    let leave = canvas::Event::Mouse(mouse::Event::CursorMoved {
        position: off_marker,
    });
    let (msg, _status) =
        dispatch_canvas_event_for_test(bars, fills, vec![], &mut state, leave, BOUNDS, off_marker);

    assert!(
        matches!(msg, Some(Message::ChartMarkerHoverEnded)),
        "leave publishes HoverEnded; got {msg:?}"
    );
    assert!(
        !state.is_hovering(),
        "state cleared after leaving the marker"
    );
}

/// T2030 — Hover over a ghost-signal marker publishes
/// `ChartMarkerHovered(Signal(0))` — proves the ghost layer's hit-rects
/// also route through `Program::update`. The previous-pass tooltip work
/// shipped the ghost variant but never proved the hover path fired for
/// signals either.
#[test]
fn cursor_moved_over_ghost_marker_publishes_signal_hover() {
    let bars = vec![
        make_bar(0, dec!(40_000)),
        make_bar(60, dec!(40_100)),
        make_bar(120, dec!(40_050)),
    ];
    // No fills, one ghost signal at the leftmost ts.
    let signals = vec![make_signal(0, Side::Sell)];
    let mut state = ChartHoverState::default();

    let marker_local = expected_marker_local(&bars, dec!(40_000));
    let position = screen_point_for_marker(marker_local);
    let event = canvas::Event::Mouse(mouse::Event::CursorMoved { position });

    let (msg, status) =
        dispatch_canvas_event_for_test(bars, vec![], signals, &mut state, event, BOUNDS, position);

    assert!(
        matches!(
            msg,
            Some(Message::ChartMarkerHovered(ChartMarkerIndex::Signal(0)))
        ),
        "hover over ghost publishes ChartMarkerHovered(Signal(0)); got {msg:?}"
    );
    assert_eq!(status, Status::Captured);
}

/// T2030 — The cursor leaving the canvas entirely while hovering a
/// marker publishes `ChartMarkerHoverEnded` so the tooltip clears.
///
/// Operator's 2026-05-11 report: the tooltip would "stick" — once a
/// hover fired, moving the cursor off the chart left the tooltip
/// painted on whatever marker the cursor last touched. The pre-T2030
/// implementation `?`-bailed at `cursor.position_in(bounds)?` for
/// every `CursorMoved`, so the off-canvas exit never reached the hit-
/// test branch that would have published `HoverEnded`. T2030 reworks
/// `update` to publish `HoverEnded` even when `position_in` returns
/// `None` provided the state had a prior hover.
#[test]
fn cursor_leaving_canvas_while_hovering_publishes_hover_ended() {
    let (bars, fills, marker_local) = three_bar_series_with_left_fill();
    let mut state = ChartHoverState::default();

    // Step 1: cursor lands on the marker — state flips to hover.
    let on_marker = screen_point_for_marker(marker_local);
    let enter = canvas::Event::Mouse(mouse::Event::CursorMoved {
        position: on_marker,
    });
    let (msg, _) = dispatch_canvas_event_for_test(
        bars.clone(),
        fills.clone(),
        vec![],
        &mut state,
        enter,
        BOUNDS,
        on_marker,
    );
    assert!(
        matches!(msg, Some(Message::ChartMarkerHovered(_))),
        "enter publishes Hovered"
    );
    assert!(state.is_hovering(), "state remembers hover");

    // Step 2: cursor moves to a position OUTSIDE the canvas bounds
    // entirely. The Cursor delivered to `update` is still
    // `Available(point)`, but `position_in(bounds)` will return None
    // because `point` is outside `bounds`. Pre-T2030 the `?` here
    // swallowed the event; T2030's rework publishes `HoverEnded`
    // so the tooltip clears.
    let off_canvas = Point::new(BOUNDS.x + BOUNDS.width + 50.0, BOUNDS.y - 30.0);
    let leave = canvas::Event::Mouse(mouse::Event::CursorMoved {
        position: off_canvas,
    });
    let (msg, _status) =
        dispatch_canvas_event_for_test(bars, fills, vec![], &mut state, leave, BOUNDS, off_canvas);
    assert!(
        matches!(msg, Some(Message::ChartMarkerHoverEnded)),
        "cursor leaving canvas while hovering must publish HoverEnded (T2030 \
         operator-reported bug); got {msg:?}"
    );
    assert!(
        !state.is_hovering(),
        "state must clear when cursor leaves canvas"
    );
}

/// T2030 — Idempotence: the same `CursorMoved` event fired twice in a
/// row (still inside the same marker's hit-rect) publishes the hover
/// message exactly **once** — the second dispatch returns `None`
/// because the state already records that hover. Prevents redundant
/// `update` cycles in the running cockpit.
#[test]
fn cursor_moved_repeated_over_same_marker_publishes_once() {
    let (bars, fills, marker_local) = three_bar_series_with_left_fill();
    let mut state = ChartHoverState::default();
    let position = screen_point_for_marker(marker_local);
    let event = canvas::Event::Mouse(mouse::Event::CursorMoved { position });

    let (msg1, _status1) = dispatch_canvas_event_for_test(
        bars.clone(),
        fills.clone(),
        vec![],
        &mut state,
        event.clone(),
        BOUNDS,
        position,
    );
    assert!(msg1.is_some(), "first dispatch publishes a Hovered message");

    let (msg2, _status2) =
        dispatch_canvas_event_for_test(bars, fills, vec![], &mut state, event, BOUNDS, position);
    assert!(
        msg2.is_none(),
        "second dispatch (same marker) must NOT republish; got {msg2:?}"
    );
}
