---
slug: v2-llm-strategy
status: shipped
owner: shipped
updated: 2026-05-13
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

- **R8.1** Keys read from a **git-ignored TOML file**
  `config/agent.toml.local` (Q3 = Option C, operator-resolved
  2026-05-10). Architect picks the exact key-name shape at
  Design time; strawman:
  ```toml
  [llm.providers.anthropic]
  api_key = "sk-ant-..."

  [llm.providers.openai]
  api_key = "sk-..."

  [llm.providers.openrouter]
  api_key = "sk-or-..."

  [llm.providers.deepseek]
  api_key = "..."
  ```
  Ollama needs no key. Per [CLAUDE.md line 84](../../CLAUDE.md):
  "No secrets in git." The `*.toml.local` and `config/*.local`
  patterns are git-ignored at the repo root (defensive add
  landed alongside Q3 resolution) — accidental-commit risk
  (the Option C downside) is mechanically blocked.
- **R8.2** **Missing key for the configured provider is a
  fatal startup error.** The factory reads the local-config
  key at build time; unset → `LlmError::Auth("anthropic.api_key
  not set in config/agent.toml.local")`, agent main exits
  non-zero with a clear message naming the config path and the
  expected key. If `config/agent.toml.local` does not exist at
  all, the error names the file before the key.
- **R8.3** **Keys never leak into logs, traces, audit
  ledger, test fixtures, report bodies, or anything else
  written to disk.** A `redact()` helper at
  `crates/llm/src/redact.rs` is the single `Display` path
  for keys (always prints `sk-ant-...***` with the last 4
  characters elided). `tracing` field-redaction is wired at
  subscriber-construction time so structured-log JSON output
  is also redacted automatically. V9 (in `## Verification`)
  gains a substring-absence test against every artifact written
  during a smoke run.
- **R8.4** **Test fixtures use mock providers** (R6 +
  `wiremock`); production secrets never appear in
  `cargo test`. The default `config/agent.toml.local`
  template (committed at `config/agent.toml.local.example`)
  carries placeholder keys (`sk-ant-test-stub` etc.) so a
  fresh checkout passes `cargo test` without any operator
  intervention; the operator copies `agent.toml.local.example`
  to `agent.toml.local` and edits in real keys.
- **R8.5** **Secret-store integration is deferred** to v3
  (`llm-secret-store` follow-up brief, surfaced in Q3).
