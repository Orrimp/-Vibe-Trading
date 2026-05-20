---
slug: ui-rethink-phase-c-sidebar-ia
status: proposed
owner: pending-analyst
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-b-lab-run v0.2.0
---

# UI rethink Phase C — Sidebar IA flip + Live + Strategy registry

> Third concrete feature in the UI rethink dev-note
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/ui-rethink-2026-05-17.md).
> Dev-note §6 Phase C is the **scope source-of-truth**; this brief is
> the **implementation contract**. Predecessor:
> [`ui-rethink-phase-b-lab-run v0.2.0`](../ui-rethink-phase-b-lab-run/feature.md)
> shipped 2026-05-19. Lab + chart + Phase A/B groundwork stay intact;
> this phase replaces the surrounding shell.

## Why

The cockpit currently ships a **seven-screen sidebar** (Home / Live /
Compare / Strategies / Memory / Models / Trail / Settings — eight if
you count Lab, which Phase A inserted). Per the dev-note's
operator-locked audit (§§ 1, 2, 3), the IA is **not coherent**:

- "Home" duplicates "Live"; they cover the same operator job (J1
  "what is the cockpit doing right now"). Phase C lands `Live` as the
  authoritative redesigned Home.
- "Strategies" exposes runtime panels for already-active strategies;
  what the operator wants is a **strategy registry** (J6 "what's
  available; what's tested; what's shipped"). Phase C lands the
  redesigned Strategies as a registry view.
- "Risk", "Control", "Debug" are operator-locked under "Settings" (§
  3) — Phase C rolls them up into a single Settings screen with
  sub-tabs.
- The sidebar's flat 7-entry layout doesn't scale to the new Lab
  (Phase A) + future Trail (Phase D) + Compare (Phase E) + Memory
  (Phase F) plan. Phase C lands the **three-group structure** from
  dev-note § 3 (Lab / Live / Audit-as-Trail vs. Strategy / Memory /
  Models / Settings).

## Scope (dev-note §6 Phase C — operator-locked)

1. **New sidebar IA** — three-group structure (operator-locked at
   dev-note §3): the **work zone** (Lab / Live / Trail / Compare), the
   **library zone** (Strategies / Memory / Models), and the **chrome
   zone** (Settings + assistant slot reserved for Phase 6).
2. **`Live` screen** — replaces the old `Home`/`Screen::Home` (Phase A
   marked it `#[deprecated]`). Carries J1: ticker strip, recent
   activity, open positions, latency mirror. Reuses widgets already
   shipped; no new chart code.
3. **`Strategy registry` screen** — replaces the old `Strategies`
   panel-style screen. Carries J6: list of registered strategies
   (shipped / candidate / archived) with status, last backtest
   anchor, last live run timestamp, click-to-Lab-with-this-strategy.
4. **`Settings` rollup** — Risk + Control + Debug fold into Settings
   sub-tabs (operator-locked dev-note §3). Single screen with three
   tabs; one navigation entry in the sidebar.
5. **Compat shim** — old `Message::SwitchScreen(Screen::Home)`,
   `Screen::Risk`, `Screen::Control`, `Screen::Debug` arms stay
   deprecated for one cycle; route to `Screen::Live` / `Screen::Settings`
   internally. Tests keep using the deprecated variants until the
   next phase prunes the shim.

## Out of scope

- Phase D (Trail view), Phase E (Compare matrix), Phase F (Memory +
  Models + Assistant slot). Each is its own brief.
- Sidebar visual restyle beyond the IA flip — no Lumen-token
  changes, no new fonts, no animation overhaul. Visual polish lives
  in the existing `lumen-design-adoption` umbrella.
- Removing the deprecated `Screen::*` variants. Phase C keeps them
  as compatibility shims for one cycle (analyst confirms the
  retirement target in the brief).
- New widgets for `Live` / `Strategy registry` / `Settings` — each
  reuses existing widgets where possible. New widgets only land if
  the analyst's Q section escalates a specific gap to the operator.
- Operator preferences / persistence for which sub-tab is active in
  Settings (a Lumen-design-adoption follow-up, not Phase C).

## Open questions for analyst

Likely Qs the analyst will surface (placeholders — analyst refines):

- **Q1** — Compat shim retirement: one cycle (Phase D) or two cycles
  (defer until Phase F)? Default: one cycle (Phase D removes the
  `#[deprecated]` variants).
- **Q2** — Settings tab ordering: Risk → Control → Debug, or some
  operator-justified order (e.g. Risk-first because most-consulted)?
- **Q3** — Strategy registry empty-state: when no strategies are
  registered, render a "Register a strategy" call-to-action or a
  blank slate with a "Run a backtest to register" hint?
- **Q4** — Live screen ticker-strip: should it inherit the
  `chart-fixture-line-clipping v1.0.0` chart canvas (rendering now
  correct end-to-end) for a sparkline-style mini-chart, or stay
  text-only?
- **Q5** — Sidebar visual representation of the three groups:
  vertical separators (operator-friendly), section headers (denser),
  or no visible grouping (cleanest minimalism)?

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical** (R10.1). Phase C
   touches no strategy / audit / exec / report path; anchor risk is
   zero by construction (dev-note §6 Phase C: *"Anchor risk: zero"*).
2. **Lab + chart + Phase A/B surface unchanged** — the Lab screen,
   chart widget, Train sub-panel, Run button, `run_delta_badge`,
   `engine::run_scenario` dispatch all stay byte-identical.
3. **`cockpit-smoke` PASS 0 panics** (R10.4 inherited pattern).
4. **`cockpit-performance-and-input-responsiveness v1.0.0`
   idle-CPU floor (≤13.1%)** stays under budget — no new
   `tokio::time::interval` redraws, no new subscriptions per
   compositor frame.
5. **`spec-lint` Phase C contribution = 0** (current baseline 87
   carry-forward).
6. **No new external crate deps**; no new Lumen tokens; no iced
   version bump.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-C-001` to be opened in proposed state
by analyst pass.

## Changelog

- 2026-05-20 (orchestrator): promoted from dev-note §6 Phase C to
  proposed feature. Predecessor verified at
  `ui-rethink-phase-b-lab-run v0.2.0`. Status `proposed`; awaiting
  analyst pass.
