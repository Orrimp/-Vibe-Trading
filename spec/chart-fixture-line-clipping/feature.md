---
slug: chart-fixture-line-clipping
status: proposed
owner: pending-analyst
updated: 2026-05-20
version: 0.0.1
predecessor: chart-canvas-overhaul v1.10.0
---

# Chart fixture line clipping (`chart-fixture-line-clipping`)

## Symptom

When the cockpit boots under `--features fixtures` (the only mode an
operator can launch without a live runtime), the chart on the **Lab**
screen renders an incomplete price line:

- **First (leftmost) data point is at the middle of the canvas**, not
  at the left axis-gutter edge.
- Only ~22 line segments are visible (covering ~12:37 → 12:59 on the
  x-axis), even though the x-axis labels span the full
  **12:00 → 12:59** range.
- The chart "ends somewhere" on the right — the line terminates
  before the right axis.

## Evidence

This is reproducible in the committed visual baselines:

- `crates/ui/tests/visual-baselines/charts_screen_dark_typical.png`
- `crates/ui/tests/visual-baselines/charts_screen_dark_floor.png`
- `crates/ui/tests/visual-baselines/charts_screen_dark_operator.png`

All three were refreshed at commit `8be4c3d` (cockpit-training-control
v0.2.0 ship) as "legitimate Lab Train sub-panel composition drift." The
visual-diff refresh accepted the composition shift but the underlying
**chart-line-clipping bug was already in the pre-refresh baselines** —
it predates Phase B and v1.11. Operator surfaced it 2026-05-20 during
the post-v1.11 live cockpit run.

## Root cause (orchestrator diagnostic 2026-05-20)

**Bug is in iced canvas rendering, NOT in chart data.** Probes
exhaustively confirmed:

1. `bars.len() == 60` at the chart's `draw()` call — all 60 bars
   reach the canvas.
2. `inner = (56, 8, 1628, 576)` in canvas-local coordinates — the
   inner rect is correctly sized to span the full canvas minus
   gutters.
3. The path-construction loop at `crates/ui/src/widgets/chart.rs:443-454`
   computes `(x, y)` for every bar correctly — idx=0 → (56, 70),
   idx=30 → (884, 193), idx=59 → (1684, 551). Probe-dumped each
   point.
4. The canvas's reported `bounds = (196, 269, 1708, 616)` in screen
   coordinates is correct.
5. Frame-local → screen mapping is correct (a probe dot at frame
   center (854, 308) appears at screen (1050, 577) = canvas origin
   + frame center).

**What's broken:** corner-dot probes drawn at frame-local (0,0),
(w,0), (0,h), (w,h) — only the bottom-right corner dot renders.
Center dot renders. Top-left, top-right, bottom-left dots are
invisible. A red rectangle outline drawn around the full inner rect
shows only its bottom-right ~quarter.

The canvas frame is composing/clipping to roughly **(390, 540) to
(1880, 855)** in screen coords — the bottom-right quarter of the
canvas widget's reported bounds. Everything outside that region is
not blitted to the surface, even though `frame.fill()` /
`frame.stroke()` are called with valid coords.

## Likely failure modes (architect to confirm)

- **F-CHART-1 — iced 0.14 canvas/tiny-skia layout/render race.** The
  `Container::new(canvas).width(Length::Fill).height(Length::Fill)`
  wrap may be reporting wrong bounds to tiny-skia's compositor on
  the first paint, causing it to scissor to a stale (smaller) rect.
- **F-CHART-2 — `Frame::new(renderer, bounds.size())` viewport bug.**
  The frame might be created with a smaller viewport than declared
  and only blits the bottom-right portion that fits the smaller
  viewport.
- **F-CHART-3 — column/row layout interaction.** The Lab screen's
  `Column` may give the chart `chart_body` a `Length::Fill` that
  resolves to a different layout box than what iced's renderer
  uses for the canvas scissor.

## Failed fix attempts (orchestrator 2026-05-20)

Two minimal-touch attempts to repair the clip were tried and reverted:

