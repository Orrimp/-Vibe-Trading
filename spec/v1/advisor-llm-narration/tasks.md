---
slug: advisor-llm-narration
status: in-progress
owner: developer
updated: 2026-06-22
---

# Tasks — advisor LLM "why this one" narration (F9)

Seam: **[ADR-0064](../../../_bmad-output/planning-artifacts/architecture/decisions/0064-advisor-llm-narration-seam.md)**.
F9 is the F6 forward-plan agent→iced return path (ADR-0062) made **triggered**
(opt-in "Explain") + guarded by a **deterministic faithfulness post-check**.

**Parallelism:** the developer track (D) and the ui-designer track (U) run in
**parallel**. The seam between them is two closed `ui`-owned types the
ui-designer can mock from day 1: `NarrationState`
(`{ NotRequested | InFlight | Ready(SmolStr) | FellBack }`) and the new
`Message::BakeoffNarrationRequested` / `Message::BakeoffNarrationCompleted`
variants. The developer owns the generator + the post-check + the wiring + the
mirror field + the fake fixtures + the post-check unit tests + the
anti-hallucination e2e; the ui-designer owns the "Explain" control + the 4 render
states + the render PNGs. The integration point is the
`#[cfg(feature = "live")]` recipe/adapter (one edit site, per ADR-0064 § D4).

## Hard constraints (carry through every task)

- Do **NOT** modify `crates/llm` (consume the shipped `LlmProvider` trait +
  `BudgetedProvider` + `CachedSystemPromptBuilder` + `CacheBreakpoint::Ephemeral`
  unchanged) or the frozen `backtest::bakeoff` types.
- **No `llm` type crosses `view`** — the narration reaches the screen as the plain
  `NarrationState` enum. The only `agent`/`llm` narration type named on the `ui`
  side is the one `#[cfg(feature = "live")]` recipe/adapter.
- **No network in any test or render** — only the fake `LlmProvider` seam.
- The narration is **ephemeral** (NOT written to the bake-off report artifact) and
  runs **no backtest scenario** → `scripts/verify_anchors.sh` stays **119/119**.
  Touch **zero** files under `spec/*/reports/`. The CLAUDE.md day-1
  equity-divergence e2e is **N/A** (read-only narration — no equity/signal/fill,
  like F6).
- `Result<T, E>` in lib code, no `.unwrap()` outside tests, `tracing` not
  `println!`, `cargo fmt` + `cargo clippy -- -D warnings` clean.

---

## Developer track (D) — `agent::narration` + the post-check + wiring + tests

- [x] **D1 — `agent::narration` module skeleton + `NarrationFacts`.** New
  `crates/agent/src/narration.rs` (the `agent::plan` twin). Define `NarrationFacts`
  carrying ONLY the exact machine values the prompt may speak about (the
  `RecommendationOutcome`, the crowned `StrategyId` display string, each
  candidate's already-computed KPIs as canonical strings, each candidate's
  `RobustnessFlag`, the reason codes). Build it from `backtest::BakeoffReport` at
  this one boundary (the `BakeoffReportMirror::from_report` precedent). Re-export
  from `agent::lib` (`NarrationFacts`, `NarrationOutcome`). _acceptance: builds a
  `NarrationFacts` from a fixture `BakeoffReport`; carries no `llm` type._
  - **file:line** `crates/agent/src/narration.rs:74` (`NarrationFacts`), `:117` (`from_report`)
  - **test** `cargo test -p agent --lib narration::tests::d4_faithful_narration_passes`
  - **output** `test narration::tests::d4_faithful_narration_passes ... ok`

- [x] **D2 — `NarrationOutcome` (the `core`-clean return type).** A closed
  `agent`-owned enum `NarrationOutcome { Ready(SmolStr) | FellBack }` — no `llm`
  type, no `ChatResponse`. This is what crosses the agent→iced seam (the
  `ForwardPlan` precedent). _acceptance: `Clone + Debug`; names no `llm`/`backtest`
  type; re-exported from `agent`._
  - **file:line** `crates/agent/src/narration.rs:47` (`NarrationOutcome`), `src/lib.rs:33`
  - **test** `cargo test -p agent --lib narration::tests::d5_faithful_fake_produces_ready`
  - **output** `test narration::tests::d5_faithful_fake_produces_ready ... ok`

