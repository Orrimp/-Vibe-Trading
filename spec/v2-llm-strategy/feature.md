---
slug: v2-llm-strategy
status: in-progress
owner: analyst
updated: 2026-05-10
version: 2.0.0
---

# v2 LLM strategy

## Why

This brief promotes the **v2 LLM strategy** queue item from
[backlog.md → Active](../backlog.md#active) (promoted 2026-05-10)
into a real feature. It is the **project's first LLM integration**
— the v2-era kickoff — and likely the largest-scope feature shipped
to date.

Two crates have been **pre-positioned** for this moment, both for
two minor versions, neither carrying any callers:

1. [`crates/llm/`](../../crates/llm/) — a 23-line stub from v0
   ([`crates/llm/src/lib.rs`](../../crates/llm/src/lib.rs)). One
   trait `LlmProvider` with `name(&self)`, one `LlmError` enum
   with three variants. Zero impls, zero callers. Line 3 reads
   "v0 stub — no LLM calls are made in v0."
2. [`crates/cost/`](../../crates/cost/) — the **fully wired
   surface** from v0. `CostEvent::Llm { provider, model, tier,
   role, tokens_in, tokens_out, tokens_cached_in, usd,
   correlation_id }` ([event.rs](../../crates/cost/src/event.rs)),
   `CostSink` trait, `CostBudget` with auto-degrade at 80% / block
   at 100% already implemented at
   [`crates/cost/src/budget.rs:40-53`](../../crates/cost/src/budget.rs),
   and `LedgerCostSink` writing balanced entries to
   `expense:llm:<tier>` + `liabilities:llm_accrued`. v0 posts
   zero entries; the chart of accounts is populated. Architecture
   rationale at
   [architecture.md → Cost telemetry](../architecture.md#cost-telemetry--dedicated-cost-crate--confirmed-2026-04-17)
   (lines 2870–2922): "`llm` depends on `cost` (at v0.5)."

So the **scaffolding has been waiting**. v2 wires the actual LLM
calls through it.

The contract this feature inherits is set by four product sections:

1. [product.md → LLM strategy](../product.md#llm-strategy)
   (lines 240–258) — names the **dual-tier model use** (deep_think
   + quick_think), the **provider abstraction** (Anthropic
   default, OpenAI-compatible, Ollama), and four **cost controls**
   (hard monthly token budget per role, mandatory prompt caching
   on stable system prompts, tool-use schemas instead of
   free-text parsing, cheaper-tier auto-selected when input
   confidence is high).
2. [product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)
   (lines 332–347) — **v2 LLM monthly ceiling = $200** inside a
   $360 total opex. Hard rule: at 80% spend the agent restricts
   to `quick_think`; at 100% it reverts to deterministic-only
   mode and raises a cockpit alert. Ladder confirmed 2026-04-17.
3. [product.md → Trading-time agent roster](../product.md#trading-time-agent-roster)
   (lines 105–168) — names ten LLM-driven agent roles across the
   five-layer hierarchy. The roster is the **consumer
   landscape**; the scope-split decision (Q1) picks which (if
   any) of these consumers ship in v2.0.0 vs follow-up briefs.
4. [product.md → Strategy library — roadmap](../product.md#strategy-library--roadmap)
   (lines 171–186) — the v2 row reads **"LLM-augmented
   news/sentiment overlay — First LLM-in-the-loop strategy"**.
   Bundling this overlay into v2.0.0 is one of the live
   alternatives Q1 surfaces.

**Inherited dependency from v1.8.0.** When reflection-memory
shipped 2026-05-08, its **Q1 was resolved Option A — deterministic
v1, LLM enrichment deferred** to "after v2 LLM ships"
([reflection-memory feature.md Q1](../reflection-memory/feature.md#q1--llm-driven-post_mortem_analyst-vs-deterministic-v1)).
Two follow-up briefs are blocked on this one:
**`reflection-memory-llm-enrichment`** (the deferred
post_mortem_analyst LLM-rewrite of the lesson card `note` field)
and **`reflection-memory-trader-wiring`** (top-K retrieval into
the trader, deferred from
[reflection-memory Q4](../reflection-memory/feature.md#q4--retrieval-at-decision-time-trader-vs-report-only)).

**Terms-of-art (one-line glosses):**

- **Deep-think tier** — the more capable, more expensive LLM for
  trader decisions, researcher debates, and post-trade analysis.
  Strawman default per Q2: Claude Opus 4.7 (`claude-opus-4-7`).
- **Quick-think tier** — the cheaper, faster LLM for analyst
  summaries, news classification, high-volume structured calls.
  Strawman default per Q2: Claude Haiku 4.5
  (`claude-haiku-4-5-20251001`).
- **Provider** — a concrete LLM API behind the Rust trait. Three
  first-class per product.md: Anthropic, OpenAI-compatible
  (covers OpenAI, OpenRouter, DeepSeek, LM Studio), Ollama.
- **Prompt caching** — Anthropic's API feature giving ~75%
  discount on repeated input tokens for 5 minutes after a cache
  hit. Applied via **cache breakpoints** at stable boundaries.
- **Cache breakpoint** — an Anthropic API marker meaning "cache
  everything from the previous breakpoint up to here for 5
  minutes." Up to 4 per request.
- **Tool-use schema** — a structured-output contract (declared
  JSON schema) the LLM fills in instead of emitting prose.
  product.md mandates this over free-text parsing.
- **`LedgerCostSink`** — the existing v0 sink at
  [`crates/cost/src/sink.rs`](../../crates/cost/src/sink.rs)
  that posts a balanced double-entry pair against
  `expense:llm:<tier>` + `liabilities:llm_accrued` per
  `CostEvent::Llm`.
- **Budget gate** — the pre-call check against
  `CostBudget::mode_override()` that downgrades deep_think to
  quick_think at 80% spend or blocks at 100%.
- **Record/replay** — the deterministic-replay path required by
  [product.md → Operating modes](../product.md#operating-modes)
  line 292 ("Research — backtest only, deterministic seeds, no
  LLM cost (cached responses replay)"). A `RecordingProvider`
  records `(request_hash, response)` pairs to SQLite; a
  `ReplayProvider` reads them back and panics on a miss.

**Scope decision — surfaced as Q1 (load-bearing).** This brief's
single most important decision is whether v2 ships
**foundation-only** (LLM trait + 3 provider impls + cost wiring +
prompt-cache layer + budget gate + record/replay + a single
`cargo run --bin llm-smoke` end-to-end test) and lets each LLM
consumer become a follow-up brief, OR **bundles** one or more
consumers (post_mortem enrichment, news/sentiment overlay, trader
debate) into the first ship. **All R-items below assume the
foundation-only scope; if the operator picks "foundation + N
consumers", the architect re-scopes.**

This feature is **the LLM substrate made callable**. After it
ships, every queued LLM consumer drops in as an R-level addition
on a stable trait surface, instead of re-litigating the trait
shape on each follow-up brief.

## Requirements

Numbered, testable, derived from the four product.md sections
above and the existing scaffolding. Each ends with a one-line
**acceptance** the tester can verify. All requirements preserve
the existing `Strategy` trait shape (no trait changes), the
audit chart of accounts (no new accounts; `expense:llm:*` and
`liabilities:llm_accrued` already exist per v0 R3.2), the 9
locked strategy-backtest anchor SHA-256s (no impact —
non-strategy under Q1 = Option A), and the 2 locked
operator-success-report anchors (the only changing byte is the
`LLM spend` line — see Q11).

This feature is **non-strategy in v2.0.0** under the analyst's
foundation-only Q1 recommendation. If the operator picks
"foundation + news/sentiment overlay" the strategy-impact
clause lifts and the architect adds a strategy-side R-section.

### R1 — `LlmProvider` trait + request/response types

- **R1.1** Replace the v0 stub with a real trait. Shape is
  **architect's call** (Q4); the analyst's strawman:
  ```rust
  #[async_trait::async_trait]
  pub trait LlmProvider: Send + Sync {
      fn name(&self) -> &str;
      fn provider_kind(&self) -> ProviderKind;
      async fn complete(
          &self,
          request: ChatRequest,
      ) -> Result<ChatResponse, LlmError>;
  }
  ```
  Async (every consumer is in a tokio task), non-streaming at
  v2 (streaming is a v3 follow-up), tool-use included from day
  one (product.md mandates structured output), batch deferred
  (consumer-brief concern when it lands).
- **R1.2** `ChatRequest` carries: `model: ModelId`, `tier:
  LlmTier` (re-uses [`cost::LlmTier`](../../crates/cost/src/event.rs)),
  `role: AgentRole` (re-uses `cost::AgentRole`), `system:
  Vec<SystemBlock>` (cache-breakpoint-aware, composed by R3
  builder), `messages: Vec<ChatMessage>`, `tools:
  Vec<ToolSchema>` (R5; empty for free-text), `max_tokens:
  u32`, `temperature: Option<f32>`, `correlation_id: Uuid`.
- **R1.3** `ChatResponse` carries: `content:
  Vec<ContentBlock>` (text and/or tool-use blocks),
  `stop_reason: StopReason`, `usage: TokenUsage`
  (`tokens_in / tokens_out / tokens_cached_in` — field-for-
  field rename of the
  [`CostEvent::Llm`](../../crates/cost/src/event.rs) fields),
  `model: ModelId` (echoed; OpenAI-compatible providers may
  route across models), `correlation_id: Uuid` (echoed).
- **R1.4** `LlmError` extends the v0 stub minimally — variants
  are **architect's call** (Q4); strawman: keep `Provider`,
  `RateLimited`, `Timeout`; add `BudgetExceeded` (R4),
  `InvalidResponse(String)` (R5 schema-validation failure),
  `ReplayMiss(String)` (R6), `Network(reqwest::Error)`,
  `Auth(String)` (R8 missing key).
- **Acceptance:** `cargo build -p llm` clean; `cargo doc -p llm
  --no-deps` warning-clean; unit test asserts
  `ChatRequest::new(model, tier, role)` builds with sensible
  defaults; unit test asserts every `LlmError` variant has a
  non-empty `Display` impl.

### R2 — Provider implementations

- **R2.1** **Anthropic provider** at
  `crates/llm/src/providers/anthropic.rs`. Uses the official
  Anthropic Rust SDK if it exists at v2 ship time, otherwise
  raw `reqwest` against `api.anthropic.com/v1/messages`.
  Supports prompt caching (R3) — sends `cache_control:
  {"type": "ephemeral"}` markers. Supports tool use (R5).
  Reports `tokens_cached_in` from the response's
  `cache_read_input_tokens` field.
- **R2.2** **OpenAI-compatible provider** at
  `crates/llm/src/providers/openai.rs`. Uses the
  `async-openai` crate (named in
  [architecture.md line 4386](../architecture.md)).
  Configurable base URL covers OpenAI, OpenRouter, DeepSeek,
  LM Studio. Supports tool use via the OpenAI tool-calling
  API. **Cache markers silently dropped** (logged at
  `tracing::debug!`); `tokens_cached_in = 0`. Architect may
  pick a per-provider cache-support matrix in Q5.
- **R2.3** **Ollama provider** at
  `crates/llm/src/providers/ollama.rs`. Uses `ollama-rs` or
  raw `reqwest` against `localhost:11434/api/chat`. **No
  prompt caching, best-effort tool-use** (JSON-validate the
  prose; fall back to `LlmError::InvalidResponse`). Cost is
  always `$0.00`; the cost event still fires with
  `tokens_in/out` so the operator sees throughput.
- **R2.4** **Provider factory.**
  `LlmProviderFactory::from_config(cfg: &LlmConfig)` is the
  single place that reads API keys (R8); consumers receive
  `Arc<dyn LlmProvider>` and never touch credentials.
- **Acceptance:** for each provider, an integration test
  against `wiremock` (Anthropic, OpenAI) or a mock Ollama
  server asserts (a) a `ChatRequest` produces a correctly-
  shaped HTTP request, (b) a canned response parses into a
  `ChatResponse` with correct `usage`, (c) a simulated
  rate-limit surfaces as `LlmError::RateLimited` after R7's
  retry budget. `cargo test -p llm --features
  integration-test` clean.

### R3 — Prompt-cache strategy + `CachedSystemPrompt` builder

- **R3.1** A `CachedSystemPrompt` builder layers system
  prompts as `(project_ctx, role_ctx, dynamic_ctx)`:
  - **Project context** — stable across deployment (~1k
    tokens of "you are a trading agent inside a Rust crypto-
    trading binary, the audit ledger is double-entry SQLite,
    your decisions feed into the executor…"). Cached at the
    **first** breakpoint.
  - **Role context** — stable per agent role (~1k tokens of
    "you are the `sentiment_analyst`, classify the following
    N tweets into the five-tier scale…"). Cached at the
    **second** breakpoint.
  - **Per-call dynamic** — current ledger snapshot, current
    tweets, etc. Not cached.
- **R3.2** Cache-breakpoint placement strategy is
  **architect's call** (Q5): TTL-driven (rely on the 5-minute
  window) vs explicit invalidation. Strawman: TTL-driven at
  v2.0.0; explicit invalidation lands when a prompt-library
  pipeline lands (post-v2 follow-up).
- **R3.3** The builder is **provider-aware**: Anthropic
  emits real cache breakpoints; OpenAI silently drops; Ollama
  no-op. Consumers always compose against the builder; the
  provider quietly decides whether the markers translate to
  a cache hit.
- **R3.4** A `tracing` event `llm.cache.hit_ratio` aggregates
  per role per day; surfaced in the operator-success-report's
  System Health line as a percentage. Architect to finalise
  the metric shape (Q5).
- **Acceptance:** unit test asserts the builder produces
  byte-stable `Vec<SystemBlock>` for the same input; an
  Anthropic-mock integration test asserts the request body
  contains exactly two `cache_control: {"type": "ephemeral"}`
  markers; an OpenAI-mock test asserts the markers are
  absent from the request body.

### R4 — Budget enforcement gate

- **R4.1** Every LLM call routes through a **pre-call
  budget check** against
  [`CostBudget::mode_override()`](../../crates/cost/src/budget.rs):
  - `Some(DeepThink)` (spend < 80%) → call proceeds as-is.
  - `Some(QuickThink)` (spend ≥ 80%) → request `tier`
    downgraded to `QuickThink`, `model` remapped to the
    configured `quick_think` model (R4.4),
    `tracing::warn!("llm.budget.degrade_to_quick_think")`
    fires, call proceeds.
  - `None` (spend ≥ 100%) → call **blocked**;
    `LlmError::BudgetExceeded` returned without HTTP
    request; cockpit alert fires (R11 / Q10).
- **R4.2** **Pre-call estimate** uses a token-counter
  helper (Anthropic's `count_tokens` endpoint; OpenAI's
  `tiktoken` rules). Estimate added to spent counter
  **only after the call succeeds** — failed pre-call
  estimates are not billed.
- **R4.3** **Post-call reconciliation** updates the
  spent counter with actual `tokens_in + tokens_out`
  (cached-in tokens at the discounted price). The
  reconciliation is the source of truth.
- **R4.4** **Model remap on degrade** reads the agent
  TOML's `[llm.deep_think]` / `[llm.quick_think]`
  ([product.md line 321](../product.md#configuration-surface)).
  System prompt is **not** rewritten; same prompt → cheaper
  model. (Consumers needing tier-specific prompts compose
  two `CachedSystemPrompt` builders and pick at the call
  site.)
- **R4.5** **Where in the call path the gate fires** is
  **architect's call** (Q6): factory-level decorator
  (`BudgetedProvider<Inner>` — strawman) vs in-impl vs
  explicit consumer-side helper.
- **Acceptance:** unit test seeds budget at $179.99 / $200,
  asserts deep-think downgrades to quick-think with model
  remap; second test seeds $200.01, asserts
  `LlmError::BudgetExceeded` with **zero** outbound HTTP
  calls; third test seeds $0.00, asserts deep-think passes
  through untouched.

### R5 — Tool-use schemas

- **R5.1** Tool-use is **first-class** at v2 per
  [product.md line 257](../product.md#llm-strategy). Request
  carries `tools: Vec<ToolSchema>`; response carries
  interleaved `ContentBlock::ToolUse { name, input }` and
  `ContentBlock::Text { ... }`.
- **R5.2** **JSON Schema is the lingua franca.** A
  `ToolSchema` is `(name, description, input_schema:
  serde_json::Value)`. Consumers compose schemas with
  `schemars` (analyst strawman; architect picks the validator
  in Q4).
- **R5.3** **Schema-validation pass** on every tool-use
  response: `input` is JSON-schema-validated before being
  surfaced. Failures → `LlmError::InvalidResponse(msg)`.
  Consumers never see malformed tool-use input.
- **R5.4** **Ollama best-effort.** Ollama doesn't enforce
  tool-use schemas server-side; the impl validates the
  model's prose, falling back to `InvalidResponse` if the
  model didn't conform. Consumers should not rely on tool-
  use precision when running against Ollama (the local-dev
  path).
- **Acceptance:** unit test composes a
  `propose_trade(side, size, confidence)` schema, mocks an
  Anthropic response with a valid `ToolUse` block, asserts
  the consumer receives the parsed `input`; second test
  mocks a response with a schema-violating `input`, asserts
  `LlmError::InvalidResponse`.

### R6 — Record/replay for research mode

- **R6.1** A `RecordingProvider<Inner: LlmProvider>`
  decorator on every successful `complete(request)`:
  (1) hashes the request (SHA-256 over canonical JSON of
  `(model, system, messages, tools, max_tokens,
  temperature)`; `correlation_id` excluded so the same
  prompt from different consumer runs hits the same cache
  entry), (2) forwards to inner, (3) writes
  `(request_hash, response)` to SQLite at
  `data/llm-replay.db` (Q4 / Q8 — config-overridable).
- **R6.2** A `ReplayProvider` reads from the same cache.
  Cache miss → `LlmError::ReplayMiss(hash)`. Strict-replay
  consumers (the backtest binary in research mode)
  propagate the error; best-effort consumers can fall
  through to a real provider (architect's call on whether
  this fall-through ships in v2 or v3).
- **R6.3** **Operating-mode wiring.** The agent TOML's
  `mode: research | paper | live`
  ([product.md line 327](../product.md#configuration-surface))
  picks the variant:
  - `research` → `ReplayProvider` (cache-only; misses fatal).
  - `paper` → `RecordingProvider<Anthropic | OpenAI |
    Ollama>` (real call + record).
  - `live` → bare provider (no recording overhead in prod).
- **R6.4** **Test fixture cache** at
  `crates/llm/tests/fixtures/llm-replay.db` is **versioned
  in git** so `cargo test --workspace` is deterministic.
  The runtime cache at `data/llm-replay.db` is git-ignored.
- **R6.5** Re-recording the same hash is an **idempotent
  overwrite** (last write wins) so the dev fixing prompt
  drift sees a clean refresh path. Logs every overwrite at
  `tracing::info!`.
- **Acceptance:** unit test runs `RecordingProvider<Mock>`,
  asserts a row landed in the cache, then runs
  `ReplayProvider` against the same request and asserts
  byte-identical response; second test runs `ReplayProvider`
  against an un-cached hash and asserts
  `LlmError::ReplayMiss`.

### R7 — Rate-limit handling + retries

- **R7.1** Every provider HTTP call wraps in a retry loop
  with **exponential backoff and full jitter**:
  - Up to 3 retries on `429` or `503`.
  - Backoff base 500ms, cap 8s, formula
    `sleep = random(0, min(cap, base * 2^attempt))`.
  - After 3 retries, returns `LlmError::RateLimited`.
- **R7.2** **No circuit breaker at v2.0.0.** Defer to v3
  once provider-failure-rate observability exists.
  Architect may push back (Q9).
- **R7.3** **Network errors propagate immediately as
  `LlmError::Network`** (no retry) — transport-level
  failures usually indicate config problems retries don't
  fix.
- **Acceptance:** unit test mocks 3×429 → 200 and asserts
  the call succeeds within `8s + 4s + 2s + jitter`; second
  test mocks 4×429 and asserts `LlmError::RateLimited` after
  exactly 3 retries.

### R8 — API key management

- **R8.1** Keys read from **environment variables only** at
  v2.0.0: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
  `OPENROUTER_API_KEY`, `DEEPSEEK_API_KEY`. Ollama needs no
  key. Per [CLAUDE.md line 84](../../CLAUDE.md): "No secrets
  in git. Keys in env / secret store per architecture.md."
- **R8.2** **Missing key is a fatal startup error.** The
  factory reads the configured provider's key at build time;
  unset → `LlmError::Auth("ANTHROPIC_API_KEY not set")`,
  agent main exits non-zero with a clear message.
- **R8.3** **Keys never leak into logs, traces, audit
  ledger, or test fixtures.** A `redact()` helper at
  `crates/llm/src/redact.rs` is the single `Display` path
  for keys (always prints `sk-ant-...***` with the last 4
  characters elided).
- **R8.4** **Test fixtures use mock providers** (R6 +
  `wiremock`); production secrets never appear in
  `cargo test`. CI sets `ANTHROPIC_API_KEY=sk-ant-test-stub`
  so the build doesn't hard-fail when keys are intentionally
  not provided.
- **R8.5** **Secret-store integration is deferred** to v3
  (`llm-secret-store` follow-up brief, surfaced in Q3).
- **Acceptance:** unit test asserts `LlmProviderFactory::build`
  with `ANTHROPIC_API_KEY` unset returns `LlmError::Auth`;
  second test asserts `redact("sk-ant-secret-12345")` does
  **not** contain `secret-12345`; CI grep confirms zero
  secret strings in `target/logs/*.log` after a smoke run.

### R9 — Cost telemetry wired through

- **R9.1** Every successful call posts a `CostEvent::Llm` to
  the configured `CostSink`. Field population: `provider`
  from the impl, `model` from `ChatResponse::model` (echoed),
  `tier` from the (possibly degraded) request, `role` from
  the request, `tokens_*` from `ChatResponse::usage`, `usd`
  from R9.2 lookup, `correlation_id` from the request.
- **R9.2** **Cost-rate lookup** is a `match (provider, model)`
  table (architect's call on module location — Q7). Strawman
  v2.0.0 entries (USD per million tokens):
  - Anthropic Claude Opus 4.7: input $15, output $75,
    cached-input $1.50.
  - Anthropic Claude Haiku 4.5: input $1.00, output $5.00,
    cached-input $0.10.
  - OpenAI GPT-5: TBD at architect time (operator confirms
    current price at handoff).
  - Ollama (any model): $0 across the board.
  Hard-coded base table + TOML override at
  `config/agent.toml` for ephemeral changes. Reviewed
  quarterly during the operator's costs.md run.
- **R9.3** **Failed calls post no cost event** — only
  successful calls are billed. (Anthropic and OpenAI both
  bill on the request; the v2 stance is "we eat intermittent
  network errors as goodwill" until operator feedback says
  otherwise. Architect may revisit — Q7.)
- **R9.4** The existing `LedgerCostSink`
  ([sink.rs](../../crates/cost/src/sink.rs)) is the
  production sink: `Arc::new(LedgerCostSink::new(ledger))`.
  Tests use `NoopCostSink`.
- **R9.5** The operator-success-report's **`LLM spend:
  $X.XX / $200`** line now reflects real spend. Per
  [operator-success-reports R8.4](../operator-success-reports/feature.md#r8--no-regression-in-non-report-code-paths)
  the line currently reads `$0.00 / $135`; v2 updates the
  denominator to `$200` (Q11) and the numerator becomes
  non-zero in `paper` / `live` modes. In `research` mode,
  R6.3 + R9.3 mean the replay provider posts zero events,
  so research backtests still anchor against `$0.00`.
- **Acceptance:** integration test fires one call against a
  mock returning `usage: {tokens_in: 1000, tokens_out: 200,
  tokens_cached_in: 500}`, asserts one `CostEvent::Llm` lands
  in `LedgerCostSink` with the expected USD per the
  strawman pricing ($0.000075 cached + $0.0075 uncached
  input + $0.015 output = ~$0.0226); second test fires a
  failing call and asserts zero cost events recorded.

### R10 — End-to-end smoke binary

- **R10.1** New binary at
  `crates/llm/src/bin/llm_smoke.rs`. Round-trips one prompt
  ("Reply with the literal string `OK` and nothing else.")
  through each configured provider; prints a result table:
  ```
  provider     model              tokens_in tokens_out usd     latency_ms result
  anthropic    claude-opus-4-7    23        1          $0.00   847        OK
  openai       gpt-5              23        1          $0.00   523        OK
  ollama       llama3.3:70b       19        1          $0.00   1241       OK
  ```
  Exits 0 if all returned `OK`; 1 otherwise.
- **R10.2** **Smoke run is paper-mode by default** —
  records to `data/llm-replay.db` so subsequent
  `--mode research` runs replay the same fixture.
- **R10.3** **No real API calls during `cargo test`** —
  the smoke binary has its own
  `tests/smoke_test.rs` integration test that runs against
  `wiremock` mocks. Real `cargo run --bin llm-smoke` is
  operator-invoked, not tester-invoked.
- **Acceptance:** `cargo build --bin llm-smoke` clean;
  `cargo test --test smoke_test` end-to-end against
  wiremock fixtures green; rustdoc on `llm_smoke.rs`
  documents the table format.

### R11 — Cockpit alert on budget events

- **R11.1** Budget gate (R4.1) degrade or block fires an
  audit-ledger memo at `expense:llm:*` with a `tag`
  carrying the event type (`budget_degrade_to_quick_think |
  budget_block`).
- **R11.2** Cockpit "LLM budget" tile shows current-month
  spend as `$143.21 / $200 = 71.6%`, flips colour at the
  80% and 100% thresholds. Implementation is **architect's
  call** (Q10) — strawman: a single right-rail tile in the
  cockpit's header bar. **Lumen Phase 6 Assistant slot is
  NOT wired in this brief** (Phase 6 is its own follow-up
  brief gated on this one shipping).
- **R11.3** Next operator-success-report after a budget
  event includes a one-line entry in System Health:
  `Budget event: degraded to quick_think at 14:23 UTC on
  2026-06-12`.
- **R11.4** **No email/Slack/push at v2.0.0.** Surface
  outside the running process is a v3 follow-up — Q10.
- **Acceptance:** integration test against a fixture ledger
  with $179 / $200 of synthetic spend fires a degrade and
  asserts (a) memo lands with `budget_degrade_to_quick_think`
  tag, (b) next-rendered report's System Health contains the
  degrade-event line; UI smoke test asserts the cockpit tile
  shows the correct percentage.

### R12 — Configuration surface

- **R12.1** New TOML keys at `config/agent.toml`:
  ```toml
  [llm]
  budget_usd_month = 200.0
  default_provider = "anthropic"
  replay_cache_path = "data/llm-replay.db"

  [llm.deep_think]
  provider = "anthropic"
  model = "claude-opus-4-7"

  [llm.quick_think]
  provider = "anthropic"
  model = "claude-haiku-4-5-20251001"

  [llm.providers.anthropic]
  base_url = "https://api.anthropic.com/v1"
  # api_key sourced from $ANTHROPIC_API_KEY (R8.1)

  [llm.providers.openai]
  base_url = "https://api.openai.com/v1"

  [llm.providers.ollama]
  base_url = "http://localhost:11434"
  ```
- **R12.2** Config validation at startup. Missing required
  keys → fatal startup error. Unknown keys →
  `tracing::warn!` (forward-compat for follow-up briefs).
- **R12.3** **No new bus channels.** LLM providers are
  call-and-return. (Specific consumers may add channels in
  follow-up briefs.)
- **Acceptance:** unit test against
  `config::Config::from_toml(...)` asserts v2 keys parse and
  missing keys produce a clear error; second test asserts
  agent startup hard-fails cleanly when `ANTHROPIC_API_KEY`
  is unset under `default_provider = "anthropic"`.

### R13 — Documentation

- **R13.1** Crate-level rustdoc at `crates/llm/src/lib.rs:1`
  enumerates trait, three providers, prompt-cache builder,
  budget gate, record/replay. Replaces the v0 stub note.
- **R13.2** New runbook `spec/runbooks/llm-cost.md`: how to
  read the report's `LLM spend` line, what the operator does
  on a degrade event, how to update cost-rate entries, how
  to swap providers (TOML edit + restart).
- **R13.3** New runbook `spec/runbooks/llm-replay.md`: how
  research mode uses replay, how to refresh the cache (run
  `cargo run --bin llm-smoke --mode paper`), how to interpret
  a `ReplayMiss` failure in a backtest.
- **Acceptance:** `cargo doc --workspace --no-deps`
  warning-clean; both runbooks exist and pass `markdownlint`.

### R14 — No regression in non-LLM code paths

- **R14.1** **No `Strategy` trait change** under Q1 = Option
  A. (Option B/C/D add a strategy-side R-section.)
- **R14.2** **9 strategy-backtest anchor SHAs at
  [`spec/anchors.toml`](../anchors.toml) lines 15–58 stay
  byte-identical** under Q1 = Option A.
- **R14.3** **2 operator-success-report anchors at lines
  67–75** either stay byte-identical (Q11 Option B — defer
  re-lock to first consumer brief) or re-lock once in this
  brief (Q11 Option A or C — architect's call).
- **R14.4** **No real-API dependency in `cargo test
  --workspace`** — all tests use mock providers.
- **Acceptance:** the 9 strategy anchors stay byte-identical;
  `cargo test --workspace --all-targets` green; CI assertion
  of zero outbound HTTPS connections to `*.anthropic.com /
  *.openai.com / *.openrouter.ai / *.deepseek.com` during
  the test run.

## Backtest Scenarios

This feature is **non-strategy** under Q1 = Option A — does not
validate edge. Scenarios below are **smoke-test scenarios** for
provider integrations, not edge-validation backtests. The 9
strategy-backtest anchors are out of scope (R14.2).

### Scenario: `llm-smoke-anthropic`

- **Transport:** `wiremock` mocking
  `api.anthropic.com/v1/messages`.
- **Request:** R10 prompt — "Reply with the literal string
  `OK` and nothing else."
- **Expected response:** `OK`.
- **Cost event:** one `CostEvent::Llm` with `provider:
  Anthropic, model: "claude-opus-4-7", tokens_in: 23,
  tokens_out: 1, tokens_cached_in: 0`, USD per R9.2 strawman.

### Scenario: `llm-smoke-openai`

Same shape against `wiremock` mocking
`api.openai.com/v1/chat/completions`.

### Scenario: `llm-smoke-ollama`

Same shape against a local mock Ollama server replying to
`/api/chat`. USD = $0.

### Scenario: `llm-replay-roundtrip`

- **Phase 1 (record):** `RecordingProvider<MockAnthropic>`
  runs the R10 prompt; cache row lands in
  `crates/llm/tests/fixtures/llm-replay.db`.
- **Phase 2 (replay):** `ReplayProvider` reads the same hash
  and returns the recorded response without calling the
  inner provider.
- **Cache miss case:** `ReplayProvider` against an un-cached
  prompt returns `LlmError::ReplayMiss(hash)`.

### Scenario: `llm-budget-degrade`

- **Fixture ledger:** $179 of synthetic LLM spend (89.5% of
  $200 ceiling).
- **Request:** `ChatRequest{tier: DeepThink}`.
- **Expected behaviour:** downgrade to `QuickThink` with
  model remap; call proceeds; `tracing::warn!` fires; audit
  memo (R11.1) lands.

### Scenario: `llm-budget-block`

- **Fixture ledger:** $200.01 of synthetic spend.
- **Request:** any.
- **Expected behaviour:** `LlmError::BudgetExceeded`
  returned with **zero** outbound HTTP requests; cockpit
  alert raised.

## Verification

The tester's contract for declaring this feature done. All
items must be green before VERDICT → PASS. Mapping to R-numbered
requirements is explicit.

- **V1 Static checks pass.** `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings`
  clean, `cargo audit` no unpatched advisories,
  `cargo deny check` passes. `crates/llm/` adds
  `#![deny(clippy::float_arithmetic)]` consistent with
  `crates/reflection/` at v1.8.
- **V2 `cargo test --workspace` green.** Zero failures, zero
  unexplained `#[ignore]`. Includes test surfaces for
  R1–R12.
- **V3 Smoke binary runs end-to-end.** `cargo run --bin
  llm-smoke` against `wiremock` (CI) prints a green table
  for all three providers and exits 0. Operator-invoked path
  against real APIs is **out of CI**; invoked manually
  before the v2.0.0 ship gate.
- **V4 No real-API calls during workspace tests.** CI
  network-policy assertion: zero outbound HTTPS to
  `*.anthropic.com / *.openai.com / *.openrouter.ai /
  *.deepseek.com` during `cargo test --workspace`.
- **V5 Cost telemetry round-trip.** Integration test fires
  one LLM call, asserts a balanced `expense:llm:<tier>` ↔
  `liabilities:llm_accrued` journal pair, asserts
  `audit::query::global_debit_credit_sum` returns
  `(total_dr, total_cr)` with `|dr - cr| ≤ 1e-8`.
- **V6 Budget gate determinism.** Two runs of the
  `llm-budget-degrade` scenario at the same `(ledger_state,
  request)` produce byte-identical degrade events
  (`correlation_id` excluded).
- **V7 Replay determinism.** Two runs of `ReplayProvider`
  against the same cached hash produce byte-identical
  responses (load-bearing for research-mode anchor
  stability).
- **V8 Anchor stability under Q1 = Option A.** 9 strategy-
  backtest anchor SHAs at lines 15–58 stay byte-identical
  (R14.2). 2 operator-success-report anchors at lines 67–75
  stay byte-identical OR re-lock once per Q11 (architect's
  call).
- **V9 No-secrets-in-logs.** Grep over `target/logs/*.log`
  after a smoke run finds zero occurrences of any test API
  key string. R8.3 redact helper is the only `Display` path
  for keys.
- **V10 Performance.** Each provider's `complete()` call
  against a wiremock fixture completes in `< 200ms` test
  wall-clock; the 3-provider smoke completes in `< 1s`
  total. Real-API latencies documented in smoke rustdoc, not
  gated.
- **V11 Replay cache compatibility.** The fixture cache at
  `crates/llm/tests/fixtures/llm-replay.db` is readable by
  the v2 `ReplayProvider`; schema-migration test asserts the
  cache schema is forward-compat for v3 additions.

Failure routing:

- Static / test failure → `developer`.
- Trait-shape change required (R1) → `architect`.
- Provider impl breakage (e.g. Anthropic deprecates
  `cache_control`) → `developer`.
- Budget-gate policy change (e.g. operator wants 90%
  threshold instead of 80%) → `analyst` (cascades into
  product.md edits, not in-place developer work).
- Replay determinism breaks → `architect` (request-hash
  schema evolves).
- Real-API smoke failure in operator's environment →
  `operator` resolves (likely API-key issue).
- Anchor change in strategy-backtest anchors (R14.2
  violated) → `analyst` (strategy-side leak; escalate to
  re-scope, do not silently re-lock).

## Notes / Open questions

The analyst defers these decisions to the architect (or to the
operator where flagged). The brief is written so each question
can be answered without reshaping the requirements above. Q-items
are tagged `[OPERATOR-DECIDE]` (operator must resolve before
architect can land) or `[ARCHITECT-DECIDE]` (architect lands in
the Design section after operator clears the operator-decides).
**Q1, Q2, Q3, Q10 block architect handoff**; the rest unblock
once Q1 lands.

### Q1 — Scope split: foundation-only vs foundation + N consumers [OPERATOR-DECIDE]

**The load-bearing decision of this brief.** Every downstream
Q-item is shaped by it.

product.md names ten LLM-driven roles
([Trading-time agent roster](../product.md#trading-time-agent-roster))
and one LLM-augmented strategy
([Strategy library — roadmap](../product.md#strategy-library--roadmap)
v2 row: "LLM-augmented news/sentiment overlay"). Three
consumers are pre-staged:

- `reflection-memory-llm-enrichment` — the `post_mortem_analyst`
  LLM-rewrites the lesson card's `note: Option<String>` field
  (currently `None` in v1.8 per
  [reflection-memory R1.1](../reflection-memory/feature.md#r1--lesson-card-data-model)).
- `news-sentiment-overlay` — the v2-strategy-roadmap entry;
  feeds `news_analyst` + `sentiment_analyst` LLM output into a
  strategy override.
- `trader-debate` — the bull/bear researcher debate + trader
  synthesis loop, the highest-token-budget consumer.

**Tradeoff:**

- **Option A (foundation-only):** v2.0.0 ships trait + 3
  providers + cache + budget + replay + smoke binary; no
  consumers. Each consumer = follow-up brief.
  - **Pro:** Smaller test surface per ship. Operator picks
    consumer order rather than locking it now. Risk
    isolation (foundation issues don't block consumers,
    consumer issues don't block the foundation). 9 strategy
    anchors don't move.
  - **Con:** v2.0.0 has **no operator-visible LLM behaviour**
    — `LLM spend: $0.00 / $200` until the first consumer
    ships. Smoke binary is the only "demo" artefact.
- **Option B (foundation + news/sentiment overlay):** Bundles
  the v2-strategy-roadmap consumer.
  - **Pro:** Visible behaviour at ship; v2 row in
    product.md ships in v2.0.0, not v2.1.0; cost report
    shows real spend.
  - **Con:** Brief size doubles; test surface = foundation +
    strategy impl + prompt design + tool-use schema; 10th
    anchor lands and may move existing anchors; consumer
    order locked.
- **Option C (foundation + post_mortem enrichment):** Bundles
  the v1.8-deferred consumer.
  - **Pro:** No strategy impact (post_mortem runs off the
    decision hot path); v1.8 carry-forward debt paid the day
    v2 ships.
  - **Con:** Brief still doubles; reflection-memory-llm-
    enrichment is well-spec'd in v1.8 Q1 deferral and
    bundling means re-doing that scoping inside v2.
- **Option D (foundation + multiple consumers):** Maximum
  scope; highest risk; highest payoff.
  - **Con:** A single consumer's prompt design could block
    the foundation from shipping at all. Brief size 4–5x.

[ANALYST-RECOMMENDATION]: **Option A (foundation-only).**
Reasons:
1. **Independent verification surfaces.** Foundation is
   verified by `cargo run --bin llm-smoke` round-tripping
   three providers; bundling adds prompt-quality and
   output-quality verification that's a different test kind.
2. **Smaller test budget per ship.** Foundation-only is
   already ~14 R + 11 V; bundling adds 4–8 R per consumer.
3. **Operator picks consumer order.** Multiple consumers
   queue up; the operator's priority among them is real
   product information that an analyst-locked ordering
   throws away.
4. **Risk isolation.** Foundation issues (Anthropic API
   change, prompt-cache field rename) don't block consumer
   dev; consumer issues (prompt iteration, tool-use schema
   drift) don't block the foundation.
5. **Anchor stability.** Foundation-only doesn't touch the
   9 strategy anchors. Bundling news/sentiment overlay
   re-locks at least one; trader debate re-locks all 9.

The operator may push back if the `LLM spend: $0.00 / $200`
post-v2 cost-report line feels demo-failure-y. Counter:
smoke-binary output in the v2 presenter deck is the demo,
not the cost report.

**Operator decision required before architect handoff.**

### Q2 — Default provider + tier model assignments [OPERATOR-DECIDE]

product.md line 250: "Anthropic (default, with prompt
caching)." Live alternatives:

- **Option A — Anthropic.** `deep_think`: Claude Opus 4.7
  (`claude-opus-4-7`). `quick_think`: Claude Haiku 4.5
  (`claude-haiku-4-5-20251001`). Pro: prompt caching is
  first-class on Anthropic — only provider where R3
  breakpoints translate to discount. Cheapest at scale.
  Con: single vendor (mitigated by OpenAI-compatible flip).
- **Option B — OpenAI.** `deep_think`: `gpt-5` (or current
  at v2 ship; operator confirms model id at handoff time).
  `quick_think`: `gpt-4-mini` (or current). Pro: broader
  ecosystem familiarity. Con: loses prompt caching's 75%
  discount.
- **Option C — Local-first (Ollama for quick_think,
  Anthropic for deep_think only).** Pro: $0 marginal cost
  on the high-volume tier. Con: loses tool-use precision on
  quick_think (R5.4); 70B Ollama model needs ~40GB VRAM —
  the project's "single VM" host probably has no GPU
  ([product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)).

[ANALYST-RECOMMENDATION]: **Option A (Anthropic, both
tiers).** Prompt caching is the cheapest path at v2 scale;
single-vendor risk is mitigated by the OpenAI-compatible
provider being a one-config-flip away.

Knowledge cutoff: this brief assumes `claude-opus-4-7` and
`claude-haiku-4-5-20251001` are current at v2 ship time.
Operator confirms at architect handoff; if the model family
has rev'd, architect updates pricing (R9.2) and config
defaults (R12.1).

### Q3 — API key management at v2.0.0 [OPERATOR-DECIDE]

R8.1 picks env-var-only. Alternatives:

- **Option A (env-vars only — analyst pick):** Simplest.
  Operator sets `ANTHROPIC_API_KEY` in `.envrc` or systemd
  unit file. No new dep.
- **Option B (platform secret store):** macOS Keychain on
  dev machines; AWS Secrets Manager / HashiCorp Vault on
  prod. New dep; better rotation story.
- **Option C (config-file with explicit acknowledgement):**
  Keys in `config/agent.toml.local` (git-ignored). Lower
  setup friction; higher accidental-commit risk.

[ANALYST-RECOMMENDATION]: **Option A (env-vars).** Defers
Option B to a v3 follow-up brief (`llm-secret-store`) once
the deployment surface stabilises (currently single VM;
secret store becomes meaningful at multi-host).

**Operator decision required before architect handoff.**

### Q4 — Trait shape: async, streaming, batch, tool-use, errors [ARCHITECT-DECIDE]

R1.1 strawman:
```rust
async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;
```

Open dimensions:

- **Q4a Sync vs async.** Strawman: async (every consumer in
  a tokio task).
- **Q4b Streaming vs non-streaming.** Strawman: non-
  streaming at v2.0.0 (streaming = v3). Architect may push
  back if a v2 consumer (e.g. trader debate UI) needs
  token-by-token output. Counter: at v2 there's no cockpit
  chat surface (Lumen Phase 6 ships separately).
- **Q4c Tool-use from day one.** Strawman: yes, per
  [product.md line 257](../product.md#llm-strategy).
- **Q4d Batch API.** Strawman: deferred. Most v2 consumers
  are single-prompt; batch is a sentiment-analyst-at-scale
  concern.
- **Q4e `serde_json::Value` vs typed `ToolSchema`.**
  Strawman: `serde_json::Value` + `schemars` producer +
  `jsonschema` validator (architect picks the validator).
- **Q4f `LlmError` variant set.** Strawman: Provider,
  RateLimited, Timeout, BudgetExceeded, InvalidResponse,
  ReplayMiss, Network, Auth.

[ANALYST-RECOMMENDATION]: ship the strawmen unless specific
reasons emerge. Architect's call.

### Q5 — Prompt-cache strategy: TTL-driven vs explicit + breakpoint placement [ARCHITECT-DECIDE]

Anthropic's prompt cache: 5-minute TTL, up to 4 breakpoints
per request, ~75% discount on cached input tokens.

- **Q5a TTL-driven vs explicit invalidation.** Strawman:
  TTL-driven at v2.0.0; explicit invalidation lands when a
  prompt-library / version-bump pipeline exists (post-v2).
- **Q5b Breakpoint count.** Strawman: 2 (project + role).
  Architect may use 3 or 4.
- **Q5c Builder location.** Strawman: sibling builder
  `CachedSystemPrompt`; the trait sees a flat
  `Vec<SystemBlock>`. Anthropic-specific cache behaviour
  lives in the Anthropic provider impl.
- **Q5d Cache-hit-rate metric.** Strawman: `tracing` event
  per role per day; surfaced in System Health line.
  Architect may pick Prometheus histogram or moving average.
- **Q5e Cache invalidation on prompt-text edits.** Strawman:
  rely on natural cache misses (edited prompt → different
  cache key → new entry).

[ANALYST-RECOMMENDATION]: ship the strawmen.

### Q6 — Budget gate placement: pre-call vs post-call vs decorator [ARCHITECT-DECIDE]

R4 strawman: pre-call estimate + post-call reconciliation +
factory-level `BudgetedProvider` decorator.

- **Q6a Where the gate fires.** Three options: factory-
  level decorator (strawman; hard to forget), in-impl
  (more code), explicit consumer-side helper (easy to
  forget). Strawman is the decorator because forgetting is
  a $200 foot-gun.
- **Q6b Pre-call estimate accuracy.** Strawman: use
  `max_tokens` as the estimate input (conservative;
  fail-closed). Alternative: `max_tokens / 2` (heuristic;
  risks $0.01 overshoots).
- **Q6c Concurrent-call budget race.** Two LLM calls fired
  in parallel may both pass the pre-call check with $1 of
  headroom and both succeed at $1.50 each, blowing the
  budget by $1. Strawman: `AtomicU64`-backed
  `spent_usd_cents` so pre-call checks are atomic-add-and-
  compare. Architect may pick a mutex or per-tier
  semaphore.

[ANALYST-RECOMMENDATION]: ship the strawmen. Q6c is the
most load-bearing — if missed, the $200 ceiling becomes a
soft cap.

### Q7 — Cost-rate provider lookup: hard-coded vs TOML vs API metadata [ARCHITECT-DECIDE]

Pricing varies per provider per model and changes
~quarterly.

- **Option A (hard-coded `match`):** Compile-time check
  every (provider, model) the agent uses has a price.
  Reviewed quarterly during costs.md run.
- **Option B (TOML-driven):** Operator updates without
  recompiling. Loses compile-time check (typo in model id
  silently matches no entry → USD = $0).
- **Option C (API metadata):** Some providers expose
  pricing in their model-list endpoint; Anthropic doesn't.
  Mixed-source data.

[ANALYST-RECOMMENDATION]: **hybrid — hard-coded base table
+ TOML override for ephemeral changes** (R9.2 strawman).
Compile-time check on the base; operator can patch a price
without recompiling for emergencies. Architect's call on
module location (analyst's prior:
`crates/llm/src/pricing.rs` since `(provider, model)` is
LLM-domain-specific; `cost` crate is the recording
substrate, not the rate source).

### Q8 — Deterministic replay scope and storage [ARCHITECT-DECIDE]

R6 strawman: SQLite at `data/llm-replay.db`; fixture cache
at `crates/llm/tests/fixtures/llm-replay.db` versioned in
git.

- **Q8a Hash function.** Strawman: SHA-256 over
  canonicalised JSON of `(model, system, messages, tools,
  max_tokens, temperature)`. `correlation_id` excluded so
  the same prompt from different consumer runs hits the
  same cache. Temperature included (changes output
  distribution).
- **Q8b Schema migration.** Strawman: `schema_version`
  column; ReplayProvider asserts version ≤ supported.
- **Q8c Cache size cap.** Strawman: no cap; operator manages
  via `rm`. Architect may pick LRU.
- **Q8d Fixture content.** Strawman: one canned response
  per provider (3 rows). Architect may pick richer (one per
  agent role).
- **Q8e Concurrent-write safety.** Strawman: SQLite WAL
  mode handles it; writer is single-task per process.

[ANALYST-RECOMMENDATION]: ship the strawmen. Q8a (hash
schema) is the most load-bearing — a change invalidates
every cached response, with migration cost on every v3+
brief.

### Q9 — Rate-limit handling: backoff + jitter vs circuit breaker [ARCHITECT-DECIDE]

R7 strawman: exponential backoff with full jitter, max 3
retries, then `LlmError::RateLimited`. No circuit breaker.

- **Q9a Retry budget.** Strawman: 3 (Anthropic's 1-min
  rate-limit window fits inside backoff with cap=8s + 3
  retries).
- **Q9b Circuit breaker.** Strawman: no, defer to v3.
  Counter: a degraded provider could pile up retry latency
  for hours in a long-lived process before the operator
  notices. Strawman defers because we don't have provider-
  failure-rate observability yet.
- **Q9c Jitter formula.** Strawman: full jitter (AWS-
  recommended). Alternative: equal jitter.

[ANALYST-RECOMMENDATION]: ship the strawmen. Architect may
push back on Q9b if there's a strong long-running-process
argument.

### Q10 — Cost-control surfacing: where the operator sees budget events [OPERATOR-DECIDE — informational]

R11 strawman: cockpit "LLM budget" tile + audit memo +
operator-success-report System Health line.

Live alternatives:
- **More:** email (SMTP dep), Slack webhook (webhook dep),
  push (APNs/FCM dep).
- **Less:** drop the cockpit tile (memo + report line only).

[ANALYST-RECOMMENDATION]: ship the strawman. Email/Slack/
push are v3 follow-up briefs once the operator has lived
with v2 and named what's missing. **Operator decision
(informational): is the strawman enough?** Doesn't block
architect handoff — architect lands the strawman; if the
operator wants more after seeing the Design, a follow-up
brief absorbs it.

### Q11 — Operator-success-report `LLM spend` denominator update [ARCHITECT-DECIDE]

[operator-success-reports R8.4](../operator-success-reports/feature.md#r8--no-regression-in-non-report-code-paths)
specifies the System Health line as `LLM spend: $X.XX /
$135`. v2 bumps the ceiling to $200
([product.md line 339](../product.md#cost-economics--monthly-ceiling)).

- **Option A** Architect updates the report-render string in
  this brief; re-locks the 2 operator-success-report
  anchors.
- **Option B** Defer to first LLM-consumer brief; v2.0.0
  ships with the report still showing `$135`.
- **Option C** 1-line denominator hot-fix in this brief
  (`$135 → $200`); numerator stays $0 under foundation-only
  scope; anchors re-lock once. Then re-lock again when first
  consumer ships and numerator changes.

[ANALYST-RECOMMENDATION]: **Option C** — 1-line change makes
the report immediately reflect the v2 ladder; double re-lock
is OK because body changes are real both times.

### Q12 — Consumer brief ordering [ANALYST-INTERNAL — for operator awareness]

Under Q1 = Option A, several consumer briefs queue up. The
analyst's suggested ordering, for operator visibility:

1. **`reflection-memory-llm-enrichment`** — pre-staged in
   v1.8 Q1 deferral. Lowest risk (post_mortem off the
   decision hot path); pays off the v1.8 carry-forward
   debt; exercises every foundation R-item against a real
   consumer.
2. **`news-sentiment-overlay`** — the v2 strategy roadmap
   entry. First LLM-driven strategy; re-locks ≥1 strategy
   anchor.
3. **`trader-debate`** — bull/bear debate + trader
   synthesis; highest token budget; re-locks all 9 strategy
   anchors. Belongs later to amortise the v2 ceiling over
   fewer high-cost consumers.
4. **`reflection-memory-trader-wiring`** — top-K retrieval
   into the trader's decision; depends on trader-debate
   landing first (the trader is the consumer of retrieved
   cards in product.md layer 3).

This is a prior, not a commitment. Operator decides at each
follow-up brief's spawn time. Captured here for visibility.

## Changelog

- 2026-05-10 (analyst): initial brief — foundation-only
  scope-split recommendation in Q1, twelve open questions
  surfaced for architect/operator resolution, R1–R14 derived
  from product.md LLM strategy + cost economics + operating
  modes sections, V1–V11 verification contract, six smoke +
  replay + budget backtest scenarios. Q1, Q2, Q3, Q10 block
  architect handoff. HANDOFF → architect pending operator
  resolution of Q1, Q2, Q3, Q10.
