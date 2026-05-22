---
slug: v3-llm-forecaster
status: proposed
owner: analyst
updated: 2026-05-22
version: 0.1.0
predecessor: v2-llm-strategy v2.0.0
parent: strategy-reformulation-survey-2026-05-22 Candidate 5
promoted_2026_05_22: Queue → Active by operator under v3-volatility-forecaster-noop-fix v0.1.0 deck approval (C1 retired with NEGATIVE-NET-DELTA evidence; C5 picked over C2 for moat-alignment + crates/llm infra reuse). Next: analyst-bridge to populate tasks.md (the existing analyst pass was spec-only design exploration; needs OD/architect/dev waves scoped before architect M-T1).
promotion_ref: spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md
---

# v3 LLM-as-forecaster — reflection-memory + audit-trail-anchored signal

> **Spec-only design exploration — NO code commitment.** Per operator
> Q-SEQ HYBRID at the 2026-05-22 strategy-reformulation-survey
> operator-decide (~6-8 weeks total budget split across C1 + C2 + C5;
> 3 analyst passes in parallel), this brief authors the full R / K /
> H / Q register but **does not promote to Active**. Architect M-T1
> and developer waves are **DEFERRED** until either (i) C1
> (`v3-volatility-forecasting`) ships a verdict and operator
> promotes this slug, or (ii) operator explicitly promotes ahead of
> C1's verdict. The slug `v3-llm-forecaster` is the analyst-final
> pick (alternates `v3-llm-reflection-overlay`, `v3-llm-as-forecaster`
> rejected as longer + less hyphenation-friendly). Trace row
> `REQ-V3-LLM-FORECASTER-001` opens in `draft` state — no `arch`,
> `crates`, `tests`, `anchors` columns until promotion.

## Why

