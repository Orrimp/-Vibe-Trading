---
slug: v25-kronos-forecast-overlay
status: candidate
owner: pending-analyst
updated: 2026-05-10
version: 2.5.0
---

# v2.5 — Kronos foundation-model forecast overlay (candidate)

> **Stub feature file.** This is a candidate for the v2.5 DL-forecaster
> slot in [`spec/product.md`](../product.md#strategy-library--roadmap).
> Not promoted; no analyst spawn. Holds the file-system slot and points
> at the technical evaluation. Promotion happens after v2 LLM ships.

## Status

- **Not active.** Sits in `spec/backlog.md` Strategy queue as a
  candidate for the v2.5 row.
- **Awaits analyst spawn.** Analyst takes ownership when the
  operator promotes from Queue to Active.
- **Blocks on:** v2-llm-strategy ship. v2 LLM is paused at
  architect→developer handoff; resumption breadcrumb at
  [`spec/v2-llm-strategy/orchestrator-scope-check-2026-05-10.md`](../v2-llm-strategy/orchestrator-scope-check-2026-05-10.md).
  Reason for the dependency: v2.5's research-mode determinism
  contract reuses the record/replay cache pattern from v2 LLM's Q8
  resolution, so v2.5's analyst and architect want to inherit a
  shipped pattern rather than re-design the same primitive.

## Pre-evaluation breadcrumb

A pre-analyst technical evaluation of the [Kronos](https://github.com/shiyu-coder/Kronos)
foundation model lives at
[`spec/dev-notes/kronos-evaluation-2026-05-10.md`](../dev-notes/kronos-evaluation-2026-05-10.md).
That dev-note holds:

- License + maturity signals (MIT, AAAI 2026 paper, 23.8k stars).
- Model architecture summary (decoder-only Transformer, 5 sizes,
  hierarchical OHLCV tokens, 512 / 2048-token context).
- Three integration paths (subprocess + IPC, ONNX + `tract`,
  candle native) with the orchestrator's prior on Option B.
- Author-flagged caveats and how our existing risk / exec / audit
  surfaces respond to them.
- Eight open questions the analyst will resolve at promotion time
  (Q1 pre-trained vs fine-tuned, Q2 model size, Q3 integration
  path, Q4 forecast horizon, Q5 pure-strategy vs overlay shape,
  Q6 determinism, Q7 anchor impact, Q8 cost telemetry).

The analyst reads that breadcrumb first when this candidate is
promoted, then expands it into the full feature brief (`## Why`,
`## Requirements`, etc.) per the standard analyst output contract
at [`.claude/agents/analyst.md`](../../.claude/agents/analyst.md).

## What changes when this is promoted

When the operator promotes this candidate (likely after v2 LLM
ships and the operator wants to evaluate a DL-forecasting layer):

1. `status: candidate` flips to `status: in-progress`.
2. `owner: pending-analyst` flips to `owner: analyst`.
3. The analyst expands this stub into a full feature brief —
   answering the 8 open questions in the dev-note plus surfacing
   any new ones.
4. The architect resolves architecture-decide questions (most
   likely the integration path Q3 + the determinism contract Q6).
5. The developer ships.
6. Tester re-runs the 9 strategy backtest anchors (must stay
   byte-identical — Kronos is additive, not replacing) plus locks
   a 10th anchor for the new Kronos strategy scenario.

## Cross-references

- [`spec/dev-notes/kronos-evaluation-2026-05-10.md`](../dev-notes/kronos-evaluation-2026-05-10.md)
  — the technical breadcrumb (orchestrator-authored 2026-05-10).
- [`spec/product.md`](../product.md) § "Strategy library —
  roadmap" — the v2.5 row Kronos slots into.
- [`spec/architecture.md`](../architecture.md) § ML serving — names
  `tract` as the project's ONNX-serving default (relevant to
  integration path Option B).
- [`spec/v2-llm-strategy/feature.md`](../v2-llm-strategy/feature.md)
  § Design Q8 — the record/replay cache pattern v2.5 inherits.
- [`spec/v15a-mean-reversion-pairs/feature.md`](../v15a-mean-reversion-pairs/feature.md)
  § T717 — the anchor-re-lock precedent v2.5 mirrors if any
  existing report-rendering anchor moves.
- [`crates/llm/src/lib.rs`](../../crates/llm/src/lib.rs) — the
  `LlmProvider` trait shape (after v2 LLM ships) is a sibling
  pattern; if Kronos lands as a "ForecastProvider"-shaped trait,
  the analyst confirms whether it lives in `crates/llm/`,
  `crates/models/` (existing scaffolding crate), or a new
  `crates/forecast/` crate.

## Changelog

- 2026-05-10 (orchestrator): stub created during v2-llm-strategy
  pause, after operator asked *"Can we learn something from
  [Kronos] and update our product?"* and approved the
  recommendation to capture as a v2.5 candidate without rewriting
  the v2 brief. Frontmatter `status: candidate` is a new status
  in this project; previously only `in-progress`, `shipped`,
  `roadmap`, and `reserved` were used. `candidate` distinguishes
  "operator-flagged for future evaluation" from `reserved`
  ("operator-committed; analyst spawns later"). The technical
  evaluation (license, integration paths, open questions) lives
  in the sibling dev-note rather than in this stub.
