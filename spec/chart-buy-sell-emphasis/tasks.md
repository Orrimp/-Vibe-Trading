---
slug: chart-buy-sell-emphasis
status: in-progress
owner: analyst
updated: 2026-05-10
---

# Tasks — Chart buy/sell emphasis

High-level **milestones only** for now. The architect expands each
milestone into developer tasks (working name **T20xx**, but the
architect picks the exact range after Q-resolution; the next free
range after T19xx — claimed by `v2-llm-strategy` — is T20xx unless
the architect prefers a different counter) once Q1–Q9 are resolved
and Q4 / Q5 / Q8 are operator-confirmed.

Anchored to the R-items in
[`feature.md`](feature.md). Each milestone closes one or more R-items
and lands behind one or more V-items. **No file-level task fan-out
until architect Design** — premature task enumeration would re-litigate
the open questions (especially Q1 signal source plumbing, which
shapes M3 entirely).

## Milestones

### M1 — Marker visual fixes (size + outline + z-order + line-snap)

Pure rewrite of
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs).
Closes **R1** (markers obvious), **R2** (above the line), **R3**
(snapped to line), **R6** (keep triangle). Verified by **V1**, **V2**,
**V9** (existing snapshot churn expected on this milestone), **V10**
(determinism).

- Bump `MARKER_SIZE_PX` per Q6 architect resolution.
- Add `BORDER_STRONG` outline draw pass.
- Re-order `ChartProgram::draw` so the marker fill runs after the
  line stroke.
- Replace `fill.price.get()` y-source with the polyline-interpolation
  y-snap helper (Q2 architect resolution picks (a) or (b)).
- Optional drop-shadow pass per Q6 architect resolution.

**Blocked on:** Q2 (y-snap method), Q6 (visual treatment) — both
`[ARCHITECT-DECIDE]`.

### M2 — Tooltip subsystem + click-through-to-modal

New widget + cockpit state + message arms + integration test. Closes
**R4** (hover tooltip + R4.5 click → modal). Verified by **V3**
(hover), **V4** (click → modal), **V9**, **V13** (consistency).

- New `widgets::chart_tooltip` (or in-canvas overlay if Q3 picks
  (b) — architect chooses widget vs canvas).
- `Cockpit.chart_tooltip: Option<ChartTooltipView>` plus
  `Message::ChartMarkerHovered(usize)` /
  `Message::ChartMarkerHoverEnded` arms.
