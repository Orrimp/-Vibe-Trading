---
adr: 0019
title: v2 — LLM strategy foundation (Q4–Q11)
status: accepted
date: 2026-05-10
supersedes: none
superseded-by: none
---

# ADR-0019: v2 — LLM strategy foundation (Q4–Q11)

## Context

v2.0.0 ships LLM integration as **foundation-only** — no LLM
consumers ship in v2.0.0; the trait surface, three provider impls
(Anthropic / OpenAI-compatible / Ollama), prompt-cache builder,
budget gate with auto-degrade, record/replay for research mode,
tool-use schemas, and rate-limit handling all land together so
each consumer (Q1–Q3 in the brief; not architect-resolved yet) is
its own follow-up. The architect-round answered Q4–Q11 covering the
trait surface, prompt-cache, budget, cost-rate lookup, replay
storage, rate-limit handling, and one specific reports-side hot-fix
for the LLM-spend denominator.

## Decisions

### Q4 — Trait shape: async + non-streaming + tool-use-from-day-one

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: ChatRequest)
        -> Result<ChatResponse, LlmError>;
}
```

Streaming deferred to v3 (additive). Tool-use is mandatory at v2
per [`../product.md` § LLM strategy](../../product.md#llm-strategy);
delaying = breaking change later. Batch deferred. Schemas as
`serde_json::Value` validated by `jsonschema`; typed schemas are a
consumer-side ergonomic via `schemars`. `LlmError` has 8 variants:
`Provider | RateLimited | Timeout | BudgetExceeded |
InvalidResponse | ReplayMiss | Network | Auth`. Cost-crate rename:
the `cost` crate's `LlmProvider` enum (provider id) is renamed
`ProviderKind` to free the trait name. Mechanical rename only.

### Q5 — Prompt-cache strategy: TTL + 2 breakpoints + provider-aware builder

TTL-driven cache with two breakpoints (system, last-user). The
builder is provider-aware — Anthropic gets `cache_control` blocks,
OpenAI-compatible gets the standard prompt prefix, Ollama gets a
no-op pass-through. Per-role cache-hit-rate Prometheus counter
pair (`hits` + `misses`) for operator visibility.

### Q6 — Budget gate: `BudgetedProvider<Inner>` decorator + AtomicU64 cents

Factory-level `BudgetedProvider<Inner: LlmProvider>` decorator
wraps the provider. `AtomicU64` cents counter. 0.2% documented
overshoot bound (a request mid-flight at the moment the budget
trips is allowed to complete). On exceeded: `LlmError::BudgetExceeded`
returned without calling the inner provider.

### Q7 — Cost-rate lookup: hybrid table + TOML override

Hard-coded base table at `crates/llm/src/pricing.rs` for the three
providers' published rates at v2 ship. TOML override at
`config/llm-pricing.toml` for ops-time updates without recompile.
TOML wins on conflict.

### Q8 — Replay storage: SQLite WAL + canonical-JSON SHA-256

Replay records in a SQLite database (separate from the audit
ledger) in WAL mode. Request canonicalisation: stable JSON with
sorted keys, body-SHA-256 indexed. `schema_version` migration table
for future shape changes. 9-row fixture for unit tests. Strict
replay only at v2.0.0 — a request that doesn't hit the replay
cache returns `LlmError::ReplayMiss` in replay mode.

### Q9 — Rate-limit handling: exponential backoff + full jitter, 3 retries

Exponential backoff with full jitter, max 3 retries. No circuit
breaker at v2.0.0. `Retry-After` header honored when present.
Beyond 3 retries → `LlmError::RateLimited` to the caller.

### Q11 — Operator success report LLM-spend denominator: Option C hot-fix

The `report-sample-7d` / `report-sample-90d` anchors lock against
a `denominator` field that needs a one-line hot-fix to handle the
v2 cost-counter shape. **Option C ratified** — fix in this brief;
the two `report-sample-*` anchors re-lock **once** at
`T_FINAL_V2_LLM_STRATEGY`. Anchors do not mutate after that.

## Alternatives considered

- **Streaming at v2.0.0.** Adds significant complexity to the
  trait and the `BudgetedProvider` decorator. Rejected.
- **Tool-use as a separate breaking-change milestone.** Forces v3
  to break the trait. Rejected — pay the cost once at v2.
- **`LlmError` as a single variant with a code.** Surrenders
  exhaustive `match` at consumer sites. Rejected.
- **Per-request budget enforcement.** O(N) overhead per request
  vs O(1) for the AtomicU64 counter. Rejected.
- **TOML-only cost table** (no hard-coded base). Operator
  bootstrapping friction; ship-day pricing should be in code.
  Rejected in favour of hybrid.

## Consequences

- The 8-variant `LlmError` becomes the load-bearing surface for
  every LLM consumer. Adding a variant later = breaking change to
  every consumer.
- Two anchors re-lock once at v2 ship (Q11). After that, the
  `report-sample-*` body bytes are immutable until a future ADR
  supersedes this one.
- The `ProviderKind` rename ripples through `crates/cost` and
  every site that referenced the old `LlmProvider` enum. Mechanical
  but invasive.
- Replay infra is now project-load-bearing for v2 LLM testing.
  Future LLM consumers must commit to the strict-replay-only rule
  or supersede this ADR.

## Changelog
- 2026-05-10 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  v2 — LLM strategy resolutions during Phase 1A Session 10.
- 2026-05-29 (architect): v2.1 tracing-Layer redactor M-T1 ratified
  (`REQ-V2-1-TRACING-LAYER-REDACTOR-001`). Closes the pass-3
  deferred half of R8.3 secret-redaction documented at
  `crates/llm/src/redact.rs:18-26`. Layer shape: closed 9-rule
  regex set + per-site marker-field opt-out (NO bypass allowlist)
  + 14-day WARN mode via `REDACT_LAYER_MODE=warn|gate` env var
  before v0.2.0 gate-default flip. Layer module at
  `crates/llm/src/redact_layer.rs` (co-located with the pure-fn
  `redact()` per R-NR.1 reuse). Wire-up via new
  `llm::tracing_init::install_global()` helper called by 17
  binary entry points (architect-audit finding: every existing
  binary uses single-Layer `fmt().init()` shape, must migrate
  to `registry().with(...)` composition). Provider-header bypass
  lives at `reqwest` wire layer (Q-RED-2 (a) DURABLE), not in
  the redactor. Anchor contract zero delta — 75/75 byte-identical
  pre/post per R-NR.3 hard gate. NO new ADR; this Changelog row
  is the architectural record. See
  [`spec/v2-1-tracing-layer-redactor/feature.md ## Design`](../../v1/v2-1-tracing-layer-redactor/feature.md#design)
  for D-RED-1..D-RED-9.