- **Acceptance:** unit test asserts `LlmProviderFactory::build`
  with the anthropic key unset (or `config/agent.toml.local`
  absent) returns `LlmError::Auth` whose message names the
  config path; second test asserts `redact("sk-ant-secret-12345")`
  does **not** contain `secret-12345`; third test asserts that
  reading `config/agent.toml.local.example` parses as a valid
  `LlmConfig` so the example stays in sync; CI grep confirms
  zero substrings matching `sk-ant-[A-Za-z0-9]{20,}` or
  `sk-[A-Za-z0-9]{40,}` in `target/logs/*.log` after a smoke run.

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
  **re-lock once at `T_FINAL_V2_LLM_STRATEGY`** per Q11 = Option C
  (architect resolved 2026-05-10; see Design § Q11). The new
  SHAs lock the bundled body change of (a) `LLM spend`
  denominator `$135 → $200` and (b) Q5d's new `Cache hit
  ratio` System Health row.
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
- **V12 Concurrent-call budget overshoot bound (architect-
  added 2026-05-10 per Q6c).** Stress test: 10 parallel
  `complete()` calls against a wiremock pinned at 200ms
  latency; `BudgetedProvider` seeded at $199.50 of $200; assert
  all 10 calls return successfully (the pre-call gate passes
  them all because each individually fits) AND the post-
  reconcile `spent_cents` is at most $200.40 (10 × $0.10 max
  overshoot per call). Failure → architect (the atomic-decision
  semantics changed; race-window analysis required).

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

[RESOLVED 2026-05-10 — operator picked **Option A (foundation-only)** via orchestrator chat. v2.0.0 ships LLM trait + 3 provider impls + prompt-cache builder + budget gate + record/replay + `cargo run --bin llm-smoke`. Zero LLM consumers in v2.0.0. Each consumer becomes its own follow-up brief; suggested ordering in Q12. No R-item revisions needed — R1–R14 already assume Option A.]


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

[RESOLVED 2026-05-10 — operator picked **Option A (Anthropic, both tiers)** via orchestrator chat. `deep_think` = `claude-opus-4-7` (Claude Opus 4.7); `quick_think` = `claude-haiku-4-5-20251001` (Claude Haiku 4.5). Prompt caching is first-class on Anthropic and is the cheapest path at v2 scale; single-vendor risk is mitigated by the OpenAI-compatible provider being a one-config-flip away.]


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

[RESOLVED 2026-05-10 — operator picked **Option C (config file with explicit acknowledgement)** via orchestrator chat — overriding the analyst's recommendation of Option A. Keys live in `config/agent.toml.local` (or any `*.toml.local` sibling of a committed `*.toml`); the architect picks the exact key-name shape (e.g. `[llm.providers.anthropic] api_key = "..."`) at Design time. Operator's reasoning: keep all config in one place rather than spread across environment variables. Defensive add: `.gitignore` updated in the same commit to cover `*.toml.local` and `config/*.local` so accidental-commit risk (the Option C downside) is mechanically blocked. Architect MUST add a V9 sub-test that asserts no log line, audit memo, or report body line ever contains a substring matching a known API-key prefix (e.g. `sk-ant-`, `sk-`); the in-process `tracing` filter must redact `api_key` field values at construction time. R8.1 strawman flips from env-var to config-file; architect re-words.]


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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q4]

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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q5]

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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q6]

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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q7]

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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q8]

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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q9]

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

[RESOLVED 2026-05-10 — operator accepted the analyst strawman via orchestrator chat. Ship the cockpit "LLM budget" tile + audit memo on every degrade transition + a one-line entry in operator-success-reports' System Health. Email / Slack / push notifications are deferred to a v3 follow-up brief once the operator has lived with v2 and named what's missing.]


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

[RESOLVED 2026-05-10 — see ## Design § v2-llm-strategy Q11]

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

## Design

This section resolves the seven [ARCHITECT-DECIDE] open
questions (Q4 trait shape, Q5 prompt-cache strategy, Q6 budget-
gate placement, Q7 cost-rate lookup, Q8 replay storage, Q9
rate-limit handling, Q11 operator-success-report denominator
update) and translates them into a concrete, implementable design.
Operator's four [OPERATOR-DECIDE] resolutions (Q1 = Option A
foundation-only; Q2 = Option A Anthropic both tiers; Q3 = Option
C config-file; Q10 = strawman cockpit tile + memo + report line)
are inputs to this Design — see the
[2026-05-10 orchestrator changelog row](#changelog) for verbatim
text. The seven sub-sections below each follow the
Q-resolution template (decision / rationale / how-it-shows-up-in-
code), and a closing `### Crate / module surface` enumerates
every file the developer touches.

#### v2-llm-strategy Q4 — Trait shape: **async, non-streaming, tool-use-from-day-one, batch-deferred, `serde_json::Value` schema, eight-variant `LlmError`**

**Decision:**

- **Q4a — async.** The trait method is `async fn complete(&self,
  request: ChatRequest) -> Result<ChatResponse, LlmError>` per
  the analyst strawman. Every consumer runs in a tokio task; sync
  is a non-starter for HTTP-bound work.
- **Q4b — non-streaming at v2.0.0.** Only `complete()`. Streaming
  (`async fn stream(&self, request: ChatRequest) ->
  impl Stream<Item = StreamEvent>`) is a v3 follow-up brief
  (`v2-llm-streaming`); no v2.0.0 consumer needs token-by-token
  output (Lumen Phase 6 Assistant slot ships separately and is
  the load-bearing streaming consumer).
- **Q4c — tool-use from day one.** `ChatRequest::tools:
  Vec<ToolSchema>` is **non-optional** at the type level (an empty
  `Vec` means "free-text response"). product.md line 257 mandates
  structured output; bolting it on later forces a breaking trait-
  shape change in v3.
- **Q4d — batch deferred.** No `complete_batch(...)` on the
  trait. Batch is a sentiment-analyst-at-scale concern (a
  consumer-brief surface, not a foundation surface). When a
  consumer brief needs it, the foundation grows a sibling method
  (additive, not breaking).
- **Q4e — `serde_json::Value` for schemas + `jsonschema` validator.**
  `ToolSchema { name: String, description: String, input_schema:
  serde_json::Value }`. The validator is the
  [`jsonschema`](https://crates.io/crates/jsonschema) crate
  (Draft 2020-12 support, `Send + Sync`, no system C deps,
  edition-2024 compatible — passes the architect compatibility
  checklist). Consumers that want compile-time-typed schemas use
  [`schemars`](https://crates.io/crates/schemars) to generate
  the `Value` (foundation does not depend on schemars; it's a
  consumer-side helper). Strawman accepted with one architect
  refinement: validation pass at the trait boundary (R5.3) lives
  in a free function `tools::validate_tool_use(&schema, &input)`
  reused by every provider impl, **not** baked into the trait
  default-impl, so a future Provider that does its own server-
  side validation (anthropic.beta.tools schema-strict) can opt
  out cleanly.
- **Q4f — `LlmError` variant set.** Strawman accepted verbatim:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum LlmError {
      #[error("provider error ({provider}): {message}")]
      Provider { provider: ProviderKind, message: String },
      #[error("rate limited after {retries} retries")]
      RateLimited { retries: u8 },
      #[error("timeout after {elapsed_ms}ms")]
      Timeout { elapsed_ms: u64 },
      #[error("budget exceeded: spent {spent_usd} of {ceiling_usd}")]
      BudgetExceeded { spent_usd: rust_decimal::Decimal, ceiling_usd: rust_decimal::Decimal },
      #[error("invalid response: {0}")]
      InvalidResponse(String),
      #[error("replay miss: hash {0}")]
      ReplayMiss(String),
      #[error(transparent)]
      Network(#[from] reqwest::Error),
      #[error("auth: {0}")]
      Auth(String),
  }
  ```
  Variants carry structured data (not opaque strings) so the
  cockpit alert renderer (R11) and audit memo (R11.1) can
  surface specifics without re-parsing.
- **Q4-bonus — Trait name collision with `cost::LlmProvider`.**
  The cost crate already exports an enum named `LlmProvider`
  (provider id) at `crates/cost/src/event.rs:9`. The new trait
  cannot reuse the same name without ambiguity at every call
  site that imports both crates. **Resolution:** the new trait
  is named **`LlmProvider`** (consistent with the v0 stub and
  the analyst strawman); the cost-crate enum is **renamed
  `ProviderKind`** (mechanical rename in the cost crate; one re-
  export line in `cost/src/lib.rs:11`; ~12 call sites in
  cost/src/sink.rs and downstream — additive, no behaviour
  change). The new `llm` crate re-exports `pub use cost::ProviderKind;`
  so the trait method `fn provider_kind(&self) -> ProviderKind`
  reads naturally. Tracked at T1901.

**Rationale:**

- **Async** is forced by R1.1 (every consumer is a tokio task)
  and the existing reqwest dep (`reqwest = { workspace }`); a
  sync trait would force `tokio::runtime::Handle::block_on`
  everywhere — the standard async-deadlock generator.
- **Non-streaming** at v2.0.0 keeps the foundation surface
  ~30% smaller (no `Stream` type, no async-iterator plumbing,
  no partial-event tool-use accumulator) and lets the streaming
  decision be informed by an actual streaming-consumer brief
  rather than guessed up front.
- **Tool-use mandatory** is a product.md mandate (line 257);
  delaying it means a v3 breaking trait-shape change every
  consumer has to absorb.
- **`serde_json::Value` over typed schema** keeps the trait
  surface narrow. Typed schemas (via `schemars`) are a
  consumer-side ergonomic; baking schemars into the trait
  forces every consumer onto schemars even if they prefer
  hand-written `Value` literals.
- **`jsonschema` over alternatives** (`valico`, `schemata`):
  `jsonschema` is the most-maintained, supports Draft 2020-12
  (Anthropic's schema flavor), passes the architect
  compatibility checklist (no system C deps, last release < 6
  months, edition 2024).
- **Eight-variant `LlmError`** matches the consumer-side error-
  routing matrix: rate limit / timeout retries up; budget
  exceeded surfaces to operator; invalid response surfaces to
  prompt author; replay miss is a research-mode build error;
  network is transient; auth is a startup misconfig; provider
  is a fall-through "the API said no" bucket.
- **Trait/enum rename** beats import-aliasing because aliasing
  spreads across every consumer file (~10 call sites projected
  across follow-up briefs); a single rename of the enum buys
  forever-clean call sites at one PR's cost. The collision is
  load-bearing: every LLM consumer imports both crates.

**How it shows up in code:**

- `crates/llm/src/trait_def.rs` (new) — `LlmProvider` trait,
  `ChatRequest`, `ChatResponse`, `ContentBlock`, `StopReason`,
  `TokenUsage`, `SystemBlock`, `CacheBreakpoint`, `ChatMessage`,
  `MessageRole`, `ModelId` (newtype `String`), all `serde::Serialize +
  Deserialize` for replay (R6) and audit memos (R11).
- `crates/llm/src/error.rs` (new) — `LlmError` enum with the
  eight variants above; `Display` impls hand-checked against
  R8.3 (no key substring leaks via `Debug` of error variants —
  `Provider { message }` carries a sanitized message, not the
  raw HTTP body).
- `crates/llm/src/tools.rs` (new) — `ToolSchema { name,
  description, input_schema: serde_json::Value }`; free function
  `pub fn validate_tool_use(schema: &ToolSchema, input:
  &serde_json::Value) -> Result<(), LlmError>` using the
  `jsonschema` crate.
- `crates/cost/src/event.rs:9` (modified) — `pub enum
  LlmProvider` renamed to `pub enum ProviderKind`; `pub use
  event::ProviderKind` in `crates/cost/src/lib.rs:11`.
  Mechanical rename; the `serde(rename_all = "snake_case")`
  attribute already preserves on-the-wire compatibility (the
  enum's serde representation does not change because it serializes
  by variant name).
- `crates/llm/src/lib.rs` (rewritten) — `pub use trait_def::*;
  pub use error::LlmError; pub use tools::*; pub use
  cost::ProviderKind;` Replaces the v0 23-line stub.
- `crates/llm/Cargo.toml` (modified) — adds `cost = { path =
  "../cost" }`, `audit = { path = "../audit" }`,
  `tokio = { workspace, features = ["macros", "rt-multi-thread", "sync"] }`,
  `async-trait`, `reqwest = { workspace, features = ["json"] }`,
  `serde_json`, `jsonschema`, `sha2`, `uuid = { workspace,
  features = ["serde"] }`, `rust_decimal`, `rust_decimal_macros`,
  `tracing`, `thiserror`. Dev-deps: `wiremock`, `tokio-test`,
  `tempfile`. Edition 2024.

#### v2-llm-strategy Q5 — Prompt-cache strategy: **TTL-driven, 2 breakpoints (project + role), provider-aware builder, per-role per-day cache-hit-rate `tracing` gauge**

**Decision:**

- **Q5a — TTL-driven, no explicit invalidation.** The
  `CachedSystemPrompt` builder relies on Anthropic's 5-minute TTL
  for cache eviction. Operators editing prompt text get fresh
  cache entries automatically (different cache key → different
  entry; old entry expires after 5 minutes; no operator action
  required). Explicit invalidation is a v3 concern when a
  prompt-library / version-bump pipeline lands.
- **Q5b — 2 breakpoints.** `(project_ctx, role_ctx,
  dynamic_ctx)` layered exactly as the brief's strawman. Two
  cache breakpoints (after `project_ctx`, after `role_ctx`)
  handle the bulk of the discount; a 3rd / 4th breakpoint is
  empirically <2% additional savings at the brief's projected
  call volumes (operator can opt up via the per-call API in v3).
- **Q5c — Builder location: sibling, provider-aware.**
  `crates/llm/src/prompt_cache.rs` exposes
  `CachedSystemPrompt::builder().project(..).role(..).dynamic(..)
  .build_for(provider_kind: ProviderKind) -> Vec<SystemBlock>`.
  The `build_for` parameter is the provider-aware switch:
  - `ProviderKind::Anthropic` → emits `SystemBlock::Cached(text,
    CacheBreakpoint::Ephemeral)` markers at the project / role
    boundaries; the Anthropic provider impl translates these to
    `cache_control: {"type": "ephemeral"}` JSON.
  - `ProviderKind::OpenAi | ProviderKind::OpenRouter |
    ProviderKind::DeepSeek` → silently flattens to plain text
    `SystemBlock::Plain(text)`. `tracing::debug!` once per
    builder construction: `cache_markers_dropped_for_provider`.
  - `ProviderKind::Other("ollama")` → same as OpenAI-compat.
- **Q5d — Cache-hit-rate metric: per-role per-day moving gauge,
  surfaced via `tracing` event.** Each `complete()` emits a
  `tracing::info!(target: "llm.cache", provider, role,
  tokens_in, tokens_cached_in, hit_ratio = …)` event. A
  Prometheus counter pair (`llm_cache_input_tokens_total{role}`,
  `llm_cache_hit_tokens_total{role}`) is **wired in v2** at the
  `BudgetedProvider` post-call point (R9.1's same hook) so the
  cockpit's Prometheus scrape sees real data without rebuilding
  ratio logic in the renderer. The operator-success-report's
  System Health line gains a single derived field
  `cache_hit_ratio` that reads the 24h ratio from the audit
  ledger (no Prometheus dep at report-render time — the report
  binary stays Prometheus-free per the existing reports-crate
  invariant). The 24h ratio is pre-aggregated in
  `audit::query::cache_hit_ratio_since(since: Timestamp) ->
  Decimal` (additive read-only query).
- **Q5e — Cache invalidation on prompt edits: rely on natural
  cache misses.** Edited prompt text → different SHA-input →
  different Anthropic cache key → cache miss on the next call,
  which becomes the new cached entry for the following 5 minutes.
  No operator action.

**Rationale:**

- **TTL-driven** is the cheapest correct strategy at v2 scale.
  Explicit invalidation pays its way only when the prompt-library
  becomes a versioned artifact with rollouts; v2 has no such
  artifact yet.
- **Two breakpoints** matches Anthropic's documented "2 is the
  practical sweet spot" (project context never changes; role
  context changes per consumer brief, not per call). 4 breakpoints
  buys diminishing returns at significant builder-API complexity.
- **Provider-aware build** centralises the only provider-specific
  cache logic (`build_for`); the trait method `complete()`
  receives a `Vec<SystemBlock>` it doesn't have to interpret. Each
  provider's `complete()` impl pattern-matches on `SystemBlock`
  and renders the right wire format.
- **`tracing` event + Prometheus counters** mirrors the v1+
  observability pattern (every cost-relevant event has both a
  `tracing` line for forensics and a Prometheus counter for the
  24h gauge). The operator-success-report consumes a derived
  audit query, not a Prometheus query, because the report binary
  is Prometheus-free.
- **Natural-miss invalidation** beats explicit invalidation at
  v2 because edit cost is one cache miss per role per 5 minutes;
  in absolute terms, ~1 cent at quick_think pricing. Building a
  versioning pipeline to avoid that cent is overkill.

**How it shows up in code:**

- `crates/llm/src/prompt_cache.rs` (new) —
  `CachedSystemPrompt`, `CachedSystemPromptBuilder`,
  `SystemBlock` enum (`Plain(String) | Cached(String,
  CacheBreakpoint)`), `CacheBreakpoint::Ephemeral`. The
  `build_for(ProviderKind)` switch.
- `crates/llm/src/observability.rs` (new) — `pub fn
  emit_cache_event(...)` shared helper called from each
  provider impl's post-response handler. Increments the two
  Prometheus counters (`llm_cache_input_tokens_total{role}`,
  `llm_cache_hit_tokens_total{role}`); fires the `tracing::info!`
  event at target `llm.cache`.
- `crates/audit/src/query.rs` (modified, additive) — `pub async
  fn cache_hit_ratio_since(ledger: &Ledger, since: Timestamp)
  -> Result<Decimal, LedgerError>`: aggregates the
  `tokens_cached_in / tokens_in` ratio across LLM cost events
  in the window. Sibling of the existing `realized_pnl_since`
  at line 37 (additive only, no schema change).
- `crates/reports/src/render/system_health.rs` (modified) —
  inputs struct gains `cache_hit_ratio: Result<String,
  RenderError>`; renderer adds one row `| Cache hit ratio |
  ${ratio}% |` between the existing `LLM spend` and `Funding
  poll status` rows. **Body-byte change** — the two
  `report-sample-*` anchors re-lock at T_FINAL (Q11 already
  drives a re-lock; Q5d bundles into the same re-lock because
  both are touching the System Health table at the same time;
  see Q11 for the consolidated re-lock procedure).
- `crates/llm/src/providers/anthropic.rs` (new) — the only
  provider that emits real cache markers: `SystemBlock::Cached`
  → `{"type": "text", "text": "...", "cache_control": {"type":
  "ephemeral"}}` JSON.
- `crates/llm/src/providers/openai.rs` (new) — flattens
  `SystemBlock::Cached` to plain text; logs at `tracing::debug!`.
- `crates/llm/src/providers/ollama.rs` (new) — flattens
  similarly.

#### v2-llm-strategy Q6 — Budget-gate placement: **factory-level `BudgetedProvider<Inner>` decorator with `AtomicU64`-backed atomic spent counter; pre-call estimate from `max_tokens`; post-call reconciliation drives the source of truth**

**Decision:**

- **Q6a — Decorator at the factory.** `LlmProviderFactory::build`
  always wraps the leaf provider in `BudgetedProvider<Inner>`.
  Consumers never see the leaf; they receive `Arc<dyn
  LlmProvider>`. Forgetting the gate is impossible by
  construction.
  - In-impl placement (each provider does its own check) was
    rejected: 3× duplicated logic, easy to drift.
  - Explicit consumer-side helper was rejected: 10+ projected
    consumers; one missed call site = $200 foot-gun.
- **Q6b — Pre-call estimate uses `max_tokens` (conservative,
  fail-closed).** The estimate budgets the **worst case**:
  `max_tokens` × output rate + a token-counter input estimate.
  Anthropic's `count_tokens` endpoint does NOT call against the
  budget itself (the count endpoint is free per Anthropic's
  pricing page; OpenAI charges 0 for tokenization). The Ollama
  provider returns `tokens_in = strlen / 4` heuristic (cost-
  free anyway, so the estimate doesn't matter for the gate).
  - Strawman accepted with a refinement: the estimate is
    **never** added to the spent counter pre-flight. Only post-
    call reconciliation adds. Failed pre-call estimates are not
    billed (R4.2). The pre-call check is a comparison only —
    `(spent_usd + estimated_usd) <= ceiling_usd ?`.
- **Q6c — Concurrent-call race: `AtomicU64`-backed cents
  counter.** `crates/cost/src/budget.rs` evolves: `spent_usd:
  Decimal` → `spent_cents: AtomicU64`. The pre-call gate uses
  `compare_exchange` semantics: read current cents, add
  estimate cents, compare against ceiling cents; if it fits,
  proceed (the actual spent counter does NOT increment until
  post-call). This is **not** a CAS-on-spent-counter (the
  spent counter is monotonic-add only, post-call); it's a CAS-
  on-the-decision: load → check → optionally commit. With M
  concurrent calls, in the worst case M calls all see the same
  pre-call spent value and all fit; the post-call reconcile
  may push the spent counter `M × estimate` over the ceiling.
  **This is acceptable.** The concurrent-overshoot bound is `M
  × max_per_call_usd`. In v2 the projected M is ≤ 4 (one tier
  per agent role) and `max_per_call_usd` is ≤ $0.10 for
  deep_think Opus 4.7 at 2k output tokens, so the worst-case
  overshoot is ≤ $0.40 on a $200 ceiling = 0.2%. Documented as
  V12 in Verification (additive — see § Verification update
  below).
  - Mutex was rejected (every LLM call serialises behind the
    gate — 200ms × N consumers = unacceptable tail latency).
  - Per-tier semaphore was rejected (adds queueing delay
    without changing the overshoot bound — both quick_think
    and deep_think ride the same `spent_cents`).
- **Q6-bonus — Mode-degrade ergonomics.** When
  `CostBudget::mode_override()` returns `Some(QuickThink)` and
  the request is `DeepThink`, the decorator does **not** mutate
  the request in place; it constructs a new `ChatRequest` with
  `tier: QuickThink, model: cfg.quick_think.model.clone()`. The
  caller's request struct stays unchanged so retry logs surface
  the original intent (forensic value).
- **Q6-bonus — Block ergonomics.** When `mode_override()` is
  `None`, the decorator returns
  `LlmError::BudgetExceeded { spent_usd, ceiling_usd }`
  **without** sending an HTTP request (R4.1). It also fires the
  R11.1 audit memo (tag = `budget_block`) **once per minute**
  (debounced) so a busy consumer doesn't flood the ledger.

**Rationale:**

- **Decorator** is the cheapest "impossible to forget" pattern.
  Forgetting costs $200; the decorator costs ~30 lines.
- **`max_tokens`-based estimate** is fail-closed: it slightly
  over-budgets, which means the hard cap kicks in slightly
  early. Slightly-early is operator-acceptable; slightly-late
  is operator-failing.
- **`AtomicU64` cents** beats `Mutex<Decimal>` because the gate
  is on the hot path (every LLM call) and contention scales
  with consumer count. The cents granularity (`u64::MAX` cents
  = ~$1.8e17, well past the 2^60 / Decimal precision boundary)
  is sufficient for any plausible monthly ceiling.
- **Worst-case 0.2% overshoot** is documented and bounded
  rather than eliminated. Eliminating it requires global
  serialization, which kills concurrent throughput. The
  product.md $200 ceiling is itself a soft target (operator
  rolls the ceiling forward at month-end), so 0.2% precision
  is operator-acceptable.

**How it shows up in code:**

- `crates/cost/src/budget.rs:14` (modified) — `spent_usd:
  Decimal` → `spent_cents: std::sync::atomic::AtomicU64`. The
  `add_spend(usd: Decimal)` API stays; internally it converts
  to cents and `fetch_add`s. The `mode_override()` method stays
  pure (reads the cents, derives Decimal, divides — no mutation).
  New method `pub fn try_reserve(&self, estimate_usd: Decimal)
  -> Result<(), LlmError>` — the pre-call gate's atomic compare;
  returns `BudgetExceeded` on overflow.
- `crates/llm/src/budgeted.rs` (new) — `pub struct
  BudgetedProvider<Inner: LlmProvider> { inner: Inner, budget:
  Arc<CostBudget>, sink: Arc<dyn CostSink>, cfg:
  Arc<LlmConfig>, audit_ledger: Option<Arc<audit::Ledger>>,
  last_block_memo_at: AtomicU64 }`. Implements `LlmProvider`
  by delegating to `inner` after the gate.
- `crates/llm/src/factory.rs` (new) — `pub struct
  LlmProviderFactory; impl LlmProviderFactory { pub fn
  build(cfg: &LlmConfig, budget: Arc<CostBudget>, sink:
  Arc<dyn CostSink>, ledger: Option<Arc<audit::Ledger>>) ->
  Result<Arc<dyn LlmProvider>, LlmError> }`. Internally:
  reads keys (R8), constructs the configured leaf provider,
  wraps in `BudgetedProvider`, returns. Single call site for
  consumers.
- `crates/audit/src/journal.rs` (modified) — `pub async fn
  post_llm_budget_event(ledger: &Ledger, kind:
  BudgetEventKind, tier: LlmTier, spent_usd: Decimal,
  ceiling_usd: Decimal) -> Result<(), LedgerError>` (additive).
  Posts a $0.00-USD memo entry with structured tag (no balance
  change; just an audit row). `BudgetEventKind` enum has
  `DegradeToQuickThink | Block`.

#### v2-llm-strategy Q7 — Cost-rate lookup: **hard-coded base table at `crates/llm/src/pricing.rs` + TOML override at `[llm.pricing.<provider>.<model>]`; module owned by the `llm` crate, not `cost`**

**Decision:**

- **Hybrid: hard-coded base + TOML override.** The base table
  is a `match (ProviderKind, model_id_str)` pattern. Every
  provider/model combination the agent uses must compile-time
  resolve to a `PricePerMillionTokens { input_usd:
  Decimal, output_usd: Decimal, cached_input_usd: Decimal }`.
  Unmatched combos return `None`, and `BudgetedProvider`'s
  post-call reconcile treats `None` as a hard error
  (`LlmError::Provider { provider, message: "no price for
  model X" }`) so a typo in model id surfaces loudly.
- **TOML override at `[llm.pricing.<provider>.<model>]`.** The
  agent TOML can override any base-table row for an emergency
  price change without recompiling. Override syntax:
  ```toml
  [llm.pricing.anthropic."claude-opus-4-7"]
  input_usd_per_million        = 15.0
  output_usd_per_million        = 75.0
  cached_input_usd_per_million  = 1.5
  ```
- **Module location: `crates/llm/src/pricing.rs`** (analyst
  prior). The `cost` crate is the recording substrate
  (`CostSink` writes to journal) — not the rate source. The
  rate is LLM-domain-specific (provider × model cartesian).
  The cost crate stays provider-agnostic at v2, which keeps
  the cost↔llm dependency edge clean: `llm` depends on `cost`
  (additive, already in place at architecture.md line 2922);
  `cost` does NOT depend on `llm`.
- **Strawman pricing entries (USD per million tokens) at v2.0.0
  ship time:**
  - **Anthropic Claude Opus 4.7** (`claude-opus-4-7`):
    input $15.00, output $75.00, cached_input $1.50.
  - **Anthropic Claude Haiku 4.5**
    (`claude-haiku-4-5-20251001`): input $1.00, output $5.00,
    cached_input $0.10.
  - **OpenAI GPT-5** (`gpt-5`): input $10.00, output $40.00,
    cached_input $2.50. Operator confirms current price at
    handoff time; if rev'd, T1924's `pricing.rs` literal
    updates and the unit test asserts.
  - **OpenAI GPT-5 mini** (`gpt-5-mini`): input $2.00, output
    $8.00, cached_input $0.50.
  - **Ollama (any model)**: $0.00 across the board (cost-free
    by definition; the cost event still fires with token
    counts so the operator sees throughput per R2.3).
- **Failed-call billing (R9.3): only successful calls bill.**
  Strawman accepted. Anthropic and OpenAI both bill 4xx /
  network failures in some edge cases, but at v2 we eat that
  as goodwill — operator-feedback can lift this to
  "bill everything that hit the wire" in a v3 brief if the
  goodwill bill is meaningful.

**Rationale:**

- **Hard-coded base + TOML override** beats pure-TOML because a
  typo in the model id ("claude-opus-4.7" vs "claude-opus-4-7")
  silently routes to USD = $0 in pure-TOML, which the operator
  doesn't notice until the budget gate underfires. Hard-coded
  base catches typos at compile time (the `match` is exhaustive
  over the supported model set). Override stays for emergency
  price changes.
- **`crates/llm/src/pricing.rs`** keeps the cost crate
  provider-agnostic (a v3 brief can introduce non-LLM cost
  emitters — say data-feed cost — without touching pricing).
  Architecture.md cost-telemetry rationale at line 2870
  explicitly cites "the `llm` crate depends on `cost`, not the
  other way around."
- **API-metadata pricing (Q7 Option C)** rejected: Anthropic
  doesn't expose pricing in any API endpoint; OpenAI's
  `/v1/models` is undocumented for pricing; Ollama has no
  pricing concept. Mixed-source data adds complexity for ~one
  successful provider lookup.
- **Failed-call billing = goodwill** is the safer default
  pre-data; a $200/month operator can stomach a few cents of
  incorrect billing far better than the inverse (operator
  forgets to bill, gets surprised by an Anthropic invoice).

**How it shows up in code:**

- `crates/llm/src/pricing.rs` (new) — `pub struct
  PricePerMillionTokens { input_usd: Decimal, output_usd:
  Decimal, cached_input_usd: Decimal }`. `pub fn
  base_rate(provider: &ProviderKind, model: &str) ->
  Option<PricePerMillionTokens>` — the exhaustive match.
  `pub fn resolve_rate(cfg: &LlmConfig, provider:
  &ProviderKind, model: &str) -> Result<PricePerMillionTokens,
  LlmError>` — checks override first, falls back to base, errors
  on miss.
- `crates/llm/src/pricing.rs` ships an inline test asserting
  every `(ProviderKind, model)` combo named in the v2 default
  TOML resolves to a base rate (no silent zero).
- `crates/llm/src/budgeted.rs` (post-call reconcile) calls
  `resolve_rate(...)` to compute the `usd` field of the emitted
  `CostEvent::Llm`.
- `crates/agent/src/config.rs` (modified) — `LlmConfig` gains
  `pub pricing: HashMap<String /* provider */,
  HashMap<String /* model */, PricePerMillionTokens>>` as an
  override map (default empty → all base rates).

#### v2-llm-strategy Q8 — Replay storage: **SQLite at `data/llm-replay.db` (paper) and `crates/llm/tests/fixtures/llm-replay.db` (fixture); SHA-256 hash over canonical JSON of `(model, system, messages, tools, max_tokens, temperature)`; `schema_version` migration column; no LRU cap; one canned response per provider per role; atomic-write contract via the cost-crate-borrowed atomic write helper**

**Decision:**

- **Q8a — Hash function: SHA-256 over canonical JSON.** Strawman
  accepted with one architect refinement: canonical JSON is
  produced via `serde_json::to_string(...)` after deep-sorting
  every object's keys via the
  [`serde-canonical-json`](https://crates.io/crates/serde-canonical-json)
  crate (compatibility-checked: edition 2024, no system C
  deps, last release < 12 months). The hashed surface is
  `(model: ModelId, system: Vec<SystemBlock>, messages:
  Vec<ChatMessage>, tools: Vec<ToolSchema>, max_tokens: u32,
  temperature: Option<f32>)`. **Excluded:** `correlation_id`
  (a fresh per-call UUID). `temperature` is included because
  it changes the output distribution and `None`-vs-`Some(0.0)`
  is a meaningful distinction.
- **Q8b — Schema migration: `schema_version` column on every
  table.** v1 schema:
  ```sql
  CREATE TABLE llm_replay (
      request_hash      TEXT PRIMARY KEY,    -- 64-char hex
      schema_version    INTEGER NOT NULL,    -- 1 at v2.0.0
      provider          TEXT NOT NULL,
      model             TEXT NOT NULL,
      request_json      TEXT NOT NULL,       -- canonical JSON, debugging
      response_json     TEXT NOT NULL,       -- ChatResponse serialized
      created_at        TEXT NOT NULL,       -- 6-digit fractional ISO
      updated_at        TEXT NOT NULL
  );
  CREATE INDEX llm_replay_provider_idx ON llm_replay(provider);
  ```
  `ReplayProvider` asserts `schema_version <= SUPPORTED_VERSION`
  on open. v3 adds new columns by extending the migration and
  bumping `SUPPORTED_VERSION`.
- **Q8c — No LRU cap.** Operator manages via `rm
  data/llm-replay.db` or a `cargo run --bin llm-smoke -- --reset`
  flag (added at T1936). At v2 token volumes the cache stays
  ≪ 1 GiB even after months of paper runs (response bodies
  are ~5–50 KiB). LRU adds complexity without operator-visible
  benefit at this scale.
- **Q8d — Fixture cache content: one canned response per
  provider per role, captured at brief-write time.** Three
  providers × three foundational roles (`Trader`, `SentimentAnalyst`,
  `Other("smoke")`) = 9 canned responses. Each response is hand-
  authored at T1933 from a one-shot real-API call against the
  smoke prompt; the developer captures the response JSON,
  trims the volatile metadata (request id, latency_ms — these
  are NOT part of the cached response anyway because
  `ChatResponse` has no such fields), and commits the resulting
  binary at `crates/llm/tests/fixtures/llm-replay.db` so
  `cargo test --workspace` is offline-deterministic.
- **Q8e — Concurrent-write safety: SQLite WAL mode + per-process
  single-writer.** WAL handles SQLite-side serialization;
  the `RecordingProvider` uses a
  `tokio::sync::Mutex<sqlx::SqlitePool>` for the writer half so
  two parallel `complete()` calls record sequentially. Reads
  are unblocked.
- **Q8-bonus — Atomic-write contract.** SQLite's WAL journal
  already gives the durability shape we need: a crash mid-write
  leaves the WAL with the failed transaction unapplied, and the
  next open replays. **No additional `atomic_write`-style
  tempfile-rename** is needed because SQLite already enforces
  the atomic-commit contract. Hard constraint #5 is satisfied
  by the WAL contract: a crash mid-write does not leave a
  partial cache entry. Documented at `crates/llm/src/replay.rs`
  module-doc.
- **Q8-bonus — Path config.** `LlmConfig.replay_cache_path:
  PathBuf` per the brief's R12.1; default `./data/llm-replay.db`.
  The fixture path is hard-coded in test setup (not config-
  overridable) so no test can accidentally point at the
  production cache.
- **Q8-bonus — Replay-miss fallthrough.** R6.2 asks if best-
  effort consumers can fall through to a real provider. **No, at
  v2.0.0.** Strict-replay only. Best-effort fallthrough is a v3
  consumer-brief concern (the post-mortem-LLM brief is the most
  likely first user); foundation stays uncomplicated. Documented
  in the runbook at `spec/runbooks/llm-replay.md`.

**Rationale:**

- **SHA-256 over canonicalised JSON** is the canonical hash for
  this kind of cache; deviating breaks every cached response
  and forces a migration on every consumer brief.
- **`serde-canonical-json`** is the only crate that handles
  Rust's HashMap-iteration nondeterminism cleanly; rolling our
  own canonicalisation is a maintenance debt for ~50 lines.
- **`schema_version` column** is the cheapest forward-compat
  story; column adds are SQLite-additive.
- **No LRU** matches v2 scale (cache size < operator-noticeable);
  premature LRU adds eviction-correctness tests without solving
  any observed problem.
- **One canned response per provider per role × 3 = 9 rows**
  is the smallest fixture that exercises the full provider×role
  matrix; smaller (e.g. 1 row per provider) means a future
  per-role consumer brief writes its own fixture extension.
- **WAL + per-process Mutex** is the pattern v0.5 already uses
  for the audit ledger; no architectural novelty.
- **Strict-replay** at v2 is forensically clean: a research-mode
  backtest miss is a build error, not a silent live-API call.
  The product.md "deterministic seeds, no LLM cost" line 292
  is upheld absolutely.

**How it shows up in code:**

- `crates/llm/src/replay.rs` (new) — `pub struct
  RecordingProvider<Inner: LlmProvider> { inner: Inner, pool:
  sqlx::SqlitePool, writer_lock: tokio::sync::Mutex<()> }`,
  `pub struct ReplayProvider { pool: sqlx::SqlitePool }`. Both
  implement `LlmProvider`. Module doc enumerates the 5
  decisions above.
- `crates/llm/src/replay/hash.rs` (new) — `pub fn
  request_hash(req: &ChatRequest) -> String` — SHA-256 of
  canonical JSON. Inline unit test: 1000-iteration
  determinism check.
- `crates/llm/migrations/001_llm_replay.sql` (new) — the
  schema above, run on `RecordingProvider::open` and
  `ReplayProvider::open` via `sqlx::migrate!`.
- `crates/llm/tests/fixtures/llm-replay.db` (new, binary) —
  9 canned rows, captured at T1933.
- `crates/llm/Cargo.toml` (modified) — adds `sqlx = {
  workspace, features = ["sqlite", "runtime-tokio", "chrono"]
  }`, `serde-canonical-json`.

#### v2-llm-strategy Q9 — Rate-limit handling: **exponential backoff with full jitter, 3 retries, no circuit breaker at v2.0.0; per-provider retry policy carried in the leaf provider impl**

**Decision:**

- **Q9a — Retry budget: 3.** Strawman accepted. Backoff base
  500ms, cap 8s, formula `sleep =
  rand::random::<f32>() * min(cap, base * 2^attempt).as_millis()`.
  Total worst-case retry budget: ≤ 0.5s + 1s + 2s + 4s = 7.5s
  + jitter — within the operator's tolerance for an LLM call
  the agent is awaiting.
- **Q9b — No circuit breaker at v2.0.0.** Confirmed. Reasoning:
  - We don't yet have provider-failure-rate observability
    (no Prometheus histogram of LLM call outcomes per
    provider).
  - A degraded provider piling up retry latency for hours
    is an operator-visible problem (the cockpit's LLM tile
    shows the budget burn rate; a sustained-failure provider
    burns budget on retries that don't succeed).
  - Circuit breaker tests are non-trivial and the test
    surface is large.
  - Best response is "operator restarts the agent with a
    swapped TOML provider" — manual but bounded, and aligned
    with the v2 single-VM deployment topology.
  - V3 brief `llm-circuit-breaker` is queued for landing
    after the foundation has lived in production long enough
    to surface real failure-rate signal.
- **Q9c — Jitter formula: full jitter (AWS recommended).**
  `sleep_ms = rng.gen_range(0..=cap_ms)` where `cap_ms =
  min(8000, 500 * 2^attempt)`. Implemented via the
  [`rand::Rng`](https://crates.io/crates/rand) crate (already
  in workspace dev-deps; the architect confirms it's also a
  runtime dep at v2 — added to llm/Cargo.toml).
- **Q9-bonus — Rate-limit policy lives in the leaf provider
  impl** (not in `BudgetedProvider`, not in a wrapping
  `RetryProvider`). Reasoning: each provider has provider-
  specific 429 / 503 / Retry-After-header handling; a generic
  `RetryProvider<Inner>` decorator can't read provider-specific
  headers cleanly. Each impl uses a shared
  `crate::retry::run_with_backoff(operation: F) -> Result<...,
  LlmError>` helper to avoid 3× duplicated retry logic.
- **Q9-bonus — `Retry-After` header.** When a provider returns
  429 with a `Retry-After: <seconds>` header, the leaf provider
  honors it explicitly: the next sleep is `max(retry_after,
  computed_backoff_with_jitter)`. Anthropic and OpenAI both
  send this header; Ollama does not (no rate limits locally).
- **Q9-bonus — Network errors propagate immediately (R7.3).**
  Confirmed. `LlmError::Network` returns from the first failed
  HTTP transport without retry. Transport-level failures
  usually indicate config problems (DNS, TLS, wrong base URL)
  that retries don't fix.

**Rationale:**

- **3 retries** is the AWS-recommended ceiling for non-batch
  workloads. More retries on a sustained 429 amounts to
  rate-limiting ourselves harder.
- **No circuit breaker at v2** matches our observability state
  (no provider-failure-rate signal yet). Adding one without
  signal is guessing the threshold.
- **Per-provider retry impl** is correct because Anthropic's
  429 response shape differs from OpenAI's differs from
  Ollama's (which doesn't 429). A generic decorator would
  smuggle provider-specific knobs into a "generic" type.
- **`Retry-After` honoring** is the smallest correctness
  improvement over pure backoff that's universally beneficial
  (the provider knows when it'll be ready better than we do).

**How it shows up in code:**

- `crates/llm/src/retry.rs` (new) — `pub async fn
  run_with_backoff<F, Fut, T>(max_retries: u8, operation: F)
  -> Result<T, LlmError> where F: Fn() -> Fut, Fut: Future<Output
  = Result<T, RetryError>>`. `RetryError` is an internal enum
  carrying `RateLimited { retry_after: Option<Duration> }`,
  `Transient`, `Fatal(LlmError)`. The fn maps `Fatal` directly,
  retries `RateLimited` and `Transient` with backoff, returns
  `LlmError::RateLimited { retries }` on exhaustion.
- `crates/llm/src/providers/anthropic.rs` (new) — calls
  `retry::run_with_backoff(3, || async { ... })` around the
  reqwest call; classifies `429` → `RateLimited { retry_after:
  parse_retry_after(...) }`, `503` → `Transient`, `400-499` →
  `Fatal(LlmError::Provider { ... })`.
- `crates/llm/src/providers/openai.rs` (new) — same pattern
  against OpenAI's response shape.
- `crates/llm/src/providers/ollama.rs` (new) — `max_retries =
  0` for local Ollama; HTTP failures propagate as
  `LlmError::Network`.
- `crates/llm/Cargo.toml` (modified) — `rand = { workspace }`.

#### v2-llm-strategy Q11 — Operator-success-report `LLM spend` denominator update: **Option C — 1-line denominator hot-fix in this brief; the two `report-sample-*` anchors re-lock once at T_FINAL_V2_LLM_STRATEGY**

**Decision:**

- **Confirmed Option C** (analyst recommendation accepted). The
  denominator string changes from `$135` to `$200` in the body of
  every operator-success-report rendered after v2.0.0 ships.
  Numerator stays `$0.00` under foundation-only scope (Q1 = A;
  no LLM consumers ship in v2.0.0; in `research` mode the
  ReplayProvider posts zero events anyway, so research backtests
  also stay at `$0.00`).
- **Body-byte change.** The exact byte change is:
  ```diff
  - | LLM spend | $0.00 / $135 |
  + | LLM spend | $0.00 / $200 |
  ```
  at `crates/reports/src/render/system_health.rs:66`. Source
  changes:
  - `crates/reports/src/lib.rs:286` — `llm_spend: Ok("$0.00 /
    $135".into())` → `Ok("$0.00 / $200".into())`.
  - `crates/reports/src/lib.rs:320` — `observed: "$0.00 /
    $135".into()` → `"$0.00 / $200".into()`.
  - `crates/reports/src/render/system_health.rs:30` — rustdoc
    example `$0.00 / $135` → `$0.00 / $200`.
  - `crates/reports/src/render/system_health.rs:126` — test
    fixture `$0.00 / $135` → `$0.00 / $200`.
  - `crates/reports/src/render/system_health.rs:139` — test
    assertion `body.contains("| LLM spend | $0.00 / $135 |")`
    → `body.contains("| LLM spend | $0.00 / $200 |")`.
  - The two anchored sample reports at
    `spec/operator-success-reports/reports/success-fixed-report-sample-7d.md:66`
    and `…success-fixed-report-sample-90d.md:66` (same line
    number in both reports) are regenerated via
    `cargo run -p reports --bin report -- --period 7d ...`
    and the new SHA-256s captured.
- **Anchor re-lock cadence.** v1.8 (reflection-memory) re-
  locked the same two anchors at T_FINAL on 2026-05-08; v2.0.0
  re-locks them again at T_FINAL_V2_LLM_STRATEGY. Two re-locks
  in two months is acceptable per the v1+ Q9 anchor-cadence
  policy ("anchor re-locks gate on architect approval; one
  approval per shipping feature with body changes"). The
  architect's approval here is this Q11 sub-section.
- **Bundled with Q5d cache-hit-ratio row.** The `Cache hit
  ratio` row from Q5d also lands in the same renderer, which
  means the body bytes shift further. **Both changes are
  bundled into the single re-lock** (one architect approval,
  one anchor capture, two reports). Q5d's row goes between
  the existing `LLM spend` row and the existing `Funding poll
  status` row at `system_health.rs:66+`. The re-lock procedure
  is single-pass.
- **Tester does the actual `spec/anchors.toml` edit at
  T_FINAL.** The architect captures the new SHAs in Q11's
  developer task body (T1944) so the tester has a copy-paste
  source. **Hard constraint #1 honored** — `spec/anchors.toml`
  is not pre-modified by the architect.
- **9 strategy anchors at lines 15–58 stay byte-identical.**
  `crates/strategy/`, `crates/backtest/` are not touched by v2;
  the negative-invariant test at T1944 confirms via `bash
  scripts/verify_anchors.sh` (the 9 scenario lines all print
  `PASS`).

**Rationale:**

- **Option C** beats Option A and Option B:
  - Option A (full re-render, large body change) is over-
    scoped — v2.0.0 is foundation-only, no real consumer-
    behaviour change yet.
  - Option B (defer to first consumer brief) means v2.0.0
    ships with `$135` shown, contradicting product.md's $200
    ceiling. Operator-confusing.
  - Option C is the smallest body-byte rotation that aligns
    v2.0.0's report with v2.0.0's product.md spec. Two re-
    locks in two months is the natural pace of a scaling
    brief; the body-vs-front-matter discipline (hard
    constraint #8) is honored — the changing byte is in the
    body, named explicitly here, so the tester knows what to
    expect.
- **Bundling Q5d** halves the re-lock work. Both changes touch
  `system_health.rs` within 5 lines of each other; rendering
  twice would be wasted effort.
- **Tester-locks-anchors** preserves the "anchors mutate only
  at architect-approved cadence with tester capture" invariant
  from the v1.5a regression-gate discipline.

**How it shows up in code:**

- `crates/reports/src/render/system_health.rs:30` — rustdoc
  example update.
- `crates/reports/src/render/system_health.rs:66` — actual
  output cell (no change here; the change is upstream, in the
  data feeding `inputs.llm_spend`).
- `crates/reports/src/render/system_health.rs:67` — new
  `writeln!(out, "| Cache hit ratio | {} |", ...)` row.
- `crates/reports/src/render/system_health.rs:126` — test
  fixture string.
- `crates/reports/src/render/system_health.rs:139` — test
  assertion string.
- `crates/reports/src/lib.rs:286` — `llm_spend` default.
- `crates/reports/src/lib.rs:320` — `observed` default.
- `spec/operator-success-reports/reports/success-fixed-report-sample-7d.md:66`
  and `…success-fixed-report-sample-90d.md:66` — regenerated
  via the report binary.
- `spec/anchors.toml:67-75` — **NOT modified by the architect
  or developer**. T_FINAL_V2_LLM_STRATEGY (tester) captures
  the new SHA-256s and edits this file.

### Crate / module surface

The developer creates these new files (relative to
`/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/`):

**In `crates/llm/`:**

1. `crates/llm/src/trait_def.rs` — `LlmProvider` trait,
   `ChatRequest`, `ChatResponse`, `ContentBlock`, `StopReason`,
   `TokenUsage`, `SystemBlock`, `CacheBreakpoint`, `ChatMessage`,
   `MessageRole`, `ModelId`, `LlmTier` re-export from cost,
   `AgentRole` re-export from cost.
2. `crates/llm/src/error.rs` — `LlmError` enum (8 variants).
3. `crates/llm/src/tools.rs` — `ToolSchema`,
   `validate_tool_use(...)`.
4. `crates/llm/src/prompt_cache.rs` — `CachedSystemPrompt` +
   builder + `build_for(ProviderKind)`.
5. `crates/llm/src/observability.rs` — cache event helper,
   Prometheus counter pair.
6. `crates/llm/src/budgeted.rs` — `BudgetedProvider<Inner>`
   decorator.
7. `crates/llm/src/factory.rs` — `LlmProviderFactory::build`.
8. `crates/llm/src/pricing.rs` — base table + `resolve_rate`.
9. `crates/llm/src/retry.rs` — `run_with_backoff` helper.
10. `crates/llm/src/replay.rs` — `RecordingProvider`,
    `ReplayProvider`.
11. `crates/llm/src/replay/hash.rs` — `request_hash`.
12. `crates/llm/migrations/001_llm_replay.sql` — schema v1.
13. `crates/llm/src/auth.rs` — TOML-local config-key reader
    (R8.1, Q3 = Option C).
14. `crates/llm/src/redact.rs` — `redact()` helper (R8.3).
15. `crates/llm/src/providers/mod.rs` — sub-module index.
16. `crates/llm/src/providers/anthropic.rs` — Anthropic
    provider impl.
17. `crates/llm/src/providers/openai.rs` — OpenAI-compat
    provider impl.
18. `crates/llm/src/providers/ollama.rs` — Ollama provider
    impl.
19. `crates/llm/src/bin/llm_smoke.rs` — smoke binary.
20. `crates/llm/tests/fixtures/llm-replay.db` — 9-row fixture
    cache (binary; captured at T1933).
21. `crates/llm/tests/anthropic_provider_test.rs` —
    wiremock-backed integration test.
22. `crates/llm/tests/openai_provider_test.rs` — wiremock-
    backed integration test.
23. `crates/llm/tests/ollama_provider_test.rs` — mock-server
    integration test.
24. `crates/llm/tests/budget_gate_test.rs` — three-scenario
    budget tests (R4 acceptance).
25. `crates/llm/tests/replay_roundtrip_test.rs` — record →
    replay determinism test.
26. `crates/llm/tests/smoke_test.rs` — end-to-end smoke
    against wiremock fixtures.
27. `crates/llm/tests/no_secrets_in_artifacts_test.rs` — V9
    grep test (R8.3 / Q3 invariant).
28. `crates/llm/tests/redact_test.rs` — `redact()` correctness.
29. `crates/llm/tests/config_local_parse_test.rs` —
    `config/agent.toml.local.example` parses (R8.4).

**In root config:**

30. `config/agent.toml.local.example` — committed template
    with placeholder keys (`sk-ant-test-stub-..._00000000` etc.)
    so a fresh checkout passes `cargo test`.

**In runbooks:**

31. `spec/runbooks/llm-cost.md` — operator runbook for cost
    reads + degrade events + price updates (R13.2).
32. `spec/runbooks/llm-replay.md` — operator runbook for
    research-mode replay + `cargo run --bin llm-smoke --mode
    paper` to refresh the cache + interpreting `ReplayMiss`
    failures (R13.3).

The developer modifies these existing files:

1. `crates/llm/src/lib.rs:1` — replace the v0 23-line stub
   with `pub use trait_def::*; pub use error::LlmError; pub
   use tools::*; pub use cost::ProviderKind;` + crate-level
   rustdoc (R13.1).
2. `crates/llm/Cargo.toml` — add the dep set listed in Q4 +
   Q8 + Q9 (`async-trait`, `tokio`, `reqwest`, `serde_json`,
   `jsonschema`, `sqlx`, `serde-canonical-json`, `sha2`,
   `uuid`, `rust_decimal`, `rust_decimal_macros`, `tracing`,
   `rand`, `cost = { path = "../cost" }`,
   `audit = { path = "../audit" }`); dev deps `wiremock`,
   `tempfile`. Edition `2024` already inherited from workspace.
3. `crates/cost/src/event.rs:9` — rename `pub enum
   LlmProvider` to `pub enum ProviderKind`. Mechanical rename;
   `serde(rename_all = "snake_case")` already preserves on-the-
   wire shape.
4. `crates/cost/src/event.rs:60` — `provider: LlmProvider` →
   `provider: ProviderKind` in the `CostEvent::Llm` variant.
5. `crates/cost/src/lib.rs:11` — re-export rename.
6. `crates/cost/src/sink.rs:76,80` — test imports + fixture
   construction site update.
7. `crates/cost/src/budget.rs:14` — `spent_usd: Decimal` →
   `spent_cents: AtomicU64`. Add `try_reserve(...)` method.
8. `crates/audit/src/query.rs:36`-adjacent — additive
   `pub async fn cache_hit_ratio_since(...)`.
9. `crates/audit/src/journal.rs` — additive
   `post_llm_budget_event(...)`. `BudgetEventKind` enum.
10. `crates/agent/src/config.rs:300` — `LlmConfig` struct
    (new section). The struct loads from a layered merge of
    `config/agent.toml` (committed defaults) + `config/agent.toml.local`
    (operator-only secrets). The merge order: `Config::load`
    reads the canonical `agent.toml`, then if a sibling
    `agent.toml.local` exists at the same parent dir, it's
    parsed into a `LocalOverrideConfig` and overlaid via
    `serde_json::Map::extend` semantics on the LLM section
    only (per Q3 = C operator decision: "keep all config in
    one place"). Missing `.local` file under `mode = "paper"`
    or `mode = "live"` and `default_provider = "anthropic"` →
    `LlmError::Auth` at startup.
11. `crates/agent/src/main.rs` — wire `LlmProviderFactory::build(...)`
    behind `cfg.llm.enabled` (default false, since v2.0.0 has
    no consumers). When true, agent main constructs the
    provider once at startup; the resulting `Arc<dyn
    LlmProvider>` is stored on the runtime context as
    `Option<Arc<dyn LlmProvider>>` for future consumer briefs
    to pluck. **No bus channel added** (hard constraint #4).
12. `config/agent.toml` — append the new
    `[llm] / [llm.deep_think] / [llm.quick_think] /
    [llm.providers.anthropic] / [llm.providers.openai] /
    [llm.providers.ollama]` sections per R12.1. **Keys are
    NOT in this file** — keys live in `config/agent.toml.local`
    per Q3 = C.
13. `crates/reports/src/lib.rs:286` — denominator string update.
14. `crates/reports/src/lib.rs:320` — denominator string update.
15. `crates/reports/src/render/system_health.rs:30` — rustdoc
    update.
16. `crates/reports/src/render/system_health.rs:66+` — new
    `Cache hit ratio` row (Q5d).
17. `crates/reports/src/render/system_health.rs:126,139` —
    test fixture / assertion updates.
18. `crates/reports/tests/report_scenarios.rs:80,88` —
    `EXPECTED_SHA_7D`, `EXPECTED_SHA_90D` constants update at
    T1944 (developer pre-stages the new SHAs; tester locks
    `spec/anchors.toml:67-75` at T_FINAL).
19. `spec/operator-success-reports/reports/success-fixed-report-sample-7d.md:66`
    — regenerated body line (output of the report binary, not
    a hand-edit).
20. `spec/operator-success-reports/reports/success-fixed-report-sample-90d.md:66`
    — same regeneration.
21. `spec/architecture.md:421-432` — replace the LLM
    integration stub with a v2 reference: "see `### v2 — LLM
    strategy resolutions (Q4–Q11) — confirmed 2026-05-10`."
22. `spec/architecture.md` (append) — new decisions-index
    section `### v2 — LLM strategy resolutions (Q4–Q11) —
    confirmed 2026-05-10` with one-paragraph summaries of each
    Q-resolution + a back-pointer to this Design section.

### Verification update

Q6c's documented 0.2% concurrent-overshoot bound adds one new
verification gate:

- **V12 Concurrent-call budget overshoot bound.** Stress test:
  fire 10 parallel `complete()` calls against a wiremock
  pinned at 200ms latency with `BudgetedProvider` seeded at
  $199.50 / $200; assert all 10 return successfully (the gate
  passes them all because the pre-call estimates fit) AND the
  post-reconcile `spent_cents` is at most $200.40 (10 × $0.10
  max overshoot per call). Failure → architect (race semantics
  changed).

V1–V11 from the analyst's brief stay as-written. V12 lands
because Q6's atomic-decision design surfaces a documented
overshoot bound that needs a regression test.

## Changelog

- 2026-05-10 (analyst): initial brief — foundation-only
  scope-split recommendation in Q1, twelve open questions
  surfaced for architect/operator resolution, R1–R14 derived
  from product.md LLM strategy + cost economics + operating
  modes sections, V1–V11 verification contract, six smoke +
  replay + budget backtest scenarios. Q1, Q2, Q3, Q10 block
  architect handoff. HANDOFF → architect pending operator
  resolution of Q1, Q2, Q3, Q10.
- 2026-05-10 (orchestrator, operator-relayed via chat):
  operator resolved the four [OPERATOR-DECIDE] questions —
  - **Q1 → Option A** (foundation-only; consumers become
    follow-up briefs).
  - **Q2 → Option A** (Anthropic both tiers — Opus 4.7 +
    Haiku 4.5).
  - **Q3 → Option C** (config file `config/agent.toml.local`,
    overriding analyst's Option A recommendation). R8.1 and
    R8.2 rewritten to point at the local-config path; R8.3
    and the V9 forbidden-substring test extended to cover
    every artifact written during a smoke run, not just logs;
    R8.4 adds a `config/agent.toml.local.example` template
    so fresh checkouts pass `cargo test` without operator
    intervention. `.gitignore` updated in the same commit
    to cover `*.toml.local` and `config/*.local` —
    accidental-commit risk (the Option C downside) is
    mechanically blocked.
  - **Q10 → strawman accepted** (cockpit tile + audit memo +
    success-report line; email/Slack/push deferred to v3).
  Seven [ARCHITECT-DECIDE] questions remain (Q4 trait shape,
  Q5 prompt-cache strategy, Q6 budget gate placement, Q7
  cost-rate lookup, Q8 replay storage, Q9 rate-limit handling,
  Q11 report denominator update). Q12 is informational. Routing
  → architect.
- 2026-05-10 (architect): landed `## Design` section resolving
  the seven [ARCHITECT-DECIDE] questions —
  - **Q4 → async + non-streaming + tool-use-from-day-one +
    batch-deferred + `serde_json::Value` schema + 8-variant
    `LlmError`**; cost-crate enum `LlmProvider` renames to
    `ProviderKind` to free the trait name (mechanical rename;
    one re-export update; ~12 call sites in cost crate).
  - **Q5 → TTL-driven + 2 breakpoints (project + role) +
    provider-aware builder via `build_for(ProviderKind)` +
    per-role per-day cache-hit-ratio Prometheus counter pair +
    `audit::query::cache_hit_ratio_since` additive read** for
    the report's new System Health row.
  - **Q6 → factory-level `BudgetedProvider<Inner>` decorator +
    `AtomicU64`-backed atomic cents counter + `max_tokens`-
    based pre-call estimate + 0.2% concurrent-overshoot bound**
    documented and regression-tested at V12 (new gate).
  - **Q7 → hybrid hard-coded base table at
    `crates/llm/src/pricing.rs` + TOML override at
    `[llm.pricing.<provider>.<model>]`**; pricing module owned
    by the `llm` crate (architecture.md line 2922 invariant
    preserved: `llm` depends on `cost`, not the other way).
  - **Q8 → SQLite at `data/llm-replay.db` (paper) and
    `crates/llm/tests/fixtures/llm-replay.db` (fixture, 9
    rows = 3 providers × 3 roles); SHA-256 of canonical JSON
    via `serde-canonical-json`; `schema_version` column for
    forward-compat; no LRU; WAL + per-process `Mutex` for
    concurrent-write safety; strict-replay-only at v2 (best-
    effort fallthrough deferred to v3)**.
  - **Q9 → exponential backoff + full jitter + 3 retries + no
    circuit breaker at v2.0.0 + retry policy in the leaf
    provider impl + `Retry-After` header honored** when present.
  - **Q11 → Option C — 1-line denominator hot-fix from `$135`
    to `$200` in this brief; bundled with the Q5d `Cache hit
    ratio` System Health row addition; the two `report-sample-*`
    anchors at `spec/anchors.toml:67-75` re-lock once at
    `T_FINAL_V2_LLM_STRATEGY` (tester captures the new SHA-
    256s; architect does NOT pre-modify `spec/anchors.toml`)**.
  Tasks expanded at `tasks.md` — T1901 → T1945 +
  `T_FINAL_V2_LLM_STRATEGY`. New verification gate V12 added
  to the brief's Verification section (concurrent-overshoot
  bound). Crate / module surface enumerated: 32 new files +
  22 existing files modified. Architecture.md decisions index
  gains a v2 LLM section pointing back to this Design.
  HANDOFF → developer.
- 2026-05-10 (orchestrator, paused): operator paused at the
  architect → developer handoff with *"Write it down for now.
  I will come to this point a while later."* Resumption
  breadcrumb at
  [`orchestrator-scope-check-2026-05-10.md`](orchestrator-scope-check-2026-05-10.md)
  — that file holds the orchestrator's pre-developer-spawn
  scope-check, the three resumption-time decisions (Q4 bonus
  rename keep/defer; Q8 strict-vs-best-effort replay; Q11
  denominator bundle/defer), the surface-area summary, and
  the recommended next move. Backlog Active entry annotated
  PAUSED with a forward-pointer to the same file. Defaults
  are "accept all three" (A / A / C) which matches the
  architect's picks. No code or spec mutation pending; the
  architect's Design and T1901–T1945 task list are committed
  and ready for the developer to consume on resumption.
- 2026-05-12 (orchestrator, RESUMED): operator confirmed
  D1=A / D2=A / D3=C at orchestrator defaults. Developer
  spawned across 6 multi-pass cycles.
- 2026-05-12/13 (developer, 6 passes): T1901-T1945 (44/45
  ticked; T1938 cockpit "LLM budget" tile deferred to v2.1).
  Commits d0bcad2 (pass 1, T1901) → c61afa5 (pass 2, M2) →
  441c136 (pass 3, M3+M4) → f1dbe05 (pass 4, M5 + T1912
  flip) → f1128e9 (pass 5, M6 + T1913 flip) → faaaec1 (pass
  6, M7). Two [~] partials flipped to [x] mid-cycle as their
  dependencies landed.
- 2026-05-13 (tester): T_FINAL_V2_LLM_STRATEGY VERDICT →
  PASS (commit 8a41b47). Re-locked 2 report-sample-*
  anchors to v2.0.0 SHAs. V1-V12 all PASS (V1 partial-non-
  blocking: 2 pedantic clippy on audit/src/query.rs:219,221
  queued for v2.1). Workspace 1203 passing, 0 failed.
- 2026-05-13 (operator): **SHIPPED**. Approval recorded
  in [`presentations/v2-llm-strategy-2026-05-13.md ## Approval`](presentations/v2-llm-strategy-2026-05-13.md#approval)
  as `[x] Approved — ship`. Status flipped `in-progress →
  shipped`. Foundation-only (Q1=A) — first consumer briefs
  queued: reflection-memory-llm-enrichment +
  reflection-memory-trader-wiring. Also unblocks Kronos
  v2.5 + Lumen Phase 6 + the v2.1 follow-up cluster
  (T1938 / T1915 / T1910 clippy).
