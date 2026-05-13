---
adr: 0020
title: Chart buy/sell emphasis (v1.9) — resolutions (Q1–Q3, Q6, Q7, Q9)
status: accepted
date: 2026-05-10
supersedes: none
superseded-by: none
---

# ADR-0020: Chart buy/sell emphasis (v1.9) — resolutions (Q1–Q3, Q6, Q7, Q9)

## Context

v1.9 adds buy / sell markers to the cockpit chart canvas, a per-bar
volume histogram, and on-marker tooltips. Six architect-questions
(Q1–Q3, Q6, Q7, Q9 — the others were UI-designer-resolved) covered
signal source plumbing, marker placement math, tooltip
implementation, marker visual treatment, the histogram widget
boundary, and `SignalView`'s placement. UI feature with zero
anchor impact on strategy/backtest paths.

## Decisions

### Q1 — Signal source plumbing: additive `strategy_signals` table

Option (a): a new additive `strategy_signals` table in the audit
DB plus a polled reader. The cockpit polls every N seconds (default
1s) for new signals since the last poll cursor. Config-gated; off
by default for backtests. No new bus channel — signals are derived
data, not events.

### Q2 — Marker y-snap method: linear interpolation

Option (b): linear interpolation between the open and close of the
bar in which the signal fired, weighted by the signal's
sub-bar-resolution timestamp. The exact `y` value is deterministic
from `(bar.open, bar.close, signal.ts, bar.ts, bar.duration)` — no
mid-price snapshot required.

### Q3 — Tooltip implementation: custom canvas pointer-tracking

Option (b): custom canvas pointer-tracking + custom-drawn tooltip
overlay (not iced's stock tooltip). Stock tooltip can't render
multi-line markdown-ish content and has fixed positioning that
doesn't track the chart's coordinate system. The custom impl
shares the existing chart-canvas hit-test grid.

### Q6 — Marker visual treatment: dual-layer

Each marker is a dual-layer treatment:

- **Fill layer**: 13-px filled triangle, 1-px `BORDER_STRONG`
  outline, `shadow_1`-derived drop shadow. Uses the Lumen tokens
  from [ADR-0018](0018-lumen-phase-1-foundation.md).
- **Ghost layer**: 8-px triangle, `UP_400` / `DOWN_400` colour
  (semantic ramp from ADR-0018), 60% alpha, no outline / shadow.
  Renders behind the fill layer as a faint trail of past markers
  within the visible window.

### Q7 — Per-bar histogram widget shape

Option (b): a new `widgets::volume_histogram` widget. Reuses the
existing chart-canvas coordinate system but renders independently
in its own row beneath the price chart. Allows per-bar volume to
share the chart's x-scale without crowding the price plot.

### Q9 — `SignalView` placement: `crates/core/src/views.rs`

`SignalView` lives in `crates/core/src/views.rs` as a sibling of
`FillView`. Per the cross-crate placement rule
([ADR-0012](0012-v05-broadcast-bus-extensions.md)): if audit
produces and UI consumes, the type lives in `trading_core`. Fields:
`{ ts, symbol, strategy_id, side, score?, reason? }`.

## Alternatives considered

- **Bus channel for signals** (Q1). Adds bus capacity pressure and
  forces strategies to publish in addition to journaling. Polling
  from the audit table keeps the writer surface minimal. Rejected.
- **Snap markers to bar close** (Q2). Coarser visual; misleading
  for sub-bar signals. Rejected.
- **Stock iced tooltip** (Q3). Limited content + positioning.
  Rejected.
- **`SignalView` in `crates/strategy`** (Q9). Would force `audit
  → strategy` reverse dep. Rejected.

## Consequences

- The `strategy_signals` table is the first read-mostly audit
  surface where polling beats event-driven; precedent for any
  future cockpit consumer of historical (not live) audit data.
- The dual-layer marker treatment is the precedent for any future
  chart annotation (e.g. v2 LLM-strategy reasoning callouts).
- `SignalView` becomes the third major view type in `trading_core`
  alongside `FillView` and `JournalEntryView`. The pattern of
  view-types-in-core for audit-produced UI-consumed shapes is now
  well-established across three independent features.

## Changelog
- 2026-05-10 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Chart buy/sell emphasis (v1.9) resolutions during Phase 1A
  Session 10.
