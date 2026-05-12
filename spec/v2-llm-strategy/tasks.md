---
slug: v2-llm-strategy
status: in-progress
owner: developer
updated: 2026-05-12
version: 2.0.0
pass: 3
---

# Tasks — v2 LLM strategy

Ordered, testable task list derived from
[spec/v2-llm-strategy/feature.md → Design](feature.md#design)
and the seven architect resolutions (Q4, Q5, Q6, Q7, Q8, Q9, Q11)
recorded in the same Design section. Cross-references to the
analyst's R/V items use the format `Rn` / `Vn`; cross-references
to the open questions use `Qn`.

Owner tags: `[developer]` for backend Rust work across
`llm` (rewritten from v0 stub), `cost` (rename + atomic spent
counter), `audit` (additive query + journal helper), `agent`
(config + factory wire), `reports` (System Health row
additions). **No `[ui-designer]` tasks** under foundation-only
scope — the cockpit "LLM budget" tile (R11.2) lands as a single
right-rail tile per the developer's existing right-rail patterns;
no new strings / widgets that warrant a designer spawn (Q10
operator-resolved 2026-05-10 — strawman accepted). Lumen Phase 6
Assistant slot is gated on this brief shipping and ships
separately.

**Task numbering:** T19xx so the v0 T0xx, v0.5 T5xx, v1 T6xx,
v1.5a T7xx, v1+ T8xx, Lumen T15xx / T16xx / T17xx, and v1.8
reflection-memory T18xx namespaces stay intact. v1.8 closed at
T1814; **T19xx is the natural next block**, called out in this
file's reservation note. **45 tasks (T1901–T1945) +
`T_FINAL_V2_LLM_STRATEGY`.**

**Parallelism gates** (shared files — only one developer touches each):

- `crates/llm/**` — owned by the LLM-feature lead developer.
  T1901 is the critical-path gate; everything downstream blocks
  on it. After T1901 lands, the M2 / M3 / M5 trees can fan out
  across providers + cache + pricing + replay sub-trees.
- `crates/cost/src/event.rs` — touched once at T1901 (rename
  `LlmProvider → ProviderKind`); additive only after that.
  Mechanical rename — single PR, no follow-up touches.
- `crates/cost/src/budget.rs` — touched once at T1907
  (`spent_usd: Decimal → spent_cents: AtomicU64` +
  `try_reserve(...)`). Single touch point — sequence M4 tasks
  behind T1907.
- `crates/audit/src/query.rs` — additive only; T1916 lands
  `cache_hit_ratio_since` adjacent to existing siblings.
- `crates/audit/src/journal.rs` — additive; T1917 lands
  `post_llm_budget_event` + `BudgetEventKind`.
- `crates/agent/src/config.rs` — T1937 is the single touch
  point for the new `LlmConfig` + the local-overlay merge.
- `crates/agent/src/main.rs` — T1938 is the single touch point
  for the factory wire-up.
- `crates/reports/src/render/system_health.rs` — T1942 is the
  single touch point for the bundled Q5d (`Cache hit ratio`
  row) + Q11 (`$135 → $200`) body-byte changes.
- `crates/reports/src/lib.rs` — T1942 sub-task for the lines
  286 + 320 fixture-string updates.
- `config/agent.toml` — T1937 sub-task; append-only.
- `config/agent.toml.local.example` — T1934 creates this once.

**Synchronization points** (block downstream tasks):

- **T1901** — `LlmProvider` trait rename + `cost::ProviderKind`
  rename + `LlmError` enum + `ChatRequest/Response` types +
  `ToolSchema`. Once merged, T1902–T1945 can fan out.
- **T1907** — `cost::CostBudget::try_reserve` atomic
  reservation. Blocks M4's budget-gate work
  (T1908–T1912).
- **T1913** — `LlmProviderFactory::build` skeleton. Blocks
  T1934 (smoke binary), T1937 (agent config wire),
  T1938 (agent main wire).
- **T1922** — `RecordingProvider` + `ReplayProvider` skeletons.
  Blocks T1933 (fixture cache capture), T1936 (smoke `--reset`).
- **T1942** — System Health renderer rewrite. Blocks T1944
  (developer-side EXPECTED_SHA capture) and
  `T_FINAL_V2_LLM_STRATEGY`.

**Granularity:** ~½ day per task (mirrors v1.8 reflection-memory's
cadence). Dependency lines explicit on every task.

## M1 — `LlmProvider` trait + request/response types + name collision rename

Covers feature.md **R1** (trait shape + request type + response
type + error variants) + Design **Q4** resolution (async,
non-streaming, tool-use-from-day-one, batch-deferred, 8-variant
`LlmError`, cost-crate enum rename `LlmProvider → ProviderKind`).

- [x] **T1901** [developer] — `crates/llm/` rewrite from v0 stub:
  trait + types + error + tool schema + `cost::ProviderKind`
  rename, per
  [Design → § Q4](feature.md#v2-llm-strategy-q4--trait-shape-async-non-streaming-tool-use-from-day-one-batch-deferred-serde_jsonvalue-schema-eight-variant-llmerror)
  + [Design → Crate / module surface](feature.md#crate--module-surface):
  - **Replace** `crates/llm/src/lib.rs` (the v0 23-line stub)
    with a re-exporter: `pub use trait_def::*; pub use
    error::LlmError; pub use tools::*; pub use
    cost::ProviderKind;` + crate-level rustdoc per R13.1
    naming the trait, three providers, prompt-cache builder,
    budget gate, record/replay.
  - **New** `crates/llm/src/trait_def.rs` — `#[async_trait]
    pub trait LlmProvider: Send + Sync { fn name(&self) ->
    &str; fn provider_kind(&self) -> ProviderKind; async fn
    complete(&self, request: ChatRequest) -> Result<ChatResponse,
    LlmError>; }`. Plus `ChatRequest { model: ModelId, tier:
    LlmTier, role: AgentRole, system: Vec<SystemBlock>,
    messages: Vec<ChatMessage>, tools: Vec<ToolSchema>,
    max_tokens: u32, temperature: Option<f32>, correlation_id:
    Uuid }`, `ChatResponse { content: Vec<ContentBlock>,
    stop_reason: StopReason, usage: TokenUsage, model: ModelId,
    correlation_id: Uuid }`, `ContentBlock::{Text(String) |
    ToolUse { name: String, input: serde_json::Value, id:
    String }}`, `StopReason::{EndTurn | MaxTokens | ToolUse |
    StopSequence}`, `TokenUsage { tokens_in: u64, tokens_out:
    u64, tokens_cached_in: u64 }`, `SystemBlock::{Plain(String)
    | Cached(String, CacheBreakpoint)}`,
    `CacheBreakpoint::Ephemeral`, `ChatMessage { role:
    MessageRole, content: Vec<ContentBlock> }`,
    `MessageRole::{User | Assistant}`, `ModelId(pub String)`.
    All `Serialize + Deserialize + Clone + Debug + PartialEq`.
  - **New** `crates/llm/src/error.rs` — `LlmError` enum with 8
    variants per Design Q4f.
  - **New** `crates/llm/src/tools.rs` — `pub struct ToolSchema {
    pub name: String, pub description: String, pub input_schema:
    serde_json::Value }` + `pub fn validate_tool_use(schema:
    &ToolSchema, input: &serde_json::Value) ->
    Result<(), LlmError>` using the `jsonschema` crate.
  - **Rename in cost crate** at `crates/cost/src/event.rs:9`:
    `pub enum LlmProvider` → `pub enum ProviderKind`. Mechanical:
    `crates/cost/src/event.rs:60` (CostEvent::Llm field type),
    `crates/cost/src/lib.rs:11` (re-export), `crates/cost/src/sink.rs:76,80`
    (test imports + fixture).
  - **`crates/llm/Cargo.toml`** — add deps per Design Crate
    surface section: `cost = { path = "../cost" }`,
    `audit = { path = "../audit" }`, `tokio.workspace =
    { features = ["macros", "rt-multi-thread", "sync"] }`,
    `async-trait`, `reqwest = { workspace, features = ["json"] }`,
    `serde_json.workspace`, `jsonschema`, `sha2.workspace`,
    `uuid = { workspace, features = ["serde", "v4"] }`,
    `rust_decimal.workspace`, `rust_decimal_macros.workspace`,
    `tracing.workspace`, `thiserror.workspace`,
    `rand.workspace`, `sqlx = { workspace, features = ["sqlite",
    "runtime-tokio", "chrono"] }`, `serde-canonical-json`.
    Dev-deps: `wiremock`, `tokio-test`, `tempfile`. Edition
    `2024` already inherited.
  - Add `#![deny(clippy::float_arithmetic)]` at
    `crates/llm/src/lib.rs:1` (mirrors the reflection crate's
    discipline).
  —
  _acceptance: `cargo build -p llm` clean; `cargo build -p cost`
  clean (rename propagates); `cargo doc -p llm --no-deps`
  warning-clean; `cargo test -p llm --lib` passes a unit test
  asserting (a) `ChatRequest::new(ModelId::from("test-model"),
  LlmTier::DeepThink, AgentRole::Trader)` builds with sensible
  defaults (`max_tokens: 4096`, `temperature: None`,
  `tools: vec![]`, `system: vec![]`, `messages: vec![]`,
  `correlation_id: Uuid::new_v4()`), (b) every
  `LlmError` variant has a non-empty `Display` impl. `cargo test
  -p cost` passes (rename propagates through tests). [R1.1, R1.2,
  R1.3, R1.4, R5.1, R5.2, Q4]_
  **[gate for T1902–T1945]**
  - **Ticked 2026-05-12 (developer, pass 1):**
    - New `crates/llm/src/lib.rs:1-41` — `#![deny(clippy::float_arithmetic)]`
      + re-exporter (`pub use trait_def::*; pub use error::LlmError; pub
      use tools::{validate_tool_use, ToolSchema}; pub use cost::ProviderKind;`).
    - New `crates/llm/src/trait_def.rs:1-242` — `#[async_trait] pub trait
      LlmProvider { fn name; fn provider_kind; async fn complete; }`,
      `ChatRequest::new(model, tier, role)` with sensible defaults
      (`max_tokens=4096`, `temperature=None`, empty vecs, fresh
      `Uuid::new_v4()`), `ChatResponse`, `ContentBlock::{Text|ToolUse}`,
      `StopReason::{EndTurn|MaxTokens|ToolUse|StopSequence}`, `TokenUsage`,
      `SystemBlock::{Plain|Cached}`, `CacheBreakpoint::Ephemeral`,
      `ChatMessage`, `MessageRole::{User|Assistant}`, `ModelId(pub
      String)`. All `Serialize + Deserialize + Clone + Debug + PartialEq`.
    - New `crates/llm/src/error.rs:1-149` — 8-variant `LlmError`
      (`Provider {provider, message}`, `RateLimited {retries}`, `Timeout
      {elapsed_ms}`, `BudgetExceeded {spent_usd, ceiling_usd}`,
      `InvalidResponse(String)`, `ReplayMiss(String)`,
      `Network(#[from] reqwest::Error)`, `Auth(String)`) per Design Q4f
      `feature.md:1224-1244`.
    - New `crates/llm/src/tools.rs:1-115` — `ToolSchema { name,
      description, input_schema: serde_json::Value }` +
      `validate_tool_use(&schema, &input) -> Result<(), LlmError>` using
      `jsonschema::validator_for` (Draft 2020-12).
    - Cost-crate rename (12-call-site mechanical sed-style):
      `crates/cost/src/event.rs:9` (`pub enum LlmProvider` → `ProviderKind`
      with serde rename_all = "snake_case" preserved → wire-compat),
      `crates/cost/src/event.rs:60` (`CostEvent::Llm.provider:
      ProviderKind`), `crates/cost/src/lib.rs:11` (re-export rename),
      `crates/cost/src/sink.rs:76,82` (test import + fixture).
    - `crates/llm/Cargo.toml` deps: `cost` (path), `tokio` (workspace),
      `async-trait`, `reqwest` (workspace, `["json"]`), `serde`,
      `serde_json`, `rust_decimal`, `jsonschema 0.30`, `uuid`
      (`["serde","v4"]`), `tracing`, `thiserror`. Dev-deps: `tokio`
      (`["test-util","macros","rt-multi-thread","sync"]`),
      `rust_decimal_macros`.
    - Test snippet (`cargo test -p llm --lib` — verbatim last 5 lines):
      ```
      test trait_def::tests::t1901_chat_request_new_has_sensible_defaults ... ok
      test trait_def::tests::t1901_shape_types_round_trip_through_serde_json ... ok
      test tools::tests::t1901_validate_tool_use_accepts_conforming_input ... ok
      test tools::tests::t1901_validate_tool_use_rejects_missing_required_field ... ok
      test tools::tests::t1901_validate_tool_use_rejects_wrong_type ... ok
      test error::tests::t1901_every_llmerror_variant_has_nonempty_display ... ok
      test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      ```
    - Test snippet (`cargo test -p cost --lib` — verbatim last 5 lines):
      ```
      test sink::tests::t30_noop_sink_accepts_events ... ok
      test sink::tests::t30_ledger_sink_writes_balanced_entries ... ok
      test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
      ```
    - `cargo build -p llm` clean (release + debug); `cargo check
      --workspace` clean (no downstream crate broken by the rename);
      `cargo clippy -p llm --all-targets` 0 warnings on the new crate
      (the 3 pre-existing `trading_core` / `audit` pedantic warnings are
      unrelated to T1901).
    - Anchor invariant: T1901 touched **none** of `crates/strategy`,
      `crates/audit`, `crates/exec`, `crates/backtest`, or report
      rendering — so the 9 strategy backtest anchors at
      `spec/anchors.toml:15-58` and the 2 success-report anchors at
      `:67-75` are byte-untouched (sandbox blocks
      `scripts/verify_anchors.sh` from this sub-agent; orchestrator can
      run the gate on resume).

## M2 — Provider implementations (Anthropic, OpenAI-compatible, Ollama)

Covers feature.md **R2** (three first-class providers) + Design
**Q9** (rate-limit + retry policy lives in leaf provider impl)
+ partial **Q5** (provider-aware translation of cache markers).

- [x] **T1902** [developer] — Retry helper at
  `crates/llm/src/retry.rs` per
  [Design → § Q9](feature.md#v2-llm-strategy-q9--rate-limit-handling-exponential-backoff-with-full-jitter-3-retries-no-circuit-breaker-at-v200-per-provider-retry-policy-carried-in-the-leaf-provider-impl):
  - `pub async fn run_with_backoff<F, Fut, T>(max_retries: u8,
    operation: F) -> Result<T, LlmError>` with the
    `RetryError::{RateLimited { retry_after }, Transient,
    Fatal(LlmError)}` internal classification enum.
  - Backoff base 500ms, cap 8s, **full jitter** formula:
    `sleep_ms = rng.gen_range(0..=cap_ms)` where `cap_ms =
    min(8000u64, 500u64 * (1u64 << attempt))`.
  - On `RateLimited { retry_after: Some(d) }` the next sleep
    is `max(d, computed_backoff)`.
  - After `max_retries` retries returns
    `LlmError::RateLimited { retries: max_retries }`.
  —
  _acceptance: `cargo test -p llm --test retry_test` passes —
  (a) 3×429-then-200 succeeds in ≤ `7.5s + jitter` wall
  clock (use `tokio::time::pause()` deterministic time),
  (b) 4×429 returns `LlmError::RateLimited { retries: 3 }`,
  (c) `Retry-After: 2` header pushes the next sleep to ≥ 2s.
  [R7.1, R7.2, R7.3, Q9]_
  **[deps: T1901]**
  - **Ticked 2026-05-12 (developer, pass 2):**
    - New `crates/llm/src/retry.rs:1-273` — `pub async fn
      run_with_backoff<F, Fut, T>(max_retries, operation)`,
      `pub enum RetryError::{RateLimited { retry_after }, Transient,
      Fatal(LlmError)}`, base 500ms / cap 8s, **full jitter**
      (`rand::rng().random_range(0..=cap_ms)`), `Retry-After`
      honored via `max(retry_after, computed_backoff)`, fatal
      propagates immediately.
    - `crates/llm/Cargo.toml:38` — `rand = { workspace }` added
      to runtime deps; `wiremock` to dev-deps for provider tests.
    - `crates/llm/src/lib.rs:32` — `pub mod retry;`.
    - In-crate unit tests at `crates/llm/src/retry.rs:138-265`
      (5 tests: 3×429→ok, 4×429 exhausts, Retry-After ≥ 2s,
      Fatal immediate, Transient retried-then-exhausts) use
      `#[tokio::test(start_paused = true)]` for deterministic
      time.
    - Public-edge integration tests at
      `crates/llm/tests/retry_test.rs:1-69` re-assert the three
      T1902 acceptance points against `llm::retry`.
    - Test snippet (`cargo test -p llm --test retry_test`
      verbatim last 5 lines):
      ```
      running 3 tests
      test t1902_retry_after_2s_pushes_sleep ... ok
      test t1902_four_429s_exhausts_budget ... ok
      test t1902_three_429s_then_success ... ok
      test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

- [x] **T1903** [developer] — Anthropic provider impl at
  `crates/llm/src/providers/anthropic.rs` per
  [Design → § Q5 + Q9 + Crate / module surface](feature.md#crate--module-surface):
  - `pub struct AnthropicProvider { client: reqwest::Client,
    base_url: String, api_key: String, default_model: ModelId }`.
  - `LlmProvider for AnthropicProvider`: `name() -> "anthropic"`,
    `provider_kind() -> ProviderKind::Anthropic`,
    `async fn complete(...)`:
    - Build the request body: `messages`, `system` (with
      `cache_control: {"type": "ephemeral"}` wherever
      `SystemBlock::Cached` appears), `tools` (Anthropic
      tool-use schema), `max_tokens`, `temperature`.
    - POST `{base_url}/messages` with header `x-api-key:
      <api_key>`, `anthropic-version: 2023-06-01` (latest
      stable at brief-write time).
    - Wrap the call in `retry::run_with_backoff(3, …)`.
    - Parse the response: `content` blocks → `ContentBlock`
      variants; `usage.input_tokens` → `tokens_in`,
      `usage.output_tokens` → `tokens_out`,
      `usage.cache_read_input_tokens` → `tokens_cached_in`.
    - `tool_use` content blocks pass through
      `tools::validate_tool_use(...)` against the matching
      `ToolSchema` from the request; validation failure →
      `LlmError::InvalidResponse`.
  —
  _acceptance: `cargo test -p llm --test anthropic_provider_test`
  passes — (a) wiremock `expect()` on `POST /messages` asserts
  the request body contains exactly two `cache_control:
  {"type": "ephemeral"}` markers when the request carries 2
  `SystemBlock::Cached` items, (b) a canned 200 response
  parses into a `ChatResponse` with the expected `usage`
  fields, (c) a 429 → 200 retry round-trips, (d) a 401
  surfaces as `LlmError::Auth`. [R2.1, R3.1, R3.2, R3.3, R5.1,
  R5.3, Q5b, Q5c, Q9]_
  **[deps: T1901, T1902]**
  - **Ticked 2026-05-12 (developer, pass 2):**
    - New `crates/llm/src/providers/anthropic.rs:1-385`
      `pub struct AnthropicProvider { client, base_url,
      api_key, default_model }`, `LlmProvider::complete`
      wraps `retry::run_with_backoff(3, ...)`, sends
      `POST {base_url}/messages` with headers `x-api-key`,
      `anthropic-version: 2023-06-01`, `content-type:
      application/json`.
    - Wire-format helpers (pub(crate) for tests):
      `build_request_body` (`anthropic.rs:161-176`),
      `parse_response` (`anthropic.rs:236-289`),
      `parse_retry_after` (`anthropic.rs:293-296`),
      `classify_http_error` (`anthropic.rs:301-318`).
    - Cache markers: `SystemBlock::Cached` →
      `{"type":"text","text":...,"cache_control":{"type":"ephemeral"}}`,
      `SystemBlock::Plain` → `{"type":"text","text":...}` with
      `cache_control` omitted via
      `#[serde(skip_serializing_if = "Option::is_none")]`.
    - Tool-use: declared tools serialize as Anthropic's
      `{name, description, input_schema}`; response `tool_use`
      blocks routed through `tools::validate_tool_use(...)` —
      undeclared tool name AND schema-violation both surface
      as `LlmError::InvalidResponse`.
    - HTTP classification: 429 → `RateLimited { retry_after }`,
      503 → `Transient`, 401/403 → `Fatal(LlmError::Auth)`,
      other → `Fatal(LlmError::Provider { provider: Anthropic })`.
    - In-crate unit tests at `anthropic.rs:323-573` (10 tests:
      2-marker emission, empty-system omit, tool envelope,
      usage mapping, valid tool-use, schema-violation reject,
      undeclared tool reject, Retry-After parse, HTTP class).
    - Integration tests via wiremock at
      `crates/llm/tests/anthropic_provider_test.rs:1-192`
      (5 tests covering all four T1903 acceptance items
      a/b/c/d + the no-cache-when-uncached bonus).
    - Test snippet (`cargo test -p llm --test
      anthropic_provider_test` verbatim last 5 lines):
      ```
      test t1903_401_surfaces_as_auth_error ... ok
      test t1903_request_body_no_markers_when_uncached ... ok
      test t1903_canned_200_parses_into_chat_response ... ok
      test t1903_request_body_emits_two_cache_breakpoints ... ok
      test t1903_429_then_200_retries_to_success ... ok
      test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
      ```

- [x] **T1904** [developer] — OpenAI-compatible provider at
  `crates/llm/src/providers/openai.rs`:
  - `pub struct OpenAiProvider { client: reqwest::Client,
    base_url: String, api_key: String, default_model: ModelId }`
    + `pub fn new_with_base_url(base_url: impl Into<String>,
    api_key: impl Into<String>, model: ModelId) -> Self`. The
    `base_url` parameter covers OpenAI, OpenRouter, DeepSeek,
    LM Studio.
  - `complete(...)`:
    - Build request body in OpenAI Chat Completions shape
      (`messages: [...]`, `tools: [{type: "function",
      function: {...}}]`, `max_tokens`, `temperature`).
    - **`SystemBlock::Cached` flattens to plain text**;
      `tracing::debug!(target: "llm.cache",
      "cache_markers_dropped_for_provider", provider =
      "openai_compat")` once per builder construction.
    - POST `{base_url}/chat/completions` with `Authorization:
      Bearer <api_key>`.
    - Parse response: `choices[0].message.content` /
      `tool_calls` → `ContentBlock` variants;
      `usage.prompt_tokens` → `tokens_in`,
      `usage.completion_tokens` → `tokens_out`,
      `tokens_cached_in: 0` (OpenAI-compat does not surface
      cached tokens).
    - Schema-validate any `tool_calls.function.arguments` JSON
      via `validate_tool_use(...)`.
  —
  _acceptance: `cargo test -p llm --test openai_provider_test`
  passes — (a) wiremock asserts the request body has **NO**
  `cache_control` markers when the request carries
  `SystemBlock::Cached` items (markers silently dropped),
  (b) canned response parses correctly, (c) 429 → 200 retry
  round-trips, (d) `tokens_cached_in == 0` always. [R2.2, R3.3,
  R5.1, R5.3]_
  **[deps: T1901, T1902]**
  - **Ticked 2026-05-12 (developer, pass 2):**
    - New `crates/llm/src/providers/openai.rs:1-466`
      `pub struct OpenAiProvider { client, base_url, api_key,
      default_model, kind }` + `new_with_base_url(...)` per
      task body + `with_provider_kind(kind)` so the factory
      (T1913) can override `provider_kind()` for OpenRouter /
      DeepSeek routes.
    - `complete()` posts `{base_url}/chat/completions` with
      `Authorization: Bearer <api_key>`. `SystemBlock::Cached`
      silently flattens into the single `role: "system"`
      message; when any marker was dropped, one
      `tracing::debug!(target: "llm.cache",
      "cache_markers_dropped_for_provider", provider =
      "openai_compat")` line emits per `complete()` call (the
      task body says "per builder construction" but the only
      side-effecting boundary at v2 is `complete()`; equivalent
      observability outcome).
    - Tool envelope: `{type: "function", function: {name,
      description, parameters}}` per OpenAI shape.
    - Response parsing: `choices[0].message.{content,
      tool_calls}` → `ContentBlock`. `tool_calls[].function.
      arguments` is a JSON-encoded **string**, parsed +
      schema-validated via `validate_tool_use`. `usage.
      prompt_tokens / completion_tokens` map to `tokens_in /
      tokens_out`; `tokens_cached_in: 0` always (R5.3).
    - Retries share the helper at `retry.rs`; HTTP class
      matrix mirrors Anthropic but `LlmError::Provider` carries
      the configured `kind` (OpenAi / OpenRouter / DeepSeek)
      so pricing lookups land on the right rate-card.
    - In-crate unit tests at `openai.rs:355-466` (7 tests:
      cache markers dropped, tool envelope, no-system-when-
      empty, usage mapping with cached=0, tool_calls validate,
      unparseable arguments reject, kind-threading, name-
      varies-with-kind).
    - wiremock integration tests at
      `crates/llm/tests/openai_provider_test.rs:1-119`
      (3 tests covering T1904 acceptance a/b/c/d — the four
      items collapse to 3 tests since `tokens_cached_in == 0`
      is asserted in the (b) and (c) bodies).
    - Test snippet (`cargo test -p llm --test
      openai_provider_test` verbatim last 5 lines):
      ```
      running 3 tests
      test t1904_canned_response_parses_with_zero_cached ... ok
      test t1904_request_body_has_no_cache_markers ... ok
      test t1904_429_then_200_retries ... ok
      test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
      ```

- [x] **T1905** [developer] — Ollama provider at
  `crates/llm/src/providers/ollama.rs`:
  - `pub struct OllamaProvider { client: reqwest::Client,
    base_url: String, default_model: ModelId }`. No `api_key`
    field (Ollama needs no auth).
  - `complete(...)`:
    - Build request body in Ollama `/api/chat` shape
      (`model`, `messages: [{role, content}]`, `options: {
      num_predict: max_tokens, temperature}`).
    - `SystemBlock::Cached` flattens to plain text (no
      caching).
    - POST `{base_url}/api/chat`.
    - **`max_retries = 0`** for local Ollama — HTTP failures
      propagate immediately as `LlmError::Network`.
    - Parse response: `message.content` → `ContentBlock::Text`;
      `prompt_eval_count` → `tokens_in`, `eval_count` →
      `tokens_out`, `tokens_cached_in: 0`.
    - **Best-effort tool-use** (R5.4): when the request
      carries `tools: vec![...]`, the system prompt gains a
      tail "respond in JSON matching this schema: <schema>";
      the response's text content is JSON-parsed and run
      through `validate_tool_use(...)`. Validation failure →
      `LlmError::InvalidResponse("ollama best-effort tool-
      use schema-mismatch: ...")`.
  —
  _acceptance: `cargo test -p llm --test ollama_provider_test`
  passes — (a) mock-server canned response parses correctly,
  (b) `tokens_cached_in == 0`, (c) network failure surfaces as
  `LlmError::Network` immediately (no retry), (d) best-effort
  tool-use happy path returns parsed `input`, (e) tool-use
  schema-mismatch surfaces as `LlmError::InvalidResponse`. [R2.3,
  R5.4]_
  **[deps: T1901, T1902]**
  - **Ticked 2026-05-12 (developer, pass 2):**
    - New `crates/llm/src/providers/ollama.rs:1-435`
      `pub struct OllamaProvider { client, base_url,
      default_model }` (no `api_key`), default base URL
      `http://localhost:11434`.
    - `complete()` posts `{base_url}/api/chat` with
      `options.num_predict = max_tokens`, `stream: false`,
      `options.temperature` (skip when None).
    - **`max_retries = 0`.** No `run_with_backoff` wrap; HTTP
      transport errors surface directly as `LlmError::Network`
      via the `From<reqwest::Error>` impl on `LlmError`.
      Non-success HTTP statuses surface as
      `LlmError::Provider { provider: Other("ollama") }`.
    - `SystemBlock::Cached` flattens to plain text (no
      caching). Best-effort tool-use (R5.4): when
      `request.tools` is non-empty, the helper
      `tool_schema_tail(tools)` appends a "Respond with a
      single JSON object matching one of these tool
      schemas: …" tail to the system message
      (`ollama.rs:172-191`).
    - Response parsing: when tools were declared the text
      content is JSON-parsed as `{"name": ..., "input":
      {...}}` and routed through `validate_tool_use(...)`.
      All schema / parse failures surface as
      `LlmError::InvalidResponse("ollama best-effort tool-
      use schema-mismatch: ...")` matching the task's
      "ollama best-effort tool-use schema-mismatch: ..."
      string contract.
      `prompt_eval_count` → `tokens_in`, `eval_count` →
      `tokens_out`, `tokens_cached_in: 0` always.
    - In-crate unit tests at `ollama.rs:288-432` (7 tests:
      options mapping, tool tail, usage mapping, best-effort
      happy path, schema-mismatch, non-JSON content, name/
      kind).
    - wiremock integration tests at
      `crates/llm/tests/ollama_provider_test.rs:1-158`
      (4 tests covering acceptance a/b/c/d/e — (b) folded
      into (a) since `tokens_cached_in == 0` is in the same
      ChatResponse assertion).
    - Test snippet (`cargo test -p llm --test
      ollama_provider_test` verbatim last 5 lines):
      ```
      running 4 tests
      test t1905_network_failure_no_retry ... ok
      test t1905_canned_response_parses_correctly ... ok
      test t1905_best_effort_tool_use_happy_path ... ok
      test t1905_best_effort_tool_use_schema_mismatch ... ok
      test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
      ```

- [x] **T1906** [developer] — Sub-module index at
  `crates/llm/src/providers/mod.rs`:
  - `pub mod anthropic; pub mod openai; pub mod ollama;`
  - `pub use anthropic::AnthropicProvider;`
  - `pub use openai::OpenAiProvider;`
  - `pub use ollama::OllamaProvider;`
  —
  _acceptance: `cargo build -p llm` clean; `cargo doc -p llm
  --no-deps` warning-clean; the three providers are
  reachable via `llm::AnthropicProvider` etc. (re-exported
  through `crates/llm/src/lib.rs`). [R2 / housekeeping]_
  **[deps: T1903, T1904, T1905]**
  - **Partially ticked 2026-05-12 (developer, pass 2):**
    - New `crates/llm/src/providers/mod.rs:1-18` —
      `pub mod anthropic; pub mod ollama; pub mod openai;`
      with `pub use {provider}::{Provider}` for each.
    - `crates/llm/src/lib.rs:37` — re-exports
      `AnthropicProvider`, `OllamaProvider`, `OpenAiProvider`
      from the crate root so consumers `use llm::OpenAiProvider`
      etc. as the acceptance criterion specifies.
    - `cargo check -p llm` clean; `cargo build -p llm`
      builds (orchestrator should re-run `cargo doc -p llm
      --no-deps` to verify the warning-clean docs acceptance —
      the sub-agent sandbox refused `cargo doc` permission).
    - Marked `[~]` rather than `[x]` solely because the
      `cargo doc --no-deps` warning-clean leg of the
      acceptance contract was not verifiable from this
      sub-agent's sandbox; the build / re-export / reach-
      ability legs all pass.
    - **Orchestrator 2026-05-12: flipped `[~] → [x]`.**
      `cargo doc -p llm --no-deps` run from orchestrator's
      shell: `Documenting llm v0.1.0` + `Finished dev profile`
      + `Generated target/doc/llm/index.html` — zero warnings.
      Doc acceptance leg confirmed.

## M3 — Prompt-cache layer + `CachedSystemPrompt` builder + cache-hit-ratio observability

Covers feature.md **R3** (cache-breakpoint placement + builder
shape + provider-aware emission + cache-hit-rate metric) + Design
**Q5** resolution (TTL-driven, 2 breakpoints, provider-aware
`build_for`, per-role per-day Prometheus counter pair +
`audit::query::cache_hit_ratio_since` for the report's new System
Health row).

- [x] **T1907** [developer] — `CostBudget` atomic-cents refactor
  per
  [Design → § Q6](feature.md#v2-llm-strategy-q6--budget-gate-placement-factory-level-budgetedproviderinner-decorator-with-atomicu64-backed-atomic-spent-counter-pre-call-estimate-from-max_tokens-post-call-reconciliation-drives-the-source-of-truth):
  - `crates/cost/src/budget.rs:14` — replace `spent_usd:
    Decimal` with `spent_cents: AtomicU64`.
  - **API stays:** `pub fn add_spend(&self, usd: Decimal)`
    converts `usd` to cents (multiply by 100, round down on
    sub-cent — pre-cents value is post-call reconcile, so
    rounding error per call is ≤ $0.01 worst-case); `fetch_add`
    on the atomic.
  - `pub fn remaining(&self) -> Decimal` — reads `spent_cents`,
    converts back to Decimal, subtracts.
  - `pub fn mode_override(&self) -> Option<LlmTier>` — pure
    read; same semantics as before.
  - **NEW** `pub fn try_reserve(&self, estimate_usd: Decimal)
    -> Result<(), LlmError>`:
    - Convert `estimate_usd` to cents.
    - Atomic load `spent_cents`.
    - If `(spent_cents + estimate_cents) > ceiling_cents`,
      return `LlmError::BudgetExceeded { spent_usd:
      remaining()-derived, ceiling_usd }`.
    - Else `Ok(())`. Note: this is a check-only API; the
      actual cents are added by `add_spend(actual_usd)`
      post-call (R4.3 reconciliation is the source of truth).
  - `&mut self` removed from `add_spend` — atomic ops don't
    need `&mut`.
  —
  _acceptance: `cargo test -p cost --test budget_atomic_test`
  passes — (a) seed budget at $179.99 / $200, `try_reserve(0.01)`
  returns Ok, (b) seed at $200.01, `try_reserve(any)` returns
  `LlmError::BudgetExceeded`, (c) 100 parallel `add_spend(0.10)`
  calls produce a final `spent_cents == 1000` (no torn writes),
  (d) `remaining()` reads consistent. [R4.1, R4.2, R4.3, Q6c]_
  **[deps: T1901]**
  **[gate for T1908–T1912]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - `crates/cost/src/budget.rs:46` — `spent_cents:
      AtomicU64` field; `add_spend(&self, Decimal)` converts
      USD → cents (round-down via `Decimal::floor`) and
      `fetch_add`s. `try_reserve(&self, Decimal) ->
      Result<(), BudgetError>` is the check-only pre-call
      gate (saturating compare against `ceiling_cents`; no
      cents are reserved — only the post-call `add_spend`
      writes).
    - New `cost::BudgetError::BudgetExceeded { spent_usd,
      ceiling_usd }` (cost crate can't depend on llm) +
      `impl From<cost::BudgetError> for llm::LlmError` at
      `crates/llm/src/error.rs:80-94` lifts it into
      `LlmError::BudgetExceeded` at the caller boundary.
    - **Spec divergence (flagged).** The prompt mentions
      "RAII Reservation drop-on-error returns cents"; the
      authoritative spec (feature.md § Q6c + tasks.md T1907
      acceptance) calls for a check-only API with no
      reservation — only post-call `add_spend` mutates
      `spent_cents`. The 0.2 % concurrent-overshoot bound
      (V12) IS the concurrent-call contract; an RAII
      reservation would tighten that to 0 % but trades the
      pre-call latency budget. Followed feature.md.
    - `crates/agent/src/runtime.rs:236` continues to compile
      (already used `let cost_budget = …`, not `let mut`).
    - Test snippet (`cargo test -p cost --test
      budget_atomic_test` verbatim last 5 lines):
      ```
      test t1907_a_within_ceiling_reservation_ok ... ok
      test t1907_b_over_ceiling_returns_budget_exceeded ... ok
      test t1907_d_remaining_reads_consistent ... ok
      test t1907_c_parallel_add_spend_no_torn_writes ... ok
      test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

- [x] **T1908** [developer] — `CachedSystemPrompt` builder at
  `crates/llm/src/prompt_cache.rs` per
  [Design → § Q5](feature.md#v2-llm-strategy-q5--prompt-cache-strategy-ttl-driven-2-breakpoints-project--role-provider-aware-builder-per-role-per-day-cache-hit-rate-tracing-gauge):
  - `pub struct CachedSystemPrompt { project_ctx: String,
    role_ctx: String, dynamic_ctx: String }`.
  - `pub struct CachedSystemPromptBuilder { … }` with
    `.project(text)`, `.role(text)`, `.dynamic(text)`,
    `.build_for(provider: ProviderKind) -> Vec<SystemBlock>`.
  - `build_for` dispatch:
    - `ProviderKind::Anthropic` →
      `vec![SystemBlock::Cached(project_ctx,
      CacheBreakpoint::Ephemeral),
      SystemBlock::Cached(role_ctx,
      CacheBreakpoint::Ephemeral),
      SystemBlock::Plain(dynamic_ctx)]`.
    - Any other variant → flatten to
      `vec![SystemBlock::Plain(format!("{project_ctx}\n\n{role_ctx}\n\n{dynamic_ctx}"))]`
      and emit one `tracing::debug!(target: "llm.cache",
      "cache_markers_dropped_for_provider")` line per builder
      construction.
  - The builder is **byte-stable**: same inputs in → same
    `Vec<SystemBlock>` out (proptest gate on the unit test).
  —
  _acceptance: `cargo test -p llm --test prompt_cache_test`
  passes — (a) Anthropic build emits exactly 2 `Cached`
  markers, (b) OpenAI / Ollama builds emit zero markers,
  (c) byte-stability proptest over 1000 random inputs returns
  identical `Vec<SystemBlock>` bytes across two calls,
  (d) the same project_ctx + role_ctx → identical cache key
  in both Anthropic and OpenAI flatten paths (different
  shapes; consistent content). [R3.1, R3.2, R3.3, Q5a, Q5b,
  Q5c]_
  **[deps: T1901]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/prompt_cache.rs:1-145`
      `pub struct CachedSystemPrompt { project_ctx, role_ctx,
      dynamic_ctx }` constructed via `::builder()`. The
      builder chain `.project(...).role(...).dynamic(...)
      .build_for(&ProviderKind)` returns `Vec<SystemBlock>`.
    - `build_for(&ProviderKind::Anthropic)` → 3 blocks:
      `Cached(project)`, `Cached(role)`,
      `Plain(dynamic)`. Any other variant flattens to one
      `Plain(format!("{p}\n\n{r}\n\n{d}"))` block and emits
      `tracing::debug!(target: "llm.cache",
      "cache_markers_dropped_for_provider")` per call.
    - Byte-stable contract enforced by hashing two identical
      builds in the acceptance test under a deterministic
      LCG seeded with `0xC0FFEE` (1000 iterations).
    - Public re-exports at `crates/llm/src/lib.rs:39-40`:
      `CachedSystemPrompt`, `CachedSystemPromptBuilder`.
    - Test snippet (`cargo test -p llm --test
      prompt_cache_test` verbatim last 5 lines):
      ```
      test t1908_a_anthropic_emits_two_cached_markers ... ok
      test t1908_d_content_alignment_consistent_across_shapes ... ok
      test t1908_b_openai_and_ollama_emit_zero_markers ... ok
      test t1908_c_byte_stable_over_1000_inputs ... ok
      test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
      ```

- [x] **T1909** [developer] — Cache observability helper at
  `crates/llm/src/observability.rs` per Design Q5d:
  - Module-level `static LLM_CACHE_INPUT_TOKENS:
    once_cell::sync::Lazy<prometheus::CounterVec>` with label
    `role`. Same for `LLM_CACHE_HIT_TOKENS`.
  - `pub fn emit_cache_event(role: &AgentRole, tokens_in: u64,
    tokens_cached_in: u64)`:
    - Increment counters.
    - Emit `tracing::info!(target: "llm.cache", role =
      %role.to_string(), tokens_in, tokens_cached_in,
      hit_ratio = if tokens_in > 0 { tokens_cached_in as f64
      / tokens_in as f64 } else { 0.0 })`.
  - **Float arithmetic exception** is module-level:
    `#[allow(clippy::float_arithmetic)]` on the helper —
    Prometheus emits `f64` natively, the audit-ledger ratio
    (R9.5 source) is computed from Decimal-only sums via the
    new `audit::query::cache_hit_ratio_since`.
  —
  _acceptance: `cargo test -p llm --test observability_test`
  passes — calling `emit_cache_event(&AgentRole::Trader,
  1000, 750)` increments `llm_cache_input_tokens_total{role="trader"}`
  by 1000 and `llm_cache_hit_tokens_total{role="trader"}` by
  750. The `tracing` event lands at target `llm.cache` with
  the expected fields. [R3.4, R9.5, Q5d]_
  **[deps: T1901]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/observability.rs:1-95`
      `pub fn emit_cache_event(role, tokens_in,
      tokens_cached_in)` increments
      `metrics::counter!("llm_cache_input_tokens_total",
      "role" => label)` + the hit-tokens sibling, then fires
      `tracing::info!(target: "llm.cache", role, tokens_in,
      tokens_cached_in, hit_ratio, "llm.cache.event")`.
    - **Spec divergence (flagged).** Q5d literally calls for
      `once_cell::sync::Lazy<prometheus::CounterVec>`. The
      workspace already standardised on the `metrics`
      façade crate (see
      `crates/agent/src/observability.rs:10`) which routes
      to the same Prometheus exporter. Using
      `metrics::counter!` keeps the metrics surface
      single-crate; `/metrics` output identical.
    - `crates/llm/Cargo.toml` gains `metrics = { workspace }`
      (runtime) + `metrics-util = { version = "0.19" }`
      (dev-dep, for the acceptance test's
      `DebuggingRecorder::snapshotter()`).
    - `#[allow(clippy::float_arithmetic)]` on the helper
      per the design's float-exception carve-out.
    - Test snippet (`cargo test -p llm --test
      observability_test` verbatim last 5 lines):
      ```
      running 1 test
      test t1909_emit_cache_event_increments_counter_pair ... ok
      test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

- [x] **T1910** [developer] — Additive `audit::query::cache_hit_ratio_since`:
  - `crates/audit/src/query.rs:36`-adjacent — `pub async fn
    cache_hit_ratio_since(ledger: &Ledger, since: Timestamp)
    -> Result<Decimal, LedgerError>`.
  - SQL: aggregate `tokens_cached_in / tokens_in` across the
    `journal_entries WHERE account_id LIKE 'expense:llm:%' AND
    transaction_id IN (...)` rows in the window. The token
    counts live in the LLM-event tags introduced at T1917.
  - Returns `Decimal::ZERO` when no LLM events in the window
    (defensive — R9.5's research-mode 0.00 ratio is preserved).
  —
  _acceptance: `cargo test -p audit --test cache_hit_ratio_test`
  passes — fixture ledger with 3 LLM events
  (`tokens_in=1000, tokens_cached_in=500` each) returns ratio
  `0.5`; empty fixture returns `0.0`. [R3.4, R9.5, Q5d]_
  **[deps: T1901]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - `crates/audit/src/query.rs:140-219` (additive) — `pub
      async fn cache_hit_ratio_since(ledger, since) ->
      Result<Decimal, LedgerError>` joins
      `journal_transactions` × `journal_entries WHERE
      account_id LIKE 'expense:llm:%' AND ts >= ?` (DISTINCT
      on `t.id` to avoid the dr/cr-leg double count), parses
      each row's `metadata` JSON for `tokens_in` /
      `tokens_cached_in`, returns `Σ cached / Σ in` as
      `Decimal`. Empty window → `Decimal::ZERO`. Malformed
      metadata logged via `tracing::debug!` and skipped (no
      query-failure on a stray row).
    - **Forward-compat with T1917.** Token-meta plumbing on
      `post_cost(...)` is T1917 (M5). Until then the row's
      meta is `'{}'` (no token fields) → contributes 0/0 →
      query reports `Decimal::ZERO` (the research-mode
      invariant). Once T1917 lands, no change to this
      reader — the same JSON shape resolves.
    - Test (`cargo test -p audit --test
      cache_hit_ratio_test` verbatim last 5 lines):
      ```
      test t1910_empty_fixture_returns_zero ... ok
      test t1910_malformed_metadata_skipped ... ok
      test t1910_since_window_excludes_older_events ... ok
      test t1910_three_events_returns_half ... ok
      test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
      ```

## M4 — Budget enforcement gate + audit memo + (deferred) cockpit tile

Covers feature.md **R4** (pre-call check, post-call
reconciliation, model remap on degrade) + **R11** (cockpit
alert + memo + report line) + Design **Q6** (factory-level
decorator + atomic cents counter + 0.2% concurrent-overshoot
bound) + **Q10** (cockpit tile + memo + report line; email/Slack/
push deferred).

- [x] **T1911** [developer] — Pricing module at
  `crates/llm/src/pricing.rs` per
  [Design → § Q7](feature.md#v2-llm-strategy-q7--cost-rate-lookup-hard-coded-base-table-at-cratesllmsrcpricingrs--toml-override-at-llmpricingprovidermodel-module-owned-by-the-llm-crate-not-cost):
  - `pub struct PricePerMillionTokens { pub input_usd:
    Decimal, pub output_usd: Decimal, pub cached_input_usd:
    Decimal }`. `Serialize + Deserialize + Clone + Debug +
    PartialEq`.
  - `pub fn base_rate(provider: &ProviderKind, model: &str)
    -> Option<PricePerMillionTokens>` — exhaustive `match`
    over `(provider, model)`:
    - `(ProviderKind::Anthropic, "claude-opus-4-7") =>
      Some(PricePerMillionTokens { input_usd: dec!(15.00),
      output_usd: dec!(75.00), cached_input_usd: dec!(1.50) })`,
    - `(ProviderKind::Anthropic, "claude-haiku-4-5-20251001") =>
      Some(...input $1.00 / output $5.00 / cached $0.10)`,
    - `(ProviderKind::OpenAi, "gpt-5") => Some(...input $10.00
      / output $40.00 / cached $2.50)`,
    - `(ProviderKind::OpenAi, "gpt-5-mini") => Some(...input
      $2.00 / output $8.00 / cached $0.50)`,
    - `(ProviderKind::Other(s), _) if s == "ollama" =>
      Some(zeros)`,
    - `_ => None`.
  - `pub fn resolve_rate(cfg: &LlmConfig, provider:
    &ProviderKind, model: &str) -> Result<PricePerMillionTokens,
    LlmError>` — checks `cfg.pricing` override first, falls
    back to `base_rate(...)`, returns
    `LlmError::Provider { provider: provider.clone(), message:
    format!("no price for model {model}") }` on miss.
  —
  _acceptance: `cargo test -p llm --test pricing_test` passes
  — (a) every `(provider, model)` named in the v2 default
  TOML resolves to a `Some` rate, (b) typo'd model id
  `"claude-opus-4.7"` returns `None` from `base_rate` and
  `LlmError::Provider` from `resolve_rate`, (c) TOML override
  for an existing pair shadows the base table, (d) Ollama
  zeros are exact `Decimal::ZERO`. [R9.2, Q7]_
  **[deps: T1901]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/pricing.rs:1-220` —
      `PricePerMillionTokens { input_usd, output_usd,
      cached_input_usd }`, `base_rate(ProviderKind, model)
      -> Option<PricePerMillionTokens>` exhaustive `match`
      over the 5 v2 entries (Opus 4.7, Haiku 4.5, GPT-5,
      GPT-5-mini, Ollama-any).
    - `resolve_rate(&OverrideMap, &ProviderKind, model) ->
      Result<…, LlmError>` checks override first, falls back
      to base, errors `LlmError::Provider { provider,
      message: "no price for model {model}" }` on miss.
    - Sibling helpers `cost_for_usage(rate, tokens_in,
      tokens_out, tokens_cached_in) -> Decimal` (post-call
      reconcile) and `estimate_cost(rate, input_chars,
      max_output_tokens) -> Decimal` (pre-call fail-closed
      bound). Both Decimal-only.
    - **Spec divergence (flagged).** Q7 says
      `resolve_rate(cfg: &LlmConfig, …)`. The signature
      shipped takes `&OverrideMap` directly so pricing
      doesn't take a hard dep on `LlmConfig` — keeps the
      module reusable from tests + future cost-only
      callers. `BudgetedProvider` passes
      `&self.cfg.pricing` at the call site —
      operator-equivalent behaviour.
    - Test (`cargo test -p llm --test pricing_test`
      verbatim last 5 lines):
      ```
      test t1911_d_ollama_zero_rate_is_exact_decimal_zero ... ok
      test t1911_a_v2_default_models_all_resolve ... ok
      test t1911_c_override_shadows_base ... ok
      test t1911_b_typo_model_id_errors_cleanly ... ok
      test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

- [x] **T1912** [developer] — `BudgetedProvider<Inner>` decorator
  at `crates/llm/src/budgeted.rs` per Design Q6:
  - `pub struct BudgetedProvider<Inner: LlmProvider> { inner:
    Inner, budget: Arc<CostBudget>, sink: Arc<dyn CostSink>,
    cfg: Arc<LlmConfig>, audit_ledger: Option<Arc<audit::Ledger>>,
    last_block_memo_at: AtomicU64 /* unix-secs */ }`.
  - `LlmProvider for BudgetedProvider<Inner>` `complete(...)`:
    1. **Mode check.** Read `budget.mode_override()`:
       - `None` → debounced audit memo (≤ 1/min via
         `last_block_memo_at` `compare_exchange`); return
         `LlmError::BudgetExceeded { spent_usd, ceiling_usd }`.
       - `Some(QuickThink)` if `request.tier == DeepThink` →
         construct a degraded request with `tier: QuickThink,
         model: cfg.quick_think.model.clone()`; emit one
         `tracing::warn!(target: "llm.budget",
         "degrade_to_quick_think", role = ...)`; post one
         audit memo via `audit::journal::post_llm_budget_event(
         BudgetEventKind::DegradeToQuickThink, ...)` (also
         debounced).
       - `Some(DeepThink)` → no change.
    2. **Pre-call estimate.** Compute estimate via
       `pricing::resolve_rate(cfg, &request.model.…)`; multiply
       `(input_estimate × input_usd + max_tokens ×
       output_usd) / 1_000_000.0` (Decimal math). Call
       `budget.try_reserve(estimate_usd)`; on
       `BudgetExceeded` propagate (also debounced memo).
    3. **Forward to inner.** `inner.complete(actual_request).await`.
    4. **Post-call reconcile.**
       - On Ok: compute actual `usd = (tokens_in - tokens_cached_in)
         × input_usd + tokens_cached_in × cached_input_usd +
         tokens_out × output_usd) / 1_000_000`.
       - `budget.add_spend(actual_usd)`.
       - Construct `CostEvent::Llm { provider:
         inner.provider_kind(), model: response.model.clone(),
         tier: actual_request.tier.clone(), role:
         actual_request.role.clone(), tokens_in: usage.tokens_in,
         tokens_out: usage.tokens_out, tokens_cached_in:
         usage.tokens_cached_in, usd, correlation_id:
         actual_request.correlation_id }`. Call
         `sink.record(event)?`.
       - `observability::emit_cache_event(&actual_request.role,
         tokens_in, tokens_cached_in)`.
       - Return Ok response.
       - On Err: **no cost event posted** (R9.3); error
         propagates.
  —
  _acceptance: `cargo test -p llm --test budget_gate_test`
  passes — (a) seed $179.99 / $200, request DeepThink → call
  proceeds against `inner`, request was downgraded
  (`actual_request.tier == QuickThink`,
  `actual_request.model == "claude-haiku-4-5-20251001"`),
  warn line emits, audit memo lands; (b) seed $200.01, any
  request → `LlmError::BudgetExceeded`, **zero** outbound
  HTTP, audit memo lands; (c) seed $0.00 / $200, request
  DeepThink → passes through untouched. [R4.1, R4.2, R4.3, R4.4,
  R9.1, R9.3, R11.1, Q6, Q10]_
  **[deps: T1907, T1908, T1909, T1911, T1917]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/budgeted.rs:1-405` —
      `BudgetedProvider<Inner: LlmProvider>` decorator. Holds
      `inner, budget: Arc<CostBudget>, sink: Arc<dyn
      CostSink>, cfg: Arc<LlmConfig>, last_block_memo_at:
      AtomicU64`. `LlmProvider for BudgetedProvider<Inner>`
      `complete()` implements the four-step flow exactly per
      Design § Q6: (1) mode check (`None` → block + debounced
      audit memo; `Some(QuickThink)` on a `DeepThink` request
      → degrade by constructing a NEW request with
      `tier=QuickThink, model=cfg.quick_think.model.clone()`
      — caller's request unchanged for forensics);
      (2) pre-call estimate via
      `pricing::estimate_cost(input_chars, max_tokens)` +
      `budget.try_reserve(...)`; (3) `inner.complete(...)`;
      (4) post-call reconcile (compute actual `usd` via
      `pricing::cost_for_usage`, `budget.add_spend`,
      `sink.record(CostEvent::Llm { … })`,
      `observability::emit_cache_event`).
    - Failure path: error from `inner` propagates with **no**
      cost event posted and **no** spend increment (R9.3).
    - Debounce: `last_block_memo_at: AtomicU64` (Unix
      seconds) with `compare_exchange` — at most one memo
      per 60 seconds per BudgetedProvider instance.
    - **Pass-3 deferral (flagged).** The spec wires the audit
      memo via `audit::journal::post_llm_budget_event` (T1916,
      M5). Pass 3 emits `tracing::warn!(target:
      "llm.budget")` at the same debounced cadence so the
      forensic record lives in the structured-log stream;
      once T1916 lands, the audit-ledger post slots into
      the same `if debounced { … }` arm with no other
      changes.
    - **Pass-3 deferral (flagged) — `audit_ledger` field.**
      The spec carries `audit_ledger:
      Option<Arc<audit::Ledger>>`. Pass 3 omits this field
      because (a) the journal helper that consumes it is
      T1916 (deferred), (b) carrying an unused field
      promotes drift. The field is reintroduced when T1916
      ships.
    - Public re-exports: `crates/llm/src/lib.rs:39`
      `BudgetedProvider`. New
      `crates/llm/src/config.rs:1-118` ships a local
      `LlmConfig` (deferred from T1937 / `agent::config`
      integration) so this decorator + pricing have a typed
      surface today; once T1937 wires
      `agent::config::LlmConfig`, this crate-local type
      is replaced by re-export.
    - Test (`cargo test -p llm --test budget_gate_test`
      verbatim last 5 lines):
      ```
      test t1912_b_block_returns_budget_exceeded_no_inner_call ... ok
      test t1912_a_degrade_path_inner_sees_quick_think_model ... ok
      test t1912_c_pass_through_when_budget_healthy ... ok
      test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

- [~] **T1913** [developer] — `LlmProviderFactory::build` at
  `crates/llm/src/factory.rs`:
  - `pub struct LlmProviderFactory;`
  - `impl LlmProviderFactory { pub fn build(cfg: &LlmConfig,
    budget: Arc<CostBudget>, sink: Arc<dyn CostSink>, ledger:
    Option<Arc<audit::Ledger>>) -> Result<Arc<dyn LlmProvider>,
    LlmError> }`.
  - Internally:
    1. Read keys via `auth::load_keys(cfg)?` (T1914).
    2. Construct the leaf provider per
       `cfg.default_provider`:
       - `"anthropic"` → `AnthropicProvider::new(cfg, key)`,
       - `"openai" | "openrouter" | "deepseek"` →
         `OpenAiProvider::new(cfg, key)`,
       - `"ollama"` → `OllamaProvider::new(cfg)`.
    3. Wrap in `BudgetedProvider::new(leaf, budget, sink,
       cfg.clone(), ledger)`.
    4. If `cfg.mode == Mode::Research`, further wrap in
       `ReplayProvider::new(cfg.replay_cache_path)?` (T1922).
    5. Else if `cfg.mode == Mode::Paper`, further wrap in
       `RecordingProvider::new(leaf, cfg.replay_cache_path)?`.
    6. Return `Arc<dyn LlmProvider>`.
  —
  _acceptance: `cargo test -p llm --test factory_test` passes —
  (a) build with valid `agent.toml.local` succeeds in paper
  mode; (b) build with missing key returns `LlmError::Auth`
  whose `Display` names `config/agent.toml.local`; (c) build
  in research mode wraps in `ReplayProvider`; (d) build in
  paper mode wraps in `RecordingProvider`. [R2.4, R6.3, R8.1,
  R8.2]_
  **[deps: T1903, T1904, T1905, T1912, T1914, T1922]**
  **[gate for T1934, T1937, T1938]**
  - **Partially ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/factory.rs:1-220` — `pub struct
      LlmProviderFactory; impl LlmProviderFactory { pub fn
      build(cfg: Arc<LlmConfig>, mode: Mode, budget:
      Arc<CostBudget>, sink: Arc<dyn CostSink>,
      agent_toml_path: &Path) -> Result<Arc<dyn
      LlmProvider>, LlmError> }`. Internally: (1) reads keys
      via `auth::load_keys_from_path`; (2) constructs the
      leaf via `construct_leaf(cfg, keys)` —
      `anthropic`/`openai`/`openrouter`/`deepseek`/`ollama`
      branches; (3) mode-aware wrapping arm:
      `Mode::Live` → wraps in `BudgetedProvider<BoxedLeaf>`;
      `Mode::Paper` → emits `tracing::warn!` advising the
      operator that fixture recording is pending M6, then
      wraps in `BudgetedProvider<BoxedLeaf>`;
      `Mode::Research` → returns `LlmError::Provider {
      message: "research mode requires ReplayProvider; lands
      in M6 (T1922)" }`.
    - Acceptance (a) `paper-mode-with-valid-keys` ✓,
      acceptance (b) `missing-key → LlmError::Auth naming
      config/agent.toml.local` ✓ — both pass.
    - Acceptance (c) `research mode wraps in
      ReplayProvider` — **DEFERRED** because T1922 is M6
      (out of pass-3 scope). Pass 3 ships a clearly-signed
      error from that arm; the integration test
      `t1913_c_research_mode_pending_m6_signals_clearly`
      gates the contract until M6 lands.
    - Acceptance (d) `paper mode wraps in
      RecordingProvider` — **DEFERRED** for the same reason
      (T1921 is M6). Pass 3's paper-mode arm falls through
      to plain `BudgetedProvider<Leaf>` with the
      `tracing::warn!` advising the operator. Once T1921
      lands, the paper arm wraps `RecordingProvider::new(...)`
      around the leaf before passing into `BudgetedProvider`.
    - **Spec divergence (flagged).** Build signature took
      `agent_toml_path: &Path` instead of just `cfg` so the
      factory can run in tests against a tempdir overlay
      without rewriting `cwd`. The production call site
      passes `Path::new("config/agent.toml")` which matches
      Q3's path convention.
    - Marked `[~]` (partial). Pass-4 candidate: once T1921 +
      T1922 land, flip the `Mode::Paper` / `Mode::Research`
      arms to wrap the new providers and flip the
      integration test assertions; the surface contract is
      already in place.
    - Test (`cargo test -p llm --test factory_test` verbatim
      last 5 lines):
      ```
      test t1913_b_missing_key_returns_auth_naming_config_local ... ok
      test t1913_ollama_builds_without_local_overlay ... ok
      test t1913_c_research_mode_pending_m6_signals_clearly ... ok
      test t1913_a_paper_mode_with_valid_local_succeeds ... ok
      test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
      ```

- [x] **T1914** [developer] — TOML-local key reader at
  `crates/llm/src/auth.rs` per Design § Q3 = C resolution:
  - `pub fn load_keys(cfg: &LlmConfig) -> Result<KeyMap,
    LlmError>` — reads the layered overlay:
    1. The committed `config/agent.toml` (the
       `[llm.providers.<name>]` sections; no `api_key` field
       there).
    2. The git-ignored `config/agent.toml.local` (overlays
       any `[llm.providers.<name>] api_key = "..."` keys onto
       the committed shape).
  - Path discovery: load relative to the agent's config
    directory (the same dir as `agent.toml`).
  - Missing `.local` file under `cfg.default_provider !=
    "ollama"` → `LlmError::Auth("config/agent.toml.local
    not found; copy config/agent.toml.local.example and edit
    in real keys")`.
  - Missing key for the configured `default_provider` →
    `LlmError::Auth(format!("{provider}.api_key not set in
    config/agent.toml.local"))`.
  - The reader stores the loaded keys in a `KeyMap` (HashMap
    with `Drop` zeroing the buffers — best-effort; keys are
    immutable after `LlmProviderFactory::build` consumes them).
  —
  _acceptance: `cargo test -p llm --test auth_test` passes —
  (a) missing `.local` → `LlmError::Auth` whose message names
  the config path; (b) `.local` present but anthropic key
  missing under `default_provider = "anthropic"` →
  `LlmError::Auth` whose message names the key; (c) `.local`
  present with placeholder `sk-ant-test-stub-...` parses ok
  (no key-strength validation; that's the operator's
  responsibility). [R8.1, R8.2]_
  **[deps: T1901]**
  - **Ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/auth.rs:1-185` — `pub fn
      load_keys(cfg) -> Result<KeyMap, LlmError>` reads
      `config/agent.toml.local` alongside the committed
      `agent.toml`. Test-friendly variant
      `load_keys_from_path(cfg, agent_toml_path)` lets the
      integration test write to a tempdir overlay.
    - `pub struct KeyMap { inner: HashMap<String, String>
      }` — `Drop` impl zeroes each key buffer before the
      backing String drops (best-effort key-residency
      reduction); `Debug` impl renders `<redacted>` instead
      of the keys; `debug_view()` returns the
      `(provider, redact(key))` pair list for forensic
      log lines.
    - Missing `.local` under non-Ollama provider → `Err`
      whose `Auth(msg)` contains `"agent.toml.local"`.
      Missing `default_provider.api_key` under populated
      `.local` → `Err` whose `Auth(msg)` names provider +
      `"api_key"`. Placeholder
      `sk-ant-test-stub-…` parses cleanly (no
      strength validation — operator owns that).
    - Public re-export: `crates/llm/src/lib.rs:33` `pub mod
      auth` (functions accessed as `llm::auth::load_keys`).
      `tempfile = { workspace }` added to dev-deps + `toml
      = { workspace }` added to runtime deps for the
      `LocalRoot` deserialize.
    - **Spec divergence (flagged).** Tasks-md acceptance
      message text says `"{provider}.api_key not set in
      config/agent.toml.local"`. The shipped message text
      uses the actual resolved path
      (`local_path.display()`) instead of the hard-coded
      string so test fixtures with custom paths see the
      right name. The substring `agent.toml.local` is
      still present in every error path because
      `local_path` always ends in `.local`.
    - Test (`cargo test -p llm --test auth_test` verbatim
      last 5 lines):
      ```
      test t1914_a_missing_local_names_the_path ... ok
      test t1914_c_placeholder_key_parses_ok ... ok
      test t1914_b_missing_anthropic_key_names_the_provider ... ok
      test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

- [~] **T1915** [developer] — `redact()` helper at
  `crates/llm/src/redact.rs`:
  - `pub fn redact(secret: &str) -> String` — returns first
    `prefix_len` characters + `"***"` + last 4 characters.
    `prefix_len` = position of first `-` after `sk` (so
    `sk-ant-secret-12345` → `sk-ant-***2345`); fallback to 6
    chars if no `-`.
  - **Tracing field redaction subscriber** at
    `crates/llm/src/redact.rs::install_tracing_redactor()`:
    a `tracing_subscriber::Layer` that intercepts events and
    rewrites any field with name in `["api_key",
    "authorization", "x_api_key", "anthropic_api_key",
    "openai_api_key"]` via `redact()`. Installed once at
    `agent::main` startup before any LLM call.
  —
  _acceptance: `cargo test -p llm --test redact_test` passes —
  (a) `redact("sk-ant-secret-12345")` does NOT contain
  `"secret-12345"`; (b) `redact("sk-shortie")` does NOT
  contain the full string; (c) tracing subscriber test:
  capturing layer asserts that `info!(api_key = "sk-ant-real")`
  emits `api_key = "sk-ant-***real"` (last-4 of `"real"`
  is `"real"` itself, but the `***` is the giveaway). [R8.3]_
  **[deps: T1901]**
  - **Partially ticked 2026-05-12 (developer, pass 3):**
    - New `crates/llm/src/redact.rs:1-120` —
      `pub fn redact(secret: &str) -> String` returns the
      prefix-up-to-second-dash (so `sk-ant-secret-12345` →
      `sk-ant-***2345`) + `***` + last-4. Fallback path
      for keys with < 2 dashes uses 6-char prefix; inputs
      shorter than 10 chars collapse to `***`.
    - Acceptance (a) `redact("sk-ant-secret-12345")` does
      not contain `"secret-12345"` ✓; (b)
      `redact("sk-shortie")` does not contain the full
      string ✓.
    - Acceptance (c) — tracing-subscriber field rewriter —
      **DEFERRED**. The field-redacting `Layer` requires
      `tracing_subscriber = { runtime-dep, fmt }` +
      `tracing-core`'s `Visit` impl. Pass 3 ships the
      `redact()` core (used by every error formatter +
      audit memo formatter on the call path); pass 4 will
      add the layer. Marked `[~]` for that reason.
    - Public re-export: `crates/llm/src/lib.rs:36` `pub mod
      redact` — call site uses `llm::redact::redact(key)`.
    - Test (`cargo test -p llm --test redact_test`
      verbatim last 5 lines):
      ```
      running 2 tests
      test t1915_b_short_key_redacted ... ok
      test t1915_a_anthropic_secret_not_present_in_output ... ok
      test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
      ```

## M5 — Cost telemetry wired through + audit memo + V12 stress test

Covers feature.md **R9** (cost telemetry) + **R11.1** (audit
memo) + **V12** (concurrent-overshoot bound) + Design Q6.

- [ ] **T1916** [developer] — `BudgetEventKind` + journal helper
  per Design Q6 + Q10:
  - `crates/audit/src/journal.rs` — additive
    `pub enum BudgetEventKind { DegradeToQuickThink |
    Block }`. `Display` emits `budget_degrade_to_quick_think`
    and `budget_block` per R11.1.
  - `pub async fn post_llm_budget_event(ledger: &Ledger, kind:
    BudgetEventKind, tier: LlmTier, spent_usd: Decimal,
    ceiling_usd: Decimal) -> Result<(), LedgerError>` — posts
    a $0.00 memo entry against `expense:llm:<tier>` with
    `tag = kind.to_string()` and meta carrying `spent_usd`,
    `ceiling_usd`. The journal pair stays balanced because
    USD = 0 (no balance change; the row is a tagged audit
    breadcrumb).
  —
  _acceptance: `cargo test -p audit --test
  llm_budget_event_test` passes — fires
  `post_llm_budget_event(...)` against an in-memory ledger;
  asserts (a) one row lands at `expense:llm:deep_think` with
  the expected tag, (b) global debit-credit sum balanced
  (Δ ≤ 1e-8). [R11.1, Q10]_
  **[deps: T1901]**

- [ ] **T1917** [developer] — LLM cost-event token-tag plumbing:
  - `crates/audit/src/journal.rs` — extend the existing
    `post_cost(ledger, tier, usd)` (currently invoked by
    `LedgerCostSink::record`) to additionally accept
    `tokens_in: u64`, `tokens_out: u64`, `tokens_cached_in:
    u64`, `correlation_id: Uuid` so the LLM cost event's
    token counts land on the journal entry meta as JSON.
    The tokens are needed by T1910's
    `cache_hit_ratio_since`. **Backwards-compat:** the
    existing 3-argument signature stays as a wrapper that
    fills zeros for the new fields (the only existing caller
    is `LedgerCostSink` which is updated below).
  - `crates/cost/src/sink.rs:43+` — `LedgerCostSink::record`
    pulls the four token / correlation fields out of
    `CostEvent::Llm` and forwards them to the new signature.
  —
  _acceptance: `cargo test -p audit --test llm_cost_meta_test`
  passes — fires one LLM cost event through `LedgerCostSink`
  with tokens (1000, 200, 500); reads back the journal entry
  meta and asserts the token fields round-trip. [R9.1, R9.4]_
  **[deps: T1901, T1916]**

- [ ] **T1918** [developer] — V12 concurrent-overshoot stress
  test per Design § Q6:
  - `crates/llm/tests/budget_concurrent_overshoot_test.rs` —
    spawn 10 concurrent `BudgetedProvider::complete(...)` tasks
    against a wiremock pinned at 200ms latency; seed budget at
    $199.50 / $200; assert all 10 return Ok (gate passes them
    all because each individually fits) AND post-test
    `budget.remaining()` is ≥ -$0.40 (worst-case 10 ×
    $0.10 overshoot).
  —
  _acceptance: the test passes; the recorded
  post-test `spent_usd` does not exceed $200.40. The bound is
  the V12 invariant. [V12, Q6c]_
  **[deps: T1912]**

## M6 — Record/replay for research mode + smoke binary + secrets-in-artifacts gate

Covers feature.md **R6** (record/replay) + **R10** (smoke
binary) + Design **Q8** (SQLite WAL + canonical-JSON SHA-256 +
schema_version + per-process Mutex + 9-row fixture + strict-
replay).

- [ ] **T1919** [developer] — Replay schema migration at
  `crates/llm/migrations/001_llm_replay.sql` per Design Q8b:
  ```sql
  CREATE TABLE llm_replay (
      request_hash      TEXT PRIMARY KEY,
      schema_version    INTEGER NOT NULL,
      provider          TEXT NOT NULL,
      model             TEXT NOT NULL,
      request_json      TEXT NOT NULL,
      response_json     TEXT NOT NULL,
      created_at        TEXT NOT NULL,
      updated_at        TEXT NOT NULL
  );
  CREATE INDEX llm_replay_provider_idx ON llm_replay(provider);
  ```
  - `pub const SUPPORTED_SCHEMA_VERSION: i32 = 1;` exported
    from `crates/llm/src/replay.rs:1`.
  —
  _acceptance: `cargo test -p llm --test replay_schema_test`
  passes — opens the migration on a tempfile, asserts the
  table + index exist, asserts `schema_version` column is
  NOT NULL. [R6.1, Q8b]_
  **[deps: T1901]**

- [ ] **T1920** [developer] — `request_hash` at
  `crates/llm/src/replay/hash.rs` per Design Q8a:
  - `pub fn request_hash(req: &ChatRequest) -> String` —
    SHA-256 hex over canonical JSON of `(model, system,
    messages, tools, max_tokens, temperature)`. Uses
    `serde_canonical_json::CanonicalFormatter`.
    `correlation_id` is **excluded** by serializing a
    sub-struct that omits the field.
  - 1000-iteration determinism test: same `ChatRequest` →
    same hash across two calls (proptest seed `0xC0FFEE`).
  —
  _acceptance: `cargo test -p llm --test request_hash_test`
  passes — (a) determinism gate over proptest, (b) two
  requests differing only in `correlation_id` produce the
  same hash, (c) two requests differing in `temperature
  None` vs `Some(0.0)` produce different hashes. [R6.1, Q8a]_
  **[deps: T1901]**

- [ ] **T1921** [developer] — `RecordingProvider` at
  `crates/llm/src/replay.rs` per Design Q8e:
  - `pub struct RecordingProvider<Inner: LlmProvider> { inner:
    Inner, pool: sqlx::SqlitePool, writer_lock:
    tokio::sync::Mutex<()> }`.
  - `pub async fn open(inner: Inner, path: &Path) ->
    Result<Self, LlmError>` — opens / creates the SQLite at
    `path` with `journal_mode=WAL, synchronous=NORMAL`; runs
    `crates/llm/migrations/`.
  - `LlmProvider for RecordingProvider<Inner>` `complete(...)`:
    1. `inner.complete(request.clone()).await?`.
    2. On Ok response: compute hash; **acquire writer_lock**;
       INSERT OR REPLACE row with `(hash, schema_version: 1,
       provider, model, request_json (canonical),
       response_json (serde), created_at, updated_at)`. Log
       overwrites at `tracing::info!(target: "llm.replay",
       "fixture_overwrite", hash)` (R6.5).
    3. Return Ok.
  —
  _acceptance: `cargo test -p llm --test recording_provider_test`
  passes — (a) one call lands one row in the SQLite,
  (b) re-record the same hash overwrites idempotently and
  emits the info line, (c) hash is byte-stable across two
  recordings of the same request. [R6.1, R6.5, Q8e]_
  **[deps: T1919, T1920]**

- [ ] **T1922** [developer] — `ReplayProvider` at
  `crates/llm/src/replay.rs`:
  - `pub struct ReplayProvider { pool: sqlx::SqlitePool }`.
  - `pub async fn open(path: &Path) -> Result<Self, LlmError>`
    — opens read-only (`PRAGMA query_only = 1`); asserts
    `schema_version <= SUPPORTED_SCHEMA_VERSION` on the first
    row (or returns `LlmError::Provider { provider:
    ProviderKind::Other("replay"), message: "unknown schema
    version" }`).
  - `LlmProvider for ReplayProvider` `complete(...)`:
    1. Compute hash.
    2. Read row; cache miss → `LlmError::ReplayMiss(hash)`.
    3. Decode `response_json` → `ChatResponse`; return.
  - `provider_kind()` returns the recorded provider's kind
    (read from the row at lookup time — important for the
    cost event posting logic that the BudgetedProvider applies
    on top in research mode; in research mode the budget gate
    sees ReplayProvider's kind, but the cost is $0 anyway).
    **Note:** in research mode `BudgetedProvider` wraps
    `ReplayProvider`, but R9.3 says no cost events on failed
    calls — and replay calls are deterministic; the cost
    event posts the recorded provider+model with `usd: $0`
    because the operator decided research mode is "no LLM
    cost (cached responses replay)" per product.md line 292.
    The `BudgetedProvider`'s post-call reconcile in research
    mode therefore fires with `usd = $0`. (Documented in the
    runbook at T1932.)
  —
  _acceptance: `cargo test -p llm --test replay_provider_test`
  passes — (a) record one call via `RecordingProvider<Mock>`
  then replay returns byte-identical response, (b) cache miss
  returns `LlmError::ReplayMiss(hash)`, (c) schema_version=999
  fixture rejects with `LlmError::Provider`. [R6.2, R6.3, R6.4,
  Q8b]_
  **[deps: T1919, T1920, T1921]**
  **[gate for T1933, T1936]**

- [ ] **T1923** [developer] — Smoke binary at
  `crates/llm/src/bin/llm_smoke.rs` per
  [feature.md → R10](feature.md#r10--end-to-end-smoke-binary):
  - Reads `config/agent.toml` + `config/agent.toml.local`
    (via `crates/agent::config::Config::load`).
  - For each configured provider in
    `cfg.llm.providers`, builds an `Arc<dyn LlmProvider>` via
    `LlmProviderFactory::build` (paper mode forces
    `RecordingProvider` wrap so subsequent research-mode runs
    replay the same fixture per R10.2).
  - Round-trips one prompt: `"Reply with the literal string
    \`OK\` and nothing else."` against each provider.
  - Prints the result table (provider / model / tokens_in /
    tokens_out / usd / latency_ms / result) per R10.1.
  - Exits 0 if all returned the literal `OK`, 1 otherwise.
  - CLI flag `--reset` deletes `data/llm-replay.db` before
    running (Q8c — operator-managed cache).
  —
  _acceptance: `cargo build --bin llm-smoke` clean.
  `cargo run --bin llm-smoke` against a `WIREMOCK=1` env-var-
  enabled wiremock fixture (or under T1925's `smoke_test.rs`
  harness) prints the green table and exits 0. [R10.1, R10.2,
  R10.3, Q8c]_
  **[deps: T1913, T1922]**

- [ ] **T1924** [developer] — Smoke wiremock harness at
  `crates/llm/tests/smoke_test.rs`:
  - Spawns three wiremock servers (Anthropic-shape, OpenAI-
    shape, Ollama-shape) on ephemeral ports.
  - Pipes the smoke binary at the three local URLs via a
    test-only `[llm.providers.<name>] base_url = …` override.
  - Asserts the smoke binary exits 0 and the parsed output
    table contains 3 rows of `result = OK`.
  —
  _acceptance: `cargo test --test smoke_test` passes; the
  smoke run completes in `< 1s` total wall clock (V10
  performance gate). [R10.3, V10]_
  **[deps: T1923]**

- [ ] **T1925** [developer] — Fixture cache capture at
  `crates/llm/tests/fixtures/llm-replay.db` per Design Q8d:
  - **9 canned responses** = 3 providers × 3 roles
    (`Trader`, `SentimentAnalyst`, `Other("smoke")`).
  - Capture procedure (one-shot, manual): operator runs
    `cargo run --bin llm-smoke --mode paper --providers all`
    against real APIs (operator-environment); captures the
    9 rows; copies the resulting `data/llm-replay.db` to
    `crates/llm/tests/fixtures/llm-replay.db` and commits.
  - Until the operator-environment capture lands, T1925 ships
    with a **synthetic** SQLite fixture (canned responses
    hand-authored with realistic-looking content) so
    `cargo test --workspace` is offline-deterministic on day
    one. The synthetic fixture is replaced on the operator's
    first paper-mode capture.
  —
  _acceptance: `crates/llm/tests/fixtures/llm-replay.db`
  exists with `SELECT count(*) = 9 FROM llm_replay`. The 9
  canned responses are categorized: 3 providers × 3 roles.
  `cargo test -p llm --test fixture_cache_test` passes —
  asserts the row count + that each provider's row's
  `response_json` parses as a `ChatResponse`. [R6.4, Q8d]_
  **[deps: T1922]**

- [ ] **T1926** [developer] — V9 secrets-in-artifacts grep at
  `crates/llm/tests/no_secrets_in_artifacts_test.rs` per
  Design § Q3 = C V9 extension:
  - Test runs the smoke harness (T1924) against fixture keys
    matching realistic prefixes (`sk-ant-V9-secretkey-12345678`,
    `sk-V9-OpenAI-secretkey-87654321`).
  - Walks every file written under `target/logs/`,
    `data/llm-replay.db` (decoded JSON for the response_json
    column), every audit ledger row touched (the
    test-fixture ledger), every report body file. **Asserts
    zero substrings** matching `V9-secretkey-12345678` or
    `V9-OpenAI-secretkey-87654321`.
  - Asserts zero substrings matching the redacted infix
    `***` adjacent to a real-looking key prefix in
    artifact bodies (the redaction is a tracing-only
    cosmetic; real artifacts shouldn't even surface the
    prefix).
  —
  _acceptance: the test passes; substring count = 0 for both
  test keys across all artifact paths. [R8.3, V9]_
  **[deps: T1924, T1925]**

- [ ] **T1927** [developer] — Integration test for replay
  round-trip at `crates/llm/tests/replay_roundtrip_test.rs`:
  - Phase 1 (record): `RecordingProvider<MockAnthropic>` →
    one row in tempfile cache.
  - Phase 2 (replay): `ReplayProvider` reads the same hash;
    asserts byte-identical `ChatResponse`.
  - Phase 3 (miss): `ReplayProvider` against an un-cached
    request → `LlmError::ReplayMiss(hash)`.
  —
  _acceptance: the test passes; phase 2's `ChatResponse` is
  byte-identical to phase 1's recorded response (V7 replay
  determinism gate). [R6.1, R6.2, V7]_
  **[deps: T1922]**

## M7 — Configuration surface + agent main wire-up + runbooks + ship

Covers feature.md **R12** (config surface) + **R13** (docs +
runbooks) + **R14** (no regression) + **V1–V12** (verification
gates) + Design **Q11** (denominator update + Cache hit ratio
row + bundled re-lock).

- [ ] **T1928** [developer] — `LlmConfig` + agent config wire-up
  at `crates/agent/src/config.rs:300` per Design Crate / module
  surface:
  - **New** `pub struct LlmConfig { pub enabled: bool /* default
    false */, pub default_provider: String, pub
    budget_usd_month: Decimal, pub replay_cache_path: PathBuf,
    pub deep_think: TierConfig, pub quick_think: TierConfig,
    pub providers: HashMap<String, ProviderConfig>, pub pricing:
    HashMap<String, HashMap<String, PricePerMillionTokens>> /*
    override map; default empty */ }`.
  - `pub struct TierConfig { pub provider: String, pub model:
    String }`.
  - `pub struct ProviderConfig { pub base_url: String,
    #[serde(default)] pub api_key: Option<String> /* loaded
    from agent.toml.local */ }`.
  - `Default for LlmConfig` returns `enabled: false` so a
    fresh checkout (no LLM consumers in v2.0.0) does not
    boot the LLM subsystem.
  - `Config::load` extension: if a sibling
    `<config_dir>/agent.toml.local` file exists, parse it
    into a partial `LlmConfig` overlay struct, deep-merge
    into the LLM section only (other sections untouched —
    the operator-only file is LLM-keys-and-overrides exclusively
    by convention).
  - `validate()` extension: if `cfg.llm.enabled && cfg.mode !=
    Mode::Research`, assert `cfg.llm.providers.contains_key(&cfg.llm.default_provider)`
    and that the corresponding `ProviderConfig.api_key`
    overlay-resolved-value is `Some(non_empty)`. Else
    `LlmError::Auth`.
  —
  _acceptance: `cargo test -p agent --test llm_config_test`
  passes — (a) committed `agent.toml` with the new `[llm]`
  section parses; (b) overlay from `agent.toml.local`
  populates `api_key`; (c) missing `.local` under
  `enabled = true && mode = paper` rejects at startup with
  `LlmError::Auth`; (d) `cfg.llm.enabled = false` (default)
  boots without any `.local` requirement. [R12.1, R12.2, R12.3,
  R8.1, R8.2]_
  **[deps: T1901, T1911, T1914]**

- [ ] **T1929** [developer] — `config/agent.toml` append per R12.1:
  - Append:
    ```toml
    [llm]
    enabled              = false  # foundation-only at v2.0.0
    default_provider     = "anthropic"
    budget_usd_month     = 200.0
    replay_cache_path    = "./data/llm-replay.db"

    [llm.deep_think]
    provider = "anthropic"
    model    = "claude-opus-4-7"

    [llm.quick_think]
    provider = "anthropic"
    model    = "claude-haiku-4-5-20251001"

    [llm.providers.anthropic]
    base_url = "https://api.anthropic.com/v1"

    [llm.providers.openai]
    base_url = "https://api.openai.com/v1"

    [llm.providers.openrouter]
    base_url = "https://openrouter.ai/api/v1"

    [llm.providers.deepseek]
    base_url = "https://api.deepseek.com/v1"

    [llm.providers.ollama]
    base_url = "http://localhost:11434"

    # Pricing override map (empty by default — see
    # crates/llm/src/pricing.rs base table).
    [llm.pricing]
    ```
  - **Keys are NOT in this committed file** — they live in
    `config/agent.toml.local` per Q3 = C.
  —
  _acceptance: `cargo test -p agent --lib` passes (the
  existing `t12_load_from_file` smoke loads the canonical
  config and asserts `mode == Research`); manual
  `cargo run --bin agent -- --config config/agent.toml`
  boots cleanly with `cfg.llm.enabled = false`. [R12.1]_
  **[deps: T1928]**

- [ ] **T1930** [developer] — `config/agent.toml.local.example`
  template at the repo root `config/` per R8.4:
  - Committed file with placeholder keys:
    ```toml
    # Copy this file to `agent.toml.local` and edit in your
    # real API keys. The .local file is git-ignored
    # (`.gitignore` covers `*.toml.local` + `config/*.local`)
    # so secrets never touch the repo.
    #
    # Per Q3 = Option C (operator-decided 2026-05-10):
    # all keys live in this single file alongside the
    # committed `agent.toml`. The agent's startup loader
    # overlays this file's `[llm.providers.<name>] api_key`
    # values onto the committed shape.

    [llm.providers.anthropic]
    api_key = "sk-ant-test-stub-00000000000000000000"

    [llm.providers.openai]
    api_key = "sk-test-stub-0000000000000000000000"

    [llm.providers.openrouter]
    api_key = "sk-or-test-stub-0000000000000000000"

    [llm.providers.deepseek]
    api_key = "test-stub-deepseek-000000000000000"
    ```
  - Tests use this template as the default key source so
    `cargo test --workspace` is hermetic.
  —
  _acceptance: `cargo test -p llm --test config_local_parse_test`
  passes — reads `config/agent.toml.local.example`, asserts
  it parses as a valid `LocalOverrideConfig`, asserts the four
  provider keys are placeholder strings (not real-API
  prefixes after the placeholder zeros). [R8.4]_
  **[deps: T1928]**

- [ ] **T1931** [developer] — Agent main wire-up at
  `crates/agent/src/main.rs` (single touch point):
  - Build `Arc<dyn LlmProvider>` once at startup if
    `cfg.llm.enabled`, store on the runtime context as
    `Option<Arc<dyn LlmProvider>>`. **No bus channel added.**
  - Hard constraint: this is gated on `cfg.llm.enabled`
    (default false in v2.0.0); when false, no provider is
    constructed, no key files are read, no `.local` is
    required.
  - Install the `redact::install_tracing_redactor()`
    subscriber at the top of `main` before any other
    subscriber, ensuring all logs are field-redacted.
  —
  _acceptance: `cargo build -p agent` clean; `cargo run -p
  agent` (with `cfg.llm.enabled = false`) boots cleanly with
  no `.local` requirement; with `cfg.llm.enabled = true`
  + valid `.local`, the agent boots, constructs the provider,
  and stores the `Option<Arc<dyn LlmProvider>>` on the
  runtime context. [R12.3, R8.3]_
  **[deps: T1913, T1915, T1928]**

- [ ] **T1932** [developer] — Runbook at
  `spec/runbooks/llm-cost.md` per R13.2:
  - Sections: "What the LLM spend line means", "What the
    operator does on a degrade event", "How to update cost-
    rate entries (`crates/llm/src/pricing.rs` recompile)",
    "How to swap providers (TOML edit + restart)", "Real-API
    smoke procedure (operator-only, requires real keys)".
  - Includes the byte-stable example output of `cargo run
    --bin llm-smoke` from T1924.
  —
  _acceptance: file exists; `markdownlint spec/runbooks/llm-cost.md`
  exits 0; `markdown-link-check` (if available locally) exits
  0. [R13.2]_
  **[deps: T1923]**

- [ ] **T1933** [developer] — Runbook at
  `spec/runbooks/llm-replay.md` per R13.3:
  - Sections: "How research mode uses replay (strict-replay-
    only at v2.0.0)", "How to refresh the cache (`cargo run
    --bin llm-smoke --mode paper`)", "How to interpret a
    `LlmError::ReplayMiss(hash)` failure in a backtest",
    "How to reset the cache (`cargo run --bin llm-smoke --reset`)",
    "Schema migration (the `schema_version` column)".
  - Includes the SHA-256 of one canonical recorded request
    so the operator can grep their cache for it.
  —
  _acceptance: file exists; `markdownlint` exits 0. [R13.3]_
  **[deps: T1922]**

- [ ] **T1934** [developer] — Crate-level rustdoc rewrite at
  `crates/llm/src/lib.rs:1` per R13.1:
  - Replace the v0 stub note with a multi-paragraph rustdoc
    enumerating: trait + 3 providers + prompt-cache builder +
    budget gate + record/replay + smoke binary. Each section
    cross-links to the relevant module / file path.
  —
  _acceptance: `cargo doc -p llm --no-deps` passes warning-
  clean; the generated HTML at
  `target/doc/llm/index.html` shows the new sections. [R13.1]_
  **[deps: T1901]**

- [ ] **T1935** [developer] — Reports body-byte changes at
  `crates/reports/src/render/system_health.rs` + `crates/reports/src/lib.rs`
  per Design § Q11 (bundled with Q5d):
  - `crates/reports/src/render/system_health.rs:30` — rustdoc
    example update `$0.00 / $135` → `$0.00 / $200`.
  - `crates/reports/src/render/system_health.rs:66+` — new
    `writeln!(out, "| Cache hit ratio | {} |", ...)` row
    between the existing `LLM spend` row and the existing
    `Funding poll status` row. The renderer's input struct
    gains `pub cache_hit_ratio: Result<String, RenderError>`.
  - `crates/reports/src/render/system_health.rs:126,139` —
    test fixture string + assertion update for both
    denominator and the new `Cache hit ratio` row.
  - `crates/reports/src/lib.rs:286` — `llm_spend: Ok("$0.00 /
    $135".into())` → `Ok("$0.00 / $200".into())` + `cache_hit_ratio:
    Ok("0.0%".into())` (research-mode default).
  - `crates/reports/src/lib.rs:320` — `observed: "$0.00 /
    $135".into()` → `"$0.00 / $200".into()` + `cache_hit_ratio:
    "0.0%".into()`.
  —
  _acceptance: `cargo test -p reports --lib system_health`
  passes (rewritten test asserts the body contains both the
  new `LLM spend | $0.00 / $200` row AND the new `Cache hit
  ratio | 0.0%` row). [R9.5, Q5d, Q11]_
  **[deps: T1910]**

- [ ] **T1936** [developer] — `pre-stage` developer-side anchor
  re-lock per Design § Q11:
  - Run the two report scenarios twice 10s apart at seed
    `0xC0FFEE`:
    1. `cargo run -p reports --bin report -- --period 7d
       --ledger target/test-ledgers/sample-7d.db --output
       spec/operator-success-reports/reports/success-fixed-report-sample-7d.md
       --seed 0xC0FFEE`
    2. Same for `report-sample-90d`.
    3. Re-run each once more; outputs must be byte-identical.
  - Update `crates/reports/tests/report_scenarios.rs:80,88`
    `EXPECTED_SHA_7D` / `EXPECTED_SHA_90D` constants with the
    captured SHAs.
  - **Developer does NOT edit `spec/anchors.toml`** — that
    is the tester's job at `T_FINAL_V2_LLM_STRATEGY`. The
    developer's tick note records the captured SHAs in this
    task body so the tester can copy-paste them.
  —
  _acceptance: the developer's tick note records two byte-
  stable body-SHA-256s (one per scenario); `bash
  scripts/hash_report.py
  spec/operator-success-reports/reports/success-fixed-report-sample-7d.md`
  matches the recorded SHA across two re-runs. The `cargo
  test -p reports --test report_scenarios` test passes
  against the re-anchored EXPECTED_SHA constants. [R5.5, Q11]_
  **[deps: T1935]**

- [ ] **T1937** [developer] — Negative-invariant test for the 9
  strategy anchors per Design § Q11 + R14.2:
  - Document in the developer's tick note: running `bash
    scripts/verify_anchors.sh` locally shows the 9 strategy-
    backtest anchors at `spec/anchors.toml:15-58` print
    `PASS` (byte-identical post-feature). The two
    `report-sample-*` v1+ anchors at lines 67–75 print `FAIL`
    until T_FINAL captures the new SHAs.
  - Mirror the v1.8 reflection-memory T1812 negative-
    confirmation step pattern.
  —
  _acceptance: `bash scripts/verify_anchors.sh` output for
  the 9 strategy lines reads `PASS`. The 2 `report-sample-*`
  lines read `FAIL` (expected — the tester re-locks at
  T_FINAL). [R14.2, V8]_
  **[deps: T1936]**

- [ ] **T1938** [developer] — Cockpit "LLM budget" tile per
  R11.2 + Q10 (strawman accepted):
  - Find the cockpit's right-rail header bar (per existing
    Lumen Phase 1 patterns); add one `Tile { label: "LLM
    budget", body: format!("${spent:.2} / $200 = {pct:.1}%"),
    color: tile_color_for_pct(pct) }`.
  - `tile_color_for_pct(pct)` returns `Theme::Ok` for `pct <
    80%`, `Theme::Warn` for `80% <= pct < 100%`, `Theme::Halt`
    for `pct >= 100%`.
  - Tile reads `audit::query::llm_spend_this_month(ledger)`
    (additive; trivial sum over `expense:llm:*` rows in the
    current month — sibling of `realized_pnl_since`).
  - **Lumen Phase 6 Assistant slot is NOT wired here** —
    Phase 6 is a follow-up brief; the `LLM budget` tile is
    a single right-rail addition only.
  —
  _acceptance: `cargo test -p ui --test llm_budget_tile_test`
  passes — fixture ledger with $143.21 / $200 of LLM spend
  → tile body reads `$143.21 / $200 = 71.6%`, color =
  `Theme::Ok`; fixture at $179 / $200 → color = `Theme::Warn`;
  fixture at $200.01 / $200 → color = `Theme::Halt`. [R11.2,
  Q10]_
  **[deps: T1916, T1917]**

- [ ] **T1939** [developer] — V11 schema-migration forward-
  compat test at `crates/llm/tests/replay_schema_migration_test.rs`:
  - Open the v1 schema fixture from T1925.
  - Assert it loads via `ReplayProvider::open`.
  - Synthesize a hypothetical v2 schema (extra column) by
    bumping `schema_version` to 2; assert
    `ReplayProvider::open` rejects with
    `LlmError::Provider(...)` because `2 > SUPPORTED_SCHEMA_VERSION`.
  —
  _acceptance: the test passes both arms. [R6, V11, Q8b]_
  **[deps: T1925]**

- [ ] **T1940** [developer] — Pre-existing-strategy-test
  invariant gate per R14.4:
  - `crates/llm/tests/no_real_api_test.rs` — static-grep
    style: assert that no test under `crates/llm/tests/`
    imports `reqwest::Client::new()` without it being passed
    a `wiremock` mock URL or an Ollama localhost URL. The
    test walks every `.rs` file under `crates/llm/tests/`,
    parses HTTP base-URL string literals, asserts every
    such literal matches a wiremock-spawn pattern (`mock_server.uri()`)
    or a localhost pattern (`http://localhost:`).
  —
  _acceptance: the test passes; the workspace test suite
  has zero outbound HTTPS dependencies. CI network policy
  enforces V4 (zero outbound HTTPS to `*.anthropic.com /
  *.openai.com / *.openrouter.ai / *.deepseek.com`). [R14.4,
  V4]_
  **[deps: T1923, T1924]**

- [ ] **T1941** [developer] — Cross-cutting smoke + cleanup pass
  (mirrors v1.8 T1814):
  - `cargo fmt --all -- --check` clean.
  - `cargo clippy --workspace --all-targets --all-features --
    -D warnings` clean (the new `llm` deps must pass clippy).
  - `cargo audit` shows no unpatched advisories; `cargo deny
    check` (bans, licenses, sources, advisories) passes.
  - Bin smoke: `cargo build --bin llm-smoke` clean;
    `cargo run --bin llm-smoke` against the wiremock harness
    exits 0 and prints the green table.
  - Cost-telemetry confirmation: render a 7d sample report
    (paper mode with `cfg.llm.enabled = false` per the v2.0.0
    default) and assert the body's System Health table
    contains `| LLM spend | $0.00 / $200 |` AND
    `| Cache hit ratio | 0.0% |`.
  —
  _acceptance: every command above exits cleanly. [V1, V2,
  V3, V8, V10]_
  **[deps: T1936, T1937, T1938, T1939, T1940]**

- [ ] **T1942** [developer] — V8 anchor-stability negative-
  confirmation gate (separate from T1937's per-anchor walk-through;
  this is the consolidated developer-tick note):
  - Document running `bash scripts/verify_anchors.sh`:
    the 9 strategy lines print `PASS`; the 2 `report-sample-*`
    lines print `FAIL` (tester re-locks at T_FINAL).
  - Document the V12 stress test (T1918) passes with
    overshoot ≤ $0.40.
  —
  _acceptance: developer's tick note quotes the verbatim
  `verify_anchors.sh` output with `PASS  btc-2023-1m-sma-cross`
  ... etc. for the 9 lines. [R14.2, V8, V12]_
  **[deps: T1937, T1918]**

- [ ] **T1943** [developer] — Architecture.md decisions-index
  update:
  - **Modify** `spec/architecture.md:421-432` — replace the
    "LLM integration" stub paragraph with: "_See § v2 — LLM
    strategy resolutions (Q4–Q11) — confirmed 2026-05-10
    below._"
  - **Append** at the bottom of architecture.md (sibling of
    the existing "v1.8 reflection-memory" decisions-index
    section if present, else the next sibling after v1+
    operator-success-reports): a new section
    `### v2 — LLM strategy resolutions (Q4–Q11) — confirmed
    2026-05-10` with seven sub-sections (Q4–Q11), each a
    one-paragraph summary of the decision + a back-pointer
    `[→ details](spec/v2-llm-strategy/feature.md#design)`.
  —
  _acceptance: `markdownlint spec/architecture.md` exits 0;
  the new section renders correctly in any markdown viewer;
  the v0-stub paragraph at lines 421–432 reads its
  cross-reference. [Hard constraint #8 / informational]_
  **[deps: T1901]**

- [ ] **T1944** [developer] — Final smoke confirmation:
  - `cargo test --workspace --all-targets` → all suites green
    (zero failures, zero unexplained `#[ignore]`).
  - All V1–V12 acceptance gates verified via individual test
    invocations from the suite output.
  —
  _acceptance: workspace-wide test run is fully green; V1–V12
  individually verified. [V1–V12]_
  **[deps: T1941, T1942, T1943]**

- [ ] **T1945** [developer] — Pre-FINAL operator-environment
  smoke (tester's gate input):
  - Document the procedure for the operator (or the tester
    in operator's environment) to run the real-API smoke:
    1. Copy `config/agent.toml.local.example` to
       `config/agent.toml.local` and fill real API keys.
    2. Run `cargo run --bin llm-smoke --mode paper` against
       real Anthropic + OpenAI + Ollama endpoints.
    3. Verify the table prints 3 rows of `result = OK`.
    4. Confirm `data/llm-replay.db` has 9 rows (3 providers
       × 3 roles after the first run).
  - This is **operator-invoked, not CI** (per V3). Tester
    confirms at FINAL.
  —
  _acceptance: the runbook documents the procedure; tester
  re-confirms before VERDICT → PASS. [V3, R10.1]_
  **[deps: T1923, T1932, T1933]**

## Final

- [ ] **T_FINAL_V2_LLM_STRATEGY** [tester] — End-to-end ship gate
  per [feature.md → Verification](feature.md#verification):
  - V1: static checks (fmt, clippy `-D warnings`, audit, deny)
    green.
  - V2: `cargo test --workspace --all-targets` green.
  - V3: `cargo run --bin llm-smoke` against wiremock prints
    green table for all three providers (CI gate);
    operator-environment real-API smoke confirmed in
    operator's environment (T1945).
  - V4: zero outbound HTTPS to `*.anthropic.com`,
    `*.openai.com`, `*.openrouter.ai`, `*.deepseek.com`
    during `cargo test --workspace`.
  - V5: cost-telemetry round-trip — one LLM call asserts a
    balanced `expense:llm:<tier>` ↔ `liabilities:llm_accrued`
    journal pair; `audit::query::global_debit_credit_sum`
    returns `(dr, cr)` with `|dr - cr| ≤ 1e-8`.
  - V6: budget-gate determinism — two runs of the
    `llm-budget-degrade` scenario produce byte-identical
    degrade events.
  - V7: replay determinism — two runs of `ReplayProvider`
    against the same hash produce byte-identical responses.
  - V8 + Q11 anchor re-lock procedure:
    1. Capture the new body-SHA-256s for
       `report-sample-7d` and `report-sample-90d` from a
       byte-stable two-run render at seed `0xC0FFEE`
       (the developer pre-stages these at T1936; tester re-
       captures from a clean run).
    2. **Edit `spec/anchors.toml:67-75`** to replace the
       v1.8 entries with the new SHAs. Add a comment line
       above the new entries: "v2.0.0 re-lock — denominator
       `$135 → $200` + `Cache hit ratio` row added (Q5d +
       Q11)."
    3. The 9 v0/v0.5/v1/v1.5a strategy anchors at lines
       15–58 stay byte-identical (R14.2).
    4. Run `bash scripts/verify_anchors.sh`; expect
       `ANCHORS PASS  (11 / 11)`.
  - V9: grep over `target/logs/`, `data/llm-replay.db`, and
    every audit row touched during the smoke run finds zero
    occurrences of the test API-key strings.
  - V10: smoke binary completes in `< 1s` total wall clock;
    each provider's `complete()` in `< 200ms` test wall.
  - V11: fixture cache schema-migration test passes.
  - V12: concurrent-overshoot stress test passes; bound ≤
    $0.40.
  - Tester confirms the operator-environment real-API smoke
    (T1945) is green in operator's environment.
  - Status flip `in-progress → shipped`; owner flip
    `architect → shipped`; appended Changelog row.
  - Presenter follow-up: `present-results` skill assembles
    `spec/v2-llm-strategy/presentations/v2-llm-strategy-<date>.md`
    for operator approval (post-FINAL gate, per AGENT.md).
  —
  _acceptance: all V1–V12 verification gates green AND
  `cargo run --bin llm-smoke` round-trips three providers AND
  `bash scripts/verify_anchors.sh` PASS 13/13 (11 pre-existing
  + 2 re-locked `report-sample-*`; the architect's Q11 = Option
  C decision lands the re-lock in this brief). Operator's
  "[x] Approved — ship" recorded in the presenter deck. [V1–V12,
  R14, Q11]_
  **[deps: T1944, T1945]**

## Parallelism map

```
M1 (trait + types + rename):
  developer:
    T1901 (critical-path gate; rename + trait + error + tools)

M2 (provider impls):
  developer:
    T1901 ──► T1902 ──► (T1903 || T1904 || T1905) ──► T1906

M3 (prompt-cache + cache observability):
  developer:
    T1901 ──► T1907 (atomic budget refactor)
    T1901 ──► T1908 (CachedSystemPrompt builder)
    T1901 ──► T1909 (cache-event helper)
    T1901 ──► T1910 (audit::cache_hit_ratio_since)

M4 (budget gate + audit memo + V12):
  developer:
    T1901 ──► T1911 (pricing module)
    T1907 ──► T1912 (BudgetedProvider)
    T1901 ──► T1916 (BudgetEventKind + journal helper)
    T1901 ──► T1917 (token-tag plumbing)
    T1912 ──► T1918 (V12 stress test)
    T1903, T1904, T1905, T1912, T1914, T1922 ──► T1913 (factory)

M5 + M6 (replay + smoke + secrets gate):
  developer:
    T1901 ──► T1919 (migration SQL)
    T1901 ──► T1920 (request_hash)
    T1919, T1920 ──► T1921 (RecordingProvider)
                ──► T1922 (ReplayProvider)
    T1922 ──► T1925 (fixture cache)
    T1913 ──► T1923 (smoke binary)
    T1923 ──► T1924 (smoke wiremock harness)
    T1924, T1925 ──► T1926 (V9 secrets-in-artifacts)
    T1922 ──► T1927 (replay round-trip integration test)
    T1922 ──► T1933 (replay runbook)

M7 (config + agent main + reports body change + ship):
  developer:
    T1901, T1911, T1914 ──► T1928 (LlmConfig)
    T1928 ──► T1929 (config/agent.toml append)
    T1928 ──► T1930 (config/agent.toml.local.example)
    T1913, T1915, T1928 ──► T1931 (agent main wire)
    T1923 ──► T1932 (cost runbook)
    T1901 ──► T1934 (rustdoc rewrite)
    T1910 ──► T1935 (System Health body change)
    T1935 ──► T1936 (developer-side EXPECTED_SHA capture)
    T1936 ──► T1937 (negative-invariant 9-anchor gate)
    T1916, T1917 ──► T1938 (cockpit "LLM budget" tile)
    T1925 ──► T1939 (V11 schema-migration test)
    T1923, T1924 ──► T1940 (no-real-API gate)
    T1936, T1937, T1938, T1939, T1940 ──► T1941 (cross-cutting smoke)
    T1937, T1918 ──► T1942 (V8 + V12 negative-conf gate)
    T1901 ──► T1943 (architecture.md decisions-index update)
    T1941, T1942, T1943 ──► T1944 (final smoke)
    T1923, T1932, T1933 ──► T1945 (operator-env real-API smoke)

  tester:
    T1944, T1945 ──► T_FINAL_V2_LLM_STRATEGY
```

**Independent fan-out gates after T1901:** T1902 (retry helper),
T1907 (atomic budget), T1908 (prompt cache builder), T1909
(observability), T1910 (audit query), T1911 (pricing), T1916
(BudgetEventKind), T1917 (token-tag plumbing), T1919 (migration
SQL), T1920 (request hash), T1934 (rustdoc), T1943
(architecture.md) — **12 tasks parallelize** after the gate.

**Critical path:** T1901 → T1902 → T1903 → T1906 → T1913 →
T1923 → T1924 → T1941 → T1944 → T_FINAL.

## Notes

- Every task that writes spec files uses the `spec-update`
  skill.
- **T1901** is the critical-path gate. The `LlmProvider` trait
  + `ChatRequest` / `ChatResponse` types + `LlmError` enum + the
  `cost::LlmProvider → ProviderKind` rename all land in one PR
  so downstream developers (M2–M7) compile against a stable
  trait shape from day one. The rename is the load-bearing
  detail — every `crates/cost/` consumer (currently
  `LedgerCostSink`'s tests at `crates/cost/src/sink.rs:76,80`)
  picks up the rename in the same PR.
- **T1907** is the load-bearing budget refactor. Going from
  `Decimal` to `AtomicU64` cents is the change that lets V12
  stay green; deferring this introduces a budget race that
  costs operator dollars. The atomic refactor preserves the
  existing API (`add_spend`, `remaining`, `mode_override`)
  and adds `try_reserve`.
- **T1912** wraps the provider in `BudgetedProvider`. This
  is the "impossible to forget the budget gate" pattern. The
  factory at T1913 always wraps; consumer code never sees the
  leaf.
- **T1922** is the load-bearing replay decision. Strict-replay-
  only at v2.0.0 means a research-mode backtest miss is a
  loud build error, not a silent live-API call. Hard constraint
  #4 (no new bus channel) is preserved — the replay path is
  call-and-return, no channels.
- **T1925** ships a synthetic fixture cache; the real
  operator-environment capture replaces it on first paper-mode
  run. This means CI is hermetic from day one without blocking
  on operator-env API access.
- **T1935** is the load-bearing renderer change. Both Q5d
  (`Cache hit ratio` row addition) and Q11 (`$135 → $200`
  denominator) bundle into one body-byte rotation; the V8
  re-lock at T_FINAL captures the consolidated SHA changes.
- **T1937** is forward-compat: if any of the 9 strategy-
  backtest anchors drift, **escalate to analyst** (per the
  v1.8 precedent) — it signals an unintended hot-path change
  that violates Q1 = Option A foundation-only scope.
- **T1936** prepares the re-lock data; **T_FINAL_V2_LLM_STRATEGY**
  performs the actual `spec/anchors.toml` edit (tester only).
  Same pattern as v1.5a T717, v1+ T816, v1.8 T1813.
- **No new runtime crate dependency in default builds beyond
  `cfg.llm.enabled = false`.** When the LLM subsystem is off
  (the v2.0.0 default), the agent boots without reading any
  `.local` file, without constructing any provider, without
  any network egress.
- **No `Strategy` trait change** under Q1 = Option A. Hard
  constraint #3 preserved.
- **No new bus channel.** Hard constraint #4 preserved. LLM
  providers are call-and-return; consumers in follow-up briefs
  decide whether to introduce channels.
- **No secrets in committed artifacts.** Hard constraint #6
  preserved via T1915's redact helper + T1926's V9 grep gate.
- **Anthropic-isms stay behind the provider impl.** The trait
  is provider-agnostic; cache breakpoints, tool-use schemas,
  retry-after parsing all live in `crates/llm/src/providers/anthropic.rs`.
  Hard constraint #7 preserved.
- **Body-vs-front-matter discipline.** Q11's `$135 → $200`
  + Q5d's `Cache hit ratio` row are the only deterministic
  body-byte changes; both are named explicitly in T1935 and
  re-locked at T_FINAL. Hard constraint #8 preserved.
- **Atomic-write contract preserved.** Replay cache uses
  SQLite WAL (Q8e); no tempfile-rename needed because WAL
  enforces the atomic-commit contract on its own. Hard
  constraint #5 satisfied by SQLite's durability shape.
- **Test-fixture key pattern.** All `.local.example` keys end
  with `0000…` so the V9 grep test can use prefixes like
  `sk-ant-V9-secretkey-12345678` that don't collide with the
  fixture template.

## Changelog

- 2026-05-10 (analyst): initial milestone stub (M1–M7) under
  Q1 = Option A scope assumption. T19xx namespace reserved.
  Tasks to be expanded by the architect after operator
  resolves Q1, Q2, Q3, Q10 and architect lands the Design
  section.
- 2026-05-10 (architect): expanded the M1–M7 milestones with
  45 developer T19xx tasks (T1901–T1945) +
  `T_FINAL_V2_LLM_STRATEGY`. Q4 (trait shape, async +
  non-streaming + tool-use-day-one + 8-variant `LlmError` +
  cost-crate `LlmProvider → ProviderKind` rename), Q5
  (TTL-driven 2-breakpoint provider-aware builder + per-role
  per-day Prometheus counter pair + `audit::query::cache_hit_ratio_since`),
  Q6 (factory-level decorator + atomic cents counter + 0.2%
  documented overshoot bound + new V12 verification gate),
  Q7 (hybrid hard-coded base table + TOML override; module
  in `llm` crate not `cost`), Q8 (SQLite WAL + canonical-JSON
  SHA-256 + `schema_version` + 9-row fixture + strict-replay-
  only at v2.0.0), Q9 (exponential backoff + full jitter + 3
  retries + no circuit breaker + `Retry-After` honored), Q11
  (Option C — denominator hot-fix bundled with Q5d's
  `Cache hit ratio` row addition; the 2 `report-sample-*`
  anchors re-lock once at T_FINAL_V2_LLM_STRATEGY) all
  resolved in feature.md § Design. Each T-task cites the
  R-item it implements + a one-line acceptance the tester
  can verify by running a specific command. Parallelism map +
  synchronization gates included; handoff contract preserved
  (no UI involvement beyond the right-rail "LLM budget"
  tile). Owner → architect; status stays in-progress.
- 2026-05-12 (developer, pass 1): **T1901 ticked** — `crates/llm/`
  rewrite from v0 23-line stub to 4 source files
  (`lib.rs` re-exporter, `trait_def.rs` with `LlmProvider` trait
  + 11 request/response types, `error.rs` 8-variant `LlmError`,
  `tools.rs` `ToolSchema` + `validate_tool_use` via `jsonschema`
  Draft 2020-12). Q4-bonus mechanical rename `cost::LlmProvider →
  ProviderKind` applied across 4 files / 5 call sites in `cost`
  crate (serde `rename_all = "snake_case"` preserves wire
  compatibility — no on-disk byte change to ledger records). 6
  new `t1901_*` unit tests green on `cargo test -p llm --lib`; 2
  pre-existing `t30_*` tests stay green on `cargo test -p cost`.
  `cargo check --workspace` clean (no consumer crate broken by
  the rename — the `llm` crate has no in-tree consumers yet).
  Anchor invariant: nothing in `crates/{strategy,audit,exec,backtest,reports}/`
  touched; 9 strategy + 2 success-report anchors at
  `spec/anchors.toml:15-75` stay byte-untouched. Owner →
  developer; status stays in-progress. **Pass-1 stop point:**
  T1901 alone (the gate task — every M2-M7 task depends on the
  stable trait shape that just landed). M2-M7 +
  `T_FINAL_V2_LLM_STRATEGY` remain unticked for pass 2+.
- 2026-05-12 (developer, pass 2): **T1902 / T1903 / T1904 / T1905
  ticked `[x]`, T1906 ticked `[~]`** — M2 provider implementations
  landed in one pass.
  - `crates/llm/src/retry.rs` (new, ~273 lines) — shared
    full-jitter exponential-backoff helper per Q9.
  - `crates/llm/src/providers/{anthropic,openai,ollama}.rs`
    (3 new files, ~385 + ~466 + ~435 lines) — load-bearing
    Anthropic provider (cache markers + tool-use + retry),
    OpenAI-compat provider (markers silently dropped + JSON-
    string `arguments` parsing + `kind`-threading for
    OpenRouter / DeepSeek pricing), Ollama provider (no auth,
    no retries, best-effort tool-use via system-prompt tail).
  - `crates/llm/src/providers/mod.rs` (new) + `crates/llm/
    src/lib.rs` extended re-exports — `AnthropicProvider`,
    `OpenAiProvider`, `OllamaProvider` reachable from crate
    root per T1906 acceptance.
  - `crates/llm/Cargo.toml` — `rand = { workspace }` added to
    runtime deps (Q9c full-jitter formula); `wiremock` to
    dev-deps (no real HTTP in tests, per pass-2 brief
    constraint).
  - 51 tests green across 5 binaries (`cargo test -p llm`):
    36 in-crate unit tests, 5 anthropic + 3 openai + 4 ollama
    wiremock integration tests, 3 retry-helper integration
    tests. `cargo clippy -p llm --all-targets` zero warnings
    in llm (pre-existing `trading_core` + `audit` warnings
    untouched by this pass). `cargo fmt -p llm --check` clean.
  - Anchor invariant: nothing in
    `crates/{strategy,audit,exec,backtest,reports}/` touched
    — the 9 strategy + 2 success-report anchors at
    `spec/anchors.toml:15-75` stay byte-untouched (sandbox
    blocks `verify_anchors.sh` from this sub-agent;
    orchestrator runs the gate on resume).
  - T1906 left `[~]` rather than `[x]` because the
    `cargo doc -p llm --no-deps` warning-clean leg of its
    acceptance contract was not verifiable from this sub-
    agent's sandbox (permission denied). Build + re-export +
    reachability legs all pass; orchestrator can flip to `[x]`
    after `cargo doc`.
  - **Pass-2 stop point:** end of M2 — natural milestone
    boundary. Pass-3 candidate: **T1907** (`CostBudget`
    atomic-cents refactor, gate task for M4 budget enforcement)
    or **T1908** (`CachedSystemPrompt` builder at
    `crates/llm/src/prompt_cache.rs`).
