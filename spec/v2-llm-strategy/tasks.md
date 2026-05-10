---
slug: v2-llm-strategy
status: in-progress
owner: analyst
updated: 2026-05-10
version: 2.0.0
---

# Tasks — v2 LLM strategy

**Stub.** This file is the analyst's milestone outline. Concrete
developer tasks (`T1901`, `T1902`, …) are **not enumerated yet** —
that is the architect's job after the four operator-decides
([feature.md → Q1, Q2, Q3, Q10](feature.md#notes--open-questions))
land and the architect publishes the Design section.

**Task numbering reservation:** **T19xx**.

The numbering history of the project:
- v0 → T0xx
- v0.5 → T5xx
- v1 → T6xx
- v1.5a → T7xx
- v1+ operator-success-reports → T8xx
- Lumen design adoption → T15xx / T16xx / T17xx
- v1.8 reflection-memory → T18xx
- **v2 LLM strategy → T19xx** (this brief)

T19xx is the natural next block; no namespace collisions.

**Architect:** when you land the Design section in
[feature.md → Design](feature.md#design), expand each milestone
below into ordered T19xx tasks, mirroring the granularity of
[reflection-memory tasks.md](../reflection-memory/tasks.md)
(~½ day per task; gate / synchronization-point annotations on the
critical-path tasks; explicit `[deps: T19yy]` lines).

## Scope assumption: foundation-only (Q1 = Option A)

Milestones below assume the analyst's Q1 = Option A scope
(foundation-only — LLM trait + 3 provider impls + cost wiring +
prompt-cache layer + budget gate + record/replay + smoke binary,
**no LLM consumers**). If the operator picks Option B/C/D, the
architect adds milestones M7+ for the bundled consumer(s):

- Q1 = Option B (foundation + news/sentiment overlay) → add
  **M7 — News/sentiment overlay strategy** + **M8 — Strategy
  backtest scenarios + anchor re-lock**.
- Q1 = Option C (foundation + post_mortem enrichment) → add
  **M7 — `reflection-memory-llm-enrichment` integration**
  (LLM `note` field, prompt design, replay-cache fixture
  extension, operator-success-report anchor re-lock).
- Q1 = Option D (foundation + multiple consumers) → architect
  scopes the per-consumer milestones explicitly; brief size 4–5x.

## M1 — `LlmProvider` trait + request/response types

Covers feature.md **R1** (trait shape + request type +
response type + error variants).

Architect resolutions landing here:
- **Q4** (trait shape — async, non-streaming, tool-use-from-day-one,
  batch-deferred, error variant set).

Outputs:
- New trait + types in `crates/llm/src/lib.rs` replacing the v0
  23-line stub.
- Trait + request + response + error rustdocs.
- `cargo build -p llm` + `cargo doc -p llm --no-deps` clean.

**Tasks T19xx will be expanded by the architect after Q4 resolution.**

## M2 — Provider implementations (Anthropic, OpenAI-compatible, Ollama)

Covers feature.md **R2** (three first-class providers).

Architect resolutions landing here:
- (none specific to provider impls; the trait shape from M1 is
  the input).

Outputs:
- `crates/llm/src/providers/anthropic.rs`.
- `crates/llm/src/providers/openai.rs`.
- `crates/llm/src/providers/ollama.rs`.
- `LlmProviderFactory::from_config(cfg: &LlmConfig)` at
  `crates/llm/src/factory.rs` reading the agent TOML to build
  the configured provider.
- Per-provider integration tests against `wiremock` mocks
  (Anthropic, OpenAI) and a mock Ollama server.
- `cargo test -p llm --features integration-test` clean.

**Tasks T19xx will be expanded by the architect.**

**Critical-path note:** M2 blocks M3, M4, M5, M6. Anthropic
provider impl in particular is the foundation for the prompt-
cache layer (M3) — if the Anthropic SDK shape evolves between
brief writing and developer pickup, the architect re-confirms
R2.1 against the live API at the start of M2.

## M3 — Prompt-cache layer + `CachedSystemPrompt` builder

Covers feature.md **R3** (cache-breakpoint placement +
builder shape + provider-aware emission +
cache-hit-rate metric).

Architect resolutions landing here:
- **Q5** (TTL-driven vs explicit invalidation; breakpoint count;
  trait-vs-sibling location; cache-hit-rate metric shape).

Outputs:
- `crates/llm/src/prompt_cache.rs` — `CachedSystemPrompt`
  builder with `(project_ctx, role_ctx, dynamic_ctx)` layered
  composition.
- `Vec<SystemBlock>` with optional `CacheBreakpoint` markers.
- Provider-aware translation: Anthropic emits real
  `cache_control` markers; OpenAI silently drops; Ollama no-op.
- `tracing` event for cache-hit-rate metric.
- Unit + integration tests.

**Tasks T19xx will be expanded by the architect after Q5
resolution.**

## M4 — Budget enforcement gate

Covers feature.md **R4** (pre-call check, post-call
reconciliation, model remap on degrade) + **R11** (cockpit
alert + memo + report line).

Architect resolutions landing here:
- **Q6** (gate placement: factory decorator vs in-impl vs
  explicit helper; pre-call estimate accuracy; concurrent-call
  race handling).
- **Q10** (cockpit alert surface — strawman: tile + memo + report
  line).

Outputs:
- `BudgetedProvider<Inner>` decorator at
  `crates/llm/src/budgeted.rs` (strawman placement — Q6).
- Pre-call check against `cost::CostBudget::mode_override()`.
- Atomic concurrent-call-safe spent-counter (Q6c).
- Post-call reconciliation that updates `CostBudget` from
  `ChatResponse::usage`.
- Model-remap-on-degrade logic per agent TOML's
  `[llm.deep_think]` / `[llm.quick_think]` model ids.
- `LlmError::BudgetExceeded` propagation.
- Cockpit "LLM budget" tile (R11.2 strawman) — architect's call
  on whether this lands here or in a sibling UI brief.
- Audit-ledger memo on budget events (R11.1).
- Unit tests for the degrade and block paths.

**Tasks T19xx will be expanded by the architect after Q6
resolution.**

## M5 — Tool-use schemas + cost-rate lookup + cost telemetry

Covers feature.md **R5** (tool-use schemas) + **R9** (cost
telemetry wired through, including the cost-rate provider
lookup) + **R12.1** (TOML config keys).

Architect resolutions landing here:
- **Q4e** (`serde_json::Value` vs typed schema; schema-validation
  library).
- **Q7** (cost-rate lookup: hard-coded match vs TOML override
  vs API metadata; module location — `cost` crate vs `llm`
  crate).

Outputs:
- `ToolSchema` type at `crates/llm/src/tools.rs`.
- JSON-schema validation pass on tool-use response blocks.
- Provider-specific tool-use translation (Anthropic + OpenAI
  native; Ollama best-effort with prose-validation fallback).
- Pricing module (analyst strawman:
  `crates/llm/src/pricing.rs`; architect may pick
  `crates/cost/src/pricing.rs`).
- `CostEvent::Llm` construction at the `BudgetedProvider`
  boundary (post-call reconciliation point).
- TOML config keys per R12.1.
- Integration tests asserting one LLM call → one balanced
  `expense:llm:<tier>` ↔ `liabilities:llm_accrued` journal pair.

**Tasks T19xx will be expanded by the architect after Q4e + Q7
resolutions.**

## M6 — Record/replay for research mode + smoke binary

Covers feature.md **R6** (record/replay) + **R10** (smoke
binary) + **R7** (rate-limit retries) + **R8** (API key
management) + **R13** (rustdoc + runbooks).

Architect resolutions landing here:
- **Q8** (request-hash schema; cache schema migration; cache
  size cap; fixture cache content; concurrent-write safety).
- **Q9** (retry budget; circuit-breaker decision; jitter
  formula).

Outputs:
- `RecordingProvider<Inner>` decorator + `ReplayProvider`
  at `crates/llm/src/replay.rs`.
- SQLite cache at `data/llm-replay.db` (schema-versioned).
- Fixture cache at `crates/llm/tests/fixtures/llm-replay.db`
  with one canned response per provider.
- Per-provider retry loop with exponential-backoff +
  full-jitter (R7).
- Env-var-only API-key reader at
  `crates/llm/src/auth.rs` (R8.1).
- `redact()` helper at `crates/llm/src/redact.rs` (R8.3).
- `cargo run --bin llm-smoke` binary at
  `crates/llm/src/bin/llm_smoke.rs` (R10).
- `cargo test --test smoke_test` against wiremock fixtures.
- Rustdoc on `lib.rs` lifting the v0 stub note (R13.1).
- `spec/runbooks/llm-cost.md` + `spec/runbooks/llm-replay.md`
  (R13.2 + R13.3).

**Tasks T19xx will be expanded by the architect after Q8 + Q9
resolutions.**

## M7 — Ship gate (VERDICT → PASS)

Covers feature.md **V1–V11** verification contract + **R14**
(no regression in non-LLM code paths) + **Q11**
(operator-success-report `LLM spend` denominator update from
`/$135` to `/$200`).

Architect resolutions landing here:
- **Q11** (denominator update — Option A in this brief vs Option
  B in first consumer brief vs Option C 1-line hotfix here;
  analyst recommendation: Option C).

Outputs:
- All verification gates green per V1–V11.
- 9 strategy-backtest anchor SHAs at
  [`spec/anchors.toml`](../anchors.toml) lines 15–58 byte-
  identical (R14.2).
- 2 operator-success-report anchor SHAs at lines 67–75 either
  byte-identical (Q11 Option B) or re-locked once (Q11 Option
  A or C).
- `cargo test --workspace --all-targets` green.
- Operator-invoked smoke (real API keys) green in operator's
  environment.
- Presenter deck at
  `spec/v2-llm-strategy/presentations/v2-llm-strategy-<date>.md`.

**Tasks T19xx will be expanded by the architect after V1–V11
gate is wired.**

## Notes

- This stub deliberately leaves T-numbers unenumerated.
  Spec-skill discipline: tasks expand only after the architect
  publishes the Design section.
- Foundation-only scope (Q1 = Option A) keeps the milestone
  count at **6 build milestones + 1 ship gate** (M1–M7). If
  Q1 lands Option B/C/D the architect appends consumer-
  milestones at M8+ and the brief grows accordingly.
- Per the project's `[parallelism rules]`
  ([AGENT.md](../../AGENT.md#parallelism-rules)), several M-
  level tasks parallelize across developers once M1 (the
  trait shape) lands: M2 (provider impls) and M3 (prompt-
  cache builder) and M6's pricing-module + auth-helper
  components can run in parallel under different developers.
  The architect lays out the parallelism gates explicitly when
  T19xx tasks expand.
- **No `[ui-designer]` tasks** under foundation-only scope. The
  cockpit "LLM budget" tile (R11.2) is the only UI surface
  this brief introduces; analyst's prior is the developer
  ships it as a single right-rail tile in the cockpit's
  header bar (Lumen Phase 6 Assistant slot is gated on this
  brief and ships separately, so no Phase 6 work happens in
  v2.0.0). Architect may push back and spawn a `[ui-designer]`
  for the tile if it's bigger than a single tile (Q10).

## Changelog

- 2026-05-10 (analyst): initial milestone stub (M1–M7) under
  Q1 = Option A scope assumption. T19xx namespace reserved.
  Tasks to be expanded by the architect after operator
  resolves Q1, Q2, Q3, Q10 and architect lands the Design
  section.
