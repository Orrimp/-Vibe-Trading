---
adr: 0064
title: Advisor LLM "why this one" narration seam — agent-side generator + frozen faithfulness post-check + plain ui mirror + triggered second async step
status: accepted
date: 2026-06-22
supersedes: none
superseded-by: none
---

# ADR-0064: Advisor LLM "why this one" narration seam

## Context

The single-coin investment-advisor MVP (product pivot 2026-06-19) journey step 3
(feature `advisor-llm-narration`, roadmap **F9**) is the first **in-app LLM
consumer** the codebase ships. The bake-off (F1–F8) already emits a fully
**structured** result — `backtest::bakeoff::Recommendation { outcome, winner,
benchmark_kpis, winner_kpis, winner_robustness, reasons }` plus per-candidate
`CandidateKpis` + `RobustnessFlag` — and the leaderboard already renders it as
honest **templated copy** (`headline_copy` + `reason_copy` in
`crates/ui/src/screens/leaderboard.rs`, one line per `ReasonLabel`). F8
(ADR-0063) made the robustness gate actually bite on the advisor path and locked
LLM/ML as **narration-only, never a ranking input**.

F9 delivers the narration F8 reserved: on an **explicit operator action**, the
LLM renders the ALREADY-COMPUTED structured facts as faithful plain-language
prose (why the crowned strategy won, what buy-and-hold did, why the runners-up
lost). It is a **READ-ONLY rendering of the decided result** — the LLM never
enters the ranking, never emits a `Signal`/`Direction`/score/crown.

**The central hazard governs the whole design.** An LLM that fabricates a
reason, predicts a price, invents a number, or recommends beyond the structured
result would destroy the product's "measured robustness, not asserted alpha"
credibility — the very credibility F8 just made real — in one sentence. So the
design's load-bearing element is not the prompt; it is a **deterministic,
unit-testable post-check** that is the net.

Four structural facts, verified against code, force the seam decisions:

- **`crates/llm` is a shipped foundation with zero prior in-app consumers.**
  `LlmProvider::complete(ChatRequest) -> Result<ChatResponse, LlmError>`
  (`trait_def.rs:245`), `BudgetedProvider` (budget gate + auto-degrade),
  `CachedSystemPromptBuilder` + `CacheBreakpoint::Ephemeral` (prompt caching),
  record/replay — all present, none consumed in-app (reflection's `generate_card`
  is deterministic, not LLM-wired). F9 must consume these **unchanged**.
- **The `Arc<dyn LlmProvider>` is already built AGENT-SIDE.** `agent::main`
  (`main.rs:265`) and `cockpit_live` construct it once at boot, gated on
  `cfg.llm.enabled` (default `false`), and store it as an
  `Option<Arc<dyn LlmProvider>>`. There is no second construction site; `ui`
  never touches credentials.
- **The exact agent→iced return-path precedent already ships (ADR-0062 F6).**
  The forward-plan seam built `RunHandles.plan_tx: Option<mpsc::Sender<ForwardPlan>>`
  (`runtime.rs:167`) → the iced thread holds the matching `Receiver`
  (`cockpit_live.rs:502`, `_plan_rx_live`) → an iced recipe lands
  `Message::ForwardPlanReceived(ForwardPlanView)` (`state.rs:2098`); `ui` mirrors
  the `core`-typed `agent::config::ForwardPlan` into a pure-`ui` `ForwardPlanView`
  via a **single `#[cfg(feature = "live")]` adapter** (`forward_plan/adapter.rs`).
  F9's narration return path is structurally the same shape.
- **The `ui` view path is `llm`-free and must stay that way.** `ui`'s `llm` dep
  is **optional**, pulled only behind `live` for `llm::tracing_init` (the global
  redactor — `Cargo.toml:135`, T-RED-D10). No `llm::` type appears in
  `screens/` / `state.rs` / `shell.rs` today. The standing invariant
  (ADR-0023 / ADR-0041 / ADR-0059 / ADR-0060 / ADR-0062): **no `strategy` /
  `exec` / `forecast` / `llm` type crosses `view`.** Because `ui`'s default
  features include `live` (which pulls `agent` → `llm`), the gate is **not**
  "`llm` absent from `cargo tree -p ui`" — it is "no `llm` type reaches a `view`
  function; the narration crosses as a plain `ui`-owned enum, exactly as
  `RecommendationMirror` does."

