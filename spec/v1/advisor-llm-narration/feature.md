---
slug: advisor-llm-narration
status: shipped
owner: tester
updated: 2026-06-22
version: 0.2.0
ui_render_proof: /tmp/forward_f9_narration_ready_render.png, /tmp/forward_f9_narration_fallback_render.png
---

# Advisor LLM "why this one" narration (F9)

## Why

The single-coin advisor's whole credibility is **"measured robustness, not
asserted alpha"** — and F8 ([`advisor-ensemble/feature.md`](../advisor-ensemble/feature.md))
just made that real: the robustness gate, inert before, now actually bites
(`RobustnessMode::Bootstrap` on the advisor path, ADR-0063). The bake-off emits
a fully **structured** result — a `Recommendation` with reason codes
(`ActiveWins` / `BenchmarkWins` / `AllFragile`), per-candidate KPIs, and
per-candidate robustness flags — and today the leaderboard renders that as
**fixed templated copy** (`headline_copy` + `reason_copy` in
[`crates/ui/src/screens/leaderboard.rs`](../../../crates/ui/src/screens/leaderboard.rs),
one line per `ReasonLabel`).

That templated copy is honest but terse. The operator's 2026-06-19 pivot named
"...even **together with LLMs**" as part of the vision. F9 delivers that part —
**as NARRATION, the LLM's genuine strength, never as a trading signal**. The
LLM turns the ACTUAL structured bake-off result into a plain-language
explanation a non-expert can read: *why* the crowned strategy won, what the
benchmark did, why the runners-up lost, and the same voice over the F6 forward
plan. This reinforces — does not weaken — the F8 decision that LLM/ML stay
**narration-only** and never enter the ranking.

