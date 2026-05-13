---
adr: 0018
title: Lumen design adoption — Phase 1 foundation (Q1–Q11)
status: accepted
date: 2026-05-04
supersedes: none
superseded-by: none
---

# ADR-0018: Lumen design adoption — Phase 1 foundation (Q1–Q11)

## Context

Phase 1 of the four-phase Lumen design-system adoption rewrites the
UI theme tokens, surface system, motion ladder, and focus-ring
treatment. Architect-round answered eleven Q's (9 design + master
Q10 + mid-phase Q11) covering the token rewrite, surface tier system,
shadow ladder, focus ring, spacing/radii/typography ladders, status
bar, principles-doc consolidation, and the iced 0.14.2 API gap for
the keyboard-focus ring. UI-only feature with zero anchor impact by
construction.

## Operator-locked constraints (master Q10)

- **No brand adoption.** No `"Lumen"` string, no logo, no wordmark.
  Cockpit binaries stay `cockpit` / `cockpit_live`.
- **No `ui::strings` rewrite.** Voice rules unchanged. Net-new
  status-bar prose constants are additive, not a rewrite.
- **No icon adoption.** Lucide stays deferred per the principles
  doc's "no icons until needed" rule.
- **Sequential phasing.** Phase 2 promotes only on Phase 1 ship +
  operator approval; same for Phase 3 / 4.

## Decisions

### Token system: replace 12 with ~50 (hard-replace, Q1)

`crates/ui/src/theme.rs` is rewritten in T1501 to ship the full
Lumen palette per `spec/design/project/colors_and_type.css`:

- **Surface tokens** (`CANVAS`, `PANEL`, `PANEL_RAISED`,
  `PANEL_SUNKEN`, `OVERLAY`) keyed to the Tier system.
- **Foreground tokens** (`FG_1` primary, `FG_2` secondary, `FG_3`
  tertiary/labels, `FG_4` placeholder, `FG_ON_ACCENT`).
- **Accent ramp** (`ACCENT`, `ACCENT_HOVER`, `ACCENT_PRESS`,
  `ACCENT_SOFT`). Single muted-teal accent (`#6FB6AE` dark /
  `#3F968D` light); colour shifts from the existing blue `#5EA3FF`.
- **Semantic ramps**: `UP_{50,400,500}` (sage), `DOWN_{50,400,500}`
  (terracotta), `WARN`, `INFO`, `BORDER_STRONG`.

Flat `theme::color::*` `SHOUTY_SNAKE_CASE` (Q10) — no nested
modules. Single source of truth: if `theme.rs` doesn't export it,
the design system doesn't have it.

### Surface tier system (Q2)

Five tiers (canvas → panel → panel_raised → panel_sunken → overlay)
with explicit elevation rules: a Tier-N surface can only be placed
on a surface of Tier ≤ N-1. Compile-time enforcement is impractical
in iced; the rule is reviewer-enforced and documented in the
principles doc.

### Whisper-shadow ladder (Q3) + focus-ring approximation (Q11)

13-step shadow ladder (`shadow_0` … `shadow_12`). The iced 0.14
`Shadow` API confirmed via `iced_core-0.14.0/src/shadow.rs`;
implementation lands as inline `iced::widget::Shadow` per token.

**iced 0.14.2 API gap** (mid-phase Q11): `button::Status` has no
`Focused` variant and `text_input::Style` has no `shadow` field.
T1504's true keyboard-focus-ring acceptance is unachievable under
the shipped framework. **Option A ratified** — Phase 1 ships
hover-state ring on buttons + `ACCENT` border-shift on focused
inputs as a bounded best-effort approximation. T1504 tick stands
as honest under iced 0.14.2 API gap. Phase-N follow-up filed in
`features/lumen-design-adoption.md` (upgrade trigger: iced version
bump exposing `button::Status::Focused` + `text_input::Style.shadow`,
OR custom-widget approach — Phase-2-or-later).

### Spacing, radii, typography ladders (Q4–Q6)

- 13-step spacing ladder (4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 /
  64 / 80 / 96 / 128 px).
- 6-step radii ladder (0 / 2 / 4 / 8 / 12 / 16 px).
- 7-step typography ladder (`caption` / `body_sm` / `body` /
  `body_lg` / `title_sm` / `title` / `display`).

All Lumen palette / numbers come from `spec/design/project/`
verbatim — the bundle is the source-of-record for future token
additions; `theme.rs` is the executable contract for what ships
today.

### Motion ladder (Q7)

Motion timings: `motion_fast` (120ms), `motion_normal` (200ms),
`motion_slow` (320ms). Default easing `ease-out-cubic`. No bespoke
animations in Phase 1; ladder ships ready for Phase 2 panel
transitions.

### Status bar (Q8)

New `widgets::status_bar` consumes existing `bus.market_health()`
producer — purely additive subscriber; no producer-side change.
Surfaces per-venue freshness (post-[ADR-0017](0017-v15b-multi-venue.md))
plus uptime indicator. Status bar lives at the bottom of the cockpit
window; principles-doc-mandated row height = 24px.

### Principles-doc consolidation (Q9)

Single-file `spec/ui-design-principles.md` supersedes the prior
multi-file split. ~480 lines covering: surface tier rules, P&L
coloring, latency bands, kill-confirm phrase pattern,
flash-on-update, charts with audit-anchored markers, sidebar IA,
screen-routing rules, motion timings, density rules. UI-designer
agent treats this file as the prose contract (the executable
contract is `theme.rs`).

## Anchor risk: zero by construction

UI-only feature; no `crates/strategy/audit/exec/backtest/reports/`
touched. 11 / 11 anchors verified byte-identical post-Phase-1 ship.
Cross-feature invariants for the 7 prior shipped features
preserved (re-grepped at architect-round time).

## Alternatives considered

- **Adopt the "Lumen" brand string in the binary names.** Operator
  locked against. Rejected.
- **Rewrite `ui::strings`.** Out of Phase 1 scope per operator lock.
  Deferred.
- **Custom widget for true keyboard focus ring at Phase 1.** Triples
  Phase 1 scope. Deferred to Phase 2+ with iced upgrade trigger.
- **Multi-file principles-doc retained.** Cross-reference cost too
  high; single file wins for the v0.5-style operator success report
  agents.

## Consequences

- `crates/ui/src/theme.rs` is the executable contract. When
  `theme.rs` and `spec/design/` diverge, **theme.rs wins** — it's
  the shipped reality; the bundle is the contract that gates
  future token additions.
- The ui-designer agent (`.claude/agents/ui-designer.md`) cites
  three design-system artefacts: `theme.rs`,
  `ui-design-principles.md`, and `spec/design/`. ADR-0018 codifies
  that hierarchy.
- The "Phase-N follow-up filed" pattern (Q11 — bounded best-effort
  with a documented upgrade trigger) is the precedent for any
  framework-API gap that hits at ship time.

## Changelog
- 2026-05-04 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Lumen design adoption — Phase 1 foundation resolutions during
  Phase 1A Session 10.
