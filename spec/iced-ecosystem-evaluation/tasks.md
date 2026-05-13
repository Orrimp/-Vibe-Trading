---
slug: iced-ecosystem-evaluation
status: in-progress
owner: architect
updated: 2026-05-13
---

<!-- M0 falsifier sub-agent pass 2026-05-13: T-M0-1 / T-M0-2 / T-M0-3
     ticked. Brief A (native table+grid+float+pin) is CLEAR-TO-SPAWN. -->


# Tasks — iced ecosystem evaluation

> **Status:** architect synthesis pass complete (feature.md v0.2.0). Tasks
> below are M0 architect-diagnostic + brief-spawn stubs. **No code changes,
> no crate adds in this brief** — each adoption brief (A / B / C / D) opens
> its own `spec/<slug>/feature.md` with its own analyst → architect → dev
> → tester loop. Detailed dev tasks are intentionally NOT enumerated here.
> They are the responsibility of the brief-specific architect pass.

## M0 — Architect-diagnostic falsifiers (zero-cost, orchestrator-runnable)

Falsify the two load-bearing hypotheses BEFORE any adoption brief opens.
Both are cheap `cargo doc` + `grep` checks; both can run in a single
read-only sandbox.

- [x] **T-M0-1 — Falsify H-arch-0** (iced 0.14 native widgets reachable
  from current feature set). _Acceptance: orchestrator runs `cargo doc -p
  iced --no-deps --features "tiny-skia,thread-pool,advanced,canvas"`,
  greps the generated rustdoc index for `pub mod table`, `pub mod grid`,
  `pub mod float`, `pub mod markdown`, `pub mod pin`; emits a 5-row
  pass/fail table. PASS → Brief A unblocks. FAIL on any → Cargo.toml
  feature audit becomes a precondition._ — **Resolved (M0 falsifier
  sub-agent 2026-05-13): RESOLVED-FALSIFIED-partial.** Per-widget:
  table=REACHABLE, grid=REACHABLE, float=REACHABLE, pin=REACHABLE,
  markdown=NOT-REACHABLE under current features (requires `markdown`
  feature flag in Cargo.toml). See
  [feature.md ## H-arch-0](feature.md#hypothesis-register-architect-2026-05-13).
  Brief A scope (table+grid+float+pin) is CLEAR; M5 markdown viewer
  needs 1-line Cargo.toml change as precondition. _Sandbox caveat:
  `cargo doc` blocked; substituted build-artifact `.d` manifest +
  fingerprint JSON evidence._
- [x] **T-M0-2 — Falsify H-arch-7** (native `table` lacks virtualization
  in iced 0.14). _Acceptance: from the same `cargo doc` output, grep the
  `table` module for `lazy` / `virtual` / `with_offset` / `row_provider`
  patterns. PASS (no virtualization API) → A3 stays HELD per architect
  decision (agent_feed keeps hand-roll). FAIL (virtualization found) →
  A3 unblocks for Brief A scope expansion._ — **Resolved (M0 falsifier
  sub-agent 2026-05-13): RESOLVED-UNFALSIFIED-partial.** Indirect
  evidence (iced_widget partitions `lazy` into a separate feature-gated
  module sibling to `table`; `lazy.rs` ABSENT, `table.rs` PRESENT in
  compiled `.d` manifest) is consistent with H-arch-7. Direct
  `table.rs` source grep is an orchestrator-only step because the
  sub-agent sandbox blocked `~/.cargo/registry/` reads. **A3
  (`agent_feed.rs`) stays HELD per architect decision.** See
  [feature.md ## H-arch-7](feature.md#hypothesis-register-architect-2026-05-13).
- [x] **T-M0-3 — Falsify H-arch-2** (chart_tooltip is canvas-draw, not
  widget-tree). _Acceptance: `grep -nE "draw_tooltip|ChartProgram::draw"
  crates/ui/src/widgets/chart_tooltip.rs crates/ui/src/widgets/chart.rs`
  — confirms call-graph is canvas-internal (expected). CONFIRMED →
  document that native `float` does NOT apply to chart_tooltip in Brief
  A scope; FALSIFIED → re-scope Brief A to include chart_tooltip._ —
  **Resolved (M0 falsifier sub-agent 2026-05-13): RESOLVED-UNFALSIFIED
  (CONFIRM).** `chart_tooltip::draw_tooltip` is a free function
  ([`chart_tooltip.rs:68`](../../crates/ui/src/widgets/chart_tooltip.rs#L68))
  called from inside `ChartProgram::draw`
  ([`chart.rs:468`](../../crates/ui/src/widgets/chart.rs#L468));
  `ChartProgram` implements `canvas::Program<Message>`
  ([`chart.rs:226`](../../crates/ui/src/widgets/chart.rs#L226)); no
  `impl Widget` exists in `chart_tooltip.rs`. Native `float` does NOT
  apply to chart_tooltip without first lifting it out of the canvas;
  documented in Brief A scope as expected. See
  [feature.md ## H-arch-2](feature.md#hypothesis-register-architect-2026-05-13).
- [ ] **T-M0-4 — Operator routing of Q-O1 / Q-O2 / Q-O3** (architect
  hands off; orchestrator owns the route). _Acceptance: operator answers
  in the presentation reply (or via direct chat); architect records the
  answers in [`feature.md ## Operator-input questions`](feature.md#operator-input-questions)
  as resolved with a 2026-05-13+ datestamp._

## M1 — Brief A: Native iced 0.14 widget adoption (stub)

Opens after T-M0-1 PASSES + operator confirms Q-O3 ordering.

- [ ] **T-A-spawn — Spawn analyst for Brief A** (`spec/iced-native-widget-adoption/feature.md`).
  _Acceptance: analyst brief authored against the per-candidate cost
  table in [`feature.md ## Brief A`](feature.md#brief-a--native-iced-014-widget-adoption);
  status `draft`; predecessor pointer to this brief._
  - Sub-targets (analyst expands these): A1 positions, A2 strategies,
    A4 kpi_strip, A5 journal_transaction_modal. A3 agent_feed
    conditional on H-arch-7 outcome.
  - Files touched (estimate): 4-5 widget files,
    ~900-1100 LOC retired, ~20 `.snap` baselines refreshed, 0 PNG
    baselines diff, 0 new transitive crates.
  - Falsifier link: [H-arch-1](feature.md#hypothesis-register-architect-2026-05-13),
    H-arch-7 (gates A3), H-arch-8 (gates A4).

## M2 — Brief B: `iced_aw` cherry-pick (stub)

Opens after Brief A ships (operator approval recorded).

- [ ] **T-B-spawn — Spawn analyst for Brief B** (`spec/iced-aw-cherry-pick/feature.md`).
  _Acceptance: analyst brief authored against per-candidate cost table in
  [`feature.md ## Brief B`](feature.md#brief-b--iced_aw-cherry-pick);
  status `draft`; predecessor pointer to Brief A's shipped state._
  - Sub-targets (analyst expands): B1 date_picker (viewer bin / v1.11),
    B2 spinner (panel_state::Loading replacement), B3 badge (status
    chips).
  - Files touched (estimate): viewer-bin + ~3 widget files + Cargo.toml
    (+1 direct dep `iced_aw = "0.14"`), ~50-100 LOC retired + new
    surface, ~13 `.snap` baselines refreshed, 0 PNG diff.
  - Falsifier link: [H-arch-4](feature.md#hypothesis-register-architect-2026-05-13)
    (date_picker), H-arch-9 (spinner determinism), H-arch-10 (badge
    styling).

## M3 — Brief C: `iced_dialog` chrome (stub, gated)

Opens ONLY IF H-arch-6 falsifies (native `float` lacks focus-trap API).
Default expectation: brief stays closed.

- [ ] **T-C-decide — Falsify H-arch-6** (run during Brief A dev pass).
  _Acceptance: `cargo doc -p iced --no-deps` + grep `float` module for
  focus-handling APIs. PASS (focus APIs present) → Brief C STAYS
  CLOSED, journal_transaction_modal absorbs into Brief A. FAIL → Brief
  C unblocks._
- [ ] **T-C-spawn (conditional) — Spawn analyst for Brief C**
  (`spec/iced-dialog-modal-chrome/feature.md`). _Acceptance: only if
  T-C-decide FAILS. Falsifier link: H-arch-11._

## M4 — Brief D: `plotters-iced2` SPIKE (stub, gated)

Research-only spike. Opens ONLY after Brief A ships + operator approves
opening it (per Q-O3 default = "proceed in architect's order").

- [ ] **T-D-spawn (conditional) — Spawn analyst for Brief D spike**
  (`spec/plotters-iced2-sparkline-spike/feature.md`).
  _Acceptance: 1 dev-day spike port of [`sparkline.rs`](../../crates/ui/src/widgets/sparkline.rs)
  (180 LOC) to plotters-iced2 backend; two-consecutive-run determinism
  check; emits routing recommendation (CONFIRM-SKIP or surface-falsification)
  back to operator. Falsifier link:
  [H-arch-5](feature.md#hypothesis-register-architect-2026-05-13)._
  - Critical: spike does NOT commit adoption. Outcome routes to operator
    for go/no-go on broader chart-stack consolidation.

## M5 — Operator-gated: Q-O2 markdown viewer (stub)

Opens IF operator answers Q-O2 = ADOPT.

- [ ] **T-Q-O2-spawn (conditional) — Spawn analyst for markdown viewer**
  (`spec/iced-markdown-viewer-integration/feature.md`).
  _Acceptance: analyst brief authored to enable iced `markdown` feature
  + viewer-bin panel rendering the 11 committed backtest reports. Status
  `draft`; predecessor pointer to this brief; H-arch-3 the falsifier._
  - Files touched (estimate): viewer-bin + Cargo.toml feature flag, 0
    new deps, +1 viewer-bin `.snap` baseline, 0 PNG diff, no cockpit
    risk.

## Notes

- **Read order for orchestrator:** [`feature.md ## Design — architect
  synthesis`](feature.md#design--architect-synthesis) first (Q-resolutions
  + 4-brief ordering); then [`## Hypothesis register`](feature.md#hypothesis-register-architect-2026-05-13)
  (12 H-arch-N entries); then [`## Operator-input questions`](feature.md#operator-input-questions)
  (Q-O1 / Q-O2 / Q-O3 routes).
- **Anchor risk: zero across all 4 briefs.** No changes to
  `crates/strategy/`, `crates/audit/`, `crates/exec/`, `crates/backtest/`,
  or report rendering. The 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) are not in scope for ANY of A /
  B / C / D / markdown-viewer.
- **PNG baseline impact: zero across all 4 briefs.** None of the briefs
  touches the Charts screen (the only surface backed by PNG baselines
  per [`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md)).
  All visual deltas land on the 65 non-Charts `.snap` baselines and are
  refreshed via `cargo insta review`.
- **Capability boundary enforcement:** Per [`AGENT.md ## Architect =
  hypothesis only`](../../AGENT.md#architect--hypothesis-only), every
  H-arch-N falsifier is `cargo doc` / `cargo tree` / `grep` based — none
  require a display server, GPU, or live cockpit run. Orchestrator owns
  any falsifier that escalates beyond that scope.
- **Operator gating chain:** M0 → operator (Q-O1 / Q-O2 / Q-O3) → M1
  (Brief A) → M2 (Brief B) → (M3 conditional, M4 conditional, M5
  conditional). Sequential by default per [`AGENT.md ## Parallelism
  caveat`](../../AGENT.md#parallelism-caveat).

## Changelog

- 2026-05-13 (M0 falsifier sub-agent — orchestrator-routed, read-only
  sandbox): Ticked T-M0-1, T-M0-2, T-M0-3 with cite back to
  [feature.md ## Hypothesis register](feature.md#hypothesis-register-architect-2026-05-13).
  Per-hypothesis status: H-arch-0 = RESOLVED-FALSIFIED-partial
  (table/grid/float/pin REACHABLE; markdown gated behind `markdown`
  feature flag); H-arch-2 = RESOLVED-UNFALSIFIED (chart_tooltip is
  canvas-internal as architect predicted); H-arch-7 =
  RESOLVED-UNFALSIFIED-partial (indirect feature-graph evidence
  consistent with eager-only table; direct `table.rs` grep blocked by
  sub-agent sandbox — orchestrator may re-run for full confirmation).
  Brief A native-widget scope (table+grid+float+pin) is CLEAR-TO-SPAWN.
  A3 (`agent_feed.rs`) stays HELD per architect decision. M5 markdown
  viewer requires a 1-line Cargo.toml feature flag addition as
  precondition. HANDOFF → orchestrator.
- 2026-05-13 (architect): synthesis pass. Replaced empty stub with M0
  diagnostic + four brief-spawn stubs (A native widgets, B iced_aw,
  C iced_dialog gated, D plotters-iced2 spike) + M5 operator-gated
  markdown-viewer stub. Owner analyst → architect. Status stays
  `in-progress` (next handoff = orchestrator routes operator Qs).
- 2026-05-13 (analyst, initial draft): architect-decide stub posted with
  expected ordering (H3 markdown → H4 date_picker → H1 table → H2 float
  → H5 plotters-iced2). Architect's resolved order revised per cost +
  risk analysis; see [`feature.md ## Adoption priority`](feature.md#adoption-priority--four-briefs-recommended-ordering).