- [x] **D3 — The role-locked, cache-marked prompt builder (layer 1).** Compose a
  `ChatRequest` whose **static** system prompt (role lock + faithfulness
  constraints + the explicit NON-goals + the not-advice framing) is built via
  `CachedSystemPromptBuilder` with a `CacheBreakpoint::Ephemeral` boundary; the
  small variable user turn carries only the `NarrationFacts`. Model = the Anthropic
  default (the request flows through the injected `BudgetedProvider`-wrapped
  provider). _acceptance: a unit test asserts the request's system prompt carries a
  `SystemBlock::Cached(_, CacheBreakpoint::Ephemeral)` prefix and the user turn
  contains the facts; no network._
  - **file:line** `crates/agent/src/narration.rs:687` (`build_narration_request`), `:661` (system prompt)
  - **test** `cargo test -p agent --lib narration::tests::d3_request_has_ephemeral_cache_block`
  - **output** `test narration::tests::d3_request_has_ephemeral_cache_block ... ok`

- [x] **D4 — The FROZEN faithfulness post-check (THE LOAD-BEARING GUARD).** A pure,
  deterministic, `llm`-free `check_faithful(text: &str, facts: &NarrationFacts) ->
  FaithfulnessVerdict` (`Pass | Reject(RejectReason)`), implementing the **frozen**
  predicate set + banned-phrase list of **ADR-0064 § D2** verbatim: **P1** wrong
  crown (a non-`facts.winner` id near a frozen crown lexeme, or the winner not
  named when active wins), **P2** contradicted `RecommendationOutcome` (the three
  frozen contradiction cases), **P3** fabricated number (every numeric token
  exact-string-matches a `num`-formatter canonical rendering of a `facts` KPI;
  ordinals + the literal years/window lengths in `facts` ignored), **P4** the
  frozen predict/advise banned-phrase list. `RejectReason` drives a `tracing::warn`
  on rejection (a leaky provider is observable); it never reaches `ui`. _acceptance:
  the predicate set + banned-phrase list match ADR-0064 § D2 exactly; pure (no I/O,
  no `llm` dep); the P3 number match is exact-string, not float-tolerant._
  - **file:line** `crates/agent/src/narration.rs:507` (`check_faithful`), `:291` (BANNED_PHRASES), `:338` (CROWN_LEXEMES)
  - **test** `cargo test -p agent --lib narration::tests::d4_p1_wrong_crown_rejects narration::tests::d4_p2_benchmark_wins_but_active_claims_win_rejects narration::tests::d4_p3_fabricated_number_rejects narration::tests::d4_p4_banned_phrase_will_rise_rejects narration::tests::d4_faithful_narration_passes`
  - **output** `test result: ok. 21 passed; 0 failed; 0 ignored` (all 21 narration tests green)

- [x] **D5 — `generate_narration(provider, facts) -> NarrationOutcome`.** The async
  orchestrator: build the request (D3) → `provider.complete(req).await` → on `Ok`,
  extract the text and run `check_faithful` (D4); on `Pass` → `Ready(text)`; on
  **any** of {provider `Err` (disabled / network / timeout / `BudgetExceeded` /
  `ReplayMiss`), empty/non-text response, post-check `Reject`} → `FellBack`. No
  error is surfaced as an error — the fallback is the floor (ADR-0064 § D6).
  _acceptance: every failure mode returns `FellBack`; a faithful provider returns
  `Ready`; takes `provider: &Arc<dyn LlmProvider>` so the fake seam injects here._
  - **file:line** `crates/agent/src/narration.rs:772` (`generate_narration`)
  - **test** `cargo test -p agent --lib narration::tests::d5_faithful_fake_produces_ready narration::tests::d11_budget_exceeded_produces_fellback`
  - **output** `test result: ok. 21 passed; 0 failed; 0 ignored`

- [x] **D6 — The triggered second-async-step wiring (agent side).** Add the
  `RunHandles` mpsc channel(s) for the narration request/return (symmetric with the
  F5/F6 `forward_rx` / `plan_tx` pattern): the iced thread sends a `core`-clean
  `NarrationRequest { facts }`; the agent's narration task runs `generate_narration`
  on the side-thread runtime and returns the `NarrationOutcome`. `None` channels →
  byte-identical to today (headless / soak unaffected). _acceptance: `cockpit_live`
  constructs the channels under `live`; headless `trading` bin + soak pass `None`;
  the request carries `core`/`facts` data, never a `BakeoffReport`._
  - **file:line** `crates/agent/src/runtime.rs:178` (`narration_request_rx`), `:188` (`narration_outcome_tx`), `:1259` (narration task spawn)
  - **test** `cargo test -p agent` (all 97 lib tests + 55 integration tests pass with None channels)
  - **output** `test result: ok. 97 passed; 0 failed` (lib) + all integration test results `ok`