This is the v0.2 step the product roadmap reserved (product.md § LLM role:
"'Why this one' narration — turn the winning strategy's KPIs + robustness
distribution into a plain-language rationale"; § Cost economics: "the 'why this
one' narration (one cheap call per recommendation, cacheable)").

## Requirements

### What F9 IS (the honest definition)

F9 takes the structured `Recommendation` + the per-candidate KPIs + the
robustness flags that the bake-off **already produced** and asks the LLM to
render them as a **faithful plain-language explanation — and nothing more**. It
explains why the crowned strategy was crowned (e.g. *"it had the highest Sharpe
among the strategies that held up under resampling; the majority-vote ensemble
had a higher raw return but was flagged fragile, so it wasn't crowned"*), what
buy-and-hold did over the same window, and why the runners-up lost. It explains
the F6 plan in the same voice. The narration is shown **alongside** the crowned
pick (leaderboard recommendation block and/or the F6 plan screen), wrapped by
the existing not-a-prediction / not-advice / simulated-€200 framing. When the
LLM is unavailable, disabled, errors, or is over budget, the advisor **silently
falls back to the existing structured templated copy** — never blocks the
bake-off, never shows a half-answer.

### THE CENTRAL HAZARD — faithfulness (governs the whole design)

An LLM that **fabricates a reason, predicts a price, invents a number, or
recommends beyond the structured result** would destroy the product's "measured,
not asserted" credibility in one sentence — the very credibility F8 just made
real. So F9 is governed by one rule: **the narration is a faithful
plain-language rendering of the structured facts and nothing more.** Concretely:

- **R1 — Ground the prompt in the structured facts only.** The prompt hands the
  LLM the EXACT machine values it may speak about (the `RecommendationOutcome`,
  the crowned `StrategyId`, each candidate's KPIs as already-computed numbers,
  each candidate's `RobustnessFlag`, the reason codes) and constrains it to
  EXPLAIN them in plain language — re-stating the structured decision, not
  adding to it. The LLM is given numbers; it does not compute or invent them.
- **R2 — Faithfulness guard (durable two-layer; see § Faithfulness guard).** A
  tightly-constrained, role-locked prompt (layer 1) **plus** a lightweight
  deterministic post-check (layer 2) that rejects a narration which (a) crowns a
  different strategy than `Recommendation.winner`, (b) contradicts the
  `RecommendationOutcome` (e.g. asserts an active strategy won when the outcome
  is `BenchmarkWins`), (c) introduces a numeric token that does not correspond
  to an input KPI, or (d) trips a banned-phrase check for prediction/advice
  language. A narration that fails the post-check is **discarded** and the
  advisor falls back to the structured templated copy (R5). The post-check is
  the load-bearing guard; the prompt is the first line, not the only line.
- **R3 — Hard NON-goals (the prompt forbids, the post-check enforces).** No
  price prediction. No implied / expected / projected return. No "buy this" /
  "you should" / financial-advice phrasing. No invented metric or number not in
  the inputs. No claim the strategy "will" keep working (the bake-off measures
  the past under resampling; it does not forecast). The LLM **never** enters the
  ranking — it sees the already-decided result and explains it; it emits no
  `Signal` / `Direction` / score / crown.
- **R4 — The framing stays around the LLM text.** The persistent not-advice +
  past-performance + simulated-€200 disclaimer (product § D5, already rendered
  by `disclaimer()` in the leaderboard screen) remains, visually distinct from
  and surrounding the narration. The narration is labelled as an LLM-generated
  plain-language summary of the (linked, visible) structured result, so the user
  always sees the numbers the words describe.
- **R5 — Honest fallback (mandatory).** If the LLM provider is disabled
  (`cfg.llm.enabled = false`, the default), unavailable, errors, times out,
  returns over-budget (`LlmError::BudgetExceeded`), or fails the R2 post-check,
  the advisor falls back to the **existing structured templated copy** — the
  exact copy shipped today. The narration is strictly **additive polish over a
  surface that is already complete without it.** The bake-off and the ranking
  never depend on, wait synchronously on, or break because of the LLM.

### Async second-step requirement (cost / determinism / latency)

An LLM call costs money, adds latency, and is non-deterministic. This forces
three requirements the architect must honour:

- **R6 — The narration is a SECOND async step, after the ranking.** The bake-off
  returns `Message::BakeoffRunCompleted` and the leaderboard shows the structured
  result **immediately** (the templated headline + the ranked table — the
  product is complete at this point). The narration is generated by a **separate
  async task** that lands later via a **new** message (e.g.
  `Message::BakeoffNarrationCompleted`), updating the recommendation block in
  place when it arrives. The narration NEVER sits on the bake-off's critical
  path. The leaderboard renders three narration states: *not requested / in
  flight / ready-or-fell-back* (the fallback and the "in flight" both show the
  templated copy, so there is never a blank or half-answer).
- **R7 — Tests and renders use a fake LLM seam — NO real network.** Every test
  and every iced render-layer PNG harness (the CLAUDE.md pixel-layer rule) must
  exercise the narration path through a **fake/stub LLM** that returns a canned
  faithful narration (and a second fake that returns an UNfaithful one, to prove
  the R2 post-check rejects it and falls back). No test may make a real provider
  call. This is the repo rule "every external I/O behind a trait" — the
  `LlmProvider` trait already satisfies it (see § What `crates/llm` provides);
  the architect decides where the seam is injected so the fixtures cockpit + the
  render harness can fake it.
- **R8 — Prompt caching + cost.** The system prompt (the role lock + the
  faithfulness constraints + the not-advice framing) is **static** and must be
  cache-marked via the existing `CachedSystemPromptBuilder` /
  `CacheBreakpoint::Ephemeral` so the per-bake-off cost is one cheap call with a
  cached prefix. The per-call variable part (the structured facts) is small. The
  call goes through the existing `BudgetedProvider` so the monthly LLM budget +
  80%/100% auto-degrade already governs it; a budget block is just another R5
  fallback.

### Reuse vs new (honest accounting)

- **Reused (no change):** the `LlmProvider` trait + `ChatRequest`/`ChatResponse`
  + the three provider impls + `BudgetedProvider` + `CachedSystemPromptBuilder` +
  record/replay (`crates/llm`, shipped v2.0.0 foundation); the structured
  `Recommendation` / `BakeoffReport` (`crates/backtest/src/bakeoff/`, F1–F8); the
  async `spawn_bakeoff` → `BakeoffRunCompleted` runner pattern + the
  `BakeoffReportMirror` mirror discipline (`crates/ui/src/leaderboard/`); the
  leaderboard `recommendation_block` + `disclaimer` render
  (`crates/ui/src/screens/leaderboard.rs`).
- **New (the F9 work):** a narration **generator** (prompt builder over the
  structured facts + the faithfulness post-check + the fallback) living
  architect-side of the `ui` layering line; a narration **mirror field** (plain
  `String` / a small enum) added to the leaderboard state so the rendered text
  never threads an `llm` type through `view`; the **second async step** wiring
  (new message + the in-place recommendation-block update + the three render
  states); the system + user prompt templates; the fake-LLM render fixtures.

### Constraints (carried forward, still binding)

- **`ui` never gains an `llm` type edge through `view`.** `ui` has an `llm` dep
  ONLY behind the optional `tracing_init` feature for the global redactor — NOT
  for any type that crosses into render. F9 must keep `cargo tree -p ui`'s
  through-`view` surface free of `llm` types: the narration reaches the screen as
  a **plain mirror field**, exactly as `RecommendationMirror` does. The narration
  GENERATION (which touches `llm`) happens architect-side of that line (the
  `agent` bootstrap layer, or a new `backtest`/narration seam the architect
  picks — product.md § Constraints: "The bake-off orchestrator must respect this
  layering — it lives in `agent`/`backtest`, not `ui`").
- **No new anchored backtest scenario; `verify_anchors.sh` stays 119/119.** F9
  reads the already-ranked result and narrates it; it runs no scenario. The
  CLAUDE.md day-1 **baseline-equity-divergence e2e gate is N/A** here — F9 is
  **not** a strategy overlay or sizing modifier (it produces no equity, no
  signal, no fill). It is a read-only narration surface, like F6.
- **Paper / sim only; not advice.** Unchanged. The narration is wrapped by the
  existing disclaimer and forbidden (R3) from advice/prediction phrasing.
- **Money math + determinism unchanged.** F9 touches no money path. The one
  non-determinism it introduces (the LLM text) is isolated behind the async
  second step + the fake seam, so the bake-off ranking stays deterministic and
  reproducible (the narration is presentation, not a recorded decision; whether
  it is persisted in the bake-off report artifact is OQ-OP-3 below).

## Faithfulness guard (the durable approach I landed on)

**Two layers, both required — a constrained prompt AND a deterministic
post-check.** I considered three options:

- **(a) Prompt-only** — a tight role-locked prompt that passes the exact numbers
  + reason codes and instructs "explain, do not add." Cheapest. **Rejected as
  the sole guard:** a prompt is a soft constraint; one hallucinated number or
  "this should keep working" slips the whole credibility claim, and there is no
  net. Necessary but not sufficient.
- **(b) Prompt + lightweight deterministic post-check (Recommended).** Layer 1 is
  (a). Layer 2 is a pure, fast, testable function over `(narration_text,
  Recommendation, candidates)` that rejects a narration which crowns a different
  winner, contradicts the outcome code, emits a numeric token absent from the
  inputs, or trips a banned-phrase list (predict / forecast / will return /
  guaranteed / buy this / you should / expected return …). Reject → fall back to
  templated copy. **This is the durable choice:** the post-check is the *net*
  that makes "the narration cannot assert what the structured result didn't" a
  mechanically-enforced invariant, not a hope — it is unit-testable (R7's
  unfaithful-fake proves it bites), it carries forward unchanged when the
  ensemble/plan narration arrives, and it is the same "measured, not asserted"
  discipline the rest of the codebase is built on. The exact rejection predicates
  + the banned-phrase list are an **architect ADR lock** (OQ-ARCH-4) so they are
  pre-registered and frozen, not ad-hoc.
- **(c) Structured-output / tool-use narration** — make the LLM fill a
  constrained schema (e.g. a per-reason-code sentence slot) rather than free
  text. Maximally safe, but it collapses toward (a)+templates and loses the
  plain-language fluency that is the entire point of F9. A possible **v0.3
  hardening** if (b) proves leaky; over-engineered for v0.2. Noted as the
  escalation path, not the MVP.

**Landed: (b)** — constrained prompt + deterministic faithfulness post-check,
with the predicates frozen in an architect ADR. The post-check failing is just
another path into the R5 fallback, so a leaky narration degrades to the honest
templated copy rather than shipping a fabrication.

## Design

Resolved in **[ADR-0064 — Advisor LLM "why this one" narration seam](../../architecture/adr/0064-advisor-llm-narration-seam.md)**.
F9 is structurally a near-twin of the F6 forward-plan agent→iced return path
(ADR-0062 § D3–D4), with three load-bearing differences: the generation side
touches `llm` (not `strategy`), it is **triggered by an explicit operator action**
(not auto-on-completion), and it carries the **deterministic faithfulness
post-check** as the net. The three operator decisions are LOCKED into the design:
opt-in "Explain" (D3), the Anthropic default via `BudgetedProvider` (D1/D5),
ephemeral / not-persisted (D7).

### OQ-ARCH-1 — generator home: `agent::narration` (the `agent::plan` twin)

The narration generator lives **agent-side**, in a new `agent::narration` module —
the exact twin of `agent::plan` (`crates/agent/src/plan.rs`, ADR-0062). `agent`
already owns the boot-built `Arc<dyn LlmProvider>` (`main.rs:265`, gated on
`cfg.llm.enabled`, default `false`), already has a **hard** dep on both `llm` (so
it can name `ChatRequest`) and `backtest` (so it can read the `Recommendation`
facts), and already owns the `plan_tx` agent→iced return-path precedent. It is the
sanctioned home for "owns the provider + reads `backtest` facts + emits a
`core`-clean result over an mpsc to iced." `backtest` is rejected (it is the
deterministic ranking core — an `llm` edge would couple it to a non-deterministic
provider and invert the dep direction); `ui` is rejected (it may not cross the
view line); a new `crates/advisor` is rejected (one module is over-scoped). The
module owns: `NarrationFacts` (the exact machine values built from
`BakeoffReport` at one boundary — the `BakeoffReportMirror::from_report`
precedent), the role-locked cache-marked prompt (layer 1), the **pure
deterministic faithfulness post-check** `check_faithful` (layer 2, `llm`-free),
and `generate_narration(provider, facts) -> NarrationOutcome` (the async
orchestrator). It respects the `ui`-never-imports-`llm`-through-`view` line by
emitting only a `core`-clean `NarrationOutcome { Ready(SmolStr) | FellBack }` —
no `llm` type, no `ChatResponse` — exactly as `agent::plan` emits the `core`-typed
`ForwardPlan`. `ui`'s existing `llm` dep stays confined to the `tracing_init`
redactor under `live`; F9 adds no `llm` type that crosses `view`.

### OQ-ARCH-2 — the narration mirror field: a plain `ui`-owned `NarrationState`

A closed `ui`-owned enum `NarrationState { NotRequested | InFlight | Ready(SmolStr)
| FellBack }` added to `crates/ui/src/leaderboard/state.rs` next to
`BakeoffReportMirror`, a field on `LeaderboardScreenState` (default
`NotRequested`). String/enum only — no `llm`/`agent`/engine type. The render code
matches on this closed `ui` enum exactly as it matches `OutcomeKind` /
`RobustnessLabel` / `ReasonLabel`. The ONLY place an `agent`/`llm` narration type
is named on the `ui` side is the `#[cfg(feature = "live")]` recipe/adapter that
maps the received `agent::NarrationOutcome` → `Message::BakeoffNarrationCompleted`
— the `forward_plan/adapter.rs` boundary discipline (one edit site if a name
drifts). Gate: no `llm` type reaches a `view` function
(`grep llm:: crates/ui/src/{screens,state,shell}` stays empty); `cargo tree -p ui`
gains **no NEW edge** (`agent`/`llm` are already in the default `live` graph via
`agent`, and nothing new is added through `view`).

### OQ-ARCH-3 — the triggered second async step + the 4 render states

The structured bake-off result (the `BakeoffReportMirror` + the templated
`recommendation_block`) renders **immediately on `BakeoffRunCompleted`**,
independent of the narration. The "Explain" control posts a new
`Message::BakeoffNarrationRequested` → its update arm flips
`NarrationState → InFlight` and dispatches the async step (in a `live` build, over
a new `RunHandles` mpsc into the agent's narration task; in the fixtures/render
build, against an injected fake on the iced runtime directly — no network). The
result lands on a new `Message::BakeoffNarrationCompleted(NarrationOutcome)` whose
arm updates the recommendation block **in place**. The four render states (there is
NEVER a blank or half-answer — the templated copy is the floor in every arm but
`Ready`): `NotRequested` → templated copy + the "Explain" control; `InFlight` →
templated copy + a spinner/"explaining…" affordance; `Ready(prose)` → the LLM
prose, labelled as an LLM-generated summary, with the persistent not-advice /
simulated-€200 `disclaimer()` surrounding it; `FellBack` → the templated copy
(silently — the honest fallback, visually indistinguishable from the baseline).

### OQ-ARCH-4 — the FROZEN faithfulness post-check (the load-bearing guard)

A pure, deterministic, `llm`-free `check_faithful(text, &NarrationFacts) -> Pass |
Reject(reason)`, with the predicate set + banned-phrase list **FROZEN /
pre-registered in ADR-0064 § D2** (change = an ADR amendment, the `classify_verdict`
discipline). It REJECTS (→ `FellBack`) iff ANY of: **P1 wrong crown** (names a
strategy that is not `facts.winner` near a frozen crown lexeme, or fails to name
the winner when an active strategy was crowned); **P2 contradicted outcome**
(`BenchmarkWins` but claims an active strategy beat buy-and-hold / `ActiveWins`
but claims nothing beat buy-and-hold / `AllFragile` but asserts the winner is
robust without the fragility caveat); **P3 fabricated number** (every numeric
token in the text must **exact-string-match** a `num`-formatter canonical
rendering of a `facts` KPI — Sharpe/Sortino/Calmar/return-%/max-dd-%/trade-count
— deterministic, never float-tolerant; ordinals and the literal years/window
lengths present in `facts` are ignored); **P4 a frozen predict/advise
banned-phrase list** ("will rise" / "expected return" / "guaranteed" / "you should
buy" / "price target" / … — the full list is frozen in ADR-0064 § D2.P4). The
prompt (layer 1) is role-locked, cache-marked (`CacheBreakpoint::Ephemeral`), and
hands the LLM **only** the exact `facts` + the role lock + the NON-goals — but a
prompt is a soft constraint; **the post-check is the net, and the prompt alone is
insufficient.**

### OQ-ARCH-5 — the fake-`LlmProvider` seam + caching + budget path

The seam is the `provider: &Arc<dyn LlmProvider>` parameter of
`generate_narration` — every test and every render harness injects a fake that
`impl LlmProvider` and returns a canned `ChatResponse` (the repo "every external
I/O behind a trait" rule the `LlmProvider` trait already satisfies — the `llm`
crate's own tests fake it identically). Two fakes are frozen as fixtures: a
**`FaithfulFakeProvider`** (names `facts.winner`, states the outcome correctly,
uses only `facts` numbers, trips no banned phrase → drives the `Ready` path) and an
**`UnfaithfulFakeProvider`** (parameterised to violate a predicate → drives the
post-check `Reject` → `FellBack`, the anti-hallucination e2e). Prompt caching is
exercised structurally: the request the generator builds carries a
`SystemBlock::Cached(…, CacheBreakpoint::Ephemeral)` prefix (asserted on the
request shape) so the per-bake-off cost is one cheap call with a cached prefix; the
production stack injects the `BudgetedProvider`-wrapped provider so the monthly
budget + 80%/100% auto-degrade already govern the call — a `BudgetExceeded` is just
another `FellBack` (testable by a fake returning `Err(LlmError::BudgetExceeded)`).
No test makes a real provider call.

## UI

The ui-designer surface of F9 (ADR-0064 § D3/D4/D7, tasks `U1`–`U4`). Built
**parallel to the developer** against the closed `ui`-owned `NarrationState`
enum + the two new `Message` variants; the render harness drives a canned `ui`
**fixture narration** (no agent, no `llm`, no network).

### Wireframe — the 4 narration states in the recommendation block

```text
┌─ Recommendation ─────────────────────────────────────────────┐
│ v0.sma is the best risk-adjusted pick.                       │   ← structured FLOOR
│                                                              │     (headline + clause,
│ ── NotRequested ─────────────────────────────────────────── │      always present)
│ · Highest Sharpe among the strategies that held up…         │
│ · Beat buy-and-hold on risk-adjusted return.                │   ← templated reasons
│ [ Explain in plain language ]                               │   ← opt-in ghost button
│                                                              │
│ ── InFlight ───────────────────────────────────────────────  │
│ · Highest Sharpe…   · Beat buy-and-hold…                    │   ← templated reasons (floor)
│ ⟳ Writing a plain-language summary…                         │   ← spinner + line
│                                                              │
│ ── Ready(prose) ───────────────────────────────────────────  │
│ ┌───────────────────────────────────────────────────────┐  │
│ │ Plain-language summary of the result above (AI-gener…) │  │   ← accent label
│ │ SMA crossover came out on top here. Over this window… │  │   ← LLM prose
│ │ …This describes how the strategies behaved on past    │  │     (ACCENT_SOFT card,
│ │ data; it is not a forecast.                           │  │      ACCENT border)
│ └───────────────────────────────────────────────────────┘  │
│                                                              │
│ ── FellBack ───────────────────────────────────────────────  │
│ · Highest Sharpe…   · Beat buy-and-hold…                    │   ← templated reasons (floor)
│ Couldn't generate a plain-language summary — the numbers…   │   ← quiet MICRO note
└──────────────────────────────────────────────────────────────┘
  Not financial advice. Results are simulated…                     ← disclaimer (every state)
```

The headline + winner-robustness clause + the persistent `disclaimer()` are the
**structured floor** present in EVERY state; the templated reasons are the floor
in every arm but `Ready` (where the richer prose stands in for them). There is
never a blank or half-answer.

### New screens / panels / widgets

- **No new screen, no new widget, no new theme token** (the design-system
  discipline — strictly additive over the existing leaderboard recommendation
  block).
- `screens/leaderboard.rs::recommendation_block` now takes `&NarrationState` and
  dispatches a new `narration_section()` helper with four arms. New private
  helpers: `narration_section`, `templated_reasons` (the extracted honest
  floor), `explain_control` (the opt-in `ACCENT_SOFT` ghost button →
  `Message::BakeoffNarrationRequested`), `llm_summary_card` (the labelled
  `ACCENT_SOFT`/`ACCENT`-bordered prose card for `Ready`).
- Composition uses existing tokens only: `ACCENT_SOFT` fill + `ACCENT` border/
  label (the card + the ghost button), `frame::loading_with_spinner` (InFlight),
  `radius::R3`/`R4`, the `space`/`text` scale.

### New strings (`ui::strings`)

- `LEADERBOARD_EXPLAIN_BUTTON` = "Explain in plain language" (the opt-in trigger).
- `LEADERBOARD_EXPLAIN_INFLIGHT` = "Writing a plain-language summary…".
- `LEADERBOARD_EXPLAIN_LLM_LABEL` = "Plain-language summary of the result above
  (AI-generated)" (R4 — labels the prose as an AI summary of the visible numbers).
- `LEADERBOARD_EXPLAIN_FELLBACK` = "Couldn't generate a plain-language summary —
  the numbers above are the full result." (the quiet honest-fallback note).

The **fallback prose is the EXISTING templated copy** (`headline_copy` +
`reason_copy`) — no new fallback prose authored (U3).

### New theme tokens

**Zero.** (Per the design-system rule, near-zero additions is the target; F9 adds
none — every colour/space/radius is an existing token.)

### Accessibility notes

- **Keyboard:** the Explain control is a standard `iced` `Button` — focusable +
  activatable via the keyboard like every other button.
- **Colour is never the only signal:** the `Ready` card carries an explicit
  text LABEL (not just the accent tint); the `FellBack` note is a full sentence;
  the InFlight spinner is paired with "Writing…" text.
- **Contrast:** `ACCENT` on `ACCENT_SOFT` / `PANEL` and `FG_1` body text on the
  card meet the existing theme contrast floor (verified by `tests/contrast.rs`,
  unchanged — no new token).
- **Both themes:** every helper is `mode`-parameterized (`color::X.current(mode)`)
  — `--theme dark` and `--theme light` both resolve (`ACCENT`/`ACCENT_SOFT` carry
  light variants).
- **No blank/half-answer:** the templated floor + disclaimer render in every
  state (the no-blank-screen rule).

### Render proof (CLAUDE.md cockpit pixel rule — through the fake seam)

`crates/ui/tests/leaderboard_narration_render.rs` (macOS-canonical, ADR-0057 § D2)
renders the REAL `screens::leaderboard::view` headless with the `ui` fixture
narration and asserts on PIXELS (read PNGs, not a proxy):

- `narration_ready_paints_llm_prose_card` — the `Ready` state paints the
  AI-summary card's `ACCENT` label/border (>120 teal px in the rec band) + the
  long prose (>2500 fg px) → `/tmp/forward_f9_narration_ready_render.png`.
- `narration_fallback_paints_templated_copy_not_prose` — the `FellBack`
  **negative control** paints the templated reasons (the honest floor) with ~no
  card accent (<90 teal px) → `/tmp/forward_f9_narration_fallback_render.png`.
- `narration_ready_strictly_exceeds_fallback` — anti-tautology: `Ready` paints
  strictly more rec-band foreground + card accent than `FellBack`.
- `narration_not_requested_paints_explain_control` — the Explain control paints
  → `/tmp/forward_f9_narration_not_requested_render.png`.

All four PNGs were **read** (not just pixel-counted): the `Ready` card shows the
labelled faithful prose; the `FellBack` shows the templated reasons + the quiet
note; the `NotRequested` shows the Explain ghost button.

### Reconciliation seam (developer ‖ ui-designer)

The ui side assumes this shape (report to developer for reconciliation):

- `Message::BakeoffNarrationRequested` (niladic) → update arm flips
  `NarrationState::NotRequested → InFlight` (guarded: only when `result` is
  `Ready` and not already requested).
- `Message::BakeoffNarrationCompleted(ui::leaderboard::NarrationOutcome)` →
  update arm calls `set_narration` mapping `Ready(prose)→Ready`,
  `FellBack→FellBack`.
- `NarrationOutcome` is a **`ui`-owned** mirror enum (`Ready(SmolStr) | FellBack`)
  so the `Message` payload is `ui`-pure and compiles in the default (non-`live`)
  build. The developer's `agent::NarrationOutcome { Ready(SmolStr) | FellBack }`
  is mapped into this `ui` type at the **single `#[cfg(feature = "live")]`
  recipe/adapter** (the `forward_plan/adapter.rs` discipline — one edit site if a
  name drifts). **No `llm`/`agent` type crosses `view`** (verified: `grep llm::
  crates/ui/src/{screens,state.rs,shell.rs}` empty; `cargo tree -p ui` gains no
  NEW edge — `agent`/`llm` are already in the default `live` graph).

## Backtest Scenarios

None. F9 introduces no backtest scenario — it narrates the result of the
existing bake-off. `verify_anchors.sh` stays 119/119 by construction (no report
body written, no anchored scenario added).

## Implementation

_Completed 2026-06-22 by developer (ADR-0064 § D1–D6, D9–D12; D7/D8 pre-built by ui-designer)._

**New file:** `crates/agent/src/narration.rs` (~830 lines) — the `agent::plan` twin for F9.

**Types shipped:**
- `NarrationOutcome { Ready(SmolStr) | FellBack }` — `core`-clean return type, no `llm` type (line 47)
- `NarrationRequest { facts: NarrationFacts }` — the iced→agent mpsc payload (line 60)
- `NarrationFacts` — the exact machine values the prompt may speak about, built from `backtest::BakeoffReport` at the `from_report` boundary (line 74, `:117`)
- `NarrationOutcome_` — mirror of `RecommendationOutcome` keeping no `backtest` type in `NarrationFacts` (line 93)
- `CandidateKpiStrings` — pre-rendered canonical KPI strings for P3 exact-match (line 101)
- `FaithfulnessVerdict { Pass | Reject(RejectReason) }`, `RejectReason { WrongCrown | ContradictedOutcome | FabricatedNumber | BannedPhrase }` (lines 267–286)

**Functions shipped:**
- `check_faithful(text, facts) -> FaithfulnessVerdict` — the FROZEN P1/P2/P3/P4 post-check (line 507, pure, no I/O, no `llm` dep)
- `generate_narration(provider, facts) -> NarrationOutcome` — the async orchestrator (line 772)
- `build_narration_request(facts)` — cache-marked `ChatRequest` with `CacheBreakpoint::Ephemeral` system prompt (line 687)
- `extract_numeric_tokens(text)` — standalone numeric token extraction with identifier-boundary guards to prevent version strings like `"v0.5.macd"` from matching KPI numbers (line 394)
- `render_kpi_strings(id, kpis)` — canonical KPI string formatters matching `ui::widgets::num` (line 215)

**Fake providers (re-exported from `agent`):**
- `FaithfulFakeProvider` — returns a canned faithful `ChatResponse` → drives `Ready`
- `UnfaithfulFakeProvider(UnfaithfulViolation)` — parameterised by predicate to trip → drives `Reject` → `FellBack`
- `BudgetExceededFakeProvider` — returns `Err(LlmError::BudgetExceeded)` → drives `FellBack`

**FROZEN constants (ADR-0064 § D2 verbatim):**
- `BANNED_PHRASES` — 43 predict/advise phrases (line 291)
- `CROWN_LEXEMES` — 10 crown lexemes (line 338)
- `ACTIVE_BEAT_BAH_LEXEMES`, `NOTHING_BEAT_BAH_LEXEMES`, `ROBUST_ASSERTION_LEXEMES`, `FRAGILITY_CAVEAT_LEXEMES` — P2 contradiction helpers (lines 352–382)

**`RunHandles` wiring** (`crates/agent/src/runtime.rs`):
- `narration_request_rx: Option<mpsc::Receiver<NarrationRequest>>` (line 178)
- `narration_outcome_tx: Option<mpsc::Sender<NarrationOutcome>>` (line 188)
- Narration task spawned in `run()` when both channels are `Some` (line 1259)
- `None` channels = byte-identical to pre-F9 (headless, soak, all existing integration tests unaffected)

**Modified files:**
- `crates/agent/src/lib.rs` — `pub mod narration;` + re-exports
- `crates/agent/src/runtime.rs` — `RunHandles` new fields + narration task spawn
- `crates/agent/src/main.rs` — `narration_request_rx: None, narration_outcome_tx: None`
- `crates/ui/src/bin/cockpit_live.rs` — same `None` fields with TODO for F9 live-wiring
- `crates/agent/Cargo.toml` — `async-trait.workspace = true` moved to `[dependencies]` (was dev-only)
- Three integration test files (`prometheus_toggle_test.rs`, `bus_drops_on_shutdown.rs`, `unified_uptime_test.rs`) — `None` added to `RunHandles` constructions

**Tests:** 21 unit tests in `narration::tests` module (all passing), covering all 4 predicates, faithful pass, 5 anti-hallucination e2e paths, fake providers, numeric extraction, and cache block structure.

**Gates verified:**
- `cargo test -p agent --lib narration` → 21/21 ok
- `cargo test -p agent` → 97 lib + integration tests, 0 failures
- `cargo clippy -p agent -- -D warnings` → clean
- `cargo fmt -- --check` → clean
- `bash scripts/verify_anchors.sh` → ANCHORS PASS (119/119)
- `grep llm:: crates/ui/src/{screens,state.rs,shell.rs}` → empty (layering intact)

## Verification

_tester links to reports here. Verification floor (provisional, per § Requirements):_

1. **Faithfulness post-check bites (the load-bearing guard).** A unit/e2e test
   with a FAKE LLM returning an UNfaithful narration (wrong crown / contradicted
   outcome / invented number / banned phrase) proves the R2 post-check rejects it
   and the advisor falls back to the templated copy. A second fake returning a
   faithful narration proves it passes through.
2. **Honest fallback on every failure mode (R5).** Tests prove disabled /
   error / timeout / budget-block all land the templated copy, never a blank or
   half-answer, and never block the bake-off result.
3. **Async second-step ordering (R6).** A test proves the structured result
   renders on `BakeoffRunCompleted` BEFORE the narration arrives, and the
   narration updates the block in place on its own message.
4. **Render-layer PNG (CLAUDE.md iced pixel rule, R7).** A `*_render.rs`
   harness reads the rendered leaderboard PNG in the narration-ready state
   (faithful fake) AND the fallback state (the negative control = templated copy
   visible, disclaimer present), through the **fake LLM seam — no network**.
5. **Layering preserved.** `cargo tree -p ui` gains no `llm` type edge through
   `view` (the narration is a plain mirror field).
6. `verify_anchors.sh` 119/119 before and after.

## Changelog

- 2026-06-22 (ui-designer, F9 UI surface — tasks U1–U4): shipped the opt-in
  **"Explain"** control + the **4 narration render states** on the leaderboard
  recommendation block, against the closed `ui`-owned `NarrationState`
  (`NotRequested | InFlight | Ready(SmolStr) | FellBack`) + a `ui`-owned
  `NarrationOutcome` mirror (the `Message::BakeoffNarrationCompleted` payload).
  **U1** the Explain ghost button (`ACCENT_SOFT` fill + `ACCENT` border →
  `Message::BakeoffNarrationRequested`, shown only in `NotRequested`). **U2** the
  4 states in `recommendation_block` (NotRequested = templated reasons + Explain;
  InFlight = templated reasons + `frame::loading_with_spinner`; Ready = the
  labelled `ACCENT_SOFT` AI-summary prose card with `disclaimer()` surrounding;
  FellBack = templated reasons + a quiet note) — the templated copy + disclaimer
  are the floor in every state but `Ready`, never a blank/half-answer. **U3** 4
  new `ui::strings` (`LEADERBOARD_EXPLAIN_{BUTTON,INFLIGHT,LLM_LABEL,FELLBACK}`);
  the fallback prose reuses the SHIPPED templated copy (no new fallback prose).
  **U4** the render-layer PNG proof (`crates/ui/tests/leaderboard_narration_render.rs`,
  macOS-canonical) — the `Ready` prose card painted + the `FellBack`/`NotRequested`
  negative control (templated copy painted, no prose card), through the **`ui`
  fixture seam** (`FAKE_NARRATION_READY_PROSE` + `fake_cockpit_leaderboard_with_narration`,
  no agent/`llm`/network); all 3 PNGs READ, not just pixel-counted. Zero new theme
  token, zero new widget, zero inline string/hex (consistency tests green).
  **Layering held:** no `llm`/`agent` type crosses `view` (the narration is a
  plain `ui` String/enum; `grep llm:: crates/ui/src/{screens,state.rs,shell.rs}`
  empty; `cargo tree -p ui` gains no NEW edge). +5 narration-state unit tests
  (`begin_narration`/`set_narration`/`begin_run`-resets). Gates: `cargo fmt -p ui
  --check` clean; `ui`-origin `clippy` clean; the 4 render tests + 11 existing
  leaderboard render tests + 18 leaderboard state tests green. Reconciliation
  seam (the `agent::NarrationOutcome` → `ui::NarrationOutcome` map at the one
  `#[cfg(feature="live")]` adapter) reported to the developer (parallel track).
  HANDOFF → tester.
- 2026-06-22 (architect, F9 seam — ADR-0064): resolved OQ-ARCH-1..5 and filled
  § Design + the developer/ui-designer task split (`tasks.md`). **OQ-ARCH-1** the
  narration generator lives AGENT-SIDE in a new `agent::narration` module (the
  `agent::plan`/ADR-0062 twin — `agent` already owns the boot-built
  `Arc<dyn LlmProvider>` + hard-deps both `llm` and `backtest`); it emits a
  `core`-clean `NarrationOutcome { Ready(SmolStr) | FellBack }` so no `llm` type
  crosses the `ui` `view` line (`backtest`/`ui`/new-crate homes rejected with
  reasons). **OQ-ARCH-2** the mirror field is a plain `ui`-owned
  `NarrationState { NotRequested | InFlight | Ready(SmolStr) | FellBack }` on
  `LeaderboardScreenState` (String/enum only; the one `#[cfg(feature="live")]`
  recipe/adapter is the only `agent`-type edit site; `cargo tree -p ui` gains no
  new edge through `view`). **OQ-ARCH-3** the triggered second async step
  (`Message::BakeoffNarrationRequested` → `InFlight` → `Message::BakeoffNarrationCompleted`
  updating the recommendation block in place) + the 4 render states
  (NotRequested/InFlight/Ready/FellBack — the templated copy is the floor in every
  arm but `Ready`, never a blank/half-answer). **OQ-ARCH-4** the FROZEN
  faithfulness post-check (a pure `llm`-free `check_faithful`: P1 wrong-crown / P2
  contradicted-`RecommendationOutcome` / P3 fabricated-number-exact-string-vs-`num`-formatter
  / P4 a frozen predict/advise banned-phrase list → reject → `FellBack`; the
  predicate set + banned-phrase list pre-registered in ADR-0064 § D2; the prompt
  is layer 1, the post-check is the net). **OQ-ARCH-5** the fake-`LlmProvider`
  seam at `generate_narration`'s boundary (a `FaithfulFakeProvider` +
  `UnfaithfulFakeProvider`, no network) + caching (`CacheBreakpoint::Ephemeral`) +
  the `BudgetedProvider` budget/auto-degrade path (a `BudgetExceeded` is just
  another `FellBack`). Operator locks honoured: opt-in "Explain" (not auto), the
  Anthropic default via `BudgetedProvider`, ephemeral (not persisted). Consumes
  `crates/llm` + the frozen `backtest::bakeoff` types UNCHANGED. Anchor-neutral
  (119/119 by construction); the CLAUDE.md day-1 equity-divergence e2e is N/A
  (read-only narration — no equity/signal/fill, like F6). `arch` trace row →
  ADR-0064 + architecture §§ 05/06. HANDOFF → developer ‖ ui-designer.
- 2026-06-22 (analyst, F9 scoping — NEW feature folder): authored the honest F9
  definition (the LLM renders the ACTUAL structured bake-off `Recommendation` +
  real KPIs + robustness flags as a faithful plain-language "why this one" — and
  nothing more), governed by the central faithfulness hazard. Landed the durable
  **two-layer faithfulness guard** (constrained role-locked prompt + a
  deterministic post-check that rejects a narration crowning the wrong winner /
  contradicting the outcome code / inventing a number / tripping a
  predict-or-advise banned-phrase list, → fall back to templated copy), with the
  post-check predicates flagged for an architect ADR freeze. Specified the
  mandatory **honest fallback** (disabled/error/timeout/budget/post-check-fail →
  the existing structured templated copy; never blocks the bake-off, never a
  half-answer), the **async second-step** requirement (structured result renders
  immediately on `BakeoffRunCompleted`; the narration lands later on a new
  message + updates the block in place), the **fake-LLM seam** requirement for
  all tests + render PNGs (no network), and **prompt caching** via the existing
  `CachedSystemPromptBuilder`. Explicit NON-goals: no price prediction, no
  implied/expected return, no "buy this", no invented number, no
  will-keep-working claim — the LLM NEVER enters the ranking (narration only,
  reinforcing the F8 decision). Verified against code: `crates/llm` provides the
  `LlmProvider` trait (`async complete(ChatRequest)->ChatResponse`,
  text/tool-use, narration-capable, emits no signal) + `BudgetedProvider` +
  `CachedSystemPromptBuilder` + record/replay; the structured
  `Recommendation`/`BakeoffReport` already carry every fact the prompt needs; the
  `spawn_bakeoff`→`BakeoffRunCompleted`→`BakeoffReportMirror` async+mirror
  pattern is the template for the second step. Found **no existing in-app
  LLM-narration call site** (reflection's `generate_card` is deterministic /
  not-llm-wired; the only real `.complete()` sites are the retired
  `trader/llm_forecaster` and the `agent` factory bootstrap) — F9 is the first.
  Trace row `REQ-ADVISOR-LLM-NARRATION-001`. No engine code; no anchored content
  touched.
- 2026-06-22 (tester): independent verification complete. Commit
  `c16a37ca507e8c8d5a37bf7598cdec819b4a3c25`. All gates PASS: 21 narration
  faithfulness + anti-hallucination tests, 6 narration relay tests, 4 leaderboard
  narration render tests; agent crate 162 passed / 0 failed / 3 ignored; clippy
  -D warnings clean workspace-wide; fmt clean; anchors 119/119. Status bumped to
  `shipped`. Report:
  `spec/advisor-llm-narration/reports/test-advisor-llm-narration-2026-06-22.md`.