1. **Remove the outer `Container::new(chart_body).width(Length::Fill).height(Length::Fill)`
   wrap in `crates/ui/src/screens/lab.rs:338-341`** (the "belt-and-braces"
   defensive wrap). **Result: NO CHANGE.** Bug persists with identical
   clip pattern. Falsifies the "outer Container is the culprit" branch
   of F-CHART-3.
2. **Replace `Canvas::height(Length::Fill)` with
   `Canvas::height(Length::Fixed(400.0))` (and the inner Container
   match)** in `crates/ui/src/widgets/chart.rs:235-240`. **Result: bug
   PERSISTS but visible area shrinks proportionally** — with a 400 px
   canvas, only the rightmost ~100 px shows the line; with a 616 px
   canvas, ~590 px shows. The clip is NOT Length::Fill-specific;
   replacing with a fixed height makes the clip worse, not better.

These narrow the fix scope: the bug is NOT a simple Length::Fill
nesting issue in either `chart::view` or `lab::view`. It is somewhere
deeper — likely in iced 0.14's `Canvas::draw` invocation, `Frame::new`,
or tiny-skia's compositor scissor / clip-rect handling. An architect
pass with access to iced 0.14 source is the right next step.

## Investigation evidence on disk

Probes were applied to `crates/ui/src/widgets/chart.rs` then
reverted at the same commit. Re-running the probes after the
revert by applying:

```rust
// At the top of `ChartProgram::draw`:
eprintln!("CHART-DRAW bounds={:?}", bounds);
// In the line-stroke pass, replace the `Path::new(...)` block with:
let outline = Path::new(|b| {
    b.move_to(Point::new(inner.x, inner.y));
    b.line_to(Point::new(inner.x + inner.width, inner.y));
    b.line_to(Point::new(inner.x + inner.width, inner.y + inner.height));
    b.line_to(Point::new(inner.x, inner.y + inner.height));
    b.line_to(Point::new(inner.x, inner.y));
});
frame.stroke(&outline, Stroke::default()
    .with_color(iced::Color::from_rgb(1.0, 0.0, 0.0)).with_width(3.0));
let frame_w = bounds.width;
let frame_h = bounds.height;
for (px, py, c) in [
    (0.0_f32, 0.0, (1.0_f32, 1.0, 0.0)),
    (frame_w, 0.0, (1.0, 0.5, 0.0)),
    (0.0, frame_h, (0.0, 1.0, 1.0)),
    (frame_w, frame_h, (1.0, 0.0, 1.0)),
    (frame_w / 2.0, frame_h / 2.0, (0.0, 1.0, 0.0)),
] {
    let dot = Path::circle(Point::new(px, py), 20.0);
    frame.fill(&dot, iced::Color::from_rgb(c.0, c.1, c.2));
}
```

Then `cargo test -p ui --test visual_snapshots charts_screen_dark_typical
-- --nocapture` and inspect `target/visual-diff/charts_screen_dark_typical-actual.png`.

## Scope when promoted

- **M0 (analyst-runnable):** inspect `crates/ui/src/widgets/chart.rs`
  draw pass (look at `ChartProgram::draw`), `lab.rs:241-244` chart-binding,
  and `state.rs:1305` `BarReceived` arm. Identify which hypothesis
  matches.
- **M1:** fix the dominant cause; ensure all 60 bars render across
  the full inner-rect width.
- **M2:** add a regression test against the live chart that asserts
  the price-line's leftmost rendered x ≈ `inner.x` (within tolerance).

## Out of scope

- Live (non-fixtures) chart rendering — `cockpit_live` may have a
  different code path.
- Equity overlay rendering — affects only the right-side overlay,
  not the price-line itself.
- Markers / signals — those are scatter points, not the price line.

## Non-regression contract

- 22 body-SHA-256 anchors stay byte-identical.
- After fix: visual_snapshots baselines REFRESH (the current
  baselines are buggy artifacts).
- `cockpit-smoke` stays green.

## Trace

Trace row pending; opens proposed when analyst spawn fires.

## Changelog

- 2026-05-20 (orchestrator, bug filed): operator surfaced during
  live cockpit run post-v1.11. Bug is pre-existing — committed
  visual baselines reproduce. Promoted to candidate; analyst spawn
  when operator triggers.