- [x] **D7 — The `NarrationState` mirror field (ui state — owned by D, consumed by
  U).** Add the closed `ui`-owned `NarrationState { NotRequested | InFlight |
  Ready(SmolStr) | FellBack }` to `crates/ui/src/leaderboard/state.rs` as a field on
  `LeaderboardScreenState` (default `NotRequested`), with `begin_narration()` /
  `set_narration(outcome)` helpers mirroring `begin_run` / `finish_run`. String/enum
  only. _acceptance: no `llm`/`agent` type named in `state.rs`; default is
  `NotRequested`; unit tests on the state transitions._
  - **note**: Pre-built by ui-designer parallel track (verified present in `crates/ui/src/leaderboard/state.rs` before developer work began). All `NarrationState` transitions tested in ui crate.

- [x] **D8 — The new messages + update arms + the in-place block update.** Add
  `Message::BakeoffNarrationRequested` (flips `NarrationState → InFlight` +
  dispatches the async step) and `Message::BakeoffNarrationCompleted(NarrationOutcome)`
  (maps `Ready(prose) → NarrationState::Ready(prose)`, `FellBack →
  NarrationState::FellBack`) to `crates/ui/src/state.rs`, plus the one
  `#[cfg(feature = "live")]` recipe/adapter mapping the received
  `agent::NarrationOutcome` → `Message::BakeoffNarrationCompleted` (the
  `forward_plan/adapter.rs` boundary — the only `agent`-type edit site). The
  structured result must already have rendered on `BakeoffRunCompleted`; the
  narration never re-fetches or blocks it. _acceptance: an async-ordering test
  proves the structured result renders (with `NarrationState::NotRequested`) before
  any narration arrives, and the narration updates the block in place on its own
  message._
  - **note**: Pre-built by ui-designer parallel track (verified present in `crates/ui/src/state.rs` before developer work began).

- [x] **D9 — The fake-`LlmProvider` fixtures (faithful + unfaithful, no network).**
  In the ui fixtures (and/or an `agent`/test-support module the render harness can
  reach), provide a `FaithfulFakeProvider` (`impl LlmProvider`, returns a canned
  faithful `ChatResponse` for the given facts → drives `Ready`) and an
  `UnfaithfulFakeProvider` (parameterised: wrong crown / contradicted outcome /
  fabricated number / banned phrase → drives `Reject` → `FellBack`). _acceptance:
  both `impl LlmProvider`; neither makes a network call; the unfaithful fake trips
  exactly the targeted predicate._
  - **file:line** `crates/agent/src/narration.rs` — `FaithfulFakeProvider` (after `generate_narration`), `UnfaithfulFakeProvider` with `UnfaithfulViolation`, `BudgetExceededFakeProvider`; re-exported from `agent::lib`
  - **test** `cargo test -p agent --lib narration::tests::d11_anti_hallucination_wrong_crown_produces_fellback narration::tests::d5_faithful_fake_produces_ready`
  - **output** `test result: ok. 21 passed; 0 failed; 0 ignored`

- [x] **D10 — The post-check unit tests (deterministic, no LLM).** One test per
  predicate, each asserting REJECT → `FellBack`: (P1) crowns the wrong winner;
  (P2) contradicts the `RecommendationOutcome` (claims active beat the benchmark
  when the outcome is `BenchmarkWins`); (P3) fabricates a number absent from
  `NarrationFacts`; (P4) uses a banned phrase. PLUS a faithful narration asserted
  ACCEPT → `Ready`. Call `check_faithful` directly. _acceptance: 5 tests, all
  deterministic, no provider/LLM involved._
  - **file:line** `crates/agent/src/narration.rs` — `d4_p1_wrong_crown_rejects`, `d4_p2_benchmark_wins_but_active_claims_win_rejects`, `d4_p3_fabricated_number_rejects`, `d4_p4_banned_phrase_will_rise_rejects`, `d4_faithful_narration_passes` (plus 6 additional per-predicate coverage tests)
  - **test** `cargo test -p agent --lib narration`
  - **output** `test result: ok. 21 passed; 0 failed; 0 ignored; finished in 0.00s`

- [x] **D11 — The anti-hallucination e2e (the F9 day-1-equivalent gate).** Drive
  the narration path with the `UnfaithfulFakeProvider` end-to-end and assert the
  surface lands `NarrationState::FellBack` (the templated copy), NOT the unfaithful
  prose — proving the net catches a bad LLM end-to-end. PLUS: a faithful-fake e2e
  asserting `Ready(prose)`, and a `BudgetExceeded`-fake e2e asserting `FellBack`.
  _acceptance: the unfaithful prose never reaches `NarrationState::Ready`; all
  through the fake seam, no network._
  - **file:line** `crates/agent/src/narration.rs` — `d11_anti_hallucination_wrong_crown_produces_fellback`, `d11_anti_hallucination_contradicted_outcome_produces_fellback`, `d11_anti_hallucination_fabricated_number_produces_fellback`, `d11_anti_hallucination_banned_phrase_produces_fellback`, `d11_budget_exceeded_produces_fellback`
  - **test** `cargo test -p agent --lib narration::tests::d11`
  - **output** `test result: ok. 21 passed; 0 failed; 0 ignored; finished in 0.00s`

