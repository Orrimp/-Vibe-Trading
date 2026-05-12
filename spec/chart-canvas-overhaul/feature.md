---
slug: chart-canvas-overhaul
status: shipped
owner: shipped
updated: 2026-05-12
version: 1.10.0
predecessor: chart-buy-sell-emphasis (1.9.0)
---

# Chart canvas overhaul

## Why

This brief is opened in response to a **second operator visual-
verification pass** against the v1.9.0 ship of
[`chart-buy-sell-emphasis`](../chart-buy-sell-emphasis/feature.md).
The operator ran the cockpit on a **3360×1890 native Retina** desktop
display (the daily-driver hardware, not a test rig) and reported,
verbatim:

> "I belive the error with the chart is not fix.
> - I still dont see tooltip overlay
> - The charts are croped and the lines are only partial visible on smaller screen. The UI does not scale
> - The charts needs to scale like svg with the size of the window.
> - No legend.
> - Not centered
> - No price or time axis."

Six items. Three are **regressions vs. v1.9.0** — the M6.2 hardening
pass at
[`spec/chart-buy-sell-emphasis/reports/m6.2-hardening-2026-05-11.md`](../chart-buy-sell-emphasis/reports/m6.2-hardening-2026-05-11.md)
claimed PASS on tooltip-flash-and-disappear (T2033) and
chart-cropping (T2032), and the final tester report at
[`spec/chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md`](../chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md)
recorded `VERDICT → PASS` — but on the operator's actual hardware
both bugs are still visible. The other three are **scope additions**
the v1.9.0 brief never covered (Q5 layout (β) explicitly omitted a
legend; Q-list never mentioned axes; "centered" is a new term).

The orchestrator confirmed with the operator on 2026-05-12: **one
feature, all six items**, going through the full
analyst → architect → (developer ‖ ui-designer) → tester → presenter
pipeline (not a hotfix-split, not a free-form patch).

### Why the v1.9.0 cycle missed this

Three precondition failures, all upstream of the developer pass and
ratified by the tester:

1. **Tester ran headless / screenshot pipeline at the floor
   resolution.** The M6.2-hardening screenshots
   (`m6.2-hardening-2026-05-11.md` §5) sample at 1280×720 / 1600×900
   / 1920×1080 — none above the floor and none at native 3360×1890.
   The 2-of-4-markers anomaly in `cockpit-T2032-1920x1080.png` was
   surfaced as an "Open Observation for Tester Pickup" but the tester
   accepted the PASS verdict anyway because the VERDICT contract is
   built on `cargo test` + `verify_anchors.sh`, neither of which
   exercises the visual surface the operator perceived.
2. **Cockpit binary opens at min-size every launch.** The shared
   helper at
   [`crates/ui/src/window_icon.rs:111-118`](../../crates/ui/src/window_icon.rs)
   sets `size = Size::new(MIN_WINDOW_WIDTH_PX, MIN_WINDOW_HEIGHT_PX)
   = (1280.0, 720.0)`. On a 3360×1890 native Retina that's a tiny
   window in the corner of the screen until the operator manually
   resizes. Every iced re-layout-on-resize gotcha that exists in 0.14
   is exercised by that manual resize.
3. **The unit tests for T2032 (`chart_canvas_height_grows_with_body_height`)
   and T2033 (`chart_tooltip_view_built_from_canvas_state_without_round_trip`)
   are pure-arithmetic helpers** — they assert budgets and
   round-trip-removal, not the actual iced widget tree behaviour.
   The operator's perceived chart shape lives downstream of those
   helpers, in the iced runtime's `Length::Fill` propagation and the
   canvas `Program::update` event-pump.

This brief explicitly closes those preconditions in addition to the
six R-items: **see R6 (visual-verification gate)**.

### Glossary (one-line, used throughout)

- **Inner rect** — the canvas rectangle inside the `space::S` (8 px)
  gutter where the price line + markers are drawn; computed by
  [`canvas_chart::inner_rect`](../../crates/ui/src/widgets/canvas_chart.rs).
- **Axis gutter** — the *new* outer band added by this feature to
  host price labels (left) and time labels (bottom) outside the plot
  area. Distinct from `inner_rect`'s 8-px decorative gutter.
- **Legend** — small inset card labelling each marker layer + the
  price line so the operator doesn't need to remember v1.9.0 marker
  semantics.
- **Native Retina** — 3360×1890 logical pixels on the operator's
  display; iced 0.14 reports this via `Window::Size` after applying
  the platform's HiDPI scaling factor.
- **Canvas cache invalidation** — iced's `canvas::Canvas` widget
  caches the result of `Program::draw` between repaints; the cache
  invalidates on bounds change. Suspect path for R1.3 (SVG-like
  scaling).
- **Body-SHA-256 anchor** — the 11 locked regression hashes at
  [`spec/anchors.toml`](../anchors.toml). All 11 stay green under
  this brief (R7 negative invariant).

**Version proposal: `1.10.0`** — the natural next step on the v1.x
cockpit line (`chart-buy-sell-emphasis` shipped at `1.9.0`; nothing
is in flight at `1.9.x`). Minor-version bump because this is a
visual-shape change to a shipped surface, not a patch. The Lumen
phase numbering stays untouched.

**Predecessor: `chart-buy-sell-emphasis` v1.9.0.** Every R-item in
that brief stays in force; this brief *adds and corrects*, it does
not retire anything. See R7 / R8 for the explicit non-regression
contracts.

---

## Requirements

Numbered, testable. Tester contract in **V-items** below. Each R-item
ends with a one-line **Acceptance** clause callable from `cargo test`
or `verify_anchors.sh` or a visual-verification screenshot the
developer must capture at the operator's resolution.

R1–R3 close the **regressions**; R4–R5 add the **new surfaces**
(axes, legend); R6 is the new **visual-verification gate**; R7–R8
pin **non-regression** of the v1.9.0 R-items and anchor set.

### R1 — Tooltip overlay actually renders on hover at 3360×1890

The operator reports the tooltip is **invisible** on their hardware,
even though v1.9.0 T2033 claimed to fix the flash-and-disappear via
decoupling. Three candidate hypotheses (analyst surfaces; architect
chooses or eliminates):

1. **Event pump regression.** Despite the unit test
   `chart_tooltip_hover_fires` (T2030) proving `Program::update`
   publishes `Message::ChartMarkerHovered` on a synthetic
   `CursorMoved` event, the *live* iced runtime at 3360×1890 may
   never deliver `CursorMoved` to the canvas at all — possible
   causes: HiDPI logical-vs-physical coordinate confusion, the
   chart-body `Container::new(...).width(Length::Fill).height(
   Length::Fill)` swallowing pointer events because of a missing
   `id`, or `mouse::Cursor::Available(p)` returning a point whose
   `position_in(bounds)` collapses to `None` because `bounds.size`
   is reported in physical pixels and `cursor.position` in logical.
2. **Hit-rect math regression at scale.** The
   `MARKER_HIT_RECT_PX = 28.0` constant
   ([`chart.rs:55`](../../crates/ui/src/widgets/chart.rs)) is in
   *logical* pixels; on a 3360×1890 Retina display the iced renderer
   may report a different bounds-vs-cursor scaling that misaligns
   the rect by integer factors. The hit-test branch in
   `ChartProgram::update` at
   [`chart.rs:138-177`](../../crates/ui/src/widgets/chart.rs)
   then returns `None` for every cursor position, and the tooltip
   never enters its draw path.
3. **Draw-pass regression.** Pass 6 in
   [`chart.rs:347-353`](../../crates/ui/src/widgets/chart.rs)
   reads `state.hovered_marker_idx + state.hovered_marker_centroid`
   and calls `chart_tooltip::draw_tooltip` — but `draw_tooltip`'s
   `compute_card_rect` at
   [`chart_tooltip.rs`](../../crates/ui/src/widgets/chart_tooltip.rs)
   may emit a rect with `y < 0` or `y > bounds.height` at this
   resolution, painting the card offscreen with no visible warning.

**R1.1** Architect picks the hypothesis (or layers fixes for several)
and pins it in `## Design`. Live diagnosis MUST run on a 3360×1890
window before the design ratifies a cause — architect or
ui-designer captures cursor-and-bounds telemetry via a temporary
`tracing::debug!` instrumentation pass on `Program::update` if the
inspection cannot be done from static reading alone.

**R1.2** The fix MUST land such that hover over any visible
fill-marker or ghost-marker on the operator's 3360×1890 cockpit
window surfaces the tooltip card (R4.2 fields from v1.9.0) within
**one render frame** of the cursor entering the marker's hit-rect,
and the card dismisses within **one render frame** of the cursor
leaving the hit-rect.