- Wire marker click to the existing
  `Message::TapeRowClicked(transaction_id)` arm (reuses
  tape-row-audit-modal's shipped wiring).
- New `ui::strings::CHART_TOOLTIP_*` constants per R4.7.
- New panel snapshot `chart_tooltip_buy_paper_fill.snap`.

**Blocked on:** Q3 (tooltip implementation shape), Q4 (tooltip
content fields — operator confirms strawman).

### M3 — Signal source + layered render (ghost + fill)

The load-bearing milestone for this feature. Closes **R5**
(layered marker source), **R9** (no new bus channels in recommended
path), **R10** (consistency). Verified by **V5** (ghost+fill render),
**V11** (new audit reader), **V12** (config-gate default-off).

- **Architect's Q1 resolution determines the shape of this entire
  milestone**:
  - **If Q1 = (a) new audit log row:** new audit migration, new
    `audit::query::recent_signals` reader, new `core::SignalView`
    type, new `enable_signal_log` config field (default `false`),
    new cockpit `chart_signals: PanelState<Vec<SignalView>>` field +
    `Message::ChartSignalsLoaded` arm.
  - **If Q1 = (b) in-memory buffer:** new bus channel (caveat: the
    "no new bus channel" hard-constraint analyst-flagged in `## Why`
    re-litigates here), cockpit ring buffer, no audit row.
  - **If Q1 = (c) backtest-replay-only:** ghost layer is gated on
    cockpit's viewer-mode flag; live mode renders no ghosts;
    milestone shrinks to a cockpit-only render arm.
- Layered draw-order pass (gridlines → labels → line → ghosts →
  fills) — overlaps with M1's re-order work; architect sequences
  M1 ↔ M3 carefully.
- Ghost-marker tooltip variant per R5.6.

**Blocked on:** Q1 (signal source — load-bearing), Q9 (`SignalView`
shape).

### M4 — Counter views (tile + histogram + position mirror)

New widgets + Charts-screen layout reshape. Closes **R7** (three
counter views), **R8** (layout reshape). Verified by **V6** (tile
arithmetic), **V7** (snapshot), **V9**, **V10**.

- New `widgets::volume_tile` (or `kpi_strip` reuse — Q5 +
  architect picks).
- New `widgets::volume_histogram` per Q7 architect resolution (or
  extended `widgets::sparkline` if architect picks reuse).
- Reuse `widgets::positions` filtered to the active symbol per
  R7.3.
- Charts-screen reshape per Q5 operator resolution (analyst
  recommends layout (β)).
- New panel snapshot `charts_screen_with_counters_and_chart.snap`.

**Blocked on:** Q5 (layout — operator), Q7 (histogram widget shape
— architect).

### M5 — Ship gate

Tester runs the full V-pass. Closes nothing new; gates the entire
feature. Verified by **V8** (anchors 11/11), **V9** (workspace
green), **V10** (determinism), **V13** (consistency).

- `cargo test --workspace` green.
- `cargo test -p ui` + `cargo test -p ui --features live` green.
- `cargo test -p audit recent_signals` green (Q1 = (a) path only).
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`,
  zero diffs.
- New snapshots determinism-checked (two consecutive runs
  byte-identical).
- HANDOFF → presenter (operator approval gate per the standard
  spec-driven flow).

**Blocked on:** M1–M4 all green.

## Task expansion

Tasks **T20xx** (or whichever range the architect picks — next free
after T19xx held by v2-llm-strategy) will be expanded by the architect
after Q-resolution, in the `## Design` section of
[`feature.md`](feature.md) and below this milestone list. Expected
task count at architect-pass:

- **M1:** 3–4 tasks (size/outline/order edit, y-snap helper,
  consistency-test refresh, snapshot re-bless).
- **M2:** 4–5 tasks (`Message::*` arms, tooltip widget/canvas,
  click-to-modal wiring, strings constants, snapshot +
  integration test).
- **M3:** 6–10 tasks depending on Q1 resolution — Q1=(a) is the
  largest (migration + audit reader + core type + config gate +
  cockpit state + ghost-render pass + tooltip variant); Q1=(c) the
  smallest (cockpit viewer-mode gate only).
- **M4:** 4–6 tasks (volume-tile widget, histogram widget,
  positions-filter helper, screen-layout reshape, snapshot).
- **M5:** 1 tester task + `T_FINAL_CHART_BUY_SELL_EMPHASIS`.

**Estimated total: 18–26 tasks**, conditional on Q1 resolution.

## Parallelism hints (analyst's prior — architect confirms)

- **M1 ‖ M2** — different code paths (chart.rs marker draw vs new
  tooltip widget + state.rs Message arms). Architect can fan out
  these two milestones in parallel if the y-snap helper from M1
  doesn't ripple into M2's hit-rect math.
- **M3 critical-path.** Q1 = (a) adds an audit migration; migrations
  in this project are sequential and one-task-at-a-time. M3 likely
  blocks M4's start if M4's tile-arithmetic reads from `chart_signals`
  for any sub-feature (e.g. "include ghost-signal-implied volume
  alongside fill volume" — not in the current R7 strawman but
  operator may extend at Q4 resolution time).
- **M4 ‖ M3** — possible if M4 reads only from `chart_markers`
  (which already exists). Default-on parallelism in the analyst's
  prior; architect Design confirms.
- **M5 blocks on M1+M2+M3+M4 all green** (tester contract).

## Owner tags (analyst's prior)

- **M1:** `[ui-designer]` — pure widget edit.
- **M2:** `[ui-designer]` for widget + snapshot; `[developer]` for
  the `Message::*` exhaustive arms in `state.rs`.
- **M3:** `[developer]` for migration + audit reader + config field;
  `[ui-designer]` for the ghost-render pass + tooltip variant.
- **M4:** `[ui-designer]` predominantly; `[developer]` for any
  derived-state helper that lives outside `widgets/`.
- **M5:** `[tester]` — sole owner, runs the V-pass.

Architect confirms at Design time.
