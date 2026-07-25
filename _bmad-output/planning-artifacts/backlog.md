---
slug: backlog
status: living
owner: orchestrator
updated: 2026-07-25
---

# Backlog (forward-looking queue)

> **What has been built lives in [CHANGELOG.md](../../CHANGELOG.md)** — one line per
> feature, grouped by subsystem/version. **What's currently in flight lives in
> [`sprint-status.yaml`](../implementation-artifacts/sprint-status.yaml)** (the live board —
> `in-progress`/`ready-for-dev`/`review` stories) — that supersedes this file's old `## Active`
> section as of the 2026-07-25 BMAD-migration Phase 5b cutover. **This file is now the lean
> forward-looking queue only**, ported verbatim from `spec/backlog.md`'s `## Queue (open /
> deferred)` section (full pre-cutover history, including the shipped-history archaeology and the
> completed remediation P0-P8 plan, is archived at
> [`docs/archive/pre-bmad-spec/backlog.md`](../../docs/archive/pre-bmad-spec/backlog.md) and in
> git history).
>
> **Strategy research is CONCLUDED (2026-06-08) → ship passive.** Across all three
> reachable channels (price/OHLCV, derivatives-positioning, on-chain) no active
> strategy beat passive buy-and-hold net of cost under the frozen block-bootstrap
> robustness rule. **No active-strategy bets remain.** Terminal verdict + scope:
> [`PRD.md`](PRD.md).
>
> **Status (2026-07-09): FEATURE-COMPLETE.** The advisor MVP, the v2 research-driven
> tranche, and the v3 "prove it's done" close-out have all shipped — see
> [`../../CHANGELOG.md`](../../CHANGELOG.md). The only genuinely-open items are below.

## Queue

_(open / deferred items)_

### Deferred by decision
- **cockpit-cross-platform CI** — Linux/Windows source shipped + macOS-verified; the
  3-OS GitHub Actions matrix was ACTIVATED 2026-07-10 (remediation P7); the run-2
  Linux/Windows shakeout is the open work (fix-forward) — tracked as story
  `6-9-cockpit-cross-platform` (`in-progress`).
- **`lab-recipe-test-harness v0.3.0+`** — Recipe / subscription harness extension;
  robustness gate cleared, awaiting an analyst spawn. **Still wanted** — re-confirmed in the
  v3 close-out (2026-07-09) as the one genuinely-open forward *build* item; it is infra, NOT
  required for product feature-completeness (see [`../../CHANGELOG.md`](../../CHANGELOG.md)
  § Deferred). Tracked as story `2-63-lab-recipe-test-harness-v0-3-extension` (`backlog`).

### Gated on the parked v2 LLM strategy
- **Lumen Phase 6 — right-rail Assistant slot** — reserved column-track in the shell grid;
  hidden until the v2 LLM strategy is enabled. Tracked as story
  `2-7-lumen-phase-6-assistant-slot` (`backlog`).
- **v2.1 cockpit LLM-budget tile + pedantic clippy cleanup** — deferred indefinitely (program concluded).
- **v2 LLM evolution** (`v2x-trading-state-bus`, `v26-bakeoff-llm-arbiter`) — deferred; gated on
  re-activating the LLM desk, which is support-layer scope, not alpha.

### Gated on an operator decision
- **advisor-reflection-decision-loop (the honest C4)** — architecture done
  (`arch-done`), build pending an operator green-light (or an explicit park via the
  do-not-build check). Tracked as story `3-18-advisor-reflection-decision-loop`
  (`ready-for-dev`); design preserved at
  [`docs/archive/pre-bmad-spec/advisor-reflection-decision-loop/`](../../docs/archive/pre-bmad-spec/advisor-reflection-decision-loop/).

### Future fresh program (NOT a continuation of the concluded hunt)
- **C4 — deterministic learning loop** (reflection-feedback decision seam; PRD core
  pillar 3) — never built; would adapt param/route selection from the reflection store through
  the sanctioned ADR-0041 layering seam. Moot while passive is the shipped strategy.
- **Untested orthogonal channels** — options/implied-vol (Deribit DVOL), cross-asset/macro
  (DXY, rates, SPX), social/sentiment. Out of scope for the concluded hunt; each would be a
  **fresh** program with its own data adapter and backtest, not a re-open of this one.

> Speculative UI test-infra candidates (AccessKit shadow-tree assertions, VLM second-opinion
> judge, comet debugger, inspect-MCP shim, mutation-testing pass, …) lived here as unscheduled
> ideas; they are preserved in git history and re-proposable on demand rather than carried inline.

## Recent (shipped)

See **[CHANGELOG.md](../../CHANGELOG.md)** for the full per-version shipped index, and
`git log -- docs/archive/pre-bmad-spec/<path>/` (or the pre-migration `spec/<slug>/` path) for
any feature's narrative history. The BMAD-native per-feature record is the story file at
`_bmad-output/implementation-artifacts/{epic}-{story}-{slug}.md`.

## Conventions

- This file holds the **forward-looking queue only**; shipped work is recorded in
  [CHANGELOG.md](../../CHANGELOG.md) and in `sprint-status.yaml`/the story files, not here.
- One-line entries; a queued item is promoted to a story (`Status: ready-for-dev`) only
  when an analyst/architect picks it up.
- The orchestrator owns this file; agents may suggest additions, the operator approves promotions.
- Items can stay indefinitely; stale items get a `_decayed_` tag rather than silent deletion.