Candidate 5 from the
[strategy reformulation survey](../dev-notes/strategy-reformulation-survey-2026-05-22.md#candidate-5--reflection-memory-as-forecaster-v2-llm-signal)
asks the load-bearing strategic question the v25 DL journey couldn't
ask: **does the differentiator (persistent reflection memory + audit
ledger, locked at [product.md § Differentiator line 80](../product.md#differentiator))
extract alpha as a signal source, or is it purely an explainability
asset?** The survey rated C5 as

- **highest novelty** (Sequence B "highest-EV-first single-bet" head
  pick — survey § Sequence B / line 880);
- **best product.md moat-statement alignment** — the moat is
  `(2) + (4)` per [product.md line 79-83](../product.md#differentiator)
  (persistent reflection memory + auditable double-entry); C5 builds
  a signal source that compounds *both*;
- **highest-variance EV** (LOW-MEDIUM prior on +0.10 Sharpe-delta vs
  v1 baseline; could be 0 Sharpe-delta or 0.2+; survey K-llm-3 line 497).

After the v25 DL umbrella retired 2026-05-22 (joint F4-F4-F4 across
3 model checkpoints / 2 model families / 2 horizons —
[retrospective § Headline verdict line 22](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md#headline-verdict)),
the operator's prior shifted: the next research direction should
**not** chase `(same 5-feature input + 1h or 24h next-bar log-return
target)`. Per the retrospective's "what COULD usefully chase" §
line 192:

> **Reflection-memory consumption** — the v2 LLM analyst now has
> Memory + Models screens (Phase F). Could the reflection-memory
> itself + LLM debate provide a forecast-equivalent signal without DL?

C5 is the answer-shape to that bullet. It is the **only candidate in
the 7-row survey** whose hypothesis is information-theoretically
independent of `DL-on-OHLCV` (survey § "Independence from v2.5 F4"
column — HIGHEST). The signal source is **LLM reasoning over
context + persistent memory + cross-pair narratives**; the DL F4-F4-F4
evidence chain bounds neither the prior nor the variance here.

### Strategic wake conditions (all met)

All three pre-requisites for C5 to be a *real* feature (not just a
survey row) shipped before this analyst pass:

1. **`v2-llm-strategy v2.0.0`** shipped 2026-05-13
   ([backlog Recent line 2087-ish](../backlog.md);
   [feature.md](../v2-llm-strategy/feature.md)). The
   foundation-only ship surfaces:
   - `LlmProvider` trait + 3 provider impls (Anthropic / OpenAI-
     compatible / Ollama) at [`crates/llm/src/trait_def.rs`](../../crates/llm/src/trait_def.rs)
     + [`providers/`](../../crates/llm/src/providers/).
   - `BudgetedProvider<Inner>` decorator at
     [`crates/llm/src/budgeted.rs`](../../crates/llm/src/budgeted.rs)
     — auto-degrades at 80 % monthly spend; blocks at 100 % per
     [product.md § Cost economics line 346](../product.md#cost-economics--monthly-ceiling).
   - `RecordingProvider` + `ReplayProvider` decorators at
     [`crates/llm/src/recording.rs`](../../crates/llm/src/recording.rs)
     + [`crates/llm/src/replay.rs`](../../crates/llm/src/replay.rs)
     — sqlite-backed `(request_hash, response)` cache; **load-bearing
     for Q5 determinism** below.
   - `CachedSystemPromptBuilder` with two cache breakpoints at
     [`crates/llm/src/prompt_cache.rs`](../../crates/llm/src/prompt_cache.rs)
     — provider-aware; Anthropic emits real `cache_control: ephemeral`
     markers; ~75% input-token discount on repeat prompts.
   - `ToolSchema` + JSON-schema validation at
     [`crates/llm/src/tools.rs`](../../crates/llm/src/tools.rs) —
     structured-output contract; no free-text parsing.

2. **`reflection-memory v0.1.0`** + **Phase F Memory screen** shipped.
   - The `LessonCard` writer pipeline + `top_k` retrieval + 32-dim
     deterministic embedding live at
     [`crates/reflection/src/`](../../crates/reflection/src/);
     `top_k` signature is
     `store.top_k(query: RetrievalQuery, k: usize) -> Vec<(LessonCard, score)>`
     ([`store/mod.rs:29`](../../crates/reflection/src/store/mod.rs)).
   - Phase F Memory screen at
     [`crates/ui/src/screens/memory.rs`](../../crates/ui/src/screens/memory.rs)
     renders cards reverse-chronologically per
     [Phase F R1.2](../ui-rethink-phase-f-memory-models-assistant/feature.md#r1--screensmemory-j7--reflection-memory).
   - **The lesson-card store is currently consumer-poor.** Per
     [`crates/reflection/src/lib.rs:11-18`](../../crates/reflection/src/lib.rs):
     > "Q4 = report-only — The trader's `Strategy` trait does not
     > consume retrieval; the only caller of `retrieve_top_k` is the
     > operator success report's memory-highlights renderer. Trader-
     > side wiring is a follow-up brief named
     > `reflection-memory-trader-wiring`."

3. **`crates/audit/` + audit-tick stream** shipped Phase D + Phase D+
   ([Phase D feature.md](../ui-rethink-phase-d-trail/feature.md)).
   Every LLM call already audit-emits via the v2-llm-strategy ship;
   the audit ledger is the LLM-strategy ground truth.

4. **Phase F Assistant slot** wakened structurally at v0.1.0
   ([Phase F R3 + Q4=(a) stub](../ui-rethink-phase-f-memory-models-assistant/feature.md#r3--right-rail-assistant-slot-lumen-phase-6)).
   The Lumen-Phase-6 right-rail body lives at
   [`crates/ui/src/assistant/view.rs`](../../crates/ui/src/assistant/view.rs);
   the open-state placeholder reads
   > "Assistant offline. v2 LLM wiring lands in v0.2.0."
   **Promoting this slot to render LLM reasoning trace IS the
   highest product-differentiation surface this feature can ship** —
   Q4 below pins the consumer shape.

### The hypothesis chain (one line each)

- **H1** (alpha) — LLM-as-forecaster + reflection-memory + audit-trail
  produces ≥ +0.10 Sharpe-delta vs v1 momentum baseline on BS-1 /
  BS-2 realdata scenarios (the v2.5 alpha threshold preserved).
- **H2** (cost) — LLM call cost (Claude pricing × calls/day) is <
  operator's monthly cost budget at meaningful trading frequency
  (v2 ceiling $200 / month per
  [product.md § Cost economics line 343](../product.md#cost-economics--monthly-ceiling)).
- **H3** (UX) — Reasoning trace quality is operator-reviewable in the
  Phase F Assistant slot (subjective; operator-judged).
- **H4** (determinism) — Replay-cache + `temperature = 0` +
  prompt-cache produces byte-identical anchored backtests across
  re-runs (the v2.5 anchor-byte-identity invariant preserved).
- **H5** (schedule) — 3-5 week feasibility per survey (lots of new
  surface; novel territory; HIGH variance).

### Differentiator alignment

Per [product.md § Differentiator line 79-83](../product.md#differentiator):

> "Confirmed bet (2026-04-17): long-term moat is (2) + (4) —
> persistent reflection memory plus auditable double-entry ledger.
> Most projects can write 'more agents'; few persist real
> institutional memory or expose a defensible audit trail."

C5 is the **only** survey candidate where the signal source IS the
moat. Other candidates (vol / regime / non-DL) extract alpha
*orthogonally* to memory + audit. C5 puts memory + audit on the
critical path of signal emission — if it works, the moat compounds
*as the strategy*, not just *around* it. The downstream
narrative-bearing artifact (a reasoning trace per decision, anchored
into the audit ledger, retrievable as a lesson card, surfaced in the
Phase F Assistant slot, retro-fed into the next decision via top_k)
**is** the differentiator made trading-time-callable.

## Requirements

> Numbered + testable. Architect M-T1 (when funded) ratifies these
> into a `decomp.md` Wave plan; developer waves cite by R-id.

### R1 — `LlmForecaster` trait + signal shape

- **R1.1** — New trait `LlmForecaster: Send + Sync + 'static` at
  `crates/strategy/src/llm_forecaster/trait_def.rs`. Signature:
  ```rust
  #[async_trait::async_trait]
  pub trait LlmForecaster {
      fn name(&self) -> &str;
      async fn forecast(
          &self,
          ctx: ForecastContext,
      ) -> Result<LlmForecast, LlmForecasterError>;
  }
  ```
  **Architect M-T1 lock**: whether this trait sits at the strategy
  layer (Q4=a standalone) or under `crates/forecast::ForecastProvider`
  (Q4=b overlay) is **operator-decide** per Q4 below. Default below
  picks Q4=(a)+(c) hybrid — standalone strategy + Phase F Assistant
  slot consumer; (b) overlay deferred to v0.2.0.

- **R1.2** — `LlmForecast` payload (Q1 default = a):
  ```rust
  pub struct LlmForecast {
      pub rating: Rating,           // STRONG_SELL | SELL | HOLD | BUY | STRONG_BUY
      pub confidence: Confidence,   // Decimal in [0, 1]
      pub horizon: Horizon,         // Short | Medium | Long per product.md line 158
      pub reasoning_trace: String,  // human-readable; anchored
      pub reasoning_trace_sha256: [u8; 32],  // body-SHA over trace; anchor-stable
      pub retrieved_lessons: Vec<LessonCardRef>,  // top-K from reflection-memory
      pub cost_event: CostEventRef, // links audit `expense:llm:*` row
      pub correlation_id: Uuid,     // links to audit + replay-cache
  }
  ```
  - Q1=(a) shape — discrete 5-tier rating + confidence + reasoning
    trace. **Analyst-recommended** (matches
    [product.md § Five-tier rating scale line 156](../product.md#five-tier-rating-scale)
    verbatim). Q1=(b) μ-equivalent forecast number rejected because
    the joint F4-F4-F4 evidence says μ-prediction is the wrong task
    shape; emitting μ-equivalent here would re-litigate the
    falsified hypothesis. Q1=(c) regime label overlaps C2.
    Q1=(d) free-form is the v0.2.0 follow-on (current shape is
    structured directive + free-form trace; covers (d) hybrid).

- **R1.3** — `Rating::to_signal_overlay()` — converts to a
  `crates/strategy::Signal` overlay weight in `[-1.0, +1.0]` for
  consumers in Q4=(b) overlay shape. Default mapping:
  STRONG_SELL=-1 / SELL=-0.5 / HOLD=0 / BUY=+0.5 / STRONG_BUY=+1.
  **Architect M-T1 lock** on exact mapping; analyst default trivial.

- **R1.4** — `LlmForecasterError` enum:
  `Provider(LlmError)` (wraps `llm::LlmError`),
  `BudgetExceeded` (R5 budget gate),
  `ReplayMiss(String)` (replay-cache miss in research mode),
  `InvalidResponse(String)` (R3 schema-validation failure on the
  forecast tool-use payload),
  `ReflectionStoreError(reflection::ReflectionStoreError)` (top_k
  failure),
  `Timeout` (per-tick LLM wall-clock budget exceeded; Q5b sub-decision).

- **Acceptance:** `cargo build -p strategy` clean; unit test asserts
  every `Rating` round-trips through `to_signal_overlay()`; unit test
  asserts `LlmForecast` is `serde::Serialize` (R7 audit-emission).

### R2 — `ForecastContext` (LLM input)

- **R2.1** — Input shape (Q2 default = d, all-of-the-above):
  ```rust
  pub struct ForecastContext {
      pub symbol: Symbol,
      pub now: DateTime<Utc>,
      pub recent_bars: Vec<Ohlcv>,           // raw 5-feature OHLCV window per Q2a
      pub indicators: TechnicalIndicators,   // summarised technicals per Q2b
      pub top_k_lessons: Vec<LessonCard>,    // reflection-memory top_k per Q2c
      pub recent_decisions: Vec<DecisionRef>,// last N audit rows for context
      pub correlation_id: Uuid,
  }
  ```
  - Q2=(d) all-of-the-above is **analyst-recommended** because (a)
    alone re-litigates v2.5 task framing; (b) + (c) are the
    differentiator. Q2=(a) raw OHLCV only rejected on F-verdict
    grounds (the LLM should not be asked to repeat what 2 DL
    paradigms already F4'd at).
  - **Pre-call prompt-token budget**: target < 8k tokens total per
    forecast tick (analyst-strawman). Architect M-T1 to bench
    actual token counts via Anthropic's `count_tokens` per R5.2.

- **R2.2** — `TechnicalIndicators` shape — re-uses existing
  [`crates/forecast/src/features.rs`](../../crates/forecast/src/features.rs)
  5-feature window verbatim where possible; adds derived RSI / MACD /
  Bollinger / vol-of-vol per
  [product.md § Technical indicator suite line 212-217](../product.md#technical-indicator-suite-v0).
  **Architect M-T1 lock** on exact indicator set;
  analyst-strawman = {RSI(14), MACD(12,26,9), BB(20,2), ATR(14),
  realized_vol_24h, vol_of_vol_7d}.

- **R2.3** — `top_k_lessons` retrieval (Q3 default = c, hybrid):
  - Query the existing `reflection::retrieve_top_k(store, query, k=5)`
    ([`crates/reflection/src/retrieval.rs:22`](../../crates/reflection/src/retrieval.rs))
    on a `RetrievalQuery` derived from `(symbol, regime_tag,
    recent_outcome)`. Default K = 5 per
    [`crates/reflection::REPORT_TIME_TOP_K`](../../crates/reflection/src/lib.rs:69)
    (existing constant).
  - Q3=(c) hybrid composition: cards are top-K by similarity AND a
    1-paragraph distilled summary of the last N days of
    lesson-card-cluster activity (if the
    `reflection-memory-distillation` follow-on has shipped; if not,
    fall back to (a) top-K only and log a `tracing::info!` breadcrumb).
  - Q3=(a) top-K only is the safe v0.1.0 default; Q3=(b) full
    ledger rejected on cost grounds (token-cost-blow-up; survey
    K-llm-2 line 491).

- **R2.4** — Backtest-time `ForecastContext` is **deterministic**:
  every field derives from `recent_bars` (deterministic) +
  `top_k_lessons` (deterministic given a seeded store snapshot —
  see R6 replay-cache).
  - **K1 below** tracks the open determinism of `top_k_lessons`:
    if the reflection-store mutates between backtest re-runs, the
    forecasts mutate. R6 replay-cache mitigates by pinning the
    `(prompt_hash, response)` pair regardless of upstream context
    changes; tester verifies via re-run byte-identity.

- **Acceptance:** unit test asserts `ForecastContext::from_runtime(
  symbol, now, runtime)` produces a deterministic context given
  fixed inputs; integration test asserts two re-runs of the same
  backtest produce byte-identical `ForecastContext` payloads.

### R3 — `LlmForecaster` impl: prompt + tool-use schema

- **R3.1** — `crates/strategy/src/llm_forecaster/anthropic_impl.rs`
  (default impl over `Arc<dyn llm::LlmProvider>`):
  ```rust
  pub struct LlmForecasterImpl {
      provider: Arc<dyn llm::LlmProvider>,
      store: Arc<dyn reflection::ReflectionStore>,
      prompt_builder: Arc<llm::CachedSystemPromptBuilder>,
      tool_schema: llm::ToolSchema,  // R3.3
      config: LlmForecasterConfig,
  }
  ```

- **R3.2** — System prompt structure (composed via
  [`llm::CachedSystemPromptBuilder`](../../crates/llm/src/prompt_cache.rs)):
  - **Project context** (cached, ~800 tokens) — "You are the
    `llm_forecaster` agent inside a Rust crypto trading binary.
    The audit ledger is double-entry SQLite; your forecasts feed
    the strategy crate; the reflection memory persists every closed
    trade as a lesson card you can retrieve."
  - **Role context** (cached, ~1200 tokens) — "Forecast the next-1h
    direction of the given symbol using the five-tier rating scale
    (STRONG_SELL | SELL | HOLD | BUY | STRONG_BUY) with confidence
    in [0, 1]. Use the provided recent OHLCV bars, technical
    indicators, and retrieved lesson cards from past trades. Emit
    your reasoning trace; cite which lesson cards (if any) influenced
    the decision. Call the `propose_forecast` tool."
  - **Per-call dynamic** (NOT cached) — `ForecastContext` rendered
    as a structured block (JSON or markdown table; architect M-T1
    decides — analyst-strawman = markdown for human-readability).

- **R3.3** — Tool-use schema (R5.1-R5.3 from v2-llm-strategy
  applied here):
  ```rust
  ToolSchema {
      name: "propose_forecast",
      description: "Emit a forecast for the given symbol.",
      input_schema: json!({
          "type": "object",
          "required": ["rating", "confidence", "horizon", "reasoning_trace"],
          "properties": {
              "rating": { "enum": ["STRONG_SELL","SELL","HOLD","BUY","STRONG_BUY"] },
              "confidence": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
              "horizon": { "enum": ["short","medium","long"] },
              "reasoning_trace": { "type": "string", "minLength": 50, "maxLength": 2000 },
              "cited_lesson_ids": { "type": "array", "items": { "type": "string" } }
          }
      }),
  }
  ```
  Schema validation per
  [`llm/src/tools.rs` R5.3](../../crates/llm/src/tools.rs) — malformed
  tool-use input → `LlmError::InvalidResponse(msg)` → wrapped as
  `LlmForecasterError::InvalidResponse`.

- **R3.4** — `temperature` pinned at `Some(0.0)` for backtest mode;
  `seed` (where supported — Anthropic does not, OpenAI does) pinned
  at a config constant. **H4 falsification** below tests byte-
  identity.

- **Acceptance:** unit test asserts the composed `ChatRequest` has
  exactly 2 cache breakpoints (project + role boundaries); unit
  test asserts the schema validates a known-good response and
  rejects a known-bad one; integration test with `wiremock` mocks
  an Anthropic `propose_forecast` tool-use response and asserts
  the `LlmForecast` round-trips through Q1=(a) decode.

### R4 — Strategy consumer shape

> Q4 — operator-decide on (a) / (b) / (c) / (d). Defaults below
> assume the analyst-recommended hybrid **(a) + (c)** — standalone
> strategy + Phase F Assistant slot consumer; (b) overlay deferred
> to v0.2.0; (d) all-three-as-opt-in-builders is the v0.2.0+ end-state.

- **R4.1 — Q4=(a) Standalone Strategy** (analyst-recommended for
  v0.1.0). New `LlmForecasterStrategy: Strategy` at
  `crates/strategy/src/llm_forecaster/strategy.rs`. Emits a
  `Signal` per bar derived from `LlmForecaster::forecast()`. The
  strategy is **registered** in
  [`crates/strategy/src/registry.rs`](../../crates/strategy/src/registry.rs)
  under name `"llm_forecaster_v3"`; opt-in via
  `config/agent.toml [[strategies]] kind = "llm_forecaster_v3"`.

- **R4.2 — Q4=(b) Overlay on v1 momentum (DEFERRED to v0.2.0)**.
  The overlay-on-momentum pattern mirrors
  [`crates/strategy/src/tcn_overlay_momentum.rs`](../../crates/strategy/src/tcn_overlay_momentum.rs).
  Defer because: (i) standalone Q4=(a) needs a verdict first
  (does the LLM produce a positive Sharpe-delta *alone* before
  combining); (ii) overlay composition adds variance to the
  evidence read.

- **R4.3 — Q4=(c) Phase F Assistant slot consumer** (analyst-
  recommended for v0.1.0 — parallel deliverable to R4.1). Promotes
  the
  [Phase F Assistant slot](../ui-rethink-phase-f-memory-models-assistant/feature.md#r3--right-rail-assistant-slot-lumen-phase-6)
  from "Assistant offline" placeholder to a live LLM reasoning
  trace view:
  - At each `LlmForecaster::forecast()` call, the `reasoning_trace`
    + `cited_lesson_ids` + `cost_event` post to a new
    `Message::AssistantReasoningTraceUpdate(payload)` variant on
    the cockpit message bus.
  - The Assistant slot body renders the most-recent trace + a
    scrollable history of the last N (~20) traces.
  - **R3.2 wake condition is the operator-decide signal that the
    body lights up** — at v0.1.0 the body remains placeholder UNLESS
    Q4=(c) operator-routes "yes light the slot". This is **the
    biggest product-differentiation surface in C5**: the operator
    *sees* the reasoning, retrievable lesson cards, and audit-row
    correlation IDs as the strategy runs.

- **R4.4 — Q4=(d) All-three-as-opt-in-builders (DEFERRED to v0.2.0+)**.
  The end-state where Q4=(a) + (b) + (c) are independent registered
  builders the operator composes via config. Deferred because each
  builder ships independently; trying to ship all 3 at v0.1.0
  blows past the survey's 5-9 week estimate.

- **Acceptance for v0.1.0 (Q4 = a + c):**
  `cargo test -p strategy --features llm-forecaster` clean;
  `cargo run --bin cockpit-live -- --strategy llm_forecaster_v3`
  emits forecasts to audit + Assistant slot; e2e test asserts a
  forecast tick (i) emits to audit, (ii) renders in Assistant slot,
  (iii) consumes a top_k retrieval, (iv) increments
  `CostBudget` per R5.

### R5 — Cost budget gate (mandatory)

> Inherits R4 from
> [v2-llm-strategy](../v2-llm-strategy/feature.md#r4--budget-enforcement-gate).
> Every LLM call routes through
> [`crates/llm::BudgetedProvider`](../../crates/llm/src/budgeted.rs)
> — the gate is shipped; this section pins how C5 consumes it.

- **R5.1** — `LlmForecasterImpl::provider` is wrapped via
  `llm::BudgetedProvider<Inner>` at construction time
  ([`llm::LlmProviderFactory::build`](../../crates/llm/src/factory.rs)
  already does this when `cfg.llm.enabled = true`).

- **R5.2** — **Pre-call token-budget estimate** uses Anthropic's
  `count_tokens` endpoint (or `tiktoken` rules for OpenAI). Target
  per-call budget ≤ 8k input tokens + 1k output tokens (analyst-
  strawman). Architect M-T1 to bench actual numbers via a
  `cargo run --bin llm-forecaster-bench` ahead of the first real
  training-cost projection.

- **R5.3** — **Hard cost ceiling per backtest** =
  `config.llm_forecaster.cost_cap_usd_per_backtest` (default $20;
  inside the v2 LLM ceiling $200/month per
  [product.md line 343](../product.md#cost-economics--monthly-ceiling)).
  Exceeding triggers `LlmForecasterError::BudgetExceeded` propagated
  to the backtest binary, which short-circuits with an explicit
  error log.

- **R5.4** — **N-bar batching** mitigation (per survey K-llm-2
  line 491) — `config.llm_forecaster.fire_every_n_bars` (default
  24 — fires once per day on hourly bars). Between firings, the
  strategy carries forward the last `LlmForecast` as its current
  signal. Default 24 keeps cost ~ $1.50/symbol-day at deep-think
  Claude Opus pricing (analyst-strawman; H2 below falsifies).

- **R5.5** — **Auto-degrade at 80%** — inherits
  [v2-llm-strategy R4.1 + R4.4](../v2-llm-strategy/feature.md#r4--budget-enforcement-gate)
  verbatim — when monthly spend ≥ 80%, deep-think tier downgrades
  to quick-think (Claude Haiku); model remap reads
  `config.llm.quick_think`. At 100% spend, calls block and the
  strategy emits `Signal::Hold` with a `tracing::warn!`
  breadcrumb.

- **Acceptance:** unit test asserts a fire-every-24-bars
  strategy over a 24-bar window produces exactly 1 LLM call (not 24);
  unit test asserts a budget-exhausted `forecast()` call returns
  `BudgetExceeded` without an outbound HTTP request (mock asserts
  no call recorded); integration test asserts a 100-bar fire-every-1
  scenario produces ≤ N calls where N matches the budget cap.

### R6 — Determinism contract (load-bearing for anchored backtests)

> **Q5 = (b) — Replay-cache pattern.** Analyst-recommended.
> Inherits
> [`crates/llm::RecordingProvider`](../../crates/llm/src/recording.rs)
> + [`ReplayProvider`](../../crates/llm/src/replay.rs) — the
> sqlite-backed `(request_hash, response)` cache already shipped
> at v2.0.0. C5 extends *usage* to backtests; no new infra.

- **R6.1** — `temperature = 0.0` pinned per R3.4. **Necessary but
  not sufficient** — Anthropic API at `temperature=0` is not
  byte-deterministic across server restarts (model weights can
  shift between deploys; outputs drift). Q5=(a) "rely on
  `temperature=0` + `seed`" rejected as insufficient evidence base.

- **R6.2** — `RecordingProvider` writes every call to
  `data/llm-replay.db` (live mode) or
  `data/llm-forecaster-replay.db` (separate namespace; architect
  M-T1 to decide); `ReplayProvider` reads back. Backtest mode
  (`config.mode = "research"`) uses `ReplayProvider`; cache miss
  → `LlmError::ReplayMiss(hash)` → `LlmForecasterError::ReplayMiss`
  → backtest binary short-circuit with an explicit error log.

- **R6.3** — The `(request_hash, response)` cache is **anchored
  inline** with the backtest report. The first paper-trading run
  populates the cache; subsequent backtest re-runs read from cache
  → byte-identical `reasoning_trace` + `cited_lesson_ids` + `rating` +
  `confidence` → byte-identical backtest report → anchor stays
  byte-identical.

- **R6.4** — **Re-recording protocol**: the operator explicitly
  invokes `cargo run --bin llm-forecaster-rerecord -- --scenario
  top10-2023-fy-llm-forecaster-realdata` to refresh the cache.
  Re-recording is **destructive** to the anchor (the body-SHA-256
  changes); the rerecord binary emits a `MIGRATION` warning per
  [v25-tcn-overlay precedent](../v25-tcn-overlay/feature.md).

- **R6.5** — **Cache versioning**: `(request_hash, response)` cache
  rows carry a `cache_schema_version` field. Bumping the schema
  invalidates the anchor (architect M-T1 owns the migration shape).

- **R6.6** — `ForecastContext::request_hash()` is the canonical
  SHA-256 over the prompt body — symbol + bars + indicators +
  lessons + model_id + temperature + seed. **Architect M-T1 lock**
  on canonical serialisation (analyst-strawman: serde_json with
  sorted keys; identical-by-bytes input → identical SHA).

- **Acceptance:** integration test runs a backtest twice on a
  freshly-rebuilt cache; asserts byte-identical report bodies
  (`scripts/hash_report.py` returns the same SHA both runs);
  integration test mutates one input bar and asserts the cache
  miss is detected and `ReplayMiss` surfaces.

### R7 — Audit-ledger emission (every LLM call → audit row)

- **R7.1** — Every `LlmForecaster::forecast()` call emits:
  1. A `CostEvent::Llm` row to
     [`crates/cost/src/event.rs`](../../crates/cost/src/event.rs)
     — already wired via `BudgetedProvider`.
  2. A new `JournalEntry { kind: "llm_forecast", payload: ... }`
     row to
     [`crates/audit/`](../../crates/audit/)
     — payload = serialised `LlmForecast` (R1.2). **Architect M-T1
     lock** on the journal-entry SQL row shape (additive migration
     # 011 or later).
  3. A `Message::AuditTick` ride-along per
     [Phase D audit-tick stream](../ui-rethink-phase-d-trail/feature.md#r4--audit-tick-stream)
     so the Phase D Trail view + Phase F Assistant slot + Phase F
     Memory screen all see the row live.

- **R7.2** — `LlmForecast::reasoning_trace_sha256` (R1.2) is
  anchor-stable — the body-SHA over the reasoning trace text. This
  enables a future audit-ledger query
  ("which trace bodies fired the most"); architect M-T1 to decide
  the index shape.

- **R7.3** — **Audit-ledger query helpers** (DEFERRED to v0.2.0):
  - `audit::query::llm_forecasts_by_symbol(symbol, since, until)`.
  - `audit::query::llm_forecasts_by_rating(rating)`.
  Architect M-T1 to scope.

- **Acceptance:** integration test asserts a `forecast()` call
  produces exactly 1 `CostEvent` row + 1 `JournalEntry` row + 1
  `AuditTick` broadcast.

### R8 — Backtest scenarios + report shape

- **R8.1** — New backtest scenarios:
  - `top10-2023-fy-llm-forecaster-realdata` — full year 2023
    realdata, 10 USDT pairs (analyst-strawman; architect M-T1
    confirms).
  - `top10-2024-fy-llm-forecaster-realdata` — full year 2024
    realdata.
  - `top10-2023-fy-llm-forecaster-overlay-on-v1-realdata`
    (DEFERRED to v0.2.0 — depends on Q4=(b) overlay shape).

- **R8.2** — Report shape mirrors
  [v25a-patchtst-overlay R8](../v25a-patchtst-overlay/feature.md)
  — markdown with frontmatter holding generated-at / git-commit /
  wall-clock; body is the deterministic anchored part. New columns:
  - LLM cost USD total + per-call.
  - Cache hit ratio (target ≥ 90% on re-runs).
  - Top-K lesson-card retrieval distribution.
  - `reasoning_trace_sha256` histogram (most-frequent traces).

- **R8.3** — Sharpe-delta comparison vs v1 momentum baseline per
  [v25-tcn-overlay precedent](../v25-tcn-overlay/feature.md). Gate:
  ≥ +0.10 (the +0.10 alpha-unlock threshold; H1).

- **R8.4** — **F-verdict applicability**: ADR-0033 § D3 F-verdict
  algorithm is **LLM-specific N/A** (the F1-F4 priority tree assumes
  a μ-prediction model with `r_hat` distribution). C5 needs a
  **new ADR**: see Q6 below for the analyst-recommended ADR-0038
  shape (accuracy + calibration + cost-per-decision + reasoning-
  trace quality).

- **Acceptance:** backtest binary runs both scenarios end-to-end;
  reports written under
  `spec/v3-llm-forecaster/reports/backtest-top10-2023-fy-llm-forecaster-realdata.md`
  + `…-2024-fy-...md`; Sharpe-delta column present.

### R9 — Phase F Assistant slot promotion (Q4=(c) deliverable)

- **R9.1** — `crates/ui/src/assistant/view.rs` body swaps from
  placeholder to live reasoning-trace renderer (gated by a
  feature flag or runtime config so the v0.2.0 Phase F shipped
  byte-identity is preserved):
  ```rust
  pub enum AssistantMode {
      Offline,         // Phase F v0.1.0 default — placeholder
      ReasoningTrace,  // v0.2.0 + this feature — live LLM trace
  }
  ```
  At v0.1.0 of *this* feature (v3-llm-forecaster), `AssistantMode`
  flips to `ReasoningTrace` when the strategy is enabled.

- **R9.2** — Body composition (top-down):
  - Header line: most-recent forecast summary
    (`<symbol> · <rating> · conf=<confidence>`).
  - Cost line: cumulative LLM spend today / this month / budget cap.
  - Reasoning trace card (markdown; the `reasoning_trace` field of
    `LlmForecast`).
  - Cited lesson cards section (rendered as compact
    `LessonCardCard` widgets — re-uses
    [`crates/ui/src/memory/`](../../crates/ui/src/memory/)
    components from Phase F).
  - Scrollable history of last N (~20) traces.
  - Chevron → opens the audit row for the underlying journal entry
    (re-uses
    [Phase D `Message::OpenTrailFor(audit_id)`](../ui-rethink-phase-d-trail/feature.md)).

- **R9.3** — **R7.2 Phase F preservation** — when the strategy is
  *disabled* (default — opt-in via config), the Assistant slot
  body reverts to the Phase F v0.1.0 placeholder copy. All 22 +
  Phase F snapshot baselines stay byte-identical at default config.

- **Acceptance:** new snapshot baseline
  `assistant_slot__llm_forecaster_active__most_recent_trace`;
  proptest layout-invariants pass for the new mode; the existing
  `assistant_slot__open_stub` baseline stays byte-identical when
  strategy disabled.

### R10 — Non-regression contract

- **R10.1** — **30 body-SHA-256 anchors stay byte-identical**. C5
  is anchor-additive — new `top10-2023-fy-llm-forecaster-realdata`
  + `top10-2024-fy-llm-forecaster-realdata` anchors lock under a
  new version pin `v3.0.0-llm-forecaster`. The 30 existing anchors
  are untouched.

- **R10.2** — **v25-tcn / v25a-patchtst / v0 / v0.5 / v1 strategies
  byte-identical** — C5 adds a new strategy; no existing strategy
  body is touched. Asserted via test
  `crates/strategy/tests/llm_forecaster_neutrality.rs` (NEW —
  re-runs `top10-2023-fy-tcn-overlay-realdata` and asserts the
  body-SHA `8fa47f49…` is unchanged after the new strategy is
  registered).

- **R10.3** — **Phase F v0.1.0 surfaces byte-identical at default
  config** — Memory + Models screens unchanged; Assistant slot
  body reverts to placeholder when strategy disabled. R9.3 above.

- **R10.4** — **No new external crate deps** (the `llm` + `reflection`
  + `audit` + `cost` crates are all shipped; `crates/strategy/`
  Cargo.toml gains 4 path-deps).

- **R10.5** — **No iced bump**; the vendored `iced_tiny_skia` fork
  stays untouched per
  [CLAUDE.md operator-lock 2026-05-20](../../CLAUDE.md#vendored-dependencies).

- **R10.6** — **`spec-lint` contribution = 0**.

- **R10.7** — **No audit-ledger writer touch** — additive migration
  011 (or later); existing ledger rows untouched.

- **R10.8** — **No reflection-memory writer touch** — C5 is a *read*
  consumer of `top_k`. The lesson-card writer pipeline (Phase F
  predecessor) stays unchanged. The
  `reflection-memory-trader-wiring` follow-up brief (deferred per
  [reflection-memory Q4 line 13-18](../../crates/reflection/src/lib.rs))
  is **superseded by this feature** — C5 IS the trader-wiring.

- **Acceptance:** `scripts/verify_anchors.sh` → 32 / 32 PASS at
  v0.1.0 ship (30 existing + 2 new).

## Q-questions (operator-decide)

### Q1 — Signal shape

(a) **Discrete 5-tier rating + confidence + reasoning trace**.
(b) μ-equivalent forecast as a number (overlap with retired v2.5 task).
(c) Regime label (overlap with C2).
(d) Free-form reasoning + structured directive.

**Analyst-recommended: (a)** — matches
[product.md § Five-tier rating scale line 156](../product.md#five-tier-rating-scale)
verbatim; cleanly serialisable; cleanly anchorable. (b) re-litigates
v2.5 F-verdict territory; (c) overlaps C2; (d) is the v0.2.0+
follow-on.

### Q2 — Input shape

(a) Raw recent OHLCV bars only.
(b) Summarised technical indicators only.
(c) Reflection-memory lesson cards only.
(d) **All of (a) + (b) + (c)** + recent audit decisions.

**Analyst-recommended: (d)** — gives the LLM the differentiator
(memory) + the standard quant primitives (technicals) + the raw data
(OHLCV) + the operator's recent decisions. (a) alone is the v2.5
task framing the F-verdict already rejected; (b) alone is weaker;
(c) alone is too memory-myopic.

### Q3 — Memory consumption shape

(a) Top-K retrieval from reflection-memory store (default K = 5
    per `REPORT_TIME_TOP_K`).
(b) Full reflection ledger as prompt context (cost concern; rejected
    per survey K-llm-2 line 491).
(c) **Summarised + top-K hybrid** (top-K cards + distilled summary
    if distillation has shipped, else fallback to top-K only).

**Analyst-recommended: (c)** — best signal density per token; if
distillation hasn't shipped (currently it hasn't —
[`reflection/src/lib.rs:20-24`](../../crates/reflection/src/lib.rs)
gates distillation as a follow-up brief), fall back to (a) and log.

### Q4 — Consumer shape (load-bearing for differentiation)

(a) **Standalone Strategy** emitting Signal (`LlmForecasterStrategy`).
(b) Overlay on existing momentum (mirrors v2.5 TCN overlay).
(c) **Advisory in the Phase F Assistant slot** (UI surface — promotes
    the slot body from placeholder to live reasoning trace).
(d) All three as opt-in builders (v0.2.0+ end-state).

**Analyst-recommended: (a) + (c) hybrid** — standalone strategy
emits the signal AND promotes the Phase F Assistant slot to render
the reasoning trace. **The Assistant slot integration is the
highest product-differentiation surface in C5** — it's where the
operator *sees* the LLM reasoning + retrieved lessons + audit
correlation live. (b) overlay defers to v0.2.0 (needs a positive
verdict on (a) first). (d) all-three-as-builders is the end-state
once all three are battle-tested.

### Q5 — Determinism contract

(a) Require `temperature = 0` + fixed seed + prompt-cache-hit
    (insufficient — Anthropic deploys can drift across server
    restarts).
(b) **Build a replay-cache** around LLM calls (mirror of
    [`crates/replay-cache/`](../../crates/replay-cache/) +
    [`crates/llm::RecordingProvider`](../../crates/llm/src/recording.rs)
    already shipped — extends usage to backtests).
(c) Accept non-determinism, anchor on aggregate Sharpe over
    multi-seed Monte Carlo.

**Analyst-recommended: (b)** — `temperature = 0` alone is necessary
but not sufficient; the replay-cache pattern is already shipped at
v2-llm-strategy v2.0.0 (`RecordingProvider` + `ReplayProvider` per
[v2-llm R6](../v2-llm-strategy/feature.md#r6--recordreplay-for-research-mode));
C5 extends *usage* (not infra). (c) is the fallback if (b) fails
H4 falsification.

**Sub-decision Q5b — per-call wall-clock timeout**: default
`config.llm_forecaster.timeout_ms = 30_000` (30s per forecast tick;
analyst-strawman). Architect M-T1 to refine.

### Q6 — Verdict shape (NEW ADR likely)

ADR-0033 § D3 F-verdict algorithm is **LLM-specific N/A** (assumes
μ-prediction model with `r_hat` distribution). C5 needs a new
verdict shape:

(a) Re-use ADR-0033 F-verdict adapted to map rating-distribution to
    F1-F4 priorities (forced fit; analyst NACK).
(b) **New ADR-0038 "LLM-forecaster verdict criteria"** with new
    priorities:
    - **L1** = Bias collapse — model produces ≥ 95% HOLD ratings
      (no opinion; analogue of v2.5 F1 "collapse to zero").
    - **L2** = Calibration failure — confidence values don't
      correlate with realized outcome (e.g. high-confidence
      forecasts no more accurate than low-confidence; analogue of
      F2 calibration).
    - **L3** = Cost overrun — actual cost per backtest > 2× projected
      budget (no analogue in F-verdict; new for C5).
    - **L4** = Reasoning trace degenerate — traces are < 50 chars,
      or duplicate across > 50% of calls (no analogue in F-verdict;
      new for C5).
    - **L0** = PASS — none of L1-L4 trigger; Sharpe-delta ≥ +0.10.
(c) No new ADR; track verdict criteria inline in the report.

**Analyst-recommended: (b)** — ADR-0038 lock; the L1-L4 priorities
are LLM-specific and worth codifying once across future LLM-strategy
ships. Architect M-T1 owns the ADR draft.

### Q7 — Anchor strategy

(a) Anchors `top10-2023-fy-llm-forecaster-realdata` +
    `top10-2024-fy-llm-forecaster-realdata` under new version pin
    **`v3.0.0-llm-forecaster`**.
(b) Anchors under `v2.x` (re-use v2.5a.0-patchtst-style versioning).
(c) Skip anchors at v0.1.0; anchor only after positive Sharpe-delta
    verdict.

**Analyst-recommended: (a)** — new version pin signals the v3 era
(per [product.md § Strategy library line 184](../product.md#strategy-library--roadmap)
v3 = RL policy historically; C5 redefines v3 as LLM-as-forecaster).
Anchor risk = MEDIUM (LLM determinism + replay-cache must lock
correctly per H4). Anchors lock at tester M-FINAL after H4
byte-identity holds across 3 re-runs.

### Q8 — Relationship to `v2x-trading-state-bus`

(a) Build C5 as **part of** `v2x-trading-state-bus` refactor — the
    `TradingState { fundamentals, sentiment, news, technical,
    debate, ... }` struct from
    [backlog Queue § Process line 580-591](../backlog.md) becomes
    the substrate for `ForecastContext`.
(b) Build C5 **standalone** for v0.1.0; refactor to `TradingState`
    in v0.1.1 once both ship.

**Analyst-recommended: (b)** — `v2x-trading-state-bus` is a separate
queue item with its own analyst-spawn timeline. Coupling C5 to it
would block C5 on a refactor the operator hasn't promoted.
Standalone v0.1.0 ships `ForecastContext` (R2.1) as a concrete
struct; v0.1.1 lifts to `TradingState` if both ship. **Open
question for orchestrator**: if `v2x-trading-state-bus` is
promoted ahead of C5, the precedence flips — C5 then depends on
`TradingState`'s shape. Surface this as a sequencing decision at
operator-decide time.

## K-risk register

### K1 — Reflection-store top_k determinism under backtest re-runs
**Risk:** `reflection::retrieve_top_k(store, query, k)` is
deterministic given a fixed store state, but the store mutates
between backtests (paper-trading writes new lesson cards). Two
backtests run on different days against a live store will see
different `top_k_lessons` → different `ForecastContext` → different
LLM responses → different anchors.
**Severity:** HIGH at v0.1.0 if R6 replay-cache misses; LOW once
R6 cache populates.
**Mitigation:** R6 replay-cache pins the `(prompt_hash, response)`
pair regardless of upstream context changes — once cached, replay
returns the same response even if `top_k_lessons` changes. The
*prompt_hash* IS the determinism-anchor; as long as
`ForecastContext::request_hash` is canonically serialised (R6.6),
re-runs hit cache. Tester verifies H4 byte-identity across 3 re-runs.
**Open question for architect M-T1:** should the backtest binary
take a `--reflection-store-snapshot` flag that pins the store state
to a frozen sqlite dump? Analyst-strawman: yes, for safety;
defer the implementation to architect.

### K2 — LLM cost blow-up at fine cadence
**Risk:** Naive per-bar firing on 10 symbols × 8760 hourly bars/year
= 87,600 LLM calls/year. At deep-think Claude Opus pricing
(~$0.015/call with 8k input + 1k output cached), that's ~$1,314/year
per backtest run — well above the v2 ceiling $200/month.
**Severity:** HIGH if R5.4 N-bar batching default is wrong.
**Mitigation:** R5.4 fire-every-N-bars (default N = 24 →
~3,650 calls/year/symbol = ~$55/year). Prompt caching collapses
the input-token cost ~ 4× (per Anthropic's 75% discount on cache
hits). N=24 hourly = once-per-day on a 1-week horizon ratings
strategy — analyst's read is this is the sweet spot for
"meaningful trading frequency" vs cost. Architect M-T1 to bench
actual costs at H2 falsification time.

### K3 — Reasoning trace quality is subjective + unanchorable
**Risk:** The `reasoning_trace` text is what the operator reads in
the Phase F Assistant slot — but its quality is subjective
("does this trace help me trust the strategy?"). Unlike F-verdict
which is mechanically deterministic, trace quality is operator-judged.
**Severity:** MEDIUM (UX + product-differentiation; not a
correctness risk).
**Mitigation:** ADR-0038 L4 priority (Q6=(b)) defines mechanical
degeneracy gates (< 50 char traces; > 50% duplicate). Beyond that,
the operator reviews trace quality at presenter time. Acceptance
criteria H3 below is explicitly subjective — operator-judged.

### K4 — Anthropic API non-determinism across server deploys
**Risk:** Even with `temperature = 0`, Anthropic re-deploys can
shift model outputs (rare; documented). If a re-deploy happens
mid-backtest cache build, the cache contains pre-deploy responses;
post-deploy fresh calls return drift. R6 replay-cache mitigates
once populated, but the **first** cache-build run is exposed.
**Severity:** MEDIUM at first-cache-build; LOW once cache populates.
**Mitigation:** R6.4 re-recording protocol — explicit operator
gate. Architect M-T1 to decide: (i) anchor only after 3
back-to-back identical cache-build runs (analyst-recommended); (ii)
accept drift and re-anchor on every Anthropic model-version bump
(operationally heavy).

### K5 — Replay-cache size + checkout-friendliness
**Risk:** A full-year backtest at N=24 fire cadence = 3,650 cache
rows/symbol × 10 symbols = 36,500 rows. Each row ≈ 2-4 KB
(request_hash + reasoning_trace + JSON) → ~100-150 MB cache.
Git-checked-in is non-starter; runtime-only is fine but breaks
fresh-checkout determinism for CI/tester.
**Severity:** MEDIUM (operational).
**Mitigation:** Architect M-T1 decides:
  - (i) Cache lives at `data/llm-forecaster-replay.db`, git-ignored;
    fresh-checkout cold-runs populate over hours and cost actual
    LLM dollars.
  - (ii) Cache lives at `crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz`
    (git-LFS or split), checked in. Fresh-checkout determinism
    preserved.
  - (iii) Cache lives off-repo at S3/B2; bootstrap script
    `scripts/bootstrap_llm_replay.sh` fetches. Adds cloud spend
    (rejected per product.md no-cloud-spend operator-lock per
    [DR / backups line 438](../product.md#open-decisions)).
**Analyst-recommended: (ii)** — cache checked in (compressed); same
pattern as
[`crates/llm/tests/fixtures/llm-replay.db`](../../crates/llm/tests/fixtures/)
already shipped at v2-llm-strategy v2.0.0.

### K6 — Q6 ADR-0038 scope creep
**Risk:** Authoring ADR-0038 (LLM-forecaster verdict criteria L1-L4)
risks bikeshedding the priority tree. Architect M-T1 may surface
edge cases the analyst-strawman doesn't cover.
**Severity:** LOW (process).
**Mitigation:** Analyst-strawman L1-L4 priorities are minimal
viable; architect M-T1 owns the ADR draft. If the ADR draft adds >
2 new priorities, surface as operator-decide before locking.

### K7 — Phase F Assistant slot promotion couples C5 to UI ship cadence
**Risk:** Q4=(c) promotes the Phase F Assistant slot body. If C5
ships before the operator approves the slot-body promotion, the
slot body stays at placeholder; the highest product-differentiation
surface is invisible.
**Severity:** MEDIUM (product, not code).
**Mitigation:** R9.3 — Assistant slot body is **runtime-gated** by
strategy-enabled flag. Default (strategy disabled) keeps the
placeholder per Phase F v0.1.0 ship. The operator opts in at
config-edit time; no UI ship coordination needed. The Q4=(c)
deliverable is the body composition (R9.2); the actual operator-
visible promotion is config-driven.

### K8 — Survey 5-9 week cost estimate variance
**Risk:** Survey § Candidate 5 line 480 estimates **5-9 weeks**
total wall-clock. The operator-locked Q-BUDGET is **6-8 weeks
total** for C1 + C2 + C5 combined — C5 alone could blow past
budget.
**Severity:** HIGH (schedule).
**Mitigation:** This brief is **spec-only design exploration**
(per operator Q-SEQ HYBRID line of the operator-decide context).
Architect M-T1 + developer waves DEFERRED. If/when C1 ships and
operator promotes C5, the architect M-T1 pass refines the cost
estimate; if it exceeds 4 weeks for the dev impl alone, surface as
operator-decide before kicking off Wave A.

### K9 — Sequencing dependency on C1 verdict
**Risk:** Operator Q-SEQ HYBRID — C5 spec authored now, code
deferred until C1 ships its verdict. If C1 ships POSITIVE
(+0.10 Sharpe-delta), the freed analyst/dev bandwidth may stay
allocated to C1 follow-ons (e.g. vol-targeting+regime ensemble)
rather than C5. C5 could stay in `draft` indefinitely.
**Severity:** LOW (operator-decide; spec stays useful as a sleep
reference).
**Mitigation:** This brief is durable spec. If C5 never promotes,
the spec captures the design exploration cost (~2-3 weeks of
analyst pass time) which is **information-bearing** regardless —
the survey + this brief together are the canonical answer to
"could we forecast via LLM?"

### K10 — `v2x-trading-state-bus` sequencing ambiguity
**Risk:** Per Q8, C5 may be built standalone (default) or as part
of the v2x state-bus refactor. If operator promotes
`v2x-trading-state-bus` BEFORE C5, this brief's `ForecastContext`
(R2.1) needs to be refactored to `TradingState` substrate — a
non-trivial migration.
**Severity:** MEDIUM (sequencing).
**Mitigation:** Surface Q8 explicitly at operator-decide. If
operator pre-commits to `v2x-trading-state-bus` first, this brief's
R2.1 is amended to use `TradingState` directly. Architect M-T1
owns the refactor decision at promotion time.

## H-hypothesis register

### H1 — +0.10 Sharpe-delta vs v1 baseline (alpha gate)
**Claim:** A `LlmForecasterStrategy` consuming `ForecastContext`
(R2.1) per Q1=(a) discrete rating produces ≥ +0.10 Sharpe-delta vs
the v1 cross-sectional momentum baseline on
`top10-2023-fy-llm-forecaster-realdata` + `…-2024-fy-…` realdata
backtest scenarios.
**Falsification:** R8 backtest reports Sharpe-delta column < +0.10
on either scenario. Per survey LOW-MEDIUM prior — could be 0 or
0.2+. Tester locks the report-bytes anchor at M-FINAL after H4
byte-identity.
**Why this number:** the +0.10 alpha-unlock threshold has been the
canonical bar since v2.5 TCN per
[v25-tcn-overlay feature.md](../v25-tcn-overlay/feature.md);
v2.5 TCN BS-1/BS-2 hit +0.018 / +0.045; v2.5a PatchTST hit +0.006;
all F4. C5's prior is higher-variance — could be 0 (LLM
produces noise) or > +0.20 (lesson-card-aware analyst finds
patterns DL missed).

### H2 — LLM cost per backtest < $50
**Claim:** A full-year backtest at N=24 fire cadence (R5.4 default)
on 10 symbols costs < $50 in LLM tokens (well inside v2 ceiling
$200/month per
[product.md line 343](../product.md#cost-economics--monthly-ceiling)).
**Falsification:** Architect M-T1 + dev waves bench actual cost via
`cargo run --bin llm-forecaster-bench` on a 1-month slice; project
to full-year. If projection > $50, R5.4 default N bumps to 168
(weekly cadence; ~$8/year) or the deep-think tier downgrades.
**Why this number:** at Claude Opus pricing (~$0.015/call with
cache; ~$0.005/call cache-hit), 3,650 calls/symbol/year × 10
symbols ≈ $55 cold-run; with > 90% cache hit, ≈ $10-15
warm-run. $50 is the cold-run ceiling.

### H3 — Operator reviews trace quality as "trust-bearing"
**Claim:** At presenter time, the operator judges the rendered
reasoning traces in the Phase F Assistant slot (R9) as "useful for
understanding why the strategy traded." Subjective; operator-judged.
**Falsification:** Operator reads 10-20 sample traces and judges
< 50% as understandable. Mitigation: ADR-0038 L4 (Q6=(b)) gates
trace degeneracy mechanically; UX gates beyond that are
operator-decide.
**Why this matters:** H3 IS the differentiator gate. If the
operator can't read the traces, the moat is invisible and C5 has
extracted alpha but not narrative.

### H4 — Replay-cache produces byte-identical backtests
**Claim:** Running the same backtest scenario twice (against a
populated replay-cache) produces byte-identical report bodies
(`scripts/hash_report.py` returns the same SHA both runs).
**Falsification:** Tester runs the scenario twice; SHAs differ.
Diagnose: (i) the request_hash canonicalisation has order-
dependence (R6.6 fix); (ii) the response decoding has timing-
dependent ordering (architect M-T1 audits); (iii) the prompt
includes a timestamp or other run-varying value (R3 fix).
**Why this matters:** H4 is the anchor-precondition gate. Without
H4, anchors can't lock; without anchors, the 30-anchor regression
gate breaks the v2.5 invariant.

### H5 — 3-5 week dev impl feasibility
**Claim:** Once promoted, the architect M-T1 + developer waves
(A-F) ship in ≤ 4 weeks wall-clock.
**Falsification:** Architect M-T1 ratifies a decomp.md with a
Wave plan totaling > 4 weeks; or developer Wave B trips the
1.5× tripwire (survey Q-BUDGET line 924).
**Why this number:** survey § Candidate 5 line 480 estimates
5-9 weeks total; the analyst pass (this brief) is ~2 weeks (one
analyst); remaining 3-7 weeks across architect + dev + tester.
H5 budget is 4 weeks for dev impl alone; tighter than survey
median.

## Non-regression contract

1. **30 body-SHA-256 anchors stay byte-identical** (R10.1). New
   anchors `top10-2023-fy-llm-forecaster-realdata` +
   `top10-2024-fy-llm-forecaster-realdata` lock under
   `v3.0.0-llm-forecaster` at tester M-FINAL.
2. **All shipped strategies byte-identical** (R10.2). New strategy
   is additive; existing strategy bodies untouched. Asserted via
   `crates/strategy/tests/llm_forecaster_neutrality.rs`.
3. **Phase F v0.1.0 surfaces byte-identical at default config**
   (R10.3, R9.3). Assistant slot body reverts to placeholder when
   strategy disabled.
4. **No new external crate deps** (R10.4).
5. **No iced bump** — `iced_tiny_skia` fork stays untouched per
   CLAUDE.md operator-lock 2026-05-20 (R10.5).
6. **`spec-lint` contribution = 0** (R10.6).
7. **No audit-ledger writer touch** — additive migration only
   (R10.7).
8. **No reflection-memory writer touch** — read-only consumer
   (R10.8). C5 supersedes the deferred
   `reflection-memory-trader-wiring` follow-up.

## Acceptance criteria

### M0 — Analyst synthesis (this pass)
- [x] R1..R10 anchored to survey § Candidate 5 (lines 432-537) +
      operator Q-PICK/Q-SEQ/Q-BUDGET context (2026-05-22).
- [x] Q1-Q8 surfaced with analyst-recommended defaults.
- [x] K1-K10 risk register; K1 (top_k determinism) + K4 (Anthropic
      drift) + K5 (cache-checkout) + K8 (5-9w variance) + K9 (C1
      sequencing dependency) + K10 (`v2x-trading-state-bus`
      sequencing ambiguity) flagged as load-bearing.
- [x] H1-H5 falsifiable hypotheses with falsification protocol per
      each.
- [x] Non-regression contract enumerated (8 items).
- [x] Predecessor crates audited:
      - `crates/llm/` shipped v2.0.0 — `LlmProvider` trait +
        `BudgetedProvider` + `RecordingProvider/ReplayProvider` +
        `CachedSystemPromptBuilder` + `ToolSchema`.
      - `crates/reflection/` shipped v0.1.0 — `top_k` retrieval +
        `LessonCard` writer + Phase F Memory screen.
      - `crates/audit/` Phase D shipped — audit-tick stream + Trail
        view.
      - `crates/ui/src/assistant/` Phase F shipped — slot body
        placeholder ready for promotion.
- [x] Trace row `REQ-V3-LLM-FORECASTER-001` to be opened in
      `draft` state by this pass.
- [x] Backlog Queue § Strategy entry to be added (NOT Active).
- [x] Tasks.md analyst T-A1..T-An ordered checklist authored.

### M-OD — Operator-decide (Q1-Q8 + promotion)
- [ ] Q1 — Signal shape (analyst-recommended: a — discrete rating).
- [ ] Q2 — Input shape (analyst-recommended: d — all-of-the-above).
- [ ] Q3 — Memory consumption (analyst-recommended: c — hybrid).
- [ ] Q4 — Consumer shape (analyst-recommended: a + c hybrid).
- [ ] Q5 — Determinism contract (analyst-recommended: b — replay-
      cache + `temperature=0`).
- [ ] Q6 — Verdict shape (analyst-recommended: b — new ADR-0038
      L1-L4).
- [ ] Q7 — Anchor strategy (analyst-recommended: a — new
      `v3.0.0-llm-forecaster` version pin).
- [ ] Q8 — `v2x-trading-state-bus` relationship (analyst-recommended:
      b — standalone v0.1.0, refactor v0.1.1).
- [ ] **Promotion gate**: operator promotes Queue → Active **only
      after** C1 ships its verdict OR explicit early-promotion
      directive.

### M-T1 — Architect decomposition (DEFERRED until promotion)
- [ ] Architect resolves K1 (reflection-store snapshot flag for
      backtest determinism).
- [ ] Architect resolves K2 (LLM cost benchmark + N-bar batching
      tuning).
- [ ] Architect resolves K4 (Anthropic drift policy — analyst-
      recommended: anchor only after 3 back-to-back identical
      cache-build runs).
- [ ] Architect resolves K5 (cache checkout — analyst-recommended:
      check-in compressed cache).
- [ ] Architect drafts ADR-0038 (LLM-forecaster verdict criteria
      L1-L4 per Q6=(b)).
- [ ] Architect resolves Q8 sequencing under operator's promotion
      decision (standalone vs `TradingState` substrate).
- [ ] Architect decomposes R1-R10 into ordered T-D-N tasks per
      wave. Suggested wave map (subject to architect refinement):
      - Wave A = `LlmForecaster` trait + `LlmForecast` payload +
        `ForecastContext` (R1 + R2).
      - Wave B = `LlmForecasterImpl` (R3) + prompt-cache wiring +
        tool-use schema.
      - Wave C = `LlmForecasterStrategy` (R4.1) + registry wiring.
      - Wave D = Backtest scenarios (R8) + replay-cache wiring (R6).
      - Wave E = Audit emission (R7) + cost budget (R5).
      - Wave F = Phase F Assistant slot promotion (R9) + snapshot
        baselines + layout-invariants.
      - Wave G = ADR-0038 + non-regression tests + tester handoff.

### M-FINAL — Tester sweep (DEFERRED until dev waves complete)
- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
      exit 0.
- [ ] `cargo test --workspace --lib` 100% PASS.
- [ ] New snapshot baselines:
      - `assistant_slot__llm_forecaster_active__most_recent_trace`
        (Q4=c body promotion).
      - `assistant_slot__llm_forecaster_disabled__placeholder` (R9.3
        byte-identity guard).
- [ ] `scripts/verify_anchors.sh` → 32 / 32 PASS (30 existing + 2
      new). Non-negotiable (R10.1).
- [ ] `cockpit-smoke` → 0 panic lines on `llm_forecaster_v3`
      enabled config (R10.3).
- [ ] H4 byte-identity test: backtest re-run produces identical
      SHA. Non-negotiable (anchor pre-condition).
- [ ] H2 cost benchmark recorded in test report.
- [ ] H1 Sharpe-delta verdict per ADR-0038 L0-L4 priorities.
- [ ] Author `spec/v3-llm-forecaster/reports/test-final-<YYYY-MM-DD>.md`.

### M-PRESENTER — Operator approval (DEFERRED until tester PASS)
- [ ] Presenter deck enumerates H1-H5 falsification results.
- [ ] Presenter renders 10-20 sample reasoning traces from the
      Phase F Assistant slot for operator H3 trust-judgment.
- [ ] Presenter renders Sharpe-delta + ADR-0038 verdict.
- [ ] Operator-approval routes:
      - (a) PASS — ship; promote to paper-trading stage per
        [product.md § Strategy lifecycle line 304-312](../product.md#strategy-lifecycle--promotion-gates).
      - (b) HOLD — investigate L1-L4 verdict; re-tune N-bar
        cadence or prompt structure; re-run.
      - (c) F-equivalent — retire C5; preserve spec as
        what-not-to-chase reference (mirrors v2.5 DL retirement
        pattern).

## Cost estimate

> Per survey § Candidate 5 line 480: **~5-9 weeks** total wall-
> clock (HIGH variance per survey K-llm-3). This brief refines
> per-stage:

- Analyst pass (this brief): ~1 week (done; this pass).
- Architect M-T1: ~1-2 weeks (ADR-0038 + Wave A-G decomp + K1-K10
  resolutions).
- Developer waves A-G: ~3-5 weeks (novel surface — `LlmForecaster`
  trait + prompt design + Phase F Assistant slot promotion are
  all new; LLM cost-bench + 2 backtest scenarios; replay-cache
  extension).
- Tester M-FINAL + 3× re-run byte-identity verification: ~1 week.
- Presenter + operator-decide: ~1-2 days.

**Total wall-clock: ~6-9 weeks** (lower if architect ships in 1
week; upper if developer waves trip the survey K-llm-2 cost-
blow-up risk).

**Anchor risk**: MEDIUM (LLM determinism + replay-cache must lock
correctly per H4; 30 → 32 anchor count). Higher than v2.5a PatchTST
(LOW; pure additive checkpoint) but lower than v2.5b vanilla
Transformer (rejected — would have been HIGH due to architectural
churn).

**Cost contingency**: if H1 returns positive (≥+0.10 Sharpe-delta),
operator likely promotes Q4=(b) overlay + Q4=(d) all-three-as-
builders as v0.2.0 follow-ons (~2-3 additional weeks).

## Trace

Trace row `REQ-V3-LLM-FORECASTER-001` to be opened in `draft`
state by this analyst pass. `arch`, `crates`, `tests`, `anchors`
columns to be filled by architect / developer / tester
respectively, **only after operator promotion** per Q-SEQ HYBRID.

## Open questions for orchestrator-routing

- **Q-PROMOTE** — when does operator promote Queue → Active? Per
  Q-SEQ HYBRID, default is "after C1 ships its verdict". Operator
  may pre-commit at any time.
- **Q-V2X-SEQ** — `v2x-trading-state-bus` sequencing per Q8. If
  the operator promotes `v2x-trading-state-bus` ahead of C5, this
  brief's R2.1 amends to `TradingState` substrate.
- **Q-ASSISTANT-WAKE** — Q4=(c) Phase F Assistant slot body
  promotion. Default = runtime-gated (R9.3) so default-disabled
  config stays Phase F byte-identical. Operator may opt for an
  unconditional slot promotion (less safe).

## Design

> **Architect M-T1 pass 2026-05-22.** Decomposition lives at
> [`spec/v3-llm-forecaster/decomp.md`](decomp.md) (~720 lines)
> with ADR-0039 at
> [`spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md`](../architecture/adr/0039-llm-forecaster-verdict-criteria.md).
> This section is the load-bearing summary — the decomp.md is the
> authoritative artifact developer waves consume.

### Module layout

```
crates/strategy/src/llm_forecaster/
├── mod.rs              # re-exports + module docs
├── trait_def.rs        # LlmForecaster async trait
├── types.rs            # LlmForecast, ForecastContext, Rating, Confidence,
│                       #   Horizon, LlmForecasterError, LessonCardRef, CostEventRef
├── anthropic_impl.rs   # LlmForecasterImpl over Arc<dyn llm::LlmProvider>
├── strategy.rs         # LlmForecasterStrategy: Strategy (on_bar consumer)
├── prompt.rs           # System-prompt composition via CachedSystemPromptBuilder
├── tool_schema.rs      # propose_forecast ToolSchema definition
└── verdict.rs          # classify_l + classify_l_alpha per ADR-0039 § D1
```

Plus:
- `crates/backtest/src/scenarios/llm_forecaster.rs` (new) +
  `reports/llm_forecaster_report.rs` (new) +
  `bin/llm_forecaster_rerecord.rs` (new).
- `crates/llm/src/bin/llm_forecaster_bench.rs` (new, ~150 LoC; spike
  T-AR-8 + Wave D cost-cap verification).
- `crates/forecast/src/bin/sharpe_comparison.rs` (additive dispatch
  arm for `--scenario llm-forecaster-bs1`).
- `crates/audit/migrations/011_llm_forecast.sql` (additive
  migration; `JournalEntry { kind: "llm_forecast", payload }`).
- `crates/ui/src/assistant/state.rs` + `view.rs` extensions
  (`AssistantMode::ReasoningTrace` variant; runtime-gated per R9.3).

### Signal pipeline shape (T-AR-1)

`LlmForecasterStrategy::on_bar` follows this sequence (decomp § T-AR-1):

```text
1. window.push(bar)
2. if bars_since_last_fire < fire_every_n_bars (default 24): carry-forward
3. ctx = ForecastContext::from_runtime(symbol, now, runtime)
4. let request_hash = ctx.request_hash()  // canonical sha256 per T-AR-2
5. forecast = block_on(forecaster.forecast(ctx))  // async-from-sync via tokio
6. cache forecast at self.last_forecast.insert(symbol, forecast)
7. emit Signal per rating → SignalKind mapping
8. emit audit row + AuditTick (Wave E)
9. return vec![signal]
```

**5-tier rating → 3-variant SignalKind mapping** (architect-locked):

| Rating          | SignalKind | Preservation site                                   |
|-----------------|------------|-----------------------------------------------------|
| STRONG_BUY      | Buy        | `LlmForecast.rating` (audit JournalEntry + Phase F Assistant slot reasoning trace) |
| BUY             | Buy        | same                                                |
| HOLD            | Hold       | same                                                |
| SELL            | Sell       | same                                                |
| STRONG_SELL     | Sell       | same                                                |

`quantity_scale` inherits the noop-fix default `1.0` (no vol-targeting
behavior). STRONG distinction is preserved in audit + reasoning
trace + verdict L1 `hold_frac` denominator (full 5-tier histogram).

### Determinism contract (T-AR-5; K4 mitigation)

5-layer stack:

1. `temperature = 0.0` pinned at every call site.
2. `RecordingProvider` + `ReplayProvider` sqlite cache; cache-miss
   FATAL in research mode.
3. Re-record protocol via `cargo run --bin llm-forecaster-rerecord`.
4. **3-back-to-back identical cache-build run gate at M-FINAL**
   before anchor lock (analyst-recommended K4 mitigation).
5. `cache_schema_version` migration shape (v=1 at v0.1.0).

### Verdict shape (T-AR-9 ADR-0039)

ADR-0039 § D1 L0-L4 priority tree (analyst-strawman LOCKED per Q6
operator constraint). PARALLEL to ADR-0033 § D3 (F-verdict) and
ADR-0038 § D1 (V-verdict), NOT extension. Architect cap "≤2 new
priorities beyond strawman before re-surface" enforced.

L_ALPHA strategy-side gate inherits Sharpe-delta thresholds from
ADR-0038 § D1.c verbatim (cross-paradigm comparability).

### Cost gating (T-AR-4; K2 mitigation)

Per-tier per-call caps: Haiku $0.01 / Sonnet $0.05 / Opus $0.15 /
Ollama $0.00. Per-backtest caps (architect-bumped from analyst-
strawman per cold-record math): Haiku $100 / Sonnet $100 / Opus $300.
N-bar batching: `fire_every_n_bars` default = 24 (once-per-day on
hourly bars). Q5b timeout refined: `timeout_ms = 45_000` (2.25×
Anthropic Sonnet p99 margin; safety for Ollama on slow developer
hardware).

### Anchor delta

- New namespace `v3.0.0-llm-forecaster`.
- +2 rows: `top10-2023-fy-llm-forecaster-realdata`,
  `top10-2024-fy-llm-forecaster-realdata`.
- Anchor count progression: **34 → 36** at developer Wave G close.
- `sharpe-comparison-llm-forecaster-bs1-realdata` NOT anchored at
  v0.1.0; lift route at v0.1.1 if L0/L-ALPHA-UNLOCKED or L-MARGINAL.

### Wave plan (decomp § T-AR-7)

| Wave | Scope                                                                  | Dep      | Est wall-clock |
|------|------------------------------------------------------------------------|----------|----------------|
| A0   | Spike prefix — bench bin + prompt iteration + cache-hit empirical      | —        | 2-3 days       |
| A    | Foundation (`LlmForecaster` trait + payload + ForecastContext)         | A0       | 1-3 days       |
| B    | Impl + prompt + tool schema + bench bin                                | A        | 3-7 days       |
| C    | Strategy registry + Signal mapping                                     | B        | 2-4 days       |
| D    | Backtest scenarios + replay-cache wiring (parallel with C)             | B        | 3-7 days       |
| E    | Audit + cost-budget wiring + Sharpe-comparison bin + verdict.rs        | C        | 3-5 days       |
| F    | Phase F Assistant slot body promotion (UNGATED per Q4=(a)+(c))         | C        | 3-5 days       |
| G    | ADR commit + non-regression + tester handoff (serial closure)          | A-F      | 2-3 days       |

Total dev wall-clock estimate: 3-5 weeks (matches H5 + survey
median).

## Implementation

> **Spike T-AR-8 (developer agent, 2026-05-22)**: PARTIAL spike complete.
> Full dev-note at
> [`spec/dev-notes/v3-llm-forecaster-prompt-spike-2026-05-22.md`](../dev-notes/v3-llm-forecaster-prompt-spike-2026-05-22.md).
>
> **Status**: `ANTHROPIC_API_KEY` not configured — empirical sections (latency
> measurements, quality assessment, tier comparison) are BLOCKED pending operator
> supplying the key. Analytical sections are complete:
>
> - Prompt template v3 designed (verbatim in dev-note).
> - Cost projections from first principles: Haiku 4.5 cold-record ~$24–$30/year
>   at N=24 cadence (vs architect $80 strawman — architect's number was based on
>   older model pricing; see dev-note § D2).
> - Infrastructure confirmed fit-for-purpose via code inspection (no blocker).
> - 6-item delta-list surfaced: D1 (CRITICAL: Sonnet not in pricing table), D2
>   (stale cost math in decomp), D3 (model ID alignment risk), D4 (minor n_calls
>   typo in decomp example), D5 (prompt renderer must omit absent distillation
>   heading), D6 (CanonicalContext field order confirmed correct).
>
> **Operator action required before Wave A empirical phase**: create
> `config/agent.toml.local` with real Anthropic API key.
> Wave A type-level tasks (T-D-N(A1)–T-D-N(A5)) may proceed WITHOUT the key.
> Wave B onwards requires it for `LlmForecasterImpl` end-to-end testing.
>
> **Route recommendation**: Route B (proceed with Wave A + delta-patch D1/D3 first).

### Wave A implementation (developer agent, 2026-05-22)

**T-D-N(A1..A5) COMPLETE.** Wave A foundation shipped. Summary:

**Files created:**
- `crates/strategy/src/llm_forecaster/mod.rs` — module root + re-exports.
- `crates/strategy/src/llm_forecaster/trait_def.rs` — `LlmForecaster: Send + Sync + 'static` async trait.
- `crates/strategy/src/llm_forecaster/types.rs` — `LlmForecast`, `Rating`, `Confidence`, `Horizon`,
  `ForecastContext`, `LlmForecasterError`, `LlmForecasterConfig`, `StubForecaster`, `CanonicalContext`,
  `TechnicalIndicators`, `RecentDecision`, `LessonCardRef`, `CostEventRef`. All with `Serialize + Deserialize`.
- `crates/strategy/src/llm_forecaster/canonicalize.rs` — `hex_encode`, `sha256`, `versions_coherent` helpers.
- `crates/strategy/src/llm_forecaster/strategy.rs` — `LlmForecasterStrategy: Strategy` Wave A skeleton
  with disabled guard (R9.3), carry-forward (R5.4), fire-cadence (N=24 default), `StubForecaster` wiring.
- `crates/strategy/tests/llm_forecaster_payload.rs` — 25 integration tests (Wave A acceptance gate).

**lib.rs updated:** `pub mod llm_forecaster;` added.
**Cargo.toml updated:** `llm`, `reflection`, `uuid`, `tokio` (rt), `pollster` added to dependencies.

**Wave A deviations from decomp.md:**
1. `ForecastContext::test_fixture()` shipped instead of `ForecastContext::from_runtime()`. The `from_runtime`
   method requires live runtime state (reflection-store + indicator cache + audit ledger) which is not wired
   until Wave C. The `test_fixture` builder covers all Wave A testing needs. `from_runtime` lands at Wave C.
2. `strategy.rs` in Wave A (not Wave C). The architect's decomp placed `strategy.rs` under Wave C because
   it depends on `LlmForecasterImpl` (Wave B). However, the Wave A skeleton of `strategy.rs` does NOT use
   `LlmForecasterImpl` — it takes `Arc<dyn LlmForecaster>` and works with `StubForecaster`. This is pure
   type scaffolding and advances the Wave C spec cleanly without creating a real LLM dependency.
3. `ForecastContext` does NOT derive `PartialEq + Eq` because `trading_core::Bar` does not implement them.
   Cache-key equality is served by `request_hash()` which is the architect-specified contract.

**Cargo checkpoint (verified 2026-05-22):**
- `cargo fmt --check` — PASS.
- `cargo clippy -p strategy -- -D warnings` — PASS (note: pre-existing `backtest::main.rs` unreachable-code
  error exists in the workspace; NOT introduced by Wave A — confirmed via git stash).
- `cargo test --workspace --lib --features candle` — 311 PASS (0 failures).
- `cargo test -p strategy --test llm_forecaster_payload` — 25 PASS.
- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (34 / 34)` (additive-zero confirmed).

### Wave C implementation (developer agent, 2026-05-22)

**T-D-N(C1..C4) COMPLETE.** Wave C reflection-memory top-K retrieval wiring + Signal mapping + carry-forward at 24-bar fire cadence shipped.

**Files created:**
- `crates/strategy/tests/llm_forecaster_signal_mapping.rs` — 12 new R2-style regression tests covering: all 5 rating tiers map to correct `SignalKind`, noop-fix lesson guard (non-HOLD ratings mutate kind), carry-forward across 2 variants, fire-cadence counter (exactly 1 call per N bars), disabled guard (R9.3), `from_runtime` with `NullReflectionStore` (empty lessons), hash determinism, multi-symbol isolation, multiple-window fire pattern.

**Files modified:**
- `crates/strategy/src/llm_forecaster/types.rs` — added `ForecastContext::from_runtime()` async builder (Wave C production constructor). Calls `reflection::retrieve_top_k` with `RetrievalQuery` built from `bar.symbol` + BTC regime (via `classify_regime`; fallback to `Chop` on empty btc_closes). Stubs `TechnicalIndicators` (Wave D wires real indicator cache). Added `FromRuntimeError` type.
- `crates/strategy/src/llm_forecaster/strategy.rs` — extended `LlmForecasterStrategy` to hold `Arc<dyn ReflectionStore>` + `btc_closes: Vec<(Timestamp, Decimal)>`. Updated `new()` signature. Added `new_for_test()` convenience constructor (`#[cfg(test)]`). Wired `from_runtime()` in `on_bar` fire path replacing the Wave A `test_fixture()` stub; handles `FromRuntimeError` gracefully (carry-forward on failure). Updated doc comment to Wave C scope.
- `crates/strategy/src/llm_forecaster/mod.rs` — re-exports `FromRuntimeError`.
- `crates/strategy/src/registry.rs` — added `"llm_forecaster_v3"` to `load_from_toml` (`StubForecaster` + `NullReflectionStore`; real impl wired by application binary via `register()`).
- `crates/strategy/tests/llm_forecaster_payload.rs` — updated `LlmForecasterStrategy::new` call sites to new 5-arg signature via `make_strategy` helper.
- `crates/reflection/src/store/mod.rs` — added `NullReflectionStore` no-op implementation (used in test path; always returns empty `Vec`).
- `crates/reflection/src/lib.rs` — re-exports `NullReflectionStore`.

**Wave C deviations from decomp.md:**
1. `LlmForecasterStrategy::new` signature extended (was 3-arg Wave A; now 5-arg with `reflection_store` + `btc_closes`). This is additive — all existing callers updated. Wave A `new_for_test` preserves backward-compat for in-crate unit tests.
2. `TechnicalIndicators` in `from_runtime` are stubs (`dec!(50)` RSI + zeroes) — real indicator cache wiring is Wave D. Decomp.md explicitly defers indicator computation to Wave D; Wave C "analytical-only" scope accepted.
3. `NullReflectionStore` added to `crates/reflection/src/store/mod.rs` as a public export. This is additive to the reflection crate; no existing tests affected.

**Cargo checkpoint (verified 2026-05-22):**
- `cargo fmt --check` — PASS.
- `cargo clippy --workspace --features candle -- -D warnings` — PASS.
- `cargo test --workspace --lib --features candle` — 311 PASS (0 failures).
- `cargo test -p strategy --test llm_forecaster_signal_mapping` — 12 PASS.
- `cargo test -p strategy --test llm_forecaster_payload` — 25 PASS.
- `cargo test -p strategy --test llm_forecaster_wiremock` — 17 PASS.
- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (34 / 34)` (additive-zero confirmed).

## Changelog

- 2026-05-22 (developer agent, spike T-AR-8): Spike dev-note written (PARTIAL —
  empirical sections blocked; analytical sections complete). `## Implementation`
  section added. Delta-list D1–D6 surfaced. Route B recommended.
- 2026-05-22 (architect M-T1): `## Design` section added.
  T-AR-1..T-AR-10 closed; ADR-0039 LLM-forecaster verdict criteria
  L0-L4 written + registered (status `accepted`); `decomp.md`
  authored at `spec/v3-llm-forecaster/decomp.md` (~720 lines)
  covering Module layout + Signal pipeline + Determinism + Verdict
  + Cost gating + Anchor delta + Wave plan A-G with cargo
  invocations + expected literals + 5-cell joint advisory verdict
  routing table. Spike T-AR-8 = YES (2-3 day prefix to Wave A).
  Wave F UNGATED per Q4=(a)+(c) hybrid operator-pick. Baseline
  `ANCHORS PASS (34 / 34)` quoted from
  `bash scripts/verify_anchors.sh`. Anchor delta plan: 34 → 36 at
  developer Wave G close. HANDOFF → orchestrator → spike →
  developer Wave A.
- 2026-05-22 (analyst): initial brief — R1-R10, Q1-Q8, K1-K10,
  H1-H5, non-regression contract; predecessor
  `v2-llm-strategy v2.0.0` (shipped 2026-05-13); parent
  `strategy-reformulation-survey-2026-05-22` Candidate 5; trace
  row `REQ-V3-LLM-FORECASTER-001` to be opened in `draft`;
  backlog Queue § Strategy entry to be added; HANDOFF →
  operator-decide-after-C1-ships (Q-PROMOTE) → architect M-T1 (only
  after promotion).
