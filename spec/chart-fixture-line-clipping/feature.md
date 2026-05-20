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

## Hypothesis seeds (analyst owns falsification)

1. **H-CHART-1 — bar buffer underseeded.** `crates/ui/src/bin/cockpit.rs:190`
   seeds 60 bars per symbol via `synthetic_candles`. `CHART_BUFFER_CAPACITY`
   is 60. But the actively-displayed symbol's bars may only be the last
   ~22 due to a venue/symbol key mismatch in `ChartBuffer.push_bar`
   `(bar.venue, bar.symbol.clone())`.
2. **H-CHART-2 — line draw pass iterates only visible-y-range bars.**
   The y-axis auto-scale in `chart_canvas.rs` might compute the price
   range from a subset (e.g. last N bars) and clamp earlier bars'
   line segments off-canvas. The bars exist; they're just rendered
   outside the visible y-band.
3. **H-CHART-3 — `x_for_index` uses a different `count` than `bars.len()`.**
   If `count` reflects "expected bars" (60) but the actual `bars`
   Vec is shorter, indices 0..len() get plotted in the LEFT portion
   of the inner rect — opposite of what the operator sees. **Probably
   not this one** based on symptom direction.
4. **H-CHART-4 — chart only draws lines between consecutive markers
   that share a strategy-active window.** If a "strategy active"
   window starts at bar idx ~38, only bars 38-59 get the price-line
   pass. **Unlikely** — the chart should draw price for every bar.
5. **H-CHART-5 — `BarReceived` evicts bars due to a stale
   `chart_buffer.series` HashMap key.** If a re-keyed symbol causes
   `push_bar` to create a NEW deque, the first batch is lost. The
   fixtures boot path sends bars BEFORE the operator selects a
   symbol — the resulting `(Venue::Binance, "BTCUSDT")` key gets
   60 bars; but maybe the chart later renders against a different
   `(venue, symbol)` tuple that has fewer bars.

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
