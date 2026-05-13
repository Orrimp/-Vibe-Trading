---
adr: 0011
title: v0.5 — Strategies panel in right column of cockpit, above Open positions
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0011: v0.5 — Strategies panel in right column of cockpit, above Open positions

## Context

v0.5 adds a Strategies panel to the live cockpit: which strategies
are running, last swap event, signal count in the last 60s. The
cockpit's existing four panels (P&L, feed-latency badge, "Stop
trading", Open positions, Live tape) have established operator muscle
memory from v0; the new panel needs a placement that doesn't disrupt
that.

## Decision

Right column, above "Open positions", in a new `StrategiesPanel`
widget. The existing left column (P&L card, latency badge, "Stop
trading" button) is action-oriented; the right column (Open
positions, Live tape) is observation-oriented. Strategies are
observation-oriented — they pair naturally with positions.

```
┌──────────────────────────────────┬─────────────────────────────────────┐
│  P&L                             │  Strategies (v0.5 new)              │
│  Feed latency                    │  Open positions                     │
│  Stop trading (destructive)      │  Live tape                          │
└──────────────────────────────────┴─────────────────────────────────────┘
```

Architect fixes the panel's column position and the
Model/Message surface. Final widget composition (column widths, row
heights, padding) is ui-designer's call — see
[`../v05-composed-strategies/feature.md#design`](../../v05-composed-strategies/feature.md#design).

## Alternatives considered

- **Left column next to kill switch (analyst's initial suggestion).**
  Co-locating a passive observation panel with a destructive action
  crowds the decision surface and creates visual competition between
  "look at this" and "do this". Rejected.
- **Full cockpit re-wireframe.** v0 layout is stable and the operator
  has muscle memory on the four existing panels. Additive placement
  keeps cognitive load low. Rejected.
- **Separate top-level window.** The strategies panel is part of the
  cockpit's live view, not a standalone tool. The `viewer` binary is
  the offline-report surface. Rejected.

## Consequences

- The kill switch stays the biggest thing in the left column, which
  protects operator muscle memory from v0.
- Future passive observation panels (e.g. v1+ regime-detection state,
  v2 LLM-strategy reasoning trace) follow the same column-discipline
  rule: observation in the right column, destructive action in the
  left.
- Final pixel-level placement is delegated to ui-designer in the
  feature file. This ADR locks only the cross-feature
  column-discipline rule.

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `spec/architecture.md` § v0.5 — cockpit strategies panel layout
  (Q4) during Phase 1A Session 6 (2026-05-13). Source link
  `features/v05-composed-strategies.md` rewritten to
  `../v05-composed-strategies/feature.md` (post-folder-migration
  path).
