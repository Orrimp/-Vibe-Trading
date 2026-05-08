---
slug: lumen-phase-6-assistant-slot
status: reserved
owner: analyst
updated: 2026-05-04
version: 2.5.0
---

# Lumen Phase 6 — Assistant slot (forward-compat reservation only)

> **Phase 6 of 6** in the
> [`lumen-design-adoption`](lumen-design-adoption.md) initiative.
> Master roadmap is the orientation; this brief is the **shippable
> feature** at the time it lands. Operator-locked constraints apply
> without re-litigation.
>
> **Status: reserved.** Was originally Phase 4 in the pre-2026-05-04
> roadmap; renumbered to Phase 6 at the roadmap revision. **Not
> implemented in this initiative.** Phase 6's full scope ships with
> the v2 LLM strategy; until then the lumen-design-adoption
> initiative records only the slot reservation.

## Why

The Lumen `Assistant.jsx` and Shell `right-side AI assistant panel`
is **opt-in, collapsible, and remembers state**
([`spec/design/project/README.md:131`](../design/project/README.md)).
The execution-mode toggle in Phase 5's HumanControl
(Observe / Supervised / Auto) maps directly onto the v2 LLM gate —
Supervised = trade-by-trade approval, Auto = within-envelope
autonomy, Observe = paper only. The Assistant slot is the surface
the operator uses to converse with the v2 LLM strategy at decision
time.

## Scope at the time it lands (with v2 LLM)

- **Right-rail collapsible panel slot** in the shell, hidden by
  default, revealed when the v2 LLM strategy is enabled.
- **Composer + message-list widget pattern** aligned to
  [`spec/design/project/ui_kits/desktop/Assistant.jsx`](../design/project/ui_kits/desktop/Assistant.jsx).
- **Wires into the v2 LLM trait** the architect defines at v2
  kickoff.
- **Coexists with the Phase 2 sidebar nav** (assistant rail on
  the right; nav on the left). Phase 2 must not consume the right
  column-track.

## Scope at the lumen-design-adoption initiative time (NOW)

**Zero shipped UI.** Phase 2's shell grid reserves the right-rail
column-track in advance — see
[`lumen-phase-2-shell-ia-charts.md`](lumen-phase-2-shell-ia-charts.md)
for the column-track contract. No widget, no module, no token; just
the layout reservation in the Phase 2 shell. The
[architecture.md](../architecture.md) Frontend section gets a two-
line forward-compat note documenting the slot at Phase 2 landing.

## Anchor risk

Out of scope — not implemented here. When the v2 LLM strategy
analyst spawns Phase 6, anchor risk is reassessed at that time.

## Promotion trigger

Phase 6's adoption brief lands when **v2 LLM is approved**. The v2
LLM is its own queued backlog item with its own analyst /
architect / developer pipeline; lumen-design-adoption considers
Phase 6 "reserved", not "deferred", until v2 LLM kicks off.

## Open questions (deferred until promotion)

- All Phase 6 design questions defer to v2 LLM kickoff. Master
  roadmap's Q9 (forward-compat pre-reservation breadth) was
  answered "lazily" at Phase 1 kickoff — Phase 2's shell grid
  reservation will be the minimum viable hook.

## Acceptance criteria

- Phase 6 has no acceptance criteria at lumen-design-adoption
  initiative time. The v2 LLM analyst writes them at promotion.

## Changelog

- 2026-05-04 (analyst, master-roadmap revision): stub created at
  the 6-phase roadmap revision. Replaces the Phase 4 reservation
  in the pre-revision master roadmap. Renumbered Phase 4 → Phase 6.
  Status remains `reserved` — no analyst spawn until v2 LLM is
  approved.
