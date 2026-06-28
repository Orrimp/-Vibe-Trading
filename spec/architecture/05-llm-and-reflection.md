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

Default stack: `candle` for prototyping, ONNX via `tract` for serving
production-trained models (named in
[10-foundation-libraries.md § Numerics & ML](10-foundation-libraries.md#numerics--ml)).

The first concrete consumer landed at v2.5 with the Kronos
foundation-model forecast overlay — see
[12-forecast-overlay.md](12-forecast-overlay.md) for the cross-cutting
`ForecastProvider` trait + signal-level overlay composition pattern,
and [ADR-0027](adr/0027-kronos-onnx-tract-integration.md) for the
v2.5-specific Kronos resolutions (Option B ONNX + `tract`, base
102.3M, inherited record/replay determinism contract).

Future DL/ML model integrations should follow the
[12-forecast-overlay.md](12-forecast-overlay.md) pattern as the
default shape (forecast → overlay → strategy), departing only via a
new ADR with explicit rationale.

## LLM integration

Foundation resolved at v2.0.0 in
[ADR-0019 — v2 LLM strategy foundation (Q4–Q11)](adr/0019-v2-llm-strategy.md).

The trait surface, three provider impls (Anthropic / OpenAI-compatible /
Ollama), prompt-cache builder, budget gate with auto-degrade,
record/replay for research mode, tool-use schemas, and rate-limit handling
all land in v2.0.0 as foundation-only — no LLM consumers ship in v2.0.0;
each consumer is its own follow-up brief. Reflection-memory is the
canonical first consumer; see the cross-link below.

### First in-app LLM consumer — the advisor "why this one" narration (F9)

The single-coin-advisor "why this one" **narration** (feature
[`advisor-llm-narration`](../v1/advisor-llm-narration/feature.md), pivot F9,
[ADR-0064](adr/0064-advisor-llm-narration-seam.md)) is the **first place an
`Arc<dyn LlmProvider>` is actually called in-app**. It is a **READ-ONLY**
plain-language rendering of the ALREADY-DECIDED structured bake-off
`Recommendation` (the LLM never enters the ranking — narration only, reinforcing
the F8 lock). Two seam rules generalise beyond F9:

- **The generator is agent-side, behind a deterministic faithfulness post-check.**
  It lives in `agent::narration` (the `agent::plan`/ADR-0062 twin — `agent` owns
  the boot-built provider + hard-deps `llm` AND `backtest`) and emits a
  `core`-clean `NarrationOutcome { Ready(SmolStr) | FellBack }`, so no `llm` type
  crosses the `ui` `view` line. The **load-bearing guard is a pure, `llm`-free,
  unit-testable post-check** (`check_faithful`) whose reject predicates +
  predict/advise banned-phrase list are FROZEN in ADR-0064 § D2 — a narration that
  crowns the wrong winner, contradicts the outcome code, fabricates a number
  (exact-string-vs-`num`-formatter), or trips a banned phrase falls back to the
  templated copy. **Any LLM rendering of a structured engine result must carry this
  post-check pattern** — the prompt is a soft first line, the post-check is the net.
- **Honest fallback + ephemeral.** Every failure mode (disabled / unavailable /
  error / timeout / `BudgetExceeded` / post-check-reject) lands the existing
  templated copy; the LLM never blocks or breaks the decision. The prose is NOT
  persisted (the structured `Recommendation` is the reproducible artifact). The
  call goes through `BudgetedProvider` (the monthly budget + auto-degrade governs
  it) with a `CacheBreakpoint::Ephemeral` static prefix (one cheap cached call).

Tests and render harnesses exercise the path through a **fake `LlmProvider`**
(faithful + unfaithful) with **no network**, per the "every external I/O behind a
trait" rule.

## Reflection-memory

Reflection-memory (LLM lessons persisted across runs and re-injected into
prompts on the next session) is documented under
[`spec/reflection-memory/feature.md`](../v1/reflection-memory/feature.md) —
it's a feature, not an architectural invariant. Cross-reference here so
new architects know to look there before re-debating "should we
re-inject prior session learnings".

## Changelog
- 2026-06-22 (architect): added § "First in-app LLM consumer — the advisor 'why
  this one' narration (F9)" recording [ADR-0064](adr/0064-advisor-llm-narration-seam.md):
  the agent-side `agent::narration` generator (the `agent::plan` twin), the FROZEN
  deterministic faithfulness post-check as the load-bearing guard (the
  prompt-is-soft / post-check-is-the-net pattern any structured-result LLM
  rendering must carry), the honest templated fallback, ephemeral non-persistence,
  the `BudgetedProvider` + `CacheBreakpoint::Ephemeral` cost path, and the
  fake-`LlmProvider` (faithful + unfaithful, no-network) test seam. F9 is the first
  in-app `Arc<dyn LlmProvider>` call site (reflection's `generate_card` is
  deterministic / not LLM-wired).
- 2026-05-16 (architect): replaced the ML/DL deliberate-stub paragraph
  with the v2.5 Kronos pointer (cross-link to ADR-0027 and the new
  cross-cutting [12-forecast-overlay.md](12-forecast-overlay.md)
  pattern file). The candle-vs-burn-vs-tract default rationale is
  retained in [10-foundation-libraries.md § Numerics & ML](10-foundation-libraries.md#numerics--ml)
  so this file no longer carries it twice.
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