**R1.3** The fix MUST survive a manual window-resize from 1280×720
floor to 3360×1890 native (operator's actual workflow) AND from
3360×1890 native to 1280×720 floor (defence-in-depth). Both
directions need a positive hover test in the developer's M-pass
screenshot evidence.

**R1.4** The `chart_tooltip_hover_fires` test (T2030) MUST stay
green; if R1's fix requires reshaping the canvas-event surface the
test exercises, the test gets extended (not deleted) to cover the
new shape.

**Acceptance:** developer screenshot evidence at 3360×1890 native
showing the tooltip card rendered over a hovered fill-marker
(operator's actual hardware capture, per R6); existing `chart_tooltip_hover_fires`
+ `chart_tooltip_integration` tests green; new test
`chart_tooltip_renders_at_retina_resolution` (architect names the
exact test ID) green against a 3360×1890 synthetic bounds rectangle.

### R2 — Chart fills its allocated rectangle on every window size

> **RESOLVED 2026-05-12 (operator-confirmed).** After M2/M3 axes
> landed, the "lines only partial visible on smaller screen"
> symptom is gone from the operator's side. The original
> hypothesis that the canvas was painting at half-scale was
> empirically disproved (see [`## Diagnostic — CORRECTED`](#diagnostic--corrected-2026-05-12-orchestrator-led)
> below) — the apparent shortness was a fixture-shape artifact,
> not a rendering bug. R2 is closed for v1.10.0; no further work
> required. Original analyst text retained below for audit trail.

The operator reports "charts are croped and the lines are only
partial visible on smaller screen." Cropping at 3360×1890 means the
canvas's `inner_rect` math is producing a draw region that doesn't
match what the iced layout system is allocating. Two candidate
hypotheses:

1. **Padding leak.** The Charts screen at
   [`crates/ui/src/screens/charts.rs:188-235`](../../crates/ui/src/screens/charts.rs)
   uses `.padding(space::L as u16)` (16 px) and `.spacing(space::M)`
   (12 px) on the outer `Column`. The chart-body `Container::new(
   chart_body).width(Length::Fill).height(Length::Fill)` then
   receives an allocation reduced by those numbers — but the
   canvas's `Program::draw` reads `bounds.size()` as the
   *container-allocated* size, then applies *another* 8 px gutter
   via `inner_rect`. At small window sizes the gutter consumes a
   meaningful percentage of the inner; at large window sizes it
   doesn't. If the operator sees "lines only partial visible" the
   suspicion is that the price line is drawing past `inner.right`
   or below `inner.bottom`.
2. **Canvas-cache stale.** iced's `canvas::Canvas` caches the
   `Geometry` returned by `Program::draw` between repaints; the
   cache invalidates only on geometry-change signals (bounds
   change, state change). If the resize event fires but the cache
   doesn't invalidate, the chart paints a stale geometry sized for
   the previous bounds — exactly the "lines only partial visible"
   symptom. Suspect path: the M6.2-T2033 fix decoupled tooltip
   draw from `Cockpit.chart_tooltip`, which may have also
   inadvertently broken the cache-invalidation signal that fed off
   `chart_tooltip` state changes.

**R2.1** Architect picks the hypothesis and lands the fix in
`crates/ui/src/widgets/chart.rs` and/or
`crates/ui/src/widgets/canvas_chart.rs`. The fix MUST NOT break
any existing snapshot baseline (R7 anchor invariant); if it does,
architect routes back to analyst for an explicit scope expansion.

**R2.2** The price line + markers MUST render entirely within
the chart canvas's allocated bounds at **every** window size from
1280×720 floor to 3360×1890 native, inclusive. "Partial visible"
is a fail.

**R2.3** Visual verification at three sizes: 1280×720 (floor),
~1920×1080 (mid), 3360×1890 (operator native). Developer captures
all three at native resolution (NOT downscaled, R6).

**Acceptance:** three developer-captured screenshots at the three
sizes (R2.3) showing the chart line fully within the canvas
allocation with no clipping; existing
`chart_canvas_height_grows_with_body_height` test stays green;
new test `chart_inner_rect_stays_within_canvas_bounds` (architect
names) asserts `inner_rect(bounds.size()).right <= bounds.width
&& inner_rect(bounds.size()).bottom <= bounds.height` for a
sweep of bounds sizes from 100×100 to 3360×1890.

### R3 — Chart scales like SVG with window size

The operator's literal request: "The charts needs to scale like svg
with the size of the window." Two readings:

- **(a)** The chart should re-paint at the new size on every window
  resize event (iced re-layout should already do this; the operator
  may be perceiving a stale cache per R2's hypothesis 2).
- **(b)** The chart should *visually upscale proportionally* —
  font sizes, gridline counts, marker sizes, line stroke widths all
  scale with the canvas allocation, so the chart at 3360×1890 reads
  as a "zoomed-in" version of the chart at 1280×720, not as a
  1280×720 chart with extra empty space around it.

**R3.1** Analyst recommends **(a) is the load-bearing fix** (the
operator says "scale like svg" — SVG by default re-paints at the
new bounds without intrinsic re-typesetting; iced canvas should do
the same). Architect confirms or expands to (b).

**R3.2 (assumes R3.1=(a)).** Resize-driven repaint MUST land within
**one frame** of the window-resize-end event. No stale-frame
between resize-end and the next paint. Cache invalidation triggered
on bounds change.

**R3.3 (if architect expands to (b)).** Font sizes, marker sizes,
line stroke widths all parameterise on canvas allocation rather
than on absolute pixel constants. Analyst flags: this is a
significant scope expansion that ripples through R1 (hit-rect
size), R4 (axis label sizes), R5 (legend chip sizes). Operator
clarification needed if architect picks (b) — see **Q1**.

**R3.4** The volume histogram below the chart (R7.2 from v1.9.0)
MUST scale identically — the chart and the histogram share a
horizontal time axis once R4.2 lands, so they must scale as a unit.

**Acceptance:** manual resize from 1280×720 to 3360×1890 on the
operator's cockpit produces a smooth repaint at every interpolation
window-size; no stale frames; both chart and histogram scale together.
New test `chart_repaints_on_bounds_change` (architect names) asserts
canvas cache invalidates when bounds change between calls to
`Program::draw`.

### R4 — Price axis and time axis

v1.9.0 ships price labels *inside* the canvas at `inner.x + 4.0`
([`chart.rs:540`](../../crates/ui/src/widgets/chart.rs)) — five
right-of-line labels that overlap the price-line in busy zones.
There is no time axis at all.

**R4.1 — Price axis (vertical).** Move price labels OUT of the
chart canvas's inner rect into a dedicated **left** or **right**
gutter (architect picks; see **Q2**). Gutter width parameterised
on the longest label's pixel width at `text::MICRO` size + a
`space::S` left/right pad.

- **R4.1.1** Five labels at the same gridline positions v1.9.0
  uses; same `text::MICRO` size; same `color::FG_3` colour. No
  new tokens.
- **R4.1.2** Each label paired with a 4-px tick mark drawn into
  the inner rect at the gridline's `y`.
- **R4.1.3** Optional thin (1-px) vertical axis line in
  `color::BORDER_1 @ alpha 0.4` (already shipped pattern) drawn
  along the gutter's edge — architect confirms (**Q2** sub-q).

**R4.2 — Time axis (horizontal).** New horizontal axis below the
chart canvas, **above** the per-bar volume histogram (the histogram
shares the time axis — single source of truth).

- **R4.2.1** Tick marks at every Nth bar — `N=5` for a 60-bar window
  gives 12 ticks (one every 5 minutes); `N=10` gives 6 ticks (one
  every 10 minutes). Architect picks; analyst recommends `N=10`
  for visual density at 1280×720 floor. See **Q3**.
- **R4.2.2** Label format: **`HH:MM`** (24-hour UTC). RFC3339
  timezone discipline matches the rest of the cockpit (audit
  ledger, tooltips, journal modal). See **Q4** for operator
  preference on local-time vs UTC display.
- **R4.2.3** Multi-day rolling-window-eventually-replaces-1m-bars
  future scope: if the bar window ever spans >24 hours, the format
  switches to `MMM DD` for the date portion. v1.10.0 ships with
  60-minute window from v1.9.0; multi-day is out of scope for
  this brief. Architect documents the future-compat shape in
  Design.

**R4.3 — Axis gutter as Lumen token.** Introduce `theme::layout::
AXIS_GUTTER_PX` (analyst strawman: 48 px for the price gutter, 24
px for the time gutter). Architect picks the exact values per Lumen
typography metrics; new token additions go through the Lumen
design discipline (no inline magic numbers in
`crates/ui/src/widgets/chart.rs`).

**R4.4 — Axis labels are derived state.** No new state on
`Cockpit`. Labels are recomputed on every paint from
`self.bars.first().open_ts` / `self.bars.last().close_ts` and the
`price_range(&self.bars)` helper that already exists at
[`chart.rs:601-618`](../../crates/ui/src/widgets/chart.rs). No
caching; the canvas-cache invalidation logic from R3 covers any
per-frame re-render cost.

**R4.5 — Strings via `ui::strings`.** No inline string literals.
Axes carry no user-facing strings per se (the labels are formatted
numerics and timestamps), but if architect chooses to include unit
suffixes (`USDT` on the price axis, `UTC` on the time axis), they
land as `CHART_AXIS_PRICE_UNIT` / `CHART_AXIS_TIME_UNIT` constants
in `ui::strings`.

**Acceptance:** new snapshot `charts_screen_with_axes` (architect
names) captures the Charts screen with both axes visible at
1280×720; visual-verification screenshot at 3360×1890 (R6) shows
the same axes scale correctly with the chart; existing
`charts_screen_with_counters_and_chart` baseline either updates
(expected churn — analyst flags as a "Type A" baseline-churn item)
or splits into two scenarios depending on architect's call.

### R5 — Legend

v1.9.0 ships markers + line + ghosts without explaining what they
mean — the operator has to remember that "▲ = buy in UP_500" /
"▼ = sell in DOWN_500" / "faded ▲ = buy signal not yet executed".
A legend closes this gap.

**R5.1 — Legend content (5 entries):**

1. **Buy (executed)** — `▲` in `UP_500`, label
   `CHART_LEGEND_BUY_LABEL = "Buy"` (existing string).
2. **Sell (executed)** — `▼` in `DOWN_500`, label
   `CHART_LEGEND_SELL_LABEL = "Sell"`.
3. **Buy signal (not executed)** — `▲` in `UP_400 @ 60% alpha`,
   label `CHART_LEGEND_BUY_GHOST_LABEL = "Buy signal"`.
4. **Sell signal (not executed)** — `▼` in `DOWN_400 @ 60% alpha`,
   label `CHART_LEGEND_SELL_GHOST_LABEL = "Sell signal"`.
5. **Price** — short line stub in `color::ACCENT`, label
   `CHART_LEGEND_PRICE_LABEL = "Price"`.

**R5.2 — Layout.** Three plausible placements; architect picks per
**Q5**.

- **(a)** Top-right inset over the chart canvas (small card, ~140 px
  wide, `space::S` padded, `PANEL_RAISED` background, `BORDER_STRONG`
  1-px outline — same chrome as the tooltip per v1.9.0 Q3 resolution).
  Pro: doesn't reshape the screen; uses dead space. Con: covers
  ~3-5% of the chart's top-right; potentially obscures markers
  near the recent-time / high-price corner.
- **(b)** Below the chip row, above the status strip — new horizontal
  row in the Charts screen Column. Pro: never covers data; clean
  separation. Con: another fixed-height strip eats vertical budget
  the chart already shares with the histogram.
- **(c)** Inline at the right edge of the existing status strip
  (which currently holds the volume tile + position mirror per
  v1.9.0 R7.1/R7.3). Pro: reuses existing strip allocation. Con:
  five legend entries plus three volume-tile cells plus a position
  mirror is crowded; the status strip already pushes hard against
  the 1280-px floor.

**R5.3 — Legend visibility.** Always visible by default. Architect
may add a "hide legend" toggle as a follow-up scope, but v1.10.0
ships it on.

**R5.4 — Legend re-uses marker glyphs.** The legend's triangles
MUST use the same `draw_triangle` helper at
[`chart.rs:556`](../../crates/ui/src/widgets/chart.rs) the chart
uses, sized down to `text::MICRO` height (analyst strawman: 10 px).
Single source of truth for the marker glyph shape.

**Acceptance:** new snapshot `charts_screen_with_legend` (architect
names); legend entries match the rendered marker styles in the
chart canvas; `cargo test -p ui --test consistency` stays green
(no inline hex, no inline strings).

### R6 — Visual-verification gate (process-level)

The v1.9.0 PASS verdict on a 1280×720 tester capture is the
**load-bearing incident** that lets this brief exist. R6 prevents
the next recurrence.

**R6.1** Developer pass MUST capture **at least three** screenshots
at the developer's native macOS Retina resolution during M-pass
completion. The three sizes:

- **1280×720** (the floor — verifies the v1.9.0 M6.2 capture
  baseline still works).
- **~1920×1080** or whatever the developer's first natural resize
  target is.
- **3360×1890 native Retina** — the operator's actual hardware.
  This is NOT a downscaled tester capture; it is the developer's
  own cockpit window resized to the operator's screen size, then
  captured at native pixel resolution via `screencapture -x`.

**R6.2** Each screenshot MUST be referenced from this brief's
`## Implementation` section at developer-tick time, AND from the
developer's tester-handoff message. The tester verifies the
screenshots are at the claimed resolutions before issuing PASS.

**R6.3** The `capture-screenshot` skill at
[`.claude/skills/capture-screenshot/SKILL.md`](../../.claude/skills/capture-screenshot/SKILL.md)
gains an `at_native_retina` operator-instruction branch — see
**Q6** for whether this is in-scope for this feature or a separate
skill-tooling brief.

**R6.4** Tester gate: `T_FINAL_CHART_CANVAS_OVERHAUL` MUST NOT
emit `VERDICT → PASS` without the three screenshots present and
the operator confirmation that the 3360×1890 capture shows the
six R-items resolved.

**Acceptance:** developer ticks every M-pass task with a citation
to the relevant screenshot path; tester report references all
three; orchestrator passes the 3360×1890 screenshot to the operator
before presenter spawn.

### R7 — Non-regression of v1.9.0 R-items

Every R-item from
[`spec/chart-buy-sell-emphasis/feature.md`](../chart-buy-sell-emphasis/feature.md)
stays green:

- **R1–R6 (v1.9.0)** — marker visual treatment unchanged
  (13 px filled triangle, `BORDER_STRONG` outline, `whisper_shadow`,
  ghost layer at 60% alpha in `_400` tier). This brief MUST NOT
  shrink, recolour, or re-shape any marker.
- **R7 (v1.9.0)** — counter views (volume tile + per-bar histogram
  + open-position mirror) stay rendered. R5 legend placement
  (Q5 resolution) must not displace any of these.
- **R8 (v1.9.0)** — Layout β stays. Adding a price-axis gutter
  on the left (or right) reshapes the chart canvas's horizontal
  budget by `AXIS_GUTTER_PX` — architect re-derives the
  `chart_canvas_height_for_body` helper in
  [`crates/ui/src/screens/charts.rs:78-88`](../../crates/ui/src/screens/charts.rs)
  to account for the new gutter. Existing test
  `chart_canvas_height_grows_with_body_height` either updates or
  splits into a width-and-height pair.
- **R9 (v1.9.0)** — determinism, read-only invariants, no new bus
  channels. This brief honours all three: no audit writes, no
  bus changes, no strategy code touched.
- **R10 (v1.9.0)** — consistency tests (no inline hex, no inline
  strings, `Message::*` exhaustiveness). New R-items must conform.
- **R11 (v1.9.0)** — cross-feature invariants. Phase 1
  status-bar / focus-ring stay untouched; Phase 2 chart-buffer
  rolling-60-bar shape unchanged; tape-row-audit-modal modal pattern
  unaffected.
- **R12 (v1.9.0)** — viewer-parity scope. v1.10.0 is **also**
  cockpit-only. The viewer at
  [`crates/ui/src/bin/viewer.rs`](../../crates/ui/src/bin/viewer.rs)
  does NOT inherit axes / legend in v1.10.0. Operator confirms
  via **Q7**.

**R7.1** All 11 anchored reports stay byte-identical (R9.4 v1.9.0
positive invariant carries forward).

**Acceptance:** `bash scripts/verify_anchors.sh` returns
`ANCHORS PASS (11 / 11)`; full `cargo test --workspace` exit code
`0`; specifically the 8 v1.9.0 V-items (V1–V8 there) all green
under the new code.

### R8 — Anchor regression: zero

This feature touches **no** strategy code, **no** risk-engine code,
**no** backtest engine, **no** report rendering. Same negative
invariant as v1.9.0 R9.4.

**Acceptance:** `bash scripts/verify_anchors.sh` outputs
`ANCHORS PASS  (11 / 11)`; zero diffs vs `spec/anchors.toml`.
**Hard gate** at tester time.

### R9 — Legend card visually distinguishable from chart panel (new, 2026-05-12)

Surfaced by the orchestrator's clean-tree screenshot
[`/tmp/orch-diag/cockpit-final-charts.png`](/tmp/orch-diag/cockpit-final-charts.png)
after T3015–T3017 landed. The legend card paints with
`color::PANEL_RAISED` fill and `color::BORDER_1 @ 1 px` outline
([`chart_legend.rs:99-118`](../../crates/ui/src/widgets/chart_legend.rs#L99-L118)).
On dark mode at 3360×1890 the `PANEL_RAISED` value sits one
luminance step above the surrounding chart `PANEL` background, and
`BORDER_1` is the lowest-contrast border tier — the resulting card
is **barely distinguishable** from the chart canvas behind it. The
five legend entries are legible but the card chrome reads as
optional/easy-to-miss, defeating the discoverability rationale
behind R5.

**R9.1** The legend card MUST be visually distinct from the chart
canvas's `PANEL` background at every theme mode (dark + light) and
at every resolution from 1280×720 floor to 3360×1890 native. "Barely
visible" is a fail.

**R9.2** The fix candidate space (UI-designer-decide; flagged as
**Q-revised-2** in Notes):
- **(a)** Swap fill to `color::PANEL_DEEP` (or a new
  `LEGEND_CARD_BG` token if no existing tier has the right
  contrast) — raises the luminance delta against `PANEL`.
- **(b)** Swap outline to `color::BORDER_STRONG @ 1 px` — keeps
  the fill but raises the edge contrast. Matches the v1.9.0
  tooltip card chrome.
- **(c)** Add a `whisper_shadow`-class drop shadow under the
  card — same chrome the chart's fill-markers use. Heavier than
  (a) / (b) but reads as "this thing floats above the chart".
- **(d)** UI-designer-decide combination of the above against
  Lumen design discipline.

**R9.3** Whichever path is chosen MUST keep `cargo test -p ui
--test consistency` green (no inline hex, no inline strings); any
new token lands in `theme.rs` per the Lumen pattern.

**R9.4** The fix MUST be visually verified on the operator's
3360×1890 hardware via R6's screenshot gate.

**Acceptance:** new ui-designer-captured screenshot at 3360×1890
shows the legend card with a clear luminance/edge delta against
the chart panel; the existing `chart_legend` unit tests + the
`chart__btc_with_two_buys_one_sell` snapshot either stay green
(if the change is token-only) or update atomically with the fix
(snapshot churn flagged in the developer's tick).

### R10 — Local time zone on the x-axis (Q4 follow-up, 2026-05-12)

Q4 (operator-locked "local browser/OS time zone") is currently
**partially landed**. The helper `local_offset_or_utc()` at
[`chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs#L125-L160)
returns `UtcOffset::UTC` unconditionally — both under `cfg(test)`
(deterministic snapshot tests) and in production — because the
workspace `time` dependency does NOT enable the `local-offset`
feature, and enabling it requires a workspace-wide `Cargo.toml`
edit which the brief's "Zero changes to non-UI crates"
non-negotiable forbids without operator approval.

The time crate's `local-offset` feature is marked unsafe-on-Linux
(`local_offset_at()` can deadlock under multi-threaded glibc
environment); the cockpit binary is **macOS-only** on the
operator's hardware, so the Linux caveat does not bite this
deployment. The orchestrator surfaces this as a real Q for the
operator before the architect commits to a path.

**R10.1** Operator decides **Q-revised-1** (Notes section): either
**(a)** enable the workspace `time` dep's `local-offset` feature
+ accept the unsafe-on-Linux caveat (macOS-only cockpit), wiring
the helper to read the OS offset in production; OR **(b)** defer
the local-time follow-up to a v1.11 brief and explicitly document
the v1.10.0 ship state as "UTC fallback, local-time deferred".

**R10.2 (assumes R10.1=(a)).** The helper reads OS offset via
`time::UtcOffset::current_local_offset()` in production; `cfg(test)`
override returns `UtcOffset::UTC` so snapshot tests stay
deterministic (the risk-register item 2 mitigation already in
`## Design` covers this).

**R10.3 (assumes R10.1=(a)).** The `Cargo.toml` edit is the
**minimum** required to enable `local-offset` — no other workspace
deps touched. Tester verifies via `verify_anchors.sh` that the 11
anchored bodies stay byte-identical (no strategy / audit / report
path affected by the feature flag flip).

**R10.4 (assumes R10.1=(b)).** The brief's Q4 resolution gets a
short note that "v1.10.0 ships UTC fallback; local-time follow-up
tracked at `spec/<v1.11-slug>/feature.md`". The cockpit displays
`HH:MM` UTC; no operator-visible label change beyond the existing
behaviour. Q4 stays operator-locked for the follow-up brief.

**Acceptance:** developer-pass evidence per the operator's
chosen path. If (a): a screenshot at 3360×1890 showing the time
axis labels in local time (e.g. CET) with the operator's wall
clock confirming the offset is correct. If (b): a short note in
`## Implementation` plus a backlog entry referencing the v1.11
follow-up slug.

---

## Verification (V-items)

Tester contract — each maps to one or more R-items above. Failure
routing per the standard analyst → architect → developer → tester
loop.

### V1 — Tooltip visible at 3360×1890

Developer-provided screenshot evidence (R6 mandatory native capture)
of the cockpit at 3360×1890 with the tooltip rendered over a hovered
fill-marker. Tester verifies the screenshot file's actual pixel
resolution via `sips -g pixelWidth -g pixelHeight` or `identify`.
**R1, R6**.

### V2 — Chart not cropped at three resolutions

Three R6-mandatory screenshots at 1280×720 / mid / 3360×1890; visual
inspection by tester confirms no chart-line clipping at any
resolution. **R2, R6**.

### V3 — Chart re-paints on resize

Manual resize evidence (developer can capture a short screen
recording or a sequence of stills during a drag-resize); tester
confirms the chart-line tracks the new bounds within one frame at
each rest point. New unit test `chart_repaints_on_bounds_change`
asserts cache invalidation. **R3**.

### V4 — Price axis labels in left/right gutter

New panel snapshot `charts_screen_with_axes` (or architect-renamed)
shows the price labels outside the inner rect, in a dedicated
gutter, with tick marks. **R4.1**.

### V5 — Time axis labels below chart

Same snapshot V4 also captures the time-axis row with `HH:MM`
labels at architect's chosen N-spacing. **R4.2**.

### V6 — Legend visible and accurate

New panel snapshot `charts_screen_with_legend` shows all five
legend entries with correct marker styles + colours + labels.
Visual inspection at developer-pass-time confirms the legend
glyphs match the chart canvas's rendered markers. **R5**.

### V7 — Anchor regression 11/11 PASS (hard gate)

`bash scripts/verify_anchors.sh` outputs `ANCHORS PASS (11 / 11)`,
zero diffs vs `spec/anchors.toml`. **R7.1, R8**.

### V8 — v1.9.0 V1–V13 all stay green

Run the v1.9.0 V-suite from
[`spec/chart-buy-sell-emphasis/feature.md`](../chart-buy-sell-emphasis/feature.md)
end-to-end and confirm no regression. Specifically:

- `cargo test -p ui --test panel_snapshots` — Phase 2 baselines +
  v1.9.0 `charts_screen_with_counters_and_chart` + v1.10.0
  new snapshots all green.
- `cargo test -p ui --features live` — green.
- `cargo test --workspace` — green (1000+ tests).
- `cargo test -p audit recent_signals` — green (5 tests).
- `cargo test -p agent config_signal_log_default_off` — green.

**R7**.

### V9 — Visual-verification gate satisfied

Tester verifies three screenshots are present at the claimed
resolutions and each is captured at the developer's actual
native pixel resolution (not downscaled). **R6**.

### V10 — Determinism: two consecutive snapshot runs byte-identical

Run V4 + V5 + V6 snapshots twice; second run byte-identical to
the first. Catches f32 non-determinism in axis tick math or
legend glyph placement. Same precedent as v1.9.0 V10. **R3, R4, R5**.

### V11 — Consistency tests stay green

`cargo test -p ui --test consistency` — green. No inline hex, no
inline strings introduced by axes / legend code. **R10 (v1.9.0
inherited)**.

### V12 — `chart_inner_rect_stays_within_canvas_bounds`

New unit test (architect names exact path) sweeping bounds sizes
from 100×100 to 3360×1890 and asserting
`inner_rect(bounds).right + axis_gutter <= bounds.width` and
`inner_rect(bounds).bottom + time_axis_gutter <= bounds.height`.
**R2, R4**.

### V13 — Tooltip-on-resize survives

New integration test (architect names) — synthetic resize event
between two `CursorMoved` events at different `bounds` rectangles
asserts the second hover correctly publishes
`Message::ChartMarkerHovered` (catches the stale-cache hypothesis
from R1.2 and R2.2 simultaneously). **R1, R2, R3**.

### V14 — Legend card visually distinguishable (new, R9)

UI-designer-captured screenshot at 3360×1890 native shows the
legend card with a clear luminance and/or edge delta against the
chart panel background in both dark and light theme modes.
Operator visual-verification before presenter spawn. **R9**.

### V15 — Local-time-axis follow-up resolved (new, R10)

Per the operator's resolution of **Q-revised-1** (Notes):

- If (a): cockpit screenshot at 3360×1890 shows time-axis labels
  in the operator's local time zone; new unit test
  `local_offset_under_production_reads_os_offset` (developer
  names) asserts the helper returns a non-UTC offset on macOS
  when the OS is set to a non-UTC zone; existing
  `local_offset_under_test_is_utc` stays green via `cfg(test)`
  override; `verify_anchors.sh` → `ANCHORS PASS (11 / 11)`.
- If (b): `## Implementation` carries a one-paragraph note
  citing the v1.11 follow-up slug; no production code change;
  V15 closes by documentation.

**R10**.

**Failure routing:**

- Static / test failure → `developer`.
- Architect-question regression (Q-resolution incompatible with
  V-items) → `architect`.
- Visual-verification screenshot doesn't show the fix on operator
  hardware → `ui-designer` (the UX/visual route per AGENT.md).
- Operator-decide regression (Q4 / Q5 / Q7 reopened) → `analyst`
  (re-scope).
- Anchor diff → `developer` (per `spec/anchors.toml` gate routing).

---

## Notes / Open questions

Each Q-item maps to either `[ARCHITECT-DECIDE]` (deferred to
Design) or `[OPERATOR-DECIDE]` (orchestrator surfaces the question
to the operator before architect spawn). The brief is written so
each question can be answered without reshaping R-items above.

### Q-revised-1 — Q4 local-time path forward [OPERATOR-DECIDE, 2026-05-12]

Original Q4 ("local browser/OS time zone for x-axis") was
operator-locked at brief draft time. Implementation at T3013
landed only the UTC-fallback half: `local_offset_or_utc()`
returns `UtcOffset::UTC` unconditionally because the workspace
`time` dep does not enable `local-offset` (architect's "zero
changes to non-UI crates" non-negotiable blocks the one-line
`Cargo.toml` edit without operator approval). The cockpit is
macOS-only on the operator's hardware, so the `local-offset`
feature's unsafe-on-Linux caveat does not bite us.

**Two paths:**

- **(a) Enable the workspace `time` `local-offset` feature now.**
  One-line edit to the workspace `Cargo.toml` enables
  `local_offset_at()` and lets the helper read the OS offset in
  production. `cfg(test)` override keeps snapshot tests
  deterministic. R10 lands fully in v1.10.0. **Caveat:** if the
  cockpit ever ships on Linux, the multi-threaded glibc
  `local_offset_at()` deadlock risk is on the table — documented
  but not blocking macOS-only deployment.
- **(b) Defer to v1.11 follow-up.** v1.10.0 ships with UTC
  fallback. A short note in `## Implementation` cites the
  follow-up slug; R10 closes by documentation. Q4 stays
  operator-locked for the follow-up.

**[ANALYST-RECOMMENDATION]:** (a). The Linux caveat is hypothetical;
the operator's workflow is macOS native; the friendliness gain is
real (wall-clock-readable time axis matches the operator's
mental model). Operator picks before architect spawn.

### Q-revised-2 — Legend card visibility fix direction [UI-DESIGNER-DECIDE, 2026-05-12]

R9 surfaces a real visibility bug: the legend card paints with
`PANEL_RAISED` fill + `BORDER_1` outline and reads as nearly
invisible against the chart's `PANEL` background on dark mode
at 3360×1890. Four candidate paths in R9.2.

**[ANALYST-RECOMMENDATION]:** ask the ui-designer agent. Without
a screenshot diff between the four candidates, analyst can't pick;
the orchestrator routes this to ui-designer at architect-spawn
time and the architect ratifies the chosen path in `## Design`.

**Sub-question:** does the fix introduce a new `LEGEND_CARD_BG`
token, or re-use an existing tier? Token discipline says: add
the token if no existing tier has the right contrast. UI-designer
makes the call.

### Q1 — "Not centered" — what does the operator see? [OPERATOR-DECIDE]

The operator listed "Not centered" as item 5. **Three plausible
readings:**

- **(a) The chart canvas isn't centered horizontally** — currently
  the canvas runs `Length::Fill` and occupies the full width of
  the chart-body container. The operator may perceive a left-bias
  if the new R4 price-axis gutter ends up on the left, leaving the
  data band visibly off-centre. Probable read for operators
  habituated to TradingView-style charts where the data band is
  visually centred between a left-side y-axis and a right-side
  scale.
- **(b) The price line isn't centred in its vertical range** — the
  `RANGE_PAD_FRACTION = 0.05` constant
  ([`canvas_chart.rs:25`](../../crates/ui/src/widgets/canvas_chart.rs))
  pads 5% above max-high and below min-low. With a strongly
  trending price the line ends up near the top or bottom of the
  range, not centred. Operator may want the line *visually
  centred* in its allocation (e.g. pad to keep the latest close
  near the midline, or apply a logarithmic transform that
  spreads near-recent volatility).
- **(c) The whole Charts screen isn't centred in the application
  window** — currently the sidebar (180 px) is left-anchored and
  the body fills the rest with `Length::Fill`. At 3360×1890 the
  body is ~3180 px wide, which the operator may visually parse
  as "off-centred" because every panel inside is rectangular and
  no element is vertically or horizontally centred. Probably not
  the read — the rest of the cockpit's screens follow the same
  layout.

**[ANALYST-RECOMMENDATION]:** ask the operator. (a) is the most
likely read given the context (the same complaint mentions axes
and legend, suggesting a TradingView mental model). If (b), this
brief grows a new R-item for vertical-range centring. If (c), the
brief expands to a Charts-screen-wide reshape that may not be
worth the scope. Operator clarifies before architect commits to
**Q1**'s resolution in `## Design`.

### Q2 — Price axis on left or right? [ARCHITECT-DECIDE]

R4.1 introduces a price-axis gutter. Two paths:

- **(a) Left.** Matches Western reading order (eye lands on price
  scale first). Reshapes the chart canvas's horizontal budget by
  ~48 px on the left.
- **(b) Right.** Matches TradingView, MetaTrader, Bloomberg
  Terminal convention. Reshapes the chart canvas's horizontal
  budget by ~48 px on the right. The price line's most-recent
  close sits adjacent to the price label, which reads as a
  natural "current price" indicator.

**[ANALYST-RECOMMENDATION]:** (b) Right — financial-chart
convention. Architect confirms; (a) is also defensible per Lumen
design discipline if a Lumen pattern document picks left.

**Sub-question:** thin 1-px axis line drawn along the gutter's
inner edge? Analyst strawman yes (matches the v1.9.0 gridline
treatment); architect picks the alpha.

### Q3 — Time-axis tick spacing N [ARCHITECT-DECIDE]

R4.2.1 needs a tick-spacing constant. Three options for a 60-bar
window:

- **(a) Every 5 bars** = 12 ticks. Dense; risks label-overlap at
  1280×720 floor.
- **(b) Every 10 bars** = 6 ticks. Sparse; reads cleanly at all
  resolutions. **[ANALYST-RECOMMENDATION]**.
- **(c) Adaptive** = compute tick count from canvas width / label
  pixel width. Cleanest visually; more code. Architect picks (c) if
  budget allows.

### Q4 — Time-axis time zone: UTC or local? [OPERATOR-DECIDE]

R4.2.2 ships time labels in `HH:MM` format. The audit ledger
records all timestamps as RFC3339 UTC; the cockpit tooltip
already shows RFC3339 UTC per v1.9.0 R4.2.

**Three options:**

- **(a) UTC.** Consistent with audit ledger + tooltips +
  presenter reports. Operator must mentally convert to local time.
- **(b) Local-display time.** What the operator's wall-clock
  shows. Friendlier; risks confusion when comparing chart times
  to audit-ledger times in the tooltip.
- **(c) Both** — UTC primary, local secondary (small annotation
  per first tick). Most informative; busiest visually.

**[ANALYST-RECOMMENDATION]:** ask the operator. (a) is the
discipline-safe answer; (b) is the friendly answer. The operator's
six-item report didn't specify, so this is a real open question.

### Q5 — Legend placement [ARCHITECT-DECIDE]

R5.2 surfaces three placements (a) / (b) / (c). Architect picks.

**[ANALYST-RECOMMENDATION]:** **(a)** top-right inset over the
chart canvas. Reasoning: doesn't reshape the Charts screen
Column; uses dead space near the top edge; matches financial-
chart convention (TradingView's legend pinned top-left over the
chart). The "covers ~3-5% of the chart" concern is real but
overlap is rare in practice (the top-right corner is the
most-recent high-price region; markers cluster around the
trading-active time band which is the middle of the time axis).

**Operator-confirm** if architect picks (a); the operator may
prefer the data band to stay legend-clear, in which case
architect routes to (b) or (c).

### Q6 — Native-retina screenshot tooling [ARCHITECT-DECIDE / process]

R6.3 surfaces a need for the `capture-screenshot` skill to support
"at native Retina resolution" capture. Two paths:

- **(a) In-scope.** Extend the skill in this brief; developer
  documents the new operator-instruction branch.
- **(b) Out of scope.** A separate process-tooling brief covers
  the skill extension; this brief uses the existing `screencapture
  -x` directly with the developer's own macOS shell.

**[ANALYST-RECOMMENDATION]:** (b) — keep this brief tight to the
six operator-reported items + the gate. Skill polish is a
follow-up; the gate enforces the discipline regardless of which
tool the developer uses.

### Q7 — Viewer parity in v1.10.0? [OPERATOR-DECIDE]

R12 v1.9.0 left viewer parity as a follow-up brief. v1.10.0 also
inherits a no-viewer-changes default. **Operator confirms.**

**[ANALYST-RECOMMENDATION]:** ship cockpit-only, same as v1.9.0.
The viewer's KPI strip + equity curve + drawdown band are
post-hoc-review widgets; axes / legend / scaling on the live
chart serve the live-monitoring case the operator described.

### Q8 — Should `standard_window_settings()` open at a larger initial size? [ARCHITECT-DECIDE]

R6.1 surfaces the precondition failure: the cockpit binary opens
at 1280×720 every launch even on a 3360×1890 display. This may not
need a fix in this brief (the operator's workflow is "resize once
on first launch, the OS remembers"), but if the architect wants
to ship a friendlier default, the initial `size` field could
detach from `min_size`.

**Three options:**

- **(a) Status quo.** `size = min_size = (1280, 720)`. Operator
  resizes once; iced 0.14 may or may not honour any OS-side
  remembered geometry on subsequent launches.
- **(b) Larger initial.** `size = (1920, 1080)` (or similar)
  while `min_size` stays `(1280, 720)`. More cinematic first-
  launch; harmless on small screens (iced clamps to the display).
- **(c) Maximised by default.** Use iced's `position: SpecificWith(
  Position::Centered)` and `maximized: true` flags on bootstrap.
  Maximally operator-friendly; debatable on smaller laptop screens.

**[ANALYST-RECOMMENDATION]:** ask architect. (b) is the
medium-risk default upgrade; (c) is the maximally-friendly default
but may surprise operators who run the cockpit alongside other
windows.

### Q9 — Anything else?

- **Chart panning / zoom?** v1.9.0 R3 explicitly defers
  pan/zoom; this brief inherits that deferral. Operator may
  surface it as item 7 later — out of scope here.
- **Crosshair on hover?** A vertical line at the cursor's x with
  the bar's open/close/high/low surfaced in the time-axis label.
  Standard financial-chart UX. Not in operator's six-item
  report; analyst recommends defer to a v1.11 brief if operator
  asks.
- **Bar count > 60?** Multi-day window; out of scope (R4.2.3
  documents the format-switch shape for future work).

---

## Backtest scenarios

_n/a — UI feature, no new backtest scenarios. Existing 11 anchored
reports guard rendering / strategy / audit-write-path drift; this
feature touches none of those code paths (R8)._

---

## Diagnostic — live cockpit pass (2026-05-12, architect)

> **SUPERSEDED — see [`## Diagnostic — CORRECTED (2026-05-12,
> orchestrator-led)`](#diagnostic--corrected-2026-05-12-orchestrator-led)
> below.** This section's Observation 2 ("canvas paints at ~½
> scale, anchored to the canvas's left edge") was **empirically
> disproven** by the orchestrator on the operator's native
> 3360×1890 hardware via two decisive instrumentation tests
> (red-rect fill + cyan-dot-per-bar). The chart line + gridlines
> + price labels render at the FULL allocated canvas width; the
> apparent shortness in the architect's screenshots was a
> fixture-shape artifact (the synthetic price walk is flat-then-
> drop, so bars 0–30 cluster vertically at the top and bars 30–60
> slope downward). Observations 1, 3, 4 below remain valid; only
> Observation 2 and the hypothesis-pinning that flowed from it
> are retracted. Original text retained verbatim for audit trail.

The architect ran the cockpit binary on the operator's macOS Retina
desktop, instrumented `ChartProgram::update` + `ChartProgram::draw`
with throttled `eprintln!`-based tracing, and captured three
screenshots covering: (a) launch state, (b) launch state with the
window maximised by the iced runtime, (c) the window manually
shrunk back to the min-size floor.  This Diagnostic locks the
load-bearing facts each R-item's design ratifies; the screenshots
are kept in [`reports/screenshots/`](reports/screenshots/) and
the raw trace at
[`reports/diagnostic-trace-2026-05-12.log`](reports/diagnostic-trace-2026-05-12.log)
(193 lines, 1230+ `update` events, 130+ `draw` events).

### Setup

- Binary: `cargo build --release -p ui --bin cockpit --features fixtures`
  succeeded; cockpit launched.
- Default screen was patched to `Screen::Charts` for the pass (one-
  line edit in `crates/ui/src/bin/cockpit.rs`; reverted after capture).
- Throttled `eprintln!` (every 20th `CursorMoved`, every 10th `draw`)
  on `bounds`, `cursor.position_in(bounds)`, and
  `state.hovered_marker_idx`.  No production code paths were changed
  beyond the temporary print.  All diagnostic edits are reverted; a
  fresh `cargo check` confirms the repository is back to baseline.
- Window listing via Cocoa `CGWindowListCopyWindowInfo` (Swift one-
  shot).  No AppleScript permissions needed.
- Screenshots via `screencapture -l <CGWindowID>` (Retina pixel
  capture — sips reports image dims at 2.06–2.11x logical, i.e. the
  display's actual HiDPI factor).

### Observation 1 — bounds propagate correctly through resizes (R3 hypothesis (a) **confirmed**, (b) **rejected**)

Trace excerpts:

```
chart_diag.draw  n=0   bounds=(196.0,162.0,1068.0x406.7)  bars=60 markers=4 signals=0
... operator (architect-as-operator) maximises window ...
chart_diag.draw  n=40  bounds=(196.0,138.6,1844.0x874.1)  bars=60 markers=4 signals=0
... shrink back to min-size ...
chart_diag.draw  n=130 bounds=(196.0,138.6,1840.0x873.1)
chart_diag.draw  n=140 bounds=(196.0,162.0,1126.0x439.7)
chart_diag.draw  n=150 bounds=(196.0,162.0,1068.0x406.7)
```

The canvas's `Program::draw` receives the new `bounds.size()` on
each window-resize-end event; iced fires `Program::draw` repeatedly
during the resize (n=40 → n=130 = 90 redraws spanning the maximise
→ shrink cycle).  **iced 0.14 already auto-invalidates the canvas
geometry cache on `bounds.size()` change.**  R2's hypothesis 2
("canvas-cache stale") is therefore rejected — the cache is not the
problem.  No additional cache-invalidation hook needed in
`canvas_chart.rs`.

The diagnostic also confirms `cursor.position_in(bounds)` returns
`Some(p)` cleanly when the cursor is over the canvas and `None`
when it leaves — at every window size, both physical and logical
coordinates round-trip correctly.  **R1 hypothesis 1 (event-pump
HiDPI regression) is therefore rejected** for the macOS/iced 0.14
runtime tested.

### Observation 2 — **Canvas rendering scale mismatch: chart paints at ~½ its claimed bounds (load-bearing for R2 + R3)**

The diagnostic log says `bounds.width=1068` (at 1280×752-window
size), but the launch-state screenshot
[`reports/screenshots/diag-cockpit-charts-current.png`](reports/screenshots/diag-cockpit-charts-current.png)
shows the chart line + gridlines + price labels all confined to
**~450 logical pixels** of horizontal canvas — only the left ~42 %
of the canvas's allocated width.  The right ~58 % of the canvas is
empty `color::PANEL` background.  Same defect at the maximised
state: bounds.width=1844 but visible drawing region ≈ 770 logical
pixels.

The price labels (`102.05`, `98.52`, `94.99`, `91.46`, `87.93`)
appear at image x ≈ 210, which maps to **logical x ≈ 100** — i.e.
INSIDE the 180-px sidebar, not at the expected `bounds.x + 12 =
208`.  Gridlines start at image x ≈ 250 → logical x ≈ 125 (also
inside the sidebar's slot).  This is the operator's "not centered"
+ "charts are croped" + "lines only partial visible" report
collapsing onto a single root cause.

**Hypothesis (architect-pinned):** iced 0.14's `Frame::new(renderer,
bounds.size())` + `Program::draw` is rendering the returned
`Geometry` at a coordinate system that's NOT scaled by the window's
DPI factor on macOS Retina — the geometry is drawn at "half the
intended size, anchored at the canvas's left edge".  The bug is
not in our `inner_rect` math (the math is correct against `bounds`)
— it's in the iced canvas → wgpu blit path.  This matches the iced
GitHub issues
[#2476](https://github.com/iced-rs/iced/issues/2476) and
[#2640](https://github.com/iced-rs/iced/issues/2640)-class
phenomena.  Defensive workarounds:

1. **Re-derive frame size from a known-good source.**  Replace
   `Frame::new(renderer, bounds.size())` with the iced 0.14 idiom
   that takes `renderer` + a `Size` that the caller obtained from
   `iced::widget::canvas::Geometry::new(...)` (newer API).  Confirm
   against iced 0.14.x patch notes.
2. **Hard-clamp via explicit `Stroke::with_line_cap` / fill
   coordinates measured against a known-good rectangle**, not
   against `bounds.size()`.
3. **Upgrade iced** to a patch that fixes the canvas scale bug, if
   available; otherwise pin to the working release and ship a
   workaround.

The developer pass MUST reproduce the bug at 1280×720 + 1920×1080
+ 3360×1890 native (R6 gate) and verify the fix at all three
sizes before tester gate.

### Observation 3 — Tooltip code path is reachable; live hover-evidence is pending (R1)

`hit_test` requires the cursor to fall inside the
`MARKER_HIT_RECT_PX = 28`-square around a marker centroid.  During
the architect's diagnostic pass the cursor never crossed a marker
hit-rect (no `hovered=Some(_)` in 1230 `update` events).  This is
not evidence of an event-pump bug — it's evidence that the architect
did not manually trigger a hover during the capture window.  Given
Observation 2 (the chart paints at ~½ scale), the **markers are
visually clustered into a sub-region** of the canvas while their
hit-rects are computed against the FULL `bounds`-width — so the
visual marker at the screen pixel where the operator's cursor lands
DOES NOT coincide with the hit-rect.  **Operator's "I still don't
see tooltip overlay" is therefore explained by Observation 2's
scale mismatch: the operator hovers over the visible marker, but
the hit-test uses the wrong-scale geometry and returns `None`.**

The fix for Observation 2 also fixes R1 by reunifying the visual
and hit-test coordinate systems.  No separate tooltip-overlay fix
needed if the canvas-scale bug is closed first.

### Observation 4 — Window opens larger than `min_size = 1280×720`

iced 0.14 on macOS opens the window at the operator's screen-fitted
size (Quartz reported W=2056 H=1196 logical on first launch in the
architect's pass; later runs reported W=1280 H=752 after the system
honored the `Settings.size`).  This is **non-deterministic across
launches** and explains why the v1.9.0 tester's 1280×720 capture
was visually clean (the chart paints OK at min-size on first launch
when iced honors `Settings.size = min_size`) while the operator's
3360×1890 hardware shows the scale defect as soon as the window is
larger than min.  **This is the Q8 surface.**

### What this Diagnostic locks

- **R1 hypothesis 2 (hit-rect math at scale) → confirmed via
  proxy.** Root cause is upstream (Observation 2's scale mismatch),
  not in `marker_hit_rect` itself.  Hit-rect math is correct
  against the geometry it sees; the geometry is wrong.
- **R1 hypothesis 1 (event-pump HiDPI confusion) → rejected.**
  `cursor.position_in(bounds)` returns clean logical coordinates
  at every tested window size.
- **R1 hypothesis 3 (tooltip card off-canvas) → not yet ruled out,
  but unlikely to be load-bearing.**  Even with a corrected scale,
  M1's developer pass MUST capture a live hover screenshot at
  3360×1890 to confirm the tooltip lands on-screen.  If not, the
  fix extends `chart_tooltip::compute_card_rect` to clamp the card
  inside `bounds` (defence-in-depth, R1.2 invariant).
- **R2 hypothesis 1 (padding leak) → rejected.**  `inner_rect` math
  is correct; the visual cropping is downstream of `Frame::new`'s
  scale defect.
- **R2 hypothesis 2 (canvas-cache stale) → rejected.**  iced 0.14
  invalidates correctly on bounds change.
- **R3 hypothesis (a) (auto-repaint on resize) → confirmed.**  No
  cache-invalidation work needed in `canvas_chart.rs`.
- **R3 hypothesis (b) (proportional intrinsic scaling) → out of
  scope for v1.10.0.**  Operator's "scale like svg" reads as (a);
  (b) is a v1.11 feature.

### Screenshots filed

| Path                                                                | Size (px)     | Window state                  | Use                              |
|---------------------------------------------------------------------|---------------|-------------------------------|----------------------------------|
| [`reports/screenshots/diag-cockpit-charts-launch.png`](reports/screenshots/diag-cockpit-charts-launch.png) | 4248 × 2528   | First-launch (iced opened maximised at 2056×1196 logical) | Shows 6 R-items at large window: no legend, no time axis, price labels misaligned, chart line stops at ~½ width.  Hi-res operator-facing evidence. |
| [`reports/screenshots/diag-cockpit-charts-current.png`](reports/screenshots/diag-cockpit-charts-current.png) | 2696 × 1640   | After manual shrink to 1280×752 (logical) | Same defects compressed.  Confirms bug repro at MIN size — the v1.9.0 tester's screenshot resolution. |
| [`reports/screenshots/diag-cockpit-charts-small-bounds.png`](reports/screenshots/diag-cockpit-charts-small-bounds.png) | 2696 × 1640   | Same as above, second capture | Cross-check (rules out a one-off transient frame). |
| [`reports/screenshots/diag-launch-home-screen.png`](reports/screenshots/diag-launch-home-screen.png) | 4112 × 2658   | Full-display capture during launch | Anchor for the diagnostic ledger; not used for R-item evidence. |

---

## Diagnostic — CORRECTED (2026-05-12, orchestrator-led)

After the developer pass landed T3001 + T3004 + T3006 + T3009–T3022
and paused all canvas-scale-dependent tasks, the orchestrator ran
the cockpit binary on the operator's native 3360×1890 Retina
hardware with two rounds of temporary diagnostic instrumentation in
`crates/ui/src/widgets/chart.rs`. Both rounds were reverted to a
clean working tree before this re-spec; the empirical evidence
lives in the screenshots cited below.

### Test 1 — Red-rect fill: canvas paints at FULL allocation

Inside `ChartProgram::draw`, immediately after computing `inner =
chart_inner_rect_with_gutters(bounds.size(), ...)`, a temporary
`frame.fill_rectangle(Point::new(inner.x, inner.y),
Size::new(inner.width, inner.height), Color { r: 1.0, g: 0.0,
b: 0.0, a: 0.4 })` was added. The expectation from the architect's
Observation 2 hypothesis (canvas paints at ~½ scale, anchored to
canvas left edge): the red rectangle should cover only the LEFT
~42 % of the canvas with the right ~58 % staying chart-`PANEL`.

**Observed:** the red rectangle **covered the FULL chart canvas
edge-to-edge** — from the left price-axis gutter to the right
margin, from the status strip to the time axis. There is no
half-scale rendering. The iced 0.14 `Frame::new(renderer,
bounds.size())` + `Program::draw` + `Geometry`-blit pipeline
produces a geometry that fills the canvas's reported `bounds`.

Evidence: [`/tmp/orch-diag/cockpit-red-rect.png`](/tmp/orch-diag/cockpit-red-rect.png).

### Test 2 — Cyan-dot per-bar: bars span the full inner-rect width

A second instrumentation round replaced the red-rect fill with
small `frame.fill_rectangle(...)` calls at each bar's computed
`(x, y)` plotting position. The expectation: if the line code's
x-coordinate math is wrong, the cyan dots cluster into a sub-
region of the canvas.

**Observed:** all **60 dots distributed evenly across the full
inner-rect width** — `first_x = 56.0, last_x = 1684.0` against
`inner.width = 1628.0` (the bars span the entire drawable area
minus the half-bar-width pad on each side). The chart line code
in `chart::draw_chart_line` is correct.

The line *appears* short in the architect's diagnostic screenshots
because the synthetic-fixture price walk is **flat-then-drop**:
bars 0–30 are roughly constant at the top of the price range
(producing a horizontal segment that visually reads as a short
horizontal line at the top), and bars 30–60 slope down to the
bottom. In the architect's compressed screenshot the flat top
segment is mistaken for "line stops at ~½ width" — but the
chart line actually traverses the entire canvas. The fix is
**nothing**; the fixture shape is the visual artifact.

Evidence:
- [`/tmp/orch-diag/cockpit-cyan-charts.png`](/tmp/orch-diag/cockpit-cyan-charts.png)
- [`/tmp/orch-diag/cockpit-cyan-dots.png`](/tmp/orch-diag/cockpit-cyan-dots.png)
- [`/tmp/orch-diag/cockpit-cyan-v2.png`](/tmp/orch-diag/cockpit-cyan-v2.png)
- Trace log at `/tmp/orch-diag/chart-draw.log` recorded the
  `first_x` / `last_x` / `inner.width` numbers above; the log
  itself was lost when the diagnostic process was killed, but
  the screenshots are durable.

### Final clean-tree screenshot — v1.10.0 v-current visual state

After both diagnostic patches were reverted and the working tree
returned to clean (T3001 + T3004 + T3006 + T3009–T3022 landed,
nothing else), the orchestrator captured the cockpit in its
v1.10.0 v-current shipping shape:

- [`/tmp/orch-diag/cockpit-final-charts.png`](/tmp/orch-diag/cockpit-final-charts.png)

This screenshot is the empirical baseline for R9 (the legend
card visibility bug surfaces clearly in this capture).

### Corrected conclusion

- **No iced 0.14 canvas-scale bug exists on the operator's
  hardware.** `Frame::new(renderer, bounds.size())` works
  correctly at 3360×1890 native Retina. The architect's GitHub
  issue references (#2476 / #2640) are real iced issues but do
  not bite this cockpit's call path.
- **R1 (tooltip invisible) — root cause not yet pinned.** The
  scale-mismatch theory that made the markers' hit-rects fail
  is now retracted. Live hover-verification is still needed on
  the operator's hardware (the orchestrator cannot simulate
  hover from sandbox without macOS Accessibility permission,
  which is still missing). T3029 owns this.
- **R2 (chart cropped) — RESOLVED.** Operator-confirmed after
  M2/M3 axes landed (see R2's resolution banner above).
- **R3 (SVG-style scaling) — RESOLVED by R2's resolution.**
  iced's auto-repaint-on-bounds-change was always correct
  (Observation 1 from the architect's diagnostic stands).
- **The developer pass was not wasted.** T3001 (diagnostic),
  T3004 + T3006 (defensive tests), T3009–T3022 (tokens, axes,
  legend wire-up, viewer parity, window-size bump) all ship
  cleanly in v1.10.0. The retracted Observation 2 only blocked
  T3002 / T3003 / T3007 / T3008 — those are closed as no-op in
  `tasks.md`.
- **New scope surfaced by the corrected baseline:** R9 (legend
  visibility — a real bug the original brief never anticipated)
  + R10 (Q4 local-time follow-up — Cargo.toml feature flip the
  developer correctly punted on without operator approval).

### Audit trail of the misdiagnosis

The architect's Observation 2 was a faithful read of the
screenshots they had at the time — the visual evidence at the
compressed scale they captured at really does look like a half-
scale canvas. The misdiagnosis was a perceptual error compounded
by:

1. The synthetic fixture's flat-then-drop price walk producing
   a visually misleading line shape.
2. The architect's screenshots being downscaled in display
   (the actual pixel resolution is preserved in the file, but
   the architect read them at a smaller on-screen size).
3. No red-rect-style "what does the canvas paint?" probe — the
   architect's instrumentation captured `bounds`/`cursor` events
   but not the actual painted geometry, so the gap between
   "iced reports bounds.width=1844" and "what gets blit'd" was
   never closed empirically.

R6 (visual-verification gate) was written for this exact failure
mode — the orchestrator's red-rect + cyan-dot probe is the
correction R6 demands but the original developer pass couldn't
execute without graphical display access. The lesson is now in
the audit trail.

---

## Design

### Resolved Qs

- **Q1 (operator-locked) — TradingView-style placement.** Chart
  canvas is centred between a left price-axis gutter
  (`AXIS_GUTTER_PRICE_PX`) and an optional right margin
  (`AXIS_GUTTER_RIGHT_PX`).  Line keeps `RANGE_PAD_FRACTION=0.05`
  inside the resulting inner rect.  *Rationale:* matches the
  operator's mental model (TradingView/MetaTrader convention);
  parameterising both gutters lets v1.11 add a right-side
  current-price tag without re-architecting.
- **Q2 (architect-decide) — Price axis on the LEFT.**  Western-
  reading-order convention; left gutter consumes ~48 px (architect's
  pick after Lumen typography metrics, see token list below).  Thin
  1-px vertical axis line in `color::BORDER_1 @ alpha 0.4` (mirrors
  the gridline treatment).  *Rationale:* the operator's "not
  centered" complaint is closed by introducing a gutter at all;
  left vs right is then a 50/50 ergonomic call and the Lumen left-
  reading-order pattern wins by symmetry with the rest of the
  cockpit's panel headers (always left-aligned).  Q1's "TradingView-
  style centering" is satisfied as long as a gutter exists on one
  side — the analyst's strawman that picked (b)/right is overridden
  here because all other Lumen panel headers anchor left and a
  left-axis is more discoverable on a 1280-px floor.
- **Q3 (architect-decide) — Adaptive tick spacing.**  Compute
  `tick_count = clamp(canvas_width_logical / 96.0, 4, 12)`
  rounded to nearest 5/10/15-bar boundary (only multiples of 5
  bars are eligible) so labels never overlap at 1280-px floor (gets
  4 ticks ≈ 15 min each) and never sparse at 3360-px (gets 12 ticks
  ≈ 5 min each).  *Rationale:* the operator works across two
  monitor sizes (laptop floor + Retina desktop); a fixed `N=10` is
  fine on 1280 but visually sparse on 3360, and a fixed `N=5` is
  cluttered on 1280.  Adaptive closes both ends.
- **Q4 (operator-locked) — Local browser/OS time zone for x-axis.**
  Tooltip `ts` keeps RFC3339-UTC representation, audit ledger keeps
  RFC3339-UTC.  Time axis labels use the platform's default time
  zone via `time::UtcOffset::current_local_offset()` (falls back to
  UTC if the local offset cannot be determined — defensive).
  *Rationale:* operator's live-monitoring workflow expects wall-
  clock-readable times; the audit ledger's UTC discipline is
  preserved for forensic / cross-day analysis.
  **STATUS 2026-05-12 (re-spec):** PARTIALLY landed — helper
  `local_offset_or_utc()` returns `UtcOffset::UTC` unconditionally
  because the workspace `time` dep does NOT enable `local-offset`
  (workspace `Cargo.toml` edit blocked by "zero changes to non-UI
  crates" non-negotiable). Operator decision required at
  **Q-revised-1** in Notes (enable feature for macOS-only cockpit,
  or defer to v1.11). See **R10**.
- **Q5 (architect-decide) — Legend = top-right inset over chart
  canvas (placement (a)).**  Card chrome: `~140 px` wide × `~80 px`
  tall, `PANEL_RAISED` background, `BORDER_STRONG @ 1 px` outline,
  `space::S` interior padding, anchored at `(inner.right - card.w -
  space::M, inner.y + space::M)`.  *Rationale:* doesn't reshape the
  Charts-screen `Column` (no fixed-height strip eats vertical
  budget the chart shares with the histogram), matches TradingView
  convention (legend pinned to the chart-canvas corner), and the
  top-right is the high-price/recent-time corner where markers
  cluster least (the operator's typical hover band is the middle
  of the time axis).  If the operator later asks for "legend
  somewhere else", v1.11 adds a 1-line `chart_legend_position`
  preference token; design is forward-compatible.
  **STATUS 2026-05-12 (re-spec):** placement landed at T3017
  (`chart.rs:485-494`), but the ui-designer's atomic chrome
  refinement swapped the outline from `BORDER_STRONG` to
  `BORDER_1` (T3016 landing note). On the orchestrator's clean-
  tree screenshot at 3360×1890 the resulting card is **barely
  visible** against the chart's `PANEL` background. **R9** opens
  this as a real bug; the architect must ratify the fix path at
  **Q-revised-2** in Notes before the developer/ui-designer
  spawn.
- **Q6 (architect-decide) — Screenshot skill extension out of
  scope.**  The developer uses the existing
  `scripts/capture_screenshot.sh` plus
  `screencapture -l <CGWindowID>` directly.  The 3-resolution gate
  is process-level (R6), not skill-level — no skill extension
  needed for v1.10.0.  *Rationale:* keep the brief tight; skill
  polish is a separate process-tooling brief whenever the workflow
  outgrows manual invocation.  A 5-line helper at
  `scripts/capture_chart_3sizes.sh` (architect proposes adding it
  during M6 if the developer needs it) drives the three captures.
- **Q7 (operator-locked) — viewer parity = BOTH.**  Cockpit live
  charts AND `crates/ui/src/bin/viewer.rs` backtest viewer get the
  axes + legend.  Architect scopes the equity-curve / drawdown-band
  / sparkline widgets as follows:
  - `equity_curve::view` — gets the **price axis** treatment (USD
    value labels in left gutter, 5 gridlines).  Time axis = **YES**
    (viewer shows the full backtest span — operators need to read
    wall-clock).  Legend = **NO** (single-series, label-redundant).
  - `drawdown_band::view` — gets the **price axis** (% labels).
    Time axis = **YES** (same span as equity_curve, shared on the
    viewer screen).  Legend = **NO**.
  - `sparkline::view` — **out of scope**.  Sparklines are 12-point
    embedded glyphs in the Strategies-detail table — axes would
    eat the entire allocation.  No axes, no legend.
- **Q8 (architect-decide) — Bump initial window size to 1920×1080.**
  `standard_window_settings()` `size = Size::new(1920.0, 1080.0)`;
  `min_size` stays `(1280.0, 720.0)`.  *Rationale:* operator's
  daily-driver is 3360×1890; the v1.9.0 tester PASS-on-1280×720
  hid the canvas-scale defect; iced clamps the boot size to the
  display anyway, so a larger default is harmless on small laptop
  screens.  Maximised-by-default is rejected — the operator may
  run the cockpit alongside other windows.

### Component decomposition

**No new crates, no `core` types touched.**  All changes live in
`crates/ui/`.  Concretely:

| File                                                     | Change                                                                 | Test surface                |
|----------------------------------------------------------|------------------------------------------------------------------------|-----------------------------|
| `crates/ui/src/widgets/canvas_chart.rs`                  | New `inner_rect_with_gutters(size, left, right, top, bottom)` helper. Existing `inner_rect` keeps the 8-px gutter shape — sparkline-callers don't migrate. | unit                        |
| `crates/ui/src/widgets/chart.rs`                         | Replace `inner_rect(bounds.size())` with `inner_rect_with_gutters(bounds.size(), AXIS_GUTTER_PRICE_PX, AXIS_GUTTER_RIGHT_PX, 0.0, AXIS_GUTTER_TIME_PX)`.  Remove inline price labels at `inner.x + 4`.  Add new draw passes: **price axis** (left gutter), **time axis** (bottom gutter), **legend** (top-right inset over inner rect).  Replace the `Frame::new(renderer, bounds.size())` call with the canvas-scale workaround (Observation 2 fix; exact form pinned at developer-pass time after verifying iced 0.14 patch state). | unit + snapshot + R6 visual |
| `crates/ui/src/widgets/chart_tooltip.rs`                 | Add `compute_card_rect` clamp inside `bounds` (defence-in-depth for R1.3). | unit                        |
| `crates/ui/src/widgets/equity_curve.rs`                  | Adopt `inner_rect_with_gutters` with left+bottom gutter set; new draw passes for price + time axes. | unit + snapshot             |
| `crates/ui/src/widgets/drawdown_band.rs`                 | Same as equity_curve.                                                  | unit + snapshot             |
| `crates/ui/src/widgets/sparkline.rs`                     | **No change.**  Out of viewer-parity scope (Q7).                       | regression-stays-green only |
| `crates/ui/src/widgets/chart_legend.rs` *(new)*          | Standalone draw helper for the 5-entry legend card.  Re-uses `chart::draw_triangle` at `text::MICRO` glyph size.  Exposed as a free function `draw_legend(frame, inner, mode)` — single canvas pass, no widget tree. | unit + snapshot             |
| `crates/ui/src/screens/charts.rs`                        | No structural change; the chart canvas's allocation arithmetic stays.  Time-axis height is consumed inside the canvas, not the Column — `chart_canvas_height_for_body` stays correct. | existing unit                |
| `crates/ui/src/bin/viewer.rs`                            | No structural change — `equity_curve::view` / `drawdown_band::view` upgrade in place. | existing snapshot           |
| `crates/ui/src/window_icon.rs`                           | `standard_window_settings()` — `size = Size::new(1920.0, 1080.0)` (was `MIN_*`).  `min_size` unchanged. | unit                        |
| `crates/ui/src/theme.rs`                                 | New tokens in `theme::layout` (below).                                  | unit                        |
| `crates/ui/src/strings.rs`                               | New `CHART_LEGEND_*` strings (R5.1) + optional `CHART_AXIS_*` unit suffixes if architect-chosen during M2/M3 (default: no suffixes, the numbers stand alone). | unit + consistency          |

### Lumen tokens introduced

All new tokens land in `crates/ui/src/theme.rs` under `pub mod
layout`:

```rust
/// Left price-axis gutter (M2 / R4.1).  Sized for a 5-digit price
/// label (`102.05`) at text::MICRO (11 px) with `space::S` left
/// and right pad.  text::MICRO at FontMono-derived width ≈ 6.5 px
/// per digit → 5 × 6.5 + 16 (pads) = 48.5 → round to 48.
pub const AXIS_GUTTER_PRICE_PX: f32 = 48.0;

/// Right canvas margin (Q1 — TradingView-style symmetry without a
/// right-side label column in v1.10).  Empty band that lets the
/// most-recent close-price marker breathe.  v1.11 may repurpose
/// this for a right-side current-price tag.
pub const AXIS_GUTTER_RIGHT_PX: f32 = 16.0;

/// Bottom time-axis gutter (M3 / R4.2).  text::MICRO baseline + 4
/// px tick + space::XXS gap = 11 + 4 + 4 + 4 = 23 → round to 24.
pub const AXIS_GUTTER_TIME_PX: f32 = 24.0;

/// Legend card chrome width (M4 / Q5).  Fits "Buy signal" /
/// "Sell signal" — the longest entry — at text::MICRO with the
/// 10-px glyph and space::S inter-column gap.
pub const LEGEND_CARD_WIDTH_PX: f32 = 140.0;
/// Legend card chrome height — 5 entries × (10 px glyph + 2 px
/// gap) + 2 × space::S pad = 76 → round to 80.
pub const LEGEND_CARD_HEIGHT_PX: f32 = 80.0;
/// Legend triangle glyph height (Q5 — half of MARKER_SIZE_PX).
pub const LEGEND_GLYPH_PX: f32 = 10.0;
```

### Data model changes

**None.**  All work is presentation-only:

- No changes to `crates/core/src/types/{Bar,FillView,SignalView}`.
- No new `Message::*` variants on `state::Message`.
- No new fields on `state::Cockpit`.
- No new audit-write paths, no bus channels, no strategy code.

This satisfies R7 v1.9.0 R9 (determinism, read-only) and R8
(zero-anchor regression by code-path inspection).

### Implementation passes (collapsed M1+M2 → M1)

The Diagnostic collapses the analyst's M1 + M2 split: R1 (tooltip
invisible) and R2 (chart cropped) share a root cause
(Observation 2's scale defect).  Tasks group accordingly under M1
in `tasks.md`.  M3 (R3 SVG-style scaling) is **closed by the M1
fix** — no separate work.  Time + price axes (M2/M3 in the old
plan) move to M2/M3 in the new plan (renumbered).

> **CORRECTION 2026-05-12 (re-spec).** The "shared root cause"
> framing is retracted — Observation 2 was empirically disproven
> (see `## Diagnostic — CORRECTED`). M1's canvas-scale tasks
> (T3002/T3003/T3007/T3008) close as **no-op** in `tasks.md`. R1
> (tooltip live-hover verification) remains open as a process item
> only (T3029: operator-blocked live-hover screenshot). R2/R3 are
> operator-resolved by the M2/M3 axes landing. The M-pass shape
> stays as-is; only the M1 task scope shrinks.

### Risk register

1. **iced 0.14 canvas scale workaround may not exist as a one-line
   fix.**  If iced's macOS-Retina geometry path is genuinely
   broken, the developer pass may need to upgrade iced (which
   ripples through the entire `crates/ui` widget surface).  *Mitigation:*
   developer kicks off M1 with a 30-minute spike against the iced
   GitHub issue tracker + a minimal reproducer in a scratch
   `examples/canvas_scale.rs`.  If the spike concludes "need iced
   upgrade", `HANDOFF → architect (re-scope)` BEFORE M2/M3/M4
   start.
   **STATUS 2026-05-12 (re-spec):** RETIRED. Orchestrator's
   red-rect + cyan-dot empirical probe disproved the canvas-
   scale defect at 3360×1890 native (see `## Diagnostic —
   CORRECTED`). Risk item is closed; no iced upgrade needed.
2. **Local-time-zone determinism for snapshot tests.**  Q4's
   resolution (local-time on x-axis) means
   `time::UtcOffset::current_local_offset()` enters the canvas
   `draw` path — non-deterministic by definition.  *Mitigation:*
   pin a TEST clock via `cfg(test)`-only override that returns
   `UtcOffset::UTC`.  Snapshot tests run with the override; the
   live binary reads the OS offset.  Documented as the
   determinism invariant under V10 in feature.md.  R7 v1.9.0
   determinism guard stays green.
3. **Charts-screen `Column`-allocation arithmetic.**  Moving the
   price labels OUT of the inner rect into the left gutter, AND
   adding a time gutter inside the canvas, both eat from the
   canvas's drawable area but DO NOT change the canvas's outer
   allocation.  *Mitigation:* `chart_canvas_height_for_body`
   stays correct as-is (it computes the canvas's vertical budget
   against fixed siblings outside the canvas).  The new
   `inner_rect_with_gutters` shrinks the drawable area inside the
   same canvas allocation.  Existing test
   `chart_canvas_height_grows_with_body_height` stays green
   verbatim.
4. **Snapshot churn.**  `panel_snapshots__charts_screen_with_counters_and_chart`
   will change (new axes + legend visible).  *Mitigation:* update
   the snapshot baseline atomically with the M2/M3/M4 landing;
   tester confirms the diff is only the expected new chrome (no
   line/marker drift).  This is the analyst's "Type A baseline
   churn" item.

### Backwards-compatibility

- **Anchor invariant (R8) — confirmed by inspection.**  All 11
  anchors live under `spec/anchors.toml` and are body-SHA-256s of
  `spec/<feature>/reports/backtest-*.md` / `success-*.md` files.
  The fixtures-based / strategy-engine / report-rendering code
  paths that produce those bodies are **untouched** by this brief
  (no edits to `crates/{strategy,risk,backtest,reports,exec,audit,
  agent,core,reflection}`).  Hard gate at tester time:
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11/11)`.
- **v1.9.0 R-items (R7 v1.10.0) — confirmed by inspection.**
  Marker visuals (13-px triangle, BORDER_STRONG outline,
  whisper_shadow), ghost layer (60% alpha, _400 tier), draw order,
  volume tile + histogram + position mirror — all preserved
  verbatim.  Adding axes/legend is additive; it doesn't
  reshape, recolour, or resize any v1.9.0 marker.
- **Viewer parity (Q7=BOTH) — defends the viewer's existing 12
  snapshot baselines** by introducing the new axes additively.
  The viewer's existing snapshots will need updating (analyst-flagged
  "Type A churn"); the developer pass updates them atomically in M5.

### Determinism

- Axis-label derived state: yes (R4.4) — recomputed on every
  paint from `bars.first/last` + `price_range`.
- Local-time-offset injection: cfg(test) → UTC; production → OS.
- No new f64-in-money math (still `Decimal`).
- No new `SystemTime::now()` reachable from a backtest path
  (axis label TS comes from `Bar.open_ts` which is already
  injected-clock-driven on the strategy/backtest side).
- HashMap iteration: not introduced.
- ChaCha20Rng: not touched.

All five determinism non-negotiables from `AGENT.md` stay green.

---

## Design — M7 (re-spec follow-ups, 2026-05-12, architect)

This section closes **Q-revised-1** and **Q-revised-2** with operator-locked
decisions and lays down the contract the developer + ui-designer execute
against in **T3027 / T3028 / T3029**. M7 lives entirely in `crates/ui/`;
**zero changes** to `crates/{strategy,risk,backtest,reports,exec,audit,
agent,core,reflection}`; **zero workspace `Cargo.toml` edits** (the `time`
`local-offset` flag stays off — Q4 defers to v1.11). The 11 body-SHA-256
anchors are guaranteed byte-identical by code-path; `verify_anchors.sh`
runs as defence-in-depth at tester time.

### Resolved Qs — M7 deltas

- **Q-revised-1 (operator-locked) — Defer Q4 local-time to v1.11.** v1.10.0
  ships with UTC x-axis labels. The `local_offset_or_utc()` helper at
  [`chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs#L125-L160)
  continues to return `UtcOffset::UTC` unconditionally in both `cfg(test)`
  and production. The doc comment is rewritten to point to the v1.11
  follow-up brief slug `chart-x-axis-local-time` (candidate stub queued
  in [`spec/backlog.md`](../backlog.md#ui--cockpit-lumen-design-system-adoption--phase-6-reserved)).
  The workspace `time` dep stays as-is — no `local-offset` feature flip
  in this brief. *Rationale:* the operator's daily workflow is macOS
  Retina; the UTC fallback is correct-but-unfriendly; v1.11 owns the
  feature-flag flip plus the determinism contract (cfg(test) override
  stays UTC). Closes R10 by R10.4 (path (b)).
- **Q-revised-2 (architect-framework, ui-designer-empirical) — Legend
  chrome escalation ladder.** The legend card's `PANEL_RAISED` fill +
  `BORDER_1` outline reads as nearly invisible against `PANEL` at
  3360×1890 dark mode (evidence:
  [`/tmp/orch-diag/cockpit-final-charts.png`](/tmp/orch-diag/cockpit-final-charts.png)).
  The architect picks the **decision framework**; the ui-designer makes
  the **empirical call** at dev-pass time after capturing screenshot
  evidence at each rung of the ladder. Closes R9. See ## Legend chrome
  ladder below for the full contract.

### Legend chrome ladder (R9 / T3027)

The legend card's current chrome lives at
[`chart_legend.rs:99-118`](../../crates/ui/src/widgets/chart_legend.rs#L99-L118)
inside `draw_legend(frame, inner, mode)`. The fix is **chrome-only**:
the card geometry, glyph palette, row count, anchor arithmetic, and
strings all stay byte-identical (R9.3 keeps `cargo test -p ui --test
consistency` green; chrome-only changes don't touch
`legend_glyphs_use_marker_palette` / `legend_labels_route_through_strings`
/ `legend_card_dimensions_match_tokens`).

The visual sibling is `chart_tooltip::draw_tooltip` (see
[`chart_tooltip.rs:80-100`](../../crates/ui/src/widgets/chart_tooltip.rs#L80-L100))
— same `Frame::fill` + `Frame::stroke` chrome pattern against the same
`PANEL` background. Tooltips are visibly distinct from the chart panel;
the legend has the same canvas, same composition mechanics, and the same
chrome budget. The escalation ladder reuses the tooltip's existing token
choices first, then escalates only if empirical contrast fails.

**Rung (a) — Fill swap to `color::PANEL_DEEP` (alias for `PANEL_SUNKEN`).**
Drop one luminance tier below the current `PANEL_RAISED` and **below**
`PANEL` itself — the dark-mode tier ladder is `PANEL_SUNKEN (0x0B0F15) <
CANVAS (0x131820) < PANEL (0x1C2127) < PANEL_RAISED (0x2A3038)` per the
`tier_ladder_dark` pinning test in `theme.rs:922-934`. **Swap the fill
on `chart_legend.rs:112` from `color::PANEL_RAISED.current(mode)` to
`color::PANEL_SUNKEN.current(mode)`**; keep `BORDER_1` outline. This
makes the card read as a *recessed* well below the chart panel rather
than a *raised* card above it — both directions of luminance delta
create visual distinction; the recessed direction is more
discriminative against `PANEL` because the absolute delta
|PANEL.lum − PANEL_SUNKEN.lum| > |PANEL.lum − PANEL_RAISED.lum| (cool-700
vs cool-900 vs cool-600). Update the snapshot stub in `legend_summary`
at `chart_legend.rs:487` from `card_background: PANEL_RAISED` to
`card_background: PANEL_SUNKEN`. Atomic snapshot regen for
`chart_legend__composition_dark`.

**Rung (b) — Outline swap to `color::BORDER_STRONG` (matches tooltip).**
If (a) alone isn't sufficient (ui-designer empirical judgement at
3360×1890 dark + light), swap the outline on `chart_legend.rs:116`
from `color::BORDER_1.current(mode)` to
`color::BORDER_STRONG.current(mode)`. This brings the legend into chrome
parity with `chart_tooltip::draw_tooltip` at
[`chart_tooltip.rs:91`](../../crates/ui/src/widgets/chart_tooltip.rs#L91)
— same outline tier, same 1-px stroke width. The architect's original
Q5 resolution (under `## Design — Resolved Qs`) actually specified
`BORDER_STRONG` outline; the ui-designer's T3016 landing note flipped
it to `BORDER_1` for "quieter" chrome — that judgement is the regression
R9 surfaces. Reverting to `BORDER_STRONG` rejoins the architect's
original spec. Update `legend_summary` to
`card_border: BORDER_STRONG @ 1px`.

**Rung (c) — Add a `shadow::shadow_1` whisper drop shadow.** If (a) + (b)
combined still read as borderline (ui-designer empirical judgement),
add a single-layer drop shadow under the card. iced 0.14's
`canvas::Frame` doesn't expose `box-shadow` natively; the workaround is
the same one `draw_marker_with_shadow` uses for the markers — paint a
1-px-offset darker-fill rectangle behind the card before the card itself.
Reference implementation:
```rust
let shadow_color = shadow::shadow_1(mode).color;
let shadow_offset = Vector::new(0.0, shadow::shadow_1(mode).offset.y);
let shadow_path = Path::rectangle(
    Point::new(card_rect.x + shadow_offset.x, card_rect.y + shadow_offset.y),
    Size::new(card_rect.width, card_rect.height),
);
frame.fill(&shadow_path, shadow_color);
// then the card fill + outline as before
```
Update `legend_summary` to add `card_shadow: shadow_1`. Cost: one extra
`Frame::fill` per chart paint — negligible.

**Rung (d) — New `color::LEGEND_CARD_BG` token (last-resort only).**
Only if (a) + (b) + (c) combined still don't yield a clear luminance/edge
delta on the operator's 3360×1890 hardware. A new token lands in
`crates/ui/src/theme.rs` under `pub mod color`, sized one specific
luminance step (architect strawman: `dark = rgb(0x06, 0x0A, 0x10)`, the
darkest dark-mode surface in the Lumen ladder; light = a slightly tinted
warm grey distinct from `PANEL_SUNKEN`). The pinning test
`tier_ladder_dark` at `theme.rs:922-934` would extend to cover the new
token. **Cost:** one new token; the non-negotiable above caps M7's token
budget at **at most one new token**, and only via rung (d). UI-designer
records the empirical rationale in `chart_legend.rs`'s module-level
docstring at landing time. Update `legend_summary` to
`card_background: LEGEND_CARD_BG`.

**Empirical acceptance criterion (V14):** the chosen rung produces a
ui-designer-captured screenshot at 3360×1890 native Retina where, in
**both** dark **and** light theme modes, the legend card is *clearly
visually distinct from the chart's `PANEL` background* — defined as
**a perceivable luminance or edge delta visible at 1× viewing distance**
(no zoom-in needed to see where the card ends and the chart begins). The
1280×720-floor screenshot must show the same visual distinction; the
ladder's rungs all hold at every supported resolution because the
chrome is token-driven, not pixel-derived.

The ui-designer climbs the ladder rung-by-rung — start at (a), capture,
review; if still borderline, escalate to (a)+(b), capture, review; etc.
**Stop at the lowest rung that satisfies V14.** Reuse beats addition;
addition beats new tokens (Lumen discipline).

### Q4 deferral (R10 / T3028)

The `local_offset_or_utc()` helper at
[`chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs#L125-L160)
stays at its current UTC-only implementation. The doc-comment delta:

- Remove the "Q4 operator-locked intent (NOT YET LANDED)" framing — it
  reads as a developer escalation that the operator has since closed.
- Replace with a forward-pointer to the v1.11 follow-up brief slug
  `chart-x-axis-local-time`, citing
  [`spec/backlog.md`](../backlog.md) for the candidate entry.
- Keep the function signature exactly as-is (`pub(crate) fn
  local_offset_or_utc() -> time::UtcOffset`) so a v1.11 implementation
  flips only the body, not the call sites. The cfg(test) override
  contract stays: snapshot tests stay deterministic at UTC.
- Keep the existing unit test `local_offset_under_test_is_utc` green
  — it asserts the function returns `UtcOffset::UTC` exactly; the
  v1.11 brief will retire it atomically with the cfg(test) override
  landing.

**No production code change in T3028 beyond the doc comment.** No
`Cargo.toml` edit. No new test. The backlog candidate stub is the
gate-out artifact.

### R1 tooltip live-hover artifact (T3029)

The orchestrator cannot drive cursor moves from the sandbox without
macOS Accessibility permission. The operator's permission grant is
**in flight** as of 2026-05-12. T3029 ships a **two-track artifact
gate** so v1.10.0 can close V1/R1.2 regardless of which track
resolves first:

- **Track A — Automated (if Accessibility resolves by dev-pass time).**
  Developer probes via:
  ```bash
  osascript -e 'tell application "System Events" to get position of window 1 of (first process whose frontmost is true)'
  ```
  Success returns a numeric `x, y` pair; **failure returns
  `-1743 Not authorized` or `osascript: Accessibility permission
  required`**. On success, developer drives the cursor onto a fill-
  marker (▲ or ▼) via `osascript -e 'tell application "System
  Events" to set the position of the mouse cursor to {X, Y}'`, then
  captures via `screencapture -l <CGWindowID>` with the tooltip
  visible. The cockpit window ID comes from
  `osascript -e 'id of window 1 of application "cockpit"'`
  (or the equivalent CG window enumeration documented in
  `scripts/capture_screenshot.sh`). The artifact file lands at
  [`spec/chart-canvas-overhaul/reports/screenshots/m7-tooltip-hover-3360x1890.png`](reports/screenshots/).
- **Track B — Operator manual (if Accessibility still not granted).**
  Operator hovers a fill-marker on the running cockpit at 3360×1890
  native, observes the tooltip card, and captures via
  `screencapture -l <CGWindowID>` themselves. Drops the file at the
  **same path**:
  [`spec/chart-canvas-overhaul/reports/screenshots/m7-tooltip-hover-3360x1890.png`](reports/screenshots/).
  Developer documents the manual-track invocation in
  `## Implementation` at landing time.

**Either way the artifact path is identical**, the V1 / V14 / V15 gate
treats the screenshot byte-identical regardless of track. The tester
verifies via `sips -g pixelWidth -g pixelHeight` that the file is at
native pixel resolution (not a downscaled save). The pixel-width is
expected to be in the 3360×1890 range (account for window-manager
chrome — `sips` reports the exact native bitmap dimensions).

**Track-resolution probe (the dev pass MUST run this first):**
```bash
osascript -e 'tell application "System Events" to keystroke ""' \
  2>&1 | grep -qF '-1743' && echo "track-B (manual)" || echo "track-A (automated)"
```
If Track A fails, the dev pass routes immediately to Track B without
re-attempting Accessibility — the operator's grant either resolves
asynchronously (Track A re-eligible on the next dev-pass) or is
intentionally deferred. No retry loop.

### Risk register — M7 deltas

1. **Snapshot churn on rung (a) or rung (b).** Changing the legend's
   fill or outline token regenerates
   `chart_legend__composition_dark` via the `legend_summary` body
   (the snapshot includes `card_background: ...` and
   `card_border: ...` lines). **Mitigation:** ui-designer regenerates
   the snapshot atomically with the chrome change and ticks T3027 with
   the cite. The snapshot is text-only — no pixel churn risk.
2. **Token-budget creep at rung (d).** If the ui-designer reaches rung
   (d) and adds `LEGEND_CARD_BG`, every downstream Lumen token-pinning
   test extends (`tier_ladder_dark`, `tier_ladder_light`). **Mitigation:**
   the ladder is designed so rungs (a)–(c) almost certainly suffice;
   rung (d) is a documented last resort with a single new token, not a
   spree. UI-designer cites empirical evidence (screenshots at each
   rung) before reaching (d).
3. **Track-A automation drift.** macOS Accessibility's permission model
   changes between OS versions; the `osascript ... mouse cursor`
   pattern works on macOS 13+ but may shift on a future major bump.
   **Mitigation:** Track B is a permanent fallback; the dev pass's
   probe explicitly differentiates the two so the fall-through is
   automatic. No long-lived dependency on Accessibility-API stability.
4. **R10 deferral leaks into v1.11 scope creep.** The v1.11 follow-up
   brief `chart-x-axis-local-time` could accumulate scope (e.g., a
   "switch tooltip TS to local time too" ask). **Mitigation:** the
   backlog stub queues *only* the `local_offset_or_utc()` body flip
   + the cfg(test) override + one new unit test
   (`local_offset_under_production_reads_os_offset`). Anything broader
   spawns a separate brief; the v1.11 analyst polices the boundary.

### Backwards-compatibility — M7

- **Anchor invariant** — chrome-only legend change touches no
  strategy/audit/report rendering; 11 anchors stay byte-identical.
  `verify_anchors.sh` runs at tester time as defence-in-depth.
- **v1.9.0 R-items (R7 v1.10.0) — confirmed.** Marker visuals, ghost
  layer, draw order, volume + position widgets are untouched. M7
  edits live in `chart_legend.rs` (chrome) and `chart.rs` (doc
  comment) only.
- **Snapshot stability.** Only `chart_legend__composition_dark`
  regenerates if rung (a)/(b)/(c)/(d) lands. The
  `chart__btc_with_two_buys_one_sell` snapshot does **not** change
  because the chart's draw-order string at `chart.rs` doesn't reference
  the legend's fill/outline tokens — the draw-order line stays
  `gridlines,price_axis,time_axis,line,ghosts,fills,tooltip,legend`.
- **Determinism.** No new RNG, no new f64 money math, no new
  `SystemTime::now()` reachable from a backtest path, no HashMap
  iteration, no ChaCha20Rng touch. All five non-negotiables stay green.

### Ownership — M7

| Task   | Primary owner | Secondary owner | Role split                                                                 |
|--------|--------------|-----------------|----------------------------------------------------------------------------|
| T3027  | ui-designer  | developer       | UI-designer picks the rung empirically + lands chrome + regenerates snapshot; developer pairs only if rung (d) lands (new token + pinning-test extension). |
| T3028  | developer    | —               | Doc-comment update + backlog candidate stub (architect handles the stub; developer cross-links from `chart.rs`). |
| T3029  | developer    | operator        | Developer runs Track A if Accessibility resolves; operator captures Track B if not. Either way developer files the artifact + cites the path in `## Implementation`. |

The developer + ui-designer **fan out in parallel** for T3027 / T3028 /
T3029 — no cross-file conflicts (T3027 touches `chart_legend.rs` chrome
lines + snapshot; T3028 touches `chart.rs:125-160` doc comment lines;
T3029 touches `## Implementation` text + drops a PNG artifact). Shared
files: none (different functions inside `chart.rs` vs `chart_legend.rs`).
The dev/ui-designer parallel spawn pattern from AGENT.md §3 applies
verbatim.

---

## Implementation

### Corrected baseline (orchestrator-led, 2026-05-12)

After the developer pass landed M0 + T3004 + T3006 + T3009–T3022
and paused all canvas-scale-dependent tasks pending orchestrator
re-scope, the orchestrator ran the empirical probe documented in
`## Diagnostic — CORRECTED`. The corrected ship state is:

**Closed as no-op** (architect misdiagnosis retracted):
- **T3002** — canvas-scale spike — no defect to investigate.
- **T3003** — canvas-scale fix — no fix to land.
- **T3007** — `chart_tooltip_renders_at_retina_resolution`
  integration test — the original test was gated on the
  canvas-scale fix's geometry; with no fix needed, the gate
  reduces to a regular hover-integration test which
  `chart_tooltip_hover_fires` already covers.
- **T3008** — pre-fix R6 baseline screenshots — never needed
  (the architect's diagnostic screenshots already cover the
  baseline).

**Shipped cleanly in v1.10.0**:
- **T3001** — diagnostic-trace artifact (retain the log; the
  architect's `## Diagnostic` section stays as superseded
  history with a leading note pointing to the CORRECTED
  section).
- **T3004** — `chart_inner_rect_stays_within_canvas_bounds`
  invariant test landed.
- **T3006** — tooltip card clamp defence-in-depth landed.
- **T3009 / T3010 / T3011** — M2 price axis + tokens +
  `inner_rect_with_gutters` helper landed.
- **T3012 / T3013** — M3 time-axis adaptive tick count + draw
  pass landed. T3013 partial — UTC fallback only; see R10.
- **T3014** — strings skipped per architect default.
- **T3015 / T3016 / T3017** — M4 legend strings + module +
  wire-up landed. **Visibility bug discovered post-landing
  (R9).**
- **T3018** — panel-snapshot ui-designer-scoped (carried).
- **T3019 / T3020** — M5 viewer parity (equity_curve +
  drawdown_band) landed.
- **T3022** — M6 cockpit initial window-size bump to
  1920×1080 landed.

**New scope opened** (architect ratifies before developer
spawn):
- **T3027** — R9 legend visibility fix (UI-designer-decide
  via Q-revised-2 in Notes).
- **T3028** — R10 Q4 local-time follow-up
  (Operator-decide via Q-revised-1 in Notes).
- **T3029** — R1 tooltip live-hover verification
  (operator-blocked artifact: orchestrator cannot simulate
  hover without macOS Accessibility permission).

The developer's earlier escalation entry remains below for the
audit trail.

### T3002 — Canvas-scale spike (developer, 2026-05-12)

> **Outcome retracted 2026-05-12 (re-spec).** The spike's
> blocking-on-graphical-environment routing was correct given
> the information the developer had; the architect's
> Observation 2 hypothesis it tried to confirm is itself
> retracted (see `## Diagnostic — CORRECTED`). Entry retained
> verbatim below for audit-trail completeness.

**Result: INCONCLUSIVE — escalation to orchestrator.**

**Investigation:**

1. Confirmed iced version pin: `iced = =0.14.0` ([crates/ui/Cargo.toml:69](../../crates/ui/Cargo.toml#L69))
   with `iced_widget = 0.14.2` (patched) ([Cargo.lock](../../Cargo.lock) —
   `iced_widget` block).
2. Audited all `Frame::new` call sites in the workspace —
   five widgets all use the identical idiom
   `Frame::new(renderer, bounds.size())`:
   - [`crates/ui/src/widgets/chart.rs:209`](../../crates/ui/src/widgets/chart.rs#L209)
   - [`crates/ui/src/widgets/equity_curve.rs:111`](../../crates/ui/src/widgets/equity_curve.rs#L111)
   - [`crates/ui/src/widgets/drawdown_band.rs:109`](../../crates/ui/src/widgets/drawdown_band.rs#L109)
   - [`crates/ui/src/widgets/volume_histogram.rs:90`](../../crates/ui/src/widgets/volume_histogram.rs#L90)
   - [`crates/ui/src/widgets/sparkline.rs:69`](../../crates/ui/src/widgets/sparkline.rs#L69)
3. Reread architect's Diagnostic Observation 2 + hypothesis ranking:
   the architect identifies a canvas-scale rendering defect inside
   iced 0.14's `Frame::new` / `Geometry`-blit pipeline on macOS
   Retina (iced GitHub issue-class #2476 / #2640).  Three suggested
   fix paths in feature.md (Diagnostic / Observation 2):
   1. Re-derive frame size from a known-good source (newer API).
   2. Hard-clamp via explicit `Stroke::with_line_cap` / fill
      coordinates measured against a known-good rectangle, not
      against `bounds.size()`.
   3. Upgrade iced to a patch that fixes the canvas scale bug.
4. Reproduced the diagnostic image inspection: both
   `diag-cockpit-charts-launch.png` (4248×2528, maximised window
   logical 2056×1196) and `diag-cockpit-charts-current.png`
   (2696×1640, min-size window logical 1280×752) show chart line +
   gridlines + price labels confined to ~42 % of the canvas's
   allocated horizontal width, with price labels rendering at
   logical x ≈ 36–100 instead of the expected `bounds.x +
   inner.x + 4 ≈ 208`.  Symptom is consistent across both window
   states — the canvas-scale defect is real, not transient.

**Blocker: iced-source-access + graphical-test-environment unavailable.**

The 30-minute spike was unable to determine which of the three
architect-stated fix paths is correct without either:

- (i) **iced 0.14 source-tree inspection** to verify the
  `Frame::new(renderer, Size)` semantics and whether a newer-API
  alternative exists in 0.14.x — the cargo registry source path
  (`$CARGO_HOME/registry/src/...`) is outside the sandboxed file
  permission set granted to this developer agent, so
  `find`/`grep`-based source-level confirmation cannot run.
- (ii) **A graphical display environment to test fixes empirically.**
  The cockpit binary is a `wgpu`/`tiny-skia`-backed iced
  application; verifying any candidate workaround at 1280×720 /
  1920×1080 / 3360×1890 native Retina requires a live macOS
  display session.  The sandboxed agent cannot launch GUI apps to
  capture the post-fix screenshots that close R6.

**Escalation routing per the task contract:**

> "T3002 spike FIRST (30 min budget). Confirm the canvas-scale fix
> doesn't require an iced 0.14 → newer upgrade. If it DOES, STOP
> and escalate to orchestrator immediately — do not implement."

The spike's blocking information gap is upstream of the
implement-vs-escalate decision — without source access I cannot
distinguish (a) vs (b) vs (c) per the architect's own
risk-register item 1.  **HANDOFF → orchestrator (re-scope
decision: provide iced-source-access + macOS-display-session, or
re-route the canvas-scale fix to an agent with that
provisioning).**

**Work continuing in parallel (non-canvas-scale-dependent tasks).**

The architect's design separates several tasks from the canvas-
scale fix.  These can land WITHOUT closing R1+R2+R3 first because
they are either:

- pure-arithmetic helpers / unit-test surfaces;
- code paths that compile and pass tests but whose visual
  verification awaits the canvas-scale fix;
- new draw passes that ride atop whatever `Frame::new` semantics
  the canvas-scale fix lands on.

Concretely, this developer pass continues with: T3004
(`inner_rect_with_gutters` invariant unit), T3006 (tooltip card
clamp defence-in-depth), T3010 (`inner_rect_with_gutters` helper
in `canvas_chart.rs`), T3011 (price-axis draw pass code — visual
verification deferred to post-canvas-scale-fix), T3012 (time-axis
draw pass), T3013 (local-time-offset injection), T3019/T3020
(viewer-parity `equity_curve` / `drawdown_band` axis adoption —
same caveat), and T3022 (initial window size bump to 1920×1080).
M6 screenshot capture (T3023/T3024/T3025/T3026) is paused
pending the canvas-scale fix + graphical-display provisioning.

_Screenshots from R6 to be filed here as the canvas-scale fix
lands and the developer agent gains GUI capture capability._

---

## Acceptance

_Operator-facing acceptance summary (presenter-owned, 2026-05-12).
Distills the V-matrix above into the ship/defer decisions the
operator approves on the presenter deck at
[`presentations/chart-canvas-overhaul-2026-05-12.md`](presentations/chart-canvas-overhaul-2026-05-12.md)._

- **V1–V13** — **PASS** (inherited from the v1.9.0 V-suite + new
  workspace unit tests at developer landings of T3014/T3015/T3019;
  see `feature.md ## Implementation` for the cited file:line +
  test-output rows).
- **V14 — Legend card visually distinguishable (new, R9) — APPROVED.**
  Operator-visible evidence:
  - [`reports/screenshots/m7-legend-after-3360x1890.png`](reports/screenshots/m7-legend-after-3360x1890.png) —
    legend card visible top-right at native Retina (rung (a) +
    rung (b) chrome — `PANEL_SUNKEN` fill + `BORDER_STRONG` outline,
    per T3027 landing note at
    [`crates/ui/src/widgets/chart_legend.rs:156`](../../crates/ui/src/widgets/chart_legend.rs#L156)
    and
    [`crates/ui/src/widgets/chart_legend.rs:160`](../../crates/ui/src/widgets/chart_legend.rs#L160)).
  - [`reports/screenshots/m7-charts-screen-3360x1890.png`](reports/screenshots/m7-charts-screen-3360x1890.png) —
    full Charts screen at native Retina (operator's manual capture):
    axes visible, price line + markers rendered, legend card in
    place, status strip across the top, USD price labels in the left
    gutter, HH:MM UTC labels on the bottom time axis. Satisfies
    V2 + V4 + V5 + V6 + V9 in one frame alongside V14.
- **V15 — Tooltip-hover live screenshot (new, R10 / R1.2) — DEFERRED.**
  Per operator decision D4
  ([`spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`](../dev-notes/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)),
  V15 acceptance moves to the first
  `iced_test::Simulator::snapshot().matches_image()` chart-hover test
  at 3360×1890 in the `ui-test-harness-bootstrap` v0.1 feature
  (backlog entry
  [`spec/backlog.md ## Process / tooling`](../backlog.md#process--tooling)).
  That snapshot test replaces the manual screenshot artifact and
  closes the operator-blocked half of R6 by construction. Existing
  [`reports/screenshots/m7-tooltip-hover-3360x1890.png`](reports/screenshots/m7-tooltip-hover-3360x1890.png)
  is preserved as **informational evidence only** — no tooltip is
  visible in the frame because `Cmd+Shift+4` moved the cursor off
  the marker before the capture fired; orchestrator confirmed via
  Swift `CGWarp` cursor automation that hover-render dependency on
  window focus is what's blocking, not a code bug. R1 root cause
  remains "not pinned in v1.10.0; falsified in v0.1 of the test
  harness" — see
  [`## Diagnostic — CORRECTED ## Corrected conclusion`](#corrected-conclusion).
- **Anchors** — `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`
  (verbatim, run 2026-05-12 from presenter; non-UI crates untouched
  this cycle, so the result is invariant by inspection but the gate
  runs as defence-in-depth per AGENT.md §3).

**Frontmatter flip:** `status: in-progress → pending-approval`.
Operator approval on the presenter deck flips to `shipped`.

---

## Verification

_tester links to reports here at VERDICT time. Mandatory: the
three R6 screenshots cited inline, plus the body-SHA-256 anchor
output._

---

## Changelog

- 2026-05-12 (analyst): initial brief drafted. Six operator-reported
  items framed as R1-R6 (three regressions + three scope additions
  + the visual-verification gate); v1.9.0 R-items preserved as
  R7-R8 non-regression invariants. Nine open questions surfaced
  (three OPERATOR-DECIDE, six ARCHITECT-DECIDE).
- 2026-05-12 (architect): live-cockpit diagnostic pass landed under
  `## Diagnostic` (four Observations + three R6 screenshots filed
  under `reports/screenshots/` + raw trace at
  `reports/diagnostic-trace-2026-05-12.log`).  Root cause for R1 +
  R2 collapsed onto a single canvas-scale rendering defect
  (Observation 2).  Q2/Q3/Q5/Q6/Q8 resolved per architect judgement;
  Q1/Q4/Q7 honoured as operator-locked.  `## Design` written
  (component decomposition, six new Lumen layout tokens, top-3
  risk register, viewer-parity scope explicit).  Ownership flipped
  to architect; tasks.md re-fanned-out under M0–M6 + M_FINAL in
  the sibling task file.
- 2026-05-12 (developer): T3002 spike INCONCLUSIVE — escalation to
  orchestrator filed in `## Implementation`.  The 30-min spike
  could not distinguish iced 0.14 fix paths (a) / (b) / (c)
  without iced-source-tree access + a graphical macOS display for
  empirical fix testing.  Continuing in parallel with the
  non-canvas-scale-dependent tasks: T3004, T3006, T3010, T3011,
  T3012, T3013, T3019, T3020, T3022.  M1 visual verification
  (R1+R2+R3) and M6 R6 screenshot capture (T3023-T3026) paused
  pending orchestrator re-scope.  Ownership flipped to developer.
- 2026-05-12 (orchestrator → analyst re-spec): empirical
  red-rect + cyan-dot probe on the operator's 3360×1890 native
  Retina **disproved** the architect's Observation 2 canvas-
  scale hypothesis. New `## Diagnostic — CORRECTED` section
  added; original `## Diagnostic` marked SUPERSEDED but
  retained verbatim for audit trail. R2 marked
  operator-RESOLVED (M2/M3 axes closed the cropping symptom).
  T3002/T3003/T3007/T3008 close as no-op in `tasks.md`. Two
  real bugs surfaced post-developer-pass: **R9** (legend card
  visually invisible against `PANEL` background — orchestrator
  clean-tree screenshot at `/tmp/orch-diag/cockpit-final-charts.png`)
  and **R10** (Q4 local-time partial landing — `time` crate
  `local-offset` feature gate). Two new operator/UI-designer
  questions filed in Notes (Q-revised-1, Q-revised-2). Three
  new tasks added: **T3027** (legend visibility fix), **T3028**
  (Q4 local-time follow-up), **T3029** (tooltip live-hover
  verification). Ownership flipped to analyst pending operator
  resolution of Q-revised-1/2 → architect spawn.
- 2026-05-12 (architect M7 design pass): `## Design — M7` section
  landed below the original `## Design` and above `## Implementation`.
  Q-revised-1 closed as **defer-to-v1.11** (UTC ships in v1.10.0;
  candidate stub `chart-x-axis-local-time` queued in
  [`spec/backlog.md`](../backlog.md)). Q-revised-2 framed as a
  four-rung chrome ladder (a) `PANEL_SUNKEN` fill → (b)
  `BORDER_STRONG` outline → (c) `shadow_1` whisper → (d) new
  `LEGEND_CARD_BG` token, with the ui-designer empirically picking
  the lowest sufficient rung at dev-pass time. T3029 specified as a
  two-track artifact gate (Track A automated `osascript` cursor +
  `screencapture -l`, Track B operator-manual capture; both paths
  produce the same artifact at
  `reports/screenshots/m7-tooltip-hover-3360x1890.png`). Ownership
  flipped to architect → handoff to dev ‖ ui-designer parallel
  spawn.
- 2026-05-12 (presenter): `## Acceptance` section added. **V14
  APPROVED** based on the M7 screenshot pair
  (`m7-legend-after-3360x1890.png` + `m7-charts-screen-3360x1890.png`).
  **V15 DEFERRED** to `ui-test-harness-bootstrap` v0.1 per operator
  decision D4 in
  [`spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`](../dev-notes/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)
  — first `iced_test::Simulator::snapshot().matches_image()`
  chart-hover test at 3360×1890 in that feature becomes the new V15
  acceptance artifact. Existing `m7-tooltip-hover-3360x1890.png`
  preserved as informational evidence (no tooltip visible — cursor
  moved off the marker before `Cmd+Shift+4` fired; not a code bug
  per orchestrator's Swift `CGWarp` automation falsification).
  Frontmatter flipped `status: in-progress → pending-approval`,
  `owner: architect → presenter`. Presenter deck written at
  [`presentations/chart-canvas-overhaul-2026-05-12.md`](presentations/chart-canvas-overhaul-2026-05-12.md);
  operator approval there flips status to `shipped`.
- 2026-05-12 (operator): **SHIPPED**. Operator approval recorded
  in [`presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](presentations/chart-canvas-overhaul-2026-05-12.md#approval)
  as `[x] Approved — ship`. V1-V13 PASS, V14 APPROVED, V15
  DEFERRED to `ui-test-harness-bootstrap` v0.1. Frontmatter
  flipped `pending-approval → shipped`; `owner: presenter →
  shipped`.
