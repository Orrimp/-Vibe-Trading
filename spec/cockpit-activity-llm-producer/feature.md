---
slug: cockpit-activity-llm-producer
version: 0.1.0
status: shipped
owner: shipped
updated: 2026-05-27
predecessor: cockpit-activity-status-bar v0.1.0
parent: cockpit-activity-status-bar v0.1.1 forward-list (Q8 LlmCall)
---

# Cockpit activity status bar — LLM-call producer (v0.1.1 follow-on)

> **Predecessor chain**: this brief sits downstream of
> [`cockpit-activity-status-bar v0.1.0`](../cockpit-activity-status-bar/feature.md)
> (shipped 2026-05-26) which landed the `EventBus::activity_tx`
> broadcast channel + `ActivitySender` / `ActivityHandle` RAII pair +
> the 24 px status-bar tape with 3 wired producers (YahooPreload /
> LabRun / Training). It is also downstream of
> [`reflection-memory-trader-wiring v0.1.0`](../reflection-memory-trader-wiring/feature.md)
> (shipped 2026-05-26) which moved the LLM-forecaster subtree to
> `crates/trader/src/llm_forecaster/` — so the producer wiring lands
> in `trader/`, NOT `strategy/`.
> **Parent forward-list**: cockpit-activity-status-bar v0.1.0 § Q8
> + § D2 already enumerated `ActivityKind::LlmCall` in the enum but
> left it unwired (R5.1). This brief is that v0.1.1 wire-up.

## Why

### Why now

Two converging triggers, both 2026-05-26:

1. **cockpit-activity-status-bar v0.1.0 § Q8 forward-list**: the v0.1.0
   architect explicitly named `ActivityKind::LlmCall` as the
   highest-value v0.1.1 producer once `v3-llm-forecaster` shipped its
   provider lifecycle (parent feature.md § R5.1, § Q8 option (c), and
   § K7 cross-feature ordering risk). That ordering condition is now
   met.
2. **reflection-memory-trader-wiring v0.1.0 ship 2026-05-26**: the
   `crates/strategy/src/llm_forecaster/` subtree migrated to
   `crates/trader/src/llm_forecaster/` to satisfy the R8.1 layering
   gate. The LLM call site is now `crates/trader/src/llm_forecaster/
   anthropic_impl.rs:412-416` (`provider.complete(request).await`
   inside `async fn forecast` of `LlmForecasterImpl`). Producer
   wiring lands in trader.

