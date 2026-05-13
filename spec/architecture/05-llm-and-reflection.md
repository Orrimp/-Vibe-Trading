---
slug: architecture-05-llm-and-reflection
status: shipped
owner: architect
updated: 2026-05-13
---

# ML / DL and LLM integration

Model serving stack (`candle` / `burn` / `tract`) and the v2 LLM strategy
trait surface. The reflection-memory loop is documented separately under
its feature folder.

## ML / DL

_Architect: pick `candle` vs `burn` vs `tract`+ONNX once the first model
is chosen. Default assumption: `candle` for prototyping, ONNX via `tract`
for serving production-trained models._

This section is a deliberate stub pending the first ML-touching feature.
The default-stack rationale lives here so the architect doesn't re-debate
it under deadline pressure when that feature lands.

## LLM integration

_Foundation resolved at v2.0.0 — see
[`spec/architecture.md` § v2 — LLM strategy resolutions (Q4–Q11) — confirmed 2026-05-10](../architecture.md#v2--llm-strategy-resolutions-q4q11--confirmed-2026-05-10)_.

The trait surface, three provider impls (Anthropic / OpenAI-compatible /
Ollama), prompt-cache builder, budget gate with auto-degrade,
record/replay for research mode, tool-use schemas, and rate-limit handling
all land in v2.0.0 as foundation-only — no LLM consumers ship in v2.0.0;
each consumer is its own follow-up brief.

The v2 resolution block currently lives at lines 2384–2528 of
`spec/architecture.md` (post-Session-2 numbering). It will be extracted
to ADR-0019 in Phase 1A Session 7 per
`outputs/architecture-split-proposal.md`. At that point this stub gains a
direct ADR link instead of an in-architecture.md anchor.

## Reflection-memory

Reflection-memory (LLM lessons persisted across runs and re-injected into
prompts on the next session) is documented under
[`spec/reflection-memory/feature.md`](../reflection-memory/feature.md) —
it's a feature, not an architectural invariant. Cross-reference here so
new architects know to look there before re-debating "should we
re-inject prior session learnings".

## Changelog
- 2026-05-13 (architect): content migrated from `spec/architecture.md`
  §§ ML/DL and LLM integration during Phase 1A Session 3. Added a brief
  pointer to the reflection-memory feature so this file is the canonical
  starting point for LLM-related architecture questions.
