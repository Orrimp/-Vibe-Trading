---
slug: v26-forecast-bakeoff
status: roadmap
owner: pending-analyst
updated: 2026-05-17
version: 2.6.0
parent: v25-dl-forecast-overlay v2.5.0 (roadmap)
predecessor: v25b-transformer-overlay v2.5.2
---

# v2.6 — Forecast bake-off + retirement (phase 4 of 4)

> **Queued phase 4 of the 4-phase DL roadmap** at
> [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md).
> After phases 1, 2, 3 ship (TCN, PatchTST, vanilla Transformer),
> compare all three architectures on the same data + scenarios. Pick
> the canonical v2.5 forecaster; mark the other two as research-mode
> only.

## Why

Per the [4-phase roadmap](../v25-dl-forecast-overlay/feature.md): the
canonical v2.5 production forecaster is decided **by evidence**, not
by architecture intuition. After three independent ships, the bake-off
phase:

1. Re-runs all three forecasters on BS-1 + BS-2 with identical
   evaluation criteria (Sharpe lift vs v1 baseline, max drawdown,
   forecast calibration, inference latency).
2. Documents the comparison in a single dev-note.
3. Picks one architecture as the canonical v2.5 production overlay.
4. Marks the other two as `status: research-mode-only` — their crates
   stay in the workspace as reference impls but they don't ship as
   live overlays.
5. (Optional) Identifies follow-up briefs for ensemble overlays if the
   three forecasters' errors are uncorrelated enough to justify it.

## Carry-forward invariants

Same data, scenarios, overlay shape, audit shape, cost telemetry as
the [parent roadmap](../v25-dl-forecast-overlay/feature.md).

## Success criterion (provisional — analyst confirms)

The winning architecture must beat the v1 baseline by some operator-
locked margin (typically 1.05× Sharpe-lift on BS-1 AND BS-2; analyst
sharpens this when phase 4 activates). If NO architecture beats v1,
the bake-off concludes v2.5 doesn't ship as production; the project
moves to v3 (RL policy) with the DL portfolio retained as research
artifacts.

## Open design option — LLM arbiter (sourced 2026-05-17)

Per the external-code survey at
[`spec/dev-notes/external-code-patterns-2026-05-17.md`](../dev-notes/external-code-patterns-2026-05-17.md),
a candidate refinement worth considering when this phase activates:
an **LLM arbiter** that reads all three forecasters' outputs (TCN /
PatchTST / Vanilla Transformer) plus the operator's strategy params
and produces a tie-break decision with a reasoning trace that lands
in the audit ledger. Adapts the bull/bear adversarial pattern from
[TradingAgents](https://github.com/TauricResearch/TradingAgents)
(specifically the
[`bull_researcher.py`](https://github.com/TauricResearch/TradingAgents/blob/main/tradingagents/agents/researchers/bull_researcher.py)
debate-with-iteration-count discipline). Backlog entry queued at
`spec/backlog.md ## Queue ## Process / tooling →
v26-bakeoff-llm-arbiter`. The analyst weighs this against the simpler
"pick by Sharpe-lift" approach when phase 4 promotes.

## Changelog

- 2026-05-17 (orchestrator): added "LLM arbiter" open design option
  sourced from the external-code-patterns survey. Backlog entry queued.
  No status change.
- 2026-05-17 (orchestrator): phase 4 stub opened. Status: roadmap
  (pending phases 1, 2, 3).