The operator's complaint that birthed v0.1.0 ("Status bar should show
all the current steps the cockpit is doing") explicitly cited LLM
calls as the next multi-second blocking activity needing visibility
(v0.1.0 feature.md § Why we're doing this now). At Anthropic Sonnet
p99 latency the timeout is hard-locked to 45 s
(`anthropic_impl.rs:410` Q5b architect lock); during a backtest fire
cadence of `fire_every_n_bars=24` (parent v3 default) operators see
24-bar gaps with zero feedback. v0.1.1 closes that gap.

### State today

- `ActivityKind::LlmCall` exists as an enum variant on
  `crates/agent/src/activity.rs:53-55` and is documented as
  "forward-listed for v0.1.1". No producer emits it. The status bar
  tape would render an LLM call correctly today — the wiring just
  isn't there.
- `LlmForecasterImpl::forecast` at
  `crates/trader/src/llm_forecaster/anthropic_impl.rs:398-427` is
  the ONLY call site. Single hot path; no fan-out; no other consumer
  of `provider.complete()` in the trader crate.
- The 153 trader integration tests (1 doc-test + 152 integration; per
  reflection-memory-trader-wiring presenter deck 2026-05-26) all
  exercise `LlmForecasterImpl::forecast` either with a stub
  forecaster (no `complete()` call) or against a `wiremock` HTTP
  fixture (real `complete()` call). The wire-up MUST not touch the
  stub path and MUST be neutral against the wiremock path.

### Operator-facing benefit

Operator running a Lab backtest with `llm_forecaster_v3` enabled
(default-disabled today; opt-in via `config.enabled = true`) sees:

- Live LLM call in flight in the bottom status bar
  (`"LLM call: claude-3-5-sonnet-20241022"` per Q1=(a) default).
- The tape's red 3-second hold (parent R2.5) on Anthropic 5xx /
  timeout / budget-cap-short-circuit (`LlmError` family).
- Per-fire-cadence visibility on a 45 s timeout window — instead of
  the current "is the cockpit hung?" question.

## Requirements

Numbered, testable. Anchored against the parent's 34-anchor
non-regression contract (v0.1.0 R-NR.1) + the trader-crate's 153
LLM-forecaster tests (R-NR contract below).

### R1 — Producer wiring at `LlmForecasterImpl::forecast`

- **R1.1** Wire an `ActivityHandle` around the `provider.complete(request).await`
  call at `crates/trader/src/llm_forecaster/anthropic_impl.rs:412-416`.
  The handle is created via
  `activity_sender.start(ActivityKind::LlmCall, label)` BEFORE the
  `.await` and dropped on either successful response OR
  `Self::map_provider_error` mapping.
- **R1.2** **Conditional wire-up**: `LlmForecasterImpl` gains a new
  optional field `activity_sender: Option<ActivitySender>` injected
  at construction time. When `None` (existing test paths + stub
  paths + the `llm_verdict` CLI bin), `forecast()` behaves
  byte-identical to today — no `ActivityHandle` is created, no
  `ActivityEvent` emitted, no perf impact. When `Some(sender)`
  (production Lab Run wiring via cockpit), the handle is created
  per-call. This is the ONLY shape that preserves the 153-test
  contract (R-NR.2).
- **R1.3** No tick events emitted at v0.1.1. The LLM call is
  opaque — no per-token streaming visibility today (Anthropic SDK
  in use is non-streaming via `provider.complete`, not
  `provider.stream`). The tape will show Start → End only. Future
  v0.1.2 can add Tick events if streaming wires (deferred — out of
  scope at this brief; noted in § Out-of-scope below).
- **Acceptance**: new test
  `crates/trader/tests/llm_forecaster_activity_tape.rs` —
  `LlmForecasterImpl` constructed with a concrete `ActivitySender`
  + wiremock provider; assert exactly 1 `ActivityEvent` Start + 1
  End of kind `LlmCall` arrives on a subscribed
  `broadcast::Receiver` for one `forecast()` call. Existing 153
  tests stay PASS.

### R2 — Label format (Q1=(a) analyst default)

- **R2.1** Label format: `"LLM call: <model_id>"` — examples:
  `"LLM call: claude-3-5-sonnet-20241022"`,
  `"LLM call: claude-3-haiku-20240307"`. NO prompt content. NO
  symbol context (the `bar.symbol` is operator-visible elsewhere
  in the cockpit; not needed here). NO lesson-card content.
- **R2.2** PII / secret redaction by construction: because the
  label is built from `self.model_id` (a `String` constant
  injected at `LlmForecasterImpl::new`) only, not from any field
  of `ForecastContext` or `LlmRequest`, prompt-content leakage is
  structurally impossible at v0.1.1. The tape cannot expose
  what was sent to the LLM.
- **R2.3** Label length ≤ 64 chars (parent R1.2 contract).
  Current Anthropic model IDs are ≤ 32 chars; the
  `"LLM call: "` prefix is 10 chars; total ≤ 42 chars. Safe by
  construction. Architect MAY tighten via const at M-T1.
- **R2.4** Label string lives in
  `crates/trader/src/llm_forecaster/anthropic_impl.rs` as a
  module-local `const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";`
  per parent R7.2 "zero string literals" contract. Architect
  decides at M-T1 whether to centralize in
  `crates/ui/src/strings.rs` — analyst-recommended **keep in
  trader/** since trader owns the producer; UI strings file is
  for UI-side copy.
- **Acceptance**: snapshot test
  `crates/trader/tests/llm_forecaster_activity_tape.rs::label_format_is_model_id_only`
  asserts the label matches `"LLM call: " + model_id` exactly and
  contains no prompt content, no symbol, no temperature, no
  lesson-card content.

### R3 — Handle lifecycle (Q2=(a) analyst default)

- **R3.1** **Send-constraint workaround = "store inside the await
  call"**: `ActivityHandle` is `!Send` (parent
  `crates/agent/src/activity.rs:180` uses `Cell<_>` for the
  100 ms throttle state and the outcome). Storing it across an
  `.await` boundary in an async fn would require either (a) `Send`
  (impossible without redesign) or (b) `!Send` future
  (compatible with `async-trait` BUT propagates `!Send` up to
  every caller — high blast radius).
- **R3.2** **Solution: drop the handle BEFORE the await returns**.
  Pattern (architect M-T1 confirms):
  ```rust
  let activity = self.activity_sender.as_ref()
      .map(|s| s.start(ActivityKind::LlmCall, format!("{}{}", ACTIVITY_LABEL_PREFIX, self.model_id)));
  let response_result = self.provider.complete(request).await
      .map_err(|e| Self::map_provider_error(e, timeout_ms));
  // mutate the activity's outcome ONLY if it exists + the response is an Err:
  if let (Some(ref h), Err(ref e)) = (&activity, &response_result) {
      h.fail(e.to_string());
  }
  // explicit drop here emits End { Success } or End { Failed(reason) }:
  drop(activity);
  let response = response_result?;
  ```
  The handle is created on one line, awaited, scope-dropped
  immediately after the `.await` resolves on the same task. The
  `!Send`-ness never crosses an await boundary because the handle
  is dropped before the next `.await` (there is none — the next
  call is sync `decode_response`). Architect M-T1 confirms this is
  the smallest blast radius (Q2=(a)).
- **R3.3** **Rejected alternatives**:
  - **(b) Arc-Mutex the handle**: introduces lock contention on a
    hot path; defeats the `Cell`-based zero-cost throttle in the
    parent's RAII design; adds `Send` to a type that doesn't need
    it. Rejected.
  - **(c) Make `ActivityHandle: Send`**: requires changing the
    parent's `crates/agent/src/activity.rs` to use `Atomic*` for
    `last_tick` + a `Mutex` for `outcome_at_drop`. Out of scope —
    parent's design lock is explicit (parent feature.md
    § R1.3 + activity.rs:177-179 doc-comment).
  - **(d) Spawn a tokio task for the handle**: requires a runtime
    handle which is already optional (`Option<tokio::runtime::Handle>`
    in `LlmForecasterStrategy`). Adds a thread hop per LLM call;
    adds startup latency; doesn't actually solve the !Send
    constraint (the spawned task captures the handle).
    Rejected.
- **R3.4** **Drop-on-panic**: parent's
  `ActivityHandle::drop` already handles panic unwinds via
  `std::thread::panicking()` check (parent activity.rs:234-240,
  emits `Failed("dropped during panic")`). If `provider.complete`
  panics mid-`.await`, the handle drops with the panic-aware
  semantic. No new code at this layer.
- **Acceptance**: new test
  `crates/trader/tests/llm_forecaster_activity_tape.rs::handle_does_not_cross_await` —
  compile-time check via `static_assertions::assert_not_impl_any!`
  on the future type if feasible, OR via a manual review note in
  the test file pointing at the line where `drop(activity)`
  precedes any subsequent `.await`. Runtime test: emit a panic
  inside the wiremock response handler; assert the tape sees
  `End(Failed("dropped during panic"))` OR
  `End(Failed("<wiremock error string>"))`.

### R4 — Failure-state handling (Q3=(a) analyst default)

- **R4.1** All `LlmForecasterError` variants map to `handle.fail()`
  before drop:
  - `Provider(LlmError::Network(_))` → `handle.fail("network error")`
  - `Provider(LlmError::Auth(_))` → `handle.fail("auth error")`
  - `Provider(LlmError::Rate(_))` → `handle.fail("rate limited")`
  - `Provider(LlmError::Server(_))` → `handle.fail("server error")`
  - `Timeout { timeout_ms }` → `handle.fail(format!("timeout {timeout_ms}ms"))`
  - `InvalidResponse { reason }` → `handle.fail(format!("invalid response: {reason}"))`
  - Other `LlmError::*` → `handle.fail("provider error")`
  - **`Provider(LlmError::BudgetExceeded)`** → `handle.fail("budget cap")`
    — this maps to the cost-cap-short-circuit path from
    `crates/trader/tests/llm_forecaster_cost_cap_short_circuit.rs`;
    the cap fires BEFORE `complete()` is called, but the wiring
    must still close the handle correctly. Architect M-T1 confirms
    the exact mapping.
- **R4.2** Operator-facing label on failure: the activity tape
  surfaces the red 3-second hold (parent R2.5) with the label
  unchanged — `"LLM call: claude-3-5-sonnet-20241022"`. The
  `fail()` reason text is captured in the `ActivityEvent` but
  NOT rendered to the operator (parent's
  `crates/ui/src/widgets/activity_tape.rs` does not surface
  `Failed(_)` payload text). The operator sees "an LLM call to
  Sonnet failed; check logs" via the red row hold; the structured
  reason is in `tracing` for debugging.
- **R4.3** **No retry**: the activity producer is decoupled from
  retry policy. v3-llm-forecaster does not retry inside
  `LlmForecasterImpl::forecast` today (the architect-locked
  contract — verify at M-T1 via grep on `anthropic_impl.rs`). If
  retry is added in a future brief, the wiring shape is "one
  activity per attempt" — out of scope.
- **Acceptance**: integration test
  `crates/trader/tests/llm_forecaster_activity_tape.rs::failed_call_emits_red_row` —
  wiremock returns 500; assert tape sees Start + End(Failed(_))
  with non-empty reason.

### R5 — Non-regression contract

- **R5.1** **34 anchors stay byte-identical**. Verified by
  `bash scripts/verify_anchors.sh` at M-FINAL. Producer wiring is
  conditional on `Option<ActivitySender>::is_some()`; existing
  anchored backtest paths construct `LlmForecasterImpl` WITHOUT
  an `ActivitySender` (the bin paths under `crates/backtest/src/bin/`
  do not import or construct `EventBus`) so the wire-up is a
  no-op for anchors by construction.
- **R5.2** **153 LLM-forecaster trader tests stay PASS**. Per
  reflection-memory-trader-wiring v0.1.0 ship 2026-05-26: the
  trader crate carries 153 tests across 11 integration test files.
  All construct `LlmForecasterImpl` via `new()` without an
  `ActivitySender`. The `Option<ActivitySender>` field defaults to
  `None`; `new()` keeps its existing signature; the new field is
  added via a builder/setter pattern OR a new
  `new_with_activity(...)` constructor. Architect picks at M-T1 —
  analyst-recommended **builder setter
  `.with_activity_sender(s)`** for source-level backward compat.
- **R5.3** **Cost tracking unchanged**. The activity-tape wiring
  does NOT touch:
  - `BudgetedProvider` cap accounting (`crates/llm/src/budgeted.rs`)
  - `LlmForecasterImpl::spawn_audit_row` audit emission
    (`anthropic_impl.rs:424`)
  - `crates/cost/` ledger
  - `AuditTick` emission via the existing strategy_*
    `EventBus` channels
- **R5.4** **No new `ActivityKind` variant**. We use the existing
  `ActivityKind::LlmCall` enum value forward-listed in v0.1.0.
- **R5.5** **No new audit migration**. The tape is in-memory only
  (parent R-NR.4). No SQLite schema change.
- **R5.6** **No new Lumen tokens, no new strings file entries**.
  Label prefix is a module-local `const` (R2.4). Activity tape
  widget already supports `ActivityKind::LlmCall` rendering with
  the existing accent/danger/dim colour set (parent R2.4).
- **R5.7** **No `crates/strategy` changes**. The LLM-forecaster
  subtree lives in `crates/trader/` per the just-shipped R8.1
  layering gate. Touching `crates/strategy/` would re-introduce
  the layering violation. Wave A files: trader only.
- **R5.8** **No `crates/agent` changes**. The
  `ActivitySender` + `ActivityKind::LlmCall` + RAII contract
  already exists; we consume but don't extend.
- **R5.9** **`cockpit-smoke` 0 panics**. Existing smoke test
  stays green; the new optional field defaults to None at every
  binary cold-start path that doesn't have an EventBus wired in
  yet.

## Hypothesis register

- **H1** — _The `!Send` `ActivityHandle` can be dropped before
  any subsequent `.await` in `LlmForecasterImpl::forecast`._
  **Falsifier**: `cargo build -p trader` fails with
  `future is not Send` OR the M-T1 architect probe surfaces a
  hidden `.await` between the handle creation and the explicit
  drop. **Status at analyst pass**: assumed TRUE (audited
  `anthropic_impl.rs:398-427` — exactly ONE `.await` at line
  415; `decode_response` at line 421 is sync; `spawn_audit_row`
  at line 424 spawns a fire-and-forget task and does NOT await
  inline). Architect M-T1 re-confirms.
- **H2** — _The 153 LLM-forecaster trader tests pass byte-
  identical post wire-up._ **Falsifier**: any of the 153 tests
  regresses. **Status at analyst pass**: assumed TRUE because
  the `Option<ActivitySender>` defaults to None; tests don't
  inject one; the production code path differs only when the
  field is Some.
- **H3** — _The activity tape's red 3-second hold semantic
  (parent R2.5) is the right operator UX for LLM failures._
  **Falsifier**: post-ship operator review surfaces "I want to
  know the error reason" or "3s is too short / too long".
  **Status at analyst pass**: untestable in code; presenter
  captures in sprint review.
- **H4** — _PII / prompt-content redaction is structurally
  enforced by R2.1 + R2.2 — no future developer can leak prompt
  content via the label without touching this brief's contract._
  **Falsifier**: a future PR adds `format!(..., bar.symbol)` or
  similar to the label string. **Status at analyst pass**:
  enforced by R2.4 (label is a module-local const concat with
  `self.model_id`; no formatting hook); enforced by code review;
  tester gate at M-FINAL grep'd against the label format.

## Risk register

- **K1** — **PII / prompt-content label leakage** (parent v0.1.0
  K4 — explicit cross-link). A future developer adds
  `format!("LLM call: {} on {}", self.model_id, ctx.symbol)` or
  similar, exposing the symbol context in the label. While
  symbol alone is not PII, it begins the slippery slope toward
  full-prompt logging. **Mitigation**: R2.1 / R2.2 / R2.4
  explicit + tester M-FINAL grep on the label string + H4
  tracker. Optional: architect M-T1 considers a
  `#[deny(clippy::format_in_format_args)]` lint at the trader
  crate level (out of scope — not free).
- **K2** — **Activity-tape lag under high LLM call rate**
  (parent R6.3 channel-lag risk). The trader crate's
  `fire_every_n_bars` default is 24 (parent v3 spec); a Lab Run
  over 1 year of hourly bars at fire cadence 24 fires
  ~360 LLM calls. Even at the maximum 1 call per 45 s timeout
  (~80 per hour realtime), the parent's 256-slot ring drains in
  ~26 seconds at 10 events/sec. Lag risk is LOW. **Mitigation**:
  the parent's `RecvError::Lagged` tracing::warn already in
  place; no new code.
- **K3** — **Cost-budget interaction**. The
  `BudgetedProvider::complete` short-circuits with
  `LlmError::BudgetExceeded` BEFORE the underlying provider is
  hit (per `llm_forecaster_cost_cap_short_circuit.rs`). The
  activity wiring must still close cleanly on this path. R4.1
  maps `BudgetExceeded` → `handle.fail("budget cap")`.
  **Mitigation**: dedicated integration test
  `crates/trader/tests/llm_forecaster_activity_tape.rs::budget_cap_short_circuit_closes_handle`.
- **K4** — **Reflection-retrieval label noise**. v3-llm-forecaster
  calls the reflection store BEFORE `LlmProvider::complete` to
  fetch top-K lesson cards (`anthropic_impl.rs` reads from
  `ForecastContext` which was already built). If a future brief
  surfaces "show reflection retrieval as a separate activity",
  there's a risk of the tape becoming noisy (one Start/End per
  reflection probe + one per LLM call = 2x events). **Mitigation**:
  v0.1.1 wires the LLM call ONLY. Reflection retrieval is
  forward-listed to v0.1.2 if operator asks for it; default is
  NO (reflection lookups are sub-millisecond — well under
  parent's 200 ms render-floor at R2.3, so they'd never render
  anyway).
- **K5** — **`crates/trader` test count drift**. v0.1.1 adds new
  integration tests (~4-6 tests under
  `crates/trader/tests/llm_forecaster_activity_tape.rs`). The
  153-test count becomes ~157-159 at M-FINAL. Tester gate
  records the actual delta in the M-FINAL report.
- **K6** — **`Option<ActivitySender>` injection-site choice**.
  `LlmForecasterImpl::new` already takes a 4-arg signature
  (provider, model_id, timeout_ms, audit_emitter — verify at
  M-T1). Adding `activity_sender` could either widen the
  constructor (source breaking) or use a builder/setter
  (additive). Analyst-recommended **builder setter** (R5.2)
  preserves source-level back-compat for the 153 tests.
- **K7** — **!Send propagation if H1 falsifies**. If a future
  refactor inserts an `.await` between handle creation and
  explicit drop, the entire `forecast` future becomes !Send.
  This propagates up to `LlmForecasterStrategy::call_forecast`
  which uses `block_on` — `block_on` accepts !Send futures so it
  would compile. But any future async caller (e.g. live trading
  loop) would break. **Mitigation**: H1 falsifier test +
  compile-time assertion (R3 acceptance).
- **K8** — **`anchored backtest paths` accidentally constructing
  an `EventBus`**. If a future refactor adds an `EventBus`
  to `crates/backtest/src/bin/` and it happens to wire the
  activity sender, the LLM call could fire activity events
  inside an anchored deterministic backtest. **Mitigation**:
  R5.1 explicit; tester gate (anchor verification) catches any
  byte drift.

## Open questions

3 surfaced; all standing-Autoapprove-eligible at analyst-recommended
defaults (this is a focused producer-wiring brief; no
mechanism choices to make — the parent v0.1.0 already locked them).

- **Q1 — Label content.**
  - (a) `"LLM call: <model_id>"` — model identifier only, no
    prompt content, no symbol context. ← **ANALYST DEFAULT**
  - (b) `"LLM call: <model_id> · <symbol>"` — adds symbol for
    context. Rejected: starts the prompt-content slippery slope
    (K1); operator can see symbol elsewhere in cockpit.
  - (c) `"LLM <provider>"` — generic, no model ID. Rejected:
    loses the most actionable bit (model version for cost / latency
    debugging).
- **Q2 — Handle ownership / Send-constraint workaround.**
  - (a) Store inside `LlmForecasterImpl::forecast` for the
    duration of the `complete()` call; explicit `drop` BEFORE
    any subsequent `.await`. !Send is fine because the call is
    awaited in-place. ← **ANALYST DEFAULT**
  - (b) `Arc<Mutex<ActivityHandle>>`. Rejected per R3.3(b).
  - (c) Make `ActivityHandle: Send` in the parent crate.
    Rejected — out-of-scope parent design change.
  - (d) Spawn a tokio task. Rejected per R3.3(d).
- **Q3 — Failure-state handling.**
  - (a) `handle.fail(error.to_string())` on every `LlmError`
    variant; surfaces in the tape as a red 3-second hold per
    parent R2.5. ← **ANALYST DEFAULT**
  - (b) `handle.cancel()` on user-cancellable errors only.
    Rejected: operator can't cancel an LLM call mid-flight today
    (no cancel surface); cancel state is unreachable.
  - (c) Drop without fail (auto-emits Success). Rejected:
    operator sees "succeeded" on a network error; misleading.

## Design (architect M-T1 work)

This section is sketched here for the architect M-T1 pass. Anchor
risk zero by construction (R5.1).

### D1 — Crate layout

| Crate | What it touches |
|-------|------------------|
| `crates/trader` | EDIT `src/llm_forecaster/anthropic_impl.rs` — add `activity_sender: Option<ActivitySender>` field + builder setter + label const + R3.2 wire-up. NEW `tests/llm_forecaster_activity_tape.rs` — 4-6 integration tests. ~ 50 LOC + ~ 200 LOC tests. |
| _no other crate touched_ | strategy / backtest / exec / risk / forecast / reports / audit / agent / ui / data / replay-cache / core / cost / models / reflection — all unchanged. |

### D2 — Wiring sketch (Q2=(a))

```rust
// crates/trader/src/llm_forecaster/anthropic_impl.rs
const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";

pub struct LlmForecasterImpl {
    // … existing fields …
    activity_sender: Option<ActivitySender>,
}

impl LlmForecasterImpl {
    /// Wire the activity producer. Returns self for chaining.
    pub fn with_activity_sender(mut self, sender: ActivitySender) -> Self {
        self.activity_sender = Some(sender);
        self
    }
}

#[async_trait]
impl LlmForecaster for LlmForecasterImpl {
    async fn forecast(&self, ctx: ForecastContext) -> Result<LlmForecast, LlmForecasterError> {
        // … existing build_request + debug! …

        let activity = self.activity_sender.as_ref().map(|s| {
            s.start(
                ActivityKind::LlmCall,
                format!("{}{}", ACTIVITY_LABEL_PREFIX, self.model_id),
            )
        });

        let response_result = self.provider.complete(request).await
            .map_err(|e| Self::map_provider_error(e, timeout_ms));

        if let (Some(ref h), Err(ref e)) = (&activity, &response_result) {
            h.fail(e.to_string());
        }
        drop(activity);                 // emits End

        let response = response_result?;
        let forecast = self.decode_response(response.clone(), &ctx)?;
        self.spawn_audit_row(&forecast, &response);
        Ok(forecast)
    }
}
```

### D3 — Wave plan (architect M-T1 confirms)

- **Wave A** (single wave) — Add field + setter + wire-up + tests.
  ~ 1 day. No parallelization useful; single file edit.

### D4 — Cross-feature interactions

- **`cockpit-activity-status-bar v0.1.0`** — parent. We consume the
  `ActivitySender` + `ActivityKind::LlmCall` enum variant; do not
  extend. Parent R5.1 forward-list closes.
- **`v3-llm-forecaster v0.1.0`** — provider lifecycle owner.
  We tap the existing `provider.complete()` call site;
  do not change LLM call shape or timeout semantics.
- **`reflection-memory-trader-wiring v0.1.0`** — established the
  trader crate that now hosts the producer wiring. No interaction
  beyond housing.
- **Live trading wiring (future)** — when a live trading loop
  constructs `LlmForecasterImpl`, it must call
  `.with_activity_sender(bus.activity())` to surface the producer.
  Today the only constructor sites are tests + the
  `llm_verdict` CLI bin; neither has an EventBus today.
  Operator-decide deferral: live wiring is a future brief.

### D5 — ADRs

No new ADR. Wiring is a tactical consumer of the parent's
ADR-0041 (or whichever number locked at v0.1.0 M-T1) on
activity-broadcast contract. Architect M-T1 confirms ADR linkage.

### D6 — Rollback path

If post-ship the operator says "remove this":

- Remove the `activity_sender` field + setter + wire-up block
  in `anthropic_impl.rs` (~ 20 LOC).
- Remove the new test file (~ 200 LOC).
- Total: ~ 220 LOC, 1 source file + 1 test file. No anchor
  changes. No audit migration. No data loss.

## Backtest Scenarios

**None.** This feature is a producer wire-up on a single async fn;
it does not touch the backtest engine's body, the matching engine,
or any scenario producer. The 34 locked anchors stay byte-identical
(R5.1).

## Out-of-scope at v0.1.1 (forward-listed for v0.1.2+)

- **Per-token streaming Tick events**. Requires switching the
  Anthropic provider to streaming API; non-trivial provider
  redesign; out of scope at the wiring brief level.
- **Reflection retrieval as a separate activity**. K4 — defer
  to operator request post v0.1.1 ship.
- **Lab UI "LLM" widget surfacing in-flight call count.** Out of
  scope at the producer brief level; visible via the activity
  tape already.
- **Cost / token-usage in label**. Out of scope per R2.1 — would
  require post-response label rewrite which the parent's
  `ActivityHandle` doesn't expose. Future enhancement to the
  parent's RAII contract if operator requests it.
- **Per-symbol activity grouping** (one Start/End per symbol in a
  batched LLM call). Out of scope — trader does not batch today.

## Cost framing

- **Analyst pass (this)**: ~ 0.5 day.
- **Operator-decide (Q1-Q3)**: ~ 30 min (all standing-Autoapprove
  eligible at defaults).
- **Architect M-T1**: ~ 0.5 day. H1 falsification probe
  (compile-test the `drop(activity)` pattern); confirm K6
  injection-site choice; lock the wire-up shape.
- **Developer M-DEV**: ~ 0.5-1 day. Single-file edit + ~ 4-6
  integration tests (~ 200 LOC).
- **Tester M-FINAL**: ~ 0.5 day. 34/34 anchors PASS, 153+ trader
  tests PASS, new tests in `llm_forecaster_activity_tape.rs` PASS.
- **Presenter**: ~ 0.5 day. Deck + sprint review.

**Total**: ~ 1-2 days end-to-end wall-clock; no LLM costs (test
fixtures use wiremock and stubs; no real Anthropic calls). Rollback
cost ~ 220 LOC.

## Changelog

- 2026-05-26 (analyst): authored v0.1.0 draft. R1-R5 + H1-H4 +
  K1-K8 + Q1-Q3 + D1-D6 + § Out-of-scope closed. Analyst-recommended
  defaults set on all 3 Qs. Anchor risk zero by construction.
  Parent forward-list (cockpit-activity-status-bar v0.1.0 § Q8
  / R5.1) closed. HANDOFF → architect (M-T1).
- 2026-05-26 (developer): M-DEV complete. T-D-N1..T-D-N5 closed.
  HANDOFF → tester.

## Implementation

_Developer M-DEV — 2026-05-26_

### Files modified

- `crates/trader/src/llm_forecaster/anthropic_impl.rs` — added
  `ACTIVITY_LABEL_PREFIX` const, `activity_sender: Option<ActivitySender>`
  field, `with_activity_sender()` builder setter, and the R3.2 wire-up
  block inside `forecast()`. Also added `agent` to imports. The
  `!Send` `ActivityHandle` is created before the `.await` and dropped
  before `decode_response()` — no `.await` crosses the handle lifetime.
- `crates/trader/Cargo.toml` — added `agent = { path = "../agent" }`
  to both `[dependencies]` and `[dev-dependencies]`; added
  `[[test]] llm_forecaster_activity_tape` entry.

### Files created

- `crates/trader/tests/llm_forecaster_activity_tape.rs` — 6 integration
  tests via wiremock. All 6 PASS.

### Deviations from spec

- None. Builder setter shape exactly as architect T-AR-1 prescribed.
  Label constant in `anthropic_impl.rs` as T-AR-3 prescribed.
  The `!Send` constraint handled via `drop(activity)` before any
  subsequent sync call (T-AR-2 confirmed).

### Gate results

| Gate | Result |
|------|--------|
| `cargo build -p trader` | PASS (exit 0) |
| `cargo clippy -p trader -- -D warnings` | PASS (exit 0) |
| `cargo test -p trader --test llm_forecaster_activity_tape` | 6/6 PASS |
| `cargo test -p trader` | PASS (exit 0, 159 total) |
| `bash scripts/verify_anchors.sh` | 34/34 PASS |

### PII redaction verification

`grep -E "symbol|prompt" <test output>` returns ZERO matches when
scanning activity event labels. The label is structurally restricted
to `"LLM call: " + self.model_id` by construction (R2.2 / K6). Test 4
(`pii_redaction_label_excludes_symbol_and_prompt`) confirms at runtime.