- [x] **D12 — Layering + anchor gates.** Confirm `grep -rn "llm::"
  crates/ui/src/{screens,state.rs,shell.rs}` stays empty (no `llm` type through
  `view`) and `cargo tree -p ui` gains no NEW edge; run `scripts/verify_anchors.sh`
  → 119/119 before and after. _acceptance: both gates green; trace `tests` +
  `crates` rows updated via `spec-update`._
  - **file:line** `scripts/verify_anchors.sh` → output; `grep llm:: crates/ui/src/{screens,state.rs,shell.rs}` → empty
  - **test** `bash scripts/verify_anchors.sh`
  - **output** `ANCHORS PASS  (119 / 119)`

## UI-designer track (U) — the "Explain" control + the 4 render states + PNGs

These run in parallel with D against the `NarrationState` enum + the two new
`Message` variants (D7/D8 names), which the ui-designer mocks from day 1 via the ui
fixtures (no agent needed for the render harness).

- [ ] **U1 — The "Explain" control on the crowned recommendation block.** Add an
  opt-in "Explain" affordance to `recommendation_block` in
  `crates/ui/src/screens/leaderboard.rs` that posts
  `Message::BakeoffNarrationRequested`. Copy via `crate::strings` (zero string
  literals); tokens via `crate::theme` (zero hex). The control shows only in the
  `NotRequested` state. _acceptance: the control renders + is wired to the message;
  no new theme token / widget._

- [ ] **U2 — The 4 render states in `recommendation_block`.** Match the
  `NarrationState` field: `NotRequested` → templated copy + the Explain control;
  `InFlight` → templated copy + a spinner/"explaining…" affordance (reuse
  `frame::loading_with_spinner` or the existing pattern); `Ready(prose)` → the LLM
  prose, **labelled** as an LLM-generated plain-language summary of the (visible)
  structured result, with the persistent `disclaimer()` still surrounding it;
  `FellBack` → the templated copy (silent — visually indistinguishable from the
  honest baseline, optionally a quiet "couldn't generate a summary" note). The
  templated copy + the disclaimer are the floor in every state but `Ready` — there
  is NEVER a blank or half-answer. _acceptance: all 4 states render something; the
  disclaimer is present in every state; copy in `strings`, tokens in `theme`._

- [ ] **U3 — The narration-label + fallback strings.** Add the LLM-summary label +
  any "explaining…" / "couldn't generate a summary" copy to
  `crates/ui/src/strings.rs`. The **fallback prose is the EXISTING templated copy**
  (`headline_copy` + `reason_copy`) — do not author new fallback prose. _acceptance:
  new strings added; the fallback reuses the shipped templated copy._

- [ ] **U4 — Render-layer PNG proof (CLAUDE.md cockpit rule — the FLOOR, through
  the fake seam, no network).** A `*_render.rs` harness (the
  `leaderboard_populated_render.rs` `iced_test::screenshot` real-renderer pattern,
  macOS-canonical per ADR-0057 § D2) reads the rendered leaderboard PNG in: (a) the
  `Ready` state (faithful fake → the LLM prose is **painted** in the block,
  disclaimer present) AND (b) the `FellBack` / `NotRequested` state (the **negative
  control** = the templated copy is painted, disclaimer present, no unfaithful
  prose). Save the PNGs under the feature's `reports/screenshots/`. _acceptance: the
  prose actually paints in `Ready`; the templated copy paints in the fallback; PNGs
  saved; no network (the ui fixture fake provides the narration)._

## Notes

- **The post-check (D4) is the heart of F9** — it is what protects the product's
  "measured robustness, not asserted alpha" credibility. Its predicate set +
  banned-phrase list are FROZEN in ADR-0064 § D2; do not improvise them. A
  substantive change is an ADR-0064 amendment, not an ad-hoc edit.
- **The templated copy is always the floor.** Every render state but `Ready` shows
  the existing structured templated copy; the fallback is silent and honest.
- **The reconciliation seam** (developer ‖ ui-designer) is the `NarrationState`
  enum + the two `Message` variants. If a name drifts at integration, the single
  `#[cfg(feature = "live")]` recipe/adapter (D8) is the one edit site — the mirror
  discipline keeps the blast radius to one function (the `forward_plan/adapter.rs`
  precedent).
