---
slug: ui-rethink-phase-d-trail
status: proposed
owner: pending-analyst
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-c-sidebar-ia v0.1.0
---

# UI rethink Phase D — Trail view (J4)

> Fourth concrete feature carved out of
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/ui-rethink-2026-05-17.md).
> Dev-note §6 Phase D is the **scope source-of-truth**; this brief is
> the **implementation contract**. Predecessor:
> [`ui-rethink-phase-c-sidebar-ia v0.1.0`](../ui-rethink-phase-c-sidebar-ia/feature.md)
> shipped 2026-05-20. The new 3-group sidebar (Phase C) already
> reserves a `Trail` entry under the Library zone (currently a Phase
> A alias to `Screen::Audit`). Phase D makes the entry meaningful.

## Why

The cockpit's audit screen currently surfaces flat journal-entry rows
(timestamp / actor / kind / payload). Per dev-note §J4 ("the
differentiator"), the operator wants a **decision-trail visualisation
of the agent pipeline** — the cockpit knows the multi-agent pipeline
produced this fill via this signal via this forecast via this LLM
debate; the audit screen should expose that lineage as a clickable
trail of agent nodes, not a flat row list.

Phase D lands this in two complementary surfaces:

1. **Trail screen mode (`Screen::Trail` route)** — the existing audit
   screen gains a new "trail" mode (chevron-toggleable from each
   journal row) that renders the upstream chain — Fill → Signal →
   Forecast → LLM debate transcript — as a stacked node graph using
   the new `widgets::trail_node` widget. Side-drawer surfaces raw
   artifacts (LLM prompt, debate transcript, forecast tensor).
2. **Live recent-activity rows gain a Trail chevron** — clicking a
   recent-activity row in the new `screens::live` panel opens that
   row's trail directly. Phase C reserved this as a follow-up; Phase D
   delivers it.

Critically: this is also the first downstream consumer of the
`audit-tick-consumer-envelope v0.1.0` broadcast stream — the deferred
**T-D-14** from that feature (`TcnForecaster::with_ledger()` runtime
wiring inside `crates/strategy`) closes here. Phase D is where the
broadcast pipe gets read.

## Scope (dev-note §6 Phase D — operator-locked)

1. **Q1 schema coverage confirmation** — the `audit::journal_entries`
   schema must carry enough information to reconstruct the chain
   `Fill → Signal → Forecast → LLM debate`. Phase A's `strategy_event`
   carries `kind` + `payload_json`, but the per-stage chain
   correlations (forecast_id → signal_id → fill_id) are TBD. Analyst
   confirms what's missing. If the schema doesn't cover, this brief
   adds the additive migration as part of M-T1 (no anchor risk;
   additive writers only).
2. **`screens::trail::view`** — new screen body (replaces the Phase A
   alias to `audit::view`). Owns the trail mode; falls back to the
   existing audit-search list mode when no row is selected. Default
   route on `Screen::Trail`. Phase A's deprecated `Screen::Audit`
   alias keeps routing here.
3. **`widgets::trail_node`** — new widget. Each node renders one
   pipeline stage (Fill / Signal / Forecast / LLM) with timestamp,
   actor, headline, and a chevron to expand the side-drawer. Visual:
   vertical stack, top→bottom upstream→downstream (latest-action at
   bottom).
4. **Side-drawer for raw artifacts** — right-rail drawer (existing
   `RIGHT_RAIL_WIDTH_PX` reserved slot in `shell.rs`) renders the
   raw payload selected from the trail node (LLM prompt, debate
   transcript JSON pretty-printed, forecast tensor as a heatmap or
   summary).
5. **Live recent-activity Trail chevron** — `screens::live::view`'s
   recent-activity panel gains a chevron per row. Clicking dispatches
   `Message::SwitchScreen(Screen::Trail)` followed by
   `Message::SelectTrailRow(audit_id)` (compound dispatch — same
   precedent as Phase C's "Open in Lab").
6. **Audit search-mode chevron** — the existing audit screen's row
   list (now `screens::trail::view`'s default mode) gains the same
   chevron per row. Same compound dispatch.
7. **First downstream consumer of `audit-tick-consumer-envelope`** —
   Phase D's reflection-equivalent state-replica reads from the
   broadcast stream to keep a hot "current trail open" mirror. This
   closes the deferred T-D-14: `TcnForecaster::with_ledger()` runtime
   wiring becomes load-bearing (the trail needs `ForecastEmitted`
   ticks at runtime, not feature-gated dead code).

## Out of scope

- Phase E (Compare matrix), Phase F (Memory + Models + Assistant slot).
- Trail-mode persistence across cockpit restarts (which row was last
  open). Lumen-design-adoption follow-up.
- Trail-mode export to file (operator can screenshot or copy the
  side-drawer JSON).
- New visual tokens or font choices. Phase D ships under existing
  Lumen tokens.
- Re-shaping the `audit::journal_entries` SQL schema beyond additive
  fields. Existing 22 anchors stay byte-identical (R10.1).

## Open questions for analyst

Likely Qs the analyst will surface:

- **Q1** — Schema gap: does `audit::journal_entries` currently carry
  the per-stage correlation IDs (forecast_id, signal_id, fill_id)?
  If yes, no schema change. If no, what migration shape?
- **Q2** — Trail node visual ordering: top-to-bottom upstream→
  downstream (latest at bottom — matches operator reading direction
  for a story) OR bottom-to-top (matches timeline UX — earliest at
  top)?
- **Q3** — Side-drawer trigger: chevron click on the trail node,
  hover, or always-on (showing the most-recent node's payload by
  default)?
- **Q4** — Phase D's first concrete downstream of `audit-tick-consumer-envelope`:
  should it be the trail-mirror itself (best K6 mitigation —
  exercises the broadcast pipe in production) OR a simpler "trail
  badge in Live" counter (smaller scope, defers broadcast pipe
  to Phase E)?
- **Q5** — Trail chevron visibility: every row (denser, more clicks
  available) OR only rows where a trail is reconstructable (less
  noisy, requires upfront classification)?

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical** (R10.1). Phase D
   adds audit writers only additively — existing schema stays. Anchor
   risk is LOW per dev-note §6 ("potentially new audit writers;
   additive only").
2. **Phase A/B/C surface unchanged** — Lab + chart + Train panel + Lab
   Run button + 3-group sidebar + Live screen + Strategy registry +
   Settings rollup all stay byte-identical.
3. **`cockpit-smoke` PASS 0 panics**.
4. **`cockpit-performance v1.0.0` idle-CPU floor ≤13.1%** preserved —
   the trail-mirror subscriber must back-pressure correctly (drop on
   lag) so it doesn't compete with the cockpit's redraw cadence.
5. **`spec-lint` Phase D contribution = 0** (baseline 87 carry-
   forward).
6. **No new external crate deps; no new Lumen tokens.**
7. **`audit-tick-consumer-envelope` invariants preserved** — Phase D's
   consumer subscribes via `AuditTickStream::into_iter_blocking()`
   (mirror barter-rs shape); no producer-side change to `Ledger`.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-D-001` to be opened in proposed state
by analyst pass.

## Changelog

- 2026-05-20 (orchestrator): promoted Phase D from dev-note §6 to
  proposed feature. Predecessor verified at
  `ui-rethink-phase-c-sidebar-ia v0.1.0`. Status `proposed`; awaiting
  analyst pass. Predecessor's deferred T-D-14 (TcnForecaster::with_ledger
  runtime wiring) becomes load-bearing in Phase D's R7.