Three operator decisions are **LOCKED** (build to these): the narration is
**opt-in** (generated on an explicit "Explain" control on the crowned pick, NOT
automatically after every bake-off — the bake-off result is complete and honest
without it); the model is the **Anthropic default** via the existing
`BudgetedProvider` (let the budget/degrade machinery handle cost — do not
pre-compromise faithfulness); and the narration is **ephemeral** — NOT persisted
in the bake-off report artifact (the structured `Recommendation` is the
reproducible artifact; the prose is a derived view).

## Decision

**D1 — The narration generator lives AGENT-SIDE, in a new `agent::narration`
module — the exact twin of `agent::plan` (ADR-0062 § D3).** `agent` already owns
the `Arc<dyn LlmProvider>`, already has a **hard** dep on both `llm` and
`backtest` (so it can name `ChatRequest` AND read `Recommendation`/`CandidateKpis`/
`RobustnessFlag`), and already owns the `plan_tx` agent→iced return-path
precedent. The module owns four pieces:

1. **`NarrationFacts`** — a small `agent`-side struct carrying ONLY the exact
   machine values the prompt may speak about (the `RecommendationOutcome`, the
   crowned `StrategyId` display string, each candidate's already-computed KPIs as
   strings, each candidate's `RobustnessFlag`, the reason codes). Built from the
   `backtest::BakeoffReport` at this one boundary — the `BakeoffReportMirror::from_report`
   precedent (the single place an engine type is read).
2. **The prompt builder (layer 1)** — composes a **role-locked, cache-marked**
   `ChatRequest`: the static system prompt (the role lock + the faithfulness
   constraints + the explicit NON-goals + the not-advice framing) is built via
   `CachedSystemPromptBuilder` with a `CacheBreakpoint::Ephemeral` boundary so the
   per-bake-off cost is one cheap call with a cached prefix; the small variable
   user turn carries only the `NarrationFacts`. The request goes through the
   injected `Arc<dyn LlmProvider>` (the `BudgetedProvider`-wrapped stack at boot),
   so the monthly budget + 80%/100% auto-degrade already govern it.
3. **The faithfulness post-check (layer 2 — THE LOAD-BEARING GUARD, D2).** A
   **pure, deterministic, `llm`-free function** `check_faithful(text,
   &NarrationFacts) -> FaithfulnessVerdict` (D2). It is the net.
4. **`generate_narration(provider, facts) -> NarrationOutcome`** — the async
   orchestrator: build the request → `provider.complete(req).await` → on `Ok`,
   extract the text and run `check_faithful`; on `Pass`, return
   `NarrationOutcome::Ready(text)`; on **any** of {provider `Err` (disabled /
   network / timeout / `BudgetExceeded` / `ReplayMiss`), empty/non-text response,
   post-check `Reject`} return `NarrationOutcome::FellBack`. There is no error
   surfaced to the UI as an error — a fallback is the honest floor (D6).

`NarrationOutcome` is a closed `core`-free `agent`-owned enum
(`Ready(SmolStr) | FellBack`) carrying a plain `SmolStr` — **NO `llm` type, NO
`ChatResponse`** — so it crosses the seam exactly as `ForwardPlan` does.

*Why `agent`, not `backtest` or `ui`:* `backtest` is the deterministic ranking
core and must not gain an `llm` edge (it would couple the reproducible engine to
a non-deterministic provider). `ui` may not cross the view line. `agent` is the
sanctioned home for "owns the provider + reads `backtest` facts + emits a
`core`-clean result over an mpsc to iced" — it is already doing exactly this for
F6. A new `crates/advisor` is rejected for the MVP (the `agent` module is one
file; a crate is over-scoped).

**D2 — The faithfulness post-check predicates + the banned-phrase list are
FROZEN here (pre-registered, not ad-hoc).** `check_faithful(text, facts)` REJECTS
the narration (→ `FellBack`) iff ANY predicate fires. The predicate set is
**closed and frozen by this ADR**; a future change to it requires an ADR
amendment (the same discipline as the `classify_verdict` freeze, ADR-0051).

- **P1 — Wrong crown.** The narration names a strategy as the winner/best/pick
  that is **not** `facts.winner` (the crowned id), OR fails to name `facts.winner`
  at all when an active strategy was crowned. *Mechanism:* case-insensitive
  search for each candidate display id (and its friendly label — "Majority vote",
  "Unanimous vote", "buy and hold") near a crown lexeme; if a non-winner id
  co-occurs with a crown lexeme, reject. The crown-lexeme set is frozen: `won`,
  `winner`, `wins`, `crowned`, `best`, `top`, `recommended`, `the pick`, `picked`,
  `came out on top`.
- **P2 — Contradicted outcome.** The narration asserts an outcome that
  contradicts `facts.outcome`:
  - `BenchmarkWins` but the text claims an **active** strategy beat / outperformed
    buy-and-hold (frozen contradiction lexemes near an active id: `beat`,
    `outperformed`, `better than holding`, `better than buy`), OR
  - `ActiveWins` but the text claims **nothing beat** buy-and-hold / holding won
    (`nothing beat`, `buy and hold won`, `holding won`, `just holding was best`),
    OR
  - `AllFragile` but the text asserts the winner is **robust / reliable / held up**
    (`robust`, `held up`, `reliable`, `survived resampling`, `passed the
    robustness`) without the fragility caveat.
- **P3 — Fabricated number.** The narration emits a **numeric token** that does
  not correspond to a value in `facts`. *Mechanism:* extract every numeric token
  from `text` via a frozen regex (signed integers / decimals / percentages,
  ignoring ordinals like "1st/2nd/3rd/4th" and the rank list positions, and
  ignoring the literal years and standing strategy-parameter window lengths that
  appear in `facts`); each remaining token must **string-match** a canonical
  rendering of some `facts` KPI (Sharpe, Sortino, Calmar, total-return %,
  max-drawdown %, trade-count) for some candidate. A token matching none → reject.
  The canonicalisation set (the exact decimal places / `%` suffix the renderer
  uses) is frozen with the formatters in `crates/ui/src/widgets/num.rs` so the
  match is exact-string, never float-tolerant.
- **P4 — Predict / advise banned phrase.** The narration trips the **frozen
  banned-phrase list** (case-insensitive substring; the prompt forbids them, the
  post-check enforces them):

  ```text
  FROZEN BANNED-PHRASE LIST (ADR-0064 § D2.P4) — predict / advise / guarantee:
    "will rise"        "will fall"          "will go up"        "will go down"
    "will increase"    "will decrease"      "will return"       "will keep"
    "will continue"    "will outperform"    "will beat"         "is going to"
    "expected return"  "expected to return" "projected return"  "future return"
    "guaranteed"       "guarantee"          "risk-free"         "sure thing"
    "you should buy"   "you should sell"    "you should invest" "should buy"
    "should sell"      "recommend buying"   "recommend selling" "we recommend you"
    "buy now"          "sell now"           "invest now"        "financial advice"
    "i advise"         "my advice"          "price target"      "going to rise"
    "going to climb"   "set to rise"        "poised to"         "likely to rise"
    "likely to climb"  "next week will"     "going forward it will"
  ```

  The list is the floor; the prompt (layer 1) hands the LLM only the facts and
  the role lock, but the prompt is a soft constraint and the post-check is the
  hard one — **a prompt alone is insufficient.** A banned-phrase hit → reject.

`FaithfulnessVerdict` is `Pass | Reject(RejectReason)` where `RejectReason` is a
closed enum (`WrongCrown | ContradictedOutcome | FabricatedNumber | BannedPhrase`)
used for the `tracing::warn` audit line on rejection (so a leaky provider is
observable) — it never reaches `ui` (the UI only sees `FellBack`).

**D3 — The narration is a TRIGGERED second async step, mirroring the F6
agent→iced return path but fired by an explicit operator action (NOT auto).** The
wiring, end to end:

- **The mirror field on `LeaderboardScreenState` (D4)** holds a closed `ui`-owned
  `NarrationState` enum. The structured bake-off result (the `BakeoffReportMirror`
  + the templated `recommendation_block`) renders **immediately on
  `BakeoffRunCompleted`**, completely independent of the narration. `NarrationState`
  defaults to `NotRequested`.
- **The "Explain" action** posts a **new** `Message::BakeoffNarrationRequested`.
  Its update arm flips `NarrationState → InFlight` and dispatches the async step:
  - In a `live` build, it sends a `NarrationRequest { facts }` over a new
    `RunHandles` mpsc into the agent (or, symmetrically with F5/F6, the iced
    thread holds a `narration_tx` and the agent's narration task holds the
    `Receiver`); the agent runs `generate_narration(provider, facts)` on the
    side-thread runtime and returns the `NarrationOutcome` over a return mpsc the
    iced thread receives via a recipe.
  - The `NarrationFacts` are built **UI-side from the already-mirrored
    `BakeoffReportMirror`** (which carries the winner, outcome, reasons, and the
    per-row KPIs) so the request crosses the seam as `core`/`ui` data, NOT an
    engine type — the request channel carries a `core`-clean
    `agent`-owned/`core`-typed `NarrationRequest`, never a `BakeoffReport`.
  - In the fixtures / no-`live` build (the render harness), the dispatch resolves
    against an **injected fake provider** (D5) on the iced runtime directly — no
    agent thread, no network — exactly as `spawn_bakeoff` resolves immediately in
    the no-`live` build.
- **The result lands on a new `Message::BakeoffNarrationCompleted(NarrationOutcome)`**
  (the iced→ui analogue of `BakeoffRunCompleted`). Its update arm updates the
  recommendation block **in place**: `Ready(prose) → NarrationState::Ready(prose)`;
  `FellBack → NarrationState::FellBack`. The bake-off result is never re-fetched
  and never blocked.
- **The four render states (D7) — there is NEVER a blank or half-answer; the
  templated copy is always the floor:**
  - `NotRequested` → the existing templated `recommendation_block` copy +
    **the "Explain" control** (the opt-in trigger).
  - `InFlight` → the existing templated copy + **a spinner/"explaining…" affordance**
    (the templated copy stays visible the whole time).
  - `Ready(prose)` → the **LLM prose** rendered in the block, labelled as an
    LLM-generated plain-language summary of the (visible, linked) structured
    result, with the persistent not-advice / simulated-€200 `disclaimer()` still
    surrounding it.
  - `FellBack` → the existing templated copy (silently — identical to
    `NotRequested` minus the control, or with a quiet "couldn't generate a summary"
    affordance; the ui-designer owns the exact treatment). The fallback is
    visually indistinguishable from the honest baseline.

**D4 — The narration crosses to `ui` as a plain `ui`-owned `NarrationState`
enum — String/enum only, no `llm`/`agent` type through `view`.** Add to
`crates/ui/src/leaderboard/state.rs`:

```rust
// crates/ui/src/leaderboard/state.rs  (sketch — developer owns the final shape)
/// The narration's lifecycle on the leaderboard recommendation block (F9).
/// String/enum only — NO `llm`/`agent`/engine type crosses `view` (the
/// RecommendationMirror discipline). The templated copy is the floor in every
/// arm except `Ready`.
#[derive(Debug, Clone, PartialEq)]
pub enum NarrationState {
    /// No "Explain" requested yet — show the templated copy + the Explain control.
    NotRequested,
    /// The narration is being generated — show the templated copy + a spinner.
    InFlight,
    /// A faithful narration passed the post-check — show this prose.
    Ready(smol_str::SmolStr),
    /// The narration was unavailable / errored / over budget / failed the
    /// post-check — show the templated copy (the honest fallback). Silent.
    FellBack,
}
```

`NarrationState` lives next to `BakeoffReportMirror` and is a field on
`LeaderboardScreenState` (default `NotRequested`). The render code matches on this
closed `ui` enum and never names an `llm` or `agent` type — identical to how the
screen matches `OutcomeKind` / `RobustnessLabel` / `ReasonLabel`. The ONLY place
an `agent`/`llm` narration type is named on the `ui` side is the
`#[cfg(feature = "live")]` recipe/adapter that maps the received
`agent::NarrationOutcome` → `Message::BakeoffNarrationCompleted` — the exact
`forward_plan/adapter.rs` boundary discipline (one edit site if a name drifts).
Gate: **no `llm` type reaches a `view` function** (the `grep llm:: crates/ui/src/{screens,state,shell}`
surface stays empty; `cargo tree -p ui` gains no NEW edge — `agent`/`llm` are
already in the `live` graph, and nothing new is added).

**D5 — The fake-`LlmProvider` seam is the injection point at `generate_narration`'s
boundary; fixtures provide BOTH a faithful and an unfaithful fake (no network).**
Because `generate_narration` takes `provider: &Arc<dyn LlmProvider>`, every test
and every render harness injects a fake that `impl LlmProvider` and returns a
canned `ChatResponse` — the repo's "every external I/O behind a trait" rule, which
`LlmProvider` already satisfies (the `llm` crate's own tests fake it the same way).
Two fakes are frozen as fixtures:

- **`FaithfulFakeProvider`** — returns a canned `ChatResponse` whose text names
  `facts.winner` as the winner, states the outcome correctly, uses only numbers
  present in `facts`, and trips no banned phrase. Drives the `Ready` path + the
  `Ready` render state.
- **`UnfaithfulFakeProvider`** — returns a canned `ChatResponse` that violates a
  predicate (parameterised: wrong crown / contradicted outcome / fabricated number
  / banned phrase). Drives the post-check `Reject` → `FellBack` path + the
  anti-hallucination e2e.

Prompt caching + the budget path are exercised **structurally**, not over the
network: the request the generator builds carries a `SystemBlock::Cached(…,
CacheBreakpoint::Ephemeral)` prefix (asserted in a unit test on the request shape),
and the production stack injects the `BudgetedProvider`-wrapped provider (so a
`BudgetExceeded` from the budget gate is just another `FellBack` — testable by a
fake that returns `Err(LlmError::BudgetExceeded)`).

**D6 — Honest fallback is mandatory and total; the narration never blocks or
breaks the bake-off.** Every failure mode lands `FellBack` → the templated copy:
provider disabled (`cfg.llm.enabled = false`, the default — in which case the
"Explain" control may be hidden entirely, or pressing it lands `FellBack`
immediately), unavailable, network error, timeout, `BudgetExceeded`, `ReplayMiss`,
empty/non-text response, OR post-check `Reject`. The bake-off ranking is
deterministic and reproducible and does NOT depend on, wait synchronously on, or
break because of the LLM. The narration is **strictly additive polish over a
surface that is already complete without it.**

**D7 — Ephemeral; the narration is NOT persisted in the bake-off report
artifact.** The structured `Recommendation` is the reproducible artifact; the
prose is a derived view held only in `NarrationState::Ready` for the session.
F9 writes no `spec/*/reports/` body and changes no `spec/anchors.toml` SHA.
`scripts/verify_anchors.sh` stays **119/119 by construction** (run before + after).
The CLAUDE.md day-1 **baseline-equity-divergence e2e gate is N/A**: F9 is a
read-only narration surface — it produces no equity, no signal, no fill, like F6
(ADR-0062 § D7) — it is NOT a strategy overlay or sizing modifier (that gate
landed on F4 / ADR-0060 § D2 and bit again on F8). This is stated explicitly so
the tester does not expect it.

**D8 — The verification contract (handed to the tester):**

1. **The post-check unit tests (the load-bearing gate, deterministic — no LLM).**
   One test per predicate, each asserting REJECT → `FellBack`:
   (P1) a narration crowning a non-winner; (P2) a narration contradicting the
   `RecommendationOutcome` (claims active beat the benchmark when the outcome is
   `BenchmarkWins`); (P3) a narration emitting a numeric token absent from
   `NarrationFacts`; (P4) a narration tripping a banned phrase. PLUS a faithful
   narration asserted ACCEPT → `Ready`. These call `check_faithful` directly.
2. **The anti-hallucination e2e (the F9 day-1-equivalent gate).** Drive the
   narration path with the `UnfaithfulFakeProvider` and assert the surface lands
   `NarrationState::FellBack` (the templated copy), NOT the unfaithful prose —
   proving the net catches a bad LLM end-to-end. A second e2e with the
   `FaithfulFakeProvider` asserts `Ready(prose)`. A third asserts every other
   `FellBack` trigger (a fake returning `Err(BudgetExceeded)` → `FellBack`).
3. **Async second-step ordering.** A test proves the structured result renders on
   `BakeoffRunCompleted` BEFORE any narration arrives (the recommendation block is
   complete with `NarrationState::NotRequested`), and the narration updates the
   block in place on its own `BakeoffNarrationCompleted` message.
4. **Render-layer PNG (the floor, per the CLAUDE.md cockpit rule, through the
   FAKE seam — no network).** The `Ready` state (faithful fake → prose painted in
   the recommendation block, disclaimer present) AND the `FellBack` / `NotRequested`
   state (the NEGATIVE CONTROL = templated copy painted, disclaimer present, no
   unfaithful prose) — pixel-verified, PNGs saved. Harness: the
   `leaderboard_populated_render.rs` `iced_test::screenshot` real-renderer pattern,
   macOS-canonical per ADR-0057 § D2.
5. **Layering preserved.** No `llm` type reaches a `view` function (the
   `screens/`/`state.rs`/`shell.rs` `llm::` surface stays empty); `cargo tree -p ui`
   gains no NEW edge.
6. `scripts/verify_anchors.sh` 119/119 before and after.

## Alternatives considered

- **Generator in `backtest` (a `backtest`-adjacent narration seam)** — rejected
  (D1): `backtest` is the deterministic ranking core; giving it an `llm` edge
  couples the reproducible engine to a non-deterministic, budget-gated provider
  and inverts the dependency direction (`backtest` would need the provider, which
  is built in `agent`). `agent` already owns the provider + reads `backtest`.
- **Generator in `ui` (resolve the LLM call inside the screen)** — rejected: it
  would force a real `llm` type across `view` (the call site, the `ChatResponse`),
  violating the standing invariant, and `ui` does not own the provider or
  credentials. The narration must be produced architect-side of the `ui` line
  (product § Constraints), exactly as F6 produces the plan agent-side.
- **Prompt-only faithfulness guard (no post-check)** — rejected as the sole guard
  (the analyst's option (a)): a prompt is a soft constraint; one hallucinated
  number or "this should keep working" slips the whole credibility claim with no
  net. The post-check (D2) is the mechanically-enforced invariant; the prompt is
  the first line, not the only line.
- **Structured-output / tool-use narration (the LLM fills a constrained schema)**
  — deferred (the analyst's option (c)): maximally safe but collapses toward
  templates + loses the plain-language fluency that is the entire point of F9. It
  is the v0.3 hardening path if the post-check proves leaky; over-engineered for
  v0.2. Noted as the escalation, not the MVP.
- **Auto-generate the narration after every bake-off** — rejected (operator
  LOCKED opt-in): the bake-off result is complete and honest with the templated
  copy; an automatic LLM call on every run spends budget + adds latency for a
  derived view the operator may not want. The "Explain" trigger makes the cost
  opt-in and the second async step is fired by an explicit action, not the
  `BakeoffRunCompleted` message.
- **Persist the prose in the bake-off report artifact** — rejected (operator
  LOCKED ephemeral): the structured `Recommendation` is the reproducible artifact;
  persisting non-deterministic prose into an anchored-adjacent artifact would
  introduce a run-varying body and break the determinism discipline. The prose is
  a session-only derived view.
- **A non-deterministic / float-tolerant number check for P3** — rejected: the
  KPI numbers are rendered by the `ui::widgets::num` formatters to fixed decimal
  places; the post-check does an **exact-string** match against those canonical
  renderings, so "did the LLM invent a number" is a deterministic, unit-testable
  predicate, not a fuzzy numeric comparison.
- **A new `crates/advisor` crate to home the generator** — rejected for the MVP:
  the generator is one `agent` module (the `agent::plan` precedent); a crate is
  over-scoped and would add an edge for no benefit.

## Consequences

- **Faithfulness is a mechanically-enforced invariant, not a hope (D2).** The
  post-check is a pure deterministic function with a frozen predicate set + a
  frozen banned-phrase list; a leaky narration degrades to the honest templated
  copy rather than shipping a fabrication. The anti-hallucination e2e (D8.2)
  proves the net bites end-to-end. The post-check carries forward unchanged when
  the F6-plan narration arrives in the same voice.
- **Layering invariant held (D4).** The narration crosses `view` as a plain
  `ui`-owned `NarrationState` enum; the `agent`→`ui` map lives in the one
  `#[cfg(feature = "live")]` recipe/adapter; the generation (which names `llm`)
  is entirely agent-side. No `llm` type reaches a `view` function; `cargo tree -p ui`
  gains no NEW edge.
- **The bake-off stays complete + deterministic without the LLM (D3 / D6).** The
  structured result renders immediately on `BakeoffRunCompleted`; the narration is
  a triggered second async step that updates the block in place; every failure
  mode (including a budget block and a post-check rejection) lands the templated
  copy. The narration never blocks, waits on, or breaks the ranking.
- **Cost is opt-in + cached + budget-governed (operator locks + D5).** One cheap
  call per "Explain" click with a cache-marked static prefix
  (`CacheBreakpoint::Ephemeral`), through the `BudgetedProvider` stack the monthly
  budget + 80%/100% auto-degrade already govern. A budget block is just another
  `FellBack`.
- **Ephemeral; anchor-neutral; equity-divergence-N/A (D7).** No anchored body
  written, no `anchors.toml` SHA changed → `verify_anchors.sh` 119/119 by
  construction. The CLAUDE.md sizing-modifier e2e gate does not apply (F9 narrates,
  it does not size/run); stated so the tester does not expect it.
- **F9 is the first in-app LLM consumer (the foundation's first pluck).** It
  consumes `crates/llm` (the trait + `BudgetedProvider` + `CachedSystemPromptBuilder`
  + record/replay) **unchanged** and the frozen `backtest::bakeoff` types unchanged;
  the only new code is the `agent::narration` module + the `ui` mirror field + the
  second-async-step message + the recommendation-block render + the fake fixtures.
- **Reuse / future-proofing.** The same `agent::narration` seam + the same
  post-check narrate the F6 forward plan in the same voice (a `NarrationFacts`
  variant over the `ForwardPlan` structured data) with no `ui` rework — the closed
  `NarrationState` is already the surface. The v0.3 structured-output hardening
  (rejected option (c)) plugs in behind the same generator boundary if the
  post-check proves leaky.
- **This ADR does not add, remove, or mutate any of the 9 anchor SHAs in
  `spec/anchors.toml`** — F9 produces no anchored artifact; the
  anchor-mutation-requires-an-ADR rule is untriggered.
- **Open (none gate the build):** if the post-check's P3 number-extraction proves
  to have false-positives on legitimately-quoted facts at integration, the
  canonical-rendering match set is the single tuning point (frozen with the `num`
  formatters); a substantive change to the predicate set or banned-phrase list is
  an ADR-0064 amendment, not an ad-hoc edit.

## Changelog

- 2026-06-22 (architect): initial accept. Homes the **LLM "why this one"
  narration seam** for feature `advisor-llm-narration` (F9, the first in-app LLM
  consumer). **D1** the generator lives AGENT-SIDE in a new `agent::narration`
  module (the `agent::plan`/ADR-0062 twin — `agent` already owns the
  `Arc<dyn LlmProvider>` + hard-deps `llm` AND `backtest`); it owns `NarrationFacts`
  (the exact machine values built from `BakeoffReport` at one boundary), the
  role-locked cache-marked prompt (layer 1), the pure deterministic faithfulness
  post-check (layer 2), and `generate_narration` returning a `core`-clean
  `NarrationOutcome { Ready(SmolStr) | FellBack }` — no `llm` type crosses. **D2**
  the post-check predicates + banned-phrase list are FROZEN here (pre-registered,
  ADR-locked): P1 wrong crown, P2 contradicted `RecommendationOutcome`, P3
  fabricated number (exact-string match against the `num`-formatter canonical
  renderings — deterministic, not float-tolerant), P4 a frozen predict/advise
  banned-phrase list; a prompt alone is insufficient, the post-check is the net;
  reject → `FellBack`. **D3** the narration is a TRIGGERED second async step (the
  operator "Explain" action posts `Message::BakeoffNarrationRequested` → InFlight
  → the agent runs `generate_narration` → `Message::BakeoffNarrationCompleted`
  updates the block in place), NOT auto-on-`BakeoffRunCompleted` (operator LOCKED
  opt-in). **D4** the narration crosses `view` as a plain `ui`-owned
  `NarrationState { NotRequested | InFlight | Ready(SmolStr) | FellBack }` (the
  `RecommendationMirror` discipline; the one `#[cfg(feature="live")]` recipe/adapter
  is the only `agent`-type edit site). **D5** the fake-`LlmProvider` seam injects
  at `generate_narration`'s boundary; fixtures provide a `FaithfulFakeProvider` +
  an `UnfaithfulFakeProvider` (no network); caching (`CacheBreakpoint::Ephemeral`)
  + the `BudgetedProvider` path are exercised structurally. **D6** mandatory honest
  fallback — disabled/unavailable/error/timeout/`BudgetExceeded`/`ReplayMiss`/empty/
  post-check-reject → the templated copy; never blocks the bake-off, never a
  half-answer. **D7** ephemeral (NOT persisted — operator LOCKED; the structured
  `Recommendation` is the reproducible artifact); anchor-neutral (119/119 by
  construction) + the CLAUDE.md day-1 equity-divergence e2e is N/A (F9 narrates,
  produces no equity/signal/fill — like F6, unlike F4/F8). **D8** verification =
  per-predicate post-check unit tests (deterministic, no LLM) + the
  anti-hallucination e2e (unfaithful fake → `FellBack`, faithful fake → `Ready`,
  budget-block → `FellBack`) + the async-ordering test + the render-layer PNG
  (`Ready` prose + the `FellBack`/`NotRequested` negative control, through the fake
  seam) + no `llm` type through `view` + 119/119. Model = the Anthropic default via
  `BudgetedProvider` (operator LOCKED). Feature: `advisor-llm-narration`. Leans on
  ADR-0062 (§ D3–D4 agent→iced return-path + the one-adapter mirror discipline),
  ADR-0059 (the `Recommendation`-not-`String` structured-data precedent + the
  `BakeoffReportMirror::from_report` one-boundary read), ADR-0019 (the v2 LLM
  foundation — trait + `BudgetedProvider` + `CachedSystemPromptBuilder` +
  record/replay), ADR-0023/0041 (ui layering — no `strategy`/`exec`/`forecast`/`llm`
  through `view`), ADR-0057 § D2 (macOS render-pixel canonicality), ADR-0051 (the
  frozen-predicate-set discipline).
