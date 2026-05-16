---
slug: architecture-05-llm-and-reflection
status: shipped
owner: architect
updated: 2026-05-16
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

Foundation resolved at v2.0.0 in
[ADR-0019 — v2 LLM strategy foundation (Q4–Q11)](adr/0019-v2-llm-strategy.md).

The trait surface, three provider impls (Anthropic / OpenAI-compatible /
Ollama), prompt-cache builder, budget gate with auto-degrade,
record/replay for research mode, tool-use schemas, and rate-limit handling
all land in v2.0.0 as foundation-only — no LLM consumers ship in v2.0.0;
each consumer is its own follow-up brief. Reflection-memory is the
canonical first consumer; see the cross-link below.

## Reflection-memory

Reflection-memory (LLM lessons persisted across runs and re-injected into
prompts on the next session) is documented under
[`spec/reflection-memory/feature.md`](../reflection-memory/feature.md) —
it's a feature, not an architectural invariant. Cross-reference here so
new architects know to look there before re-debating "should we
re-inject prior session learnings".

## Changelog
- 2026-05-16 (architect): replaced the dangling
  `../architecture.md#v2--llm-strategy-resolutions-...` anchor (which
  no longer exists after the Phase 1A compression of the monolith)
  with a direct link to `adr/0019-v2-llm-strategy.md`. Dropped the
  "will be extracted to ADR-0019 in Session 7" note — ADR-0019 is
  shipped.
- 2026-05-13 (architect): content migrated from `spec/architecture.md`
  §§ ML/DL and LLM integration during Phase 1A Session 3. Added a brief
  pointer to the reflection-memory feature so this file is the canonical
  starting point for LLM-related architecture questions.
